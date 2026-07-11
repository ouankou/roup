/*
 * Strict ompparser adapter for the opaque ROUP C ABI.
 *
 * The adapter never observes Rust layouts, borrows interior pointers, or
 * reparses a rendered clause. Tagged semantic records are traversed through
 * RoupNodeHandle; leaf spellings are copied into ompparser-owned strings.
 */

#include <OpenMPIR.h>
#include <roup.h>

#include "typed_contract.h"

#include <cctype>
#include <cstdint>
#include <cstring>
#include <functional>
#include <limits>
#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace {

OpenMPBaseLang current_lang = Lang_unknown;

constexpr std::size_t mapped_directive_count =
    0
#define ROUP_COUNT_MAPPING(Abi, Parser) +1
        ROUP_OMPPARSER_DIRECTIVES(ROUP_COUNT_MAPPING)
        ROUP_OMPPARSER_END_DIRECTIVES(ROUP_COUNT_MAPPING)
        ROUP_OMPPARSER_DIRECT_END_DIRECTIVES(ROUP_COUNT_MAPPING)
#undef ROUP_COUNT_MAPPING
    ;
constexpr std::size_t mapped_clause_count =
    0
#define ROUP_COUNT_MAPPING(Abi, Parser) +1
        ROUP_OMPPARSER_CLAUSES(ROUP_COUNT_MAPPING)
#undef ROUP_COUNT_MAPPING
    ;
static_assert(mapped_directive_count == ROUP_OMP_DIRECTIVE_OMPX + 1,
              "typed OpenMP directive ordinal map is incomplete");
static_assert(mapped_clause_count == ROUP_OMP_CLAUSE_WRITE + 1,
              "typed OpenMP clause ordinal map is incomplete");

int location_component(std::size_t value, const char *context) {
  if (value == 0 ||
      value > static_cast<std::size_t>(std::numeric_limits<int>::max())) {
    throw std::runtime_error(std::string(context) +
                             " is outside ompparser's location range");
  }
  return static_cast<int>(value);
}

void set_source_location(SourceLocation &target, const RoupSpan &span,
                         const char *context) {
  target.setLine(location_component(span.start_line, context));
  target.setColumn(location_component(span.start_column, context));
}

std::string status_context(const char *context, RoupStatus status) {
  return std::string(context) + " failed with ROUP status " +
         std::to_string(status);
}

void release_error_without_recursion(RoupErrorHandle error) {
  const RoupCallResult released = roup_error_release(error);
  if (released.status != ROUP_STATUS_OK) {
    throw std::runtime_error(
        status_context("releasing a ROUP diagnostic", released.status));
  }
}

[[noreturn]] void throw_failure(RoupCallResult result, const char *context) {
  std::string message = status_context(context, result.status);

  const RoupU32Result code = roup_error_code(result.error);
  if (code.result.status != ROUP_STATUS_OK) {
    release_error_without_recursion(code.result.error);
    release_error_without_recursion(result.error);
    throw std::runtime_error(status_context(
        "querying a ROUP diagnostic code", code.result.status));
  }

  const RoupSizeResult length = roup_error_message_length(result.error);
  if (length.result.status != ROUP_STATUS_OK) {
    release_error_without_recursion(length.result.error);
    release_error_without_recursion(result.error);
    throw std::runtime_error(status_context(
        "querying a ROUP diagnostic length", length.result.status));
  }

  std::string detail(length.value, '\0');
  const RoupSizeResult copied = roup_error_message_copy(
      result.error, reinterpret_cast<std::uint8_t *>(detail.data()),
      detail.size());
  if (copied.result.status != ROUP_STATUS_OK || copied.value != detail.size()) {
    if (copied.result.status != ROUP_STATUS_OK) {
      release_error_without_recursion(copied.result.error);
    }
    release_error_without_recursion(result.error);
    throw std::runtime_error(
        copied.result.status != ROUP_STATUS_OK
            ? status_context("copying a ROUP diagnostic", copied.result.status)
            : "ROUP diagnostic copy returned a short write");
  }

  release_error_without_recursion(result.error);
  message += " (diagnostic " + std::to_string(code.value) + "): " + detail;
  throw std::runtime_error(message);
}

void require_ok(RoupCallResult result, const char *context) {
  if (result.status != ROUP_STATUS_OK) {
    throw_failure(result, context);
  }
}

void discard_cleanup_result(RoupCallResult result) noexcept {
  if (result.status != ROUP_STATUS_OK) {
    (void)roup_error_release(result.error);
  }
}

template <typename Result>
Result require_value(Result result, const char *context) {
  require_ok(result.result, context);
  return result;
}

template <typename Length, typename Copy>
std::string copy_string(Length length_operation, Copy copy_operation,
                        const char *context) {
  const RoupSizeResult length = require_value(length_operation(), context);
  std::string value(length.value, '\0');
  const RoupSizeResult copied = require_value(
      copy_operation(reinterpret_cast<std::uint8_t *>(value.data()),
                     value.size()),
      context);
  if (copied.value != value.size()) {
    throw std::runtime_error(std::string(context) +
                             " returned a short string copy");
  }
  return value;
}

enum class FieldScope { Clause, Parameter, Node };

class FieldReader {
public:
  static FieldReader clause(RoupDirectiveHandle directive,
                            std::size_t clause_index) {
    return FieldReader(FieldScope::Clause, directive, clause_index,
                       RoupNodeHandle{});
  }

  static FieldReader parameter(RoupDirectiveHandle directive) {
    return FieldReader(FieldScope::Parameter, directive, 0,
                       RoupNodeHandle{});
  }

  static FieldReader node(RoupNodeHandle node) {
    return FieldReader(FieldScope::Node, RoupDirectiveHandle{}, 0,
                       node);
  }

  std::optional<std::size_t> find(std::uint32_t id) {
    std::optional<std::size_t> found;
    for (std::size_t index = 0; index < fields_.size(); ++index) {
      if (fields_[index].id == id) {
        if (found.has_value()) {
          throw std::runtime_error("duplicate typed field id " +
                                   std::to_string(id));
        }
        found = index;
      }
    }
    return found;
  }

  std::optional<std::string> optional_string(std::uint32_t id) {
    const std::optional<std::size_t> index = find(id);
    if (!index.has_value()) {
      return std::nullopt;
    }
    consume(*index);
    const RoupFieldInfo info = fields_[*index];
    if (info.value_kind != ROUP_FIELD_VALUE_STRING || info.count != 1) {
      throw std::runtime_error("typed field " + std::to_string(id) +
                               " is not one string");
    }
    return string_at(*index, 0);
  }

  std::string required_string(std::uint32_t id) {
    std::optional<std::string> value = optional_string(id);
    if (!value.has_value()) {
      throw std::runtime_error("missing required typed string field " +
                               std::to_string(id));
    }
    return std::move(*value);
  }

  std::vector<std::string> optional_strings(std::uint32_t id) {
    const std::optional<std::size_t> index = find(id);
    if (!index.has_value()) {
      return {};
    }
    consume(*index);
    const RoupFieldInfo info = fields_[*index];
    if (info.value_kind != ROUP_FIELD_VALUE_STRING_LIST) {
      throw std::runtime_error("typed field " + std::to_string(id) +
                               " is not a string list");
    }
    std::vector<std::string> values;
    values.reserve(info.count);
    for (std::size_t value = 0; value < info.count; ++value) {
      values.push_back(string_at(*index, value));
    }
    return values;
  }

  std::vector<std::string> required_strings(std::uint32_t id) {
    const std::optional<std::size_t> index = find(id);
    if (!index.has_value()) {
      throw std::runtime_error("missing required typed string-list field " +
                               std::to_string(id));
    }
    return optional_strings(id);
  }

  std::optional<std::uint32_t> optional_u32(std::uint32_t id) {
    const std::optional<std::size_t> index = find(id);
    if (!index.has_value()) {
      return std::nullopt;
    }
    consume(*index);
    const RoupFieldInfo info = fields_[*index];
    if (info.value_kind != ROUP_FIELD_VALUE_U32 || info.count != 1) {
      throw std::runtime_error("typed field " + std::to_string(id) +
                               " is not one u32 value");
    }
    return require_value(field_u32(*index, 0),
                         "querying a typed u32 field")
        .value;
  }

  std::uint32_t required_u32(std::uint32_t id) {
    const std::optional<std::uint32_t> value = optional_u32(id);
    if (!value.has_value()) {
      throw std::runtime_error("missing required typed u32 field " +
                               std::to_string(id));
    }
    return *value;
  }

  std::vector<std::uint32_t> optional_u32s(std::uint32_t id) {
    const std::optional<std::size_t> index = find(id);
    if (!index.has_value()) {
      return {};
    }
    consume(*index);
    const RoupFieldInfo info = fields_[*index];
    if (info.value_kind != ROUP_FIELD_VALUE_U32_LIST) {
      throw std::runtime_error("typed field " + std::to_string(id) +
                               " is not a u32 list");
    }
    std::vector<std::uint32_t> values;
    values.reserve(info.count);
    for (std::size_t value = 0; value < info.count; ++value) {
      values.push_back(require_value(field_u32(*index, value),
                                     "querying a typed u32-list field")
                           .value);
    }
    return values;
  }

  std::vector<std::uint32_t> required_u32s(std::uint32_t id) {
    if (!find(id).has_value()) {
      throw std::runtime_error("missing required typed u32-list field " +
                               std::to_string(id));
    }
    return optional_u32s(id);
  }

  std::optional<bool> optional_bool(std::uint32_t id) {
    const std::optional<std::size_t> index = find(id);
    if (!index.has_value()) {
      return std::nullopt;
    }
    consume(*index);
    const RoupFieldInfo info = fields_[*index];
    if (info.value_kind != ROUP_FIELD_VALUE_BOOL || info.count != 1) {
      throw std::runtime_error("typed field " + std::to_string(id) +
                               " is not one boolean");
    }
    const RoupU32Result value = require_value(
        field_bool(*index, 0), "querying a typed boolean field");
    if (value.value > 1) {
      throw std::runtime_error("typed boolean field has a non-boolean value");
    }
    return value.value != 0;
  }

  template <typename Convert>
  void for_each_node(std::uint32_t id, Convert convert) {
    const std::optional<std::size_t> index = find(id);
    if (!index.has_value()) {
      return;
    }
    consume(*index);
    const RoupFieldInfo info = fields_[*index];
    if (info.value_kind != ROUP_FIELD_VALUE_NODE &&
        info.value_kind != ROUP_FIELD_VALUE_NODE_LIST) {
      throw std::runtime_error("typed field " + std::to_string(id) +
                               " is not a semantic node field");
    }
    if (info.value_kind == ROUP_FIELD_VALUE_NODE && info.count != 1) {
      throw std::runtime_error("scalar semantic node field has invalid count");
    }
    for (std::size_t value = 0; value < info.count; ++value) {
      const RoupNodeResult acquired = require_value(
          field_node(*index, value), "acquiring a semantic child node");
      try {
        convert(acquired.value);
      } catch (...) {
        discard_cleanup_result(roup_node_release(acquired.value));
        throw;
      }
      require_ok(roup_node_release(acquired.value),
                 "releasing a semantic child node");
    }
  }

  void finish() const {
    for (std::size_t index = 0; index < consumed_.size(); ++index) {
      if (!consumed_[index]) {
        throw std::runtime_error("unconverted typed field id " +
                                 std::to_string(fields_[index].id));
      }
    }
  }

private:
  FieldReader(FieldScope scope, RoupDirectiveHandle directive,
              std::size_t clause_index, RoupNodeHandle node)
      : scope_(scope), directive_(directive), clause_index_(clause_index),
        node_(node) {
    const std::size_t count = require_value(field_count(),
                                            "querying typed field count")
                                  .value;
    fields_.reserve(count);
    consumed_.assign(count, false);
    for (std::size_t index = 0; index < count; ++index) {
      fields_.push_back(
          require_value(field_info(index), "querying typed field metadata")
              .value);
    }
  }

  void consume(std::size_t index) {
    if (consumed_[index]) {
      throw std::runtime_error("typed field consumed more than once");
    }
    consumed_[index] = true;
  }

  RoupSizeResult field_count() const {
    switch (scope_) {
    case FieldScope::Clause:
      return roup_clause_field_count(directive_, clause_index_);
    case FieldScope::Parameter:
      return roup_directive_parameter_field_count(directive_);
    case FieldScope::Node:
      return roup_node_field_count(node_);
    }
    throw std::runtime_error("invalid typed field scope");
  }

  RoupFieldInfoResult field_info(std::size_t index) const {
    switch (scope_) {
    case FieldScope::Clause:
      return roup_clause_field_info(directive_, clause_index_, index);
    case FieldScope::Parameter:
      return roup_directive_parameter_field_info(directive_, index);
    case FieldScope::Node:
      return roup_node_field_info(node_, index);
    }
    throw std::runtime_error("invalid typed field scope");
  }

  RoupU32Result field_bool(std::size_t field, std::size_t value) const {
    switch (scope_) {
    case FieldScope::Clause:
      return roup_clause_field_bool(directive_, clause_index_, field, value);
    case FieldScope::Parameter:
      return roup_directive_parameter_field_bool(directive_, field, value);
    case FieldScope::Node:
      return roup_node_field_bool(node_, field, value);
    }
    throw std::runtime_error("invalid typed field scope");
  }

  RoupU32Result field_u32(std::size_t field, std::size_t value) const {
    switch (scope_) {
    case FieldScope::Clause:
      return roup_clause_field_u32(directive_, clause_index_, field, value);
    case FieldScope::Parameter:
      return roup_directive_parameter_field_u32(directive_, field, value);
    case FieldScope::Node:
      return roup_node_field_u32(node_, field, value);
    }
    throw std::runtime_error("invalid typed field scope");
  }

  std::string string_at(std::size_t field, std::size_t value) const {
    switch (scope_) {
    case FieldScope::Clause:
      return copy_string(
          [&] {
            return roup_clause_field_string_length(directive_, clause_index_,
                                                   field, value);
          },
          [&](std::uint8_t *output, std::size_t capacity) {
            return roup_clause_field_string_copy(
                directive_, clause_index_, field, value, output, capacity);
          },
          "copying a typed clause string");
    case FieldScope::Parameter:
      return copy_string(
          [&] {
            return roup_directive_parameter_field_string_length(directive_,
                                                                field, value);
          },
          [&](std::uint8_t *output, std::size_t capacity) {
            return roup_directive_parameter_field_string_copy(
                directive_, field, value, output, capacity);
          },
          "copying a typed directive-parameter string");
    case FieldScope::Node:
      return copy_string(
          [&] { return roup_node_field_string_length(node_, field, value); },
          [&](std::uint8_t *output, std::size_t capacity) {
            return roup_node_field_string_copy(node_, field, value, output,
                                               capacity);
          },
          "copying a typed semantic-node string");
    }
    throw std::runtime_error("invalid typed field scope");
  }

  RoupNodeResult field_node(std::size_t field, std::size_t value) const {
    switch (scope_) {
    case FieldScope::Clause:
      return roup_clause_field_node(directive_, clause_index_, field, value);
    case FieldScope::Parameter:
      return roup_directive_parameter_field_node(directive_, field, value);
    case FieldScope::Node:
      return roup_node_field_node(node_, field, value);
    }
    throw std::runtime_error("invalid typed field scope");
  }

  FieldScope scope_;
  RoupDirectiveHandle directive_;
  std::size_t clause_index_;
  RoupNodeHandle node_;
  std::vector<RoupFieldInfo> fields_;
  std::vector<bool> consumed_;
};

OpenMPDirectiveKind directive_kind(std::uint32_t ordinal) {
  switch (ordinal) {
#define ROUP_MAP_DIRECT_END_DIRECTIVE(Abi, Parser)                            \
  case ROUP_OMP_DIRECTIVE_##Abi:                                              \
    return OMPD_##Parser;
    ROUP_OMPPARSER_DIRECT_END_DIRECTIVES(ROUP_MAP_DIRECT_END_DIRECTIVE)
#undef ROUP_MAP_DIRECT_END_DIRECTIVE
#define ROUP_MAP_DIRECTIVE(Abi, Parser)                                        \
  case ROUP_OMP_DIRECTIVE_##Abi:                                               \
    return OMPD_##Parser;
    ROUP_OMPPARSER_DIRECTIVES(ROUP_MAP_DIRECTIVE)
#undef ROUP_MAP_DIRECTIVE
#define ROUP_MAP_END_DIRECTIVE(Abi, Parser)                                    \
  case ROUP_OMP_DIRECTIVE_##Abi:                                               \
    return OMPD_end;
    ROUP_OMPPARSER_END_DIRECTIVES(ROUP_MAP_END_DIRECTIVE)
#undef ROUP_MAP_END_DIRECTIVE
  default:
    throw std::runtime_error("unknown typed OpenMP directive ordinal " +
                             std::to_string(ordinal));
  }
}

OpenMPDirectiveKind paired_end_kind(std::uint32_t ordinal) {
  switch (ordinal) {
#define ROUP_MAP_END_DIRECTIVE(Abi, Parser)                                    \
  case ROUP_OMP_DIRECTIVE_##Abi:                                               \
    return OMPD_##Parser;
    ROUP_OMPPARSER_END_DIRECTIVES(ROUP_MAP_END_DIRECTIVE)
#undef ROUP_MAP_END_DIRECTIVE
  default:
    throw std::runtime_error(
        "typed OpenMP directive is not a representable end pair: " +
        std::to_string(ordinal));
  }
}

OpenMPClauseKind clause_kind(std::uint32_t ordinal) {
  switch (ordinal) {
#define ROUP_MAP_CLAUSE(Abi, Parser)                                           \
  case ROUP_OMP_CLAUSE_##Abi:                                                  \
    return OMPC_##Parser;
    ROUP_OMPPARSER_CLAUSES(ROUP_MAP_CLAUSE)
#undef ROUP_MAP_CLAUSE
  default:
    throw std::runtime_error("unknown typed OpenMP clause ordinal " +
                             std::to_string(ordinal));
  }
}

std::uint32_t directive_ordinal(RoupDirectiveHandle directive) {
  const RoupDirectiveKind kind =
      require_value(roup_directive_kind(directive),
                    "querying the typed OpenMP directive kind")
          .value;
  if (kind.dialect != ROUP_DIALECT_OPENMP) {
    throw std::runtime_error("ompparser received a non-OpenMP directive");
  }
  return kind.ordinal;
}

std::uint32_t clause_ordinal(RoupDirectiveHandle directive,
                             std::size_t clause) {
  const RoupClauseKind kind =
      require_value(roup_clause_kind(directive, clause),
                    "querying the typed OpenMP clause kind")
          .value;
  if (kind.dialect != ROUP_DIALECT_OPENMP) {
    throw std::runtime_error("ompparser received a non-OpenMP clause");
  }
  return kind.ordinal;
}

void record_clause(OpenMPDirective &directive, OpenMPClause *clause) {
  if (clause == nullptr) {
    throw std::runtime_error("ompparser failed to allocate a clause");
  }
  std::vector<OpenMPClause *> *order = directive.getClausesInOriginalOrder();
  if (clause->getClausePosition() != -1) {
    const int position = clause->getClausePosition();
    if (position < 0 || static_cast<std::size_t>(position) >= order->size() ||
        order->at(static_cast<std::size_t>(position)) != clause) {
      throw std::runtime_error("ompparser clause position is inconsistent");
    }
    return;
  }
  order->push_back(clause);
  clause->setClausePosition(static_cast<int>(order->size() - 1));
}

void add_leaf_values(OpenMPClause &clause,
                     const std::vector<std::string> &values) {
  for (const std::string &value : values) {
    clause.addLangExpr(value.c_str(), OMPC_CLAUSE_SEP_comma, 0, 0,
                       OMP_EXPR_PARSE_variable_list);
  }
}

std::string read_clause_item(RoupNodeHandle node) {
  const RoupNodeKind kind =
      require_value(roup_node_kind(node), "querying a clause-item node kind")
          .value;
  if (kind.family != ROUP_NODE_FAMILY_CLAUSE_ITEM) {
    throw std::runtime_error("clause item has the wrong semantic node family");
  }

  FieldReader fields = FieldReader::node(node);
  std::string value;
  switch (kind.variant) {
  case ROUP_CLAUSE_ITEM_IDENTIFIER:
    value = fields.required_string(ROUP_FIELD_NAME);
    break;
  case ROUP_CLAUSE_ITEM_VARIABLE:
    value = fields.required_string(ROUP_FIELD_VARIABLE);
    break;
  case ROUP_CLAUSE_ITEM_FORTRAN_COMMON_BLOCK:
    value = "/" + fields.required_string(ROUP_FIELD_NAME) + "/";
    break;
  case ROUP_CLAUSE_ITEM_EXPRESSION:
    value = fields.required_string(ROUP_FIELD_VALUE);
    break;
  case ROUP_CLAUSE_ITEM_LEGACY_TRAILING_SLASH:
    value = fields.required_string(ROUP_FIELD_NAME) + "/";
    break;
  case ROUP_CLAUSE_ITEM_LVALUE:
    value = fields.required_string(ROUP_FIELD_VALUE);
    break;
  default:
    throw std::runtime_error("unknown typed clause-item variant");
  }
  fields.finish();
  return value;
}

std::vector<std::string> required_clause_items(FieldReader &fields) {
  if (!fields.find(ROUP_FIELD_ITEMS).has_value()) {
    throw std::runtime_error("missing required typed clause-item field");
  }
  std::vector<std::string> values;
  fields.for_each_node(ROUP_FIELD_ITEMS, [&](RoupNodeHandle node) {
    values.push_back(read_clause_item(node));
  });
  return values;
}

std::string read_omp_locator(RoupNodeHandle node) {
  const RoupNodeKind kind =
      require_value(roup_node_kind(node), "querying an OpenMP locator kind")
          .value;
  if (kind.family != ROUP_NODE_FAMILY_OMP_LOCATOR)
    throw std::runtime_error("locator has the wrong semantic node family");
  FieldReader fields = FieldReader::node(node);
  std::string value;
  if (kind.variant == ROUP_OMP_LOCATOR_LVALUE ||
      kind.variant == ROUP_OMP_LOCATOR_POTENTIAL_LVALUE) {
    value = fields.required_string(ROUP_FIELD_VALUE);
  } else if (kind.variant == ROUP_OMP_LOCATOR_DISTRIBUTED) {
    value = fields.required_string(ROUP_FIELD_VALUE);
    const std::uint32_t policy = fields.required_u32(ROUP_FIELD_KIND);
    if (policy != ROUP_OMP_DIST_DATA_DUPLICATE)
      throw std::runtime_error("unknown typed dist_data policy");
    value += " dist_data(duplicate)";
  } else if (kind.variant == ROUP_OMP_LOCATOR_ARRAY_SHAPING) {
    const std::vector<std::string> dimensions =
        fields.required_strings(ROUP_FIELD_VALUES);
    const std::string base = fields.required_string(ROUP_FIELD_BASE);
    if (dimensions.empty() || base.empty())
      throw std::runtime_error(
          "typed array-shaping locator is missing dimensions or a base");
    value = "((";
    for (const std::string &dimension : dimensions)
      value += "[" + dimension + "]";
    value += ")" + base + ")";
    fields.for_each_node(ROUP_FIELD_ITEMS, [&](RoupNodeHandle subscript) {
      const RoupNodeKind subscript_kind =
          require_value(roup_node_kind(subscript),
                        "querying array-shaping subscript kind")
              .value;
      if (subscript_kind.family !=
          ROUP_NODE_FAMILY_OMP_ARRAY_SHAPING_SUBSCRIPT) {
        throw std::runtime_error(
            "array-shaping subscript has the wrong semantic node family");
      }
      FieldReader subscript_fields = FieldReader::node(subscript);
      value += "[";
      if (subscript_kind.variant == ROUP_OMP_ARRAY_SHAPING_INDEX) {
        value += subscript_fields.required_string(ROUP_FIELD_VALUE);
      } else if (subscript_kind.variant == ROUP_OMP_ARRAY_SHAPING_SECTION) {
        const std::optional<std::string> lower =
            subscript_fields.optional_string(ROUP_FIELD_LOWER_BOUND);
        const std::optional<std::string> length =
            subscript_fields.optional_string(ROUP_FIELD_LENGTH);
        const std::optional<std::string> stride =
            subscript_fields.optional_string(ROUP_FIELD_STRIDE);
        if (lower.has_value())
          value += *lower;
        value += ":";
        if (length.has_value())
          value += *length;
        if (stride.has_value())
          value += ":" + *stride;
      } else {
        throw std::runtime_error(
            "unknown typed array-shaping subscript variant");
      }
      subscript_fields.finish();
      value += "]";
    });
  } else if (kind.variant == ROUP_OMP_LOCATOR_FORTRAN_COMMON_BLOCK) {
    value = "/" + fields.required_string(ROUP_FIELD_NAME) + "/";
  } else if (kind.variant == ROUP_OMP_LOCATOR_ALL_MEMORY) {
    value = "omp_all_memory";
  } else {
    throw std::runtime_error("unknown typed OpenMP locator variant");
  }
  fields.finish();
  return value;
}

std::vector<std::string> required_omp_locators(FieldReader &fields) {
  if (!fields.find(ROUP_FIELD_ITEMS).has_value())
    throw std::runtime_error("missing required typed locator field");
  std::vector<std::string> values;
  fields.for_each_node(ROUP_FIELD_ITEMS, [&](RoupNodeHandle node) {
    values.push_back(read_omp_locator(node));
  });
  if (values.empty())
    throw std::runtime_error("OpenMP locator list must not be empty");
  return values;
}

std::string read_omp_count(RoupNodeHandle node) {
  const RoupNodeKind kind =
      require_value(roup_node_kind(node), "querying an OpenMP count kind")
          .value;
  if (kind.family != ROUP_NODE_FAMILY_OMP_COUNT)
    throw std::runtime_error("count has the wrong semantic node family");
  FieldReader fields = FieldReader::node(node);
  std::string value;
  if (kind.variant == ROUP_OMP_COUNT_FILL) {
    value = "omp_fill";
  } else if (kind.variant == ROUP_OMP_COUNT_EXPRESSION) {
    value = fields.required_string(ROUP_FIELD_VALUE);
  } else {
    throw std::runtime_error("unknown typed OpenMP count variant");
  }
  fields.finish();
  return value;
}

std::vector<std::string> required_omp_counts(FieldReader &fields) {
  if (!fields.find(ROUP_FIELD_VALUES).has_value())
    throw std::runtime_error("missing required typed count field");
  std::vector<std::string> values;
  fields.for_each_node(ROUP_FIELD_VALUES, [&](RoupNodeHandle node) {
    values.push_back(read_omp_count(node));
  });
  if (values.empty())
    throw std::runtime_error("OpenMP counts list must not be empty");
  return values;
}

OpenMPClause *add_simple_clause(OpenMPDirective &directive,
                                OpenMPClauseKind kind,
                                const std::vector<std::string> &values) {
  OpenMPClause *clause = directive.addOpenMPClause(static_cast<int>(kind), "");
  if (clause == nullptr) {
    throw std::runtime_error("ompparser failed to create a simple clause");
  }
  add_leaf_values(*clause, values);
  record_clause(directive, clause);
  return clause;
}

