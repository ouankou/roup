# Plain Directive Rendering

OpenMP directives often include application-specific identifiers such as variable names,
array sections, or expressions. For tooling, analytics, or telemetry it is useful to
examine the *structure* of a directive without exposing those symbols. ROUP now provides
a "plain" view that keeps directive kinds, clause names, and modifiers while replacing
user-provided data with the placeholder `...`.

## Rust API

```rust
use roup::ir::{convert::convert_directive, Language, ParserConfig, SourceLocation};
use roup::parser::parse_omp_directive;

let input = "#pragma omp target data map(tofrom: a[0:N]) map(to: b[0:N])";
let (_, parsed) = parse_omp_directive(input)?;
let config = ParserConfig::with_parsing(Language::C);
let ir = convert_directive(&parsed, SourceLocation::start(), Language::C, &config)?;

assert_eq!(
    ir.to_plain_string(),
    "#pragma omp target data map(tofrom: ...) map(to: ...)"
);
```

The [`DirectiveIR::plain`](../src/ir/directive.rs) helper returns a display wrapper if
you prefer to format directly into an existing buffer. Clauses also expose
[`ClauseData::to_plain_string`](../src/ir/clause.rs) for granular inspection.

## C API

A new FFI function exposes the same capability:

```c
const OmpDirective* dir = roup_parse("#pragma omp parallel num_threads(4)");
const char* plain = roup_directive_plain(dir);
// plain == "#pragma omp parallel num_threads(...)"
```

The returned pointer remains valid until `roup_directive_free` is called. The original
API contracts (null checks, manual memory management) remain unchanged.

### Fortran Support

Both the Rust and C front-ends respect the language-specific sentinel. Passing a
Fortran directive such as `!$OMP PARALLEL REDUCTION(+:SUM)` produces a plain string
starting with `!$omp parallel` and clauses rendered with placeholders (for example
`reduction(...)`). This keeps Fortran and C directives comparable without leaking
identifiers.

### Fallback Formatting

When semantic conversion fails—typically because the clause payload is invalid or not
yet modelled—`DirectiveIR::plain` is unavailable. The C API therefore falls back to the
parser's structural view, emitting clause names with `...` placeholders. For example,
`#pragma omp parallel proc_bind(foo)` yields `#pragma omp parallel proc_bind(...)`
instead of bubbling an error. The compatibility layer mirrors this behaviour so existing
ompparser clients continue to receive usable output even for partially supported
features.

## ompparser Compatibility Layer

The compatibility shim adds `getPlainDirectiveString(const char*)` in
[`compat/ompparser/src/roup_compat.h`](../compat/ompparser/src/roup_compat.h). It
normalises input according to the active language (set via `setLang`) and returns the
plain directive string without leaking user identifiers:

```cpp
setLang(Lang_C);
std::string plain = getPlainDirectiveString("omp parallel reduction(+:sum)");
// plain == "#pragma omp parallel reduction(+: ...)"
```

For Fortran inputs the helper automatically injects the sentinel before delegating to
the Rust parser.

## Limitations

* Unknown or currently unsupported clauses fall back to the parser's structural view.
* Placeholders are always rendered as `...`; the API does not expose configurable
  replacement text yet.
* When IR conversion fails (for example due to a missing feature), the C API and
  compatibility layer degrade gracefully by showing clause names with placeholders.

These behaviours are covered by new unit, integration, and C++ compatibility tests to
prevent regressions.
