use roup::api::{OpenMpConfig, OpenMpParser};
use roup::ast::{OmpDirectiveKind, OmpReductionIdentifier};
use roup::ir::{ClauseData, ClauseItem, OrderKind, ScheduleKind};
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
        ClauseItem::LegacyTrailingSlash(identifier) => identifier.as_str(),
    }
}

#[test]
fn teams_limits_and_reduction_are_typed() {
    let parsed = parser()
        .parse("#pragma omp teams num_teams(8) thread_limit(32) reduction(+:total)")
        .expect("valid teams directive");
    let directive = parsed.directive();

    assert_eq!(directive.kind(), OmpDirectiveKind::Teams);
    let ClauseData::NumTeams { upper_bound, .. } = directive.clauses()[0].payload() else {
        panic!("expected num_teams payload");
    };
    assert_eq!(upper_bound.source(), "8");
    let ClauseData::ThreadLimit { limit } = directive.clauses()[1].payload() else {
        panic!("expected thread_limit payload");
    };
    assert_eq!(limit.source(), "32");
    let ClauseData::Reduction {
        operator, items, ..
    } = directive.clauses()[2].payload()
    else {
        panic!("expected reduction payload");
    };
    assert_eq!(operator, &OmpReductionIdentifier::Add);
    assert_eq!(items.iter().map(item_name).collect::<Vec<_>>(), ["total"]);
}

#[test]
fn teams_loop_and_distribute_modifiers_are_structured() {
    let loop_directive = parser()
        .parse("#pragma omp teams distribute parallel loop collapse(3) order(concurrent)")
        .expect("valid teams loop");
    assert_eq!(
        loop_directive.directive().kind(),
        OmpDirectiveKind::TeamsDistributeParallelLoop
    );
    let ClauseData::Collapse { n } = loop_directive.directive().clauses()[0].payload() else {
        panic!("expected collapse payload");
    };
    assert_eq!(n.source(), "3");
    assert!(matches!(
        loop_directive.directive().clauses()[1].payload(),
        ClauseData::Order {
            modifier: None,
            kind: OrderKind::Concurrent
        }
    ));

    let distribute = parser()
        .parse("#pragma omp teams distribute dist_schedule(static,4) collapse(2)")
        .expect("valid teams distribute");
    let ClauseData::DistSchedule { kind, chunk_size } =
        distribute.directive().clauses()[0].payload()
    else {
        panic!("expected dist_schedule payload");
    };
    assert_eq!(*kind, ScheduleKind::Static);
    assert_eq!(chunk_size.as_ref().map(|value| value.source()), Some("4"));
}
