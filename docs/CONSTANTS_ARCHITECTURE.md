# Constants Architecture

Directive and clause codes live in `src/c_api.rs`. The build script parses those match arms with `syn` and generates
`src/roup_constants.h`, keeping the Rust and C worlds aligned. CI re-runs the generator to confirm the committed header matches
current sources.

When adding new items:

1. Update the relevant match arms in `src/c_api.rs`.
2. Run `cargo build` (or `cargo run --bin gen`) to regenerate the header.
3. Commit both the Rust change and the refreshed header.
