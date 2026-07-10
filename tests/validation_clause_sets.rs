use roup::api::{OpenAccConfig, OpenMpConfig};
use roup::diagnostic::DiagnosticCode;
use roup::version::{CStandard, HostLanguageProfile, SourceForm};

fn omp() -> roup::api::OpenMpParser {
    OpenMpConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .unwrap()
        .parser()
}

fn acc() -> roup::api::OpenAccParser {
    OpenAccConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .unwrap()
        .parser()
}

#[test]
fn openmp_default_repeatability_accepts_standardized_repeated_clauses() {
    for source in [
        "#pragma omp parallel copyin(a) copyin(b)",
        "#pragma omp single copyprivate(a) copyprivate(b)",
        "#pragma omp simd aligned(a: 64) aligned(b: 64)",
        "#pragma omp declare simd uniform(a) uniform(b)",
        "#pragma omp taskgroup task_reduction(+: a) task_reduction(*: b)",
        "#pragma omp target is_device_ptr(a) is_device_ptr(b)",
        "#pragma omp target defaultmap(to: scalar) defaultmap(from: aggregate)",
    ] {
        omp()
            .parse(source)
            .unwrap_or_else(|error| panic!("rejected repeatable clause in {source:?}: {error}"));
    }
}

#[test]
fn openmp_explicit_unique_clauses_still_reject_duplicates() {
    for source in [
        "#pragma omp parallel num_threads(2) num_threads(4)",
        "#pragma omp for schedule(static) schedule(dynamic)",
        "#pragma omp atomic read hint(1) hint(2)",
        "#pragma omp declare simd inbranch inbranch",
    ] {
        let error = omp().parse(source).unwrap_err();
        assert_eq!(error.code(), DiagnosticCode::DuplicateClause, "{source}");
    }
}

#[test]
fn openmp_clause_sets_are_enforced() {
    for source in [
        "#pragma omp atomic read write",
        "#pragma omp atomic acquire release",
        "#pragma omp task detach(event) mergeable",
        "#pragma omp taskloop nogroup reduction(+: x)",
        "#pragma omp taskloop grainsize(2) num_tasks(4)",
        "#pragma omp unroll full partial(2)",
        "#pragma omp unroll apply(reverse)",
    ] {
        let error = omp().parse(source).unwrap_err();
        assert_eq!(error.code(), DiagnosticCode::ConflictingClauses, "{source}");
    }
    assert!(
        omp().parse("#pragma omp cancel parallel sections").is_err(),
        "cancel must name exactly one cancellable construct"
    );
}

#[test]
fn openmp_required_clause_sets_are_enforced() {
    for source in [
        "#pragma omp target data",
        "#pragma omp target update",
        "#pragma omp interop",
        "#pragma omp requires",
        "#pragma omp assume",
        "#pragma omp tile",
        "#pragma omp stripe",
        "#pragma omp split",
        "#pragma omp declare variant(fast)",
        "#pragma omp depobj(handle)",
    ] {
        let error = omp().parse(source).unwrap_err();
        assert!(
            matches!(
                error.code(),
                DiagnosticCode::MissingRequiredClause | DiagnosticCode::UnexpectedToken
            ),
            "wrong error for {source:?}: {error}"
        );
    }

    let error = omp().parse("#pragma omp interop destroy").unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::InvalidClause);
    omp()
        .parse("#pragma omp depobj(handle) destroy")
        .expect("the historical depobj destroy action may omit its argument");
}

#[test]
fn openmp_directive_clause_legality_uses_the_actual_constituent_sets() {
    for source in [
        "#pragma omp target device_type(host)",
        "#pragma omp dispatch is_device_ptr(pointer)",
        "#pragma omp dispatch has_device_addr(object)",
        "#pragma omp taskgraph priority(1)",
        "#pragma omp target data map(to: x) affinity(x)",
        "#pragma omp taskgroup task_reduction(+: x) allocate(x)",
    ] {
        omp()
            .parse(source)
            .unwrap_or_else(|error| panic!("rejected legal clause placement {source:?}: {error}"));
    }

    for source in [
        "#pragma omp simd uniform(x)",
        "#pragma omp simd inbranch",
        "#pragma omp simd firstprivate(x)",
        "#pragma omp for shared(x)",
        "#pragma omp parallel lastprivate(x)",
        "#pragma omp target update to(x) private(x)",
        "#pragma omp target update to(x) map(to: x)",
        "#pragma omp target data map(to: x) defaultmap(tofrom: scalar)",
        "#pragma omp taskloop detach(event)",
        "#pragma omp reverse full",
        "#pragma omp tile sizes(4) partial(2)",
    ] {
        let error = omp().parse(source).unwrap_err();
        assert_eq!(error.code(), DiagnosticCode::ClauseNotAllowed, "{source}");
    }
}

#[test]
fn openmp_target_data_motion_map_directions_are_strict() {
    for source in [
        "#pragma omp target enter data map(to: x)",
        "#pragma omp target enter data map(tofrom: x)",
        "#pragma omp target enter data map(alloc: x)",
        "#pragma omp target enter data map(storage: x)",
        "#pragma omp target exit data map(from: x)",
        "#pragma omp target exit data map(tofrom: x)",
        "#pragma omp target exit data map(release: x)",
        "#pragma omp target exit data map(delete: x)",
        "#pragma omp target exit data map(storage: x)",
    ] {
        omp()
            .parse(source)
            .unwrap_or_else(|error| panic!("rejected legal map direction {source:?}: {error}"));
    }

    for source in [
        "#pragma omp target enter data map(from: x)",
        "#pragma omp target enter data map(release: x)",
        "#pragma omp target enter data map(delete: x)",
        "#pragma omp target exit data map(to: x)",
        "#pragma omp target exit data map(alloc: x)",
    ] {
        let error = omp().parse(source).unwrap_err();
        assert_eq!(error.code(), DiagnosticCode::InvalidClause, "{source}");
    }
}

