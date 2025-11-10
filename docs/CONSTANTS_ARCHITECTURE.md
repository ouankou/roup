# Constants architecture

The C bindings expose numeric directive and clause identifiers via type-safe Rust enums that are converted to integers at the FFI boundary.

## Enum-based approach

ROUP uses `#[repr(i32)]` enums throughout the codebase for type safety:

- **OpenMP**: `DirectiveKindC`, `ClauseKindC`, `ReductionOperatorC`, `ScheduleKindC`, `DefaultKindC` in `src/c_api.rs`
- **OpenACC**: `AccDirectiveKindC`, `AccClauseKindC` in `src/c_api/openacc.rs`

Each enum variant has an explicit discriminant value for C API compatibility.

## Code generation

`build.rs` (or `cargo run --bin gen`) generates `roup_constants.h` from these enum definitions using `syn`:
- Parses enum definitions from `src/c_api.rs` and `src/c_api/openacc.rs`
- Writes #define macros for each variant
- Includes a checksum so CI can verify the header is current

C and C++ code include `roup_constants.h` and use the macros in switch statements.

## Adding new directives or clauses

1. Add the new variant to the appropriate enum in `src/c_api.rs` or `src/c_api/openacc.rs`
2. Update the match expression in `directive_name_to_kind()` or `clause_name_to_kind()` to return the new enum variant
3. Rebuild to regenerate `roup_constants.h`

Never change `roup_constants.h` by hand—the build will overwrite it.
