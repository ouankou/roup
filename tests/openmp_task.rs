use roup::api::{OpenMpConfig, OpenMpParser};
use roup::ast::{OmpClauseKind, OmpDirectiveKind};
use roup::ir::{ClauseData, DependType, OmpDependence};
use roup::version::{CStandard, HostLanguageProfile, SourceForm};

fn parser() -> OpenMpParser {
    OpenMpConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .unwrap()
        .parser()
}

#[test]
fn task_dependencies_are_not_left_as_raw_text() {
    let parsed = parser()
        .parse(
            "#pragma omp task if(inbranch) final(ready) priority(3) depend(inout:buf) detach(evt)",
        )
        .expect("valid task directive");
    let directive = parsed.directive();

    assert_eq!(directive.kind(), OmpDirectiveKind::Task);
    assert_eq!(
        directive
            .clauses()
            .iter()
            .map(|clause| clause.kind())
            .collect::<Vec<_>>(),
        [
            OmpClauseKind::If,
            OmpClauseKind::Final,
            OmpClauseKind::Priority,
            OmpClauseKind::Depend,
            OmpClauseKind::Detach,
        ]
    );

    let ClauseData::If { condition } = directive.clauses()[0].payload() else {
        panic!("if must retain a typed expression");
    };
    assert_eq!(condition.source(), "inbranch");

    let ClauseData::Depend {
        dependence,
        iterators,
    } = directive.clauses()[3].payload()
    else {
        panic!("depend must retain typed dependence data");
    };
    assert!(iterators.is_empty());
    let OmpDependence::Locators { kind, locators } = dependence else {
        panic!("inout dependence must retain locators");
    };
    assert_eq!(*kind, DependType::Inout);
    assert_eq!(locators.len(), 1);
}

#[test]
fn taskloop_granularity_and_reduction_are_typed() {
    let parsed = parser()
        .parse("#pragma omp taskloop simd grainsize(4) reduction(max:max_val) shared(out)")
        .expect("valid taskloop simd directive");
    let directive = parsed.directive();

    assert_eq!(directive.kind(), OmpDirectiveKind::TaskloopSimd);
    let ClauseData::Grainsize { modifier, grain } = directive.clauses()[0].payload() else {
        panic!("expected grainsize payload");
    };
    assert!(modifier.is_none());
    assert_eq!(grain.source(), "4");

    let ClauseData::Reduction {
        operator, items, ..
    } = directive.clauses()[1].payload()
    else {
        panic!("expected reduction payload");
    };
    assert_eq!(operator.to_string(), "max");
    assert_eq!(
        items.iter().map(ToString::to_string).collect::<Vec<_>>(),
        ["max_val"]
    );

    let num_tasks = parser()
        .parse("#pragma omp taskloop simd num_tasks(16)")
        .expect("num_tasks is valid without grainsize");
    let ClauseData::NumTasks { modifier, num } = num_tasks.directive().clauses()[0].payload()
    else {
        panic!("expected num_tasks payload");
    };
    assert!(modifier.is_none());
    assert_eq!(num.source(), "16");
}

#[test]
fn malformed_task_payloads_fail_instead_of_becoming_strings() {
    for source in [
        "#pragma omp task depend(inout:)",
        "#pragma omp task priority()",
        "#pragma omp task detach(evt, other)",
    ] {
        assert!(parser().parse(source).is_err(), "{source} must be rejected");
    }
}
