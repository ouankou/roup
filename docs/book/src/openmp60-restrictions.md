# OpenMP validation boundaries

The OpenMP specification defines syntax restrictions as well as rules that
depend on an enclosing program. ROUP separates those two categories without
silently skipping either one.

For the normative rules, consult the relevant directive and clause sections in
the [OpenMP 6.0 specification](https://www.openmp.org/wp-content/uploads/OpenMP-API-Specification-6-0.pdf)
or the earlier specification that introduced historical syntax.

## Checked by every parse

`OpenMpParser::parse` checks everything that can be decided from one directive
and its configured profiles, including:

- source-form, sentinel, continuation, UTF-8, and trailing-input validity;
- complete directive, parameter, clause, and modifier syntax, plus the
  explicitly supported typed host-expression grammar;
- feature introduction for the selected OpenMP and host-language versions;
- directive-clause compatibility and duplicate singleton clauses;
- structurally invalid nested directives and selectors; and
- other context-independent restrictions represented by the validator.

Failure returns one structured `Diagnostic`. A parse never returns a partial
AST, recovery node, warning-only substitute, or guessed default.
Host-language constructs outside the documented typed expression grammar are
also hard errors; ROUP does not claim to replace a complete C, C++, or Fortran
frontend.

## Facts supplied by an embedding compiler

Some specification rules require information outside the directive text, such
as declaration placement, construct association, name resolution, or whether a
host expression is constant. Use `OpenMpParser::parse_with_facts` when those
checks apply.

Facts required by the parsed construct are mandatory. A missing fact is a hard
diagnostic rather than permission to bypass the check. The compiler remains
responsible for producing truthful facts from its program representation.

## Stateful region validation

`ContextValidator` validates directive sequences whose correctness depends on
previous input, including begin/end pairing and association state. Opening and
closing locations use checked spans, so mismatches can report the related
source location.

## Contributor rule

When adding standardized syntax, record its introduction version, construct a
fully typed payload, implement all context-independent restrictions, identify
every required external fact, and add both positive and negative public-API
tests. Do not add a permissive grammar branch while deferring malformed states
to a renderer or compatibility adapter.
