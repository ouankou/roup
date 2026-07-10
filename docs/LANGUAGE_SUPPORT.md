# Host-language parsing

Every public parse selects an exact host-language profile and a compatible
source form. C and C++ profiles use `SourceForm::Pragma`; Fortran profiles use
`SourceForm::FortranFree` or `SourceForm::FortranFixed`. An incompatible pair
is a configuration error before directive parsing begins.

```rust
use roup::api::OpenMpConfig;
use roup::version::{CppStandard, HostLanguageProfile, SourceForm};

let parser = OpenMpConfig::new(
    HostLanguageProfile::Cpp(CppStandard::Cpp23),
    SourceForm::Pragma,
)?
.parser();

let parsed = parser.parse("#pragma omp target map(to: object.values[0:n])")?;
assert_eq!(parsed.directive().kind().as_str(), "target");
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Typed host data

Embedded host syntax is parsed before a directive is returned:

- Expressions are represented by `roup::host::Expr` trees. Literals,
  operators, calls, members, subscripts, C/C++ array sections, and Fortran
  section triplets have distinct typed nodes.
- Identifiers and qualified names use validated identifier types.
- Variable locators reuse the same expression tree and accept only variable
  designators. Arithmetic, conditionals, calls, and literals cannot be stored
  as locators.
- Type names are lexed into typed tokens and checked for empty input,
  unsupported characters, invalid boundaries, and unbalanced delimiters.
- Fortran section bounds retain upper-bound semantics; they are not rewritten
  into synthetic C-style length expressions.

Source text may be retained as backing storage for checked spans, but semantic
consumers inspect the typed tree. ROUP has no string-only expression, locator,
or type-name alternative.

## Host-standard gates

The profile is semantic, not descriptive metadata. Syntax unavailable in the
selected C, C++, or Fortran standard is rejected. Choosing a newer profile can
enable newer host-language syntax without changing the OpenMP or OpenACC
version policy.

## Hard-error boundary

Unknown tokens, unbalanced delimiters, empty list elements, malformed array
sections, trailing input, and invalid host/source-form combinations are hard
errors. ROUP does not guess a default language, retain an unclassified string,
or retry with a more permissive parser.

Focused coverage lives in the host parser unit tests and in
`tests/host_profile_gates.rs`, `tests/strict_clause_payloads.rs`, and
`tests/strict_error_regressions.rs`.
