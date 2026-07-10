# C tutorial

Build the optional ABI and include its checked-in header:

```bash
cargo build --release -p roup-capi
cc -std=c11 -Icrates/roup-capi/include app.c \
  -Ltarget/release -lroup_capi -o app
```

The exact additional system libraries required for a static link are platform
specific. A shared-library run must make `target/release` visible to the dynamic
loader.

## Create, parse, query, release

```c
#include "roup.h"

#include <stdint.h>
#include <stdlib.h>

static int check(RoupCallResult result) {
    if (result.status == ROUP_STATUS_OK) {
        return 1;
    }
    if (result.error.generation != 0) {
        RoupCallResult released = roup_error_release(result.error);
        if (released.status != ROUP_STATUS_OK) {
            abort();
        }
    }
    return 0;
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
    if (!check(parser.result)) {
        return EXIT_FAILURE;
    }

    const uint8_t input[] = "#pragma omp parallel private(value)";
    RoupDirectiveResult parsed =
        roup_parse(parser.value, input, sizeof(input) - 1U);
    if (!check(parsed.result)) {
        (void)roup_parser_release(parser.value);
        return EXIT_FAILURE;
    }

    RoupSizeResult clauses = roup_directive_clause_count(parsed.value);
    if (!check(clauses.result) || clauses.value != 1U) {
        (void)roup_directive_release(parsed.value);
        (void)roup_parser_release(parser.value);
        return EXIT_FAILURE;
    }

    if (!check(roup_directive_release(parsed.value)) ||
        !check(roup_parser_release(parser.value))) {
        return EXIT_FAILURE;
    }
    return EXIT_SUCCESS;
}
```

Production code should query and print the error message before releasing its
handle. Messages use two calls: `roup_error_message_length`, followed by
`roup_error_message_copy` into a buffer of exactly that many bytes. The copy is
all-or-nothing and does not append `\0`.

## Typed fields and child nodes

Each clause and directive parameter has an indexed field schema:

1. Query the field count.
2. Query `RoupFieldInfo` for each field.
3. Dispatch on `value_kind`.
4. Read boolean leaves with `*_field_bool`, closed numeric tags with
   `*_field_u32`, and UTF-8 leaves with the paired length/copy operations.
5. Acquire `NODE` or `NODE_LIST` values with `*_field_node`, recursively query
   the node, then release its independent handle.

Tagged records such as apply loop modifiers, complete applied directives,
induction identifiers, interoperability preferences, requirements, iterators,
allocator specifications, and selectors are child nodes. They are never
flattened into a clause payload string.

`roup_directive_span` and `roup_clause_span` report half-open byte ranges and
one-based line and column positions in the original UTF-8 input. This mapping
is preserved across C/C++ line splices and Fortran continuations.

Every non-OK result owns an error handle. Every successful parser, directive,
node, and error handle must be released exactly once; stale and wrong-kind
handles are hard errors.
