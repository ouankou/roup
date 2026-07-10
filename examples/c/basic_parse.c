#include "roup.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

static void fail(RoupCallResult result) {
    if (result.status == ROUP_STATUS_OK) {
        fputs("internal example error: fail called for success\n", stderr);
        exit(EXIT_FAILURE);
    }

    RoupSizeResult length = roup_error_message_length(result.error);
    if (length.result.status != ROUP_STATUS_OK) {
        fputs("unable to query ROUP error message\n", stderr);
        exit(EXIT_FAILURE);
    }

    uint8_t *message = malloc(length.value + 1U);
    if (message == NULL) {
        fputs("unable to allocate error-message buffer\n", stderr);
        exit(EXIT_FAILURE);
    }
    RoupSizeResult copied =
        roup_error_message_copy(result.error, message, length.value);
    if (copied.result.status != ROUP_STATUS_OK || copied.value != length.value) {
        free(message);
        fputs("unable to copy complete ROUP error message\n", stderr);
        exit(EXIT_FAILURE);
    }
    message[copied.value] = 0;
    fprintf(stderr, "ROUP error: %s\n", (const char *)message);
    free(message);

    RoupCallResult released = roup_error_release(result.error);
    if (released.status != ROUP_STATUS_OK) {
        fputs("unable to release ROUP error handle\n", stderr);
    }
    exit(EXIT_FAILURE);
}

static void require_ok(RoupCallResult result) {
    if (result.status != ROUP_STATUS_OK) {
        fail(result);
    }
}

int main(void) {
    RoupParserOptions options = {0};
    options.abi_version = ROUP_ABI_VERSION;
    options.struct_size = (uint32_t)sizeof(options);
    options.dialect = ROUP_DIALECT_OPENMP;
    options.version_policy = ROUP_VERSION_ANY;
    options.host_language = ROUP_HOST_C;
    options.host_standard = ROUP_C_23;
    options.source_form = ROUP_SOURCE_PRAGMA;

    RoupParserResult parser = roup_parser_create(options);
    require_ok(parser.result);

    const uint8_t source[] =
        "#pragma omp parallel num_threads(4) private(value)";
    RoupDirectiveResult parsed =
        roup_parse(parser.value, source, sizeof(source) - 1U);
    require_ok(parsed.result);

    RoupDirectiveKindResult kind = roup_directive_kind(parsed.value);
    require_ok(kind.result);
    RoupSizeResult clauses = roup_directive_clause_count(parsed.value);
    require_ok(clauses.result);

    if (kind.value.dialect != ROUP_DIALECT_OPENMP || clauses.value != 2U) {
        fputs("unexpected typed parse result\n", stderr);
        return EXIT_FAILURE;
    }

    printf("OpenMP directive ordinal %u with %zu clauses\n",
           kind.value.ordinal, clauses.value);

    require_ok(roup_directive_release(parsed.value));
    require_ok(roup_parser_release(parser.value));
    return EXIT_SUCCESS;
}
