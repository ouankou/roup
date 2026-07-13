use roup::api::{OpenAccConfig, OpenMpConfig};
use roup::version::{CStandard, HostLanguageProfile, SourceForm};

fn omp_extensions() -> roup::api::OpenMpParser {
    OpenMpConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .expect("valid OpenMP extension configuration")
        .with_ompparser_extensions()
        .parser()
}

fn acc_extensions() -> roup::api::OpenAccParser {
    OpenAccConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .expect("valid OpenACC extension configuration")
        .with_accparser_extensions()
        .parser()
}

#[test]
fn openmp_extension_dialect_is_typed_and_standard_validation_stays_strict() {
    let strict = OpenMpConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .expect("valid strict OpenMP configuration")
        .parser();
    for source in [
        "#pragma omp barrier private(a)",
        "#pragma omp simd aligned(a: 3)",
        "#pragma omp parallel if(1) if(2)",
        "#pragma omp target map(to: 1 + 2)",
        "#pragma omp parallel private(a/)",
    ] {
        assert!(
            strict.parse(source).is_err(),
            "standard dialect accepted compatibility-only input: {source}"
        );
        omp_extensions()
            .parse(source)
            .unwrap_or_else(|error| panic!("extension dialect lost typed input {source}: {error}"));
    }

    for source in [
        "#pragma omp parallel private(value@)",
        "#pragma omp parallel private(value",
        "#pragma omp target map(to:)",
    ] {
        assert!(omp_extensions().parse(source).is_err());
    }
}

#[test]
fn openacc_extension_dialect_is_typed_and_standard_validation_stays_strict() {
    let strict = OpenAccConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .expect("valid strict OpenACC configuration")
        .parser();
    for source in [
        "#pragma acc parallel default(none) default(present)",
        "#pragma acc parallel seq independent",
        "#pragma acc data copy(always, zero: x)",
        "#pragma acc data copyin(alwaysout: x)",
        "#pragma acc parallel loop gang(num: 1, num: 2)",
        "#pragma acc parallel private(a) async,",
    ] {
        assert!(
            strict.parse(source).is_err(),
            "standard dialect accepted compatibility-only input: {source}"
        );
        acc_extensions()
            .parse(source)
            .unwrap_or_else(|error| panic!("extension dialect lost typed input {source}: {error}"));
    }

    for source in [
        "#pragma acc parallel private(value@)",
        "#pragma acc parallel private(value",
        "#pragma acc data copyin()",
        "#pragma acc parallel gang(num:)",
        "#pragma acc parallel copy(a/)",
        "#pragma acc parallel reduction(+: a/)",
        "#pragma acc parallel private(a/)",
    ] {
        assert!(acc_extensions().parse(source).is_err());
    }
}
