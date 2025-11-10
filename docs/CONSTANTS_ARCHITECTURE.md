# Constants architecture

ROUP uses enum-based parsing throughout the codebase with **zero raw numbers or string comparisons**. All directive and clause types implement the `FromStr` trait, providing centralized, type-safe conversions.

## Internal parsing (Rust)

Directive and clause parsing uses `FromStr` trait implementations:
- `DirectiveKind::from_str()` in `src/ir/directive.rs`
- `ReductionOperator::from_str()`, `ScheduleKind::from_str()`, etc. in `src/ir/clause.rs`

**Enum discriminants are auto-generated** sequentially by Rust (0, 1, 2, ...). No raw numbers appear in enum definitions.

## C API

The C bindings expose numeric directive and clause identifiers via generated constants:

1. **Rust enums** have auto-generated sequential discriminants
2. **`build.rs`** parses `src/c_api.rs` to generate `roup_constants.h`
3. **C/C++ code** includes `roup_constants.h` for stable FFI constants

When adding a directive or clause:
1. Add the variant to the enum (discriminant assigned automatically)
2. Add `FromStr` implementation for string → enum conversion
3. Update `src/c_api.rs` if modifying the C API
4. Run `cargo build` to regenerate `roup_constants.h`

**Never** edit `roup_constants.h` manually—the build system regenerates it.
