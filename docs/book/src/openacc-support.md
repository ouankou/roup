# OpenACC support

ROUP models standardized OpenACC syntax from 1.0 through 3.4 for C, C++, and
Fortran. These reference chapters cross-check the 3.4 vocabulary and
restrictions against the official specification:

- [Directive and clause catalogue](./openacc/openacc-3-4-directives-clauses.md)
- [Directive-clause matrix](./openacc/openacc-3-4-directive-clause-matrix.md)
- [Restriction digest](./openacc/openacc-3-4-restrictions.md)

## Version behavior

`OpenAccConfig::new` accepts the union of standardized historical syntax.
`OpenAccConfig::exact` selects an introduction ceiling from OpenACC 1.0, 2.0,
2.5, 2.6, 2.7, 3.0, 3.1, 3.2, 3.3, or 3.4. Syntax introduced later is rejected;
older standardized syntax remains accepted.

## Typed and canonical results

Directive parameters, clauses, data modifiers, reduction operators, locators,
queue expressions, and embedded host expressions are typed before success is
returned. Standard aliases such as `dtype`, `pcopy*`, `pcreate`,
and `present_or_*` are accepted and canonicalized. The `host(var-list)` action
on `update` is likewise represented as the canonical `self` clause and typed
item-list payload while keeping its OpenACC 1.0 spelling floor. Multiword
directives and the standardized `host_data` spelling are parsed exactly;
fabricated space/underscore variants are rejected. The exact source spelling
remains available through its checked span, not as a competing semantic kind.

Unknown syntax, malformed payloads, nonstandard extensions, invalid
directive-clause combinations, duplicate singleton clauses, unavailable
features, and trailing input are hard errors. The parser never returns opaque
payload text or retries with a permissive grammar.

Public behavior is covered by `tests/openacc_public_api.rs`,
`tests/openacc_directive_parameters.rs`, `tests/feature_availability.rs`, and
the strict payload and error suites. C ABI and accparser adapter checks are
separate consumers of the safe parser.
