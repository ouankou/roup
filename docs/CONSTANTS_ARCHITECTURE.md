# Constants architecture

ROUP uses enum-based parsing with `FromStr` traits to eliminate string-based comparisons, while maintaining explicit enum discriminants for C FFI stability.

## String parsing (zero string comparisons)

All directive and clause parsing uses `FromStr` trait implementations:
- `DirectiveKind::from_str()` in `src/ir/directive.rs`
- `ReductionOperator::from_str()`, `ScheduleKind::from_str()`, etc. in `src/ir/clause.rs`

This eliminates all string-based match statements in favor of centralized, type-safe enum conversions.

## FFI enum discriminants (ABI stability required)

**Explicit discriminants in `#[repr(C)]` enums are ABI contracts that MUST remain stable.**

These are NOT "magic numbers" to eliminate—they are part of the public C API:
- `DirectiveKind` uses explicit discriminants (0, 1, 2, 3, ... with intentional gaps)
- Gaps allow logical grouping (parallel=0-19, work-sharing=10-19, tasks=30-39, etc.)
- Changing values would break `roup_constants.h` and all FFI consumers
- C/Fortran code depends on these specific numeric values

## C API constant generation

The build system generates C headers from Rust enum definitions:

1. **Rust enums** have explicit `#[repr(C)]` discriminants for ABI stability
2. **`build.rs`** parses `src/c_api.rs` to generate `roup_constants.h`
3. **C/C++ code** includes `roup_constants.h` for stable constants
4. **Fortran** uses the generated `roup_interface.mod` module

When adding a directive or clause:
1. Add the variant to the enum with an explicit discriminant (choose unused value)
2. Implement `FromStr` for string → enum conversion
3. Update `src/c_api.rs` if modifying the C API
4. Run `cargo build` to regenerate headers

**Never** edit `roup_constants.h` manually—the build system regenerates it.

## Design rationale

| Aspect | Approach | Reason |
|--------|----------|--------|
| String parsing | `FromStr` traits | Eliminates string comparisons, centralized logic |
| FFI discriminants | Explicit values | ABI stability, C interop requires fixed values |
| Discriminant gaps | Intentional | Logical grouping, room for future additions |
