# API Reference

This page points to the primary surfaces for Rust and C/C++/Fortran users. Full Rustdoc is generated as part of the build.

---

## Rust

- **Rustdoc:** `cargo doc --no-deps` generates the full API docs.
- **Parsing:** `parser::openmp::parser()` and `parser::openacc::parser()` return a `Parser` that accepts C or Fortran sentinels via `with_language(...)`. Convenience helpers `parse_omp_directive` / `parse_acc_directive` are also available.
- **AST/IR:** `Parser::parse_ast(...)` converts parsed directives into the IR layer for semantic queries and unparsing.
- **Translation:** `ir::translate::{translate_c_to_fortran, translate_fortran_to_c}` (and their `_ir` variants) map OpenMP directives between languages.
- **Debugger:** `roup_debug` binary wraps `debugger::DebugSession` for interactive and batch stepping.

Quick example:

```rust,ignore
use roup::parser::openmp;
use roup::lexer::Language;

let parser = openmp::parser().with_language(Language::C);
let (_, directive) = parser.parse("#pragma omp parallel for").unwrap();
println!("name: {}", directive.name);
```

---

## C / OpenACC APIs

`src/roup_constants.h` is generated at build time and contains:
- Directive/clause macro constants (`ROUP_OMPD_*`, `ROUP_OMPC_*`, `ROUP_ACCD_*`, `ROUP_ACCC_*`)
- Language codes: `ROUP_LANG_C = 0`, `ROUP_LANG_FORTRAN_FREE = 1`, `ROUP_LANG_FORTRAN_FIXED = 2`
- Unknown sentinel: `-1`

Key function families (OpenMP):
- `roup_parse`, `roup_parse_with_language`
- Iterators: `roup_directive_clauses_iter`, `roup_clause_iterator_next`, `*_free`
- Queries: `roup_directive_kind`, `roup_directive_clause_count`, `roup_clause_kind`, schedule/reduction/default queries, variable lists
- Translation helpers: `roup_convert_language`, `roup_string_free`

Key function families (OpenACC):
- `acc_parse`, `acc_parse_with_language`, `acc_directive_free`
- Clause queries: `acc_clause_kind`, `acc_clause_expressions_count`, `acc_clause_expression_at`, `acc_clause_modifier`, `acc_clause_operator`, `acc_clause_original_keyword`
- Directive extras: cache and wait accessors, routine name, end-paired mapping

Error handling:
- Functions returning pointers (e.g., `*_parse`, `*_iter`, string accessors) return `NULL` on error and must be checked before use.
- Functions returning integers (e.g., kind lookups) return `-1` for invalid inputs or unknown kinds.

---

## C++ RAII

See the [C++ tutorial](./cpp-tutorial.md) for RAII wrappers that call the C API and auto-free resources.
