use roup::api::{OpenMpConfig, OpenMpParser};
use roup::ast::{OmpClauseKind, OmpDirectiveKind};
use roup::ir::{ClauseData, ClauseItem, ScheduleKind};
use roup::version::{CStandard, FortranStandard, HostLanguageProfile, SourceForm};

fn c_parser() -> OpenMpParser {
    OpenMpConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .unwrap()
        .parser()
}

fn fortran_parser(form: SourceForm) -> OpenMpParser {
    OpenMpConfig::new(
        HostLanguageProfile::Fortran(FortranStandard::Fortran2023),
        form,
    )
    .unwrap()
    .parser()
}

#[test]
fn c_backslash_newline_splices_without_losing_typed_clauses() {
    let source = concat!(
        "#pragma omp parallel for \\\n",
        "    schedule(dynamic, 4) \\\n",
        "    private(i, \\\n",
        "            j)"
    );
    let parsed = c_parser().parse(source).expect("valid C line splices");
    let directive = parsed.directive();

    assert_eq!(directive.kind(), OmpDirectiveKind::ParallelFor);
    assert_eq!(directive.clauses()[0].kind(), OmpClauseKind::Schedule);
    let ClauseData::Schedule {
        kind, chunk_size, ..
    } = directive.clauses()[0].payload()
    else {
        panic!("expected typed schedule");
    };
    assert_eq!(*kind, ScheduleKind::Dynamic);
    assert_eq!(chunk_size.as_ref().map(|value| value.source()), Some("4"));

    let ClauseData::Private { items } = directive.clauses()[1].payload() else {
        panic!("expected typed private list");
    };
    assert!(matches!(
        items.as_slice(),
        [ClauseItem::Identifier(first), ClauseItem::Identifier(second)]
            if first.as_str() == "i" && second.as_str() == "j"
    ));
}

#[test]
fn c_splice_never_invents_a_token_separator() {
    assert!(
        c_parser()
            .parse(concat!("#pragma omp parallel\\\n", "for"))
            .is_err(),
        "parallel\\newlinefor is the single invalid token parallelfor"
    );

    let parsed = c_parser()
        .parse(concat!("#pragma omp parallel \\\n", " for"))
        .expect("actual source whitespace separates the tokens");
    assert_eq!(parsed.directive().kind(), OmpDirectiveKind::ParallelFor);
}

#[test]
fn malformed_c_continuations_are_hard_errors() {
    for source in [
        "#pragma omp parallel \\  \nfor",
        "#pragma omp parallel \\ \nfor",
        "#pragma omp parallel\nfor",
        "#pragma omp parallel \\\rfor",
    ] {
        assert!(c_parser().parse(source).is_err(), "{source:?} must fail");
    }
}

#[test]
fn fortran_free_continuation_preserves_only_source_whitespace() {
    let source = concat!(
        "!$omp parallel do &\n",
        "!$omp& private(i, &\n",
        "!$omp& j)"
    );
    let parsed = fortran_parser(SourceForm::FortranFree)
        .parse(source)
        .expect("valid free-form continuation");

    assert_eq!(parsed.directive().kind(), OmpDirectiveKind::ParallelDo);
    let ClauseData::Private { items } = parsed.directive().clauses()[0].payload() else {
        panic!("expected private payload");
    };
    assert_eq!(items.len(), 2);
}

#[test]
fn fortran_fixed_continuations_can_mix_standard_sentinels() {
    let source = concat!(
        "      !$OMP TEAMS DISTRIBUTE &\n",
        "      C$OMP& PARALLEL DO &\n",
        "      !$OMP& PRIVATE(I) SHARED(A)"
    );
    let parsed = fortran_parser(SourceForm::FortranFixed)
        .parse(source)
        .expect("valid fixed-form continuation");

    assert_eq!(
        parsed.directive().kind(),
        OmpDirectiveKind::TeamsDistributeParallelDo
    );
    assert_eq!(parsed.directive().clauses().len(), 2);
}

#[test]
fn malformed_fortran_continuations_are_hard_errors() {
    for source in [
        "!$omp parallel do & private(i)",
        "!$omp parallel do\n!$omp private(i)",
    ] {
        assert!(
            fortran_parser(SourceForm::FortranFree)
                .parse(source)
                .is_err(),
            "{source:?} must fail"
        );
    }
}
