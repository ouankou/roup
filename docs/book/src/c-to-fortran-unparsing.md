# C to Fortran Unparsing

ROUP can now parse C/C++ OpenMP directives and re-emit them using Fortran
syntax. This capability is inspired by the ompparser feature request to
"unparse" C input into Fortran output so that benchmark suites such as
[DataRaceBench](https://github.com/LLNL/dataracebench) can automatically obtain
Fortran variants of their kernels.

## When to Use It

- You have C benchmarks with `#pragma omp` directives and need equivalent
  Fortran source.
- You are experimenting with cross-language tooling inside ROSE or other
  compiler infrastructures that expect Fortran directives.
- You want to verify that ROUP preserves semantics when switching directive
  dialects.

## API Overview

The [`DirectiveIR`](../ir/directive.html) type now exposes
[`to_string_in_language`](../ir/directive/struct.DirectiveIR.html#method.to_string_in_language),
which renders an already-parsed directive using the syntax of the requested
[`Language`](../ir/types/enum.Language.html).

```rust
use roup::ir::{convert::convert_directive, Language, ParserConfig, SourceLocation};
use roup::parser::parse_omp_directive;

let input = "#pragma omp target teams distribute parallel for simd";
let (_, directive) = parse_omp_directive(input)?;
let config = ParserConfig::default();
let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)?;

assert_eq!(ir.to_string(), "#pragma omp target teams distribute parallel for simd");
assert_eq!(
    ir.to_string_in_language(Language::Fortran),
    "!$omp target teams distribute parallel do simd"
);
```

The helper rewrites combined constructs so that Fortran receives the `do`
variants (`parallel do`, `target parallel do`, and friends) while keeping clause
text intact.

## Notes and Limitations

- Only directive keywords change; clause spellings remain identical because the
  OpenMP specification shares the same clause vocabulary between C/C++ and
  Fortran.
- The Fortran output uses the free-form sentinel `!$omp`. Fixed-form emission is
  not yet supported.
- The conversion preserves clause ordering and spacing but does not attempt to
  refactor expressions into Fortran-specific idioms.

If you need additional mappings, please [open an issue](https://github.com/ouankou/roup/issues).
