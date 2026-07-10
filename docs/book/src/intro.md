# ROUP

ROUP is a strict, safe Rust parser for OpenMP and OpenACC directives in C,
C++, and Fortran source forms. It returns typed syntax trees suitable for
compiler frontends and source tools.

The parser has four deliberate rules:

- A successful result contains typed directive, clause, selector, locator, and
  host-expression data.
- A problem is returned immediately as one structured diagnostic. Trailing
  input, unknown fields, incompatible clauses, and unavailable features are not
  ignored.
- Version selection is cumulative. An exact specification mode rejects syntax
  introduced later, while continuing to accept standardized syntax from older
  specifications even if a later specification deprecated or removed it.
- Standard spelling aliases are accepted at the grammar boundary and map to a
  canonical semantic representation.

The root `roup` package is the complete parser and contains no unsafe Rust. A
separate optional `roup-capi` package provides an opaque-handle C ABI. The
ompparser and accparser compatibility libraries are consumers of that ABI, not
alternate parser implementations.
