# accparser compatibility audit

The compatibility criterion is the complete test suite from the pinned
accparser submodule, registered unchanged with `add_subdirectory`. At the
currently recorded revision, CTest runs all 918 upstream tests and two
repository-owned tests, for 920 tests total.

The build does not copy or edit upstream fixtures, patch reference output,
replace expected failures, maintain an allowlist, or discover only a selected
subset. An upstream registration failure, parse mismatch, missing file,
nonzero runner result, or timeout fails the compatibility gate.

## Adapter boundary

The Rust frontend remains authoritative. Directives, parameters, clauses,
modifiers, device types, locators, cache items, queue expressions, reduction
operators, aliases, and host expressions are parsed into typed AST nodes before
the C ABI exposes them. Closed semantics cross as numeric tags and structured
alternatives cross as child nodes. The adapter never classifies an enum from a
display name or reparses a rendered payload.

Source spelling is retained as typed expression provenance where accparser's
public round-trip contract requires it. This is not an opaque fallback: every
such leaf has a validated host-expression AST, and semantic consumers use that
tree. If the upstream IR cannot represent a typed value without changing its
meaning, conversion fails instead of dropping, defaulting, or substituting
data.

## Compatibility behavior covered upstream

The upstream suite exercises C, C++, and Fortran source forms; the complete
builtin fixture corpus; OpenACC-VV sources; default and explicit language
selection; historical aliases; clause merging and order; cache and array
sections; atomic expressions; source spelling; unparsing; and the public
accparser callers. The local tests additionally audit numeric enum conversion,
field consumption, source locations, malformed-input behavior, and the absence
of string-based closed-enum classification.

Failures found while enabling the complete suite resulted in typed frontend or
ABI changes, including source provenance for standard aliases, scalar cache
items, wait keyword state, `indirect`, historical qualified values, reduction
subtraction, and separate source-preserving versus compact AST rendering at the
two compatibility boundaries. No upstream fixture or expected output is
rewritten.