#[test]
fn openmp_if_targets_and_defaultmap_categories_cannot_overlap() {
    omp()
        .parse("#pragma omp target parallel if(target: on_device) if(parallel: in_team)")
        .expect("disjoint constituent if clauses are valid");

    for source in [
        "#pragma omp target parallel if(target parallel: enabled) if(target: on_device)",
        "#pragma omp target parallel if(enabled) if(parallel: in_team)",
        "#pragma omp target defaultmap(to: scalar) defaultmap(from: scalar)",
        "#pragma omp target defaultmap(to: all) defaultmap(from: scalar)",
        "#pragma omp target defaultmap(tofrom) defaultmap(from: aggregate)",
    ] {
        assert!(
            omp().parse(source).is_err(),
            "accepted overlapping modifier/category in {source:?}"
        );
    }
}

#[test]
fn openmp_default_categories_are_independently_repeatable_but_cannot_overlap() {
    omp()
        .parse("#pragma omp target default(scalar: private) default(aggregate: firstprivate)")
        .expect("distinct OpenMP 6 default categories may be specified independently");

    for source in [
        "#pragma omp target default(scalar: private) default(scalar: firstprivate)",
        "#pragma omp target default(none) default(scalar: private)",
        "#pragma omp target default(scalar: private) default(all: firstprivate)",
    ] {
        assert!(
            omp().parse(source).is_err(),
            "accepted overlapping default categories in {source:?}"
        );
    }

    assert!(
        omp()
            .parse("#pragma omp parallel default(scalar: private)")
            .is_err(),
        "categorized defaults are limited to target and target_data"
    );
}

#[test]
fn openacc_repeatability_is_evaluated_per_device_type_segment() {
    for source in [
        "#pragma acc parallel async async",
        "#pragma acc parallel loop reduction(+: total) device_type(host, gpu)",
        "#pragma acc parallel num_gangs(2) device_type(foo) num_gangs(4)",
        "#pragma acc routine device_type(foo) worker device_type(*) gang",
        "#pragma acc routine gang device_type(foo) gang",
    ] {
        acc()
            .parse(source)
            .unwrap_or_else(|error| panic!("rejected repeatable/segmented {source:?}: {error}"));
    }
}

#[test]
fn openacc_device_type_followers_and_effective_routine_clauses_are_strict() {
    for source in [
        "#pragma acc parallel device_type(foo) copy(a)",
        "#pragma acc data device_type(foo) copy(a)",
        "#pragma acc loop device_type(foo) private(a)",
        "#pragma acc routine device_type(foo) nohost",
        "#pragma acc update device_type(foo) device(a)",
        "#pragma acc routine gang worker",
        "#pragma acc routine gang gang",
        "#pragma acc routine gang device_type(foo) worker",
        "#pragma acc parallel device_type(foo) num_gangs(1) device_type(foo) num_gangs(2)",
        "#pragma acc parallel device_type(*) num_gangs(1) device_type(*) num_gangs(2)",
    ] {
        assert!(
            acc().parse(source).is_err(),
            "accepted invalid device-specific source {source:?}"
        );
    }
}

#[test]
fn openacc_local_unique_and_atomic_clause_sets_are_enforced() {
    for source in [
        "#pragma acc parallel if(a) if(b)",
        "#pragma acc parallel default(none) default(present)",
        "#pragma acc atomic read write",
    ] {
        assert!(
            acc().parse(source).is_err(),
            "accepted invalid OpenACC clause set {source:?}"
        );
    }
}

#[test]
fn openacc_required_forms_respect_historical_optional_clause_lists() {
    for source in [
        "#pragma acc data",
        "#pragma acc host_data",
        "#pragma acc routine",
        "#pragma acc enter data if(enabled)",
        "#pragma acc exit data if(enabled)",
        "#pragma acc loop seq gang",
        "#pragma acc loop auto independent",
    ] {
        acc()
            .parse(source)
            .unwrap_or_else(|error| panic!("rejected historical standardized {source:?}: {error}"));
    }

    for source in [
        "#pragma acc declare",
        "#pragma acc enter data",
        "#pragma acc exit data",
        "#pragma acc set",
        "#pragma acc update",
    ] {
        let error = acc().parse(source).unwrap_err();
        assert_eq!(
            error.code(),
            DiagnosticCode::MissingRequiredClause,
            "{source}"
        );
    }
}

#[test]
fn openacc_compute_and_loop_data_clauses_have_distinct_legality() {
    for source in [
        "#pragma acc parallel firstprivate(x)",
        "#pragma acc parallel loop firstprivate(x)",
        "#pragma acc kernels loop private(x) reduction(+: y)",
    ] {
        acc()
            .parse(source)
            .unwrap_or_else(|error| panic!("rejected legal data clause {source:?}: {error}"));
    }

    for source in [
        "#pragma acc kernels private(x)",
        "#pragma acc kernels firstprivate(x)",
        "#pragma acc kernels reduction(+: x)",
        "#pragma acc loop firstprivate(x)",
        "#pragma acc kernels loop firstprivate(x)",
    ] {
        let error = acc().parse(source).unwrap_err();
        assert_eq!(error.code(), DiagnosticCode::ClauseNotAllowed, "{source}");
    }
}
