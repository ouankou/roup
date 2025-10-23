# Plain Directive Strings

OpenMP directives often contain user-specific identifiers and expressions. When
analyzing large code bases or comparing pragmas, it is useful to normalize a
directive to its structural form while hiding project-specific symbols. ROUP now
provides a plain-string representation that keeps directive and clause keywords
but redacts variables, expressions, and other user-provided tokens.

## Rust API

1. Parse the directive and convert it to the IR layer.
2. Call [`DirectiveIR::to_plain_string()`](../../src/ir/directive.rs) to obtain the
   redacted string.

```rust
use roup::ir::{convert::convert_directive, Language, ParserConfig, SourceLocation};
use roup::parser::parse_omp_directive;

let input = "#pragma omp target data map(to: arr[0:N]) nowait";
let (_, directive) = parse_omp_directive(input)?;

let mut config = ParserConfig::default();
config.language = Language::C;
let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)?;

assert_eq!(
    ir.to_plain_string(),
    "#pragma omp target data map(to: <variable>) nowait"
);
```

The resulting string is safe to log or diff because all variables and expression
values are replaced with descriptive placeholders such as `<variable>` or
`<expr>`.

## C API

The C interface exposes a new accessor:

```c
const char* roup_directive_plain_string(const OmpDirective* dir);
```

The returned pointer remains valid until `roup_directive_free` is called. The
string uses the same redaction strategy as the Rust helper and automatically
selects the correct prefix for C/C++ (`#pragma omp`) or Fortran (`!$omp`).

## ompparser Compatibility Layer

Consumers of the ompparser drop-in replacement can retrieve the normalized
string after parsing:

```cpp
DirectivePtr dir(parseOpenMP("omp parallel private(x)", nullptr));
const char* plain = getPlainDirective(dir.get());
std::cout << plain; // "#pragma omp parallel private(<identifier>)"
```

`DirectivePtr` automatically calls `releasePlainDirective` to remove cached
strings during destruction.

## When to Use

- Generating fingerprints for pragmas without leaking internal symbol names.
- Comparing directive shapes across files or projects.
- Building tooling that focuses on directive structure rather than concrete
  identifiers.

The plain-string representation complements existing display methods; call
`to_string()` when you need the original pragma and `to_plain_string()` when you
need a sanitized version.

## Placeholder Reference

All placeholders follow a consistent vocabulary so that downstream tooling can
pattern match on specific shapes without consulting the original AST. The table
below summarizes the tokens currently emitted by the formatter:

| Placeholder      | Meaning                                                   |
| ---------------- | --------------------------------------------------------- |
| `<identifier>`   | Named variable or clause identifier                       |
| `<variable>`     | Array or variable reference with optional section syntax  |
| `<expr>`         | Arbitrary expression, literal, or evaluated clause value  |
| `<data>`         | Clause-specific payload such as mapper payloads           |

Additional placeholders may appear as new OpenMP constructs are implemented,
but the formatter always keeps directive keywords and enumerated clause values
in plain text.

## Additional Examples

```
#pragma omp parallel for if(n > 10) schedule(dynamic, chunk) reduction(+: sum)
```

Produces the following normalized string:

```
#pragma omp parallel for if(<expr>) schedule(dynamic, <expr>) reduction(+: <identifier>)
```

Fortran directives automatically adopt the language-specific prefix while still
redacting identifiers:

```
!$OMP target map(tofrom: A, B)
```

is rendered as:

```
!$omp target map(tofrom: <identifier>, <identifier>)
```
