use roup::api::{OpenMpConfig, OpenMpParser};
use roup::ast::{OmpClauseKind, OmpDirectiveKind, OmpReductionIdentifier};
use roup::ir::{ClauseData, ClauseItem, ProcBind, ScheduleKind};
use roup::version::{CStandard, HostLanguageProfile, OpenMpVersion, SourceForm};

fn parser() -> OpenMpParser {
    OpenMpConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .expect("valid C OpenMP configuration")
        .parser()
}

fn item_name(item: &ClauseItem) -> &str {
    match item {
        ClauseItem::Identifier(identifier) => identifier.as_str(),
        ClauseItem::Variable(variable) => variable.expression().source(),
        ClauseItem::FortranCommonBlock(name) => name.as_str(),
        ClauseItem::Expression(expression) => expression.source(),
    }
}

#[test]
fn parallel_clauses_are_typed_at_the_public_boundary() {
    let parsed = parser()
        .parse("#pragma omp parallel private(a, b) firstprivate(c) num_threads(4) proc_bind(close)")
        .expect("standard parallel clauses should parse");
    let directive = parsed.directive();

    assert_eq!(directive.kind(), OmpDirectiveKind::Parallel);
    assert_eq!(
        directive
            .clauses()
            .iter()
            .map(|clause| clause.kind())
            .collect::<Vec<_>>(),
        [
            OmpClauseKind::Private,
            OmpClauseKind::Firstprivate,
            OmpClauseKind::NumThreads,
            OmpClauseKind::ProcBind,
        ]
    );

    let ClauseData::Private { items } = directive.clauses()[0].payload() else {
        panic!("private must have a typed private payload");
    };
    assert_eq!(items.iter().map(item_name).collect::<Vec<_>>(), ["a", "b"]);

    let ClauseData::Firstprivate {
        modifier: None,
        items,
    } = directive.clauses()[1].payload()
    else {
        panic!("firstprivate must have a typed firstprivate payload");
    };
    assert_eq!(items.iter().map(item_name).collect::<Vec<_>>(), ["c"]);

    let ClauseData::NumThreads { nthreads, .. } = directive.clauses()[2].payload() else {
        panic!("num_threads must retain an expression AST");
    };
    assert_eq!(nthreads[0].source(), "4");
    assert_eq!(
        directive.clauses()[3].payload(),
        &ClauseData::ProcBind(ProcBind::Close)
    );
}

#[test]
fn combined_parallel_loop_preserves_typed_modifier_payloads() {
    let parsed = parser()
        .parse(
            "#pragma omp parallel for simd aligned(buf:64) schedule(static,4) collapse(2) reduction(+:sum)",
        )
        .expect("standard combined construct should parse");
    let directive = parsed.directive();

    assert_eq!(directive.kind(), OmpDirectiveKind::ParallelForSimd);
    let ClauseData::Schedule {
        kind,
        modifiers,
        chunk_size,
    } = directive.clauses()[1].payload()
    else {
        panic!("schedule must be typed");
    };
    assert_eq!(*kind, ScheduleKind::Static);
    assert!(modifiers.is_empty());
    assert_eq!(chunk_size.as_ref().map(|value| value.source()), Some("4"));

    let ClauseData::Reduction {
        modifiers,
        operator,
        items,
    } = directive.clauses()[3].payload()
    else {
        panic!("reduction must be typed");
    };
    assert!(modifiers.is_empty());
    assert_eq!(operator, &OmpReductionIdentifier::Add);
    assert_eq!(items.iter().map(item_name).collect::<Vec<_>>(), ["sum"]);
}

#[test]
fn historical_proc_bind_master_is_accepted_and_canonicalized_in_six_zero() {
    let parser = OpenMpConfig::exact(
        OpenMpVersion::V6_0,
        HostLanguageProfile::C(CStandard::C23),
        SourceForm::Pragma,
    )
    .unwrap()
    .parser();
    let parsed = parser
        .parse("#pragma omp parallel proc_bind(master)")
        .expect("historical standardized syntax remains accepted");

    assert_eq!(
        parsed.directive().clauses()[0].payload(),
        &ClauseData::ProcBind(ProcBind::Primary)
    );
}

#[test]
fn invalid_parallel_clauses_are_hard_errors() {
    for source in [
        "#pragma omp parallel unsupported_clause",
        "#pragma omp parallel nowait",
        "#pragma omp parallel private()",
    ] {
        assert!(parser().parse(source).is_err(), "{source} must be rejected");
    }
}
