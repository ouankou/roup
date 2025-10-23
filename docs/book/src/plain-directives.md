# Plain Directive Templates

Many analyses need to understand which OpenMP constructs appear in a translation
unit without seeing project-specific identifiers or expressions. ROUP now
provides **template** renderings for directives and clauses that erase
user-provided symbols while preserving structural keywords.

## Rust API

Use `DirectiveIR::to_template_string` to obtain a symbol-free representation:

```rust
use roup::ir::{convert::convert_directive, Language, ParserConfig, SourceLocation};
use roup::parser::parse_omp_directive;

let (_, directive) = parse_omp_directive(
    "#pragma omp target data map(tofrom: arr[0:N]) map(to: b[0:N])"
).unwrap();
let config = ParserConfig::default();
let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
    .unwrap();
assert_eq!(
    ir.to_template_string(),
    "#pragma omp target data map(tofrom: ) map(to: )"
);
```

Individual clauses expose `ClauseData::to_template_string`
for the same purpose. Clause templates keep spec-defined keywords (for example
`tofrom` or `static`) and drop identifier lists, numeric expressions, and mapper
names.

## C API

The C interface now exposes `const char* roup_directive_template(const
OmpDirective*)`. The pointer remains valid until the directive is released with
`roup_directive_free`.

```c
OmpDirective* dir = roup_parse("#pragma omp for schedule(static,64)");
const char* plain = roup_directive_template(dir);
// -> "#pragma omp for schedule(static, )"
```

## ompparser Compatibility Layer

Compatibility builds receive the same data. After parsing with
`parseOpenMP(...)`, call `getPlainDirective(OpenMPDirective*)` declared in
`roup_compat.h`.

```cpp
DirectivePtr dir(parseOpenMP("omp target map(to: a[0:N])", nullptr));
const char* plain = getPlainDirective(dir.get());
// -> "#pragma omp target map(to: )"
```

The helper always returns `nullptr` for directives that were not created by the
ROUP compatibility layer, keeping existing ompparser behaviour untouched.

## Use Cases

- **Static analysis** – identify directives without leaking variable names.
- **Deduplication** – group directives by structure instead of exact syntax.
- **Telemetry** – collect usage statistics without exposing application data.
