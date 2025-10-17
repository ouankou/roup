# Translating C Pragmas to Fortran Sentinels

C and C++ source files use `#pragma omp` while Fortran relies on `!$omp`.
Many OpenMP datasets – including the DataRaceBench micro benchmarks – only
ship C versions of kernels.  To experiment with Fortran tooling you previously
had to manually rewrite every directive.  ROUP now automates this workflow.

## Quick Example

```rust
use roup::ir::translate::translate_c_to_fortran;

let input = "#pragma omp parallel for private(i)";
let output = translate_c_to_fortran(input)?;
assert_eq!(output, "!$omp parallel do private(i)");
# Ok::<(), roup::ir::translate::TranslationError>(())
```

The helper parses the pragma using the C lexer, converts it into the semantic
IR and re-emits the directive with Fortran spelling rules:

- `#pragma omp` → `!$omp`
- loop constructs swap `for` for `do`
- combined constructs (e.g. `parallel for`, `target teams distribute parallel for simd`)
  use the appropriate Fortran synonyms.

## Handling Clauses

Clause spellings remain unchanged.  ROUP preserves the original clause order
and spacing, so the translation keeps semantic intent intact while only
adjusting the directive keyword itself.

```rust
use roup::ir::translate::translate_c_to_fortran;

let input = "#pragma omp for nowait collapse(2) schedule(dynamic, chunk)";
let output = translate_c_to_fortran(input)?;
assert_eq!(
    output,
    "!$omp do nowait collapse(2) schedule(dynamic, chunk)"
);
# Ok::<(), roup::ir::translate::TranslationError>(())
```

## When Translation Fails

`translate_c_to_fortran` returns a `TranslationError` in three situations:

1. **Empty input** – nothing to translate.
2. **Parser errors** – the directive could not be recognised as valid OpenMP.
3. **Conversion errors** – the directive or clauses are currently unsupported
   by ROUP's IR layer.

These errors implement `std::error::Error`, so they integrate cleanly with
common error-handling approaches such as `anyhow` or `thiserror`.

## Advanced Usage

If you need to customise expression parsing (for example to disable expression
parsing entirely) call `translate_c_to_fortran_ir` directly.  It accepts an
explicit `ParserConfig` and returns the `DirectiveIR`, letting you inspect or
further transform the semantic representation before rendering it to text.

```rust
use roup::ir::{translate::translate_c_to_fortran_ir, Language, ParserConfig};

let config = ParserConfig::string_only(Language::C);
let directive = translate_c_to_fortran_ir("#pragma omp for", config)?;
assert!(directive.language().is_fortran());
# Ok::<(), roup::ir::translate::TranslationError>(())
```

This is especially useful when integrating ROUP into larger pipelines such as
automatic benchmark converters or source-to-source translators.
