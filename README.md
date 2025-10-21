# ROUP: Rust-based OpenMP Parser

Rust-first parsing for OpenMP and OpenACC directives with Rust, C, and C++ bindings.

[![Docs](https://img.shields.io/badge/docs-roup.ouankou.com-blue)](https://roup.ouankou.com)
[![Status](https://img.shields.io/badge/status-experimental-orange)](https://github.com/ouankou/roup)

> **Experimental:** APIs may change between releases.

## Install

### Rust crate

```toml
[dependencies]
roup = "0.5"
```

### C or C++

```bash
cargo build --release
# Link against target/release/libroup.{a,so,dylib}
```

Platform-specific notes live in the [building guide](https://roup.ouankou.com/building.html).

## Highlights

- **OpenMP 3.0–6.0** directives, clauses, and combined forms.
- **OpenACC 3.4** coverage with the full matrix in [`docs/OPENACC_SUPPORT.md`](docs/OPENACC_SUPPORT.md).
- **Multi-language APIs** for idiomatic Rust and C/C++17 consumers.
- **Isolated unsafe boundary**: the FFI layer contains the only unsafe code.
- **Extensive testing** with over 600 automated checks across languages.
- **Compatibility shims** for ompparser and accparser users.

## Documentation

Guides, tutorials, and references are published at [roup.ouankou.com](https://roup.ouankou.com):

- [Getting started](https://roup.ouankou.com/getting-started.html)
- [Rust tutorial](https://roup.ouankou.com/rust-tutorial.html)
- [C tutorial](https://roup.ouankou.com/c-tutorial.html)
- [C++ tutorial](https://roup.ouankou.com/cpp-tutorial.html)
- [API reference](https://roup.ouankou.com/api-reference.html)
- [Architecture](https://roup.ouankou.com/architecture.html)
- [OpenACC support matrix](docs/OPENACC_SUPPORT.md)

## Minimal example

```rust
use roup::parser::parse;

fn main() {
    let directive = parse("#pragma omp parallel for num_threads(4)")
        .expect("valid directive");
    println!("parsed {:?} with {} clauses", directive.kind, directive.clauses.len());
}
```

More Rust, C, and C++ samples are available in [`examples/`](examples/).

## Build and test

```bash
cargo build --release
cargo test
./test.sh             # full local battery (requires optional tooling)
```

Rebuild the documentation site with `mdbook build docs/book`.

## Compatibility layers

[`compat/ompparser/`](compat/ompparser/) and [`compat/accparser/`](compat/accparser/) ship drop-in replacements for the original ompparser and accparser libraries. Use the provided `build.sh` wrappers or follow the book's compatibility chapters under [`docs/book/src/`](docs/book/src/).

## Contributing

Contributions are welcome. See the [contributing guide](https://roup.ouankou.com/contributing.html) for coding standards, testing expectations, and the pull-request workflow.

## License

BSD-3-Clause — see [LICENSE](LICENSE) for details.

© 2024-2025 Anjia Wang
