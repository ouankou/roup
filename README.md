# Rust-based OpenMP/OpenACC Unified Parser (ROUP)

ROUP is a standalone, unified parser for OpenMP and OpenACC, developed using Rust. It is designed as an extensible framework that can be expanded to support additional directive-based programming interfaces.

## What You'll Learn

This project is structured to teach Rust programming from the ground up:

1. **Basics**: Structs, enums, lifetimes, pattern matching
2. **Intermediate**: Modules, traits, collections, builder pattern
3. **Advanced**: Parser combinators, function pointers, registries

Study the commit history to see how the project evolved!

## Build and Run

```bash
cargo build
cargo test
cargo run
```

## Features

- Parse OpenMP pragma directives
- Support for clauses with various formats
- Extensible registry system for directives and clauses
- Comprehensive test suite
- Round-trip parsing (parse → display → parse)

## Example

```rust
use roup::parser::Parser;

let input = "#pragma omp parallel private(a, b) nowait";
let parser = Parser::default();
let (_, directive) = parser.parse(input).unwrap();

println!("Directive: {}", directive.name);
for clause in directive.clauses {
    println!("Clause: {}", clause);
}
```

## OpenMP Support

See [OpenMP 6.0 Support Matrix](docs/OPENMP_SUPPORT.md) for details on which directives and clauses are currently supported.

## Project Structure

```
src/
├── lib.rs           - Library entry point
├── lexer.rs         - Tokenizer using nom
└── parser/          - Parser modules
    ├── mod.rs       - Parser module root
    ├── clause.rs    - Clause parsing and registry
    ├── directive.rs - Directive parsing and registry
    └── openmp.rs    - OpenMP-specific definitions
tests/               - Integration tests
utils/               - Example binaries
```

## License

MIT License - see LICENSE file for details.
