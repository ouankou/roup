# ROUP: Rust-based OpenMP Parser

Rust-first parsing for OpenMP and OpenACC directives with optional C and C++
bindings.

[![Docs](https://img.shields.io/badge/docs-roup.ouankou.com-blue)](https://roup.ouankou.com)
[![Status](https://img.shields.io/badge/status-experimental-orange)](https://github.com/ouankou/roup)

> **Experimental:** public APIs may change between releases.

## Highlights

- **Standards coverage:** OpenMP 3.0–6.0 plus the documented
  [OpenACC 3.4 matrix](docs/OPENACC_SUPPORT.md).
- **Multi-language APIs:** idiomatic Rust with C and C++17 bindings generated
  from the same tables.
- **Unsafe boundary kept tight:** the FFI layer owns the only unsafe code.
- **Extensive validation:** 620+ automated tests including OpenMP_VV and
  OpenACCV-V round trips.
- **ompparser compatibility:** drop-in shim at [`compat/ompparser/`](compat/ompparser/).

## Quick start

**Rust**

```toml
[dependencies]
roup = "0.5"
```

**C or C++**

```bash
cargo build --release
# Link against target/release/libroup.{a,so,dylib}
```

Platform notes and additional options live in the
[building guide](https://roup.ouankou.com/building.html).

## Example

```rust
use roup::parser::parse;

fn main() {
    let directive = parse("#pragma omp parallel for num_threads(4)")
        .expect("valid directive");
    println!("parsed {:?} with {} clauses", directive.kind, directive.clauses.len());
}
```

More Rust, C, and C++ samples live in [`examples/`](examples/).

## Build and test

```bash
cargo build --release
cargo test
```

Build the mdBook with `cargo doc --no-deps`.

## ompparser compatibility

[`compat/ompparser/`](compat/ompparser/) ships headers that mirror the original
ompparser project. Build it with `./compat/ompparser/build.sh` or follow the
manual steps in [the compatibility guide](docs/book/src/ompparser-compat.md).

## Contributing

Read the [contributing guide](https://roup.ouankou.com/contributing.html) for
coding standards, tests, and the pull-request workflow.

## License

BSD-3-Clause License - see [LICENSE](LICENSE) file for details.

**Copyright © 2024-2025 Anjia Wang**