OpenMPClause *add_scalar_expression_clause(
    OpenMPDirective &directive, OpenMPClauseKind kind,
    const std::optional<std::string> &value) {
  OpenMPClause *clause =
      directive.addOpenMPClause(static_cast<int>(kind), "");
  if (clause == nullptr) {
    throw std::runtime_error(
        "ompparser failed to create a scalar-expression clause");
  }
  if (value.has_value()) {
    clause->addLangExpr(value->c_str(), OMPC_CLAUSE_SEP_space, 0, 0,
                        OMP_EXPR_PARSE_expression);
  }
  record_clause(directive, clause);
  return clause;
}

std::vector<std::string> take_one_of(FieldReader &fields,
                                     std::initializer_list<std::uint32_t> ids) {
  std::vector<std::string> values;
  bool found = false;
  for (std::uint32_t id : ids) {
    if (fields.find(id).has_value()) {
      if (found) {
        throw std::runtime_error("simple clause has multiple payload fields");
      }
      found = true;
      if (id == ROUP_FIELD_ITEMS) {
        values = required_clause_items(fields);
      } else if (id == ROUP_FIELD_VALUES || id == ROUP_FIELD_ARGUMENTS) {
        values = fields.required_strings(id);
      } else {
        values.push_back(fields.required_string(id));
      }
    }
  }
  return values;
}

void convert_simple_clause(OpenMPDirective &directive, OpenMPClauseKind kind,
                           FieldReader &fields) {
  const std::vector<std::string> values = take_one_of(
      fields, {ROUP_FIELD_VALUE, ROUP_FIELD_VALUES, ROUP_FIELD_ITEMS,
               ROUP_FIELD_ARGUMENTS, ROUP_FIELD_EVENT});
  fields.finish();
  add_simple_clause(directive, kind, values);
}

void convert_bare_clause(OpenMPDirective &directive, OpenMPClauseKind kind,
                         FieldReader &fields) {
  fields.finish();
  add_simple_clause(directive, kind, {});
}

OpenMPScheduleClauseKind schedule_kind(std::uint32_t value) {
  if (value == ROUP_OMP_SCHEDULE_STATIC)
    return OMPC_SCHEDULE_KIND_static;
  if (value == ROUP_OMP_SCHEDULE_DYNAMIC)
    return OMPC_SCHEDULE_KIND_dynamic;
  if (value == ROUP_OMP_SCHEDULE_GUIDED)
    return OMPC_SCHEDULE_KIND_guided;
  if (value == ROUP_OMP_SCHEDULE_AUTO)
    return OMPC_SCHEDULE_KIND_auto;
  if (value == ROUP_OMP_SCHEDULE_RUNTIME)
    return OMPC_SCHEDULE_KIND_runtime;
  throw std::runtime_error("unknown typed schedule kind " +
                           std::to_string(value));
}

OpenMPScheduleClauseModifier schedule_modifier(std::uint32_t value) {
  if (value == ROUP_OMP_SCHEDULE_MODIFIER_MONOTONIC)
    return OMPC_SCHEDULE_MODIFIER_monotonic;
  if (value == ROUP_OMP_SCHEDULE_MODIFIER_NONMONOTONIC)
    return OMPC_SCHEDULE_MODIFIER_nonmonotonic;
  if (value == ROUP_OMP_SCHEDULE_MODIFIER_SIMD)
    return OMPC_SCHEDULE_MODIFIER_simd;
  throw std::runtime_error("unknown typed schedule modifier " +
                           std::to_string(value));
}

void convert_schedule(OpenMPDirective &directive, FieldReader &fields) {
  const OpenMPScheduleClauseKind kind =
      schedule_kind(fields.required_u32(ROUP_FIELD_KIND));
  const std::vector<std::uint32_t> modifiers =
      fields.optional_u32s(ROUP_FIELD_MODIFIERS);
  if (modifiers.size() > 2) {
    throw std::runtime_error("ompparser supports at most two schedule modifiers");
  }
  OpenMPScheduleClauseModifier first = OMPC_SCHEDULE_MODIFIER_unspecified;
  OpenMPScheduleClauseModifier second = OMPC_SCHEDULE_MODIFIER_unspecified;
  if (!modifiers.empty())
    first = schedule_modifier(modifiers[0]);
  if (modifiers.size() == 2)
    second = schedule_modifier(modifiers[1]);
  const std::optional<std::string> chunk =
      fields.optional_string(ROUP_FIELD_CHUNK_SIZE);
  fields.finish();
  OpenMPClause *raw = OpenMPScheduleClause::addScheduleClause(
      &directive, first, second, kind,
      chunk.has_value() ? const_cast<char *>(chunk->c_str()) : nullptr);
  if (raw == nullptr) {
    throw std::runtime_error("ompparser failed to create schedule clause");
  }
  if (chunk.has_value()) {
    static_cast<OpenMPScheduleClause *>(raw)->setChunkSize(chunk->c_str());
  }
  record_clause(directive, raw);
}

OpenMPIfClauseModifier if_modifier(std::uint32_t value) {
  switch (value) {
  case ROUP_OMP_DIRECTIVE_PARALLEL: return OMPC_IF_MODIFIER_parallel;
  case ROUP_OMP_DIRECTIVE_SIMD: return OMPC_IF_MODIFIER_simd;
  case ROUP_OMP_DIRECTIVE_TASK: return OMPC_IF_MODIFIER_task;
  case ROUP_OMP_DIRECTIVE_CANCEL: return OMPC_IF_MODIFIER_cancel;
  case ROUP_OMP_DIRECTIVE_TARGET_DATA: return OMPC_IF_MODIFIER_target_data;
  case ROUP_OMP_DIRECTIVE_TARGET_ENTER_DATA:
    return OMPC_IF_MODIFIER_target_enter_data;
  case ROUP_OMP_DIRECTIVE_TARGET_EXIT_DATA:
    return OMPC_IF_MODIFIER_target_exit_data;
  case ROUP_OMP_DIRECTIVE_TARGET: return OMPC_IF_MODIFIER_target;
  case ROUP_OMP_DIRECTIVE_TARGET_UPDATE:
    return OMPC_IF_MODIFIER_target_update;
  case ROUP_OMP_DIRECTIVE_TASKLOOP: return OMPC_IF_MODIFIER_taskloop;
  case ROUP_OMP_DIRECTIVE_TEAMS: return OMPC_IF_MODIFIER_teams;
  case ROUP_OMP_DIRECTIVE_TASK_ITERATION:
    return OMPC_IF_MODIFIER_task_iteration;
  case ROUP_OMP_DIRECTIVE_TASKGRAPH: return OMPC_IF_MODIFIER_taskgraph;
  default:
    throw std::runtime_error("unsupported typed if directive-name modifier " +
                             std::to_string(value));
  }
}

void convert_if(OpenMPDirective &directive, FieldReader &fields,
                const std::optional<std::uint32_t> modifier) {
  const std::string condition =
      fields.required_string(ROUP_FIELD_CONDITION);
  fields.finish();
  OpenMPClause *clause = OpenMPIfClause::addIfClause(
      &directive,
      modifier.has_value() ? if_modifier(*modifier)
                           : OMPC_IF_MODIFIER_unspecified,
      // The third upstream parameter is a user-defined directive-name
      // modifier, not the if condition. The condition is the typed language
      // expression attached immediately below.
      nullptr);
  if (clause == nullptr) {
    throw std::runtime_error("ompparser failed to create if clause");
  }
  clause->addLangExpr(condition.c_str(), OMPC_CLAUSE_SEP_space, 0, 0,
                      OMP_EXPR_PARSE_expression);
  record_clause(directive, clause);
}

template <typename Convert>
auto required_node(FieldReader &fields, std::uint32_t id, Convert convert)
    -> decltype(convert(RoupNodeHandle{})) {
  using Result = decltype(convert(RoupNodeHandle{}));
  std::optional<Result> result;
  fields.for_each_node(id, [&](RoupNodeHandle node) {
    if (result.has_value()) {
      throw std::runtime_error("typed scalar node field has multiple values");
    }
    result = convert(node);
  });
  if (!result.has_value()) {
    throw std::runtime_error("missing required typed node field " +
                             std::to_string(id));
  }
  return std::move(*result);
}

template <typename Convert>
auto optional_node(FieldReader &fields, std::uint32_t id, Convert convert)
    -> std::optional<decltype(convert(RoupNodeHandle{}))> {
  using Result = decltype(convert(RoupNodeHandle{}));
  if (!fields.find(id).has_value()) {
    return std::nullopt;
  }
  std::optional<Result> result;
  fields.for_each_node(id, [&](RoupNodeHandle node) {
    if (result.has_value()) {
      throw std::runtime_error("typed scalar node field has multiple values");
    }
    result = convert(node);
  });
  if (!result.has_value()) {
    throw std::runtime_error("typed scalar node field is empty");
  }
  return result;
}

struct TypedMapperId {
  bool is_default;
  std::string user_name;
};

TypedMapperId read_mapper_id(RoupNodeHandle node) {
  const RoupNodeKind kind =
      require_value(roup_node_kind(node), "querying mapper identifier kind")
          .value;
  if (kind.family != ROUP_NODE_FAMILY_OMP_MAPPER_ID) {
    throw std::runtime_error(
        "mapper identifier has the wrong semantic node family");
  }
  FieldReader fields = FieldReader::node(node);
  TypedMapperId result{false, {}};
  switch (kind.variant) {
  case ROUP_OMP_MAPPER_ID_DEFAULT:
    result.is_default = true;
    break;
  case ROUP_OMP_MAPPER_ID_USER:
    result.user_name = fields.required_string(ROUP_FIELD_NAME);
    break;
  default:
    throw std::runtime_error("unknown typed mapper identifier variant");
  }
  fields.finish();
  return result;
}

std::string cpp_operator_spelling(std::uint32_t value) {
  switch (value) {
  case ROUP_OMP_CPP_OPERATOR_ADD:
    return "+";
  case ROUP_OMP_CPP_OPERATOR_SUBTRACT:
    return "-";
  case ROUP_OMP_CPP_OPERATOR_MULTIPLY:
    return "*";
  case ROUP_OMP_CPP_OPERATOR_BITWISE_AND:
    return "&";
  case ROUP_OMP_CPP_OPERATOR_BITWISE_OR:
    return "|";
  case ROUP_OMP_CPP_OPERATOR_BITWISE_XOR:
    return "^";
  case ROUP_OMP_CPP_OPERATOR_LOGICAL_AND:
    return "&&";
  case ROUP_OMP_CPP_OPERATOR_LOGICAL_OR:
    return "||";
  default:
    throw std::runtime_error("unknown typed C++ operator ordinal " +
                             std::to_string(value));
  }
}

std::string read_cpp_operator_qualifier(RoupNodeHandle node) {
  const RoupNodeKind kind =
      require_value(roup_node_kind(node),
                    "querying C++ operator qualifier node kind")
          .value;
  if (kind.family != ROUP_NODE_FAMILY_OMP_CPP_OPERATOR_QUALIFIER) {
    throw std::runtime_error(
        "C++ operator qualifier has the wrong semantic node family");
  }
  FieldReader fields = FieldReader::node(node);
  switch (kind.variant) {
  case ROUP_OMP_CPP_OPERATOR_QUALIFIER_NAME:
  case ROUP_OMP_CPP_OPERATOR_QUALIFIER_TEMPLATE_ID: {
    std::string spelling = fields.required_string(ROUP_FIELD_NAME);
    fields.finish();
    return spelling;
  }
  default:
    throw std::runtime_error("unknown typed C++ operator qualifier variant");
  }
}

std::string read_id_expression(RoupNodeHandle node) {
  const RoupNodeKind kind =
      require_value(roup_node_kind(node), "querying id-expression node kind")
          .value;
  if (kind.family != ROUP_NODE_FAMILY_OMP_ID_EXPRESSION) {
    throw std::runtime_error("id-expression has the wrong semantic node family");
  }
  FieldReader fields = FieldReader::node(node);
  switch (kind.variant) {
  case ROUP_OMP_ID_EXPRESSION_NAME:
  case ROUP_OMP_ID_EXPRESSION_CPP_TEMPLATE_ID: {
    std::string spelling = fields.required_string(ROUP_FIELD_NAME);
    fields.finish();
    return spelling;
  }
  case ROUP_OMP_ID_EXPRESSION_CPP_OPERATOR_FUNCTION: {
    const bool global = fields.optional_bool(ROUP_FIELD_GLOBAL).value_or(false);
    const std::uint32_t operation = fields.required_u32(ROUP_FIELD_OPERATOR);
    const std::optional<std::string> qualifier = optional_node(
        fields, ROUP_FIELD_QUALIFIER, read_cpp_operator_qualifier);
    fields.finish();
    std::string spelling = global ? "::" : "";
    if (qualifier.has_value()) {
      spelling += *qualifier;
      spelling += "::";
    }
    spelling += "operator";
    spelling += cpp_operator_spelling(operation);
    return spelling;
  }
  default:
    throw std::runtime_error("unknown typed id-expression variant");
  }
}

struct TypedReductionIdentifier {
  OpenMPReductionClauseIdentifier kind;
  std::string user_spelling;
};

std::string
reduction_identifier_spelling(const TypedReductionIdentifier &identifier);

TypedReductionIdentifier read_reduction_identifier(RoupNodeHandle node) {
  const RoupNodeKind kind =
      require_value(roup_node_kind(node),
                    "querying OpenMP reduction identifier node kind")
          .value;
  if (kind.family != ROUP_NODE_FAMILY_OMP_IDENTIFIER) {
    throw std::runtime_error(
        "OpenMP reduction identifier has the wrong semantic node family");
  }
  FieldReader fields = FieldReader::node(node);
  TypedReductionIdentifier result{OMPC_REDUCTION_IDENTIFIER_unknown, {}};
  switch (kind.variant) {
  case ROUP_OMP_IDENTIFIER_ADD:
    result.kind = OMPC_REDUCTION_IDENTIFIER_plus;
    break;
  case ROUP_OMP_IDENTIFIER_SUBTRACT:
    result.kind = OMPC_REDUCTION_IDENTIFIER_minus;
    break;
  case ROUP_OMP_IDENTIFIER_MULTIPLY:
    result.kind = OMPC_REDUCTION_IDENTIFIER_mul;
    break;
  case ROUP_OMP_IDENTIFIER_BITWISE_AND:
    result.kind = OMPC_REDUCTION_IDENTIFIER_bitand;
    break;
  case ROUP_OMP_IDENTIFIER_BITWISE_OR:
    result.kind = OMPC_REDUCTION_IDENTIFIER_bitor;
    break;
  case ROUP_OMP_IDENTIFIER_BITWISE_XOR:
    result.kind = OMPC_REDUCTION_IDENTIFIER_bitxor;
    break;
  case ROUP_OMP_IDENTIFIER_LOGICAL_AND:
    result.kind = OMPC_REDUCTION_IDENTIFIER_logand;
    break;
  case ROUP_OMP_IDENTIFIER_FORTRAN_LOGICAL_AND:
    result.kind = OMPC_REDUCTION_IDENTIFIER_user;
    result.user_spelling = ".and.";
    break;
  case ROUP_OMP_IDENTIFIER_LOGICAL_OR:
    result.kind = OMPC_REDUCTION_IDENTIFIER_logor;
    break;
  case ROUP_OMP_IDENTIFIER_FORTRAN_LOGICAL_OR:
    result.kind = OMPC_REDUCTION_IDENTIFIER_user;
    result.user_spelling = ".or.";
    break;
  case ROUP_OMP_IDENTIFIER_FORTRAN_LOGICAL_EQV:
    result.kind = OMPC_REDUCTION_IDENTIFIER_user;
    result.user_spelling = ".eqv.";
    break;
  case ROUP_OMP_IDENTIFIER_FORTRAN_LOGICAL_NEQV:
    result.kind = OMPC_REDUCTION_IDENTIFIER_user;
    result.user_spelling = ".neqv.";
    break;
  case ROUP_OMP_IDENTIFIER_FORTRAN_MAX:
    result.kind = OMPC_REDUCTION_IDENTIFIER_max;
    break;
  case ROUP_OMP_IDENTIFIER_FORTRAN_MIN:
    result.kind = OMPC_REDUCTION_IDENTIFIER_min;
    break;
  case ROUP_OMP_IDENTIFIER_FORTRAN_IAND:
    result.kind = OMPC_REDUCTION_IDENTIFIER_user;
    result.user_spelling = "iand";
    break;
  case ROUP_OMP_IDENTIFIER_FORTRAN_IOR:
    result.kind = OMPC_REDUCTION_IDENTIFIER_user;
    result.user_spelling = "ior";
    break;
  case ROUP_OMP_IDENTIFIER_FORTRAN_IEOR:
    result.kind = OMPC_REDUCTION_IDENTIFIER_user;
    result.user_spelling = "ieor";
    break;
  case ROUP_OMP_IDENTIFIER_NAME:
    result.kind = OMPC_REDUCTION_IDENTIFIER_user;
    result.user_spelling =
        required_node(fields, ROUP_FIELD_VALUE, read_id_expression);
    break;
  case ROUP_OMP_IDENTIFIER_FORTRAN_DEFINED_OPERATOR:
    result.kind = OMPC_REDUCTION_IDENTIFIER_user;
    result.user_spelling =
        "." + fields.required_string(ROUP_FIELD_NAME) + ".";
    break;
  default:
    throw std::runtime_error("unknown typed OpenMP identifier variant");
  }
  fields.finish();
  return result;
}

void convert_reduction(OpenMPDirective &directive, OpenMPClauseKind clause_kind,
                       FieldReader &fields,
                       bool original_uses_positional_syntax = false) {
  std::vector<std::uint32_t> modifier_variants;
  std::optional<std::uint32_t> original_sharing;
  fields.for_each_node(ROUP_FIELD_MODIFIERS, [&](RoupNodeHandle node) {
    const RoupNodeKind kind =
        require_value(roup_node_kind(node), "querying reduction modifier kind")
            .value;
    if (kind.family != ROUP_NODE_FAMILY_REDUCTION_MODIFIER) {
      throw std::runtime_error("reduction modifier has the wrong node family");
    }
    FieldReader node_fields = FieldReader::node(node);
    if (kind.variant == ROUP_REDUCTION_MODIFIER_ORIGINAL) {
      original_sharing = node_fields.required_u32(ROUP_FIELD_KIND);
    } else if (kind.variant != ROUP_REDUCTION_MODIFIER_TASK &&
               kind.variant != ROUP_REDUCTION_MODIFIER_INSCAN &&
               kind.variant != ROUP_REDUCTION_MODIFIER_DEFAULT) {
      throw std::runtime_error("unknown typed reduction modifier variant");
    }
    node_fields.finish();
    modifier_variants.push_back(kind.variant);
  });
  if (modifier_variants.size() > 1) {
    throw std::runtime_error(
        "ompparser cannot represent multiple reduction modifiers");
  }
  OpenMPReductionClauseModifier modifier = OMPC_REDUCTION_MODIFIER_unspecified;
  if (!modifier_variants.empty()) {
    switch (modifier_variants[0]) {
    case ROUP_REDUCTION_MODIFIER_TASK:
      modifier = OMPC_REDUCTION_MODIFIER_task;
      break;
    case ROUP_REDUCTION_MODIFIER_INSCAN:
      modifier = OMPC_REDUCTION_MODIFIER_inscan;
      break;
    case ROUP_REDUCTION_MODIFIER_DEFAULT:
      modifier = OMPC_REDUCTION_MODIFIER_default;
      break;
    case ROUP_REDUCTION_MODIFIER_ORIGINAL:
      modifier = OMPC_REDUCTION_MODIFIER_unknown;
      break;
    default:
      throw std::runtime_error("unknown typed reduction modifier variant");
    }
  }
  const TypedReductionIdentifier operation = required_node(
      fields, ROUP_FIELD_OPERATOR, read_reduction_identifier);
  const std::vector<std::string> items =
      required_clause_items(fields);
  fields.finish();

  const OpenMPReductionClauseIdentifier common_identifier = operation.kind;
  char *user_identifier = operation.user_spelling.empty()
                              ? nullptr
                              : const_cast<char *>(operation.user_spelling.c_str());
  OpenMPClause *raw = nullptr;
  if (clause_kind == OMPC_reduction) {
    raw = OpenMPReductionClause::addReductionClause(
        &directive, modifier, common_identifier, nullptr, user_identifier);
    auto *reduction = dynamic_cast<OpenMPReductionClause *>(raw);
    if (reduction == nullptr)
      throw std::runtime_error("ompparser returned the wrong reduction class");
    for (const std::string &item : items)
      reduction->addOperand(item);
    if (original_sharing.has_value()) {
      const char *sharing = nullptr;
      if (*original_sharing == ROUP_OMP_ORIGINAL_SHARING_DEFAULT) {
        sharing = "default";
      } else if (*original_sharing == ROUP_OMP_ORIGINAL_SHARING_PRIVATE) {
        sharing = "private";
      } else if (*original_sharing == ROUP_OMP_ORIGINAL_SHARING_SHARED) {
        sharing = "shared";
      } else {
        throw std::runtime_error("unknown typed original sharing kind");
      }
      const std::string spelling =
          original_uses_positional_syntax
              ? "original(" + std::string(sharing) + ")"
              : "original(sharing=" + std::string(sharing) + ")";
      reduction->setUserDefinedModifier(spelling.c_str());
    }
  } else {
    if (!modifier_variants.empty()) {
      throw std::runtime_error(
          "ompparser cannot represent modifiers on this reduction clause");
    }
    if (clause_kind == OMPC_in_reduction) {
      const auto identifier = static_cast<OpenMPInReductionClauseIdentifier>(
          static_cast<int>(common_identifier));
      raw = OpenMPInReductionClause::addInReductionClause(
          &directive, identifier, user_identifier);
      auto *reduction = dynamic_cast<OpenMPInReductionClause *>(raw);
      if (reduction == nullptr) {
        throw std::runtime_error(
            "ompparser returned the wrong in_reduction class");
      }
      for (const std::string &item : items)
        reduction->addOperand(item);
    } else if (clause_kind == OMPC_task_reduction) {
      const auto identifier = static_cast<OpenMPTaskReductionClauseIdentifier>(
          static_cast<int>(common_identifier));
      raw = OpenMPTaskReductionClause::addTaskReductionClause(
          &directive, identifier, user_identifier);
      auto *reduction = dynamic_cast<OpenMPTaskReductionClause *>(raw);
      if (reduction == nullptr) {
        throw std::runtime_error(
            "ompparser returned the wrong task_reduction class");
      }
      for (const std::string &item : items)
        reduction->addOperand(item);
    } else {
      throw std::runtime_error("invalid reduction clause kind");
    }
  }
  if (raw == nullptr)
    throw std::runtime_error("ompparser failed to create reduction clause");
  record_clause(directive, raw);
}

void convert_induction(OpenMPDirective &directive, FieldReader &fields) {
  const std::optional<std::uint32_t> modifier =
      fields.optional_u32(ROUP_FIELD_MODIFIER);
  const std::string step = fields.required_string(ROUP_FIELD_STEP);
  const TypedReductionIdentifier identifier = required_node(
      fields, ROUP_FIELD_IDENTIFIER, read_reduction_identifier);
  const std::vector<std::string> items = required_clause_items(fields);
  fields.finish();

  OpenMPClause *raw =
      directive.addOpenMPClause(static_cast<int>(OMPC_induction), "");
  auto *clause = dynamic_cast<OpenMPInductionClause *>(raw);
  if (clause == nullptr)
    throw std::runtime_error("ompparser failed to create induction clause");
  if (modifier.has_value()) {
    if (*modifier != ROUP_OMP_INDUCTION_RELAXED &&
        *modifier != ROUP_OMP_INDUCTION_STRICT) {
      throw std::runtime_error("unknown typed induction modifier " +
                               std::to_string(*modifier));
    }
    throw std::runtime_error(
        "ompparser cannot represent a typed induction modifier");
  }
  clause->addStepExpression(step.c_str());
  std::string joined_items;
  for (const std::string &item : items) {
    if (!joined_items.empty())
      joined_items += ", ";
    joined_items += item;
  }
  clause->addBinding(reduction_identifier_spelling(identifier).c_str(),
                     joined_items.c_str());
  record_clause(directive, raw);
}

std::string apply_loop_modifier_label(RoupNodeHandle node) {
  const RoupNodeKind kind =
      require_value(roup_node_kind(node), "querying apply modifier kind").value;
  if (kind.family != ROUP_NODE_FAMILY_OMP_APPLY_MODIFIER)
    throw std::runtime_error("apply modifier has the wrong node family");
  const char *name = nullptr;
  switch (kind.variant) {
  case ROUP_OMP_APPLY_FUSED: name = "fused"; break;
  case ROUP_OMP_APPLY_GRID: name = "grid"; break;
  case ROUP_OMP_APPLY_IDENTITY: name = "identity"; break;
  case ROUP_OMP_APPLY_INTERCHANGED: name = "interchanged"; break;
  case ROUP_OMP_APPLY_INTRATILE: name = "intratile"; break;
  case ROUP_OMP_APPLY_OFFSETS: name = "offsets"; break;
  case ROUP_OMP_APPLY_REVERSED: name = "reversed"; break;
  case ROUP_OMP_APPLY_SPLIT: name = "split"; break;
  case ROUP_OMP_APPLY_UNROLLED: name = "unrolled"; break;
  default:
    throw std::runtime_error("unknown typed apply modifier " +
                             std::to_string(kind.variant));
  }
  FieldReader fields = FieldReader::node(node);
  const std::vector<std::string> indices =
      fields.required_strings(ROUP_FIELD_INDICES);
  fields.finish();
  std::string label(name);
  if (!indices.empty()) {
    label.push_back('(');
    for (std::size_t index = 0; index < indices.size(); ++index) {
      if (index != 0)
        label += ", ";
      label += indices[index];
    }
    label.push_back(')');
  }
  return label;
}

OpenMPClauseSeparator source_clause_separator(std::uint32_t value);

void populate_apply_clause(OpenMPApplyClause &target, FieldReader fields);

