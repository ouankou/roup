# Contributing

Changes must preserve the parser's central invariant: success means a complete,
typed, validated AST. Semantic Rust types must not contain opaque text variants,
guessed defaults, or ABI layout annotations.

When adding syntax:

1. Add a parser-boundary representation and canonical typed AST shape.
2. Add an explicit specification introduction entry and any historical alias
   provenance needed for exact-version parsing.
3. Add directive/clause/context validation and a negative test for malformed
   input.
4. Extend the C ABI typed fields if the node is externally visible.
5. Extend both compatibility adapters or return a hard conversion error.

Run the deterministic repository gate before submitting a change:

```bash
./test.sh
```

The gate requires initialized submodules and the Rust, C, C++, Fortran, CMake,
and mdBook toolchains. It does not install dependencies, move submodule refs, or
skip unavailable test categories.

For a smaller Rust-only iteration:

```bash
cargo fmt --all --check
./scripts/audit_enum_safety.sh
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked -p roup
```
