# API reference

## Rust

The stable entry points are organized by responsibility:

- `roup::api`: configured OpenMP and OpenACC parser facades and parse results
- `roup::ast`: canonical typed directive and clause nodes
- `roup::host`: typed C, C++, and Fortran expression syntax
- `roup::ir`: shared typed clause components such as expressions, variables,
  locators, modifiers, and reduction operators
- `roup::version`: specification policies, host profiles, and source forms
- `roup::diagnostic` and `roup::source`: stable hard errors and checked spans
- `roup::validation`: semantic facts and stateful region validation

The parser requires an explicit host profile and source form. A parse returns
one dialect-specific result or one `Diagnostic`; it never returns a partial AST.
Generate the complete item-level API with:

```bash
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p roup --no-deps
```

## C

The optional ABI is declared entirely by
`crates/roup-capi/include/roup.h`. Its operations fall into five groups:

- parser creation and release
- parsing and directive release
- directive kind, compatibility, checked span, parameter, and clause queries
- typed field metadata and scalar/string/list element queries
- diagnostic code, checked span, message, and release operations

Every fallible operation returns a result structure whose first member is a
`RoupCallResult`. A non-OK status owns a queryable error handle. String queries
use a length call followed by an all-or-nothing copy into caller-owned bytes;
the ABI does not append a NUL terminator.

Kinds are returned as `(dialect, ordinal)` ABI values. They are explicit ABI
mappings, not Rust enum layouts. Payloads are exposed only through typed field
descriptors. `roup_directive_span`, `roup_clause_span`, and `roup_error_span`
return physical UTF-8 byte, line, and column locations.