void append_applied_directive(OpenMPApplyClause &target, RoupNodeHandle node,
                              OpenMPClauseSeparator separator) {
  const RoupNodeKind node_kind =
      require_value(roup_node_kind(node), "querying applied directive kind")
          .value;
  if (node_kind.family != ROUP_NODE_FAMILY_OMP_DIRECTIVE)
    throw std::runtime_error("applied directive has the wrong node family");

  const OpenMPDirectiveKind kind = directive_kind(node_kind.variant);
  OpenMPApplyTransformKind transform = OMPC_APPLY_TRANSFORM_unknown;
  std::string argument;
  bool saw_primary_clause = false;
  std::vector<std::pair<std::unique_ptr<OpenMPApplyClause>,
                        OpenMPClauseSeparator>>
      nested;

  FieldReader fields = FieldReader::node(node);
  (void)fields.optional_u32(ROUP_FIELD_SOURCE_ALIAS);
  if (fields.find(ROUP_FIELD_PARAMETER).has_value())
    throw std::runtime_error(
        "ompparser cannot represent parameters on an applied directive");
  fields.for_each_node(ROUP_FIELD_CLAUSES, [&](RoupNodeHandle clause_node) {
    const RoupNodeKind clause_node_kind =
        require_value(roup_node_kind(clause_node),
                      "querying applied directive clause kind")
            .value;
    if (clause_node_kind.family != ROUP_NODE_FAMILY_OMP_CLAUSE)
      throw std::runtime_error(
          "applied directive clause has the wrong node family");
    const OpenMPClauseKind clause_kind =
        ::clause_kind(clause_node_kind.variant);
    FieldReader clause_fields = FieldReader::node(clause_node);
    const OpenMPClauseSeparator clause_separator = source_clause_separator(
        clause_fields.required_u32(ROUP_FIELD_PRECEDING_SEPARATOR));
    if (clause_fields.optional_u32(ROUP_FIELD_SOURCE_ALIAS).has_value() ||
        clause_fields
            .optional_u32(ROUP_FIELD_DIRECTIVE_NAME_MODIFIER)
            .has_value()) {
      throw std::runtime_error(
          "applied directive transform carries unsupported source metadata");
    }
    if (clause_kind == OMPC_apply) {
      auto child = std::make_unique<OpenMPApplyClause>();
      populate_apply_clause(*child, std::move(clause_fields));
      nested.emplace_back(std::move(child), clause_separator);
      return;
    }
    if (kind == OMPD_unroll && clause_kind == OMPC_partial) {
      if (saw_primary_clause)
        throw std::runtime_error(
            "applied unroll directive has conflicting modifiers");
      transform = OMPC_APPLY_TRANSFORM_unroll_partial;
      argument = clause_fields
                     .optional_string(ROUP_FIELD_UNROLL_FACTOR)
                     .value_or(std::string());
      clause_fields.finish();
      saw_primary_clause = true;
      return;
    }
    if (kind == OMPD_unroll && clause_kind == OMPC_full) {
      if (saw_primary_clause)
        throw std::runtime_error(
            "applied unroll directive has conflicting modifiers");
      if (clause_fields
              .optional_string(ROUP_FIELD_FULLY_UNROLL)
              .has_value())
        throw std::runtime_error(
            "ompparser cannot represent a conditional full unroll transform");
      clause_fields.finish();
      transform = OMPC_APPLY_TRANSFORM_unroll_full;
      saw_primary_clause = true;
      return;
    }
    if (kind == OMPD_tile && clause_kind == OMPC_sizes) {
      if (saw_primary_clause)
        throw std::runtime_error(
            "applied tile directive has multiple sizes clauses");
      const std::vector<std::string> sizes =
          clause_fields.required_strings(ROUP_FIELD_VALUES);
      clause_fields.finish();
      for (const std::string &size : sizes) {
        if (!argument.empty())
          argument += ", ";
        argument += size;
      }
      transform = OMPC_APPLY_TRANSFORM_tile_sizes;
      saw_primary_clause = true;
      return;
    }
    throw std::runtime_error(
        "unsupported clause on a typed applied directive");
  });
  fields.finish();

  if (!saw_primary_clause) {
    switch (kind) {
    case OMPD_unroll: transform = OMPC_APPLY_TRANSFORM_unroll; break;
    case OMPD_reverse: transform = OMPC_APPLY_TRANSFORM_reverse; break;
    case OMPD_interchange:
      transform = OMPC_APPLY_TRANSFORM_interchange;
      break;
    case OMPD_nothing: transform = OMPC_APPLY_TRANSFORM_nothing; break;
    default:
      throw std::runtime_error(
          "ompparser cannot represent this typed applied directive kind");
    }
  }
  target.addTransformation(transform, argument, separator);
  for (auto &child : nested)
    target.addNestedApply(child.first.release(), child.second);
}

void populate_apply_clause(OpenMPApplyClause &target, FieldReader fields) {
  const std::optional<std::string> label = optional_node(
      fields, ROUP_FIELD_LOOP_MODIFIER, apply_loop_modifier_label);
  if (label.has_value())
    target.setLabel(*label);
  std::size_t count = 0;
  fields.for_each_node(
      ROUP_FIELD_APPLIED_DIRECTIVES, [&](RoupNodeHandle directive_node) {
        append_applied_directive(target, directive_node,
                                 OMPC_CLAUSE_SEP_comma);
        ++count;
      });
  fields.finish();
  if (count == 0)
    throw std::runtime_error("apply clause has no applied directives");
}

void convert_apply(OpenMPDirective &directive, FieldReader &fields) {
  OpenMPClause *raw =
      directive.addOpenMPClause(static_cast<int>(OMPC_apply), "");
  auto *clause = dynamic_cast<OpenMPApplyClause *>(raw);
  if (clause == nullptr)
    throw std::runtime_error("ompparser failed to create apply clause");
  populate_apply_clause(*clause, std::move(fields));
  record_clause(directive, raw);
}

void convert_firstprivate(
    OpenMPDirective &directive, FieldReader &fields,
    const std::optional<std::uint32_t> directive_name_modifier) {
  const std::optional<std::uint32_t> modifier =
      fields.optional_u32(ROUP_FIELD_MODIFIER);
  const std::vector<std::string> items =
      required_clause_items(fields);
  fields.finish();

  OpenMPClause *raw =
      directive.addOpenMPClause(static_cast<int>(OMPC_firstprivate), "");
  auto *firstprivate = dynamic_cast<OpenMPFirstprivateClause *>(raw);
  if (firstprivate == nullptr) {
    throw std::runtime_error("ompparser failed to create firstprivate clause");
  }
  firstprivate->setSaved(modifier.has_value());
  if (modifier.has_value()) {
    if (*modifier != ROUP_OMP_FIRSTPRIVATE_SAVED) {
      throw std::runtime_error("unknown typed firstprivate modifier " +
                               std::to_string(*modifier));
    }
  }
  if (directive_name_modifier.has_value())
    firstprivate->setCurrentDirectiveNameModifier(
        directive_kind(*directive_name_modifier));
  add_leaf_values(*firstprivate, items);
  firstprivate->clearCurrentDirectiveNameModifier();
  record_clause(directive, raw);
}

struct TypedIterator {
  std::string type_name;
  std::string variable;
  std::string start;
  std::string end;
  std::string step;
};

std::vector<TypedIterator> read_iterators(FieldReader &fields,
                                          std::uint32_t field_id) {
  std::vector<TypedIterator> result;
  fields.for_each_node(field_id, [&](RoupNodeHandle node) {
    const RoupNodeKind kind =
        require_value(roup_node_kind(node), "querying iterator node kind")
            .value;
    if (kind.family != ROUP_NODE_FAMILY_DEPEND_ITERATOR ||
        kind.variant != ROUP_NODE_RECORD) {
      throw std::runtime_error("iterator has the wrong semantic node kind");
    }
    FieldReader item = FieldReader::node(node);
    TypedIterator converted;
    converted.type_name =
        item.optional_string(ROUP_FIELD_TYPE_NAME).value_or(std::string());
    converted.variable = item.required_string(ROUP_FIELD_VARIABLE);
    converted.start = item.required_string(ROUP_FIELD_START);
    converted.end = item.required_string(ROUP_FIELD_END);
    converted.step =
        item.optional_string(ROUP_FIELD_STEP).value_or(std::string());
    item.finish();
    result.push_back(std::move(converted));
  });
  return result;
}

OpenMPDefaultClauseKind default_kind(std::uint32_t value) {
  switch (value) {
  case ROUP_OMP_DEFAULT_SHARED:
    return OMPC_DEFAULT_shared;
  case ROUP_OMP_DEFAULT_NONE:
    return OMPC_DEFAULT_none;
  case ROUP_OMP_DEFAULT_PRIVATE:
    return OMPC_DEFAULT_private;
  case ROUP_OMP_DEFAULT_FIRSTPRIVATE:
    return OMPC_DEFAULT_firstprivate;
  default:
    throw std::runtime_error("unknown typed default kind " +
                             std::to_string(value));
  }
}

OpenMPDefaultmapClauseBehavior
defaultmap_behavior(std::uint32_t value) {
  switch (value) {
  case ROUP_OMP_DEFAULTMAP_ALLOC:
    return OMPC_DEFAULTMAP_BEHAVIOR_alloc;
  case ROUP_OMP_DEFAULTMAP_TO:
    return OMPC_DEFAULTMAP_BEHAVIOR_to;
  case ROUP_OMP_DEFAULTMAP_FROM:
    return OMPC_DEFAULTMAP_BEHAVIOR_from;
  case ROUP_OMP_DEFAULTMAP_TOFROM:
    return OMPC_DEFAULTMAP_BEHAVIOR_tofrom;
  case ROUP_OMP_DEFAULTMAP_FIRSTPRIVATE:
    return OMPC_DEFAULTMAP_BEHAVIOR_firstprivate;
  case ROUP_OMP_DEFAULTMAP_NONE:
    return OMPC_DEFAULTMAP_BEHAVIOR_none;
  case ROUP_OMP_DEFAULTMAP_DEFAULT:
    return OMPC_DEFAULTMAP_BEHAVIOR_default;
  case ROUP_OMP_DEFAULTMAP_PRESENT:
    return OMPC_DEFAULTMAP_BEHAVIOR_present;
  default:
    throw std::runtime_error(
        "ompparser cannot represent typed defaultmap behavior " +
        std::to_string(value));
  }
}

OpenMPDefaultmapClauseCategory
defaultmap_category(std::uint32_t value) {
  switch (value) {
  case ROUP_OMP_DEFAULTMAP_CATEGORY_SCALAR:
    return OMPC_DEFAULTMAP_CATEGORY_scalar;
  case ROUP_OMP_DEFAULTMAP_CATEGORY_AGGREGATE:
    return OMPC_DEFAULTMAP_CATEGORY_aggregate;
  case ROUP_OMP_DEFAULTMAP_CATEGORY_POINTER:
    return OMPC_DEFAULTMAP_CATEGORY_pointer;
  case ROUP_OMP_DEFAULTMAP_CATEGORY_ALL:
    return OMPC_DEFAULTMAP_CATEGORY_all;
  case ROUP_OMP_DEFAULTMAP_CATEGORY_ALLOCATABLE:
    return OMPC_DEFAULTMAP_CATEGORY_allocatable;
  default:
    throw std::runtime_error("unknown typed defaultmap category " +
                             std::to_string(value));
  }
}

OpenMPProcBindClauseKind proc_bind_kind(std::uint32_t value) {
  switch (value) {
  case ROUP_OMP_PROC_BIND_CLOSE:
    return OMPC_PROC_BIND_close;
  case ROUP_OMP_PROC_BIND_SPREAD:
    return OMPC_PROC_BIND_spread;
  case ROUP_OMP_PROC_BIND_PRIMARY:
    return OMPC_PROC_BIND_primary;
  default:
    throw std::runtime_error("unknown typed proc_bind kind " +
                             std::to_string(value));
  }
}

OpenMPBindClauseBinding bind_kind(std::uint32_t value) {
  switch (value) {
  case ROUP_OMP_BIND_TEAMS:
    return OMPC_BIND_teams;
  case ROUP_OMP_BIND_PARALLEL:
    return OMPC_BIND_parallel;
  case ROUP_OMP_BIND_THREAD:
    return OMPC_BIND_thread;
  default:
    throw std::runtime_error("unknown typed bind kind " +
                             std::to_string(value));
  }
}

OpenMPOrderClauseModifier order_modifier(std::uint32_t value) {
  switch (value) {
  case ROUP_OMP_ORDER_REPRODUCIBLE:
    return OMPC_ORDER_MODIFIER_reproducible;
  case ROUP_OMP_ORDER_UNCONSTRAINED:
    return OMPC_ORDER_MODIFIER_unconstrained;
  default:
    throw std::runtime_error("unknown typed order modifier " +
                             std::to_string(value));
  }
}

OpenMPOrderClauseKind order_kind(std::uint32_t value) {
  if (value == ROUP_OMP_ORDER_CONCURRENT)
    return OMPC_ORDER_concurrent;
  throw std::runtime_error("unknown typed order kind " +
                           std::to_string(value));
}

OpenMPLinearClauseModifier linear_modifier(std::uint32_t value) {
  switch (value) {
  case ROUP_OMP_LINEAR_VAL:
    return OMPC_LINEAR_MODIFIER_val;
  case ROUP_OMP_LINEAR_REF:
    return OMPC_LINEAR_MODIFIER_ref;
  case ROUP_OMP_LINEAR_UVAL:
    return OMPC_LINEAR_MODIFIER_uval;
  default:
    throw std::runtime_error("unknown typed linear modifier " +
                             std::to_string(value));
  }
}

OpenMPDistScheduleClauseKind dist_schedule_kind(std::uint32_t value) {
  if (value == ROUP_OMP_SCHEDULE_STATIC)
    return OMPC_DIST_SCHEDULE_KIND_static;
  throw std::runtime_error("unknown typed dist_schedule kind " +
                           std::to_string(value));
}

OpenMPDeviceTypeClauseKind device_type_kind(std::uint32_t value) {
  switch (value) {
  case ROUP_OMP_DEVICE_TYPE_HOST:
    return OMPC_DEVICE_TYPE_host;
  case ROUP_OMP_DEVICE_TYPE_NOHOST:
    return OMPC_DEVICE_TYPE_nohost;
  case ROUP_OMP_DEVICE_TYPE_ANY:
    return OMPC_DEVICE_TYPE_any;
  default:
    throw std::runtime_error("unknown typed device_type kind " +
                             std::to_string(value));
  }
}

OpenMPDependClauseType dependence_type(std::uint32_t value) {
  switch (value) {
  case ROUP_OMP_DEPEND_IN: return OMPC_DEPENDENCE_TYPE_in;
  case ROUP_OMP_DEPEND_OUT: return OMPC_DEPENDENCE_TYPE_out;
  case ROUP_OMP_DEPEND_INOUT: return OMPC_DEPENDENCE_TYPE_inout;
  case ROUP_OMP_DEPEND_INOUTSET: return OMPC_DEPENDENCE_TYPE_inoutset;
  case ROUP_OMP_DEPEND_MUTEXINOUTSET:
    return OMPC_DEPENDENCE_TYPE_mutexinoutset;
  case ROUP_OMP_DEPEND_DEPOBJ: return OMPC_DEPENDENCE_TYPE_depobj;
  default:
    throw std::runtime_error("unknown typed dependence type " +
                             std::to_string(value));
  }
}

OpenMPDepobjUpdateClauseDependeceType
depobj_update_type(std::uint32_t value) {
  switch (value) {
  case ROUP_OMP_DEPOBJ_UPDATE_IN:
    return OMPC_DEPOBJ_UPDATE_DEPENDENCE_TYPE_in;
  case ROUP_OMP_DEPOBJ_UPDATE_OUT:
    return OMPC_DEPOBJ_UPDATE_DEPENDENCE_TYPE_out;
  case ROUP_OMP_DEPOBJ_UPDATE_INOUT:
    return OMPC_DEPOBJ_UPDATE_DEPENDENCE_TYPE_inout;
  case ROUP_OMP_DEPOBJ_UPDATE_INOUTSET:
    return OMPC_DEPOBJ_UPDATE_DEPENDENCE_TYPE_inoutset;
  case ROUP_OMP_DEPOBJ_UPDATE_MUTEXINOUTSET:
    return OMPC_DEPOBJ_UPDATE_DEPENDENCE_TYPE_mutexinoutset;
  default:
    throw std::runtime_error("unknown typed depobj update type " +
                             std::to_string(value));
  }
}

OpenMPMapClauseType map_type(std::uint32_t value) {
  switch (value) {
  case ROUP_OMP_MAP_TO: return OMPC_MAP_TYPE_to;
  case ROUP_OMP_MAP_FROM: return OMPC_MAP_TYPE_from;
  case ROUP_OMP_MAP_TOFROM: return OMPC_MAP_TYPE_tofrom;
  case ROUP_OMP_MAP_STORAGE: return OMPC_MAP_TYPE_storage;
  default:
    throw std::runtime_error("unknown typed map type " +
                             std::to_string(value));
  }
}

OpenMPMapClauseType map_type_with_source_spelling(
    const std::optional<std::uint32_t> type, std::uint32_t spelling) {
  switch (spelling) {
  case ROUP_OMP_MAP_TYPE_SPELLING_CANONICAL:
    return type.has_value() ? map_type(*type) : OMPC_MAP_TYPE_unspecified;
  case ROUP_OMP_MAP_TYPE_SPELLING_ALLOC:
    if (type != ROUP_OMP_MAP_STORAGE)
      break;
    return OMPC_MAP_TYPE_alloc;
  case ROUP_OMP_MAP_TYPE_SPELLING_RELEASE:
    if (type != ROUP_OMP_MAP_STORAGE)
      break;
    return OMPC_MAP_TYPE_release;
  case ROUP_OMP_MAP_TYPE_SPELLING_DELETE:
    if (type != ROUP_OMP_MAP_STORAGE)
      break;
    return OMPC_MAP_TYPE_delete;
  default:
    throw std::runtime_error("unknown typed map-type source spelling " +
                             std::to_string(spelling));
  }
  throw std::runtime_error(
      "typed map semantics disagree with the map-type source spelling");
}

OpenMPMapClauseModifier map_modifier(std::uint32_t value) {
  switch (value) {
  case ROUP_OMP_MAP_MODIFIER_ALWAYS: return OMPC_MAP_MODIFIER_always;
  case ROUP_OMP_MAP_MODIFIER_CLOSE: return OMPC_MAP_MODIFIER_close;
  case ROUP_OMP_MAP_MODIFIER_PRESENT: return OMPC_MAP_MODIFIER_present;
  case ROUP_OMP_MAP_MODIFIER_SELF: return OMPC_MAP_MODIFIER_self;
  case ROUP_OMP_MAP_MODIFIER_ITERATOR: return OMPC_MAP_MODIFIER_iterator;
  default:
    throw std::runtime_error("ompparser cannot represent typed map modifier " +
                             std::to_string(value));
  }
}

OpenMPMapClauseRefModifier map_ref_modifier(std::uint32_t value) {
  switch (value) {
  case ROUP_OMP_MAP_MODIFIER_REF_POINTEE: return OMPC_MAP_REF_MODIFIER_ref_ptee;
  case ROUP_OMP_MAP_MODIFIER_REF_POINTER: return OMPC_MAP_REF_MODIFIER_ref_ptr;
  case ROUP_OMP_MAP_MODIFIER_REF_POINTER_AND_POINTEE:
    return OMPC_MAP_REF_MODIFIER_ref_ptr_ptee;
  default:
    throw std::runtime_error("unknown typed map reference modifier " +
                             std::to_string(value));
  }
}

struct TypedAllocator {
  std::uint32_t kind;
  std::string custom_name;
};

TypedAllocator read_allocator_kind(RoupNodeHandle node) {
  const RoupNodeKind kind =
      require_value(roup_node_kind(node), "querying allocator-kind node")
          .value;
  if (kind.family != ROUP_NODE_FAMILY_OMP_ALLOCATOR_KIND) {
    throw std::runtime_error("allocator has the wrong semantic node family");
  }
  FieldReader fields = FieldReader::node(node);
  TypedAllocator result{kind.variant, {}};
  switch (kind.variant) {
  case ROUP_OMP_ALLOCATOR_NULL:
  case ROUP_OMP_ALLOCATOR_DEFAULT:
  case ROUP_OMP_ALLOCATOR_LARGE_CAP:
  case ROUP_OMP_ALLOCATOR_CONST:
  case ROUP_OMP_ALLOCATOR_HIGH_BW:
  case ROUP_OMP_ALLOCATOR_LOW_LAT:
  case ROUP_OMP_ALLOCATOR_CGROUP:
  case ROUP_OMP_ALLOCATOR_PTEAM:
  case ROUP_OMP_ALLOCATOR_THREAD:
    break;
  case ROUP_OMP_ALLOCATOR_CUSTOM:
    result.custom_name = fields.required_string(ROUP_FIELD_NAME);
    break;
  default:
    throw std::runtime_error("unknown typed allocator variant");
  }
  fields.finish();
  return result;
}

OpenMPUsesAllocatorsClauseAllocator
uses_allocator_kind(const TypedAllocator &value, bool &user_defined) {
  user_defined = false;
  switch (value.kind) {
  case ROUP_OMP_ALLOCATOR_DEFAULT:
    return OMPC_USESALLOCATORS_ALLOCATOR_default;
  case ROUP_OMP_ALLOCATOR_LARGE_CAP:
    return OMPC_USESALLOCATORS_ALLOCATOR_large_cap;
  case ROUP_OMP_ALLOCATOR_CONST:
    return OMPC_USESALLOCATORS_ALLOCATOR_cons_mem;
  case ROUP_OMP_ALLOCATOR_HIGH_BW:
    return OMPC_USESALLOCATORS_ALLOCATOR_high_bw;
  case ROUP_OMP_ALLOCATOR_LOW_LAT:
    return OMPC_USESALLOCATORS_ALLOCATOR_low_lat;
  case ROUP_OMP_ALLOCATOR_CGROUP:
    return OMPC_USESALLOCATORS_ALLOCATOR_cgroup;
  case ROUP_OMP_ALLOCATOR_PTEAM:
    return OMPC_USESALLOCATORS_ALLOCATOR_pteam;
  case ROUP_OMP_ALLOCATOR_THREAD:
    return OMPC_USESALLOCATORS_ALLOCATOR_thread;
  case ROUP_OMP_ALLOCATOR_CUSTOM:
    user_defined = true;
    return OMPC_USESALLOCATORS_ALLOCATOR_user;
  default:
    throw std::runtime_error("ompparser cannot represent typed uses-allocator kind " +
                             std::to_string(value.kind));
  }
}

void convert_lastprivate(OpenMPDirective &directive, FieldReader &fields) {
  const std::optional<std::uint32_t> modifier =
      fields.optional_u32(ROUP_FIELD_MODIFIER);
  const std::vector<std::string> items =
      required_clause_items(fields);
  fields.finish();
  OpenMPLastprivateClauseModifier mapped =
      OMPC_LASTPRIVATE_MODIFIER_unspecified;
  if (modifier.has_value()) {
    if (*modifier != ROUP_OMP_LASTPRIVATE_CONDITIONAL) {
      throw std::runtime_error("unknown typed lastprivate modifier " +
                               std::to_string(*modifier));
    }
    mapped = OMPC_LASTPRIVATE_MODIFIER_conditional;
  }
  OpenMPClause *clause =
      OpenMPLastprivateClause::addLastprivateClause(&directive, mapped);
  if (clause == nullptr)
    throw std::runtime_error("ompparser failed to create lastprivate clause");
  add_leaf_values(*clause, items);
  record_clause(directive, clause);
}

void convert_default(OpenMPDirective &directive, FieldReader &fields) {
  const OpenMPDefaultClauseKind kind =
      default_kind(fields.required_u32(ROUP_FIELD_KIND));
  const std::optional<std::uint32_t> category =
      fields.optional_u32(ROUP_FIELD_CATEGORY);
  fields.finish();
  if (category.has_value()) {
    (void)defaultmap_category(*category);
    throw std::runtime_error(
        "ompparser cannot represent a typed default variable category");
  }
  OpenMPClause *clause =
      OpenMPDefaultClause::addDefaultClause(&directive, kind);
  if (clause == nullptr)
    return;
  record_clause(directive, clause);
}

void convert_defaultmap(OpenMPDirective &directive, FieldReader &fields) {
  const OpenMPDefaultmapClauseBehavior behavior =
      defaultmap_behavior(fields.required_u32(ROUP_FIELD_BEHAVIOR));
  const std::optional<std::uint32_t> category =
      fields.optional_u32(ROUP_FIELD_CATEGORY);
  fields.finish();
  const OpenMPDefaultmapClauseCategory mapped_category =
      category.has_value() ? defaultmap_category(*category)
                           : OMPC_DEFAULTMAP_CATEGORY_unspecified;
  OpenMPClause *clause = OpenMPDefaultmapClause::addDefaultmapClause(
      &directive, behavior, mapped_category);
  if (clause == nullptr)
    throw std::runtime_error("ompparser failed to create defaultmap clause");
  record_clause(directive, clause);
}

void convert_linear(OpenMPDirective &directive, FieldReader &fields) {
  const std::optional<std::uint32_t> modifier =
      fields.optional_u32(ROUP_FIELD_MODIFIER);
  const std::vector<std::string> items =
      required_clause_items(fields);
  const std::optional<std::string> step =
      fields.optional_string(ROUP_FIELD_STEP);
  const std::uint32_t source_syntax =
      fields.required_u32(ROUP_FIELD_SOURCE_SYNTAX);
  fields.finish();
  if (source_syntax != ROUP_OMP_LINEAR_SOURCE_HISTORICAL &&
      source_syntax != ROUP_OMP_LINEAR_SOURCE_MODIFIER_PREFIX &&
      source_syntax != ROUP_OMP_LINEAR_SOURCE_CANONICAL_MODIFIERS) {
    throw std::runtime_error("unknown typed linear source-syntax tag " +
                             std::to_string(source_syntax));
  }
  const OpenMPLinearClauseModifier mapped =
      modifier.has_value() ? linear_modifier(*modifier)
                           : OMPC_LINEAR_MODIFIER_unspecified;
  OpenMPClause *raw = OpenMPLinearClause::addLinearClause(&directive, mapped);
  auto *clause = dynamic_cast<OpenMPLinearClause *>(raw);
  if (clause == nullptr)
    throw std::runtime_error("ompparser failed to create linear clause");
  add_leaf_values(*clause, items);
  clause->setModifierFirstSyntax(
      source_syntax == ROUP_OMP_LINEAR_SOURCE_MODIFIER_PREFIX);
  if (step.has_value()) {
    const std::string upstream_step =
        source_syntax == ROUP_OMP_LINEAR_SOURCE_CANONICAL_MODIFIERS
            ? "step(" + *step + ")"
            : *step;
    clause->setUserDefinedStep(upstream_step.c_str());
  }
  record_clause(directive, clause);
  std::vector<OpenMPClause *> *linear_clauses =
      directive.getClauses(OMPC_linear);
  if (linear_clauses->size() > 1) {
    auto *merge_target =
        dynamic_cast<OpenMPLinearClause *>(linear_clauses->front());
    if (merge_target == nullptr)
      throw std::runtime_error("ompparser linear merge target has wrong type");
    merge_target->mergeLinear(&directive, clause);
  }
}

void convert_aligned(OpenMPDirective &directive, FieldReader &fields) {
  const std::vector<std::string> items =
      required_clause_items(fields);
  const std::optional<std::string> alignment =
      fields.optional_string(ROUP_FIELD_ALIGNMENT);
  fields.finish();
  OpenMPClause *raw = OpenMPAlignedClause::addAlignedClause(&directive);
  auto *clause = dynamic_cast<OpenMPAlignedClause *>(raw);
  if (clause == nullptr)
    throw std::runtime_error("ompparser failed to create aligned clause");
  add_leaf_values(*clause, items);
  if (alignment.has_value())
    clause->setUserDefinedAlignment(alignment->c_str());
  record_clause(directive, clause);
}

