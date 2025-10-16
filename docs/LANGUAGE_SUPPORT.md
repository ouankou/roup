# Language-Aware OpenMP Parsing

ROUP now provides a language-aware front-end that extracts semantic
information from OpenMP clauses for C, C++, and Fortran sources. The
feature lives in the `ir::language_support` module and is enabled by
default via `ParserConfig`.

## Key capabilities

* Parse C/C++ array sections such as `arr[0:N:2]` into `ArraySection`
  structures.
* Parse Fortran array sections (`array(1:n)`) while respecting the
  language's case-insensitive rules.
* Preserve clause items as `ClauseItem::Variable`, `ClauseItem::Identifier`,
  or `ClauseItem::Expression`, giving downstream tools structured data
  instead of raw strings.
* Respect OpenMP map clause prefixes, including optional
  `mapper(identifier)` components.

## Configuration

Language support is controlled through `ParserConfig`:

```rust
use roup::ir::{Language, ParserConfig};

// Enable expression + language parsing for C/C++
let config = ParserConfig::with_parsing(Language::C);

// Disable the language front-end if only raw identifiers are needed
let string_only = ParserConfig::with_parsing(Language::C)
    .with_language_support(false);
```

When disabled, ROUP falls back to the legacy behaviour where clause
lists are treated as comma-separated identifiers.

## When to use

* **Always enable** when accurate IR data is required (default).
* **Disable** for debugging, regression comparisons, or when a caller
  prefers to parse clause payloads independently.

See `tests/openmp_language_support.rs` for end-to-end examples covering
C and Fortran directives.
