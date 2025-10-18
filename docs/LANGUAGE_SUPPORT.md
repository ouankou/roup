# Language-aware clause parsing

ROUP includes lightweight parsing helpers that understand the OpenMP-specific
syntax emitted by C, C++, and Fortran front-ends without implementing full
language grammars. The helpers recognise array sections, mapper prefixes, and
other surface syntax so the IR receives structured data instead of raw strings.

## Supported features

- **C and C++**: bracketed array sections, nested dimensions, template
  arguments, namespaces, and quoted strings remain intact when clauses are
  split.
- **Fortran**: parenthesised array sections, rank separators, all-elements (`:`)
  markers, and case-insensitive identifiers are normalised for the IR.
- **Shared helpers**: mapper prefixes, ternary expressions, and balanced
  delimiter tracking avoid splitting tokens inside expressions.

## Configuration

Language awareness is on by default through `ParserConfig::with_parsing`. Disable
it when you need the legacy string-only behaviour:

```rust
use roup::ir::{Language, ParserConfig};

let config = ParserConfig::with_parsing(Language::C)
    .with_language_semantics(false);
```

## Implementation notes

Relevant modules live in `src/ir/lang` (token splitting utilities) and
`src/ir/convert.rs` (clause conversion). Array sections map to
`ir::variable::ArraySection`, while identifier lists become
`ir::variable::Variable` instances with optional section data.

## Testing

`cargo test language_parsing` runs the unit tests for the helpers. Integration
coverage lives in `tests/ir_roundtrip.rs` and friends, ensuring structured data
survives parse → IR → display cycles.

## Limitations

- Mapper modifiers (`mapper(name) modifier(list)`) are parsed conservatively and
  expose raw strings for now.
- The pretty-printer currently renders Fortran sections using C-style brackets.
- Member access such as `array(i)%field` is treated as a single identifier.

Planned work includes full mapper modifier support, richer Fortran display, and
additional performance profiling once the syntax surface stabilises.
