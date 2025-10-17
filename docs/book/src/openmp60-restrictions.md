# OpenMP 6.0 Restrictions

The OpenMP specification attaches normative "Restrictions" text to almost every
directive and clause.  Rather than duplicating that material (which quickly
falls out of sync), this guide explains how to locate and verify the
restrictions relevant to any construct while working on ROUP.

## Where to look in the specification

- **Directive sections** – Each directive definition in Chapters 5–17 of the
  [OpenMP 6.0 specification](https://www.openmp.org/wp-content/uploads/OpenMP-API-Specification-6-0.pdf)
  ends with a *Restrictions* subsection.  The text typically begins with
  "Restrictions to the `<name>` directive are as follows".
- **Clause sections** – Clause descriptions follow the same pattern, usually
  starting with "Restrictions to the `<name>` clause".
- **Tables and modifiers** – Map-type modifiers, dependence types, and other
  structured lists include restrictions inline with the tables that define the
  allowed values.
- **Language-specific notes** – When the requirements differ between C/C++ and
  Fortran, the spec labels each bullet accordingly.  Keep both variants in mind
  when adding parser validation or documentation.

## Practical workflow for ROUP contributors

1. **Locate the canonical section** using the directive and clause catalogues in
   this repository.  Both
   [`openmp60-directives-clauses.md`](./openmp60-directives-clauses.md) and
   [`openmp60-directive-clause-components.md`](./openmp60-directive-clause-components.md)
   link directly to the parser keywords.
2. **Read the specification subsection** for the construct you are working on
   and note any "must", "shall", or "must not" statements.  These are the
   normative requirements that need to be respected by higher-level tooling.
3. **Capture parser limitations** in code comments or documentation if ROUP does
   not yet enforce a particular rule.  This keeps behaviour transparent for
   users of the library and the ompparser compatibility layer.
4. **Add tests where feasible**.  Many restrictions can be unit- or
   integration-tested (for example, rejecting a clause form that is not
   permitted).  When runtime enforcement is out of scope, reference the relevant
   specification section in the documentation so readers know where the gap is.

## Keeping restriction notes accurate

- **Do not duplicate specification prose** verbatim; link to the relevant
  section instead.  This avoids stale text and respects the licence of the
  official document.
- **Record deviations** whenever ROUP intentionally accepts syntax that the
  specification would forbid.  Document the reasoning in the relevant module or
  test to make review easier.
- **Use citations** (for example, `§7.5.4`) when summarising restrictions in
  release notes, tutorials, or design documents.  It gives downstream users a
  precise pointer back to the standard.

By following this workflow the repository remains aligned with the official
standard while still clearly communicating the parser's current behaviour.
