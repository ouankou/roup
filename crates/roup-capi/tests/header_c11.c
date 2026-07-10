#include "roup.h"

_Static_assert(ROUP_FIELD_VALUE_U32 == UINT32_C(2), "u32 field tag drift");
_Static_assert(ROUP_FIELD_VALUE_U32_LIST == UINT32_C(5), "u32-list field tag drift");
_Static_assert(sizeof(&roup_directive_parameter_field_u32) > 0U,
               "parameter u32 query missing");
_Static_assert(sizeof(&roup_clause_field_u32) > 0U, "clause u32 query missing");
_Static_assert(sizeof(&roup_node_field_u32) > 0U, "node u32 query missing");

enum {
    ROUP_FIXTURE_UNEXPECTED_FIELD = 100,
    ROUP_FIXTURE_UNEXPECTED_NODE = 101,
    ROUP_FIXTURE_UNEXPECTED_VALUE = 102,
    ROUP_FIXTURE_UNEXPECTED_SPAN = 103
};

static int checked_call(RoupCallResult result) {
    if (result.status == ROUP_STATUS_OK) {
        return 0;
    }
    RoupCallResult released = roup_error_release(result.error);
    return released.status == ROUP_STATUS_OK ? (int)result.status : (int)released.status;
}

static int close_parser(RoupParserHandle parser) {
    return checked_call(roup_parser_release(parser));
}

static int close_directive_and_parser(
    RoupDirectiveHandle directive, RoupParserHandle parser) {
    int directive_status = checked_call(roup_directive_release(directive));
    int parser_status = close_parser(parser);
    return directive_status != 0 ? directive_status : parser_status;
}

static int fail_after_directive(
    RoupDirectiveHandle directive, RoupParserHandle parser, int failure) {
    int cleanup = close_directive_and_parser(directive, parser);
    return cleanup != 0 ? cleanup : failure;
}

