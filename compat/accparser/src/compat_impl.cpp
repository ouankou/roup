/*
 * Strict accparser adapter for the opaque ROUP C ABI.
 *
 * The adapter consumes only typed semantic fields. It never observes Rust
 * layouts, fabricates a source prefix, reparses rendered text, repairs an
 * unparse, or turns a failure into a null result.
 */

#include <OpenACCParser.h>
#include <roup.h>

#include "typed_contract.h"

#include <algorithm>
#include <cctype>
#include <cstdint>
#include <cstring>
#include <limits>
#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <type_traits>
#include <utility>
#include <vector>

namespace {

thread_local OpenACCBaseLang current_lang = ACC_Lang_unknown;

int location_component(std::size_t value, const char *context) {
  if (value == 0 ||
      value > static_cast<std::size_t>(std::numeric_limits<int>::max())) {
    throw std::runtime_error(std::string(context) +
                             " is outside accparser's location range");
  }
  return static_cast<int>(value);
}

void set_source_location(ACC_SourceLocation &target, const RoupSpan &span,
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

std::string host_node_legacy_spelling(RoupNodeHandle node,
                                      std::uint32_t outer_field) {
  const RoupNodeKind kind =
      require_value(roup_node_kind(node), "querying host node kind").value;
  std::uint32_t spelling_field = 0;
  switch (kind.family) {
  case ROUP_NODE_FAMILY_HOST_EXPRESSION:
    spelling_field = outer_field == ROUP_FIELD_STEP
                         ? ROUP_FIELD_COMPACT_SPELLING
                         : ROUP_FIELD_SOURCE_SPELLING;
    break;
  case ROUP_NODE_FAMILY_HOST_VARIABLE:
  case ROUP_NODE_FAMILY_HOST_LVALUE:
    // accparser preserves the original spelling of data-clause variables in
    // its public string-based AST. Keep that legacy projection at this final
    // adapter boundary while the ROUP node itself remains fully typed.
    spelling_field = ROUP_FIELD_SOURCE_SPELLING;
    break;
  case ROUP_NODE_FAMILY_HOST_TYPE_NAME:
    spelling_field = ROUP_FIELD_CANONICAL_SPELLING;
    break;
  default:
    throw std::runtime_error(
        "legacy string projection requested for a non-host semantic node");
  }
  const std::size_t count =
      require_value(roup_node_field_count(node), "querying host node fields")
          .value;
  for (std::size_t index = 0; index < count; ++index) {
    const RoupFieldInfo info =
        require_value(roup_node_field_info(node, index),
                      "querying host node field metadata")
            .value;
    if (info.id != spelling_field)
      continue;
    if (info.value_kind != ROUP_FIELD_VALUE_STRING || info.count != 1) {
      throw std::runtime_error("host node source spelling has invalid shape");
    }
    return copy_string(
        [&] { return roup_node_field_string_length(node, index, 0); },
        [&](std::uint8_t *output, std::size_t capacity) {
          return roup_node_field_string_copy(node, index, 0, output, capacity);
        },
        "copying typed host node source spelling");
  }
  throw std::runtime_error("typed host node has no source spelling");
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

  std::optional<std::size_t> find(std::uint32_t id) const {
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
    if (info.count != 1 ||
        (info.value_kind != ROUP_FIELD_VALUE_STRING &&
         info.value_kind != ROUP_FIELD_VALUE_NODE)) {
      throw std::runtime_error("typed field " + std::to_string(id) +
                               " is not a single string or node");
    }
    if (info.value_kind == ROUP_FIELD_VALUE_STRING)
      return string_at(*index, 0);
    const RoupNodeResult acquired = require_value(
        field_node(*index, 0), "acquiring a typed host-language node");
    try {
      std::string result = host_node_legacy_spelling(acquired.value, id);
      require_ok(roup_node_release(acquired.value),
                 "releasing a typed host-language node");
      return result;
    } catch (...) {
      discard_cleanup_result(roup_node_release(acquired.value));
      throw;
    }
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
    if (info.value_kind != ROUP_FIELD_VALUE_STRING_LIST &&
        info.value_kind != ROUP_FIELD_VALUE_NODE_LIST) {
      throw std::runtime_error("typed field " + std::to_string(id) +
                               " is not a string or node list");
    }
    std::vector<std::string> values;
    values.reserve(info.count);
    for (std::size_t value = 0; value < info.count; ++value) {
      if (info.value_kind == ROUP_FIELD_VALUE_STRING_LIST) {
        values.push_back(string_at(*index, value));
      } else {
        const RoupNodeResult acquired = require_value(
            field_node(*index, value), "acquiring a typed host-language node");
        try {
          values.push_back(host_node_legacy_spelling(acquired.value, id));
          require_ok(roup_node_release(acquired.value),
                     "releasing a typed host-language node");
        } catch (...) {
          discard_cleanup_result(roup_node_release(acquired.value));
          throw;
        }
      }
    }
    return values;
  }

  std::vector<std::string> required_strings(std::uint32_t id) {
    if (!find(id).has_value()) {
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
                         "querying a typed OpenACC u32 field")
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
                                     "querying a typed OpenACC u32-list field")
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
        field_bool(*index, 0), "querying a typed OpenACC boolean field");
    if (value.value > 1) {
      throw std::runtime_error("typed boolean field has a non-boolean value");
    }
    return value.value != 0;
  }

  bool required_bool(std::uint32_t id) {
    const std::optional<bool> value = optional_bool(id);
    if (!value.has_value()) {
      throw std::runtime_error("missing required typed boolean field " +
                               std::to_string(id));
    }
    return *value;
  }

  template <typename Convert>
  void required_node(std::uint32_t id, Convert convert) {
    const std::optional<std::size_t> index = find(id);
    if (!index.has_value()) {
      throw std::runtime_error("missing required typed node field " +
                               std::to_string(id));
    }
    consume(*index);
    const RoupFieldInfo info = fields_[*index];
    if (info.value_kind != ROUP_FIELD_VALUE_NODE || info.count != 1) {
      throw std::runtime_error("typed field " + std::to_string(id) +
                               " is not one semantic node");
    }
    convert_node(*index, 0, convert);
  }

