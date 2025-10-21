# ROUP: Rust-based OpenMP parser

Rust-first OpenMP parsing with native Rust APIs and maintained C/C++ bindings.

[![Docs](https://img.shields.io/badge/docs-roup.ouankou.com-blue)](https://roup.ouankou.com)
[![Status](https://img.shields.io/badge/status-experimental-orange)](https://github.com/ouankou/roup)

> **Experimental:** APIs may change between releases.

## Install

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
roup = "0.5"
```

For C or C++ projects build the shared library and link against it:

```bash
cargo build --release
# Link target/release/libroup.{a,so,dylib}
```

Platform-specific steps are covered in the [building guide](https://roup.ouankou.com/building.html).

## Highlights

- **OpenMP 3.0–6.0**: directives, clauses, combined and `end` forms.
- **OpenACC 3.4**: full directive and clause matrix with alias coverage.【F:docs/OPENACC_SUPPORT.md†L1-L45】
- **Multi-language APIs**: idiomatic Rust plus tested C and C++ bindings.【F:compat/ompparser/README.md†L1-L78】
- **Minimal unsafe surface**: FFI is isolated to the compatibility layer.【F:compat/ompparser/README.md†L10-L39】
- **Extensive regression suite**: 600+ unit, integration, and round-trip tests.【F:TESTING.md†L1-L33】

## Documentation

Guides, tutorials, and the API reference live at [roup.ouankou.com](https://roup.ouankou.com). The `docs/` directory contains
the same sources, including the OpenACC coverage details in [`docs/OPENACC_SUPPORT.md`](docs/OPENACC_SUPPORT.md).

## Example

```rust
use roup::parser::parse;

fn main() {
    let directive = parse("#pragma omp parallel for num_threads(4)")
        .expect("valid directive");
    println!("parsed {:?} with {} clauses", directive.kind, directive.clauses.len());
}
```

Additional C, C++, and Fortran samples live in [`examples/`](examples/).

## Build and test

```bash
cargo build --release
cargo test
```

Run `cargo doc --no-deps` to regenerate the book’s embedded API reference. Automated testing scripts are documented in
[`TESTING.md`](TESTING.md).

## ompparser compatibility

[`compat/ompparser/`](compat/ompparser/) ships headers matching the original ompparser project. Build it with
`./compat/ompparser/build.sh` or follow the [compatibility guide](docs/book/src/ompparser-compat.md) for manual steps.

## Contributing

See the [contributing guide](https://roup.ouankou.com/contributing.html) for coding standards, testing expectations, and the
pull-request workflow.

## License

BSD-3-Clause License – see [LICENSE](LICENSE) for details.

© 2024–2025 Anjia Wang
