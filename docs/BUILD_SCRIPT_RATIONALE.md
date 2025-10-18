# Build script rationale

`build.rs` serves two roles:

1. Generate `roup_constants.h` during a normal build.
2. Verify the committed header via `cargo run --bin gen` in CI.

Both modes share the same code so the mapping logic only lives in one place. The
script parses `src/c_api.rs` (directive and clause tables) using `syn`, which
keeps the Rust API and generated header consistent.

Cargo warns that `build.rs` appears in multiple targets; we deliberately accept
that warning rather than split the logic into a helper crate. Keeping the file
in one location avoids extra workspace plumbing for ~170 lines of code.

When you add directives or clauses, update the tables in `src/c_api.rs`, rebuild
the project, and run `cargo run --bin gen` to confirm the checked-in header is
current. Implementation details live in `src/constants_gen.rs`, and the general
pattern is documented in [The Cargo Book](https://doc.rust-lang.org/cargo/reference/build-scripts.html).
