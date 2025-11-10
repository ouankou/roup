# Constants architecture

ROUP uses enum-based parsing throughout the codebase. All directive and clause types implement the `FromStr` trait, eliminating string-based comparisons in favor of centralized enum conversions.

## Internal parsing (Rust)

Directive and clause parsing uses `FromStr` trait implementations on the corresponding enums:
- `DirectiveKind::from_str()` in `src/ir/directive.rs`
- `ReductionOperator::from_str()`, `ScheduleKind::from_str()`, etc. in `src/ir/clause.rs`

This approach centralizes parsing logic, eliminates redundant match statements, and ensures type safety.

## C API

The C bindings expose numeric directive and clause identifiers. The project generates `roup_constants.h` from the authoritative tables in `src/c_api.rs`.

- `build.rs` (or `cargo run --bin gen`) parses the match arms in `directive_name_to_kind` and `convert_clause` using `syn`.
- The script writes the header and a checksum so CI can confirm the committed file is current.
- C and C++ code include `roup_constants.h` and use the macros in switches.

When a directive or clause is added, update the enum's `FromStr` implementation in the IR module, and edit `src/c_api.rs` only if modifying the C API. Never change `roup_constants.h` by hand—the next build will overwrite it.
