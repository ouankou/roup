use roup::api::{OpenAccConfig, OpenMpConfig, ParsedOpenAccDirective, ParsedOpenMpDirective};
use roup::ast::{AccClauseKind, AccClausePayload, OmpDirectiveKind};
use roup::diagnostic::{Diagnostic, DiagnosticCode};
use roup::source::Span;
use roup::validation::ContextValidator;
use roup::version::{
    CStandard, FortranStandard, HostLanguageProfile, OpenAccVersion, OpenMpVersion, SourceForm,
};

fn omp_fortran(version: OpenMpVersion, source: &str) -> Result<ParsedOpenMpDirective, Diagnostic> {
    OpenMpConfig::exact(
        version,
        HostLanguageProfile::Fortran(FortranStandard::Fortran2023),
        SourceForm::FortranFree,
    )
    .expect("valid exact Fortran OpenMP configuration")
    .parser()
    .parse(source)
}

fn omp_c(version: OpenMpVersion, source: &str) -> Result<ParsedOpenMpDirective, Diagnostic> {
    OpenMpConfig::exact(
        version,
        HostLanguageProfile::C(CStandard::C23),
        SourceForm::Pragma,
    )
    .expect("valid exact C OpenMP configuration")
    .parser()
    .parse(source)
}

fn acc_c(version: OpenAccVersion, source: &str) -> Result<ParsedOpenAccDirective, Diagnostic> {
    OpenAccConfig::exact(
        version,
        HostLanguageProfile::C(CStandard::C23),
        SourceForm::Pragma,
    )
    .expect("valid exact C OpenACC configuration")
    .parser()
    .parse(source)
}

#[test]
fn fortran_end_allocators_and_end_dispatch_are_cumulative_from_openmp_52() {
    for (source, expected) in [
        ("!$omp end allocators", OmpDirectiveKind::EndAllocators),
        ("!$omp endallocators", OmpDirectiveKind::EndAllocators),
        ("!$omp end dispatch", OmpDirectiveKind::EndDispatch),
        ("!$omp enddispatch", OmpDirectiveKind::EndDispatch),
    ] {
        assert!(
            omp_fortran(OpenMpVersion::V5_1, source).is_err(),
            "OpenMP 5.1 accepted syntax introduced in 5.2: {source:?}"
        );
        for version in [OpenMpVersion::V5_2, OpenMpVersion::V6_0] {
            let parsed = omp_fortran(version, source)
                .unwrap_or_else(|error| panic!("OpenMP {version} rejected {source:?}: {error}"));
            assert_eq!(parsed.directive().kind(), expected);
        }
    }
}

#[test]
fn fortran_only_endings_and_malformed_forms_are_hard_errors() {
    for source in [
        "#pragma omp end allocators",
        "#pragma omp endallocators",
        "#pragma omp end dispatch",
        "#pragma omp enddispatch",
    ] {
        let error = omp_c(OpenMpVersion::V6_0, source)
            .expect_err("Fortran-only paired end directive was accepted in C");
        assert_eq!(error.code(), DiagnosticCode::NotAvailableInVersion);
    }

    for source in [
        "!$omp end allocators()",
        "!$omp end allocators allocate(value)",
        "!$omp end dispatch()",
        "!$omp end dispatch nowait",
        "!$omp enddispatch trailing",
    ] {
        assert!(
            omp_fortran(OpenMpVersion::V6_0, source).is_err(),
            "unexpectedly accepted malformed paired ending {source:?}"
        );
    }
}

#[test]
fn context_validator_pairs_the_new_end_directives_exactly() {
    let source = "allocators\nend allocators\ndispatch\nend dispatch";
    let allocators = Span::new(source, 0, 10).unwrap();
    let end_allocators = Span::new(source, 11, 25).unwrap();
    let dispatch = Span::new(source, 26, 34).unwrap();
    let end_dispatch = Span::new(source, 35, source.len()).unwrap();
    let mut context = ContextValidator::new();

    context
        .begin_openmp(OmpDirectiveKind::Allocators, allocators)
        .unwrap();
    context
        .end_openmp(OmpDirectiveKind::EndAllocators, end_allocators)
        .unwrap();
    context
        .begin_openmp(OmpDirectiveKind::Dispatch, dispatch)
        .unwrap();
    context
        .end_openmp(OmpDirectiveKind::EndDispatch, end_dispatch)
        .unwrap();
    context
        .finish(Span::point(source, source.len()).unwrap())
        .unwrap();

    context
        .begin_openmp(OmpDirectiveKind::Dispatch, dispatch)
        .unwrap();
    let mismatch = context
        .end_openmp(OmpDirectiveKind::EndAllocators, end_allocators)
        .expect_err("end allocators closed a dispatch region");
    assert_eq!(mismatch.code(), DiagnosticCode::MismatchedEndDirective);
}

#[test]
fn update_host_and_self_share_one_canonical_ast_shape() {
    let host_source = "#pragma acc update host(array)";
    let self_source = "#pragma acc update self(array)";
    let host = acc_c(OpenAccVersion::V3_4, host_source).expect("host alias must parse");
    let canonical = acc_c(OpenAccVersion::V3_4, self_source).expect("self must parse");
    let host_clause = &host.directive().clauses()[0];
    let canonical_clause = &canonical.directive().clauses()[0];

    assert_eq!(host_clause.kind(), AccClauseKind::SelfClause);
    assert_eq!(canonical_clause.kind(), AccClauseKind::SelfClause);
    assert_eq!(host_clause.payload(), canonical_clause.payload());
    assert_eq!(host_clause.span().slice(host_source), Ok("host"));
    assert_eq!(canonical_clause.span().slice(self_source), Ok("self"));
    assert!(matches!(
        host_clause.payload(),
        AccClausePayload::ItemList { items, .. } if items.len() == 1
    ));
}

#[test]
fn historical_host_alias_keeps_its_openacc_10_floor() {
    acc_c(OpenAccVersion::V1_0, "#pragma acc update host(array)")
        .expect("historical host alias dates to OpenACC 1.0");
    assert!(
        acc_c(OpenAccVersion::V1_0, "#pragma acc update self(array)").is_err(),
        "canonical self spelling was accepted before OpenACC 2.0"
    );

    for malformed in [
        "#pragma acc update host",
        "#pragma acc update host()",
        "#pragma acc update host(a + b)",
        "#pragma acc update self",
        "#pragma acc update self()",
        "#pragma acc update self(a + b)",
    ] {
        assert!(
            acc_c(OpenAccVersion::V3_4, malformed).is_err(),
            "unexpectedly accepted malformed update action {malformed:?}"
        );
    }
}
