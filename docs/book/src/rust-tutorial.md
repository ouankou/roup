# Rust tutorial

## Configure explicitly

Parser configuration fixes the directive dialect, specification policy, host
language standard, and physical source form.

```rust,ignore
use roup::api::OpenMpConfig;
use roup::version::{CStandard, HostLanguageProfile, OpenMpVersion, SourceForm};

let current = OpenMpConfig::exact(
    OpenMpVersion::V6_0,
    HostLanguageProfile::C(CStandard::C23),
    SourceForm::Pragma,
)?
.parser();

let source = "#pragma omp master";
let historical = current.parse(source)?;
assert!(historical
    .compatible_versions()
    .contains(OpenMpVersion::V6_0));
assert_eq!(historical.directive().span().slice(source), Ok("master"));
# Ok::<(), Box<dyn std::error::Error>>(())
```

Exact mode is cumulative: it rejects syntax introduced after the selected
version, but accepts older standardized syntax even if the selected
specification no longer documents that spelling.

## Inspect typed data

```rust,ignore
use roup::api::OpenAccConfig;
use roup::ast::{AccClausePayload, AccDirectiveKind};
use roup::version::{CStandard, HostLanguageProfile, SourceForm};

let parser = OpenAccConfig::new(
    HostLanguageProfile::C(CStandard::C23),
    SourceForm::Pragma,
)?
.parser();
let parsed = parser.parse("#pragma acc parallel async(queue)")?;
let directive = parsed.directive();

assert_eq!(directive.kind(), AccDirectiveKind::Parallel);
assert!(matches!(
    directive.clauses()[0].payload(),
    AccClausePayload::Expression(_)
));
# Ok::<(), Box<dyn std::error::Error>>(())
```

Expressions expose a typed host-language tree through `Expression::ast()`.
Canonical formatting walks that tree; source text is retained only as backing
for checked locations. Directive and clause `span()` values always refer to the
original physical directive, including across line continuations.

## Context supplied by a compiler

Plain `parse` performs syntax, version, and context-independent semantic
validation. When a compiler can answer declaration, association, or
constant-expression questions, use `parse_with_facts`. Applicable facts are
mandatory in that mode; an omitted fact is a hard `MissingSemanticFact` or
`MissingContext` diagnostic.

```rust,ignore
use roup::validation::{AssociationKind, SemanticFacts};

let facts = SemanticFacts::new()
    .with_association(AssociationKind::SectionRegion, true);
let parsed = current.parse_with_facts("#pragma omp section", &facts)?;
assert_eq!(parsed.directive().kind().as_str(), "section");
# Ok::<(), Box<dyn std::error::Error>>(())
```

For a sequence of paired regions, use `ContextValidator` with each directive's
checked source span. Mismatched or unclosed regions are errors and include the
related opening location.
