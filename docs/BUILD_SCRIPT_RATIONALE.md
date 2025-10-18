# Build Script Overview

`build.rs` generates `roup_constants.h` from the directive and clause registries defined in `src/c_api.rs`. The same logic also
powers a verification mode used in CI to ensure the checked-in header stays in sync.

- The build script reuses the parsing helpers in `src/constants_gen.rs` to read the Rust AST via `syn`.
- During `cargo build` it emits a fresh header into `OUT_DIR`; the committed copy can be refreshed with `cargo run --bin gen`.
- Keeping generation and verification in one place avoids drift while keeping the implementation small.

When new directives or clauses are added, update `src/c_api.rs` and regenerate the header—no manual edits are required elsewhere.
