use roup::api::{OpenMpConfig, OpenMpParser};
use roup::ast::{OmpClauseKind, OmpDirectiveKind};
use roup::ir::ClauseData;
use roup::version::{FortranStandard, HostLanguageProfile, SourceForm};

fn parser(form: SourceForm) -> OpenMpParser {
    OpenMpConfig::new(
        HostLanguageProfile::Fortran(FortranStandard::Fortran2023),
        form,
    )
    .unwrap()
    .parser()
}

#[test]
fn fixed_form_supports_standard_long_and_short_sentinels() {
    for sentinel in ["!$OMP", "!$", "C$OMP", "C$", "*$OMP", "*$"] {
        let source = format!(
            "      {sentinel} PARALLEL DO &\n      {sentinel}& PRIVATE(I) &\n      {sentinel}& SHARED(A)"
        );
        let parsed = parser(SourceForm::FortranFixed)
            .parse(&source)
            .unwrap_or_else(|error| panic!("sentinel {sentinel:?} failed: {error}"));

        assert_eq!(parsed.directive().kind(), OmpDirectiveKind::ParallelDo);
        assert_eq!(
            parsed
                .directive()
                .clauses()
                .iter()
                .map(|clause| clause.kind())
                .collect::<Vec<_>>(),
            [OmpClauseKind::Private, OmpClauseKind::Shared]
        );
    }
}

#[test]
fn fixed_form_sentinels_are_case_insensitive_and_may_mix() {
    let source = concat!(
        "      !$Omp TEAMS DISTRIBUTE &\n",
        "      c$oMP& PARALLEL DO &\n",
        "      *$OmP& PRIVATE(I)"
    );
    let parsed = parser(SourceForm::FortranFixed)
        .parse(source)
        .expect("mixed fixed-form sentinels should parse");

    assert_eq!(
        parsed.directive().kind(),
        OmpDirectiveKind::TeamsDistributeParallelDo
    );
    assert_eq!(
        parsed.directive().clauses()[0].kind(),
        OmpClauseKind::Private
    );
}

#[test]
fn free_form_supports_both_continuation_sentinel_forms() {
    for source in [
        concat!("!$omp parallel do &\n", "!$omp& private(i, j)"),
        concat!("!$omp parallel do &\n", "& private(i, j)"),
    ] {
        let parsed = parser(SourceForm::FortranFree)
            .parse(source)
            .expect("valid free-form continuation");
        assert_eq!(parsed.directive().kind(), OmpDirectiveKind::ParallelDo);
        let ClauseData::Private { items } = parsed.directive().clauses()[0].payload() else {
            panic!("private list must be typed");
        };
        assert_eq!(items.len(), 2);
    }
}

#[test]
fn free_form_comments_and_empty_continuation_lines_are_explicit() {
    let source = concat!(
        "!$omp parallel do private(i, & ! first continuation\n",
        "!$omp& & ! intentionally empty\n",
        "!$omp& j)"
    );
    let parsed = parser(SourceForm::FortranFree)
        .parse(source)
        .expect("valid commented continuation");

    let ClauseData::Private { items } = parsed.directive().clauses()[0].payload() else {
        panic!("private list must be typed");
    };
    assert_eq!(items.len(), 2);
}

#[test]
fn sentinel_or_continuation_mismatches_are_hard_errors() {
    let invalid = [
        (SourceForm::FortranFree, "C$OMP PARALLEL"),
        (SourceForm::FortranFree, "!$omp parallel do & private(i)"),
        (SourceForm::FortranFixed, "#pragma omp parallel"),
        (
            SourceForm::FortranFixed,
            "      !$OMP PARALLEL DO &\n      NOT_A_SENTINEL PRIVATE(I)",
        ),
    ];

    for (form, source) in invalid {
        assert!(parser(form).parse(source).is_err(), "{source:?} must fail");
    }
}
