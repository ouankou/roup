# OpenACC coverage

ROUP tracks the complete OpenACC 3.4 keyword surface area in the mdBook. Use the
following chapters as the canonical references:

- [Directive and clause catalogue](book/src/openacc/openacc-3-4-directives-clauses.md)
- [Directive–clause matrix](book/src/openacc/openacc-3-4-directive-clause-matrix.md)
- [Restrictions digest](book/src/openacc/openacc-3-4-restrictions.md)

All directives, clauses, clause aliases, and modifiers defined by the official
[OpenACC Application Programming Interface Version 3.4](https://www.openacc.org/sites/default/files/inline-images/Specification/OpenACC-3.4.pdf)
are registered in the parser. Round-trip tests in
`tests/openacc_roundtrip.rs` and the accparser compatibility tests ensure the
registries stay synchronized with the specification.
