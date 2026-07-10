# OpenACC coverage

ROUP parses the standardized OpenACC 1.0 through 3.4 syntax surface in C, C++,
and Fortran source forms. The reference chapters cross-check the OpenACC 3.4
vocabulary and restrictions against the
[OpenACC 3.4 specification](https://www.openacc.org/sites/default/files/inline-images/Specification/OpenACC-3.4.pdf):

- [Directive and clause catalogue](book/src/openacc/openacc-3-4-directives-clauses.md)
- [Directive-clause matrix](book/src/openacc/openacc-3-4-directive-clause-matrix.md)
- [Restriction digest](book/src/openacc/openacc-3-4-restrictions.md)

## Public parsing contract

Use `OpenAccConfig` with an explicit host profile, source form, and either the
default union policy or an exact `OpenAccVersion`. Exact mode is an introduction
ceiling: it rejects syntax standardized later than the selected version while
continuing to accept all older standardized syntax.

Directives, directive parameters, clauses, data modifiers, locators, queue
expressions, reduction operators, and host expressions are returned as typed
AST data. The `cache`, `wait`, and `routine` directive parameters are parsed
once into dedicated parameter types. Unknown and malformed forms never become
raw payload strings.

## Standard aliases

OpenACC standard aliases are accepted and canonicalized. This includes
`dtype` and the historical `pcopy*`, `pcreate`, and `present_or_*` data-clause
spellings. On `update`, the historical `host(var-list)` spelling is
canonicalized to the same `self` clause kind and item-list payload as
`self(var-list)`; typed provenance retains its OpenACC 1.0 introduction floor.
Multiword directives and the standardized `host_data` spelling are parsed
exactly as specified; a fabricated space/underscore alternative is a hard
error. Consumers see one semantic kind and one typed payload shape. The checked
source span still identifies the exact spelling in the caller's source when
source-level tooling needs it.

Aliases are not assigned ad hoc integer identities. The optional C ABI returns
an explicit `(dialect, ordinal)` kind value and exposes semantic data through
typed field descriptors and child-node handles.

## Validation

Public parsing rejects unknown keywords, malformed payloads, trailing input,
unavailable-version syntax, illegal directive-clause combinations, and
duplicate singleton clauses. Checks that require facts from an embedding
compiler use `parse_with_facts`; a required but missing fact is itself an
error.

Regression coverage is organized around the public typed API in
`tests/openacc_public_api.rs`, `tests/openacc_directive_parameters.rs`,
`tests/feature_availability.rs`, `tests/host_profile_gates.rs`, and the strict
payload and error suites. The optional `roup-capi` crate and accparser adapter
are tested separately from the safe Rust parser.
