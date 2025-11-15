# ROUP: Rust-based OpenMP Parser

Rust-first parsing for OpenMP directives with C and C++ bindings.

[![Docs](https://img.shields.io/badge/docs-roup.ouankou.com-blue)](https://roup.ouankou.com)
[![Status](https://img.shields.io/badge/status-experimental-orange)](https://github.com/ouankou/roup)

> **Experimental:** APIs continue to evolve. Expect breaking changes between releases.

## Quick start

```toml
# Cargo.toml
[dependencies]
roup = "0.6"
```

```bash
# C or C++ projects
cargo build --release
# Link against target/release/libroup.{a,so,dylib}
```

Platform-specific notes live in the [building guide](https://roup.ouankou.com/building.html).

## Highlights

- **OpenMP 3.0–6.0** coverage across directives, clauses, and combined forms.
- **OpenACC 3.4** support with the matrix documented in [`docs/OPENACC_SUPPORT.md`](docs/OPENACC_SUPPORT.md).
  
    Note: generated OpenACC C macros (used by compatibility layers and C/C++ consumers)
    use the `ROUP_ACC_*` prefix (for example `ROUP_ACCD_parallel`). The
    older `ACC_*` aliases are intentionally removed from generated headers; please
    update any downstream code or documentation that depended on the legacy names.
- **Rust, C, and C++17 APIs** with a narrow unsafe boundary confined to FFI bindings.
- **Extensive tests:** 620+ automated checks, including ompparser compatibility.

## Documentation

The mdBook at [roup.ouankou.com](https://roup.ouankou.com) provides tutorials, an architecture tour, and the API reference. Each chapter mirrors the sources under [`docs/book/src/`](docs/book/src/).

## Minimal example

```rust
use roup::parser::parse;

fn main() {
    let directive = parse("#pragma omp parallel for num_threads(4)").expect("valid directive");
    println!("parsed {:?} with {} clauses", directive.kind, directive.clauses.len());
}
```

More Rust, C, and C++ samples live in [`examples/`](examples/).

## Build and test

```bash
cargo build --release
cargo test
```

Rebuild the docs with `cargo doc --no-deps` followed by `mdbook build docs/book`.

## ompparser compatibility

[`compat/ompparser/`](compat/ompparser/) ships a drop-in replacement for the original ompparser headers. Build it with `./compat/ompparser/build.sh` or follow the manual steps in [the compatibility guide](docs/book/src/ompparser-compat.md).

## Contributing

See the [contributing guide](https://roup.ouankou.com/contributing.html) for coding standards, test expectations, and the pull-request workflow.

## License

BSD-3-Clause License — see [LICENSE](LICENSE).

© 2024–2025 Anjia Wang
