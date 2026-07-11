use roup::api::{OpenAccConfig, OpenMpConfig};
use roup::ast::OmpReductionIdentifier;
use roup::host::{ExprKind, TypeName};
use roup::ir::{ClauseData, Expression, ParserConfig};
use roup::version::SourceForm;
use roup::version::{CStandard, CppStandard, FortranStandard, HostLanguageProfile};

fn expression(source: &str, profile: HostLanguageProfile) -> Result<Expression, String> {
    Expression::new(source, &ParserConfig::new(profile)).map_err(|error| error.to_string())
}

#[test]
fn c_literal_features_follow_the_selected_standard() {
    let c89 = HostLanguageProfile::C(CStandard::C89);
    let c99 = HostLanguageProfile::C(CStandard::C99);
    let c18 = HostLanguageProfile::C(CStandard::C18);
    let c23 = HostLanguageProfile::C(CStandard::C23);

    assert!(expression("1LL", c89).is_err());
    assert!(expression("1LL", c99).is_ok());
    assert!(expression("value // comment", c89).is_err());
    assert!(expression("value // comment", c99).is_ok());

    for source in ["0b1010", "1'000", "1z", "u8'x'"] {
        assert!(
            expression(source, c18).is_err(),
            "accepted {source:?} in C18"
        );
        assert!(
            expression(source, c23).is_ok(),
            "rejected {source:?} in C23"
        );
    }
}

#[test]
fn cpp_literal_features_follow_the_selected_standard() {
    let cpp11 = HostLanguageProfile::Cpp(CppStandard::Cpp11);
    let cpp14 = HostLanguageProfile::Cpp(CppStandard::Cpp14);
    let cpp20 = HostLanguageProfile::Cpp(CppStandard::Cpp20);
    let cpp23 = HostLanguageProfile::Cpp(CppStandard::Cpp23);

    for source in ["0b1010", "1'000"] {
        assert!(
            expression(source, cpp11).is_err(),
            "accepted {source:?} in C++11"
        );
        assert!(
            expression(source, cpp14).is_ok(),
            "rejected {source:?} in C++14"
        );
    }
    assert!(expression("1z", cpp20).is_err());
    assert!(expression("1z", cpp23).is_ok());
}

#[test]
fn nullptr_is_never_misclassified_in_an_older_profile() {
    let old_c = expression("nullptr", HostLanguageProfile::C(CStandard::C18)).unwrap();
    let old_cpp = expression("nullptr", HostLanguageProfile::Cpp(CppStandard::Cpp98)).unwrap();
    assert!(matches!(old_c.ast().kind, ExprKind::Name(_)));
    assert!(matches!(old_cpp.ast().kind, ExprKind::Name(_)));

    let c23 = expression("nullptr", HostLanguageProfile::C(CStandard::C23)).unwrap();
    let cpp11 = expression("nullptr", HostLanguageProfile::Cpp(CppStandard::Cpp11)).unwrap();
    assert!(matches!(c23.ast().kind, ExprKind::Literal(_)));
    assert!(matches!(cpp11.ast().kind, ExprKind::Literal(_)));
}

#[test]
fn fortran_expression_features_follow_the_selected_standard() {
    let f77 = HostLanguageProfile::Fortran(FortranStandard::Fortran77);
    let f90 = HostLanguageProfile::Fortran(FortranStandard::Fortran90);

    for source in ["1_8", "array(1:upper)", "object%field", "call(kind=8)"] {
        assert!(
            expression(source, f77).is_err(),
            "accepted {source:?} in Fortran 77"
        );
        assert!(
            expression(source, f90).is_ok(),
            "rejected {source:?} in Fortran 90"
        );
    }
}