  template <typename Convert>
  void for_each_node_list(std::uint32_t id, Convert convert) {
    const std::optional<std::size_t> index = find(id);
    if (!index.has_value()) {
      return;
    }
    consume(*index);
    const RoupFieldInfo info = fields_[*index];
    if (info.value_kind != ROUP_FIELD_VALUE_NODE_LIST) {
      throw std::runtime_error("typed field " + std::to_string(id) +
                               " is not a semantic node list");
    }
    for (std::size_t value = 0; value < info.count; ++value) {
      convert_node(*index, value, convert);
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
    const std::size_t count =
        require_value(field_count(), "querying OpenACC typed field count")
            .value;
    fields_.reserve(count);
    consumed_.assign(count, false);
    for (std::size_t index = 0; index < count; ++index) {
      fields_.push_back(require_value(field_info(index),
                                      "querying OpenACC field metadata")
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
    if (scope_ == FieldScope::Clause) {
      return roup_clause_field_count(directive_, clause_index_);
    }
    if (scope_ == FieldScope::Parameter)
      return roup_directive_parameter_field_count(directive_);
    return roup_node_field_count(node_);
  }

  RoupFieldInfoResult field_info(std::size_t index) const {
    if (scope_ == FieldScope::Clause) {
      return roup_clause_field_info(directive_, clause_index_, index);
    }
    if (scope_ == FieldScope::Parameter)
      return roup_directive_parameter_field_info(directive_, index);
    return roup_node_field_info(node_, index);
  }

  RoupU32Result field_bool(std::size_t field, std::size_t value) const {
    if (scope_ == FieldScope::Clause) {
      return roup_clause_field_bool(directive_, clause_index_, field, value);
    }
    if (scope_ == FieldScope::Parameter)
      return roup_directive_parameter_field_bool(directive_, field, value);
    return roup_node_field_bool(node_, field, value);
  }

  RoupU32Result field_u32(std::size_t field, std::size_t value) const {
    if (scope_ == FieldScope::Clause) {
      return roup_clause_field_u32(directive_, clause_index_, field, value);
    }
    if (scope_ == FieldScope::Parameter)
      return roup_directive_parameter_field_u32(directive_, field, value);
    return roup_node_field_u32(node_, field, value);
  }

  std::string string_at(std::size_t field, std::size_t value) const {
    if (scope_ == FieldScope::Clause) {
      return copy_string(
          [&] {
            return roup_clause_field_string_length(directive_, clause_index_,
                                                   field, value);
          },
          [&](std::uint8_t *output, std::size_t capacity) {
            return roup_clause_field_string_copy(
                directive_, clause_index_, field, value, output, capacity);
          },
          "copying a typed OpenACC clause string");
    }
    if (scope_ == FieldScope::Parameter) {
      return copy_string(
          [&] {
            return roup_directive_parameter_field_string_length(directive_,
                                                                field, value);
          },
          [&](std::uint8_t *output, std::size_t capacity) {
            return roup_directive_parameter_field_string_copy(
                directive_, field, value, output, capacity);
          },
          "copying a typed OpenACC directive-parameter string");
    }
    return copy_string(
        [&] { return roup_node_field_string_length(node_, field, value); },
        [&](std::uint8_t *output, std::size_t capacity) {
          return roup_node_field_string_copy(node_, field, value, output,
                                             capacity);
        },
        "copying a typed OpenACC semantic-node string");
  }

  RoupNodeResult field_node(std::size_t field, std::size_t value) const {
    if (scope_ == FieldScope::Clause)
      return roup_clause_field_node(directive_, clause_index_, field, value);
    if (scope_ == FieldScope::Parameter)
      return roup_directive_parameter_field_node(directive_, field, value);
    return roup_node_field_node(node_, field, value);
  }

  template <typename Convert>
  void convert_node(std::size_t field, std::size_t value, Convert convert) {
    const RoupNodeResult acquired = require_value(
        field_node(field, value), "acquiring an OpenACC semantic node");
    try {
      convert(acquired.value);
    } catch (...) {
      discard_cleanup_result(roup_node_release(acquired.value));
      throw;
    }
    require_ok(roup_node_release(acquired.value),
               "releasing an OpenACC semantic node");
  }

  FieldScope scope_;
  RoupDirectiveHandle directive_;
  std::size_t clause_index_;
  RoupNodeHandle node_;
  std::vector<RoupFieldInfo> fields_;
  std::vector<bool> consumed_;
};

template <typename Clause, typename = void>
struct HasMergeClause : std::false_type {};

template <typename Clause>
struct HasMergeClause<
    Clause,
    std::void_t<decltype(std::declval<Clause &>().mergeClause(
        std::declval<OpenACCDirective *>(),
        std::declval<OpenACCClause *>()))>> : std::true_type {};

template <typename Clause>
void attach_clause(OpenACCDirective &directive,
                   std::unique_ptr<Clause> clause, const RoupSpan &span) {
  OpenACCClause *base = clause.get();
  if (base->getKind() == ACCC_unknown) {
    throw std::runtime_error("refusing to attach an unknown OpenACC clause");
  }
  set_source_location(*clause, span, "OpenACC clause source location");
  std::vector<OpenACCClause *> *by_kind =
      directive.getClauses(base->getKind());
  std::vector<OpenACCClause *> *order =
      directive.getClausesInOriginalOrder();
  by_kind->push_back(base);
  order->push_back(base);
  base->setClausePosition(static_cast<int>(order->size() - 1));
  clause.release();
  if constexpr (HasMergeClause<Clause>::value) {
    if (by_kind->size() < 2)
      return;
    // Upstream mergeClause implementations own the consumption decision. A
    // consumed incoming clause is removed from both directive vectors and
    // deleted by mergeClause; an unconsumed clause remains in both vectors and
    // is deleted by OpenACCDirective's destructor. Unconditionally popping or
    // deleting here would double-delete merged clauses and orphan distinct
    // occurrences.
    static_cast<Clause *>(by_kind->front())->mergeClause(&directive, base);
  } else if constexpr (std::is_same_v<Clause, OpenACCClause>) {
    if (OpenACCDirective::getClauseMerging() && by_kind->size() > 1) {
      by_kind->pop_back();
      order->pop_back();
      delete base;
    }
  }
}

void require_nonempty(const std::string &value, const char *subject) {
  if (value.empty()) {
    throw std::runtime_error(std::string(subject) + " must not be empty");
  }
}

void require_nonempty(const std::vector<std::string> &values,
                      const char *subject) {
  if (values.empty()) {
    throw std::runtime_error(std::string(subject) + " must not be empty");
  }
  for (const std::string &value : values) {
    require_nonempty(value, subject);
  }
}

std::string read_clause_item(RoupNodeHandle node) {
  const RoupNodeKind kind =
      require_value(roup_node_kind(node),
                    "querying a typed clause-item node kind")
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
  case ROUP_CLAUSE_ITEM_OMPPARSER_TRAILING_SLASH:
    value = fields.required_string(ROUP_FIELD_NAME) + "/";
    break;
  default:
    throw std::runtime_error("unknown typed clause-item variant");
  }
  fields.finish();
  require_nonempty(value, "OpenACC clause item");
  return value;
}

std::vector<std::string> required_clause_items(FieldReader &fields) {
  if (!fields.find(ROUP_FIELD_ITEMS).has_value()) {
    throw std::runtime_error("missing required typed clause-item field");
  }
  std::vector<std::string> values;
  fields.for_each_node_list(ROUP_FIELD_ITEMS, [&](RoupNodeHandle node) {
    values.push_back(read_clause_item(node));
  });
  return values;
}

template <typename Clause>
void add_vars(Clause &clause, const std::vector<std::string> &items) {
  require_nonempty(items, "OpenACC variable list");
  for (const std::string &item : items) {
    clause.addVar(item);
  }
  if (clause.getVars().size() != items.size()) {
    throw std::runtime_error(
        "accparser silently altered a typed OpenACC variable list");
  }
}

template <typename Clause>
void fill_data_clause(Clause &clause, OpenACCDataClauseVariant variant,
                      const std::vector<std::uint32_t> &modifiers,
                      const std::vector<std::string> &items) {
  clause.setVariant(variant);
  for (const std::uint32_t modifier : modifiers) {
    clause.addModifier(roup_acc_contract::data_modifier(modifier));
  }
  if (clause.getModifiers().size() != modifiers.size()) {
    throw std::runtime_error(
        "accparser silently altered typed OpenACC data modifiers");
  }
  add_vars(clause, items);
}

OpenACCDataClauseVariant
source_data_variant(std::optional<std::uint32_t> alias,
                    OpenACCClauseKind clause_kind,
                    OpenACCDataClauseVariant canonical) {
  if (!alias.has_value())
    return canonical;
  switch (*alias) {
  case ROUP_ACC_CLAUSE_ALIAS_PCOPY:
    if (clause_kind == ACCC_copy)
      return ACCC_DATA_COPY_pcopy;
    break;
  case ROUP_ACC_CLAUSE_ALIAS_PRESENT_OR_COPY:
    if (clause_kind == ACCC_copy)
      return ACCC_DATA_COPY_present_or_copy;
    break;
  case ROUP_ACC_CLAUSE_ALIAS_PCOPYIN:
    if (clause_kind == ACCC_copyin)
      return ACCC_DATA_COPYIN_pcopyin;
    break;
  case ROUP_ACC_CLAUSE_ALIAS_PRESENT_OR_COPYIN:
    if (clause_kind == ACCC_copyin)
      return ACCC_DATA_COPYIN_present_or_copyin;
    break;
  case ROUP_ACC_CLAUSE_ALIAS_PCOPYOUT:
    if (clause_kind == ACCC_copyout)
      return ACCC_DATA_COPYOUT_pcopyout;
    break;
  case ROUP_ACC_CLAUSE_ALIAS_PRESENT_OR_COPYOUT:
    if (clause_kind == ACCC_copyout)
      return ACCC_DATA_COPYOUT_present_or_copyout;
    break;
  case ROUP_ACC_CLAUSE_ALIAS_PCREATE:
    if (clause_kind == ACCC_create)
      return ACCC_DATA_CREATE_pcreate;
    break;
  case ROUP_ACC_CLAUSE_ALIAS_PRESENT_OR_CREATE:
    if (clause_kind == ACCC_create)
      return ACCC_DATA_CREATE_present_or_create;
    break;
  default:
    break;
  }
  throw std::runtime_error(
      "typed OpenACC source alias disagrees with its canonical clause");
}

struct AccSizeValue {
  bool automatic;
  std::string expression;
};

struct AccCacheValue {
  std::uint32_t kind;
  std::string variable;
};

struct AccBindValue {
  std::string value;
  bool is_string_literal;
};

std::string encode_bind_literal(const std::string &value,
                                OpenACCBaseLang language) {
  std::string encoded;
  for (const unsigned char byte : value) {
    if (language == ACC_Lang_Fortran) {
      if (byte == '"')
        encoded += "\"\"";
      else
        encoded.push_back(static_cast<char>(byte));
      continue;
    }
    switch (byte) {
    case '\\':
      encoded += "\\\\";
      break;
    case '"':
      encoded += "\\\"";
      break;
    case '\n':
      encoded += "\\n";
      break;
    case '\r':
      encoded += "\\r";
      break;
    case '\t':
      encoded += "\\t";
      break;
    case '\a':
      encoded += "\\a";
      break;
    case '\b':
      encoded += "\\b";
      break;
    case '\v':
      encoded += "\\v";
      break;
    case '\f':
      encoded += "\\f";
      break;
    case '\0':
      encoded += "\\000";
      break;
    default:
      encoded.push_back(static_cast<char>(byte));
      break;
    }
  }
  return encoded;
}

AccBindValue read_acc_bind_target(FieldReader &fields,
                                  OpenACCBaseLang language) {
  if (!fields.find(ROUP_FIELD_VALUE).has_value()) {
    throw std::runtime_error("missing typed OpenACC bind target");
  }
  std::optional<AccBindValue> result;
  fields.required_node(ROUP_FIELD_VALUE, [&](RoupNodeHandle node) {
    if (result.has_value()) {
      throw std::runtime_error("OpenACC bind clause has duplicate targets");
    }
    const RoupNodeKind kind =
        require_value(roup_node_kind(node),
                      "querying typed OpenACC bind-target kind")
            .value;
    if (kind.family != ROUP_NODE_FAMILY_ACC_BIND_TARGET) {
      throw std::runtime_error("OpenACC bind target has the wrong node family");
    }
    if (kind.variant != ROUP_ACC_BIND_NAME &&
        kind.variant != ROUP_ACC_BIND_STRING_LITERAL) {
      throw std::runtime_error("unknown typed OpenACC bind-target variant " +
                               std::to_string(kind.variant));
    }
    FieldReader node_fields = FieldReader::node(node);
    std::string value = node_fields.required_string(ROUP_FIELD_VALUE);
    if (kind.variant == ROUP_ACC_BIND_STRING_LITERAL) {
      const std::uint32_t encoding =
          node_fields.required_u32(ROUP_FIELD_ENCODING);
      const std::uint32_t quote_style =
          node_fields.required_u32(ROUP_FIELD_QUOTE_STYLE);
      roup_acc_contract::validate_bind_encoding(encoding, language);
      if ((language == ACC_Lang_C || language == ACC_Lang_Cplusplus) &&
          quote_style != ROUP_QUOTE_DOUBLE) {
        throw std::runtime_error(
            "C and C++ OpenACC bind strings require double quotes");
      }
      if (language == ACC_Lang_Fortran &&
          quote_style != ROUP_QUOTE_SINGLE &&
          quote_style != ROUP_QUOTE_DOUBLE) {
        throw std::runtime_error(
            "unknown typed Fortran OpenACC bind quote style");
      }
      value = encode_bind_literal(value, language);
    } else {
      require_nonempty(value, "OpenACC bind name");
    }
    node_fields.finish();
    result = {std::move(value),
              kind.variant == ROUP_ACC_BIND_STRING_LITERAL};
  });
  if (!result.has_value()) {
    throw std::runtime_error("OpenACC bind clause is missing its target");
  }
  return std::move(*result);
}

std::vector<AccCacheValue> read_acc_cache_items(FieldReader &fields) {
  if (!fields.find(ROUP_FIELD_ITEMS).has_value()) {
    throw std::runtime_error("missing typed OpenACC cache-item list");
  }
  std::vector<AccCacheValue> items;
  fields.for_each_node_list(ROUP_FIELD_ITEMS, [&](RoupNodeHandle node) {
    const RoupNodeKind kind =
        require_value(roup_node_kind(node),
                      "querying typed OpenACC cache-item kind")
            .value;
    if (kind.family != ROUP_NODE_FAMILY_ACC_CACHE_ITEM) {
      throw std::runtime_error("OpenACC cache item has the wrong node family");
    }
    if (kind.variant != ROUP_ACC_CACHE_SCALAR &&
        kind.variant != ROUP_ACC_CACHE_ARRAY_ELEMENT &&
        kind.variant != ROUP_ACC_CACHE_CONTIGUOUS_SUBARRAY) {
      throw std::runtime_error("unknown typed OpenACC cache-item variant " +
                               std::to_string(kind.variant));
    }
    FieldReader node_fields = FieldReader::node(node);
    std::string variable =
        node_fields.required_string(ROUP_FIELD_VARIABLE);
    node_fields.finish();
    require_nonempty(variable, "OpenACC cache item");
    items.push_back({kind.variant, std::move(variable)});
  });
  return items;
}

OpenACCDirectiveKind read_acc_end_kind(FieldReader &fields) {
  if (!fields.find(ROUP_FIELD_KIND).has_value()) {
    throw std::runtime_error("missing typed OpenACC end kind");
  }
  std::optional<OpenACCDirectiveKind> result;
  fields.required_node(ROUP_FIELD_KIND, [&](RoupNodeHandle node) {
    if (result.has_value()) {
      throw std::runtime_error("OpenACC end directive has duplicate kinds");
    }
    const RoupNodeKind kind =
        require_value(roup_node_kind(node),
                      "querying typed OpenACC end-kind node")
            .value;
    if (kind.family != ROUP_NODE_FAMILY_ACC_END_KIND) {
      throw std::runtime_error("OpenACC end kind has the wrong node family");
    }
    FieldReader node_fields = FieldReader::node(node);
    node_fields.finish();
    switch (kind.variant) {
    case ROUP_ACC_END_ATOMIC:
      result = ACCD_atomic;
      return;
    case ROUP_ACC_END_DATA:
      result = ACCD_data;
      return;
    case ROUP_ACC_END_HOST_DATA:
      result = ACCD_host_data;
      return;
    case ROUP_ACC_END_KERNELS:
      result = ACCD_kernels;
      return;
    case ROUP_ACC_END_KERNELS_LOOP:
      result = ACCD_kernels_loop;
      return;
    case ROUP_ACC_END_LOOP:
      result = ACCD_loop;
      return;
    case ROUP_ACC_END_PARALLEL:
      result = ACCD_parallel;
      return;
    case ROUP_ACC_END_PARALLEL_LOOP:
      result = ACCD_parallel_loop;
      return;
    case ROUP_ACC_END_SERIAL:
      result = ACCD_serial;
      return;
    case ROUP_ACC_END_SERIAL_LOOP:
      result = ACCD_serial_loop;
      return;
    default:
      throw std::runtime_error("unknown typed OpenACC end-kind variant " +
                               std::to_string(kind.variant));
    }
  });
  if (!result.has_value()) {
    throw std::runtime_error("OpenACC end directive is missing its kind");
  }
  return *result;
}

AccSizeValue read_acc_size(RoupNodeHandle node) {
  const RoupNodeKind kind =
      require_value(roup_node_kind(node), "querying typed OpenACC size-node kind")
          .value;
  if (kind.family != ROUP_NODE_FAMILY_ACC_SIZE_EXPRESSION) {
    throw std::runtime_error("OpenACC size has the wrong node family");
  }
  FieldReader node_fields = FieldReader::node(node);
  if (kind.variant == ROUP_ACC_SIZE_AUTOMATIC) {
    node_fields.finish();
    return {true, {}};
  }
  if (kind.variant == ROUP_ACC_SIZE_EXPRESSION) {
    std::string expression = node_fields.required_string(ROUP_FIELD_VALUE);
    node_fields.finish();
    require_nonempty(expression, "OpenACC size expression");
    return {false, std::move(expression)};
  }
  throw std::runtime_error("unknown typed OpenACC size-node variant " +
                           std::to_string(kind.variant));
}

std::vector<AccSizeValue> read_acc_sizes(FieldReader &fields) {
  if (!fields.find(ROUP_FIELD_VALUES).has_value()) {
    throw std::runtime_error("missing typed OpenACC size-node list");
  }
  std::vector<AccSizeValue> values;
  fields.for_each_node_list(ROUP_FIELD_VALUES, [&](RoupNodeHandle node) {
    values.push_back(read_acc_size(node));
  });
  return values;
}

struct AccGangValue {
  std::uint32_t kind;
  AccSizeValue size;
};

std::vector<AccGangValue> read_acc_gang_arguments(FieldReader &fields) {
  if (!fields.find(ROUP_FIELD_ARGUMENTS).has_value()) {
    throw std::runtime_error("missing typed OpenACC gang argument list");
  }
  std::vector<AccGangValue> arguments;
  fields.for_each_node_list(ROUP_FIELD_ARGUMENTS, [&](RoupNodeHandle node) {
    const RoupNodeKind kind =
        require_value(roup_node_kind(node),
                      "querying typed OpenACC gang argument kind")
            .value;
    if (kind.family != ROUP_NODE_FAMILY_ACC_GANG_ARGUMENT) {
      throw std::runtime_error("OpenACC gang argument has the wrong node family");
    }
    FieldReader node_fields = FieldReader::node(node);
    std::optional<AccSizeValue> size;
    node_fields.required_node(ROUP_FIELD_VALUE, [&](RoupNodeHandle size_node) {
      if (size.has_value()) {
        throw std::runtime_error("OpenACC gang argument has duplicate values");
      }
      size = read_acc_size(size_node);
    });
    node_fields.finish();
    if (!size.has_value()) {
      throw std::runtime_error("OpenACC gang argument is missing its value");
    }
    arguments.push_back({kind.variant, std::move(*size)});
  });
  return arguments;
}

struct AccDeviceTypeValue {
  bool named;
  OpenACCDeviceTypeKind builtin;
  std::string name;
};

std::vector<AccDeviceTypeValue> read_acc_device_types(FieldReader &fields) {
  if (!fields.find(ROUP_FIELD_VALUES).has_value()) {
    throw std::runtime_error("missing typed OpenACC device-type list");
  }
  std::vector<AccDeviceTypeValue> values;
  fields.for_each_node_list(ROUP_FIELD_VALUES, [&](RoupNodeHandle node) {
    const RoupNodeKind kind =
        require_value(roup_node_kind(node),
                      "querying typed OpenACC device-type kind")
            .value;
    if (kind.family != ROUP_NODE_FAMILY_ACC_DEVICE_TYPE) {
      throw std::runtime_error(
          "OpenACC device type has the wrong semantic node family");
    }
    FieldReader node_fields = FieldReader::node(node);
    if (kind.variant == ROUP_ACC_DEVICE_TYPE_NAMED) {
      std::string name = node_fields.required_string(ROUP_FIELD_NAME);
      node_fields.finish();
      require_nonempty(name, "OpenACC named device type");
      values.push_back(
          {true, ACCC_DEVICE_TYPE_unknown, std::move(name)});
      return;
    }
    const OpenACCDeviceTypeKind builtin =
        roup_acc_contract::builtin_device_type(kind.variant);
    node_fields.finish();
    values.push_back({false, builtin, {}});
  });
  return values;
}

OpenACCReductionClauseOperator
read_acc_reduction_operator(FieldReader &fields) {
  std::optional<OpenACCReductionClauseOperator> result;
  fields.required_node(ROUP_FIELD_OPERATOR, [&](RoupNodeHandle node) {
    const RoupNodeKind kind =
        require_value(roup_node_kind(node),
                      "querying typed OpenACC reduction-operator kind")
            .value;
    if (kind.family != ROUP_NODE_FAMILY_ACC_REDUCTION_OPERATOR) {
      throw std::runtime_error(
          "OpenACC reduction operator has the wrong semantic node family");
    }
    FieldReader node_fields = FieldReader::node(node);
    node_fields.finish();
    result = roup_acc_contract::builtin_reduction_operator(kind.variant);
  });
  if (!result.has_value()) {
    throw std::runtime_error(
        "OpenACC reduction clause is missing its typed operator");
  }
  return *result;
}

std::unique_ptr<OpenACCDirective>
make_directive(OpenACCDirectiveKind kind, OpenACCBaseLang lang) {
  switch (kind) {
  case ACCD_cache: {
    auto directive = std::make_unique<OpenACCCacheDirective>();
    directive->setBaseLang(lang);
    return directive;
  }
  case ACCD_wait: {
    auto directive = std::make_unique<OpenACCWaitDirective>();
    directive->setBaseLang(lang);
    return directive;
  }
  case ACCD_routine: {
    auto directive = std::make_unique<OpenACCRoutineDirective>();
    directive->setBaseLang(lang);
    return directive;
  }
  case ACCD_end: {
    auto directive = std::make_unique<OpenACCEndDirective>();
    directive->setBaseLang(lang);
    return directive;
  }
  case ACCD_unknown:
    throw std::runtime_error("cannot construct an unknown OpenACC directive");
  default:
    return std::make_unique<OpenACCDirective>(kind, lang);
  }
}

std::unique_ptr<OpenACCClause> make_bare_clause(OpenACCClauseKind kind) {
  switch (kind) {
  case ACCC_async:
    return std::make_unique<OpenACCAsyncClause>();
  case ACCC_gang:
    return std::make_unique<OpenACCGangClause>();
  case ACCC_self:
    return std::make_unique<OpenACCSelfClause>();
  case ACCC_vector:
    return std::make_unique<OpenACCVectorClause>();
  case ACCC_wait:
    return std::make_unique<OpenACCWaitClause>();
  case ACCC_worker:
    return std::make_unique<OpenACCWorkerClause>();
  case ACCC_auto:
  case ACCC_capture:
  case ACCC_finalize:
  case ACCC_if_present:
  case ACCC_independent:
  case ACCC_nohost:
  case ACCC_read:
  case ACCC_seq:
  case ACCC_update:
  case ACCC_write:
    return std::make_unique<OpenACCClause>(kind);
  default:
    throw std::runtime_error("typed OpenACC clause unexpectedly has no payload");
  }
}

void convert_clause(OpenACCDirective &target, RoupDirectiveHandle source,
                    std::size_t index) {
  const RoupClauseKind typed_kind =
      require_value(roup_clause_kind(source, index),
                    "querying the typed OpenACC clause kind")
          .value;
  const OpenACCClauseKind clause_kind =
      roup_acc_contract::clause_kind(typed_kind);
  const RoupSpan span =
      require_value(roup_clause_span(source, index),
                    "querying OpenACC clause source span")
          .value;
  FieldReader fields = FieldReader::clause(source, index);
  const std::optional<std::uint32_t> source_alias =
      fields.optional_u32(ROUP_FIELD_SOURCE_ALIAS);

  switch (clause_kind) {
  case ACCC_auto:
  case ACCC_capture:
  case ACCC_finalize:
  case ACCC_if_present:
  case ACCC_independent:
  case ACCC_nohost:
  case ACCC_read:
  case ACCC_seq:
  case ACCC_update:
  case ACCC_write: {
    fields.finish();
    attach_clause(target, make_bare_clause(clause_kind), span);
    return;
  }
  case ACCC_async: {
    auto clause = std::make_unique<OpenACCAsyncClause>();
    const std::optional<std::string> value =
        fields.optional_string(ROUP_FIELD_VALUE);
    fields.finish();
    if (value.has_value()) {
      require_nonempty(*value, "OpenACC async expression");
      clause->setModifier(ACCC_ASYNC_expr);
      clause->setAsyncExpr(*value);
    }
    attach_clause(target, std::move(clause), span);
    return;
  }
  case ACCC_bind: {
    auto clause = std::make_unique<OpenACCBindClause>();
    const AccBindValue binding =
        read_acc_bind_target(fields, target.getBaseLang());
    fields.finish();
    clause->setBinding(binding.value, binding.is_string_literal);
    attach_clause(target, std::move(clause), span);
    return;
  }
  case ACCC_collapse: {
    auto clause = std::make_unique<OpenACCCollapseClause>();
    const bool force = fields.required_bool(ROUP_FIELD_FORCE);
    const std::string count = fields.required_string(ROUP_FIELD_VALUE);
    fields.finish();
    require_nonempty(count, "OpenACC collapse count");
    clause->setForce(force);
    clause->addCountExpr(count);
    attach_clause(target, std::move(clause), span);
    return;
  }
  case ACCC_copy:
  case ACCC_copyin:
  case ACCC_copyout: {
    const std::uint32_t kind_tag = fields.required_u32(ROUP_FIELD_KIND);
    const std::vector<std::uint32_t> modifiers =
        fields.required_u32s(ROUP_FIELD_MODIFIERS);
    const std::vector<std::string> items =
        required_clause_items(fields);
    fields.finish();
    const OpenACCDataClauseVariant variant = source_data_variant(
        source_alias, clause_kind,
        roup_acc_contract::copy_variant(kind_tag, clause_kind));
    if (clause_kind == ACCC_copy) {
      auto clause = std::make_unique<OpenACCCopyClause>();
      fill_data_clause(*clause, variant, modifiers, items);
      attach_clause(target, std::move(clause), span);
    } else if (clause_kind == ACCC_copyin) {
      auto clause = std::make_unique<OpenACCCopyinClause>();
      fill_data_clause(*clause, variant, modifiers, items);
      attach_clause(target, std::move(clause), span);
    } else {
      auto clause = std::make_unique<OpenACCCopyoutClause>();
      fill_data_clause(*clause, variant, modifiers, items);
      attach_clause(target, std::move(clause), span);
    }
    return;
  }
  case ACCC_create: {
    const std::uint32_t kind_tag = fields.required_u32(ROUP_FIELD_KIND);
    const std::vector<std::uint32_t> modifiers =
        fields.required_u32s(ROUP_FIELD_MODIFIERS);
    const std::vector<std::string> items =
        required_clause_items(fields);
    fields.finish();
    auto clause = std::make_unique<OpenACCCreateClause>();
    fill_data_clause(
        *clause,
        source_data_variant(source_alias, clause_kind,
                            roup_acc_contract::create_variant(kind_tag)),
        modifiers, items);
    attach_clause(target, std::move(clause), span);
    return;
  }
  case ACCC_attach:
  case ACCC_delete:
  case ACCC_detach:
  case ACCC_device:
  case ACCC_device_resident:
  case ACCC_link:
  case ACCC_use_device: {
    const std::uint32_t kind_tag = fields.required_u32(ROUP_FIELD_KIND);
    const std::vector<std::string> items =
        required_clause_items(fields);
    fields.finish();
    if (roup_acc_contract::data_clause_kind(kind_tag) != clause_kind) {
      throw std::runtime_error("typed OpenACC data kind disagrees with clause");
    }
    if (clause_kind == ACCC_device) {
      auto clause = std::make_unique<OpenACCDeviceClause>();
      require_nonempty(items, "OpenACC device list");
      for (const std::string &item : items)
        clause->addDevice(item, ACCC_CLAUSE_SEP_comma);
      if (clause->getDevices().size() != items.size())
        throw std::runtime_error("accparser altered an OpenACC device list");
      attach_clause(target, std::move(clause), span);
      return;
    }
#define ROUP_ATTACH_VAR_CASE(Kind, Class)                                    \
  if (clause_kind == Kind) {                                                 \
    auto clause = std::make_unique<Class>();                                 \
    add_vars(*clause, items);                                                \
    attach_clause(target, std::move(clause), span);                          \
    return;                                                                  \
  }
    ROUP_ATTACH_VAR_CASE(ACCC_attach, OpenACCAttachClause)
    ROUP_ATTACH_VAR_CASE(ACCC_delete, OpenACCDeleteClause)
    ROUP_ATTACH_VAR_CASE(ACCC_detach, OpenACCDetachClause)
    ROUP_ATTACH_VAR_CASE(ACCC_device_resident, OpenACCDeviceResidentClause)
    ROUP_ATTACH_VAR_CASE(ACCC_link, OpenACCLinkClause)
    ROUP_ATTACH_VAR_CASE(ACCC_use_device, OpenACCUseDeviceClause)
#undef ROUP_ATTACH_VAR_CASE
    throw std::runtime_error("unreachable OpenACC data clause kind");
  }
  case ACCC_default: {
    auto clause = std::make_unique<OpenACCDefaultClause>();
    const std::uint32_t kind_tag = fields.required_u32(ROUP_FIELD_KIND);
    fields.finish();
    clause->setKind(roup_acc_contract::default_kind(kind_tag));
    attach_clause(target, std::move(clause), span);
    return;
  }
  case ACCC_default_async:
  case ACCC_device_num:
  case ACCC_if:
  case ACCC_num_workers:
  case ACCC_vector_length: {
    const std::string value = fields.required_string(ROUP_FIELD_VALUE);
    fields.finish();
    require_nonempty(value, "OpenACC scalar expression");
    if (clause_kind == ACCC_default_async) {
      auto clause = std::make_unique<OpenACCDefaultAsyncClause>();
      clause->setAsyncExpr(value);
      attach_clause(target, std::move(clause), span);
    } else if (clause_kind == ACCC_device_num) {
      auto clause = std::make_unique<OpenACCDeviceNumClause>();
      clause->setDeviceExpr(value);
      attach_clause(target, std::move(clause), span);
    } else if (clause_kind == ACCC_if) {
      auto clause = std::make_unique<OpenACCIfClause>();
      clause->setCondition(value);
      attach_clause(target, std::move(clause), span);
    } else if (clause_kind == ACCC_num_workers) {
      auto clause = std::make_unique<OpenACCNumWorkersClause>();
      clause->setNumExpr(value);
      attach_clause(target, std::move(clause), span);
    } else {
      auto clause = std::make_unique<OpenACCVectorLengthClause>();
      clause->setLengthExpr(value);
      attach_clause(target, std::move(clause), span);
    }
    return;
  }
  case ACCC_device_type: {
    auto clause = std::make_unique<OpenACCDeviceTypeClause>();
    const std::vector<AccDeviceTypeValue> values =
        read_acc_device_types(fields);
    fields.finish();
    if (values.empty()) {
      throw std::runtime_error("OpenACC device_type list must not be empty");
    }
    bool saw_named = false;
    for (const AccDeviceTypeValue &value : values) {
      if (value.named) {
        saw_named = true;
        clause->addDeviceTypeString(value.name);
      } else {
        if (saw_named) {
          throw std::runtime_error(
              "accparser cannot preserve interleaved named and builtin "
              "OpenACC device types");
        }
        clause->addDeviceType(value.builtin);
      }
    }
    const std::size_t represented = clause->getDeviceTypes().size() +
                                    clause->getUnknownDeviceTypes().size();
    if (represented != values.size()) {
      throw std::runtime_error(
          "accparser silently altered a typed OpenACC device_type list");
    }
    attach_clause(target, std::move(clause), span);
    return;
  }
  case ACCC_deviceptr:
  case ACCC_firstprivate:
  case ACCC_no_create:
  case ACCC_present:
  case ACCC_private: {
    const std::vector<std::string> items =
        required_clause_items(fields);
    fields.finish();
#define ROUP_ITEM_CASE(Kind, Class)                                          \
  if (clause_kind == Kind) {                                                 \
    auto clause = std::make_unique<Class>();                                 \
    add_vars(*clause, items);                                                \
    attach_clause(target, std::move(clause), span);                          \
    return;                                                                  \
  }
    ROUP_ITEM_CASE(ACCC_deviceptr, OpenACCDeviceptrClause)
    ROUP_ITEM_CASE(ACCC_firstprivate, OpenACCFirstprivateClause)
    ROUP_ITEM_CASE(ACCC_no_create, OpenACCNoCreateClause)
    ROUP_ITEM_CASE(ACCC_present, OpenACCPresentClause)
    ROUP_ITEM_CASE(ACCC_private, OpenACCPrivateClause)
#undef ROUP_ITEM_CASE
    throw std::runtime_error("unreachable OpenACC item-list clause kind");
  }
  case ACCC_gang: {
    auto clause = std::make_unique<OpenACCGangClause>();
    const std::vector<AccGangValue> arguments = read_acc_gang_arguments(fields);
    fields.finish();
    if (target.getKind() == ACCD_routine &&
        (arguments.size() > 1 ||
         (!arguments.empty() && arguments.front().kind != ROUP_ACC_GANG_DIM))) {
      throw std::runtime_error(
          "OpenACC routine gang accepts only the dim argument");
    }
    for (const AccGangValue &argument : arguments) {
      OpenACCGangArgKind target_kind;
      if (argument.kind == ROUP_ACC_GANG_POSITIONAL)
        target_kind = ACCC_GANG_ARG_num_no_keyword;
      else if (argument.kind == ROUP_ACC_GANG_NUM)
        target_kind = ACCC_GANG_ARG_num;
      else if (argument.kind == ROUP_ACC_GANG_DIM)
        target_kind = ACCC_GANG_ARG_dim;
      else if (argument.kind == ROUP_ACC_GANG_STATIC)
        target_kind = ACCC_GANG_ARG_static;
      else
        throw std::runtime_error("unknown typed OpenACC gang argument variant " +
                                 std::to_string(argument.kind));
      if (argument.size.automatic && target_kind != ACCC_GANG_ARG_static) {
        throw std::runtime_error(
            "automatic OpenACC gang size requires the static modifier");
      }
      clause->addArg(target_kind, argument.size.automatic
                                      ? "*"
                                      : argument.size.expression);
    }
    attach_clause(target, std::move(clause), span);
    return;
  }
  case ACCC_num_gangs: {
    auto clause = std::make_unique<OpenACCNumGangsClause>();
    const std::vector<std::string> values =
        fields.required_strings(ROUP_FIELD_VALUES);
    fields.finish();
    require_nonempty(values, "OpenACC num_gangs list");
    for (const std::string &value : values)
      clause->addNum(value);
    attach_clause(target, std::move(clause), span);
    return;
  }
  case ACCC_reduction: {
    auto clause = std::make_unique<OpenACCReductionClause>();
    const OpenACCReductionClauseOperator op =
        read_acc_reduction_operator(fields);
    const std::vector<std::string> items =
        required_clause_items(fields);
    fields.finish();
    clause->setOperator(op);
    add_vars(*clause, items);
    attach_clause(target, std::move(clause), span);
    return;
  }
  case ACCC_self: {
    const bool has_value = fields.find(ROUP_FIELD_VALUE).has_value();
    const bool has_items = fields.find(ROUP_FIELD_ITEMS).has_value();
    if (has_value && has_items) {
      throw std::runtime_error("typed OpenACC self clause has two payloads");
    }
    if (source_alias.has_value()) {
      if (*source_alias != ROUP_ACC_CLAUSE_ALIAS_UPDATE_HOST || has_value ||
          !has_items) {
        throw std::runtime_error(
            "typed OpenACC update-host alias has an invalid payload");
      }
      const std::vector<std::string> items = required_clause_items(fields);
      fields.finish();
      auto clause = std::make_unique<OpenACCHostClause>();
      add_vars(*clause, items);
      attach_clause(target, std::move(clause), span);
      return;
    }
    auto clause = std::make_unique<OpenACCSelfClause>();
    if (has_value) {
      const std::string value = fields.required_string(ROUP_FIELD_VALUE);
      require_nonempty(value, "OpenACC self condition");
      clause->setCondition(value);
    } else if (has_items) {
      add_vars(*clause, required_clause_items(fields));
    }
    fields.finish();
    attach_clause(target, std::move(clause), span);
    return;
  }
  case ACCC_tile: {
    auto clause = std::make_unique<OpenACCTileClause>();
    const std::vector<AccSizeValue> values = read_acc_sizes(fields);
    fields.finish();
    if (values.empty())
      throw std::runtime_error("OpenACC tile size list must not be empty");
    for (const AccSizeValue &value : values)
      clause->addTileSize(value.automatic ? "*" : value.expression);
    attach_clause(target, std::move(clause), span);
    return;
  }
  case ACCC_vector:
  case ACCC_worker: {
    const std::optional<std::uint32_t> modifier =
        fields.optional_u32(ROUP_FIELD_MODIFIER);
    const std::optional<std::string> value =
        fields.optional_string(ROUP_FIELD_VALUE);
    fields.finish();
    if (modifier.has_value() != value.has_value()) {
      throw std::runtime_error(
          "typed OpenACC worker/vector modifier and expression disagree");
    }
    if (target.getKind() == ACCD_routine && value.has_value()) {
      throw std::runtime_error(
          "OpenACC routine worker/vector clauses do not accept expressions");
    }
    if (clause_kind == ACCC_vector) {
      auto clause = std::make_unique<OpenACCVectorClause>();
      if (modifier.has_value()) {
        clause->setModifier(roup_acc_contract::vector_modifier(*modifier));
      }
      if (value.has_value())
        clause->setLengthExpr(*value);
      attach_clause(target, std::move(clause), span);
    } else {
      auto clause = std::make_unique<OpenACCWorkerClause>();
      if (modifier.has_value()) {
        clause->setModifier(roup_acc_contract::worker_modifier(*modifier));
      }
      if (value.has_value())
        clause->setNumExpr(*value);
      attach_clause(target, std::move(clause), span);
    }
    return;
  }
  case ACCC_wait: {
    auto clause = std::make_unique<OpenACCWaitClause>();
    const std::optional<std::string> devnum =
        fields.optional_string(ROUP_FIELD_DEVICE_NUM);
    const std::vector<std::string> queues =
        fields.required_strings(ROUP_FIELD_VALUES);
    const bool queues_keyword =
        fields.required_bool(ROUP_FIELD_QUEUES_KEYWORD);
    fields.finish();
    if (devnum.has_value()) {
      require_nonempty(*devnum, "OpenACC wait device number");
      require_nonempty(queues, "OpenACC wait queue list");
      clause->setDevnum(*devnum);
    }
    clause->setQueues(queues_keyword);
    for (const std::string &queue : queues) {
      require_nonempty(queue, "OpenACC wait queue");
      clause->addAsyncId(queue);
    }
    attach_clause(target, std::move(clause), span);
    return;
  }
  case ACCC_host:
    throw std::runtime_error(
        "OpenACC update host must arrive as the canonical typed self clause");
  case ACCC_indirect: {
    auto clause = std::make_unique<OpenACCIndirectClause>();
    if (fields.find(ROUP_FIELD_VALUE).has_value()) {
      const AccBindValue value =
          read_acc_bind_target(fields, target.getBaseLang());
      clause->setValue(value.value, value.is_string_literal);
    }
    fields.finish();
    attach_clause(target, std::move(clause), span);
    return;
  }
  case ACCC_unknown:
    break;
  }
  throw std::runtime_error("unconverted typed OpenACC clause ordinal " +
                           std::to_string(typed_kind.ordinal));
}

void convert_parameter(OpenACCDirective &target, OpenACCDirectiveKind kind,
                       RoupDirectiveHandle source) {
  const RoupU32Result has_parameter = require_value(
      roup_directive_has_parameter(source),
      "querying OpenACC directive parameter presence");
  if (has_parameter.value > 1) {
    throw std::runtime_error("OpenACC parameter presence is not boolean");
  }
  if (has_parameter.value == 0) {
    if (kind == ACCD_cache || kind == ACCD_end) {
      throw std::runtime_error("OpenACC directive is missing its parameter");
    }
    return;
  }

  const RoupParameterKind parameter_kind =
      require_value(roup_directive_parameter_kind(source),
                    "querying typed OpenACC parameter kind")
          .value;
  if (parameter_kind.dialect != ROUP_DIALECT_OPENACC) {
    throw std::runtime_error("OpenACC adapter received a foreign parameter");
  }
  FieldReader fields = FieldReader::parameter(source);

  switch (parameter_kind.variant) {
  case ROUP_ACC_PARAMETER_CACHE: {
    if (kind != ACCD_cache)
      throw std::runtime_error("cache parameter attached to another directive");
    auto &cache = static_cast<OpenACCCacheDirective &>(target);
    const bool readonly = fields.required_bool(ROUP_FIELD_READONLY);
    const std::vector<AccCacheValue> items = read_acc_cache_items(fields);
    fields.finish();
    if (items.empty())
      throw std::runtime_error("OpenACC cache variable list is empty");
    if (readonly)
      cache.setModifier(ACCC_CACHE_readonly);
    for (const AccCacheValue &item : items)
      cache.addVar(item.variable);
    if (cache.getVars().size() != items.size()) {
      throw std::runtime_error(
          "accparser silently altered a typed cache variable list");
    }
    return;
  }
  case ROUP_ACC_PARAMETER_WAIT: {
    if (kind != ACCD_wait)
      throw std::runtime_error("wait parameter attached to another directive");
    auto &wait = static_cast<OpenACCWaitDirective &>(target);
    const std::optional<std::string> devnum =
        fields.optional_string(ROUP_FIELD_DEVICE_NUM);
    const std::vector<std::string> queues =
        fields.required_strings(ROUP_FIELD_VALUES);
    const bool queues_keyword =
        fields.required_bool(ROUP_FIELD_QUEUES_KEYWORD);
    fields.finish();
    require_nonempty(queues, "OpenACC wait directive queue list");
    if (devnum.has_value()) {
      require_nonempty(*devnum, "OpenACC wait directive device number");
      wait.setDevnum(*devnum);
    }
    wait.setQueues(queues_keyword);
    for (const std::string &queue : queues)
      wait.addAsyncId(queue);
    return;
  }
  case ROUP_ACC_PARAMETER_ROUTINE: {
    if (kind != ACCD_routine)
      throw std::runtime_error("routine parameter attached elsewhere");
    auto &routine = static_cast<OpenACCRoutineDirective &>(target);
    const std::string function = fields.required_string(ROUP_FIELD_FUNCTION);
    fields.finish();
    require_nonempty(function, "OpenACC routine function");
    routine.setName(function);
    return;
  }
  case ROUP_ACC_PARAMETER_END: {
    if (kind != ACCD_end)
      throw std::runtime_error("end parameter attached to another directive");
    auto &end = static_cast<OpenACCEndDirective &>(target);
    const OpenACCDirectiveKind paired = read_acc_end_kind(fields);
    fields.finish();
    end.setPairedDirective(new OpenACCDirective(paired,
                                                target.getBaseLang()));
    return;
  }
  default:
    throw std::runtime_error("unknown typed OpenACC parameter variant " +
                             std::to_string(parameter_kind.variant));
  }
}

std::unique_ptr<OpenACCDirective>
convert_directive(RoupDirectiveHandle source, OpenACCBaseLang lang) {
  const RoupU32Result dialect = require_value(
      roup_directive_dialect(source), "querying parsed directive dialect");
  if (dialect.value != ROUP_DIALECT_OPENACC) {
    throw std::runtime_error("OpenACC adapter received a non-OpenACC directive");
  }

  const RoupDirectiveKind typed_kind =
      require_value(roup_directive_kind(source),
                    "querying typed OpenACC directive kind")
          .value;
  const OpenACCDirectiveKind directive_kind =
      roup_acc_contract::directive_kind(typed_kind);

  std::unique_ptr<OpenACCDirective> target =
      make_directive(directive_kind, lang);
  const RoupSpan span =
      require_value(roup_directive_span(source),
                    "querying OpenACC directive source span")
          .value;
  set_source_location(*target, span, "OpenACC directive source location");
  convert_parameter(*target, directive_kind, source);

  const std::size_t clause_count =
      require_value(roup_directive_clause_count(source),
                    "querying OpenACC clause count")
          .value;
  for (std::size_t index = 0; index < clause_count; ++index) {
    convert_clause(*target, source, index);
  }
  return target;
}

struct ParseProfile {
  OpenACCBaseLang language;
  RoupParserOptions options;
};

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

ParseProfile parser_profile(const std::string &input) {
  const std::size_t first = input.find_first_not_of(" \t\r\n");
  if (first == std::string::npos)
    throw std::invalid_argument("parseOpenACC input must contain a directive");
  const std::string leading = input.substr(first);
  const bool language_is_explicit = current_lang != ACC_Lang_unknown;

  ParseProfile profile{};
  profile.options.abi_version = ROUP_ABI_VERSION;
  profile.options.struct_size = sizeof(RoupParserOptions);
  profile.options.dialect = ROUP_DIALECT_OPENACC;
  profile.options.version_policy = ROUP_VERSION_ANY;
  profile.options.version = 0;
  profile.options.flags = ROUP_PARSER_ACCPARSER_EXTENSIONS;

  if (starts_with_case_insensitive(leading, "!$acc")) {
    if (language_is_explicit && current_lang != ACC_Lang_Fortran) {
      throw std::invalid_argument(
          "Fortran OpenACC sentinel conflicts with selected base language");
    }
    profile.language = ACC_Lang_Fortran;
    profile.options.host_language = ROUP_HOST_FORTRAN;
    profile.options.host_standard = ROUP_FORTRAN_2023;
    profile.options.source_form = ROUP_SOURCE_FORTRAN_FREE;
  } else if (starts_with_case_insensitive(leading, "c$acc") ||
             starts_with_case_insensitive(leading, "*$acc")) {
    if (language_is_explicit && current_lang != ACC_Lang_Fortran) {
      throw std::invalid_argument(
          "Fortran OpenACC sentinel conflicts with selected base language");
    }
    profile.language = ACC_Lang_Fortran;
    profile.options.host_language = ROUP_HOST_FORTRAN;
    profile.options.host_standard = ROUP_FORTRAN_2023;
    profile.options.source_form = ROUP_SOURCE_FORTRAN_FIXED;
  } else if (starts_with_case_insensitive(leading, "#pragma")) {
    if (language_is_explicit && current_lang != ACC_Lang_C &&
        current_lang != ACC_Lang_Cplusplus) {
      throw std::invalid_argument(
          "C/C++ OpenACC pragma conflicts with selected base language");
    }
    profile.language = language_is_explicit ? current_lang : ACC_Lang_C;
    profile.options.host_language =
        profile.language == ACC_Lang_Cplusplus ? ROUP_HOST_CPP : ROUP_HOST_C;
    profile.options.host_standard =
        profile.options.host_language == ROUP_HOST_CPP ? ROUP_CPP_23
                                                       : ROUP_C_23;
    profile.options.source_form = ROUP_SOURCE_PRAGMA;
  } else {
    throw std::invalid_argument(
        "OpenACC input must include a pragma or Fortran sentinel");
  }
  return profile;
}

} // namespace

extern "C" void setLang(OpenACCBaseLang lang) {
  if (lang != ACC_Lang_C && lang != ACC_Lang_Cplusplus &&
      lang != ACC_Lang_Fortran) {
    throw std::invalid_argument("unsupported accparser base language");
  }
  current_lang = lang;
}

OpenACCDirective *parseOpenACC(std::string input) {
  if (input.empty()) {
    throw std::invalid_argument("parseOpenACC input must not be empty");
  }

  const ParseProfile profile = parser_profile(input);
  const RoupParserResult parser = require_value(
      roup_parser_create(profile.options), "creating ROUP OpenACC parser");
  bool parser_live = true;
  RoupDirectiveHandle directive_handle{};
  bool directive_live = false;
  try {
    const RoupDirectiveResult parsed = require_value(
        roup_parse(parser.value,
                   reinterpret_cast<const std::uint8_t *>(input.data()),
                   input.size()),
        "parsing OpenACC directive");
    directive_handle = parsed.value;
    directive_live = true;

    std::unique_ptr<OpenACCDirective> converted =
        convert_directive(directive_handle, profile.language);
    require_ok(roup_directive_release(directive_handle),
               "releasing parsed OpenACC directive");
    directive_live = false;
    require_ok(roup_parser_release(parser.value),
               "releasing ROUP OpenACC parser");
    parser_live = false;
    return converted.release();
  } catch (...) {
    if (directive_live) {
      discard_cleanup_result(roup_directive_release(directive_handle));
    }
    if (parser_live) {
      discard_cleanup_result(roup_parser_release(parser.value));
    }
    throw;
  }
}

std::string trimEnclosingWhiteSpace(std::string value) {
  const std::size_t first = value.find_first_not_of(" \t\r\n");
  if (first == std::string::npos)
    return {};
  const std::size_t last = value.find_last_not_of(" \t\r\n");
  return value.substr(first, last - first + 1);
}
