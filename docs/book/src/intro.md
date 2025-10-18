# ROUP

ROUP is a Rust crate that parses OpenMP directives and exposes the resulting
structure through idiomatic Rust APIs and a small C interface.  The project is
actively developed and the public surface may change while the parser gains
coverage, so treat the current releases as experimental.

## Highlights

- Rust-first implementation with optional bindings for C, C++, and Fortran
  consumers.
- Grammar support that tracks the OpenMP 6.0 specification, including
  loop-transform directives and meta-directives validated by the integration
  tests in `tests/`.
- Language-aware clause parsing so constructs such as array sections or mapper
  expressions round-trip cleanly for C, C++, and Fortran sources.
- Compatibility shim in `compat/ompparser/` that mirrors the original ompparser
  ABI for existing code bases.

## Example

```rust,no_run
use roup::parser::openmp;

fn main() {
    let parser = openmp::parser();
    let pragma = "#pragma omp parallel for num_threads(4)";

    match parser.parse(pragma) {
        Ok((_, directive)) => {
            println!("directive: {}", directive.name);
            println!("clauses: {}", directive.clauses.len());
        }
        Err(err) => eprintln!("parse error: {err:?}"),
    }
}
```

Additional examples for C, C++, and Fortran are available in the `examples/`
directory and linked from the language-specific tutorials.

## Documentation map

- [Getting started](./getting-started.md) – setup instructions and quickstart
  snippets for each supported language.
- [Rust tutorial](./rust-tutorial.md) – working with the parser and IR from
  Rust.
- [C tutorial](./c-tutorial.md) and [C++ tutorial](./cpp-tutorial.md) – FFI usage
  patterns and resource-management tips.
- [Architecture](./architecture.md) – overview of the parser pipeline, IR, and
  FFI boundaries.
- [FAQ](./faq.md) – common questions about language coverage and interoperability.

## Status

The project ships with integration tests that exercise keyword registration,
loop transformation parsing, metadirective handling, and language specific
behaviour (see `tests/` for details).  Please run the test suite and the helper
scripts described in `TESTING.md` when contributing changes.
