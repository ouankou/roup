# OpenMP support

ROUP models standardized OpenMP syntax from 1.0 through 6.0 for C, C++, and
Fortran. These chapters describe the strict public behavior:

- [Syntax catalogue and version policy](./openmp60-directives-clauses.md)
- [Typed directive and clause components](./openmp60-directive-clause-components.md)
- [Validation boundaries](./openmp60-restrictions.md)

## Version behavior

The default policy accepts the union of standardized historical syntax. An
exact version rejects syntax introduced later while continuing to accept all
older standardized syntax, including syntax deprecated or removed by the
selected later specification.

Standard aliases are accepted and canonicalized into one semantic AST shape.
Their introduction provenance participates in availability checks, and their
checked source spans retain the exact physical location.

## Strict typed behavior

A successful parse contains typed directive parameters, clause payloads,
selectors, locators, allocator and induction records, and host expressions.
Unknown syntax, malformed payloads, nonstandard extensions, invalid
directive-clause combinations, duplicates, unavailable features, and trailing
input are hard errors.

Optional semantic arguments added in OpenMP 6.0 are named `Option<Expression>`
fields in their typed payloads. Historical bare forms remain accepted; empty
parentheses never stand in for an omitted argument. Atomic compound spellings
canonicalize to an `atomic` directive plus typed operation and memory-order
clauses.

`parse` applies context-independent rules. `parse_with_facts` applies rules
whose facts must come from an embedding compiler, and `ContextValidator`
checks stateful region pairing across a sequence.

Regression coverage uses the configured public API in the feature-availability,
context-validation, host-profile, source-span, directive-parameter,
strict-payload, and strict-error test suites.
