use roup::api::{OpenMpConfig, ParsedOpenMpDirective};
use roup::diagnostic::Diagnostic;
use roup::ir::{ClauseData, DefaultKind, DefaultmapCategory, LinearModifier, LinearSourceSyntax};
use roup::version::{
    CStandard, CppStandard, FortranStandard, HostLanguageProfile, OpenMpVersion, SourceForm,
};

fn c(source: &str) -> Result<ParsedOpenMpDirective, Diagnostic> {
    OpenMpConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .unwrap()
        .parser()
        .parse(source)
}

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

fn cpp(source: &str) -> Result<ParsedOpenMpDirective, Diagnostic> {
    OpenMpConfig::new(
        HostLanguageProfile::Cpp(CppStandard::Cpp23),
        SourceForm::Pragma,
    )
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
fn num_threads_is_a_typed_positive_list_with_strict_prescriptiveness() {
    let historical = c_exact(
        OpenMpVersion::V2_0,
        "#pragma omp parallel num_threads(requested)",
    )
    .unwrap();
    assert!(matches!(
        historical.directive().clauses()[0].payload(),
        ClauseData::NumThreads { strict: false, nthreads }
            if nthreads.len() == 1 && nthreads[0].to_string() == "requested"
    ));

    let modern = c("#pragma omp parallel num_threads(strict: outer, inner)").unwrap();
    assert!(matches!(
        modern.directive().clauses()[0].payload(),
        ClauseData::NumThreads { strict: true, nthreads }
            if nthreads.len() == 2
    ));
    assert!(c_exact(
        OpenMpVersion::V5_2,
        "#pragma omp parallel num_threads(outer, inner)",
    )
    .is_err());
    c_exact(
        OpenMpVersion::V6_0,
        "#pragma omp parallel num_threads(outer, inner)",
    )
    .unwrap();

    for source in [
        "#pragma omp parallel num_threads()",
        "#pragma omp parallel num_threads(0)",
        "#pragma omp parallel num_threads(-1)",
        "#pragma omp parallel num_threads(1.5)",
        "#pragma omp parallel num_threads(strict:)",
        "#pragma omp parallel num_threads(a,,b)",
    ] {
        assert!(c(source).is_err(), "unexpectedly accepted {source:?}");
    }
    c("#pragma omp parallel num_threads(select(a, b))").unwrap();
}

#[test]
fn num_teams_preserves_distinct_lower_and_upper_bounds() {
    let bounded = c("#pragma omp teams num_teams(2: 8)").unwrap();
    assert!(matches!(
        bounded.directive().clauses()[0].payload(),
        ClauseData::NumTeams {
            lower_bound: Some(lower),
            upper_bound,
        } if lower.to_string() == "2" && upper_bound.to_string() == "8"
    ));
    assert!(c_exact(OpenMpVersion::V5_0, "#pragma omp teams num_teams(2: 8)").is_err());
    c_exact(OpenMpVersion::V5_1, "#pragma omp teams num_teams(2: 8)").unwrap();

    c("#pragma omp teams num_teams(flag ? low : high)").unwrap();
    for source in [
        "#pragma omp teams num_teams(0)",
        "#pragma omp teams num_teams(4: 2)",
        "#pragma omp teams num_teams(1.5: 2)",
        "#pragma omp teams num_teams(: 2)",
        "#pragma omp teams num_teams(2:)",
    ] {
        assert!(c(source).is_err(), "unexpectedly accepted {source:?}");
    }
}

#[test]
fn default_variable_categories_are_typed_and_host_gated() {
    let parsed = c("#pragma omp target default(scalar: private)").unwrap();
    assert!(matches!(
        parsed.directive().clauses()[0].payload(),
        ClauseData::Default {
            category: Some(DefaultmapCategory::Scalar),
            kind: DefaultKind::Private,
        }
    ));
    assert!(c_exact(
        OpenMpVersion::V5_2,
        "#pragma omp target default(scalar: private)",
    )
    .is_err());
    c_exact(
        OpenMpVersion::V6_0,
        "#pragma omp target default(scalar: private)",
    )
    .unwrap();

    assert!(c("#pragma omp target default(allocatable: private)").is_err());
    fortran("!$omp target default(allocatable: private)").unwrap();
    assert!(c("#pragma omp target default(typo: private)").is_err());
}

#[test]
fn linear_cumulative_grammars_canonicalize_without_losing_provenance() {
    let historical = c("#pragma omp declare simd linear(x: 2)").unwrap();
    assert!(matches!(
        historical.directive().clauses()[0].payload(),
        ClauseData::Linear {
            source_syntax: LinearSourceSyntax::Historical,
            modifier: None,
            step: Some(_),
            ..
        }
    ));

    let prefix = c("#pragma omp declare simd linear(val(x): 2)").unwrap();
    assert!(matches!(
        prefix.directive().clauses()[0].payload(),
        ClauseData::Linear {
            source_syntax: LinearSourceSyntax::ModifierPrefix,
            modifier: Some(LinearModifier::Val),
            ..
        }
    ));

    let canonical = cpp("#pragma omp declare simd linear(x: step(2), ref)").unwrap();
    assert!(matches!(
        canonical.directive().clauses()[0].payload(),
        ClauseData::Linear {
            source_syntax: LinearSourceSyntax::CanonicalModifiers,
            modifier: Some(LinearModifier::Ref),
            step: Some(_),
            ..
        }
    ));
    assert!(c_exact(
        OpenMpVersion::V5_1,
        "#pragma omp declare simd linear(x: step(2), val)",
    )
    .is_err());
    c_exact(
        OpenMpVersion::V5_2,
        "#pragma omp declare simd linear(x: step(2), val)",
    )
    .unwrap();

    for source in [
        "#pragma omp declare simd linear(x: step(1), step(2), val)",
        "#pragma omp declare simd linear(x: val, ref)",
        "#pragma omp declare simd linear(val(x) trailing)",
        "#pragma omp declare simd linear(val(): 2)",
    ] {
        assert!(c(source).is_err(), "unexpectedly accepted {source:?}");
    }
}
