# C to Fortran Directive Translation

OpenMP toolchains often need both C/C++ **and** Fortran versions of the same
benchmark. ROUP can now take a directive parsed from C/C++ syntax and emit a
Fortran equivalent without reparsing the source code.

## When to Use

- Generating Fortran test cases from existing C benchmarks
- Normalising mixed-language code bases to a single style
- Porting ompparser-driven tooling to ROUP while preserving Fortran output

## Quick Example

```rust
use roup::ir::{convert::convert_directive, Language, ParserConfig, SourceLocation};
use roup::parser::parse_omp_directive;

let input = "#pragma omp parallel for schedule(dynamic, 4)";
let (_, directive) = parse_omp_directive(input).unwrap();
let config = ParserConfig::default();

// Tell the IR layer to emit Fortran
let ir = convert_directive(&directive, SourceLocation::start(), Language::Fortran, &config)
    .unwrap();

assert_eq!(ir.to_string(), "!$omp parallel do schedule(dynamic, 4)");
```

The parser still sees the original C syntax. By selecting `Language::Fortran`
when constructing the IR (or later via `DirectiveIR::with_language`), the
Display implementation substitutes Fortran spellings:

| Canonical Kind | C/C++ Output                | Fortran Output                    |
|----------------|-----------------------------|-----------------------------------|
| `parallel for` | `#pragma omp parallel for`  | `!$omp parallel do`               |
| `for`          | `#pragma omp for`           | `!$omp do`                        |
| `teams distribute parallel for` | `#pragma omp teams distribute parallel for` | `!$omp teams distribute parallel do` |

## Translating Existing IR

Already have a `DirectiveIR` from C code? Flip the language in place or return a
new value:

```rust
use roup::ir::{DirectiveIR, DirectiveKind, Language, SourceLocation};

let mut ir = DirectiveIR::simple(
    DirectiveKind::ParallelFor,
    "parallel for",
    SourceLocation::start(),
    Language::C,
);

ir.set_language(Language::Fortran);
assert_eq!(ir.to_string(), "!$omp parallel do");

let teams = DirectiveIR::simple(
    DirectiveKind::TeamsDistributeParallelFor,
    "teams distribute parallel for",
    SourceLocation::start(),
    Language::C,
)
.with_language(Language::Fortran);

assert_eq!(teams.to_string(), "!$omp teams distribute parallel do");
```

## Compatibility Layer

The ompparser compatibility shim inherits the new behaviour automatically. Any
caller that sets the base language to Fortran now receives Fortran-prefixed
strings even when the original input used `omp ...` tokens.

## Next Steps

- Combine with [`Parser::with_language`](./fortran-tutorial.md) for full
  Fortran parsing.
- Feed the translated strings into ROSE or other Fortran-aware toolchains.
