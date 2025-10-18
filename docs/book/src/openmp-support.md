# OpenMP support

ROUP registers every directive and clause keyword from the OpenMP 6.0
specification. The canonical listings live in the generated reference chapters:

- [Directive catalogue](./openmp60-directives-clauses.md)
- [Directive/clause components](./openmp60-directive-clause-components.md)
- [Restrictions](./openmp60-restrictions.md)

## Quick facts

| Feature | Details |
| --- | --- |
| OpenMP versions | 3.0 – 6.0 |
| Directive keywords | 128 |
| Clause keywords | 132 |
| Languages | C, C++, Fortran |
| Automated tests | 620 |

Unknown directives and clauses are rejected so unsupported constructs fail fast.
The `tests/openmp_keyword_coverage.rs` integration test keeps the registry in
sync with the specification, and additional round-trip tests validate parsing,
IR conversion, and display.