void convert_dist_schedule(OpenMPDirective &directive, FieldReader &fields) {
  const OpenMPDistScheduleClauseKind kind =
      dist_schedule_kind(fields.required_u32(ROUP_FIELD_KIND));
  const std::optional<std::string> chunk =
      fields.optional_string(ROUP_FIELD_CHUNK_SIZE);
  fields.finish();
  OpenMPClause *raw =
      OpenMPDistScheduleClause::addDistScheduleClause(&directive, kind);
  auto *clause = dynamic_cast<OpenMPDistScheduleClause *>(raw);
  if (clause == nullptr)
    throw std::runtime_error("ompparser failed to create dist_schedule clause");
  if (chunk.has_value())
    clause->setChunkSize(chunk->c_str());
  record_clause(directive, clause);
}

void convert_proc_bind(OpenMPDirective &directive, FieldReader &fields) {
  const OpenMPProcBindClauseKind kind =
      proc_bind_kind(fields.required_u32(ROUP_FIELD_KIND));
  fields.finish();
  OpenMPClause *clause =
      OpenMPProcBindClause::addProcBindClause(&directive, kind);
  if (clause == nullptr)
    throw std::runtime_error("ompparser failed to create proc_bind clause");
  record_clause(directive, clause);
}

void convert_bind(OpenMPDirective &directive, FieldReader &fields) {
  const OpenMPBindClauseBinding kind =
      bind_kind(fields.required_u32(ROUP_FIELD_MODIFIER));
  fields.finish();
  OpenMPClause *clause = OpenMPBindClause::addBindClause(&directive, kind);
  if (clause == nullptr)
    throw std::runtime_error("ompparser failed to create bind clause");
  record_clause(directive, clause);
}

void convert_order(OpenMPDirective &directive, FieldReader &fields) {
  const std::optional<std::uint32_t> modifier =
      fields.optional_u32(ROUP_FIELD_MODIFIER);
  const OpenMPOrderClauseKind kind =
      order_kind(fields.required_u32(ROUP_FIELD_KIND));
  fields.finish();
  OpenMPClause *clause =
      modifier.has_value()
          ? OpenMPOrderClause::addOrderClause(
                &directive, order_modifier(*modifier), kind)
          : OpenMPOrderClause::addOrderClause(&directive, kind);
  if (clause == nullptr)
    throw std::runtime_error("ompparser failed to create order clause");
  record_clause(directive, clause);
}

void convert_device(OpenMPDirective &directive, FieldReader &fields) {
  const std::optional<std::uint32_t> modifier =
      fields.optional_u32(ROUP_FIELD_MODIFIER);
  const std::string device_num =
      fields.required_string(ROUP_FIELD_DEVICE_NUM);
  fields.finish();
  OpenMPDeviceClauseModifier mapped = OMPC_DEVICE_MODIFIER_unspecified;
  if (modifier.has_value()) {
    if (*modifier == ROUP_OMP_DEVICE_ANCESTOR) {
      mapped = OMPC_DEVICE_MODIFIER_ancestor;
    } else if (*modifier == ROUP_OMP_DEVICE_NUM) {
      mapped = OMPC_DEVICE_MODIFIER_device_num;
    } else {
      throw std::runtime_error("unknown typed device modifier " +
                               std::to_string(*modifier));
    }
  }
  OpenMPClause *clause =
      OpenMPDeviceClause::addDeviceClause(&directive, mapped);
  if (clause == nullptr)
    throw std::runtime_error("ompparser failed to create device clause");
  clause->addLangExpr(device_num.c_str(), OMPC_CLAUSE_SEP_space, 0, 0,
                      OMP_EXPR_PARSE_expression);
  record_clause(directive, clause);
}

void convert_device_type(OpenMPDirective &directive, FieldReader &fields) {
  const OpenMPDeviceTypeClauseKind kind =
      device_type_kind(fields.required_u32(ROUP_FIELD_KIND));
  fields.finish();
  OpenMPClause *clause =
      OpenMPDeviceTypeClause::addDeviceTypeClause(&directive, kind);
  if (clause == nullptr)
    throw std::runtime_error("ompparser failed to create device_type clause");
  record_clause(directive, clause);
}

void convert_allocate(OpenMPDirective &directive, FieldReader &fields) {
  const std::optional<std::string> allocator =
      fields.optional_string(ROUP_FIELD_ALLOCATOR_EXPRESSION);
  const std::optional<std::string> alignment =
      fields.optional_string(ROUP_FIELD_ALIGNMENT_EXPRESSION);
  const std::vector<std::string> items =
      required_clause_items(fields);
  const std::uint32_t source_syntax =
      fields.required_u32(ROUP_FIELD_ALLOCATE_SOURCE_SYNTAX);
  fields.finish();
  switch (source_syntax) {
  case ROUP_OMP_ALLOCATE_SOURCE_UNMODIFIED:
    if (allocator.has_value() || alignment.has_value())
      throw std::runtime_error(
          "unmodified allocate source carries a modifier expression");
    break;
  case ROUP_OMP_ALLOCATE_SOURCE_SIMPLE_ALLOCATOR:
    if (!allocator.has_value() || alignment.has_value())
      throw std::runtime_error(
          "simple allocate source has inconsistent modifier fields");
    break;
  case ROUP_OMP_ALLOCATE_SOURCE_MODIFIERS:
    if (!allocator.has_value() && !alignment.has_value())
      throw std::runtime_error(
          "complex allocate source has no modifier expression");
    break;
  default:
    throw std::runtime_error("unknown typed allocate source-syntax tag " +
                             std::to_string(source_syntax));
  }
  std::optional<std::string> upstream_allocator;
  if (allocator.has_value()) {
    upstream_allocator =
        source_syntax == ROUP_OMP_ALLOCATE_SOURCE_MODIFIERS
            ? "allocator(" + *allocator + ")"
            : *allocator;
  }
  const OpenMPAllocateClauseAllocator kind = upstream_allocator.has_value()
                                                ? OMPC_ALLOCATE_ALLOCATOR_user
                                                : OMPC_ALLOCATE_ALLOCATOR_unspecified;
  OpenMPClause *clause = OpenMPAllocateClause::addAllocateClause(
      &directive, kind,
      upstream_allocator.has_value()
          ? const_cast<char *>(upstream_allocator->c_str())
          : nullptr);
  if (clause == nullptr)
    throw std::runtime_error("ompparser failed to create allocate clause");
  auto *allocate = dynamic_cast<OpenMPAllocateClause *>(clause);
  if (allocate == nullptr)
    throw std::runtime_error("ompparser returned the wrong allocate class");
  if (alignment.has_value())
    allocate->addExtraAllocatorParameter("align(" + *alignment + ")");
  add_leaf_values(*clause, items);
  record_clause(directive, clause);
}

void convert_allocator(OpenMPDirective &directive, FieldReader &fields) {
  const std::string allocator =
      fields.required_string(ROUP_FIELD_ALLOCATOR_EXPRESSION);
  fields.finish();
  OpenMPClause *clause = OpenMPAllocatorClause::addAllocatorClause(
      &directive, OMPC_ALLOCATOR_ALLOCATOR_user,
      const_cast<char *>(allocator.c_str()));
  if (clause == nullptr)
    throw std::runtime_error("ompparser failed to create allocator clause");
  record_clause(directive, clause);
}

void convert_map(OpenMPDirective &directive, FieldReader &fields) {
  const std::optional<std::uint32_t> type =
      fields.optional_u32(ROUP_FIELD_KIND);
  const std::uint32_t type_spelling =
      fields.required_u32(ROUP_FIELD_MAP_TYPE_SPELLING);
  std::vector<std::uint32_t> modifiers =
      fields.optional_u32s(ROUP_FIELD_MODIFIERS);
  const std::optional<TypedMapperId> mapper = optional_node(
      fields, ROUP_FIELD_MAPPER, read_mapper_id);
  const std::vector<TypedIterator> iterators =
      read_iterators(fields, ROUP_FIELD_ITERATORS);
  const std::vector<std::string> items = required_omp_locators(fields);
  fields.finish();

  OpenMPMapClauseRefModifier ref = OMPC_MAP_REF_MODIFIER_unspecified;
  std::vector<OpenMPMapClauseModifier> mapped;
  std::size_t iterator_marker_count = 0;
  for (const std::uint32_t modifier : modifiers) {
    if (modifier == ROUP_OMP_MAP_MODIFIER_REF_POINTEE ||
        modifier == ROUP_OMP_MAP_MODIFIER_REF_POINTER ||
        modifier == ROUP_OMP_MAP_MODIFIER_REF_POINTER_AND_POINTEE) {
      if (ref != OMPC_MAP_REF_MODIFIER_unspecified) {
        throw std::runtime_error("map clause has multiple reference modifiers");
      }
      ref = map_ref_modifier(modifier);
    } else if (modifier == ROUP_OMP_MAP_MODIFIER_ITERATOR) {
      ++iterator_marker_count;
    } else if (modifier == ROUP_OMP_MAP_MODIFIER_DELETE &&
               type_spelling == ROUP_OMP_MAP_TYPE_SPELLING_DELETE) {
      // The pre-6.0 `delete` spelling selected the map type. ROUP carries
      // that historical semantic marker alongside its typed source spelling;
      // ompparser represents the form solely as OMPC_MAP_TYPE_delete.
      continue;
    } else {
      mapped.push_back(map_modifier(modifier));
    }
  }
  if (iterator_marker_count > 1) {
    throw std::runtime_error("map clause has multiple iterator modifiers");
  }
  if ((iterator_marker_count == 1) != !iterators.empty()) {
    throw std::runtime_error(
        "map iterator modifier and typed iterator list disagree");
  }
  if (mapper.has_value())
    mapped.push_back(OMPC_MAP_MODIFIER_mapper);
  if (iterator_marker_count == 1)
    mapped.push_back(OMPC_MAP_MODIFIER_iterator);
  if (mapped.size() > 3) {
    throw std::runtime_error(
        "ompparser cannot represent more than three map modifiers");
  }
  while (mapped.size() < 3)
    mapped.push_back(OMPC_MAP_MODIFIER_unspecified);

  const std::string mapper_name =
      mapper.has_value()
          ? (mapper->is_default ? std::string("default") : mapper->user_name)
          : std::string();
  OpenMPClause *raw = OpenMPMapClause::addMapClause(
      &directive, mapped[0], mapped[1], mapped[2],
      map_type_with_source_spelling(type, type_spelling), ref,
      mapper_name);
  auto *clause = dynamic_cast<OpenMPMapClause *>(raw);
  if (clause == nullptr)
    throw std::runtime_error("ompparser failed to create map clause");
  for (const TypedIterator &iterator : iterators) {
    clause->addIterator(iterator.type_name, iterator.variable, iterator.start,
                        iterator.end, iterator.step);
  }
  for (const std::string &item : items)
    clause->addItem(item);
  record_clause(directive, clause);
}

void convert_depend(OpenMPDirective &directive, FieldReader &fields) {
  struct TypedDependence {
    OpenMPDependClauseType type;
    std::vector<std::string> items;
  };
  const TypedDependence dependence = required_node(
      fields, ROUP_FIELD_DEPENDENCE, [](RoupNodeHandle node) {
        const RoupNodeKind kind =
            require_value(roup_node_kind(node),
                          "querying OpenMP dependence kind")
                .value;
        if (kind.family != ROUP_NODE_FAMILY_OMP_DEPENDENCE) {
          throw std::runtime_error(
              "dependence has the wrong semantic node family");
        }
        FieldReader dependence_fields = FieldReader::node(node);
        TypedDependence result;
        if (kind.variant == ROUP_OMP_DEPENDENCE_LOCATORS) {
          result.type = dependence_type(
              dependence_fields.required_u32(ROUP_FIELD_DEPEND_TYPE));
          dependence_fields.for_each_node(
              ROUP_FIELD_ITEMS, [&](RoupNodeHandle item) {
                result.items.push_back(read_omp_locator(item));
              });
        } else if (kind.variant == ROUP_OMP_DEPENDENCE_DEPOBJS) {
          result.type = OMPC_DEPENDENCE_TYPE_depobj;
          dependence_fields.for_each_node(
              ROUP_FIELD_OBJECTS, [&](RoupNodeHandle item) {
                result.items.push_back(read_clause_item(item));
              });
        } else {
          throw std::runtime_error("unknown typed OpenMP dependence variant");
        }
        dependence_fields.finish();
        if (result.items.empty()) {
          throw std::runtime_error("OpenMP dependence item list is empty");
        }
        return result;
      });
  const std::vector<TypedIterator> iterators =
      read_iterators(fields, ROUP_FIELD_ITERATORS);
  fields.finish();
  const OpenMPDependClauseModifier modifier =
      iterators.empty() ? OMPC_DEPEND_MODIFIER_unspecified
                        : OMPC_DEPEND_MODIFIER_iterator;
  OpenMPClause *raw =
      OpenMPDependClause::addDependClause(&directive, modifier, dependence.type);
  auto *clause = dynamic_cast<OpenMPDependClause *>(raw);
  if (clause == nullptr)
    throw std::runtime_error("ompparser failed to create depend clause");
  std::vector<std::vector<const char *>> definitions;
  definitions.reserve(iterators.size());
  for (const TypedIterator &iterator : iterators) {
    definitions.push_back({iterator.type_name.c_str(), iterator.variable.c_str(),
                           iterator.start.c_str(), iterator.end.c_str(),
                           iterator.step.c_str()});
  }
  clause->setDependIteratorsDefinitionClass(definitions);
  add_leaf_values(*clause, dependence.items);
  record_clause(directive, clause);
  std::vector<OpenMPClause *> *depend_clauses =
      directive.getClauses(OMPC_depend);
  if (depend_clauses->size() > 1) {
    auto *merge_target =
        dynamic_cast<OpenMPDependClause *>(depend_clauses->front());
    if (merge_target == nullptr)
      throw std::runtime_error("ompparser depend merge target has wrong type");
    merge_target->mergeDepend(&directive, clause);
  }
}

void convert_doacross(OpenMPDirective &directive, FieldReader &fields,
                      std::optional<std::uint32_t> source_alias =
                          std::nullopt) {
  const std::uint32_t kind = fields.required_u32(ROUP_FIELD_KIND);
  struct TypedIteration {
    std::uint32_t variant;
    std::vector<std::string> vector;
  };
  const TypedIteration iteration = required_node(
      fields, ROUP_FIELD_ITERATION, [](RoupNodeHandle node) {
        const RoupNodeKind iteration_kind =
            require_value(roup_node_kind(node),
                          "querying doacross iteration kind")
                .value;
        if (iteration_kind.family !=
            ROUP_NODE_FAMILY_OMP_DOACROSS_ITERATION) {
          throw std::runtime_error(
              "doacross iteration has the wrong semantic node family");
        }
        TypedIteration result{iteration_kind.variant, {}};
        FieldReader iteration_fields = FieldReader::node(node);
        if (result.variant == ROUP_OMP_DOACROSS_VECTOR) {
          iteration_fields.for_each_node(
              ROUP_FIELD_ITEMS, [&](RoupNodeHandle item) {
                const RoupNodeKind item_kind =
                    require_value(roup_node_kind(item),
                                  "querying doacross vector-item kind")
                        .value;
                if (item_kind.family !=
                        ROUP_NODE_FAMILY_OMP_DOACROSS_VECTOR_ITEM ||
                    item_kind.variant != ROUP_OMP_DOACROSS_VECTOR_ITEM) {
                  throw std::runtime_error(
                      "doacross vector item has the wrong semantic node kind");
                }
                FieldReader item_fields = FieldReader::node(item);
                std::string value =
                    item_fields.required_string(ROUP_FIELD_VARIABLE);
                const std::optional<std::uint32_t> offset_kind =
                    item_fields.optional_u32(ROUP_FIELD_KIND);
                const std::optional<std::string> offset =
                    item_fields.optional_string(ROUP_FIELD_OFFSET);
                if (offset_kind.has_value() != offset.has_value()) {
                  throw std::runtime_error(
                      "doacross offset kind and expression disagree");
                }
                if (offset_kind.has_value()) {
                  if (*offset_kind == ROUP_OMP_DOACROSS_OFFSET_ADD) {
                    value += " + ";
                  } else if (*offset_kind ==
                             ROUP_OMP_DOACROSS_OFFSET_SUBTRACT) {
                    value += " - ";
                  } else {
                    throw std::runtime_error(
                        "unknown typed doacross offset kind");
                  }
                  value += *offset;
                }
                item_fields.finish();
                result.vector.push_back(std::move(value));
              });
          if (result.vector.empty()) {
            throw std::runtime_error("doacross vector must not be empty");
          }
        } else if (result.variant != ROUP_OMP_DOACROSS_CURRENT &&
                   result.variant != ROUP_OMP_DOACROSS_PREVIOUS_CURRENT) {
          throw std::runtime_error(
              "unknown typed doacross iteration variant");
        }
        iteration_fields.finish();
        return result;
      });
  fields.finish();
  const bool source_is_empty =
      source_alias == ROUP_OMP_CLAUSE_ALIAS_DOACROSS_SOURCE_EMPTY;
  if (source_alias.has_value() && !source_is_empty) {
    OpenMPDependClauseType legacy_type;
    std::string dependence_vector;
    switch (*source_alias) {
    case ROUP_OMP_CLAUSE_ALIAS_DEPEND_SOURCE:
      if (kind != ROUP_OMP_DOACROSS_SOURCE ||
          iteration.variant != ROUP_OMP_DOACROSS_CURRENT) {
        throw std::runtime_error(
            "typed depend(source) alias has an invalid payload");
      }
      legacy_type = OMPC_DEPENDENCE_TYPE_source;
      break;
    case ROUP_OMP_CLAUSE_ALIAS_DEPEND_SINK:
      if (kind != ROUP_OMP_DOACROSS_SINK ||
          iteration.variant != ROUP_OMP_DOACROSS_VECTOR ||
          iteration.vector.empty()) {
        throw std::runtime_error(
            "typed depend(sink) alias has an invalid payload");
      }
      legacy_type = OMPC_DEPENDENCE_TYPE_sink;
      for (const std::string &item : iteration.vector) {
        if (!dependence_vector.empty())
          dependence_vector += ", ";
        dependence_vector += item;
      }
      break;
    case ROUP_OMP_CLAUSE_ALIAS_DEPEND_SINK_PREVIOUS_CURRENT:
      if (kind != ROUP_OMP_DOACROSS_SINK ||
          iteration.variant != ROUP_OMP_DOACROSS_PREVIOUS_CURRENT) {
        throw std::runtime_error(
            "typed historical depend(sink) alias has an invalid payload");
      }
      legacy_type = OMPC_DEPENDENCE_TYPE_sink;
      dependence_vector = "omp_cur_iteration - 1";
      break;
    case ROUP_OMP_CLAUSE_ALIAS_DEPEND_SOURCE_CURRENT:
      throw std::runtime_error(
          "ompparser cannot represent explicit depend(source: "
          "omp_cur_iteration) without changing its spelling");
    default:
      throw std::runtime_error(
          "typed source alias is not a historical depend clause");
    }
    OpenMPClause *raw = OpenMPDependClause::addDependClause(
        &directive, OMPC_DEPEND_MODIFIER_unspecified, legacy_type);
    auto *clause = dynamic_cast<OpenMPDependClause *>(raw);
    if (clause == nullptr) {
      throw std::runtime_error(
          "ompparser failed to create a historical depend clause");
    }
    if (!dependence_vector.empty())
      clause->addDependenceVector(dependence_vector.c_str());
    record_clause(directive, raw);
    return;
  }
  OpenMPDoacrossClauseType type;
  if (kind == ROUP_OMP_DOACROSS_SOURCE) {
    type = OMPC_DOACROSS_TYPE_source;
  } else if (kind == ROUP_OMP_DOACROSS_SINK) {
    type = OMPC_DOACROSS_TYPE_sink;
  } else {
    throw std::runtime_error("unknown typed doacross kind " +
                             std::to_string(kind));
  }
  auto clause = std::make_unique<OpenMPDoacrossClause>(type);
  if (type == OMPC_DOACROSS_TYPE_source) {
    if (iteration.variant != ROUP_OMP_DOACROSS_CURRENT) {
      throw std::runtime_error(
          "doacross source requires the current-iteration value");
    }
    if (!source_is_empty)
      clause->setSourceExpression("omp_cur_iteration");
  } else {
    if (iteration.variant == ROUP_OMP_DOACROSS_CURRENT) {
      throw std::runtime_error(
          "doacross sink cannot use the unmodified current iteration");
    }
    if (iteration.variant == ROUP_OMP_DOACROSS_PREVIOUS_CURRENT) {
      clause->addSinkArg("omp_cur_iteration - 1");
    }
    for (const std::string &item : iteration.vector)
      clause->addSinkArg(item);
  }
  OpenMPClause *raw = directive.registerClause(std::move(clause));
  record_clause(directive, raw);
}

void convert_affinity(OpenMPDirective &directive, FieldReader &fields) {
  const std::optional<std::uint32_t> modifier =
      fields.optional_u32(ROUP_FIELD_MODIFIER);
  const std::vector<TypedIterator> iterators =
      read_iterators(fields, ROUP_FIELD_ITERATORS);
  const std::vector<std::string> items = required_omp_locators(fields);
  fields.finish();
  OpenMPAffinityClauseModifier mapped = OMPC_AFFINITY_MODIFIER_unspecified;
  if (modifier.has_value()) {
    if (*modifier != ROUP_OMP_AFFINITY_ITERATOR)
      throw std::runtime_error("unknown typed affinity modifier " +
                               std::to_string(*modifier));
    mapped = OMPC_AFFINITY_MODIFIER_iterator;
  }
  if ((mapped == OMPC_AFFINITY_MODIFIER_iterator) != !iterators.empty()) {
    throw std::runtime_error(
        "affinity iterator modifier and typed iterator list disagree");
  }
  OpenMPClause *raw =
      OpenMPAffinityClause::addAffinityClause(&directive, mapped);
  auto *clause = dynamic_cast<OpenMPAffinityClause *>(raw);
  if (clause == nullptr)
    throw std::runtime_error("ompparser failed to create affinity clause");
  for (const TypedIterator &iterator : iterators) {
    clause->addIteratorsDefinitionClass(
        {iterator.type_name.c_str(), iterator.variable.c_str(),
         iterator.start.c_str(), iterator.end.c_str(), iterator.step.c_str()});
  }
  add_leaf_values(*clause, items);
  record_clause(directive, clause);
}

void convert_uses_allocators(OpenMPDirective &directive, FieldReader &fields) {
  OpenMPClause *raw =
      OpenMPUsesAllocatorsClause::addUsesAllocatorsClause(&directive);
  auto *clause = dynamic_cast<OpenMPUsesAllocatorsClause *>(raw);
  if (clause == nullptr) {
    throw std::runtime_error(
        "ompparser failed to create uses_allocators clause");
  }
  std::size_t entry_count = 0;
  fields.for_each_node(ROUP_FIELD_ALLOCATORS, [&](RoupNodeHandle node) {
    const RoupNodeKind node_kind =
        require_value(roup_node_kind(node), "querying uses-allocator node kind")
            .value;
    if (node_kind.family != ROUP_NODE_FAMILY_USES_ALLOCATOR ||
        node_kind.variant != ROUP_NODE_RECORD) {
      throw std::runtime_error("uses-allocator has the wrong semantic node kind");
    }
    FieldReader entry = FieldReader::node(node);
    const TypedAllocator allocator = required_node(
        entry, ROUP_FIELD_ALLOCATOR, read_allocator_kind);
    const std::uint32_t source_syntax =
        entry.required_u32(ROUP_FIELD_SOURCE_SYNTAX);
    const std::optional<std::string> traits =
        entry.optional_string(ROUP_FIELD_TRAITS);
    const std::optional<std::uint32_t> memspace =
        entry.optional_u32(ROUP_FIELD_MEMSPACE);
    entry.finish();
    if (memspace.has_value()) {
      switch (*memspace) {
      case ROUP_OMP_MEMORY_SPACE_DEFAULT:
      case ROUP_OMP_MEMORY_SPACE_LARGE_CAP:
      case ROUP_OMP_MEMORY_SPACE_CONST:
      case ROUP_OMP_MEMORY_SPACE_HIGH_BW:
      case ROUP_OMP_MEMORY_SPACE_LOW_LAT:
        break;
      default:
        throw std::runtime_error("unknown typed OpenMP memory-space tag " +
                                 std::to_string(*memspace));
      }
      throw std::runtime_error(
          "ompparser cannot represent a typed uses_allocators memspace");
    }
    bool user_defined = false;
    OpenMPUsesAllocatorsClauseAllocator kind =
        uses_allocator_kind(allocator, user_defined);
    if (source_syntax != ROUP_OMP_USES_ALLOCATORS_HISTORICAL &&
        source_syntax != ROUP_OMP_USES_ALLOCATORS_MODIFIER) {
      throw std::runtime_error("unknown typed uses_allocators source syntax " +
                               std::to_string(source_syntax));
    }
    if (traits.has_value() &&
        source_syntax == ROUP_OMP_USES_ALLOCATORS_MODIFIER)
      kind = OMPC_USESALLOCATORS_ALLOCATOR_unspecified;
    clause->addUsesAllocatorsAllocatorSequence(
        kind, traits.value_or(std::string()),
        user_defined || kind == OMPC_USESALLOCATORS_ALLOCATOR_unspecified
            ? allocator.custom_name
            : std::string());
    ++entry_count;
  });
  fields.finish();
  record_clause(directive, clause);
}

void convert_num_threads(OpenMPDirective &directive, FieldReader &fields) {
  const std::vector<std::uint32_t> modifiers =
      fields.optional_u32s(ROUP_FIELD_MODIFIERS);
  const std::vector<std::string> values =
      fields.required_strings(ROUP_FIELD_VALUES);
  fields.finish();
  if (values.empty())
    throw std::runtime_error("num_threads list must not be empty");
  if (modifiers.size() > 1 ||
      (!modifiers.empty() &&
       modifiers.front() != ROUP_OMP_NUM_THREADS_STRICT)) {
    throw std::runtime_error("unknown typed num_threads modifier");
  }
  OpenMPClause *raw =
      directive.addOpenMPClause(static_cast<int>(OMPC_num_threads), "");
  auto *clause = dynamic_cast<OpenMPNumThreadsClause *>(raw);
  if (clause == nullptr)
    throw std::runtime_error("ompparser failed to create num_threads clause");
  clause->setStrict(!modifiers.empty());
  for (std::size_t index = 0; index < values.size(); ++index) {
    clause->addLangExpr(values[index].c_str(),
                        index == 0 ? OMPC_CLAUSE_SEP_space
                                   : OMPC_CLAUSE_SEP_comma,
                        0, 0, OMP_EXPR_PARSE_expression);
  }
  record_clause(directive, raw);
}

void convert_num_teams(OpenMPDirective &directive, FieldReader &fields) {
  const std::optional<std::string> lower =
      fields.optional_string(ROUP_FIELD_LOWER_BOUND);
  const std::string upper = fields.required_string(ROUP_FIELD_UPPER_BOUND);
  fields.finish();
  if (lower.has_value()) {
    throw std::runtime_error(
        "ompparser cannot represent a typed num_teams lower bound");
  }
  add_simple_clause(directive, OMPC_num_teams, {upper});
}

