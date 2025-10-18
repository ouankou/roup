# Language-Aware Clause Parsing

The parser understands enough C, C++ and Fortran syntax to extract structured information from OpenMP clause payloads without
requiring full language front-ends. Highlights:

- Array sections and nested subscripts for bracket and parenthesis syntaxes.
- Mapper prefixes, ternary expressions and quoted strings without misinterpreting delimiters.
- Case-sensitive handling for C/C++ identifiers and the normalised behaviour expected by Fortran users.

Language semantics are enabled through `ParserConfig` and can be disabled when callers prefer raw strings. Consult the
`src/ir/lang/` module and the [architecture chapter](../docs/book/src/architecture.md) for implementation details and examples.
