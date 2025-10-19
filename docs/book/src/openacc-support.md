# OpenACC Support Matrix

The OpenACC implementation in ROUP tracks the entire OpenACC 3.4 specification.
The following chapters provide the canonical keyword lists:

- [Directive and clause catalogue](./openacc/openacc-3-4-directives-clauses.md)
- [Directive–clause matrix](./openacc/openacc-3-4-directive-clause-matrix.md)
- [Restrictions digest](./openacc/openacc-3-4-restrictions.md)

All entries are validated against the official
[OpenACC Application Programming Interface Version 3.4](https://www.openacc.org/sites/default/files/inline-images/Specification/OpenACC-3.4.pdf)
PDF. The `tests/openacc_roundtrip.rs` integration suite exercises every keyword
and alias, while the accparser compatibility tests mirror the coverage for the
C++ shim.

For a printable summary, see [`docs/OPENACC_SUPPORT.md`](../OPENACC_SUPPORT.md).