void convert_scalar_expression(OpenMPDirective &directive,
                               OpenMPClauseKind kind,
                               std::uint32_t field,
                               FieldReader &fields) {
  const std::string value = fields.required_string(field);
  fields.finish();
  (void)add_scalar_expression_clause(directive, kind, value);
}

void convert_destroy(OpenMPDirective &directive, FieldReader &fields) {
  const std::optional<std::string> variable =
      fields.optional_string(ROUP_FIELD_VARIABLE);
  fields.finish();
  (void)add_scalar_expression_clause(directive, OMPC_destroy, variable);
}

void convert_use(OpenMPDirective &directive, FieldReader &fields) {
  const std::string variable = fields.required_string(ROUP_FIELD_VARIABLE);
  fields.finish();
  (void)add_scalar_expression_clause(directive, OMPC_use, variable);
}

void convert_uniform(OpenMPDirective &directive, FieldReader &fields) {
  std::vector<std::string> parameters;
  fields.for_each_node(ROUP_FIELD_ARGUMENTS, [&](RoupNodeHandle node) {
    parameters.push_back(read_clause_item(node));
  });
  if (parameters.empty())
    throw std::runtime_error("uniform parameter list must not be empty");
  fields.finish();
  add_simple_clause(directive, OMPC_uniform, parameters);
}

void convert_counts(OpenMPDirective &directive, FieldReader &fields) {
  const std::vector<std::string> counts = required_omp_counts(fields);
  fields.finish();
  add_simple_clause(directive, OMPC_counts, counts);
}

void convert_enter(OpenMPDirective &directive, FieldReader &fields) {
  const std::vector<std::uint32_t> modifiers =
      fields.optional_u32s(ROUP_FIELD_MODIFIERS);
  const std::vector<std::string> items = required_clause_items(fields);
  fields.finish();
  if (items.empty())
    throw std::runtime_error("enter item list must not be empty");
  if (!modifiers.empty()) {
    if (modifiers.size() != 1 ||
        modifiers.front() != ROUP_OMP_ENTER_AUTOMAP) {
      throw std::runtime_error("unknown typed enter modifier");
    }
    throw std::runtime_error(
        "ompparser cannot represent the typed enter automap modifier");
  }
  add_simple_clause(directive, OMPC_enter, items);
}

template <typename Clause, typename Kind, typename AddClause>
void convert_data_motion(OpenMPDirective &directive, FieldReader &fields,
                         OpenMPClauseKind clause_kind,
                         Kind unspecified_kind, Kind present_kind,
                         Kind mapper_kind, Kind iterator_kind,
                         AddClause add_clause) {
  const std::vector<std::uint32_t> modifiers =
      fields.optional_u32s(ROUP_FIELD_MODIFIERS);
  const std::optional<TypedMapperId> mapper = optional_node(
      fields, ROUP_FIELD_MAPPER, read_mapper_id);
  const std::vector<TypedIterator> iterators =
      read_iterators(fields, ROUP_FIELD_ITERATORS);
  const std::vector<std::string> locators = required_omp_locators(fields);
  fields.finish();

  bool present = false;
  for (const std::uint32_t modifier : modifiers) {
    if (modifier != ROUP_OMP_DATA_MOTION_PRESENT)
      throw std::runtime_error("unknown typed data-motion modifier " +
                               std::to_string(modifier));
    if (present)
      throw std::runtime_error(
          "data-motion clause has duplicate present modifiers");
    present = true;
  }
  const std::size_t modifier_families = static_cast<std::size_t>(present) +
                                        static_cast<std::size_t>(mapper.has_value()) +
                                        static_cast<std::size_t>(!iterators.empty());
  if (modifier_families > 1) {
    throw std::runtime_error(
        "ompparser cannot represent combined typed data-motion modifiers");
  }

  const Kind kind = present         ? present_kind
                    : mapper        ? mapper_kind
                    : !iterators.empty() ? iterator_kind
                                         : unspecified_kind;
  OpenMPClause *raw = add_clause(&directive, kind);
  auto *clause = dynamic_cast<Clause *>(raw);
  if (clause == nullptr)
    throw std::runtime_error("ompparser failed to create typed data-motion clause");
  if (mapper.has_value()) {
    const std::string mapper_name =
        mapper->is_default ? std::string("default") : mapper->user_name;
    clause->setMapperIdentifier(mapper_name.c_str());
  }
  for (const TypedIterator &iterator : iterators) {
    clause->addIterator(iterator.type_name, iterator.variable, iterator.start,
                        iterator.end, iterator.step);
  }
  for (const std::string &locator : locators)
    clause->addItem(locator);
  record_clause(directive, raw);
  (void)clause_kind;
}

void convert_to(OpenMPDirective &directive, FieldReader &fields) {
  convert_data_motion<OpenMPToClause, OpenMPToClauseKind>(
      directive, fields, OMPC_to, OMPC_TO_unspecified, OMPC_TO_present,
      OMPC_TO_mapper, OMPC_TO_iterator, OpenMPToClause::addToClause);
}

void convert_from(OpenMPDirective &directive, FieldReader &fields) {
  convert_data_motion<OpenMPFromClause, OpenMPFromClauseKind>(
      directive, fields, OMPC_from, OMPC_FROM_unspecified, OMPC_FROM_present,
      OMPC_FROM_mapper, OMPC_FROM_iterator, OpenMPFromClause::addFromClause);
}

void add_requirement_clause(OpenMPDirective &directive,
                            OpenMPClauseKind kind,
                            const std::optional<std::string> &required) {
  (void)add_scalar_expression_clause(directive, kind, required);
}

void convert_requires(OpenMPDirective &directive, FieldReader &fields) {
  const std::optional<std::string> required =
      fields.optional_string(ROUP_FIELD_REQUIRED);
  fields.for_each_node(ROUP_FIELD_REQUIREMENTS, [&](RoupNodeHandle node) {
    const RoupNodeKind kind =
        require_value(roup_node_kind(node), "querying requirement node kind")
            .value;
    if (kind.family != ROUP_NODE_FAMILY_REQUIRE_MODIFIER) {
      throw std::runtime_error("requirement has the wrong semantic node family");
    }
    FieldReader entry = FieldReader::node(node);
    switch (kind.variant) {
    case ROUP_REQUIRE_REVERSE_OFFLOAD:
      entry.finish();
      add_requirement_clause(directive, OMPC_reverse_offload, required);
      break;
    case ROUP_REQUIRE_UNIFIED_ADDRESS:
      entry.finish();
      add_requirement_clause(directive, OMPC_unified_address, required);
      break;
    case ROUP_REQUIRE_UNIFIED_SHARED_MEMORY:
      entry.finish();
      add_requirement_clause(directive, OMPC_unified_shared_memory, required);
      break;
    case ROUP_REQUIRE_DYNAMIC_ALLOCATORS:
      entry.finish();
      add_requirement_clause(directive, OMPC_dynamic_allocators, required);
      break;
    case ROUP_REQUIRE_SELF_MAPS:
      entry.finish();
      add_requirement_clause(directive, OMPC_self_maps, required);
      break;
    case ROUP_REQUIRE_DEVICE_SAFESYNC:
      entry.finish();
      add_requirement_clause(directive, OMPC_device_safesync, required);
      break;
    case ROUP_REQUIRE_ATOMIC_DEFAULT_MEM_ORDER: {
      if (required.has_value()) {
        throw std::runtime_error(
            "atomic_default_mem_order cannot carry a required condition");
      }
      const std::uint32_t order =
          entry.required_u32(ROUP_FIELD_MEMORY_ORDER);
      entry.finish();
      OpenMPAtomicDefaultMemOrderClauseKind mapped;
      if (order == ROUP_OMP_MEMORY_ORDER_SEQ_CST) {
        mapped = OMPC_ATOMIC_DEFAULT_MEM_ORDER_seq_cst;
      } else if (order == ROUP_OMP_MEMORY_ORDER_ACQ_REL) {
        mapped = OMPC_ATOMIC_DEFAULT_MEM_ORDER_acq_rel;
      } else if (order == ROUP_OMP_MEMORY_ORDER_RELAXED) {
        mapped = OMPC_ATOMIC_DEFAULT_MEM_ORDER_relaxed;
      } else {
        throw std::runtime_error(
            "ompparser cannot represent requires memory order " +
            std::to_string(order));
      }
      OpenMPClause *clause =
          OpenMPAtomicDefaultMemOrderClause::addAtomicDefaultMemOrderClause(
              &directive, mapped);
      if (clause == nullptr) {
        throw std::runtime_error(
            "ompparser failed to create atomic_default_mem_order clause");
      }
      record_clause(directive, clause);
      break;
    }
    case ROUP_REQUIRE_EXTENSION: {
      if (required.has_value()) {
        throw std::runtime_error(
            "extension requirements cannot carry a required condition");
      }
      const std::string identifier =
          entry.required_string(ROUP_FIELD_VALUE);
      entry.finish();
      OpenMPClause *raw =
          OpenMPExtImplementationDefinedRequirementClause::
              addExtImplementationDefinedRequirementClause(&directive);
      auto *clause =
          dynamic_cast<OpenMPExtImplementationDefinedRequirementClause *>(raw);
      if (clause == nullptr) {
        throw std::runtime_error(
            "ompparser failed to create extension requirement clause");
      }
      clause->setImplementationDefinedRequirement(identifier.c_str());
      record_clause(directive, clause);
      break;
    }
    default:
      throw std::runtime_error("unknown typed requirement variant");
    }
  });
  fields.finish();
}

void convert_depobj_update(OpenMPDirective &directive, FieldReader &fields) {
  const OpenMPDepobjUpdateClauseDependeceType type = depobj_update_type(
      fields.required_u32(ROUP_FIELD_DEPEND_TYPE));
  const std::optional<std::string> variable =
      fields.optional_string(ROUP_FIELD_VARIABLE);
  fields.finish();
  if (variable.has_value()) {
    throw std::runtime_error(
        "ompparser cannot represent an explicit depobj update variable");
  }
  OpenMPClause *clause =
      OpenMPDepobjUpdateClause::addDepobjUpdateClause(&directive, type);
  if (clause == nullptr)
    throw std::runtime_error("ompparser failed to create depobj update clause");
  record_clause(directive, clause);
}

void convert_adjust_args(OpenMPDirective &directive, FieldReader &fields) {
  const std::uint32_t operation = fields.required_u32(ROUP_FIELD_OPERATION);
  if (operation != ROUP_OMP_ADJUST_ARGS_NEED_DEVICE_PTR) {
    throw std::runtime_error(
        "ompparser cannot faithfully represent typed adjust_args operation " +
        std::to_string(operation));
  }
  std::vector<std::string> arguments;
  fields.for_each_node(ROUP_FIELD_PARAMETERS, [&](RoupNodeHandle node) {
    const RoupNodeKind kind =
        require_value(roup_node_kind(node),
                      "querying adjust_args parameter kind")
            .value;
    if (kind.family != ROUP_NODE_FAMILY_OMP_PARAMETER_LIST_ITEM) {
      throw std::runtime_error(
          "adjust_args parameter has the wrong semantic node family");
    }
    if (kind.variant != ROUP_OMP_PARAMETER_NAMED) {
      throw std::runtime_error(
          "ompparser cannot faithfully represent positional or range "
          "adjust_args parameters");
    }
    FieldReader parameter = FieldReader::node(node);
    arguments.push_back(parameter.required_string(ROUP_FIELD_NAME));
    parameter.finish();
  });
  if (arguments.empty()) {
    throw std::runtime_error("adjust_args parameter list must not be empty");
  }
  fields.finish();
  auto clause = std::make_unique<OpenMPAdjustArgsClause>();
  clause->setModifier(OMPC_ADJUST_ARGS_need_device_ptr);
  for (const std::string &argument : arguments)
    clause->addArgument(argument);
  OpenMPClause *raw = directive.registerClause(std::move(clause));
  record_clause(directive, raw);
}

void convert_append_args(OpenMPDirective &directive, FieldReader &fields) {
  std::vector<std::string> arguments;
  std::size_t operation_count = 0;
  fields.for_each_node(ROUP_FIELD_OPERATIONS, [&](RoupNodeHandle node) {
    ++operation_count;
    if (operation_count != 1) {
      throw std::runtime_error(
          "ompparser cannot faithfully represent multiple append_args "
          "operations");
    }
    const RoupNodeKind kind =
        require_value(roup_node_kind(node),
                      "querying append_args operation kind")
            .value;
    if (kind.family != ROUP_NODE_FAMILY_OMP_APPEND_OPERATION ||
        kind.variant != ROUP_OMP_APPEND_INTEROP) {
      throw std::runtime_error("unknown typed append_args operation");
    }
    FieldReader operation = FieldReader::node(node);
    for (const std::uint32_t interop_type :
         operation.required_u32s(ROUP_FIELD_INTEROP_TYPES)) {
      if (interop_type == ROUP_OMP_INTEROP_TARGET) {
        arguments.emplace_back("target");
      } else if (interop_type == ROUP_OMP_INTEROP_TARGETSYNC) {
        arguments.emplace_back("targetsync");
      } else {
        throw std::runtime_error("unknown typed append_args interop type");
      }
    }
    operation.for_each_node(ROUP_FIELD_PREFERENCES, [](RoupNodeHandle) {
      throw std::runtime_error(
          "ompparser cannot faithfully represent append_args preferences");
    });
    operation.finish();
  });
  if (operation_count != 1 || arguments.empty()) {
    throw std::runtime_error(
        "append_args requires one non-empty typed interop operation");
  }
  fields.finish();
  auto clause = std::make_unique<OpenMPAppendArgsClause>();
  clause->setLabel("interop");
  for (const std::string &argument : arguments)
    clause->addArgument(argument);
  OpenMPClause *raw = directive.registerClause(std::move(clause));
  record_clause(directive, raw);
}

void convert_at(OpenMPDirective &directive, FieldReader &fields) {
  const std::uint32_t value = fields.required_u32(ROUP_FIELD_KIND);
  fields.finish();
  OpenMPAtClauseKind kind;
  if (value == ROUP_OMP_AT_COMPILATION) {
    kind = OMPC_AT_compilation;
  } else if (value == ROUP_OMP_AT_EXECUTION) {
    kind = OMPC_AT_execution;
  } else {
    throw std::runtime_error("unknown typed at kind " +
                             std::to_string(value));
  }
  OpenMPClause *raw =
      directive.registerClause(std::make_unique<OpenMPAtClause>(kind));
  record_clause(directive, raw);
}

void convert_severity(OpenMPDirective &directive, FieldReader &fields) {
  const std::uint32_t value = fields.required_u32(ROUP_FIELD_KIND);
  fields.finish();
  OpenMPSeverityClauseKind kind;
  if (value == ROUP_OMP_SEVERITY_FATAL) {
    kind = OMPC_SEVERITY_fatal;
  } else if (value == ROUP_OMP_SEVERITY_WARNING) {
    kind = OMPC_SEVERITY_warning;
  } else {
    throw std::runtime_error("unknown typed severity kind " +
                             std::to_string(value));
  }
  OpenMPClause *raw =
      directive.registerClause(std::make_unique<OpenMPSeverityClause>(kind));
  record_clause(directive, raw);
}

void convert_fail(OpenMPDirective &directive, FieldReader &fields) {
  const std::uint32_t value =
      fields.required_u32(ROUP_FIELD_MEMORY_ORDER);
  fields.finish();
  OpenMPFailClauseMemoryOrder order;
  if (value == ROUP_OMP_MEMORY_ORDER_SEQ_CST) {
    order = OMPC_FAIL_seq_cst;
  } else if (value == ROUP_OMP_MEMORY_ORDER_ACQUIRE) {
    order = OMPC_FAIL_acquire;
  } else if (value == ROUP_OMP_MEMORY_ORDER_RELAXED) {
    order = OMPC_FAIL_relaxed;
  } else {
    throw std::runtime_error("ompparser cannot represent fail memory order " +
                             std::to_string(value));
  }
  OpenMPClause *raw =
      directive.registerClause(std::make_unique<OpenMPFailClause>(order));
  record_clause(directive, raw);
}

void convert_init(OpenMPDirective &directive, FieldReader &fields) {
  const std::vector<std::uint32_t> interop_types =
      fields.optional_u32s(ROUP_FIELD_INTEROP_TYPES);
  const std::optional<std::uint32_t> dependence =
      fields.optional_u32(ROUP_FIELD_DEPEND_TYPE);
  const std::string variable = fields.required_string(ROUP_FIELD_VARIABLE);
  std::size_t preference_count = 0;
  fields.for_each_node(ROUP_FIELD_PREFERENCES,
                       [&](RoupNodeHandle) { ++preference_count; });
  std::optional<std::string> locator;
  fields.for_each_node(ROUP_FIELD_LOCATOR, [&](RoupNodeHandle node) {
    if (locator.has_value())
      throw std::runtime_error("init contains more than one locator");
    locator = read_omp_locator(node);
  });
  fields.finish();
  if (preference_count != 0) {
    throw std::runtime_error(
        "ompparser cannot represent typed prefer_type specifications");
  }

  auto clause = std::make_unique<OpenMPInitClause>();
  if (!interop_types.empty()) {
    if (dependence.has_value() || locator.has_value()) {
      throw std::runtime_error(
          "interop init unexpectedly contains depobj initialization fields");
    }
    for (const std::uint32_t interop_type : interop_types) {
      if (interop_type == ROUP_OMP_INTEROP_TARGET) {
      clause->addInteropType(OMPC_INIT_KIND_target);
      } else if (interop_type == ROUP_OMP_INTEROP_TARGETSYNC) {
      clause->addInteropType(OMPC_INIT_KIND_targetsync);
      } else {
        throw std::runtime_error("unknown typed interop type " +
                                 std::to_string(interop_type));
      }
    }
  } else {
    if (!dependence.has_value() || !locator.has_value()) {
      throw std::runtime_error(
          "depobj init requires one dependence type and locator");
    }
    OpenMPDependClauseType type;
    if (*dependence == ROUP_OMP_DEPOBJ_UPDATE_IN)
      type = OMPC_DEPENDENCE_TYPE_in;
    else if (*dependence == ROUP_OMP_DEPOBJ_UPDATE_OUT)
      type = OMPC_DEPENDENCE_TYPE_out;
    else if (*dependence == ROUP_OMP_DEPOBJ_UPDATE_INOUT)
      type = OMPC_DEPENDENCE_TYPE_inout;
    else if (*dependence == ROUP_OMP_DEPOBJ_UPDATE_INOUTSET)
      type = OMPC_DEPENDENCE_TYPE_inoutset;
    else if (*dependence == ROUP_OMP_DEPOBJ_UPDATE_MUTEXINOUTSET)
      type = OMPC_DEPENDENCE_TYPE_mutexinoutset;
    else
      throw std::runtime_error("unknown typed depobj init dependence " +
                               std::to_string(*dependence));
    clause->setDepinfo(type, *locator);
  }
  clause->setOperand(variable);
  OpenMPClause *raw = directive.registerClause(std::move(clause));
  record_clause(directive, raw);
}

void convert_directive_list(OpenMPDirective &directive, OpenMPClauseKind kind,
                            FieldReader &fields) {
  const std::vector<std::uint32_t> names =
      fields.required_u32s(ROUP_FIELD_DIRECTIVES);
  fields.finish();
  OpenMPClause *raw =
      directive.addOpenMPClause(static_cast<int>(kind), "");
  if (raw == nullptr)
    throw std::runtime_error("ompparser failed to create directive-list clause");
  if (kind == OMPC_absent) {
    auto *clause = dynamic_cast<OpenMPAbsentClause *>(raw);
    if (clause == nullptr)
      throw std::runtime_error("ompparser returned wrong absent clause class");
    for (const std::uint32_t name : names)
      clause->addDirective(directive_kind(name));
  } else if (kind == OMPC_contains) {
    auto *clause = dynamic_cast<OpenMPContainsClause *>(raw);
    if (clause == nullptr)
      throw std::runtime_error("ompparser returned wrong contains clause class");
    for (const std::uint32_t name : names)
      clause->addDirective(directive_kind(name));
  } else {
    throw std::runtime_error("invalid directive-list clause kind");
  }
  record_clause(directive, raw);
}

void convert_task_size(OpenMPDirective &directive, OpenMPClauseKind kind,
                       FieldReader &fields) {
  const std::optional<std::uint32_t> modifier =
      fields.optional_u32(ROUP_FIELD_MODIFIER);
  const std::string value = fields.required_string(ROUP_FIELD_VALUE);
  fields.finish();
  OpenMPClause *clause = nullptr;
  if (kind == OMPC_grainsize) {
    OpenMPGrainsizeClauseModifier mapped =
        OMPC_GRAINSIZE_MODIFIER_unspecified;
    if (modifier.has_value()) {
      if (*modifier != ROUP_OMP_GRAINSIZE_STRICT)
        throw std::runtime_error("unknown typed grainsize modifier " +
                                 std::to_string(*modifier));
      mapped = OMPC_GRAINSIZE_MODIFIER_strict;
    }
    clause = directive.addOpenMPClause(OMPC_grainsize, mapped);
  } else if (kind == OMPC_num_tasks) {
    OpenMPNumTasksClauseModifier mapped = OMPC_NUM_TASKS_MODIFIER_unspecified;
    if (modifier.has_value()) {
      if (*modifier != ROUP_OMP_NUM_TASKS_STRICT)
        throw std::runtime_error("unknown typed num_tasks modifier " +
                                 std::to_string(*modifier));
      mapped = OMPC_NUM_TASKS_MODIFIER_strict;
    }
    clause = directive.addOpenMPClause(OMPC_num_tasks, mapped);
  } else {
    throw std::runtime_error("invalid task-size clause kind");
  }
  if (clause == nullptr)
    throw std::runtime_error("ompparser failed to create task-size clause");
  clause->addLangExpr(value.c_str(), OMPC_CLAUSE_SEP_space, 0, 0,
                      OMP_EXPR_PARSE_expression);
  record_clause(directive, clause);
}

void convert_scan(OpenMPDirective &directive, OpenMPClauseKind kind,
                  FieldReader &fields) {
  const std::uint32_t mode = fields.required_u32(ROUP_FIELD_KIND);
  const std::vector<std::string> items =
      required_clause_items(fields);
  fields.finish();
  const OpenMPClauseKind expected =
      mode == ROUP_OMP_SCAN_INCLUSIVE ? OMPC_inclusive
      : mode == ROUP_OMP_SCAN_EXCLUSIVE ? OMPC_exclusive
                            : OMPC_unknown;
  if (expected == OMPC_unknown || expected != kind)
    throw std::runtime_error("scan payload mode does not match clause kind");
  OpenMPClause *raw =
      directive.addOpenMPClause(static_cast<int>(kind), "");
  auto *clause = dynamic_cast<OpenMPScanClause *>(raw);
  if (clause == nullptr)
    throw std::runtime_error("ompparser failed to create scan clause");
  for (const std::string &item : items)
    clause->addOperand(item);
  record_clause(directive, clause);
}

void convert_atomic_operation(OpenMPDirective &directive,
                              OpenMPClauseKind kind,
                              FieldReader &fields) {
  const std::uint32_t operation = fields.required_u32(ROUP_FIELD_KIND);
  const std::optional<std::string> use_semantics =
      fields.optional_string(ROUP_FIELD_USE_SEMANTICS);
  fields.finish();
  const OpenMPClauseKind expected =
      operation == ROUP_OMP_ATOMIC_READ ? OMPC_read
      : operation == ROUP_OMP_ATOMIC_WRITE ? OMPC_write
      : operation == ROUP_OMP_ATOMIC_UPDATE ? OMPC_update
                                            : OMPC_unknown;
  if (expected == OMPC_unknown || expected != kind) {
    throw std::runtime_error(
        "atomic operation payload does not match its clause kind");
  }
  (void)add_scalar_expression_clause(directive, kind, use_semantics);
}

void convert_memory_order(OpenMPDirective &directive, OpenMPClauseKind kind,
                          FieldReader &fields) {
  const std::uint32_t memory_order =
      fields.required_u32(ROUP_FIELD_MEMORY_ORDER);
  const std::optional<std::string> use_semantics =
      fields.optional_string(ROUP_FIELD_USE_SEMANTICS);
  fields.finish();
  const OpenMPClauseKind order_kind =
      memory_order == ROUP_OMP_MEMORY_ORDER_SEQ_CST ? OMPC_seq_cst
      : memory_order == ROUP_OMP_MEMORY_ORDER_ACQ_REL ? OMPC_acq_rel
      : memory_order == ROUP_OMP_MEMORY_ORDER_RELEASE ? OMPC_release
      : memory_order == ROUP_OMP_MEMORY_ORDER_ACQUIRE ? OMPC_acquire
      : memory_order == ROUP_OMP_MEMORY_ORDER_RELAXED ? OMPC_relaxed
                                                     : OMPC_unknown;
  switch (order_kind) {
  case OMPC_seq_cst:
  case OMPC_acq_rel:
  case OMPC_release:
  case OMPC_acquire:
  case OMPC_relaxed:
    break;
  default:
    throw std::runtime_error("invalid typed atomic memory order " +
                             std::to_string(memory_order));
  }
  if (order_kind != kind) {
    throw std::runtime_error(
        "memory-order payload does not match its clause kind");
  }
  (void)add_scalar_expression_clause(directive, kind, use_semantics);
}

void convert_extended_atomic(OpenMPDirective &directive,
                             OpenMPClauseKind kind, FieldReader &fields) {
  const std::uint32_t extended_kind = fields.required_u32(ROUP_FIELD_KIND);
  const std::optional<std::string> use_semantics =
      fields.optional_string(ROUP_FIELD_USE_SEMANTICS);
  fields.finish();
  const OpenMPClauseKind expected =
      extended_kind == ROUP_OMP_EXTENDED_ATOMIC_CAPTURE ? OMPC_capture
      : extended_kind == ROUP_OMP_EXTENDED_ATOMIC_COMPARE ? OMPC_compare
      : extended_kind == ROUP_OMP_EXTENDED_ATOMIC_WEAK ? OMPC_weak
                                                       : OMPC_unknown;
  if (expected == OMPC_unknown || expected != kind) {
    throw std::runtime_error(
        "extended-atomic payload does not match its clause kind");
  }
  (void)add_scalar_expression_clause(directive, kind, use_semantics);
}

void convert_optional_expression(OpenMPDirective &directive,
                                 OpenMPClauseKind kind, std::uint32_t field,
                                 FieldReader &fields) {
  const std::optional<std::string> expression = fields.optional_string(field);
  fields.finish();
  (void)add_scalar_expression_clause(directive, kind, expression);
}

void convert_selector(OpenMPDirective &directive, OpenMPClauseKind kind,
                      FieldReader &fields);
void convert_source_alias(OpenMPDirective &directive, std::uint32_t ordinal,
                          std::uint32_t source_alias, FieldReader &fields);
std::string read_stylized_expression(RoupNodeHandle node);

void convert_inductor(OpenMPDirective &directive, FieldReader &fields) {
  const std::string expression = required_node(
      fields, ROUP_FIELD_VALUE, read_stylized_expression);
  fields.finish();
  add_simple_clause(directive, OMPC_inductor, {expression});
}

