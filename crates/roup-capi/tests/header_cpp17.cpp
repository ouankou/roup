#include "roup.h"

#include <array>
#include <cstdint>

static_assert(ROUP_FIELD_VALUE_U32 == UINT32_C(2), "u32 field tag drift");
static_assert(ROUP_FIELD_VALUE_U32_LIST == UINT32_C(5), "u32-list field tag drift");
static_assert(sizeof(&roup_directive_parameter_field_u32) > 0U,
              "parameter u32 query missing");
static_assert(sizeof(&roup_clause_field_u32) > 0U, "clause u32 query missing");
static_assert(sizeof(&roup_node_field_u32) > 0U, "node u32 query missing");

namespace {
constexpr int unexpected_field = 100;
constexpr int unexpected_node = 101;
constexpr int unexpected_value = 102;

int checked_call(RoupCallResult result) {
    if (result.status == ROUP_STATUS_OK) {
        return 0;
    }
    const RoupCallResult released = roup_error_release(result.error);
    return released.status == ROUP_STATUS_OK ? static_cast<int>(result.status)
                                             : static_cast<int>(released.status);
}

int close_parser(RoupParserHandle parser) {
    return checked_call(roup_parser_release(parser));
}

int close_directive_and_parser(RoupDirectiveHandle directive, RoupParserHandle parser) {
    const int directive_status = checked_call(roup_directive_release(directive));
    const int parser_status = close_parser(parser);
    return directive_status != 0 ? directive_status : parser_status;
}

int fail_after_directive(
    RoupDirectiveHandle directive, RoupParserHandle parser, int failure) {
    const int cleanup = close_directive_and_parser(directive, parser);
    return cleanup != 0 ? cleanup : failure;
}
} // namespace

int roup_cpp17_header_fixture() {
    RoupParserOptions options{};
    options.abi_version = ROUP_ABI_VERSION;
    options.struct_size = static_cast<std::uint32_t>(sizeof(options));
    options.dialect = ROUP_DIALECT_OPENMP;
    options.version_policy = ROUP_VERSION_ANY;
    options.host_language = ROUP_HOST_CPP;
    options.host_standard = ROUP_CPP_23;
    options.source_form = ROUP_SOURCE_PRAGMA;

    const RoupParserResult parser = roup_parser_create(options);
    if (parser.result.status != ROUP_STATUS_OK) {
        return checked_call(parser.result);
    }

    constexpr std::uint8_t source[] = "#pragma omp reverse apply(reversed(1): reverse)";
    const RoupDirectiveResult directive =
        roup_parse(parser.value, source, sizeof(source) - 1U);
    if (directive.result.status != ROUP_STATUS_OK) {
        const int parse_status = checked_call(directive.result);
        const int parser_status = close_parser(parser.value);
        return parse_status != 0 ? parse_status : parser_status;
    }

    const RoupFieldInfoResult info = roup_clause_field_info(directive.value, 0U, 0U);
    if (info.result.status != ROUP_STATUS_OK) {
        const int query_status = checked_call(info.result);
        return fail_after_directive(directive.value, parser.value, query_status);
    }
    if (info.value.id != ROUP_FIELD_LOOP_MODIFIER ||
        info.value.value_kind != ROUP_FIELD_VALUE_NODE || info.value.count != 1U) {
        return fail_after_directive(directive.value, parser.value, unexpected_field);
    }

    const RoupNodeResult node = roup_clause_field_node(directive.value, 0U, 0U, 0U);
    if (node.result.status != ROUP_STATUS_OK) {
        const int query_status = checked_call(node.result);
        return fail_after_directive(directive.value, parser.value, query_status);
    }

    const RoupNodeKindResult kind = roup_node_kind(node.value);
    const RoupSizeResult fields = roup_node_field_count(node.value);
    if (kind.result.status != ROUP_STATUS_OK || fields.result.status != ROUP_STATUS_OK) {
        const int kind_status = checked_call(kind.result);
        const int fields_status = checked_call(fields.result);
        const int node_status = checked_call(roup_node_release(node.value));
        const int cleanup = close_directive_and_parser(directive.value, parser.value);
        if (kind_status != 0) return kind_status;
        if (fields_status != 0) return fields_status;
        return node_status != 0 ? node_status : cleanup;
    }
    if (kind.value.family != ROUP_NODE_FAMILY_OMP_APPLY_MODIFIER ||
        kind.value.variant != ROUP_OMP_APPLY_REVERSED || fields.value != 1U) {
        const int node_status = checked_call(roup_node_release(node.value));
        const int cleanup = close_directive_and_parser(directive.value, parser.value);
        if (node_status != 0) return node_status;
        return cleanup != 0 ? cleanup : unexpected_node;
    }

    const RoupFieldInfoResult sizes = roup_node_field_info(node.value, 0U);
    if (sizes.result.status != ROUP_STATUS_OK) {
        const int query_status = checked_call(sizes.result);
        const int node_status = checked_call(roup_node_release(node.value));
        const int cleanup = close_directive_and_parser(directive.value, parser.value);
        if (query_status != 0) return query_status;
        return node_status != 0 ? node_status : cleanup;
    }
    if (sizes.value.id != ROUP_FIELD_INDICES ||
        sizes.value.value_kind != ROUP_FIELD_VALUE_STRING_LIST || sizes.value.count != 1U) {
        const int node_status = checked_call(roup_node_release(node.value));
        const int cleanup = close_directive_and_parser(directive.value, parser.value);
        if (node_status != 0) return node_status;
        return cleanup != 0 ? cleanup : unexpected_field;
    }

    const RoupSizeResult length = roup_node_field_string_length(node.value, 0U, 0U);
    if (length.result.status != ROUP_STATUS_OK) {
        const int query_status = checked_call(length.result);
        const int node_status = checked_call(roup_node_release(node.value));
        const int cleanup = close_directive_and_parser(directive.value, parser.value);
        if (query_status != 0) return query_status;
        return node_status != 0 ? node_status : cleanup;
    }
    if (length.value != 1U) {
        const int node_status = checked_call(roup_node_release(node.value));
        const int cleanup = close_directive_and_parser(directive.value, parser.value);
        if (node_status != 0) return node_status;
        return cleanup != 0 ? cleanup : unexpected_value;
    }

    std::array<std::uint8_t, 1> value{};
    const RoupSizeResult copied =
        roup_node_field_string_copy(node.value, 0U, 0U, value.data(), value.size());
    if (copied.result.status != ROUP_STATUS_OK) {
        const int query_status = checked_call(copied.result);
        const int node_status = checked_call(roup_node_release(node.value));
        const int cleanup = close_directive_and_parser(directive.value, parser.value);
        if (query_status != 0) return query_status;
        return node_status != 0 ? node_status : cleanup;
    }
    if (copied.value != 1U || value[0] != static_cast<std::uint8_t>('1')) {
        const int node_status = checked_call(roup_node_release(node.value));
        const int cleanup = close_directive_and_parser(directive.value, parser.value);
        if (node_status != 0) return node_status;
        return cleanup != 0 ? cleanup : unexpected_value;
    }

    const int node_status = checked_call(roup_node_release(node.value));
    const int cleanup = close_directive_and_parser(directive.value, parser.value);
    return node_status != 0 ? node_status : cleanup;
}
