# OpenMP coverage

ROUP parses standardized OpenMP syntax from 1.0 through 6.0 in C, C++, and
Fortran source forms. The reference chapters summarize the syntax and point to
the [OpenMP 6.0 specification](https://www.openmp.org/wp-content/uploads/OpenMP-API-Specification-6-0.pdf):

- [Syntax catalogue and version policy](book/src/openmp60-directives-clauses.md)
- [Typed directive and clause components](book/src/openmp60-directive-clause-components.md)
- [Validation boundaries](book/src/openmp60-restrictions.md)

## Cumulative exact versions

`OpenMpConfig::exact` selects an introduction ceiling. Syntax first
standardized after that version is rejected. All standardized syntax from
earlier specifications remains accepted, including syntax deprecated or
removed from a later specification. This policy is deliberate so maintained
historical code remains parseable.

Standard aliases and replacement spellings are canonicalized into one typed
AST shape. Alias provenance participates in version checks, and checked spans
continue to identify the exact source spelling.

The Fortran-only `end allocators` and optional `end dispatch` paired endings
are available from OpenMP 5.2 onward. Their compact blank-insensitive spellings
map to the same typed directive kinds, and `ContextValidator` pairs them only
with `allocators` and `dispatch`, respectively.

## Strict typed results

Directive parameters, clauses, modifiers, selectors, locators, induction
descriptions, allocator specifications, reductions, and host expressions have
typed representations. Unknown syntax, malformed payloads, nonstandard
extensions, invalid directive-clause combinations, duplicates, unavailable
features, and trailing input are hard errors.

OpenMP 6.0 optional semantic arguments are retained as named optional
expressions in the clause payload (`required`, `use_semantics`,
`do_not_synchronize`, `can_assume`, and the clause-specific equivalents).
Their historical bare forms remain valid, while an empty `clause()` is a hard
syntax error. Atomic compound spellings are canonicalized as an `atomic`
directive with typed operation and memory-order clauses.

The OpenMP 6.0 universal `directive-name-modifier` is stored once on every
`OmpClause`, including the historical OpenMP 4.5 `if` form. The named target
must be the directive itself or an eligible constituent; overlapping `if`
targets and duplicate modifiers are hard errors. `threadset` and `memscope`
use closed enums, `looprange` has exactly ordered `first`/`count` expressions
and is fuse-only, and `graph_reset` retains its errata-defined bare form.
Mapper identifiers use one shared `OmpMapperId` that distinguishes `default`
from user identifiers in both declarations and map clauses.

Plain `parse` performs context-independent checks. `parse_with_facts` accepts
facts supplied by an embedding compiler for rules such as declaration context,
association, or constant-expression classification; missing required facts
are hard errors. `ContextValidator` checks stateful region pairing across a
directive sequence.

Public-API regression coverage includes `tests/feature_availability.rs`,
`tests/context_validation.rs`, `tests/host_profile_gates.rs`,
`tests/source_span_regressions.rs`, `tests/openmp_directive_parameters.rs`,
`tests/openmp_historical_legality.rs`, and the strict payload and error suites.
