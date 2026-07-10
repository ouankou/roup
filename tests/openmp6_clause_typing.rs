use roup::api::{OpenMpConfig, ParsedOpenMpDirective};
use roup::ast::{OmpDirectiveKind, OmpMapperId};
use roup::diagnostic::{Diagnostic, DiagnosticCode};
use roup::ir::{ClauseData, FirstprivateModifier, MemscopeKind, ThreadsetKind};
use roup::version::{CStandard, FortranStandard, HostLanguageProfile, OpenMpVersion, SourceForm};

fn c_exact(version: OpenMpVersion, source: &str) -> Result<ParsedOpenMpDirective, Diagnostic> {
    OpenMpConfig::exact(
        version,
        HostLanguageProfile::C(CStandard::C23),
        SourceForm::Pragma,
    )
    .unwrap()
    .parser()
    .parse(source)
}

fn c(source: &str) -> Result<ParsedOpenMpDirective, Diagnostic> {
    OpenMpConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .unwrap()
        .parser()
        .parse(source)
}

fn fortran(source: &str) -> Result<ParsedOpenMpDirective, Diagnostic> {
    OpenMpConfig::new(
        HostLanguageProfile::Fortran(FortranStandard::Fortran2023),
        SourceForm::FortranFree,
    )
    .unwrap()
    .parser()
    .parse(source)
}

#[test]
fn universal_directive_name_modifier_is_stored_once_on_every_clause() {
    let cases = [
        (
            "#pragma omp parallel loop private(parallel: x)",
            OmpDirectiveKind::Parallel,
        ),
        (
            "#pragma omp parallel for if(parallel for: condition)",
            OmpDirectiveKind::ParallelFor,
        ),
        (
            "#pragma omp task threadset(task: omp_pool)",
            OmpDirectiveKind::Task,
        ),
        (
            "#pragma omp atomic read memscope(atomic: device)",
            OmpDirectiveKind::Atomic,
        ),
        (
            "#pragma omp parallel reduction(parallel: +: sum)",
            OmpDirectiveKind::Parallel,
        ),
    ];

    for (source, expected) in cases {
        let parsed = c_exact(OpenMpVersion::V6_0, source)
            .unwrap_or_else(|error| panic!("failed to parse {source:?}: {error}"));
        let clause = parsed
            .directive()
            .clauses()
            .iter()
            .find(|clause| clause.directive_name_modifier().is_some())
            .expect("modified clause must be present");
        assert_eq!(clause.directive_name_modifier(), Some(expected), "{source}");
    }
}

#[test]
fn historical_if_modifier_floor_is_preserved_but_other_universal_use_starts_in_6_0() {
    let historical_if = "#pragma omp parallel for if(parallel: condition)";
    let non_if = "#pragma omp parallel loop private(parallel: x)";

    let before_if = c_exact(OpenMpVersion::V4_0, historical_if).unwrap_err();
    assert_eq!(before_if.code(), DiagnosticCode::NotAvailableInVersion);
    c_exact(OpenMpVersion::V4_5, historical_if).unwrap();

    c_exact(OpenMpVersion::V5_2, non_if).unwrap_err();
    c_exact(OpenMpVersion::V6_0, non_if).unwrap();
}

#[test]
fn modifier_must_name_an_eligible_constituent_and_if_overlap_is_rejected() {
    c("#pragma omp target parallel private(parallel: x)").unwrap();
    c("#pragma omp target parallel if(target: on_device) if(parallel: in_team)").unwrap();

    for source in [
        "#pragma omp target parallel private(target: x)",
        "#pragma omp parallel private(target: x)",
        "#pragma omp target parallel if(enabled) if(target: on_device)",
        "#pragma omp target parallel if(target: first) if(target: second)",
        "#pragma omp parallel if(parallel: first) if(second)",
    ] {
        assert!(c(source).is_err(), "unexpectedly accepted {source:?}");
    }
}

