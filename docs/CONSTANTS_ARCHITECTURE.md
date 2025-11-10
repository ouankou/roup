# Constants architecture

The C bindings expose numeric directive and clause identifiers. All numeric constants are defined as named constants in `src/c_api.rs` and `src/c_api/openacc.rs`, not as raw numbers.

## Enum-based approach

- **OpenMP**: Uses constants like `OMP_CLAUSE_KIND_SCHEDULE`, `OMP_DIRECTIVE_KIND_PARALLEL`
- **OpenACC**: Uses constants like `ACC_CLAUSE_ASYNC`, `ACC_DIRECTIVE_PARALLEL`
- Reduction operators use `ReductionOperator` enum discriminants (e.g., `ReductionOperator::Add as i32`)
- Schedule kinds use `ScheduleKind` enum discriminants (e.g., `ScheduleKind::Static as i32`)
- Default kinds use `DefaultKind` enum discriminants (e.g., `DefaultKind::Shared as i32`)

## Header generation

`build.rs` (or `cargo run --bin gen`) generates `roup_constants.h` from named constants in `src/c_api.rs`, providing C macros for switch statements. The script writes a checksum so CI can verify the header is current.

When adding directives or clauses, edit the named constants and match arms in `src/c_api.rs` only and rebuild. Never modify `roup_constants.h` manually—it will be regenerated.