int roup_c11_header_fixture(void) {
    RoupParserOptions options = {0};
    options.abi_version = ROUP_ABI_VERSION;
    options.struct_size = (uint32_t)sizeof(options);
    options.dialect = ROUP_DIALECT_OPENMP;
    options.version_policy = ROUP_VERSION_ANY;
    options.host_language = ROUP_HOST_C;
    options.host_standard = ROUP_C_23;
    options.source_form = ROUP_SOURCE_PRAGMA;

    RoupParserResult parser = roup_parser_create(options);
    if (parser.result.status != ROUP_STATUS_OK) {
        return checked_call(parser.result);
    }

    const uint8_t source[] = "#pragma omp reverse apply(reversed(1): reverse)";
    RoupDirectiveResult directive =
        roup_parse(parser.value, source, sizeof(source) - 1U);
    if (directive.result.status != ROUP_STATUS_OK) {
        int parse_status = checked_call(directive.result);
        int parser_status = close_parser(parser.value);
        return parse_status != 0 ? parse_status : parser_status;
    }

    RoupSpanResult directive_span = roup_directive_span(directive.value);
    RoupSpanResult clause_span = roup_clause_span(directive.value, 0U);
    if (directive_span.result.status != ROUP_STATUS_OK ||
        clause_span.result.status != ROUP_STATUS_OK) {
        int directive_span_status = checked_call(directive_span.result);
        int clause_span_status = checked_call(clause_span.result);
        int cleanup = close_directive_and_parser(directive.value, parser.value);
        if (directive_span_status != 0) return directive_span_status;
        return clause_span_status != 0 ? clause_span_status : cleanup;
    }
    if (directive_span.value.start_byte != 12U || directive_span.value.end_byte != 19U ||
        directive_span.value.start_line != 1U || directive_span.value.start_column != 13U ||
        clause_span.value.start_byte != 20U || clause_span.value.end_byte != 25U ||
        clause_span.value.start_line != 1U || clause_span.value.start_column != 21U) {
        return fail_after_directive(
            directive.value, parser.value, ROUP_FIXTURE_UNEXPECTED_SPAN);
    }

    RoupFieldInfoResult info = roup_clause_field_info(directive.value, 0U, 0U);
    if (info.result.status != ROUP_STATUS_OK) {
        int query_status = checked_call(info.result);
        return fail_after_directive(directive.value, parser.value, query_status);
    }
    if (info.value.id != ROUP_FIELD_LOOP_MODIFIER ||
        info.value.value_kind != ROUP_FIELD_VALUE_NODE || info.value.count != 1U) {
        return fail_after_directive(
            directive.value, parser.value, ROUP_FIXTURE_UNEXPECTED_FIELD);
    }

    RoupNodeResult node = roup_clause_field_node(directive.value, 0U, 0U, 0U);
    if (node.result.status != ROUP_STATUS_OK) {
        int query_status = checked_call(node.result);
        return fail_after_directive(directive.value, parser.value, query_status);
    }

    RoupNodeKindResult kind = roup_node_kind(node.value);
    RoupSizeResult fields = roup_node_field_count(node.value);
    if (kind.result.status != ROUP_STATUS_OK || fields.result.status != ROUP_STATUS_OK) {
        int kind_status = checked_call(kind.result);
        int fields_status = checked_call(fields.result);
        int node_status = checked_call(roup_node_release(node.value));
        int cleanup = close_directive_and_parser(directive.value, parser.value);
        if (kind_status != 0) return kind_status;
        if (fields_status != 0) return fields_status;
        return node_status != 0 ? node_status : cleanup;
    }
    if (kind.value.family != ROUP_NODE_FAMILY_OMP_APPLY_MODIFIER ||
        kind.value.variant != ROUP_OMP_APPLY_REVERSED || fields.value != 1U) {
        int node_status = checked_call(roup_node_release(node.value));
        int cleanup = close_directive_and_parser(directive.value, parser.value);
        if (node_status != 0) return node_status;
        return cleanup != 0 ? cleanup : ROUP_FIXTURE_UNEXPECTED_NODE;
    }

    RoupFieldInfoResult sizes = roup_node_field_info(node.value, 0U);
    if (sizes.result.status != ROUP_STATUS_OK) {
        int query_status = checked_call(sizes.result);
        int node_status = checked_call(roup_node_release(node.value));
        int cleanup = close_directive_and_parser(directive.value, parser.value);
        if (query_status != 0) return query_status;
        return node_status != 0 ? node_status : cleanup;
    }
    if (sizes.value.id != ROUP_FIELD_INDICES ||
        sizes.value.value_kind != ROUP_FIELD_VALUE_STRING_LIST || sizes.value.count != 1U) {
        int node_status = checked_call(roup_node_release(node.value));
        int cleanup = close_directive_and_parser(directive.value, parser.value);
        if (node_status != 0) return node_status;
        return cleanup != 0 ? cleanup : ROUP_FIXTURE_UNEXPECTED_FIELD;
    }

    RoupSizeResult length = roup_node_field_string_length(node.value, 0U, 0U);
    if (length.result.status != ROUP_STATUS_OK) {
        int query_status = checked_call(length.result);
        int node_status = checked_call(roup_node_release(node.value));
        int cleanup = close_directive_and_parser(directive.value, parser.value);
        if (query_status != 0) return query_status;
        return node_status != 0 ? node_status : cleanup;
    }
    if (length.value != 1U) {
        int node_status = checked_call(roup_node_release(node.value));
        int cleanup = close_directive_and_parser(directive.value, parser.value);
        if (node_status != 0) return node_status;
        return cleanup != 0 ? cleanup : ROUP_FIXTURE_UNEXPECTED_VALUE;
    }

    uint8_t value[1] = {0};
    RoupSizeResult copied =
        roup_node_field_string_copy(node.value, 0U, 0U, value, sizeof(value));
    if (copied.result.status != ROUP_STATUS_OK) {
        int query_status = checked_call(copied.result);
        int node_status = checked_call(roup_node_release(node.value));
        int cleanup = close_directive_and_parser(directive.value, parser.value);
        if (query_status != 0) return query_status;
        return node_status != 0 ? node_status : cleanup;
    }
    if (copied.value != 1U || value[0] != (uint8_t)'1') {
        int node_status = checked_call(roup_node_release(node.value));
        int cleanup = close_directive_and_parser(directive.value, parser.value);
        if (node_status != 0) return node_status;
        return cleanup != 0 ? cleanup : ROUP_FIXTURE_UNEXPECTED_VALUE;
    }

    int node_status = checked_call(roup_node_release(node.value));
    int cleanup = close_directive_and_parser(directive.value, parser.value);
    return node_status != 0 ? node_status : cleanup;
}
