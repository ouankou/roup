# ROUP: Rust-based OpenMP Parser

Rust-first parsing for OpenMP directives with C and C++ bindings.

[![Docs](https://img.shields.io/badge/docs-roup.ouankou.com-blue)](https://roup.ouankou.com)
[![Status](https://img.shields.io/badge/status-experimental-orange)](https://github.com/ouankou/roup)

> **Experimental:** APIs are still evolving. Expect breaking changes between releases.

## Quick start

### Rust
```toml
[dependencies]
roup = "0.5"
```

### C or C++
```bash
cargo build --release
# Link against target/release/libroup.{a,so,dylib}
```

Need platform-specific notes? See the [building guide](https://roup.ouankou.com/building.html).

## Why ROUP?

- **OpenMP 3.0–6.0 coverage**: directives, clauses, and combined forms.
- **OpenACC 3.4 coverage**: full directive and clause matrix documented in
  [`docs/OPENACC_SUPPORT.md`](docs/OPENACC_SUPPORT.md).
- **Multi-language APIs**: idiomatic Rust plus C and C++17 bindings.
- **Tight unsafe boundary**: the FFI layer is the only unsafe code.
- **Thorough testing**: 620+ automated tests across languages.
- **ompparser compatibility**: optional shim for existing consumers.

## Documentation

All guides live at [roup.ouankou.com](https://roup.ouankou.com):

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

Additional C and C++ samples are available in [`examples/`](examples/).

## Build and test

```bash
cargo build --release
cargo test
```

The book can be rebuilt with `cargo doc --no-deps`.

## ompparser compatibility

The optional layer in [`compat/ompparser/`](compat/ompparser/) exposes the same headers as the original ompparser project. Build
 it with `./compat/ompparser/build.sh` or follow the manual steps in
 [the compatibility guide](docs/book/src/ompparser-compat.md).

## Contributing

Contributions are welcome. Review the [contributing guide](https://roup.ouankou.com/contributing.html) for coding standards, te
st expectations, and the pull-request workflow.

## Learning Resources

ROUP demonstrates Rust concepts from basics to advanced:

- **Basics:** Structs, enums, pattern matching, ownership
- **Intermediate:** Traits, generics, error handling, modules
- **Advanced:** Parser combinators (nom), FFI, unsafe boundaries

**For learners:**
- Read the [Architecture Guide](https://roup.ouankou.com/architecture.html)
- Study the [examples](examples/) directory
- Check the commit history for evolution

---

## License

BSD-3-Clause License - see [LICENSE](LICENSE) file for details.

**Copyright © 2024-2025 Anjia Wang**
