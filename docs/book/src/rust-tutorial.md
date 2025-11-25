# Rust Tutorial

This tutorial shows the current Rust entry points for parsing and inspecting OpenMP and OpenACC directives.

---

## Basic Parsing

```rust,ignore
use roup::parser::openmp;
use roup::lexer::Language;

fn main() {
    let parser = openmp::parser().with_language(Language::C);
    let (_, directive) = parser
        .parse("#pragma omp parallel for num_threads(4)")
        .expect("parse");

    println!("name: {}", directive.name);
    println!("clauses: {}", directive.clauses.len());
}
```

OpenACC works the same way:

```rust,ignore
use roup::parser::openacc;
use roup::lexer::Language;

let parser = openacc::parser().with_language(Language::FortranFree);
let (_, directive) = parser.parse("!$acc parallel async(1)").unwrap();
println!("name: {}", directive.name);
```

`Language` controls sentinel parsing (`C`, `FortranFree`, `FortranFixed`). `openmp::parse_omp_directive` and `openacc::parse_acc_directive` provide convenience wrappers if you do not need to customise the parser.

---

## Error Handling

```rust,ignore
use roup::parser::openmp;

fn parse_or_report(input: &str) {
    let parser = openmp::parser();
    match parser.parse(input) {
        Ok((rest, dir)) if rest.trim().is_empty() => {
            println!("parsed {}", dir.name);
        }
        Ok((rest, _)) => eprintln!("trailing tokens: {rest:?}"),
        Err(e) => eprintln!("parse error: {e:?}"),
    }
}
```

---

## Working with Clauses

```rust,ignore
use roup::parser::openmp;

let parser = openmp::parser();
let (_, directive) = parser.parse("#pragma omp parallel private(x) reduction(+:sum)").unwrap();

for clause in &directive.clauses {
    match clause.name.as_ref() {
        "private" => println!("private({})", clause.kind),
        "reduction" => println!("reduction({})", clause.kind),
        other => println!("clause: {other}"),
    }
}
```

`clause.kind` preserves the original clause text (`Parenthesized` or `Bare`).

---

## Building AST/IR

Use `Parser::parse_ast` to build the IR and normalise clauses:

```rust,ignore
use roup::ast::ClauseNormalizationMode;
use roup::ir::ParserConfig;
use roup::parser::openmp;

let parser = openmp::parser();
let ir = parser
    .parse_ast(
        "#pragma omp parallel for private(i,j)",
        ClauseNormalizationMode::ParserParity,
        &ParserConfig::default(),
    )
    .unwrap();
println!("{}", ir.to_string());
```

---

## Translation API

Translate between C/C++ and Fortran OpenMP pragmas:

```rust,ignore
use roup::ir::translate::{translate_c_to_fortran, translate_fortran_to_c};

let f = translate_c_to_fortran("#pragma omp parallel for private(i)")?;
assert_eq!(f, "!$omp parallel do private(i)");

let c = translate_fortran_to_c("!$omp target teams distribute")?;
assert_eq!(c, "#pragma omp target teams distribute");
```

---

## Debugger

`roup_debug` (built with the crate) provides interactive and batch step tracing:

```bash
cargo run --release --bin roup_debug '#pragma omp parallel' -- --non-interactive
```

It supports OpenMP and OpenACC pragmas in C and Fortran forms.
