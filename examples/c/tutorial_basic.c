#include "roup.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

static void require_ok(RoupCallResult result) {
    if (result.status != ROUP_STATUS_OK) {
        fprintf(stderr, "unexpected ROUP status %u\n", result.status);
        if (result.error.generation != 0U) {
            RoupCallResult released = roup_error_release(result.error);
            if (released.status != ROUP_STATUS_OK) {
                fprintf(stderr, "error release failed with status %u\n", released.status);
            }
        }
        exit(EXIT_FAILURE);
    }
}

int main(void) {
    RoupParserOptions options = {0};
    options.abi_version = ROUP_ABI_VERSION;
    options.struct_size = (uint32_t)sizeof(options);
    options.dialect = ROUP_DIALECT_OPENMP;
    options.version_policy = ROUP_VERSION_EXACT;
    options.version = ROUP_OMP_VERSION_6_0;
    options.host_language = ROUP_HOST_C;
    options.host_standard = ROUP_C_23;
    options.source_form = ROUP_SOURCE_PRAGMA;

    RoupParserResult parser = roup_parser_create(options);
    require_ok(parser.result);

    const uint8_t source[] = "#pragma omp reverse apply(reversed(1): reverse)";
    RoupDirectiveResult parsed =
        roup_parse(parser.value, source, sizeof(source) - 1U);
    require_ok(parsed.result);

    RoupSizeResult field_count = roup_clause_field_count(parsed.value, 0U);
    require_ok(field_count.result);
    if (field_count.value == 0U) {
        fputs("apply clause has no typed fields\n", stderr);
        return EXIT_FAILURE;
    }

    RoupFieldInfoResult info = roup_clause_field_info(parsed.value, 0U, 0U);
    require_ok(info.result);
    if (info.value.id != ROUP_FIELD_LOOP_MODIFIER ||
        info.value.value_kind != ROUP_FIELD_VALUE_NODE || info.value.count != 1U) {
        fputs("apply loop modifier is not exposed as a typed child node\n", stderr);
        return EXIT_FAILURE;
    }

    RoupNodeResult transform =
        roup_clause_field_node(parsed.value, 0U, 0U, 0U);
    require_ok(transform.result);
    RoupNodeKindResult transform_kind = roup_node_kind(transform.value);
    require_ok(transform_kind.result);
    if (transform_kind.value.family != ROUP_NODE_FAMILY_OMP_APPLY_MODIFIER ||
        transform_kind.value.variant != ROUP_OMP_APPLY_REVERSED) {
        fputs("unexpected semantic child-node family\n", stderr);
        return EXIT_FAILURE;
    }

    printf("apply loop modifier variant %u is represented by a typed node\n",
           transform_kind.value.variant);

    require_ok(roup_node_release(transform.value));
    require_ok(roup_directive_release(parsed.value));
    require_ok(roup_parser_release(parser.value));
    return EXIT_SUCCESS;
}
