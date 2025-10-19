# OpenACC support

ROUP implements the complete OpenACC 3.4 directive and clause vocabulary. The
reference chapters below catalogue every keyword and restriction with section
and page references back to the official specification PDF.

- [Directive and clause index](./openacc/openacc-3-4-directives-clauses.md)
- [Directive–clause matrix](./openacc/openacc-3-4-directive-clause-matrix.md)
- [Restriction digest](./openacc/openacc-3-4-restrictions.md)

## Quick facts

| Feature | Details |
| --- | --- |
| OpenACC version | 3.4 |
| Directive keywords | 24 (including space/underscore aliases) |
| Clause keywords | 49 |
| Languages | C, C++, Fortran (free & fixed form) |
| Automated tests | Full directive and clause round-trip coverage |

Unknown directives and clauses are rejected so unsupported constructs fail fast.
The `tests/openacc_roundtrip.rs` suite keeps the registry in sync with the
specification and exercises round-trip formatting for every keyword, and the C
API mappings are validated to guarantee consistent clause kind values.
