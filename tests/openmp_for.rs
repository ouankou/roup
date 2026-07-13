use roup::api::{OpenMpConfig, OpenMpParser};
use roup::ast::{OmpClauseKind, OmpDirectiveKind, OmpReductionIdentifier};
use roup::ir::{ClauseData, ClauseItem, ScheduleKind};
use roup::version::{CStandard, HostLanguageProfile, SourceForm};

fn parser() -> OpenMpParser {
    OpenMpConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .unwrap()
        .parser()
}

fn item_name(item: &ClauseItem) -> &str {
    match item {
        ClauseItem::Identifier(identifier) => identifier.as_str(),
        ClauseItem::Variable(variable) => variable.expression().source(),
        ClauseItem::FortranCommonBlock(name) => name.as_str(),
        ClauseItem::Expression(expression) => expression.source(),
        ClauseItem::OmpparserTrailingSlash(identifier) => identifier.as_str(),
    }
}

#[test]
fn for_iteration_clauses_have_semantic_payloads() {
    let parsed = parser()
        .parse("#pragma omp for schedule(guided,16) ordered(2) private(i, j)")
        .expect("valid for directive");
    let directive = parsed.directive();

    assert_eq!(directive.kind(), OmpDirectiveKind::For);
    assert_eq!(
        directive
            .clauses()
            .iter()
            .map(|clause| clause.kind())
            .collect::<Vec<_>>(),
        [
            OmpClauseKind::Schedule,
            OmpClauseKind::Ordered,
            OmpClauseKind::Private,
        ]
    );

    let ClauseData::Schedule {
        kind,
        modifiers,
        chunk_size,
    } = directive.clauses()[0].payload()
    else {
        panic!("expected schedule payload");
    };
    assert_eq!(*kind, ScheduleKind::Guided);
    assert!(modifiers.is_empty());
    assert_eq!(chunk_size.as_ref().map(|value| value.source()), Some("16"));

    let ClauseData::Ordered { n } = directive.clauses()[1].payload() else {
        panic!("expected ordered payload");
    };
    assert_eq!(n.as_ref().map(|value| value.source()), Some("2"));

    let ClauseData::Private { items } = directive.clauses()[2].payload() else {
        panic!("expected private payload");
    };
    assert_eq!(items.iter().map(item_name).collect::<Vec<_>>(), ["i", "j"]);
}

#[test]
fn for_simd_vector_and_reduction_clauses_are_typed() {
    let parsed = parser()
        .parse("#pragma omp for simd linear(x:2) safelen(8) simdlen(4) reduction(-:diff)")
        .expect("valid for simd directive");
    let directive = parsed.directive();

    assert_eq!(directive.kind(), OmpDirectiveKind::ForSimd);
    let ClauseData::Linear {
        modifier,
        items,
        step,
        ..
    } = directive.clauses()[0].payload()
    else {
        panic!("expected linear payload");
    };
    assert!(modifier.is_none());
    assert_eq!(items.iter().map(item_name).collect::<Vec<_>>(), ["x"]);
    assert_eq!(step.as_ref().map(|value| value.source()), Some("2"));

    let ClauseData::Safelen { length } = directive.clauses()[1].payload() else {
        panic!("expected safelen payload");
    };
    assert_eq!(length.source(), "8");
    let ClauseData::Simdlen { length } = directive.clauses()[2].payload() else {
        panic!("expected simdlen payload");
    };
    assert_eq!(length.source(), "4");

    let ClauseData::Reduction {
        operator, items, ..
    } = directive.clauses()[3].payload()
    else {
        panic!("expected reduction payload");
    };
    assert_eq!(operator, &OmpReductionIdentifier::Subtract);
    assert_eq!(items.iter().map(item_name).collect::<Vec<_>>(), ["diff"]);
}

#[test]
fn bare_ordered_and_nowait_are_distinct_typed_payloads() {
    let parsed = parser()
        .parse("#pragma omp for ordered nowait")
        .expect("valid bare clauses");
    let clauses = parsed.directive().clauses();

    assert!(matches!(
        clauses[0].payload(),
        ClauseData::Ordered { n: None }
    ));
    assert_eq!(
        clauses[1].payload(),
        &ClauseData::Nowait {
            do_not_synchronize: None,
        }
    );
}
