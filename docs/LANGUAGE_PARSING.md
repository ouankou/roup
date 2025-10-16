# Language-Aware Clause Item Parsing

This document describes the language-sensitive clause item parser that backs
ROUP's OpenMP IR conversion pipeline.

## Overview

* The parser lives in [`src/ir/language_support.rs`](../src/ir/language_support.rs).
* It understands the subset of C/C++ and Fortran syntax that appears inside
  OpenMP pragma clause lists (identifiers and array section selectors).
* Parsed items are converted into structured [`ClauseItem::Variable`] values so
  downstream consumers have access to array section structure, not just raw
  strings.
* Expression fragments (bounds, strides, etc.) are stored as [`Expression`]
  nodes using the configured [`ParserConfig`].

## Feature Flag

The module is gated behind the `language-parsing` Cargo feature and enabled by
default. To build without language-aware parsing (restoring the legacy "split
by comma" behaviour):

```bash
cargo build --no-default-features
```

With the feature disabled all clause items are treated as plain identifiers and
no structured array information is emitted.

## Supported Syntax

### C / C++

* Simple identifiers (including namespaces such as `ns::value`).
* Multidimensional array sections using the OpenMP notation
  `var[lower:length:stride]`.
* Nested sections (`matrix[i][j:k]`) are preserved as separate dimensions.
* Template arguments are respected when splitting on commas:
  `std::map<int, float>` remains a single item.

### Fortran

* Identifiers and derived type components (e.g. `state%field`).
* Array sections inside parentheses: `a(1:n, :)` creates two dimensions.
* Triplets `lower:upper:stride` are recorded verbatim in the IR. Because the IR
  stores generic `ArraySection` data, the upper-bound is currently placed in the
  `length` slot. This mirrors previous behaviour and keeps the full source text
  intact.
* A selector consisting only of `:` becomes an "all elements" section. When the
  IR is formatted back to a string this appears as an empty selector (`[]`),
  mirroring the existing IR printer.

## Error Handling

The parser surfaces invalid constructs as
`ConversionError::InvalidClauseSyntax`. Examples include unmatched brackets or
unknown directive names. The fallback implementation (used when
`language-parsing` is disabled) never raises syntax errors because it does not
inspect clause content.

## Limitations

* The printer for `Variable` always uses the C-style `[]` delimiters, even when
  the source language is Fortran. Consumers that require exact round-tripping
  should rely on the original clause text rather than `Display` output.
* Component references such as `array(i)%field` are treated as a single
  variable; member access beyond the base identifier is preserved in the name
  string.

## Tests

Extensive unit tests cover:

* `src/ir/language_support.rs` – language-specific splitting and array
  section parsing.
* `src/ir/convert.rs` – integration of the module when building `ClauseData`
  structures for both C/C++ and Fortran directives.
* `compat/ompparser` – the compatibility layer is rebuilt and its regression
  suite passes after enabling the feature.