#[test]
fn type_name_features_use_the_same_exact_profile() {
    assert!(
        TypeName::parse_with_profile("long long", HostLanguageProfile::C(CStandard::C89),).is_err()
    );
    assert!(
        TypeName::parse_with_profile("long long", HostLanguageProfile::C(CStandard::C99),).is_ok()
    );

    assert!(
        TypeName::parse_with_profile(
            "decltype(value)",
            HostLanguageProfile::Cpp(CppStandard::Cpp98),
        )
        .is_err()
    );
    assert!(
        TypeName::parse_with_profile(
            "decltype(value)",
            HostLanguageProfile::Cpp(CppStandard::Cpp11),
        )
        .is_ok()
    );

    assert!(
        TypeName::parse_with_profile(
            "integer(kind=8)",
            HostLanguageProfile::Fortran(FortranStandard::Fortran77),
        )
        .is_err()
    );
    assert!(
        TypeName::parse_with_profile(
            "integer(kind=8)",
            HostLanguageProfile::Fortran(FortranStandard::Fortran90),
        )
        .is_ok()
    );

    assert!(
        TypeName::parse_with_profile(
            "class(my_type)",
            HostLanguageProfile::Fortran(FortranStandard::Fortran95),
        )
        .is_err()
    );
    assert!(
        TypeName::parse_with_profile(
            "class(my_type)",
            HostLanguageProfile::Fortran(FortranStandard::Fortran2003),
        )
        .is_ok()
    );
}

fn assert_omp_payload_rejected_by_c_and_cpp(source: &str) {
    for profile in [
        HostLanguageProfile::C(CStandard::C23),
        HostLanguageProfile::Cpp(CppStandard::Cpp23),
    ] {
        let parser = OpenMpConfig::new(profile, SourceForm::Pragma)
            .expect("valid pragma profile")
            .parser();
        assert!(
            parser.parse(source).is_err(),
            "accepted case-folded OpenMP payload {source:?} for {profile:?}"
        );
    }
}

fn assert_acc_payload_rejected_by_c_and_cpp(source: &str) {
    for profile in [
        HostLanguageProfile::C(CStandard::C23),
        HostLanguageProfile::Cpp(CppStandard::Cpp23),
    ] {
        let parser = OpenAccConfig::new(profile, SourceForm::Pragma)
            .expect("valid pragma profile")
            .parser();
        assert!(
            parser.parse(source).is_err(),
            "accepted case-folded OpenACC payload {source:?} for {profile:?}"
        );
    }
}

fn assert_omp_fortran_payload_accepted(source: &str) {
    OpenMpConfig::new(
        HostLanguageProfile::Fortran(FortranStandard::Fortran2023),
        SourceForm::FortranFree,
    )
    .expect("valid Fortran profile")
    .parser()
    .parse(source)
    .unwrap_or_else(|error| panic!("rejected case-insensitive OpenMP payload {source:?}: {error}"));
}

fn assert_acc_fortran_payload_accepted(source: &str) {
    OpenAccConfig::new(
        HostLanguageProfile::Fortran(FortranStandard::Fortran2023),
        SourceForm::FortranFree,
    )
    .expect("valid Fortran profile")
    .parser()
    .parse(source)
    .unwrap_or_else(|error| {
        panic!("rejected case-insensitive OpenACC payload {source:?}: {error}")
    });
}

#[test]
fn openmp_payload_keywords_are_case_sensitive_in_c_and_cpp() {
    for source in [
        "#pragma omp for schedule(STATIC)",
        "#pragma omp for schedule(MONOTONIC: static)",
        "#pragma omp parallel default(SHARED)",
        "#pragma omp target map(TO: value)",
        "#pragma omp target map(ALWAYS, to: value)",
        "#pragma omp parallel reduction(TASK, +: value)",
        "#pragma omp requires atomic_default_mem_order(SEQ_CST)",
        "#pragma omp atomic compare fail(RELAXED)",
        "#pragma omp for order(REPRODUCIBLE: concurrent)",
        "#pragma omp for order(reproducible: CONCURRENT)",
        "#pragma omp target device(DEVICE_NUM: 0)",
        "#pragma omp target if(TARGET: enabled)",
        "#pragma omp cancel PARALLEL",
        "#pragma omp target defaultmap(TOFROM: SCALAR)",
        "#pragma omp parallel proc_bind(CLOSE)",
        "#pragma omp for lastprivate(CONDITIONAL: value)",
        "#pragma omp task depend(IN: value)",
        "#pragma omp error at(EXECUTION) severity(WARNING)",
        "#pragma omp interop init(TARGET: object)",
        "#pragma omp parallel reduction(ORIGINAL(SHARING=PRIVATE), +: value)",
        "#pragma omp metadirective when(DEVICE={KIND(cpu)}: parallel)",
    ] {
        assert_omp_payload_rejected_by_c_and_cpp(source);
    }
}

