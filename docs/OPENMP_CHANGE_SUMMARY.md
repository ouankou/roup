# OpenMP Parser Update Summary

This document captures the major updates completed while extending the parser to cover the
OpenMP 6.0 directive and clause surface area targeted by the current implementation.

## Parser architecture
- Normalized directive tokenization so the lexer now accepts identifiers containing underscores.
- Updated clause and directive registries to resolve the longest directive match, enabling
  combined constructs (for example `target teams distribute`) without ambiguity.
- Added metadata-driven registration helpers that build clause rules directly from enums,
  ensuring registry entries stay synchronized with the supported keyword list.

## Directive and clause coverage
- Introduced `OpenMpDirective` and `OpenMpClause` enums to type the parser configuration and
  eliminate duplicated directive or clause strings.
- Expanded the OpenMP registry to enumerate every parallel, target, teams, for, and task
  directive variant, including all defined combined forms.
- Registered parsing rules for each clause accepted by those directives, covering optional
  argument lists, value forms, and mutually exclusive specifiers.
- Configured the registry to reject unsupported clause keywords immediately to prevent silent
  acceptance of invalid directives.

## Testing
- Added focused integration tests per directive family (`tests/openmp_parallel.rs`,
  `tests/openmp_target.rs`, etc.) to verify accepted clause combinations and guard against
  regression.
- Extended the testing harness to exercise the rejection path for unknown clauses so the stricter
  validation logic remains enforced.

## Documentation
- Authored `docs/OPENMP_SUPPORT.md`, listing every directive, combined directive, and clause
  from the OpenMP 6.0 specification with explicit support indicators.
- Linked the support matrix from the root README so contributors can quickly assess coverage and
  remaining gaps.
- Documented the stricter clause handling policy introduced by the new registry defaults.
