# OpenMP coverage

ROUP tracks the OpenMP 6.0 surface area in the mdBook. Refer to the following
chapters for the canonical lists:

- [Directive catalogue](book/src/openmp60-directives-clauses.md)
- [Directive/clause component index](book/src/openmp60-directive-clause-components.md)
- [Restrictions and notes](book/src/openmp60-restrictions.md)

The parser registers every directive and clause keyword listed there and rejects
unknown tokens so unsupported constructs fail fast. Tests in
`tests/openmp_keyword_coverage.rs` ensure the registry stays in sync with the
specification.

Looking for OpenACC details? See [`OPENACC_SUPPORT.md`](OPENACC_SUPPORT.md).
