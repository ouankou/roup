use roup::api::{OpenMpConfig, OpenMpParser};
use roup::ast::{
    OmpClauseKind, OmpConstructType, OmpDirectiveKind, OmpDirectiveParameter,
    OmpReductionIdentifier,
};
use roup::ir::{ClauseData, ClauseItem, DependType, MapType, OmpDependence, ScheduleKind};
use roup::version::{FortranStandard, HostLanguageProfile, OpenMpVersion, SourceForm};

fn parser(form: SourceForm) -> OpenMpParser {
    OpenMpConfig::new(
        HostLanguageProfile::Fortran(FortranStandard::Fortran2023),
        form,
    )
    .unwrap()
    .parser()
}

fn exact_parser(version: OpenMpVersion, form: SourceForm) -> OpenMpParser {
    OpenMpConfig::exact(
        version,
        HostLanguageProfile::Fortran(FortranStandard::Fortran2023),
        form,
    )
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
fn free_and_fixed_forms_use_the_same_typed_ast() {
    let free = parser(SourceForm::FortranFree)
        .parse("!$OMP PARALLEL PRIVATE(A, B) NUM_THREADS(4)")
        .expect("valid free-form directive");
    let fixed = parser(SourceForm::FortranFixed)
        .parse("C$OMP PARALLEL PRIVATE(A, B) NUM_THREADS(4)")
        .expect("valid fixed-form directive");

    assert_eq!(free.directive(), fixed.directive());
    assert_eq!(free.directive().kind(), OmpDirectiveKind::Parallel);
    let ClauseData::Private { items } = free.directive().clauses()[0].payload() else {
        panic!("private must have a typed payload");
    };
    assert_eq!(items.iter().map(item_name).collect::<Vec<_>>(), ["a", "b"]);
    let ClauseData::NumThreads { nthreads, .. } = free.directive().clauses()[1].payload() else {
        panic!("num_threads must retain its expression");
    };
    assert_eq!(nthreads[0].source(), "4");
}

#[test]
fn fortran_directive_and_clause_keywords_are_case_insensitive() {
    let variants = [
        "!$omp parallel do private(i)",
        "!$OMP PARALLEL DO PRIVATE(I)",
        "!$OmP pArAlLeL dO pRiVaTe(I)",
    ];

    for source in variants {
        let parsed = parser(SourceForm::FortranFree)
            .parse(source)
            .unwrap_or_else(|error| panic!("{source:?} failed: {error}"));
        assert_eq!(parsed.directive().kind(), OmpDirectiveKind::ParallelDo);
        assert_eq!(
            parsed.directive().clauses()[0].kind(),
            OmpClauseKind::Private
        );
    }
}

#[test]
fn fortran_specific_constructs_and_historical_forms_remain_available() {
    let specimens = [
        ("!$OMP DO SCHEDULE(STATIC)", OmpDirectiveKind::Do),
        ("!$OMP PARALLEL DO PRIVATE(I)", OmpDirectiveKind::ParallelDo),
        ("!$OMP WORKSHARE", OmpDirectiveKind::Workshare),
        (
            "!$OMP PARALLEL WORKSHARE",
            OmpDirectiveKind::ParallelWorkshare,
        ),
        ("!$OMP MASTER", OmpDirectiveKind::Master),
        ("!$OMP END MASTER", OmpDirectiveKind::EndMaster),
    ];

    for (source, expected) in specimens {
        let parsed = exact_parser(OpenMpVersion::V6_0, SourceForm::FortranFree)
            .parse(source)
            .unwrap_or_else(|error| panic!("{source:?} failed: {error}"));
        assert_eq!(parsed.directive().kind(), expected);
    }
}

#[test]
fn compact_fortran_aliases_are_canonicalized() {
    for source in ["!$OMP PARALLELDO PRIVATE(I)", "!$OMP ENDDO"] {
        let parsed = parser(SourceForm::FortranFree)
            .parse(source)
            .unwrap_or_else(|error| panic!("compact alias {source:?} failed: {error}"));
        let expected = if source.contains("PARALLELDO") {
            OmpDirectiveKind::ParallelDo
        } else {
            OmpDirectiveKind::EndDo
        };
        assert_eq!(parsed.directive().kind(), expected);
    }
}

#[test]
fn fortran_clause_payloads_are_structured() {
    let reduction = parser(SourceForm::FortranFree)
        .parse("!$OMP PARALLEL DO REDUCTION(.AND.: FLAGS) SCHEDULE(DYNAMIC, 4)")
        .expect("valid Fortran reduction and schedule");
    let ClauseData::Reduction {
        operator, items, ..
    } = reduction.directive().clauses()[0].payload()
    else {
        panic!("expected reduction payload");
    };
    assert_eq!(operator, &OmpReductionIdentifier::FortranLogicalAnd);
    assert_eq!(items.iter().map(item_name).collect::<Vec<_>>(), ["flags"]);
    let ClauseData::Schedule {
        kind, chunk_size, ..
    } = reduction.directive().clauses()[1].payload()
    else {
        panic!("expected schedule payload");
    };
    assert_eq!(*kind, ScheduleKind::Dynamic);
    assert_eq!(chunk_size.as_ref().map(|value| value.source()), Some("4"));

    let target = parser(SourceForm::FortranFree)
        .parse("!$OMP TARGET MAP(TO: A(1:N)) MAP(FROM: B(:, 1:M))")
        .expect("valid Fortran array sections");
    for (clause, expected_type) in target
        .directive()
        .clauses()
        .iter()
        .zip([MapType::To, MapType::From])
    {
        let ClauseData::Map {
            map_type, locators, ..
        } = clause.payload()
        else {
            panic!("expected map payload");
        };
        assert_eq!(*map_type, Some(expected_type));
        assert!(matches!(
            locators.as_slice(),
            [roup::ir::OmpLocator::PotentialLValue(_)]
        ));
    }

    let task = parser(SourceForm::FortranFree)
        .parse("!$OMP TASK DEPEND(IN: A)")
        .expect("valid task dependence");
    assert!(matches!(
        task.directive().clauses()[0].payload(),
        ClauseData::Depend {
            dependence: OmpDependence::Locators {
                kind: DependType::In,
                ..
            },
            ..
        }
    ));
}

#[test]
fn cancellation_construct_parameters_are_typed() {
    for (source, expected_kind, expected_construct) in [
        (
            "!$OMP CANCEL DO IF(COND)",
            OmpDirectiveKind::Cancel,
            OmpConstructType::For,
        ),
        (
            "!$OMP CANCELLATION POINT PARALLEL",
            OmpDirectiveKind::CancellationPoint,
            OmpConstructType::Parallel,
        ),
    ] {
        let parsed = parser(SourceForm::FortranFree)
            .parse(source)
            .unwrap_or_else(|error| panic!("{source:?} failed: {error}"));
        assert_eq!(parsed.directive().kind(), expected_kind);
        assert!(matches!(
            parsed.directive().parameter(),
            Some(OmpDirectiveParameter::Construct(construct)) if *construct == expected_construct
        ));
    }
}

#[test]
fn malformed_fortran_payloads_are_hard_errors() {
    for source in [
        "!$OMP PARALLEL PRIVATE()",
        "!$OMP DO SCHEDULE(BOGUS)",
        "!$OMP TARGET MAP(TO: A(1:@))",
        "!$OMP TASK DEPEND(IN:)",
        "!$OMP CANCEL BOGUS",
    ] {
        assert!(
            parser(SourceForm::FortranFree).parse(source).is_err(),
            "{source} must be rejected"
        );
    }
}