void convert_threadset(OpenMPDirective &directive, FieldReader &fields) {
  const std::uint32_t value = fields.required_u32(ROUP_FIELD_KIND);
  fields.finish();
  if (value == ROUP_OMP_THREADSET_OMP_POOL) {
    add_simple_clause(directive, OMPC_threadset, {"omp_pool"});
  } else if (value == ROUP_OMP_THREADSET_OMP_TEAM) {
    add_simple_clause(directive, OMPC_threadset, {"omp_team"});
  } else {
    throw std::runtime_error("unknown typed threadset kind " +
                             std::to_string(value));
  }
}

void convert_memscope(OpenMPDirective &directive, FieldReader &fields) {
  const std::uint32_t value = fields.required_u32(ROUP_FIELD_KIND);
  fields.finish();
  OpenMPMemscopeClauseKind kind;
  if (value == ROUP_OMP_MEMSCOPE_ALL) {
    kind = OMPC_MEMSCOPE_all;
  } else if (value == ROUP_OMP_MEMSCOPE_CGROUP) {
    kind = OMPC_MEMSCOPE_cgroup;
  } else if (value == ROUP_OMP_MEMSCOPE_DEVICE) {
    kind = OMPC_MEMSCOPE_device;
  } else {
    throw std::runtime_error("unknown typed memscope kind " +
                             std::to_string(value));
  }
  OpenMPClause *raw =
      directive.addOpenMPClause(static_cast<int>(OMPC_memscope), "");
  auto *clause = dynamic_cast<OpenMPMemscopeClause *>(raw);
  if (clause == nullptr)
    throw std::runtime_error("ompparser failed to create memscope clause");
  clause->setScope(kind);
  record_clause(directive, raw);
}

void convert_looprange(OpenMPDirective &directive, FieldReader &fields) {
  const std::string first = fields.required_string(ROUP_FIELD_FIRST);
  const std::string count = fields.required_string(ROUP_FIELD_COUNT);
  fields.finish();
  OpenMPClause *raw =
      directive.addOpenMPClause(static_cast<int>(OMPC_looprange), "");
  auto *clause = dynamic_cast<OpenMPLooprangeClause *>(raw);
  if (clause == nullptr)
    throw std::runtime_error("ompparser failed to create looprange clause");
  clause->addLangExpr(first.c_str(), OMPC_CLAUSE_SEP_space, 0, 0,
                      OMP_EXPR_PARSE_expression);
  clause->addLangExpr(count.c_str(), OMPC_CLAUSE_SEP_comma, 0, 0,
                      OMP_EXPR_PARSE_expression);
  record_clause(directive, raw);
}

void convert_graph_reset(OpenMPDirective &directive, FieldReader &fields) {
  const std::optional<std::string> condition =
      fields.optional_string(ROUP_FIELD_CONDITION);
  fields.finish();
  (void)add_scalar_expression_clause(directive, OMPC_graph_reset, condition);
}

void convert_clause_payload_fields(OpenMPDirective &directive,
                                   std::uint32_t ordinal,
                                   FieldReader fields,
                                   const std::optional<std::uint32_t>
                                       directive_name_modifier) {
  const OpenMPClauseKind kind = clause_kind(ordinal);
  switch (kind) {
  case OMPC_schedule:
    convert_schedule(directive, fields);
    return;
  case OMPC_if:
    convert_if(directive, fields, directive_name_modifier);
    return;
  case OMPC_reduction:
  case OMPC_task_reduction:
  case OMPC_in_reduction:
    convert_reduction(directive, kind, fields);
    return;
  case OMPC_induction:
    convert_induction(directive, fields);
    return;
  case OMPC_inductor:
    convert_inductor(directive, fields);
    return;
  case OMPC_threadset:
    convert_threadset(directive, fields);
    return;
  case OMPC_memscope:
    convert_memscope(directive, fields);
    return;
  case OMPC_looprange:
    convert_looprange(directive, fields);
    return;
  case OMPC_graph_reset:
    convert_graph_reset(directive, fields);
    return;
  case OMPC_apply:
    convert_apply(directive, fields);
    return;
  case OMPC_firstprivate:
    convert_firstprivate(directive, fields, directive_name_modifier);
    return;
  case OMPC_lastprivate:
    convert_lastprivate(directive, fields);
    return;
  case OMPC_default:
    convert_default(directive, fields);
    return;
  case OMPC_defaultmap:
    convert_defaultmap(directive, fields);
    return;
  case OMPC_linear:
    convert_linear(directive, fields);
    return;
  case OMPC_aligned:
    convert_aligned(directive, fields);
    return;
  case OMPC_dist_schedule:
    convert_dist_schedule(directive, fields);
    return;
  case OMPC_proc_bind:
    convert_proc_bind(directive, fields);
    return;
  case OMPC_bind:
    convert_bind(directive, fields);
    return;
  case OMPC_order:
    convert_order(directive, fields);
    return;
  case OMPC_device:
    convert_device(directive, fields);
    return;
  case OMPC_device_type:
    convert_device_type(directive, fields);
    return;
  case OMPC_allocate:
    convert_allocate(directive, fields);
    return;
  case OMPC_allocator:
    convert_allocator(directive, fields);
    return;
  case OMPC_map:
    convert_map(directive, fields);
    return;
  case OMPC_depend:
    convert_depend(directive, fields);
    return;
  case OMPC_doacross:
    convert_doacross(directive, fields);
    return;
  case OMPC_affinity:
    convert_affinity(directive, fields);
    return;
  case OMPC_uses_allocators:
    convert_uses_allocators(directive, fields);
    return;
  case OMPC_num_threads:
    convert_num_threads(directive, fields);
    return;
  case OMPC_num_teams:
    convert_num_teams(directive, fields);
    return;
  case OMPC_uniform:
    convert_uniform(directive, fields);
    return;
  case OMPC_counts:
    convert_counts(directive, fields);
    return;
  case OMPC_to:
    convert_to(directive, fields);
    return;
  case OMPC_from:
    convert_from(directive, fields);
    return;
  case OMPC_enter:
    convert_enter(directive, fields);
    return;
  case OMPC_destroy:
    convert_destroy(directive, fields);
    return;
  case OMPC_use:
    convert_use(directive, fields);
    return;
  case OMPC_align:
    convert_scalar_expression(directive, kind, ROUP_FIELD_ALIGNMENT, fields);
    return;
  case OMPC_final:
  case OMPC_holds:
  case OMPC_nocontext:
  case OMPC_novariants:
    convert_scalar_expression(directive, kind, ROUP_FIELD_CONDITION, fields);
    return;
  case OMPC_requires:
  case OMPC_reverse_offload:
  case OMPC_unified_address:
  case OMPC_unified_shared_memory:
  case OMPC_dynamic_allocators:
  case OMPC_self_maps:
  case OMPC_device_safesync:
  case OMPC_ext_implementation_defined_requirement:
  case OMPC_atomic_default_mem_order:
    convert_requires(directive, fields);
    return;
  case OMPC_depobj_update:
    convert_depobj_update(directive, fields);
    return;
  case OMPC_adjust_args:
    convert_adjust_args(directive, fields);
    return;
  case OMPC_append_args:
    convert_append_args(directive, fields);
    return;
  case OMPC_at:
    convert_at(directive, fields);
    return;
  case OMPC_severity:
    convert_severity(directive, fields);
    return;
  case OMPC_fail:
    convert_fail(directive, fields);
    return;
  case OMPC_init:
    convert_init(directive, fields);
    return;
  case OMPC_absent:
  case OMPC_contains:
    convert_directive_list(directive, kind, fields);
    return;
  case OMPC_grainsize:
  case OMPC_num_tasks:
    convert_task_size(directive, kind, fields);
    return;
  case OMPC_inclusive:
  case OMPC_exclusive:
    convert_scan(directive, kind, fields);
    return;
  case OMPC_read:
  case OMPC_write:
  case OMPC_update:
    convert_atomic_operation(directive, kind, fields);
    return;
  case OMPC_acq_rel:
  case OMPC_release:
  case OMPC_acquire:
  case OMPC_relaxed:
  case OMPC_seq_cst:
    convert_memory_order(directive, kind, fields);
    return;
  case OMPC_capture:
  case OMPC_compare:
  case OMPC_weak:
    convert_extended_atomic(directive, kind, fields);
    return;
  case OMPC_when:
  case OMPC_match:
  case OMPC_otherwise:
    convert_selector(directive, kind, fields);
    return;
  case OMPC_private:
  case OMPC_shared:
  case OMPC_copyin:
  case OMPC_copyprivate:
  case OMPC_use_device_ptr:
  case OMPC_use_device_addr:
  case OMPC_is_device_ptr:
  case OMPC_has_device_addr:
  case OMPC_nontemporal:
  case OMPC_thread_limit:
  case OMPC_collapse:
  case OMPC_ordered:
  case OMPC_safelen:
  case OMPC_simdlen:
  case OMPC_priority:
  case OMPC_detach:
  case OMPC_filter:
  case OMPC_hint:
  case OMPC_message:
  case OMPC_sizes:
  case OMPC_permutation:
  case OMPC_collector:
  case OMPC_combiner:
  case OMPC_graph_id:
  case OMPC_link:
  case OMPC_interop:
  case OMPC_local:
  case OMPC_initializer:
    convert_simple_clause(directive, kind, fields);
    return;
  case OMPC_partial:
    convert_optional_expression(directive, kind, ROUP_FIELD_UNROLL_FACTOR,
                                fields);
    return;
  case OMPC_indirect:
    convert_optional_expression(directive, kind, ROUP_FIELD_INVOKED_BY_FPTR,
                                fields);
    return;
  case OMPC_nowait:
  case OMPC_nogroup:
    convert_optional_expression(directive, kind,
                                ROUP_FIELD_DO_NOT_SYNCHRONIZE, fields);
    return;
  case OMPC_inbranch:
  case OMPC_notinbranch:
  case OMPC_init_complete:
    convert_optional_expression(directive, kind, ROUP_FIELD_CONDITION,
                                fields);
    return;
  case OMPC_untied:
    convert_optional_expression(directive, kind,
                                ROUP_FIELD_CAN_CHANGE_THREADS, fields);
    return;
  case OMPC_mergeable:
    convert_optional_expression(directive, kind, ROUP_FIELD_CAN_MERGE,
                                fields);
    return;
  case OMPC_full:
    convert_optional_expression(directive, kind, ROUP_FIELD_FULLY_UNROLL,
                                fields);
    return;
  case OMPC_transparent:
    convert_optional_expression(directive, kind, ROUP_FIELD_IMPEX_TYPE,
                                fields);
    return;
  case OMPC_replayable:
    convert_optional_expression(directive, kind,
                                ROUP_FIELD_REPLAYABLE_EXPRESSION, fields);
    return;
  case OMPC_safesync:
    convert_optional_expression(directive, kind, ROUP_FIELD_WIDTH, fields);
    return;
  case OMPC_no_openmp:
  case OMPC_no_openmp_constructs:
  case OMPC_no_openmp_routines:
  case OMPC_no_parallelism:
    convert_optional_expression(directive, kind, ROUP_FIELD_CAN_ASSUME,
                                fields);
    return;
  case OMPC_threads:
    convert_optional_expression(directive, kind, ROUP_FIELD_APPLY_TO_THREADS,
                                fields);
    return;
  case OMPC_simd:
    convert_optional_expression(directive, kind, ROUP_FIELD_APPLY_TO_SIMD,
                                fields);
    return;
  case OMPC_parallel:
  case OMPC_sections:
  case OMPC_for:
  case OMPC_do:
  case OMPC_taskgroup:
    convert_bare_clause(directive, kind, fields);
    return;
  default:
    throw std::runtime_error(
        "typed conversion is not implemented for OpenMP clause ordinal " +
        std::to_string(ordinal));
  }
}

OpenMPClauseSeparator source_clause_separator(std::uint32_t value) {
  if (value == ROUP_OMP_CLAUSE_SEPARATOR_SPACE)
    return OMPC_CLAUSE_SEP_space;
  if (value == ROUP_OMP_CLAUSE_SEPARATOR_COMMA)
    return OMPC_CLAUSE_SEP_comma;
  throw std::runtime_error("unknown typed clause source separator " +
                           std::to_string(value));
}

struct ConvertedClauseMetadata {
  OpenMPClauseSeparator preceding_separator;
  std::optional<std::uint32_t> source_alias;
};

ConvertedClauseMetadata convert_clause_fields(OpenMPDirective &directive,
                                               std::uint32_t ordinal,
                                               FieldReader fields) {
  const OpenMPClauseSeparator preceding_separator = source_clause_separator(
      fields.required_u32(ROUP_FIELD_PRECEDING_SEPARATOR));
  const std::optional<std::uint32_t> source_alias =
      fields.optional_u32(ROUP_FIELD_SOURCE_ALIAS);
  const std::optional<std::uint32_t> directive_name_modifier =
      fields.optional_u32(ROUP_FIELD_DIRECTIVE_NAME_MODIFIER);
  const std::size_t first_new_clause =
      directive.getClausesInOriginalOrder()->size();
  if (source_alias.has_value()) {
    convert_source_alias(directive, ordinal, *source_alias, fields);
  } else {
    convert_clause_payload_fields(directive, ordinal, std::move(fields),
                                  directive_name_modifier);
  }
  if (!directive_name_modifier.has_value() ||
      clause_kind(ordinal) == OMPC_if ||
      clause_kind(ordinal) == OMPC_firstprivate)
    return {preceding_separator, source_alias};

  const OpenMPDirectiveKind modifier =
      directive_kind(*directive_name_modifier);
  std::vector<OpenMPClause *> *clauses = directive.getClausesInOriginalOrder();
  if (clauses->size() == first_new_clause) {
    throw std::runtime_error(
        "typed directive-name modifier produced no upstream clause");
  }
  for (std::size_t index = first_new_clause; index < clauses->size(); ++index) {
    if (clauses->at(index) == nullptr)
      throw std::runtime_error(
          "ompparser stored a null clause for directive-name modifier");
    clauses->at(index)->setDirectiveNameModifier(modifier);
  }
  return {preceding_separator, source_alias};
}

void convert_clause(OpenMPDirective &directive, RoupDirectiveHandle source,
                    std::size_t index) {
  const std::size_t first_new_clause =
      directive.getClausesInOriginalOrder()->size();
  const ConvertedClauseMetadata metadata = convert_clause_fields(
      directive, clause_ordinal(source, index),
      FieldReader::clause(source, index));
  const OpenMPClauseSeparator upstream_separator =
      directive.getKind() == OMPD_atomic ? OMPC_CLAUSE_SEP_space
                                         : metadata.preceding_separator;
  RoupSpan span =
      require_value(roup_clause_span(source, index), "querying clause source span")
          .value;
  if (metadata.source_alias == ROUP_OMP_CLAUSE_ALIAS_METADIRECTIVE_DEFAULT)
    span.start_column += std::strlen("default");
  std::vector<OpenMPClause *> *clauses = directive.getClausesInOriginalOrder();
  for (std::size_t clause = first_new_clause; clause < clauses->size(); ++clause) {
    clauses->at(clause)->setPrecedingSeparator(upstream_separator);
    set_source_location(*clauses->at(clause), span, "clause source location");
  }
}

std::unique_ptr<OpenMPDirective> make_directive(OpenMPDirectiveKind kind,
                                                std::uint32_t ordinal) {
  if (kind == OMPD_atomic) {
    return std::make_unique<OpenMPAtomicDirective>();
  }
  if (kind == OMPD_end) {
    auto end = std::make_unique<OpenMPEndDirective>();
    const OpenMPDirectiveKind paired = paired_end_kind(ordinal);
    std::unique_ptr<OpenMPDirective> paired_directive =
        make_directive(paired, ordinal);
    paired_directive->setBaseLang(current_lang);
    paired_directive->setNormalizeClauses(normalize_clauses_global);
    end->setPairedDirective(std::move(paired_directive));
    return end;
  }
  switch (kind) {
  case OMPD_requires:
    return std::make_unique<OpenMPRequiresDirective>();
  case OMPD_allocate:
    return std::make_unique<OpenMPAllocateDirective>();
  case OMPD_threadprivate:
    return std::make_unique<OpenMPThreadprivateDirective>();
  case OMPD_groupprivate:
    return std::make_unique<OpenMPGroupprivateDirective>();
  case OMPD_declare_reduction:
    return std::make_unique<OpenMPDeclareReductionDirective>();
  case OMPD_declare_mapper:
    return std::make_unique<OpenMPDeclareMapperDirective>(
        OMPD_DECLARE_MAPPER_IDENTIFIER_unspecified);
  case OMPD_declare_variant:
    return std::make_unique<OpenMPDeclareVariantDirective>();
  case OMPD_declare_simd:
    return std::make_unique<OpenMPDeclareSimdDirective>();
  case OMPD_declare_target:
    return std::make_unique<OpenMPDeclareTargetDirective>();
  case OMPD_flush:
    return std::make_unique<OpenMPFlushDirective>();
  case OMPD_depobj:
    return std::make_unique<OpenMPDepobjDirective>();
  case OMPD_critical:
    return std::make_unique<OpenMPCriticalDirective>();
  default:
    return std::make_unique<OpenMPDirective>(kind, current_lang);
  }
}

OpenMPDirectiveKind source_facing_directive_kind(
    OpenMPDirectiveKind canonical, std::uint32_t source_alias) {
  if (source_alias == ROUP_OMP_DIRECTIVE_ALIAS_OPENMP60_UNDERSCORE &&
      canonical == OMPD_target_data)
    return OMPD_target_data_composite;
  return canonical;
}

void apply_directive_source_alias(OpenMPDirective &target,
                                  OpenMPDirectiveKind canonical,
                                  std::uint32_t source_alias) {
  if (source_alias == ROUP_OMP_DIRECTIVE_ALIAS_NONE)
    return;
  if (source_alias == ROUP_OMP_DIRECTIVE_ALIAS_OPENMP60_UNDERSCORE) {
    if (canonical == OMPD_declare_target ||
        canonical == OMPD_begin_declare_target ||
        canonical == OMPD_end_declare_target) {
      target.setDeclareTargetUnderscore(true);
    } else if (canonical == OMPD_end) {
      auto *end = dynamic_cast<OpenMPEndDirective *>(&target);
      OpenMPDirective *paired = end ? end->getPairedDirective() : nullptr;
      if (paired != nullptr &&
          (paired->getKind() == OMPD_declare_target ||
           paired->getKind() == OMPD_begin_declare_target ||
           paired->getKind() == OMPD_end_declare_target))
        paired->setDeclareTargetUnderscore(true);
    }
    return;
  }
  if (source_alias == ROUP_OMP_DIRECTIVE_ALIAS_FORTRAN_COMPACT) {
    if (canonical == OMPD_end) {
      auto *end = dynamic_cast<OpenMPEndDirective *>(&target);
      if (end == nullptr)
        throw std::runtime_error(
            "compact end-directive alias is attached to the wrong IR class");
      if (end->getPairedDirective() != nullptr &&
          end->getPairedDirective()->getKind() == OMPD_do)
        end->setUseCompactEndDo(true);
    } else {
      target.setCompactParallelDo(true);
    }
    return;
  }
  if (source_alias == ROUP_OMP_DIRECTIVE_ALIAS_FORTRAN_REDUNDANT_OMP) {
    if (canonical != OMPD_teams)
      throw std::runtime_error(
          "redundant omp alias is attached to a non-teams directive");
    return;
  }
  throw std::runtime_error("unknown typed OpenMP directive source alias " +
                           std::to_string(source_alias));
}

std::string read_storage_parameter_item(RoupNodeHandle node) {
  const RoupNodeKind kind =
      require_value(roup_node_kind(node), "querying storage-list item kind")
          .value;
  if (kind.family != ROUP_NODE_FAMILY_OMP_STORAGE_ITEM) {
    throw std::runtime_error(
        "storage-list item has the wrong semantic node family");
  }
  FieldReader fields = FieldReader::node(node);
  std::string value;
  switch (kind.variant) {
  case ROUP_OMP_STORAGE_ITEM_NAME:
    value = fields.required_string(ROUP_FIELD_NAME);
    break;
  case ROUP_OMP_STORAGE_ITEM_FORTRAN_COMMON_BLOCK:
    value = "/" + fields.required_string(ROUP_FIELD_NAME) + "/";
    break;
  default:
    throw std::runtime_error("unknown typed storage-list item variant");
  }
  fields.finish();
  return value;
}

std::string read_declare_target_parameter_item(RoupNodeHandle node) {
  const RoupNodeKind kind =
      require_value(roup_node_kind(node),
                    "querying declare-target-list item kind")
          .value;
  if (kind.family != ROUP_NODE_FAMILY_OMP_DECLARE_TARGET_ITEM) {
    throw std::runtime_error(
        "declare-target-list item has the wrong semantic node family");
  }
  FieldReader fields = FieldReader::node(node);
  std::string value;
  switch (kind.variant) {
  case ROUP_OMP_DECLARE_TARGET_ITEM_NAME:
    value = fields.required_string(ROUP_FIELD_NAME);
    break;
  case ROUP_OMP_DECLARE_TARGET_ITEM_FORTRAN_COMMON_BLOCK:
    value = "/" + fields.required_string(ROUP_FIELD_NAME) + "/";
    break;
  default:
    throw std::runtime_error(
        "unknown typed declare-target-list item variant");
  }
  fields.finish();
  return value;
}

std::string read_flush_parameter_item(RoupNodeHandle node) {
  const RoupNodeKind kind =
      require_value(roup_node_kind(node), "querying flush-list item kind")
          .value;
  if (kind.family != ROUP_NODE_FAMILY_OMP_FLUSH_ITEM) {
    throw std::runtime_error(
        "flush-list item has the wrong semantic node family");
  }
  FieldReader fields = FieldReader::node(node);
  std::string value;
  switch (kind.variant) {
  case ROUP_OMP_FLUSH_ITEM_IDENTIFIER:
    value = fields.required_string(ROUP_FIELD_NAME);
    break;
  case ROUP_OMP_FLUSH_ITEM_VARIABLE:
    value = fields.required_string(ROUP_FIELD_VARIABLE);
    break;
  case ROUP_OMP_FLUSH_ITEM_FORTRAN_COMMON_BLOCK:
    value = "/" + fields.required_string(ROUP_FIELD_NAME) + "/";
    break;
  default:
    throw std::runtime_error("unknown typed flush-list item variant");
  }
  fields.finish();
  return value;
}

std::string reduction_identifier_spelling(
    const TypedReductionIdentifier &identifier) {
  if (!identifier.user_spelling.empty())
    return identifier.user_spelling;
  switch (identifier.kind) {
  case OMPC_REDUCTION_IDENTIFIER_plus: return "+";
  case OMPC_REDUCTION_IDENTIFIER_minus: return "-";
  case OMPC_REDUCTION_IDENTIFIER_mul: return "*";
  case OMPC_REDUCTION_IDENTIFIER_bitand: return "&";
  case OMPC_REDUCTION_IDENTIFIER_bitor: return "|";
  case OMPC_REDUCTION_IDENTIFIER_bitxor: return "^";
  case OMPC_REDUCTION_IDENTIFIER_logand:
    return current_lang == Lang_Fortran ? ".and." : "&&";
  case OMPC_REDUCTION_IDENTIFIER_logor:
    return current_lang == Lang_Fortran ? ".or." : "||";
  case OMPC_REDUCTION_IDENTIFIER_eqv: return ".eqv.";
  case OMPC_REDUCTION_IDENTIFIER_neqv: return ".neqv.";
  case OMPC_REDUCTION_IDENTIFIER_max: return "max";
  case OMPC_REDUCTION_IDENTIFIER_min: return "min";
  default:
    throw std::runtime_error(
        "typed reduction identifier has no lexical representation");
  }
}

std::string read_stylized_expression(RoupNodeHandle node) {
  const RoupNodeKind kind =
      require_value(roup_node_kind(node),
                    "querying stylized-expression node kind")
          .value;
  if (kind.family != ROUP_NODE_FAMILY_OMP_STYLIZED_EXPRESSION) {
    throw std::runtime_error(
        "stylized expression has the wrong semantic node family");
  }
  FieldReader fields = FieldReader::node(node);
  std::string result;
  switch (kind.variant) {
  case ROUP_OMP_STYLIZED_C_CPP_EXPRESSION:
  case ROUP_OMP_STYLIZED_FORTRAN_SUBROUTINE_CALL:
    result = fields.required_string(ROUP_FIELD_VALUE);
    break;
  case ROUP_OMP_STYLIZED_FORTRAN_ASSIGNMENT: {
    const std::string target = fields.required_string(ROUP_FIELD_VARIABLE);
    const std::string value = fields.required_string(ROUP_FIELD_VALUE);
    result = target + " = " + value;
    break;
  }
  default:
    throw std::runtime_error("unknown typed stylized-expression variant");
  }
  fields.finish();
  return result;
}

std::string read_initializer_value(RoupNodeHandle node) {
  const RoupNodeKind kind =
      require_value(roup_node_kind(node), "querying initializer-value kind")
          .value;
  if (kind.family != ROUP_NODE_FAMILY_OMP_INITIALIZER_VALUE) {
    throw std::runtime_error(
        "initializer value has the wrong semantic node family");
  }
  FieldReader fields = FieldReader::node(node);
  if (kind.variant == ROUP_OMP_INITIALIZER_VALUE_EXPRESSION) {
    std::string value = fields.required_string(ROUP_FIELD_VALUE);
    fields.finish();
    return value;
  }
  if (kind.variant == ROUP_OMP_INITIALIZER_VALUE_BRACED) {
    std::vector<std::string> elements;
    fields.for_each_node(ROUP_FIELD_VALUES, [&](RoupNodeHandle element) {
      elements.push_back(read_initializer_value(element));
    });
    fields.finish();
    std::string value = "{";
    for (std::size_t index = 0; index < elements.size(); ++index) {
      if (index != 0)
        value += ", ";
      value += elements[index];
    }
    value += "}";
    return value;
  }
  throw std::runtime_error("unknown typed initializer-value variant");
}

std::string read_reduction_initializer(RoupNodeHandle node) {
  const RoupNodeKind kind =
      require_value(roup_node_kind(node),
                    "querying reduction-initializer node kind")
          .value;
  if (kind.family != ROUP_NODE_FAMILY_OMP_REDUCTION_INITIALIZER) {
    throw std::runtime_error(
        "reduction initializer has the wrong semantic node family");
  }
  FieldReader fields = FieldReader::node(node);
  std::string result;
  switch (kind.variant) {
  case ROUP_OMP_INITIALIZER_C_ASSIGNMENT:
  case ROUP_OMP_INITIALIZER_CPP_COPY:
    result = "omp_priv=" +
             required_node(fields, ROUP_FIELD_VALUE, read_initializer_value);
    break;
  case ROUP_OMP_INITIALIZER_CPP_DIRECT:
  case ROUP_OMP_INITIALIZER_C_CPP_FUNCTION_CALL:
  case ROUP_OMP_INITIALIZER_FORTRAN_SUBROUTINE_CALL:
    result = fields.required_string(ROUP_FIELD_VALUE);
    break;
  case ROUP_OMP_INITIALIZER_CPP_LIST: {
    std::vector<std::string> elements;
    fields.for_each_node(ROUP_FIELD_VALUES, [&](RoupNodeHandle element) {
      elements.push_back(read_initializer_value(element));
    });
    result = "omp_priv{";
    for (std::size_t index = 0; index < elements.size(); ++index) {
      if (index != 0)
        result += ", ";
      result += elements[index];
    }
    result += "}";
    break;
  }
  case ROUP_OMP_INITIALIZER_FORTRAN_ASSIGNMENT:
    result = fields.required_string(ROUP_FIELD_VARIABLE) + "=" +
             fields.required_string(ROUP_FIELD_VALUE);
    break;
  default:
    throw std::runtime_error("unknown typed reduction-initializer variant");
  }
  fields.finish();
  return result;
}

