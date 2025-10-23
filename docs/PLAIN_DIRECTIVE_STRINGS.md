# Plain Directive Strings in ROUP

The [ompparser issue #71](https://github.com/passlab/ompparser/issues/71) requests an
API that returns the structure of an OpenMP directive without exposing the
user-provided identifiers and expressions. This feature is now supported across
all ROUP entry points.

## Rust API (`DirectiveIR`)

* `DirectiveIR::to_plain_string()` produces a normalized directive string that
  preserves the directive kind, clause names, and spec-defined modifiers while
  removing identifiers, variables, and expressions supplied by the user.
* Example:

  ```rust
  use roup::ir::{ClauseData, DirectiveIR, DirectiveKind, Language, MapType, SourceLocation};

  let directive = DirectiveIR::new(
      DirectiveKind::TargetData,
      "target data",
      vec![ClauseData::Map {
          map_type: Some(MapType::ToFrom),
          mapper: None,
          items: vec![],
      }],
      SourceLocation::start(),
      Language::C,
  );

  assert_eq!(directive.to_plain_string(), "#pragma omp target data map(tofrom: )");
  ```

## C API

* New field `plain` is stored inside the opaque `OmpDirective`.
* New function `const char* roup_directive_plain_string(const OmpDirective*)`
  returns the sanitized string. The pointer remains valid until
  `roup_directive_free` is called.

## ompparser Compatibility Layer

* `parseOpenMP` stores the plain string in an internal `RoupDirective` subclass.
* New helper `const char* roup_get_plain_directive_string(const OpenMPDirective*)`
  exposes the sanitized directive to compatibility users.

## Testing

* Unit tests cover `DirectiveIR::to_plain_string()` for map clauses, mixed
  clauses, and language prefixes.
* Integration tests verify the parser → IR → plain-string pipeline and the C
  API function.
* Compatibility tests in `comprehensive_test.cpp` assert the plain string for
  representative directives.

These additions guarantee that every consumer of ROUP can obtain a
spec-compliant textual skeleton of parsed directives without leaking user
identifiers.
