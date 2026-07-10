use roup::api::{OpenAccConfig, OpenAccParser};
use roup::ast::AccClauseKind;
use roup::version::{FortranStandard, HostLanguageProfile, SourceForm};

fn parser(source_form: SourceForm) -> OpenAccParser {
    OpenAccConfig::new(
        HostLanguageProfile::Fortran(FortranStandard::Fortran2023),
        source_form,
    )
    .expect("valid Fortran OpenACC configuration")
    .parser()
}

#[test]
fn standard_openacc_sentinels_continue_free_and_fixed_form_directives() {
    let cases = [
        (
            SourceForm::FortranFree,
            "!$acc parallel &\n!$acc& copy(a)",
            "copy",
        ),
        (
            SourceForm::FortranFixed,
            "      !$ACC PARALLEL &\n      C$ACC& COPY(A)",
            "COPY",
        ),
    ];

    for (source_form, source, clause_spelling) in cases {
        let parsed = parser(source_form)
            .parse(source)
            .unwrap_or_else(|error| panic!("rejected {source:?}: {error}"));
        let clause = &parsed.directive().clauses()[0];
        assert_eq!(clause.kind(), AccClauseKind::Copy, "{source}");
        assert_eq!(clause.span().slice(source), Ok(clause_spelling), "{source}");
    }
}

#[test]
fn continued_fortran_directives_reject_the_other_dialect_sentinel() {
    for (source_form, source) in [
        (SourceForm::FortranFree, "!$acc parallel &\n!$omp& copy(a)"),
        (
            SourceForm::FortranFixed,
            "      !$ACC PARALLEL &\n      C$OMP& COPY(A)",
        ),
    ] {
        assert!(
            parser(source_form).parse(source).is_err(),
            "accepted cross-dialect continuation sentinel in {source:?}"
        );
    }
}