void convert_parameter_fields(OpenMPDirective &target,
                              OpenMPDirectiveKind kind,
                              std::uint32_t variant, FieldReader fields) {
  switch (variant) {
  case ROUP_OMP_PARAMETER_ALLOCATE_LIST: {
    std::vector<std::string> values;
    fields.for_each_node(ROUP_FIELD_ITEMS, [&](RoupNodeHandle node) {
      values.push_back(read_storage_parameter_item(node));
    });
    fields.finish();
    if (kind != OMPD_allocate)
      throw std::runtime_error("allocate-list parameter on wrong directive");
    auto &directive = static_cast<OpenMPAllocateDirective &>(target);
    for (const std::string &value : values)
      directive.addAllocateList(value.c_str());
    return;
  }
  case ROUP_OMP_PARAMETER_THREADPRIVATE_LIST: {
    std::vector<std::string> values;
    fields.for_each_node(ROUP_FIELD_ITEMS, [&](RoupNodeHandle node) {
      values.push_back(read_storage_parameter_item(node));
    });
    fields.finish();
    if (kind != OMPD_threadprivate)
      throw std::runtime_error(
          "threadprivate-list parameter on wrong directive");
    auto &directive = static_cast<OpenMPThreadprivateDirective &>(target);
    for (const std::string &value : values)
      directive.addThreadprivateList(value.c_str());
    return;
  }
  case ROUP_OMP_PARAMETER_GROUPPRIVATE_LIST: {
    std::vector<std::string> values;
    fields.for_each_node(ROUP_FIELD_ITEMS, [&](RoupNodeHandle node) {
      values.push_back(read_storage_parameter_item(node));
    });
    fields.finish();
    if (kind != OMPD_groupprivate)
      throw std::runtime_error(
          "groupprivate-list parameter on wrong directive");
    auto &directive = static_cast<OpenMPGroupprivateDirective &>(target);
    for (const std::string &value : values)
      directive.addGroupprivateList(value.c_str());
    return;
  }
  case ROUP_OMP_PARAMETER_DECLARE_TARGET_LIST: {
    std::vector<std::string> values;
    fields.for_each_node(ROUP_FIELD_ITEMS, [&](RoupNodeHandle node) {
      values.push_back(read_declare_target_parameter_item(node));
    });
    fields.finish();
    if (kind != OMPD_declare_target)
      throw std::runtime_error(
          "declare-target-list parameter on wrong directive");
    auto &directive = static_cast<OpenMPDeclareTargetDirective &>(target);
    for (const std::string &value : values)
      directive.addExtendedList(value.c_str());
    return;
  }
  case ROUP_OMP_PARAMETER_DEPOBJ: {
    const std::string value = fields.required_string(ROUP_FIELD_VALUE);
    fields.finish();
    if (kind != OMPD_depobj) {
      throw std::runtime_error("depobj parameter on wrong directive");
    }
    static_cast<OpenMPDepobjDirective &>(target).addDepobj(value.c_str());
    return;
  }
  case ROUP_OMP_PARAMETER_FLUSH_LIST: {
    std::vector<std::string> values;
    fields.for_each_node(ROUP_FIELD_ITEMS, [&](RoupNodeHandle node) {
      values.push_back(read_flush_parameter_item(node));
    });
    fields.finish();
    if (kind != OMPD_flush) {
      throw std::runtime_error("flush-list parameter on wrong directive");
    }
    auto &directive = static_cast<OpenMPFlushDirective &>(target);
    for (const std::string &value : values)
      directive.addFlushList(value.c_str());
    return;
  }
  case ROUP_OMP_PARAMETER_CONSTRUCT: {
    const std::uint32_t construct = fields.required_u32(ROUP_FIELD_KIND);
    fields.finish();
    if (kind != OMPD_cancel && kind != OMPD_cancellation_point) {
      throw std::runtime_error("construct parameter on wrong directive");
    }
    OpenMPClauseKind clause_kind;
    if (construct == ROUP_OMP_CONSTRUCT_PARALLEL) {
      clause_kind = OMPC_parallel;
    } else if (construct == ROUP_OMP_CONSTRUCT_SECTIONS) {
      clause_kind = OMPC_sections;
    } else if (construct == ROUP_OMP_CONSTRUCT_FOR) {
      clause_kind = current_lang == Lang_Fortran ? OMPC_do : OMPC_for;
    } else if (construct == ROUP_OMP_CONSTRUCT_TASKGROUP) {
      clause_kind = OMPC_taskgroup;
    } else {
      throw std::runtime_error("unknown typed cancellation construct " +
                               std::to_string(construct));
    }
    OpenMPClause *clause =
        target.addOpenMPClause(static_cast<int>(clause_kind), "");
    record_clause(target, clause);
    return;
  }
  case ROUP_OMP_PARAMETER_DECLARE_VARIANT: {
    const std::optional<std::string> base =
        fields.optional_string(ROUP_FIELD_BASE);
    std::string value = required_node(
        fields, ROUP_FIELD_FUNCTION, read_id_expression);
    fields.finish();
    if (kind != OMPD_declare_variant) {
      throw std::runtime_error("declare-variant parameter on wrong directive");
    }
    if (base.has_value())
      value = *base + ":" + value;
    static_cast<OpenMPDeclareVariantDirective &>(target).setVariantFuncID(
        value.c_str());
    return;
  }
  case ROUP_OMP_PARAMETER_CRITICAL_SECTION: {
    const std::string value = fields.required_string(ROUP_FIELD_VALUE);
    fields.finish();
    if (kind != OMPD_critical) {
      throw std::runtime_error("critical-section parameter on wrong directive");
    }
    static_cast<OpenMPCriticalDirective &>(target).setCriticalName(
        value.c_str());
    return;
  }
  case ROUP_OMP_PARAMETER_DECLARE_SIMD: {
    const std::string function = fields.required_string(ROUP_FIELD_FUNCTION);
    fields.finish();
    if (kind != OMPD_declare_simd) {
      throw std::runtime_error("declare-simd parameter on wrong directive");
    }
    static_cast<OpenMPDeclareSimdDirective &>(target).addProcName(function);
    return;
  }
  case ROUP_OMP_PARAMETER_DECLARE_MAPPER: {
    const std::optional<TypedMapperId> mapper = optional_node(
        fields, ROUP_FIELD_MAPPER, read_mapper_id);
    std::string type = fields.required_string(ROUP_FIELD_TYPE_NAME);
    const std::string variable =
        fields.required_string(ROUP_FIELD_VARIABLE);
    const std::optional<bool> declarator_attached =
        fields.optional_bool(ROUP_FIELD_DECLARATOR_ATTACHED);
    fields.finish();
    if (kind != OMPD_declare_mapper) {
      throw std::runtime_error("declare-mapper parameter on wrong directive");
    }
    auto &directive = static_cast<OpenMPDeclareMapperDirective &>(target);
    if (mapper.has_value()) {
      if (mapper->is_default) {
        directive.setIdentifier(OMPD_DECLARE_MAPPER_IDENTIFIER_default);
      } else {
        directive.setIdentifier(OMPD_DECLARE_MAPPER_IDENTIFIER_user);
        directive.setUserDefinedIdentifier(mapper->user_name);
      }
    } else {
      directive.setIdentifier(OMPD_DECLARE_MAPPER_IDENTIFIER_unspecified);
    }
    if (current_lang != Lang_Fortran) {
      if (!declarator_attached.has_value()) {
        throw std::runtime_error(
            "declare-mapper parameter lacks typed declarator attachment");
      }
      directive.setTypeVarHasSpace(!*declarator_attached);
    }
    directive.setDeclareMapperType(type.c_str());
    directive.setDeclareMapperVar(variable.c_str());
    return;
  }
  case ROUP_OMP_PARAMETER_DECLARE_REDUCTION: {
    const std::uint32_t source_syntax =
        fields.required_u32(ROUP_FIELD_SOURCE_SYNTAX);
    const TypedReductionIdentifier identifier = required_node(
        fields, ROUP_FIELD_NAME, read_reduction_identifier);
    const std::string operation = reduction_identifier_spelling(identifier);
    const std::vector<std::string> types =
        fields.required_strings(ROUP_FIELD_VALUES);
    const std::string combiner = required_node(
        fields, ROUP_FIELD_COMBINER, read_stylized_expression);
    const std::optional<std::string> initializer = optional_node(
        fields, ROUP_FIELD_INITIALIZER, read_reduction_initializer);
    fields.finish();
    if (kind != OMPD_declare_reduction) {
      throw std::runtime_error("declare-reduction parameter on wrong directive");
    }
    auto &directive = static_cast<OpenMPDeclareReductionDirective &>(target);
    directive.setIdentifier(operation);
    for (const std::string &type : types)
      directive.addTypenameList(type.c_str());
    if (source_syntax == ROUP_OMP_DECLARE_REDUCTION_INLINE_COMBINER) {
      directive.setCombiner(combiner.c_str());
    } else if (source_syntax ==
               ROUP_OMP_DECLARE_REDUCTION_COMBINER_CLAUSE) {
      OpenMPClause *combiner_clause =
          directive.addOpenMPClause(static_cast<int>(OMPC_combiner), "");
      if (combiner_clause == nullptr) {
        throw std::runtime_error(
            "ompparser failed to create declare-reduction combiner clause");
      }
      combiner_clause->addLangExpr(combiner.c_str(), OMPC_CLAUSE_SEP_space,
                                   0, 0, OMP_EXPR_PARSE_expression);
      record_clause(directive, combiner_clause);
    } else {
      throw std::runtime_error(
          "unknown typed declare-reduction source syntax " +
          std::to_string(source_syntax));
    }
    if (initializer.has_value()) {
      OpenMPClause *clause = OpenMPInitializerClause::addInitializerClause(
          &directive, OMPC_INITIALIZER_PRIV_user,
          const_cast<char *>(initializer->c_str()));
      if (clause == nullptr) {
        throw std::runtime_error("ompparser failed to create initializer clause");
      }
      clause->addLangExpr(initializer->c_str());
      record_clause(directive, clause);
    }
    return;
  }
  case ROUP_OMP_PARAMETER_DECLARE_INDUCTION: {
    if (kind != OMPD_declare_induction) {
      throw std::runtime_error(
          "declare-induction parameter is attached to the wrong directive");
    }
    const TypedReductionIdentifier identifier = required_node(
        fields, ROUP_FIELD_NAME, read_reduction_identifier);
    std::vector<std::string> type_specifiers;
    fields.for_each_node(ROUP_FIELD_TYPE_SPECIFIERS,
                         [&](RoupNodeHandle node) {
      const RoupNodeKind node_kind =
          require_value(roup_node_kind(node),
                        "querying induction type-specifier kind")
              .value;
      if (node_kind.family != ROUP_NODE_FAMILY_OMP_INDUCTION_TYPE) {
        throw std::runtime_error(
            "declare-induction type specifier has the wrong node family");
      }
      FieldReader type_fields = FieldReader::node(node);
      if (node_kind.variant == ROUP_INDUCTION_TYPE_SAME) {
        type_specifiers.push_back(
            type_fields.required_string(ROUP_FIELD_TYPE_NAME));
      } else if (node_kind.variant == ROUP_INDUCTION_TYPE_PAIR) {
        const std::string variable =
            type_fields.required_string(ROUP_FIELD_VARIABLE_TYPE);
        const std::string step =
            type_fields.required_string(ROUP_FIELD_STEP_TYPE);
        type_specifiers.push_back("(" + variable + ", " + step + ")");
      } else {
        throw std::runtime_error(
            "unknown typed declare-induction type specifier");
      }
      type_fields.finish();
    });
    fields.finish();
    if (type_specifiers.empty())
      throw std::runtime_error(
          "declare-induction parameter has no type specifiers");

    std::string type_list;
    for (std::size_t index = 0; index < type_specifiers.size(); ++index) {
      if (index != 0)
        type_list += ", ";
      type_list += type_specifiers[index];
    }
    OpenMPClause *raw =
        target.addOpenMPClause(static_cast<int>(OMPC_induction), "");
    auto *clause = dynamic_cast<OpenMPInductionClause *>(raw);
    if (clause == nullptr) {
      throw std::runtime_error(
          "ompparser failed to create declare-induction parameter storage");
    }
    clause->addBinding(reduction_identifier_spelling(identifier).c_str(),
                       type_list.c_str());
    record_clause(target, raw);
    return;
  }
  case ROUP_OMP_PARAMETER_OMPX: {
    if (kind != OMPD_ompx) {
      throw std::runtime_error("OMPX payload parameter on wrong directive");
    }
    std::vector<std::string> payload_items;
    fields.for_each_node(ROUP_FIELD_ITEMS, [&](RoupNodeHandle node) {
      const RoupNodeKind item_kind =
          require_value(roup_node_kind(node), "querying OMPX payload item kind")
              .value;
      if (item_kind.family != ROUP_NODE_FAMILY_OMPX_PAYLOAD_ITEM) {
        throw std::runtime_error(
            "OMPX payload item has the wrong semantic node family");
      }
      FieldReader item_fields = FieldReader::node(node);
      std::string item = item_fields.required_string(ROUP_FIELD_NAME);
      if (item_kind.variant == ROUP_OMPX_PAYLOAD_INVOCATION) {
        const std::vector<std::string> arguments =
            item_fields.required_strings(ROUP_FIELD_ARGUMENTS);
        item += "(";
        for (std::size_t index = 0; index < arguments.size(); ++index) {
          if (index != 0)
            item += ", ";
          item += arguments[index];
        }
        item += ")";
      } else if (item_kind.variant != ROUP_OMPX_PAYLOAD_IDENTIFIER) {
        throw std::runtime_error("unknown typed OMPX payload item variant");
      }
      item_fields.finish();
      payload_items.push_back(std::move(item));
    });
    fields.finish();
    if (payload_items.empty())
      throw std::runtime_error("typed OMPX payload is empty");
    std::string payload;
    for (std::size_t index = 0; index < payload_items.size(); ++index) {
      if (index != 0)
        payload += " ";
      payload += payload_items[index];
    }
    target.setFortranSentinel(OMPFS_ompx);
    target.setImplementationDefinedPayload(payload);
    return;
  }
  default:
    throw std::runtime_error(
        "typed directive parameter conversion is not implemented for variant " +
        std::to_string(variant));
  }
}

void convert_parameter(OpenMPDirective &target, RoupDirectiveHandle source,
                       OpenMPDirectiveKind kind) {
  const RoupU32Result has_parameter = require_value(
      roup_directive_has_parameter(source),
      "querying directive parameter presence");
  if (has_parameter.value == 0)
    return;
  if (has_parameter.value != 1)
    throw std::runtime_error("directive parameter presence is not boolean");
  const RoupParameterKind parameter_kind =
      require_value(roup_directive_parameter_kind(source),
                    "querying directive parameter kind")
          .value;
  if (parameter_kind.dialect != ROUP_DIALECT_OPENMP) {
    throw std::runtime_error("OpenMP adapter received a non-OpenMP parameter");
  }
  OpenMPDirective *parameter_target = &target;
  OpenMPDirectiveKind parameter_directive_kind = kind;
  if (kind == OMPD_end) {
    auto *end = dynamic_cast<OpenMPEndDirective *>(&target);
    if (end == nullptr || end->getPairedDirective() == nullptr) {
      throw std::runtime_error(
          "end directive parameter has no paired construct");
    }
    parameter_target = end->getPairedDirective();
    parameter_directive_kind = parameter_target->getKind();
  }
  convert_parameter_fields(*parameter_target, parameter_directive_kind,
                           parameter_kind.variant,
                           FieldReader::parameter(source));
}

std::unique_ptr<OpenMPDirective> convert_directive_node(RoupNodeHandle node) {
  const RoupNodeKind node_kind =
      require_value(roup_node_kind(node), "querying nested directive node kind")
          .value;
  if (node_kind.family != ROUP_NODE_FAMILY_OMP_DIRECTIVE) {
    throw std::runtime_error("nested directive has the wrong node family");
  }
  FieldReader fields = FieldReader::node(node);
  const std::uint32_t ordinal = node_kind.variant;
  const OpenMPDirectiveKind kind = directive_kind(ordinal);
  const std::uint32_t source_alias =
      fields.optional_u32(ROUP_FIELD_SOURCE_ALIAS)
          .value_or(ROUP_OMP_DIRECTIVE_ALIAS_NONE);
  std::unique_ptr<OpenMPDirective> target = make_directive(
      source_facing_directive_kind(kind, source_alias), ordinal);
  target->setBaseLang(current_lang);
  target->setNormalizeClauses(normalize_clauses_global);
  apply_directive_source_alias(*target, kind, source_alias);
  std::size_t parameter_count = 0;
  fields.for_each_node(ROUP_FIELD_PARAMETER, [&](RoupNodeHandle parameter) {
    ++parameter_count;
    const RoupNodeKind parameter_kind =
        require_value(roup_node_kind(parameter),
                      "querying nested parameter node kind")
            .value;
    if (parameter_kind.family != ROUP_NODE_FAMILY_OMP_PARAMETER) {
      throw std::runtime_error("nested parameter has the wrong node family");
    }
    convert_parameter_fields(*target, kind, parameter_kind.variant,
                             FieldReader::node(parameter));
  });
  if (parameter_count > 1)
    throw std::runtime_error("nested directive has multiple parameters");

  fields.for_each_node(ROUP_FIELD_CLAUSES, [&](RoupNodeHandle clause) {
    const RoupNodeKind clause_node_kind =
        require_value(roup_node_kind(clause), "querying nested clause node kind")
            .value;
    if (clause_node_kind.family != ROUP_NODE_FAMILY_OMP_CLAUSE) {
      throw std::runtime_error("nested clause has the wrong node family");
    }
    FieldReader clause_fields = FieldReader::node(clause);
    const std::size_t first_new_clause =
        target->getClausesInOriginalOrder()->size();
    const ConvertedClauseMetadata metadata = convert_clause_fields(
        *target, clause_node_kind.variant, std::move(clause_fields));
    const OpenMPClauseSeparator upstream_separator =
        target->getKind() == OMPD_atomic ? OMPC_CLAUSE_SEP_space
                                         : metadata.preceding_separator;
    std::vector<OpenMPClause *> *clauses =
        target->getClausesInOriginalOrder();
    for (std::size_t index = first_new_clause; index < clauses->size(); ++index)
      clauses->at(index)->setPrecedingSeparator(upstream_separator);
  });
  fields.finish();
  return target;
}

std::string selector_string_literal(std::string value, std::uint32_t encoding,
                                    std::uint32_t quote_style) {
  if (encoding == ROUP_CHARACTER_ENCODING_FORTRAN) {
    const char quote = quote_style == ROUP_QUOTE_SINGLE    ? '\''
                       : quote_style == ROUP_QUOTE_DOUBLE ? '"'
                                                          : '\0';
    if (quote == '\0')
      throw std::runtime_error("unknown typed string-literal quote style " +
                               std::to_string(quote_style));
    std::string result(1, quote);
    for (const char character : value) {
      if (character == quote) {
        result.push_back(quote);
        result.push_back(quote);
      }
      else
        result.push_back(character);
    }
    result.push_back(quote);
    return result;
  }

  if (quote_style != ROUP_QUOTE_DOUBLE)
    throw std::runtime_error(
        "C and C++ string literals require the double-quote delimiter");

  std::string prefix;
  if (encoding == ROUP_CHARACTER_ENCODING_ORDINARY) {
    prefix = "";
  } else if (encoding == ROUP_CHARACTER_ENCODING_UTF8) {
    prefix = "u8";
  } else if (encoding == ROUP_CHARACTER_ENCODING_UTF16) {
    prefix = "u";
  } else if (encoding == ROUP_CHARACTER_ENCODING_UTF32) {
    prefix = "U";
  } else if (encoding == ROUP_CHARACTER_ENCODING_WIDE) {
    prefix = "L";
  } else {
    throw std::runtime_error("unknown typed character encoding " +
                             std::to_string(encoding));
  }

  std::string result = prefix + "\"";
  for (const unsigned char character : value) {
    switch (character) {
    case '\\': result += "\\\\"; break;
    case '"': result += "\\\""; break;
    case '\n': result += "\\n"; break;
    case '\r': result += "\\r"; break;
    case '\t': result += "\\t"; break;
    case '\0': result += "\\000"; break;
    case '\a': result += "\\a"; break;
    case '\b': result += "\\b"; break;
    case '\v': result += "\\v"; break;
    case '\f': result += "\\f"; break;
    default:
      if (character < 0x20 || character == 0x7f) {
        result.push_back('\\');
        result.push_back(static_cast<char>('0' + ((character >> 6) & 0x07)));
        result.push_back(static_cast<char>('0' + ((character >> 3) & 0x07)));
        result.push_back(static_cast<char>('0' + (character & 0x07)));
      } else {
        result.push_back(static_cast<char>(character));
      }
      break;
    }
  }
  result.push_back('"');
  return result;
}

struct SelectorProperty {
  std::uint32_t variant;
  std::uint32_t closed_value;
  std::string lexical_value;
};

SelectorProperty selector_trait_value(RoupNodeHandle node) {
  const RoupNodeKind kind =
      require_value(roup_node_kind(node), "querying selector value node kind")
          .value;
  if (kind.family != ROUP_NODE_FAMILY_OMP_SELECTOR_TRAIT_VALUE) {
    throw std::runtime_error("selector trait value has the wrong node kind");
  }
  FieldReader fields = FieldReader::node(node);
  SelectorProperty result{kind.variant, 0, {}};
  switch (kind.variant) {
  case ROUP_SELECTOR_TRAIT_IDENTIFIER:
    result.lexical_value = fields.required_string(ROUP_FIELD_VALUE);
    break;
  case ROUP_SELECTOR_TRAIT_STRING_LITERAL:
    result.lexical_value = selector_string_literal(
        fields.required_string(ROUP_FIELD_VALUE),
        fields.required_u32(ROUP_FIELD_ENCODING),
        fields.required_u32(ROUP_FIELD_QUOTE_STYLE));
    break;
  case ROUP_SELECTOR_TRAIT_PREDEFINED_DEVICE_KIND:
  case ROUP_SELECTOR_TRAIT_PREDEFINED_VENDOR:
    result.closed_value = fields.required_u32(ROUP_FIELD_KIND);
    break;
  default:
    throw std::runtime_error("unknown typed selector-property variant " +
                             std::to_string(kind.variant));
  }
  fields.finish();
  return result;
}

struct SelectorNameList {
  std::uint32_t kind;
  std::vector<SelectorProperty> properties;
  std::string score;
};

SelectorNameList read_selector_name_list(RoupNodeHandle node) {
  const RoupNodeKind kind =
      require_value(roup_node_kind(node), "querying selector name-list kind")
          .value;
  if (kind.family != ROUP_NODE_FAMILY_OMP_SELECTOR_NAME_LIST_TRAIT) {
    throw std::runtime_error(
        "selector name-list trait has the wrong node family");
  }
  SelectorNameList result{kind.variant, {}, {}};
  FieldReader fields = FieldReader::node(node);
  result.score =
      fields.optional_string(ROUP_FIELD_SCORE).value_or(std::string());
  fields.for_each_node(ROUP_FIELD_PROPERTIES, [&](RoupNodeHandle property) {
    result.properties.push_back(selector_trait_value(property));
  });
  fields.finish();
  if (result.properties.empty()) {
    throw std::runtime_error(
        "selector name-list trait must contain at least one property");
  }
  return result;
}

OpenMPClauseContextKind selector_context_kind(std::uint32_t property) {
  switch (property) {
  case ROUP_OMP_DEVICE_KIND_HOST: return OMPC_CONTEXT_KIND_host;
  case ROUP_OMP_DEVICE_KIND_NOHOST: return OMPC_CONTEXT_KIND_nohost;
  case ROUP_OMP_DEVICE_KIND_ANY: return OMPC_CONTEXT_KIND_any;
  case ROUP_OMP_DEVICE_KIND_CPU: return OMPC_CONTEXT_KIND_cpu;
  case ROUP_OMP_DEVICE_KIND_GPU: return OMPC_CONTEXT_KIND_gpu;
  case ROUP_OMP_DEVICE_KIND_FPGA: return OMPC_CONTEXT_KIND_fpga;
  default:
    throw std::runtime_error("unknown typed device-kind property " +
                             std::to_string(property));
  }
}

OpenMPClauseContextVendor
selector_context_vendor(std::uint32_t property) {
  switch (property) {
  case ROUP_OMP_VENDOR_AMD: return OMPC_CONTEXT_VENDOR_amd;
  case ROUP_OMP_VENDOR_ARM: return OMPC_CONTEXT_VENDOR_arm;
  case ROUP_OMP_VENDOR_BSC: return OMPC_CONTEXT_VENDOR_bsc;
  case ROUP_OMP_VENDOR_CRAY: return OMPC_CONTEXT_VENDOR_cray;
  case ROUP_OMP_VENDOR_FUJITSU: return OMPC_CONTEXT_VENDOR_fujitsu;
  case ROUP_OMP_VENDOR_GNU: return OMPC_CONTEXT_VENDOR_gnu;
  case ROUP_OMP_VENDOR_IBM: return OMPC_CONTEXT_VENDOR_ibm;
  case ROUP_OMP_VENDOR_INTEL: return OMPC_CONTEXT_VENDOR_intel;
  case ROUP_OMP_VENDOR_LLVM: return OMPC_CONTEXT_VENDOR_llvm;
  case ROUP_OMP_VENDOR_NVIDIA: return OMPC_CONTEXT_VENDOR_nvidia;
  case ROUP_OMP_VENDOR_PGI: return OMPC_CONTEXT_VENDOR_pgi;
  case ROUP_OMP_VENDOR_TI: return OMPC_CONTEXT_VENDOR_ti;
  default:
    throw std::runtime_error("unknown typed implementation-vendor property " +
                             std::to_string(property));
  }
}

struct SelectorState {
  bool user = false;
  bool construct = false;
  bool device = false;
  bool implementation = false;
  bool isa = false;
  bool arch = false;
  bool device_num = false;
  bool extension = false;
};