#[test]
fn firstprivate_modifier_and_saved_semantics_are_canonicalized() {
    for source in [
        "#pragma omp target firstprivate(target, saved: x)",
        "#pragma omp target firstprivate(saved, target: x)",
    ] {
        let parsed = c_exact(OpenMpVersion::V6_0, source).unwrap();
        let clause = &parsed.directive().clauses()[0];
        assert_eq!(
            clause.directive_name_modifier(),
            Some(OmpDirectiveKind::Target)
        );
        assert!(matches!(
            clause.payload(),
            ClauseData::Firstprivate {
                modifier: Some(FirstprivateModifier::Saved),
                items,
            } if items.len() == 1
        ));
    }

    for source in [
        "#pragma omp target firstprivate(target, target: x)",
        "#pragma omp target firstprivate(saved, saved: x)",
        "#pragma omp target firstprivate(target: saved: x)",
        "#pragma omp target firstprivate(target: parallel: x)",
    ] {
        assert!(c(source).is_err(), "unexpectedly accepted {source:?}");
    }
}

#[test]
fn new_openmp_6_clause_payloads_have_exact_typed_shapes() {
    let threadset = c("#pragma omp task threadset(omp_team)").unwrap();
    assert_eq!(
        threadset.directive().clauses()[0].payload(),
        &ClauseData::Threadset(ThreadsetKind::OmpTeam)
    );

    let memscope = c("#pragma omp atomic read memscope(cgroup)").unwrap();
    assert_eq!(
        memscope
            .directive()
            .clauses()
            .iter()
            .find(|clause| matches!(clause.payload(), ClauseData::Memscope(_)))
            .unwrap()
            .payload(),
        &ClauseData::Memscope(MemscopeKind::Cgroup)
    );

    let looprange = c("#pragma omp fuse looprange(2, number_of_loops)").unwrap();
    assert!(matches!(
        looprange.directive().clauses()[0].payload(),
        ClauseData::Looprange { first, count }
            if first.source() == "2" && count.source() == "number_of_loops"
    ));

    let bare = c("#pragma omp taskgraph graph_reset").unwrap();
    assert_eq!(
        bare.directive().clauses()[0].payload(),
        &ClauseData::GraphReset { condition: None }
    );
    let conditional = c("#pragma omp taskgraph graph_reset(should_reset)").unwrap();
    assert!(matches!(
        conditional.directive().clauses()[0].payload(),
        ClauseData::GraphReset {
            condition: Some(condition)
        } if condition.source() == "should_reset"
    ));
}

#[test]
fn malformed_or_misplaced_openmp_6_clause_shapes_are_hard_errors() {
    for source in [
        "#pragma omp task threadset()",
        "#pragma omp task threadset(pool)",
        "#pragma omp task threadset(omp_pool, omp_team)",
        "#pragma omp atomic read memscope()",
        "#pragma omp atomic read memscope(team)",
        "#pragma omp fuse looprange(1)",
        "#pragma omp fuse looprange(1, 2, 3)",
        "#pragma omp reverse looprange(1, 2)",
        "#pragma omp taskgraph graph_reset()",
    ] {
        assert!(c(source).is_err(), "unexpectedly accepted {source:?}");
    }
}

#[test]
fn map_mapper_identifier_preserves_default_and_user_variants() {
    let default = c("#pragma omp target map(mapper(default), to: x)").unwrap();
    assert!(matches!(
        default.directive().clauses()[0].payload(),
        ClauseData::Map {
            mapper: Some(OmpMapperId::Default),
            ..
        }
    ));

    let custom = c("#pragma omp target map(mapper(custom), to: x)").unwrap();
    assert!(matches!(
        custom.directive().clauses()[0].payload(),
        ClauseData::Map {
            mapper: Some(OmpMapperId::User(name)),
            ..
        } if name.as_str() == "custom"
    ));

    let fortran_default = fortran("!$omp target map(mapper(DEFAULT), to: x)").unwrap();
    assert!(matches!(
        fortran_default.directive().clauses()[0].payload(),
        ClauseData::Map {
            mapper: Some(OmpMapperId::Default),
            ..
        }
    ));

    for source in [
        "#pragma omp target map(mapper(), to: x)",
        "#pragma omp target map(mapper(default, custom), to: x)",
        "#pragma omp target map(mapper(default), mapper(custom), to: x)",
    ] {
        assert!(c(source).is_err(), "unexpectedly accepted {source:?}");
    }
}
