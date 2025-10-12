# Build Script Design Rationale

## Overview

ROUP's `build.rs` serves a dual purpose:
1. **Build mode (default)**: Generates `roup_constants.h` during `cargo build`
2. **Verify mode**: Checks if committed header is up-to-date (used in CI)

## Single Source of Truth

All directive and clause mappings come from `src/c_api.rs`:
- `directive_name_to_kind()` defines directive codes (0-16)
- `convert_clause()` defines clause codes (0-11)

The build script parses this Rust code using `syn` to extract mappings and generate the C header.

## Dual-Purpose Design

### Why One File Instead of Separate Files?

The `build.rs` file serves as both a build script and standalone binary by design. While Cargo may warn about "file build.rs found in multiple build targets," this is **expected and harmless**.

**Advantages:**

1. **DRY Principle**: Avoids duplicating the generation/validation logic
2. **Single Source**: Both modes use identical code paths, ensuring consistency
3. **Simplicity**: ~170 lines vs. creating separate build-utils crate
4. **Standard Rust Pattern**: Widely adopted in the Rust ecosystem

### Ecosystem Examples

This pattern is well-established in popular Rust crates:

- **rustfmt**: `build.rs` with standalone verification mode
- **proc-macro2**: Unified build + test harness
- **quote**: Shared build logic across multiple targets  
- **serde_derive**: Combined build script and test utilities

This pattern is documented in [The Cargo Book](https://doc.rust-lang.org/cargo/reference/build-scripts.html) as an accepted approach for build-time code generation.

### Alternative Considered: Separate Crate

**Rejected** because it adds workspace complexity for minimal benefit:

- Would require: workspace `Cargo.toml`, separate directory, more build coordination
- Not worth it for ~170 lines of code that change infrequently
- Increases maintenance burden without clear advantages

## Maintenance

When adding new directives or clauses:

1. Update `src/c_api.rs` (`directive_name_to_kind` or `convert_clause`)
2. Run `cargo build` - the header regenerates automatically
3. Verify with `cargo run --bin gen` (checks header is current)

## References

- [Cargo Build Scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
- [The Cargo Book: Build Scripts](https://doc.rust-lang.org/cargo/reference/build-script-examples.html)
- Module documentation in `src/constants_gen.rs` for implementation details
