# ompparser compatibility audit

The compatibility criterion is the complete test suite from the pinned
ompparser submodule, registered unchanged with `add_subdirectory`. At the
currently recorded revision, CTest runs all 1,534 upstream tests and three
repository-owned tests, for 1,537 tests total.

The build does not copy or edit upstream fixtures, patch reference output,
replace expected failures, maintain an allowlist, or discover only a selected
subset. An upstream registration failure, parse mismatch, missing file,
nonzero runner result, or timeout fails the compatibility gate.

## Adapter boundary

The Rust frontend remains authoritative. Directives, clauses, selectors,
locators, expressions, aliases, and historical syntax are parsed into typed
AST nodes before the C ABI exposes them. Closed semantics cross the ABI as
numeric tags and structured alternatives cross as child nodes. The adapter
does not inspect directive or clause names to recover an enum and does not
reparse a rendered payload.

Pinned ompparser sometimes stores lexical text where ROUP has a structured
node. The adapter may render a checked node only at that final C++ IR boundary,
because that is the upstream public representation. The Rust AST never carries
an opaque replacement string. If the upstream IR cannot represent a typed
value without changing its meaning, conversion fails instead of dropping,
defaulting, or substituting data.

## Compatibility behavior covered upstream

The upstream suite exercises C, C++, and Fortran source forms; builtin and
OpenMP-VV cases; historical syntax; aliases; directive parameters; clause
merging and order; host expressions; source spelling; unparsing; and the public
ompparser examples. The local tests additionally check the typed ABI mapping,
adapter linkage, malformed-input behavior, and recently fixed structured
payloads.

Failures found while enabling the complete suite resulted in typed frontend or
ABI changes, including explicit provenance for historical aliases and clause
separators, typed array shaping and `depobj` lvalues, typed OMPX payloads,
Fortran defined operators, C++ template identifiers, and structured induction,
apply, and reduction data. No late source rewrite is used to make the upstream
expected output pass.