void convert_device_trait(OpenMPVariantClause &target, RoupNodeHandle node,
                          SelectorState &state) {
  const RoupNodeKind kind =
      require_value(roup_node_kind(node), "querying device trait node kind")
          .value;
  if (kind.family != ROUP_NODE_FAMILY_OMP_SELECTOR_DEVICE_TRAIT)
    throw std::runtime_error("device trait has the wrong node family");
  FieldReader fields = FieldReader::node(node);
  switch (kind.variant) {
  case ROUP_SELECTOR_DEVICE_NAME_LIST: {
    std::size_t name_lists = 0;
    SelectorNameList name_list{};
    fields.for_each_node(ROUP_FIELD_TRAIT_NAME, [&](RoupNodeHandle value) {
      ++name_lists;
      name_list = read_selector_name_list(value);
    });
    fields.finish();
    if (name_lists != 1) {
      throw std::runtime_error(
          "device name-list trait must contain one typed name-list node");
    }
    if (name_list.properties.size() != 1) {
      throw std::runtime_error(
          "ompparser cannot represent multiple properties in one device name-list trait");
    }
    const SelectorProperty &property = name_list.properties.front();
    switch (name_list.kind) {
    case ROUP_SELECTOR_NAME_LIST_KIND:
      if (property.variant != ROUP_SELECTOR_TRAIT_PREDEFINED_DEVICE_KIND) {
        throw std::runtime_error(
            "ompparser cannot represent an open device-kind property");
      }
      target.setContextKind(name_list.score.c_str(),
                            selector_context_kind(property.closed_value));
      return;
    case ROUP_SELECTOR_NAME_LIST_ISA:
      if (property.variant != ROUP_SELECTOR_TRAIT_IDENTIFIER &&
          property.variant != ROUP_SELECTOR_TRAIT_STRING_LITERAL) {
        throw std::runtime_error("isa requires an open lexical property");
      }
      if (state.isa)
        throw std::runtime_error(
            "ompparser cannot represent repeated isa traits");
      target.setIsaExpression(name_list.score.c_str(),
                              property.lexical_value.c_str());
      state.isa = true;
      return;
    case ROUP_SELECTOR_NAME_LIST_ARCH:
      if (property.variant != ROUP_SELECTOR_TRAIT_IDENTIFIER &&
          property.variant != ROUP_SELECTOR_TRAIT_STRING_LITERAL) {
        throw std::runtime_error("arch requires an open lexical property");
      }
      if (state.arch)
        throw std::runtime_error(
            "ompparser cannot represent repeated arch traits");
      target.setArchExpression(name_list.score.c_str(),
                               property.lexical_value.c_str());
      state.arch = true;
      return;
    default:
      throw std::runtime_error(
          "implementation-only name-list trait appeared in a device selector");
    }
  }
  case ROUP_SELECTOR_DEVICE_NUM: {
    if (state.device_num)
      throw std::runtime_error("ompparser cannot represent repeated device_num traits");
    const std::string value = fields.required_string(ROUP_FIELD_VALUE);
    fields.finish();
    target.setDeviceNumExpression("", value.c_str());
    state.device_num = true;
    return;
  }
  case ROUP_SELECTOR_DEVICE_UID:
    fields.finish();
    throw std::runtime_error(
        "ompparser cannot represent a typed target-device uid trait");
  case ROUP_SELECTOR_DEVICE_EXTENSION:
    fields.finish();
    throw std::runtime_error(
        "ompparser cannot represent a recursively typed device extension trait");
  default:
    throw std::runtime_error("unknown typed device trait variant");
  }
}

std::string selector_requirement_name(RoupNodeHandle node) {
  const RoupNodeKind kind =
      require_value(roup_node_kind(node), "querying requirement kind").value;
  if (kind.family != ROUP_NODE_FAMILY_REQUIRE_MODIFIER)
    throw std::runtime_error("requirement has the wrong node family");
  FieldReader fields = FieldReader::node(node);
  std::string result;
  switch (kind.variant) {
  case ROUP_REQUIRE_REVERSE_OFFLOAD: result = "reverse_offload"; break;
  case ROUP_REQUIRE_UNIFIED_ADDRESS: result = "unified_address"; break;
  case ROUP_REQUIRE_UNIFIED_SHARED_MEMORY:
    result = "unified_shared_memory";
    break;
  case ROUP_REQUIRE_DYNAMIC_ALLOCATORS: result = "dynamic_allocators"; break;
  case ROUP_REQUIRE_SELF_MAPS: result = "self_maps"; break;
  case ROUP_REQUIRE_DEVICE_SAFESYNC: result = "device_safesync"; break;
  case ROUP_REQUIRE_ATOMIC_DEFAULT_MEM_ORDER: {
    const std::uint32_t order =
        fields.required_u32(ROUP_FIELD_MEMORY_ORDER);
    if (order == ROUP_OMP_MEMORY_ORDER_SEQ_CST)
      result = "atomic_default_mem_order(seq_cst)";
    else if (order == ROUP_OMP_MEMORY_ORDER_ACQ_REL)
      result = "atomic_default_mem_order(acq_rel)";
    else if (order == ROUP_OMP_MEMORY_ORDER_RELEASE)
      result = "atomic_default_mem_order(release)";
    else if (order == ROUP_OMP_MEMORY_ORDER_ACQUIRE)
      result = "atomic_default_mem_order(acquire)";
    else if (order == ROUP_OMP_MEMORY_ORDER_RELAXED)
      result = "atomic_default_mem_order(relaxed)";
    else
      throw std::runtime_error("unknown typed requirement memory order");
    break;
  }
  case ROUP_REQUIRE_EXTENSION:
    result = fields.required_string(ROUP_FIELD_VALUE);
    break;
  default:
    throw std::runtime_error("unknown typed implementation requirement");
  }
  fields.finish();
  return result;
}

std::string selector_requirement(RoupNodeHandle node) {
  const RoupNodeKind kind =
      require_value(roup_node_kind(node),
                    "querying selector requirement node kind")
          .value;
  if (kind.family != ROUP_NODE_FAMILY_OMP_SELECTOR_REQUIREMENT ||
      kind.variant != ROUP_SELECTOR_REQUIREMENT) {
    throw std::runtime_error(
        "selector requirement has the wrong semantic node kind");
  }
  FieldReader fields = FieldReader::node(node);
  std::string result = required_node(
      fields, ROUP_FIELD_REQUIREMENT, selector_requirement_name);
  const std::optional<std::string> required =
      fields.optional_string(ROUP_FIELD_REQUIRED);
  fields.finish();
  if (required.has_value())
    result += "(" + *required + ")";
  return result;
}

void convert_implementation_trait(OpenMPVariantClause &target,
                                  RoupNodeHandle node,
                                  SelectorState &state) {
  const RoupNodeKind kind =
      require_value(roup_node_kind(node),
                    "querying implementation trait node kind")
          .value;
  if (kind.family != ROUP_NODE_FAMILY_OMP_SELECTOR_IMPLEMENTATION_TRAIT)
    throw std::runtime_error("implementation trait has the wrong node family");
  FieldReader fields = FieldReader::node(node);
  const std::string score =
      fields.optional_string(ROUP_FIELD_SCORE).value_or(std::string());
  switch (kind.variant) {
  case ROUP_SELECTOR_IMPLEMENTATION_NAME_LIST: {
    std::size_t name_lists = 0;
    SelectorNameList name_list{};
    fields.for_each_node(ROUP_FIELD_TRAIT_NAME, [&](RoupNodeHandle value) {
      ++name_lists;
      name_list = read_selector_name_list(value);
    });
    fields.finish();
    if (name_lists != 1) {
      throw std::runtime_error(
          "implementation name-list trait must contain one typed name-list node");
    }
    if (name_list.kind == ROUP_SELECTOR_NAME_LIST_VENDOR) {
      if (name_list.properties.size() != 1) {
        throw std::runtime_error(
            "ompparser cannot represent multiple implementation vendor properties");
      }
      const SelectorProperty &property = name_list.properties.front();
      if (property.variant != ROUP_SELECTOR_TRAIT_PREDEFINED_VENDOR) {
        throw std::runtime_error(
            "ompparser cannot represent an open implementation-vendor property");
      }
      target.setImplementationKind(
          score.c_str(), selector_context_vendor(property.closed_value));
      return;
    }
    if (name_list.kind != ROUP_SELECTOR_NAME_LIST_EXTENSION) {
      throw std::runtime_error(
          "device-only name-list trait appeared in an implementation selector");
    }
    if (name_list.properties.size() != 1) {
      throw std::runtime_error(
          "ompparser cannot represent multiple implementation extension properties");
    }
    if (state.extension) {
      throw std::runtime_error(
          "ompparser cannot represent repeated extension traits");
    }
    const SelectorProperty &property = name_list.properties.front();
    if (property.variant != ROUP_SELECTOR_TRAIT_IDENTIFIER &&
        property.variant != ROUP_SELECTOR_TRAIT_STRING_LITERAL) {
      throw std::runtime_error(
          "implementation extension requires an open lexical property");
    }
    target.setExtensionExpression(score.c_str(),
                                  property.lexical_value.c_str());
    state.extension = true;
    return;
  }
  case ROUP_SELECTOR_IMPLEMENTATION_ATOMIC_DEFAULT_MEM_ORDER:
  {
    const std::uint32_t order =
        fields.required_u32(ROUP_FIELD_MEMORY_ORDER);
    fields.finish();
    const char *spelling = nullptr;
    if (order == ROUP_OMP_MEMORY_ORDER_SEQ_CST)
      spelling = "atomic_default_mem_order(seq_cst)";
    else if (order == ROUP_OMP_MEMORY_ORDER_ACQ_REL)
      spelling = "atomic_default_mem_order(acq_rel)";
    else if (order == ROUP_OMP_MEMORY_ORDER_RELEASE)
      spelling = "atomic_default_mem_order(release)";
    else if (order == ROUP_OMP_MEMORY_ORDER_ACQUIRE)
      spelling = "atomic_default_mem_order(acquire)";
    else if (order == ROUP_OMP_MEMORY_ORDER_RELAXED)
      spelling = "atomic_default_mem_order(relaxed)";
    else
      throw std::runtime_error(
          "unknown typed selector atomic-default memory order");
    target.setImplementationRequiresExpression(score.c_str(), spelling);
    return;
  }
  case ROUP_SELECTOR_IMPLEMENTATION_REQUIREMENT: {
    const std::string requirement = required_node(
        fields, ROUP_FIELD_REQUIREMENT, selector_requirement_name);
    fields.finish();
    target.setImplementationRequiresExpression(score.c_str(),
                                                requirement.c_str());
    return;
  }
  case ROUP_SELECTOR_IMPLEMENTATION_REQUIRES: {
    std::string requirements;
    fields.for_each_node(ROUP_FIELD_PROPERTIES, [&](RoupNodeHandle property) {
      if (!requirements.empty())
        requirements += ", ";
      requirements += selector_requirement(property);
    });
    fields.finish();
    if (requirements.empty())
      throw std::runtime_error(
          "typed implementation requires trait is empty");
    target.setImplementationRequiresExpression(score.c_str(),
                                                requirements.c_str());
    return;
  }
  case ROUP_SELECTOR_IMPLEMENTATION_EXTENSION:
    fields.finish();
    throw std::runtime_error(
        "ompparser cannot represent a recursively typed implementation extension trait");
  default:
    throw std::runtime_error("unknown typed implementation trait variant");
  }
}

void convert_selector_entry(OpenMPVariantClause &target, RoupNodeHandle node,
                            SelectorState &state) {
  const RoupNodeKind kind =
      require_value(roup_node_kind(node), "querying selector entry node kind")
          .value;
  if (kind.family != ROUP_NODE_FAMILY_OMP_SELECTOR_ENTRY)
    throw std::runtime_error("selector entry has the wrong node family");
  FieldReader fields = FieldReader::node(node);
  switch (kind.variant) {
  case ROUP_SELECTOR_ENTRY_USER: {
    if (state.user)
      throw std::runtime_error("ompparser cannot represent repeated user selectors");
    const std::string score =
        fields.optional_string(ROUP_FIELD_SCORE).value_or(std::string());
    const std::string condition =
        fields.required_string(ROUP_FIELD_CONDITION);
    fields.finish();
    target.setUserCondition(score.c_str(), condition.c_str());
    target.addSelectorKind(OMPC_SELECTOR_user);
    state.user = true;
    return;
  }
  case ROUP_SELECTOR_ENTRY_CONSTRUCT:
    if (state.construct) {
      throw std::runtime_error(
          "ompparser cannot represent repeated construct selectors");
    }
    fields.for_each_node(ROUP_FIELD_ITEMS, [&](RoupNodeHandle construct) {
      const RoupNodeKind construct_kind =
          require_value(roup_node_kind(construct),
                        "querying construct selector node kind")
              .value;
      if (construct_kind.family != ROUP_NODE_FAMILY_OMP_SELECTOR_CONSTRUCT ||
          construct_kind.variant != ROUP_SELECTOR_CONSTRUCT) {
        throw std::runtime_error("construct selector has the wrong node kind");
      }
      FieldReader construct_fields = FieldReader::node(construct);
      const std::string score =
          construct_fields.optional_string(ROUP_FIELD_SCORE)
              .value_or(std::string());
      std::size_t directives = 0;
      construct_fields.for_each_node(
          ROUP_FIELD_NESTED_DIRECTIVE, [&](RoupNodeHandle directive) {
            ++directives;
            target.addConstructDirective(
                score.c_str(), convert_directive_node(directive));
          });
      if (directives != 1) {
        throw std::runtime_error(
            "construct selector must contain one nested directive");
      }
      construct_fields.finish();
    });
    fields.finish();
    target.addSelectorKind(OMPC_SELECTOR_construct);
    state.construct = true;
    return;
  case ROUP_SELECTOR_ENTRY_DEVICE:
  case ROUP_SELECTOR_ENTRY_TARGET_DEVICE: {
    if (state.device)
      throw std::runtime_error("ompparser cannot represent repeated device selectors");
    const bool is_target = kind.variant == ROUP_SELECTOR_ENTRY_TARGET_DEVICE;
    fields.for_each_node(ROUP_FIELD_TRAITS, [&](RoupNodeHandle trait) {
      convert_device_trait(target, trait, state);
    });
    fields.finish();
    target.setIsTargetDeviceSelector(is_target);
    target.addSelectorKind(is_target ? OMPC_SELECTOR_target_device
                                    : OMPC_SELECTOR_device);
    state.device = true;
    return;
  }
  case ROUP_SELECTOR_ENTRY_IMPLEMENTATION:
    if (state.implementation) {
      throw std::runtime_error(
          "ompparser cannot represent repeated implementation selectors");
    }
    fields.for_each_node(ROUP_FIELD_TRAITS, [&](RoupNodeHandle trait) {
      convert_implementation_trait(target, trait, state);
    });
    fields.finish();
    target.addSelectorKind(OMPC_SELECTOR_implementation);
    state.implementation = true;
    return;
  default:
    throw std::runtime_error("unknown typed selector entry variant");
  }
}

void convert_selector(OpenMPDirective &directive, OpenMPClauseKind kind,
                      FieldReader &fields) {
  OpenMPClause *raw = nullptr;
  if (kind == OMPC_when) {
    raw = OpenMPWhenClause::addWhenClause(&directive);
  } else if (kind == OMPC_match) {
    raw = OpenMPMatchClause::addMatchClause(&directive);
  } else if (kind == OMPC_otherwise) {
    raw = OpenMPOtherwiseClause::addOtherwiseClause(&directive);
  } else {
    throw std::runtime_error("invalid selector clause kind");
  }
  auto *target = dynamic_cast<OpenMPVariantClause *>(raw);
  if (target == nullptr)
    throw std::runtime_error("ompparser failed to create selector clause");

  SelectorState state;
  fields.for_each_node(ROUP_FIELD_ENTRIES, [&](RoupNodeHandle entry) {
    convert_selector_entry(*target, entry, state);
  });
  std::size_t nested_count = 0;
  fields.for_each_node(
      ROUP_FIELD_NESTED_DIRECTIVE, [&](RoupNodeHandle nested) {
        ++nested_count;
        std::unique_ptr<OpenMPDirective> converted =
            convert_directive_node(nested);
        if (kind == OMPC_when) {
          static_cast<OpenMPWhenClause *>(target)->setVariantDirective(
              std::move(converted));
        } else if (kind == OMPC_otherwise) {
          static_cast<OpenMPOtherwiseClause *>(target)->setVariantDirective(
              std::move(converted));
        } else {
          throw std::runtime_error(
              "match clause unexpectedly contains a nested directive");
        }
      });
  fields.finish();
  if (kind == OMPC_otherwise && nested_count != 1) {
    throw std::runtime_error(
        "otherwise clause must contain one nested directive");
  }
  if (kind == OMPC_match && nested_count != 0)
    throw std::runtime_error("match clause must not contain a nested directive");
  if (kind == OMPC_otherwise &&
      (state.user || state.construct || state.device || state.implementation)) {
    throw std::runtime_error("otherwise clause must not contain selectors");
  }
  record_clause(directive, raw);
}

void convert_source_alias(OpenMPDirective &directive, std::uint32_t ordinal,
                          std::uint32_t source_alias, FieldReader &fields) {
  const OpenMPClauseKind kind = clause_kind(ordinal);
  switch (source_alias) {
  case ROUP_OMP_CLAUSE_ALIAS_DEPEND_SOURCE:
  case ROUP_OMP_CLAUSE_ALIAS_DEPEND_SOURCE_CURRENT:
  case ROUP_OMP_CLAUSE_ALIAS_DEPEND_SINK:
  case ROUP_OMP_CLAUSE_ALIAS_DEPEND_SINK_PREVIOUS_CURRENT:
  case ROUP_OMP_CLAUSE_ALIAS_DOACROSS_SOURCE_EMPTY:
    if (kind != OMPC_doacross) {
      throw std::runtime_error(
          "typed historical depend alias is attached to another clause");
    }
    convert_doacross(directive, fields, source_alias);
    return;
  case ROUP_OMP_CLAUSE_ALIAS_METADIRECTIVE_DEFAULT: {
    if (kind != OMPC_otherwise) {
      throw std::runtime_error(
          "typed metadirective default alias is attached to another clause");
    }
    std::size_t selector_count = 0;
    fields.for_each_node(ROUP_FIELD_ENTRIES,
                         [&](RoupNodeHandle) { ++selector_count; });
    std::size_t nested_count = 0;
    std::unique_ptr<OpenMPDirective> nested;
    fields.for_each_node(
        ROUP_FIELD_NESTED_DIRECTIVE, [&](RoupNodeHandle node) {
          ++nested_count;
          nested = convert_directive_node(node);
        });
    fields.finish();
    if (selector_count != 0 || nested_count != 1 || nested == nullptr) {
      throw std::runtime_error(
          "typed metadirective default alias has an invalid payload");
    }
    OpenMPClause *raw = OpenMPDefaultClause::addDefaultClause(
        &directive, OMPC_DEFAULT_variant);
    auto *clause = dynamic_cast<OpenMPDefaultClause *>(raw);
    if (clause == nullptr) {
      throw std::runtime_error(
          "ompparser failed to create a metadirective default clause");
    }
    clause->setVariantDirective(std::move(nested));
    record_clause(directive, raw);
    return;
  }
  case ROUP_OMP_CLAUSE_ALIAS_DECLARE_TARGET_TO: {
    if (kind != OMPC_enter) {
      throw std::runtime_error(
          "typed declare-target to alias is attached to another clause");
    }
    const std::vector<std::uint32_t> modifiers =
        fields.optional_u32s(ROUP_FIELD_MODIFIERS);
    const std::vector<std::string> items = required_clause_items(fields);
    fields.finish();
    if (!modifiers.empty() || items.empty()) {
      throw std::runtime_error(
          "typed declare-target to alias has an invalid payload");
    }
    OpenMPClause *raw = OpenMPToClause::addToClause(
        &directive, OMPC_TO_unspecified);
    auto *clause = dynamic_cast<OpenMPToClause *>(raw);
    if (clause == nullptr) {
      throw std::runtime_error(
          "ompparser failed to create a declare-target to clause");
    }
    for (const std::string &item : items)
      clause->addItem(item);
    record_clause(directive, raw);
    return;
  }
  case ROUP_OMP_CLAUSE_ALIAS_PROC_BIND_MASTER: {
    if (kind != OMPC_proc_bind) {
      throw std::runtime_error(
          "typed proc_bind(master) alias is attached to another clause");
    }
    const std::uint32_t canonical = fields.required_u32(ROUP_FIELD_KIND);
    fields.finish();
    if (canonical != ROUP_OMP_PROC_BIND_PRIMARY) {
      throw std::runtime_error(
          "typed proc_bind(master) alias has a non-primary payload");
    }
    OpenMPClause *raw = OpenMPProcBindClause::addProcBindClause(
        &directive, OMPC_PROC_BIND_master);
    if (raw == nullptr) {
      throw std::runtime_error(
          "ompparser failed to create a proc_bind(master) clause");
    }
    record_clause(directive, raw);
    return;
  }
  case ROUP_OMP_CLAUSE_ALIAS_REDUCTION_ORIGINAL_POSITIONAL:
    if (kind != OMPC_reduction) {
      throw std::runtime_error(
          "typed positional original alias is attached to another clause");
    }
    convert_reduction(directive, kind, fields, true);
    return;
  default:
    throw std::runtime_error("unknown typed OpenMP source alias " +
                             std::to_string(source_alias));
  }
}

std::unique_ptr<OpenMPDirective>
convert_directive(RoupDirectiveHandle source) {
  const RoupU32Result dialect = require_value(
      roup_directive_dialect(source), "querying directive dialect");
  if (dialect.value != ROUP_DIALECT_OPENMP) {
    throw std::runtime_error("ompparser adapter received a non-OpenMP directive");
  }
  const std::uint32_t ordinal = directive_ordinal(source);
  const OpenMPDirectiveKind kind = directive_kind(ordinal);
  const std::uint32_t source_alias =
      require_value(roup_directive_source_alias(source),
                    "querying directive source alias")
          .value;
  std::unique_ptr<OpenMPDirective> target = make_directive(
      source_facing_directive_kind(kind, source_alias), ordinal);
  target->setBaseLang(current_lang);
  target->setNormalizeClauses(normalize_clauses_global);
  apply_directive_source_alias(*target, kind, source_alias);
  const RoupSpan span =
      require_value(roup_directive_span(source),
                    "querying directive source span")
          .value;
  set_source_location(*target, span, "directive source location");

  convert_parameter(*target, source, kind);
  const std::size_t clauses =
      require_value(roup_directive_clause_count(source),
                    "querying directive clause count")
          .value;
  for (std::size_t index = 0; index < clauses; ++index) {
    convert_clause(*target, source, index);
  }
  return target;
}

bool starts_with_case_insensitive(const std::string &text,
                                  const char *prefix) {
  const std::size_t length = std::strlen(prefix);
  if (text.size() < length)
    return false;
  for (std::size_t index = 0; index < length; ++index) {
    if (std::tolower(static_cast<unsigned char>(text[index])) !=
        std::tolower(static_cast<unsigned char>(prefix[index])))
      return false;
  }
  return true;
}

RoupParserOptions parser_options(const std::string &input) {
  const std::size_t first = input.find_first_not_of(" \t\r\n");
  if (first == std::string::npos)
    throw std::invalid_argument("parseOpenMP input must contain a directive");
  const std::string leading = input.substr(first);

  RoupParserOptions options{};
  options.abi_version = ROUP_ABI_VERSION;
  options.struct_size = static_cast<std::uint32_t>(sizeof(options));
  options.dialect = ROUP_DIALECT_OPENMP;
  options.version_policy = ROUP_VERSION_ANY;
  options.version = 0;
  options.flags = ROUP_PARSER_SOURCE_COMPATIBILITY;

  if (starts_with_case_insensitive(leading, "!$")) {
    if (current_lang != Lang_Fortran) {
      throw std::invalid_argument(
          "Fortran OpenMP sentinel conflicts with selected base language");
    }
    options.host_language = ROUP_HOST_FORTRAN;
    options.host_standard = ROUP_FORTRAN_2023;
    options.source_form = ROUP_SOURCE_FORTRAN_FREE;
  } else if (starts_with_case_insensitive(leading, "c$") ||
             starts_with_case_insensitive(leading, "*$")) {
    if (current_lang != Lang_Fortran) {
      throw std::invalid_argument(
          "Fortran OpenMP sentinel conflicts with selected base language");
    }
    options.host_language = ROUP_HOST_FORTRAN;
    options.host_standard = ROUP_FORTRAN_2023;
    options.source_form = ROUP_SOURCE_FORTRAN_FIXED;
  } else if (starts_with_case_insensitive(leading, "#pragma")) {
    if (current_lang != Lang_C && current_lang != Lang_Cplusplus) {
      throw std::invalid_argument(
          "C/C++ OpenMP pragma conflicts with selected base language");
    }
    options.host_language =
        current_lang == Lang_Cplusplus ? ROUP_HOST_CPP : ROUP_HOST_C;
    options.host_standard =
        current_lang == Lang_Cplusplus ? ROUP_CPP_23 : ROUP_C_23;
    options.source_form = ROUP_SOURCE_PRAGMA;
  } else {
    throw std::invalid_argument(
        "OpenMP input must include a pragma or Fortran sentinel");
  }
  return options;
}

} // namespace

extern "C" void setLang(OpenMPBaseLang lang) {
  if (lang != Lang_C && lang != Lang_Cplusplus && lang != Lang_Fortran &&
      lang != Lang_unknown) {
    throw std::invalid_argument("unsupported ompparser base language");
  }
  current_lang = lang;
}

extern "C" OpenMPDirective *
parseOpenMP(const char *input, OpenMPExprParseCallback expression_parser,
            void *expression_parser_data) {
  if (input == nullptr)
    return nullptr;
  const std::size_t length = std::strlen(input);
  if (length == 0)
    return nullptr;

  if (current_lang == Lang_unknown)
    return nullptr;

  openmpSetExprParseCallback(expression_parser, expression_parser_data);
  openmpSetExprParseMode(OMP_EXPR_PARSE_none);

  const RoupParserOptions options = parser_options(std::string(input, length));
  const RoupParserResult parser = require_value(
      roup_parser_create(options), "creating ROUP parser");
  bool parser_live = true;
  RoupDirectiveHandle directive_handle{};
  bool directive_live = false;
  try {
    const RoupDirectiveResult parsed =
        roup_parse(parser.value,
                   reinterpret_cast<const std::uint8_t *>(input), length);
    if (parsed.result.status == ROUP_STATUS_PARSE_ERROR) {
      release_error_without_recursion(parsed.result.error);
      require_ok(roup_parser_release(parser.value), "releasing ROUP parser");
      parser_live = false;
      return nullptr;
    }
    require_ok(parsed.result, "parsing OpenMP directive");
    directive_handle = parsed.value;
    directive_live = true;

    std::unique_ptr<OpenMPDirective> converted =
        convert_directive(directive_handle);
    require_ok(roup_directive_release(directive_handle),
               "releasing parsed ROUP directive");
    directive_live = false;
    require_ok(roup_parser_release(parser.value), "releasing ROUP parser");
    parser_live = false;
    return converted.release();
  } catch (...) {
    if (directive_live) {
      discard_cleanup_result(roup_directive_release(directive_handle));
    }
    if (parser_live) {
      discard_cleanup_result(roup_parser_release(parser.value));
    }
    return nullptr;
  }
}