#[test]
fn equivalent_openmp_payload_keywords_are_case_insensitive_in_fortran() {
    for source in [
        "!$omp do schedule(STATIC)",
        "!$omp do schedule(MONOTONIC: STATIC)",
        "!$omp parallel default(SHARED)",
        "!$omp target map(TO: VALUE)",
        "!$omp target map(ALWAYS, TO: VALUE)",
        "!$omp parallel reduction(TASK, +: VALUE)",
        "!$omp parallel reduction(MAX: VALUE)",
        "!$omp requires atomic_default_mem_order(SEQ_CST)",
        "!$omp atomic compare fail(RELAXED)",
        "!$omp do order(REPRODUCIBLE: CONCURRENT)",
        "!$omp target device(DEVICE_NUM: 0)",
        "!$omp target if(TARGET: ENABLED)",
        "!$omp cancel PARALLEL",
        "!$omp target defaultmap(TOFROM: SCALAR)",
        "!$omp parallel proc_bind(CLOSE)",
        "!$omp do lastprivate(CONDITIONAL: VALUE)",
        "!$omp task depend(IN: VALUE)",
        "!$omp error at(EXECUTION) severity(WARNING)",
        "!$omp interop init(TARGET: OBJECT)",
        "!$omp parallel reduction(ORIGINAL(SHARING=PRIVATE), +: VALUE)",
        "!$omp metadirective when(DEVICE={KIND(cpu)}: parallel)",
    ] {
        assert_omp_fortran_payload_accepted(source);
    }
}

#[test]
fn openacc_payload_keywords_are_case_sensitive_in_c_and_cpp() {
    for source in [
        "#pragma acc parallel loop gang(NUM: 4)",
        "#pragma acc parallel loop gang(STATIC: *)",
        "#pragma acc parallel copyin(READONLY: value)",
        "#pragma acc parallel copyout(ZERO: value)",
        "#pragma acc parallel reduction(MAX: value)",
        "#pragma acc parallel wait(DEVNUM: device: QUEUES: queue)",
        "#pragma acc wait(DEVNUM: device: QUEUES: queue)",
        "#pragma acc parallel default(PRESENT)",
        "#pragma acc loop collapse(FORCE: 2)",
        "#pragma acc loop worker(NUM: 4)",
        "#pragma acc loop vector(LENGTH: 4)",
        "#pragma acc parallel device_type(HOST)",
        "#pragma acc end PARALLEL",
    ] {
        assert_acc_payload_rejected_by_c_and_cpp(source);
    }
}

#[test]
fn equivalent_openacc_payload_keywords_are_case_insensitive_in_fortran() {
    for source in [
        "!$acc parallel loop gang(NUM: 4)",
        "!$acc parallel loop gang(STATIC: *)",
        "!$acc parallel copyin(READONLY: VALUE)",
        "!$acc parallel copyout(ZERO: VALUE)",
        "!$acc parallel reduction(MAX: VALUE)",
        "!$acc parallel wait(DEVNUM: DEVICE: QUEUES: QUEUE)",
        "!$acc wait(DEVNUM: DEVICE: QUEUES: QUEUE)",
        "!$acc parallel default(PRESENT)",
        "!$acc loop collapse(FORCE: 2)",
        "!$acc loop worker(NUM: 4)",
        "!$acc loop vector(LENGTH: 4)",
        "!$acc parallel device_type(HOST)",
        "!$acc end PARALLEL",
    ] {
        assert_acc_fortran_payload_accepted(source);
    }
}

#[test]
fn case_sensitive_user_defined_reduction_identifiers_are_preserved() {
    let parsed = OpenMpConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .unwrap()
        .parser()
        .parse("#pragma omp parallel reduction(MyReduction: value)")
        .expect("a distinct user-defined reduction identifier is valid");

    let ClauseData::Reduction { operator, .. } = parsed.directive().clauses()[0].payload() else {
        panic!("expected a typed reduction payload");
    };
    let OmpReductionIdentifier::Name(identifier) = operator else {
        panic!("expected a user-defined reduction operator");
    };
    let name = identifier
        .qualified_name()
        .expect("a C identifier has a qualified-name representation");
    assert_eq!(name.segments[0].as_str(), "MyReduction");
}
