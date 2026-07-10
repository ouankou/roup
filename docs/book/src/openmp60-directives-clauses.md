# OpenMP syntax catalogue and version policy

ROUP models standardized OpenMP syntax from version 1.0 through 6.0. The
public `OmpDirectiveKind` and `OmpClauseKind` enums are the canonical semantic
catalogue; directive parameters and clause payload enums describe the data
attached to each kind.

For normative syntax and meaning, consult the
[OpenMP 6.0 specification](https://www.openmp.org/wp-content/uploads/OpenMP-API-Specification-6-0.pdf)
and the earlier specification that introduced a historical form.

## Supported specification policies

`OpenMpConfig::new` uses `VersionPolicy::Any`, the union of standardized syntax
from every supported OpenMP version. `OpenMpConfig::exact` selects one of:

`1.0`, `1.1`, `2.0`, `2.5`, `3.0`, `3.1`, `4.0`, `4.5`, `5.0`, `5.1`, `5.2`,
or `6.0`.

An exact version is an introduction ceiling. A feature first standardized
after the selected version is rejected. A feature standardized at or before
the selected version remains accepted, even if the later specification
deprecated, renamed, or removed it. For example, OpenMP 6.0 mode still accepts
the historical `master` directive.

This cumulative policy is intentionally different from asking whether a
spelling appears in the selected specification document. It lets tools parse
maintained historical code without weakening validation of unknown or
nonstandard syntax.

## Canonical aliases

Specification-defined aliases are accepted at the syntax boundary and mapped
to one semantic representation. Canonicalization never erases the checked
source location: directive and clause spans still select the exact spelling in
the original physical source.

Alias provenance is included in availability computation. An alias therefore
cannot make a feature appear in a specification older than the one that first
standardized that spelling.

## What a catalogue entry guarantees

A public kind is not merely a recognized keyword. A successful public parse
also guarantees:

- a complete typed directive parameter and clause payload;
- a valid host-language expression, type name, identifier, or locator tree;
- availability under the selected OpenMP and host-language versions;
- context-independent directive, clause, duplicate, and nesting checks; and
- exact checked spans for directive and clause names.

Unknown keywords, implementation spellings without a standards entry,
malformed payloads, and trailing input are hard errors. There is no public raw
grammar result and no render-and-reparse path.

The public catalogue and introduction data are regression-tested by
`tests/feature_availability.rs`, while payload shape and rejection behavior are
covered by the strict payload and error suites.
