# ROUP: Rust-based OpenMP & OpenACC Parser

Rust-first parsing for OpenMP **and** OpenACC directives with C, C++, and Fortran bindings, plus compatibility shims.

[![Docs](https://img.shields.io/badge/docs-roup.ouankou.com-blue)](https://roup.ouankou.com)
[![Status](https://img.shields.io/badge/status-experimental-orange)](https://github.com/ouankou/roup)

> **Experimental:** APIs continue to evolve. Expect breaking changes between releases.

## Quick start

```toml
# Cargo.toml
[dependencies]
roup = "0.7"
```

```bash
# C/C++/Fortran projects
cargo build --release
# Link against target/release/libroup.{a,so,dylib}
```

Platform-specific notes live in the [building guide](https://roup.ouankou.com/building.html).

## Highlights

- **OpenMP 3.0–6.0** and **OpenACC 3.4** coverage across directives, clauses, aliases, and combined forms.
- **Debugger:** `roup_debug` provides interactive and non-interactive step tracing for OpenMP/OpenACC (C and Fortran sentinels).
- **Rust, C, C++17, and Fortran** APIs with unsafe code confined to the FFI bindings.
- **Compatibility layers:** drop-in replacements for ompparser and accparser (see `compat/`).
- **Extensive tests:** hundreds of automated checks, an enum/safety audit, OpenMP_VV/OpenACCV-V validation, and compat ctests.

## Documentation

The mdBook at [roup.ouankou.com](https://roup.ouankou.com) provides tutorials, an architecture tour, and the API reference. Each chapter mirrors the sources under [`docs/book/src/`](docs/book/src/).

## Minimal example (Rust)

```rust
use roup::parser::openmp;
use roup::lexer::Language;

fn main() {
    let parser = openmp::parser().with_language(Language::C);
    let (_, directive) = parser
        .parse("#pragma omp parallel for num_threads(4)")
        .expect("valid directive");
    println!(
        "parsed {:?} with {} clauses",
        directive.name,
        directive.clauses.len()
    );
}
```

More C/C++/Fortran samples live in [`examples/`](examples/). The C/OpenACC headers are generated at build time as `src/roup_constants.h`.

## Build and test

```bash
cargo build --release
cargo test
./test.sh
```

Rebuild the docs with `cargo doc --no-deps` followed by `mdbook build docs/book`.

`roup_debug` is built alongside the library: `cargo run --release --bin roup_debug '#pragma omp parallel' -- --non-interactive`.

## Compatibility layers

- [`compat/ompparser/`](compat/ompparser/) — drop-in replacement for the original ompparser.
- [`compat/accparser/`](compat/accparser/) — drop-in replacement for accparser with ROUP_ACC_* constants.

Each directory includes a `build.sh` that builds ROUP, the shim, and runs the bundled ctest suites.

## Contributing

See the [contributing guide](https://roup.ouankou.com/contributing.html) for coding standards, test expectations, and the pull-request workflow.

## License

BSD-3-Clause License — see [LICENSE](LICENSE).

© 2024–2025 Anjia Wang
