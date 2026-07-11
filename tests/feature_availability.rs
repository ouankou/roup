use roup::api::{OpenAccConfig, OpenMpConfig};
use roup::diagnostic::DiagnosticCode;
use roup::version::{
    CStandard, FortranStandard, HostLanguageProfile, OpenAccVersion, OpenMpVersion, SourceForm,
};

fn c23() -> HostLanguageProfile {
    HostLanguageProfile::C(CStandard::C23)
}

fn assert_omp_rejected(version: OpenMpVersion, source: &str) {
    let error = OpenMpConfig::exact(version, c23(), SourceForm::Pragma)
        .unwrap()
        .parser()
        .parse(source)
        .expect_err("syntax must not be accepted before its specification introduction");
    assert_eq!(
        error.code(),
        DiagnosticCode::NotAvailableInVersion,
        "{source}: {error}"
    );
}

fn assert_omp_accepted(version: OpenMpVersion, source: &str) {
    OpenMpConfig::exact(version, c23(), SourceForm::Pragma)
        .unwrap()
        .parser()
        .parse(source)
        .expect("syntax must be accepted from its introduction onward");
}

fn assert_fortran_omp_rejected(version: OpenMpVersion, source: &str) {
    let error = OpenMpConfig::exact(
        version,
        HostLanguageProfile::Fortran(FortranStandard::Fortran2023),
        SourceForm::FortranFree,
    )
    .unwrap()
    .parser()
    .parse(source)
    .expect_err("syntax must not be accepted before its specification introduction");
    assert_eq!(error.code(), DiagnosticCode::NotAvailableInVersion);
}

fn assert_fortran_omp_accepted(version: OpenMpVersion, source: &str) {
    OpenMpConfig::exact(
        version,
        HostLanguageProfile::Fortran(FortranStandard::Fortran2023),
        SourceForm::FortranFree,
    )
    .unwrap()
    .parser()
    .parse(source)
    .expect("syntax must be accepted from its introduction onward");
}

fn assert_acc_rejected(version: OpenAccVersion, source: &str) {
    let error = OpenAccConfig::exact(version, c23(), SourceForm::Pragma)
        .unwrap()
        .parser()
        .parse(source)
        .expect_err("syntax must not be accepted before its specification introduction");
    assert_eq!(
        error.code(),
        DiagnosticCode::NotAvailableInVersion,
        "{source}: {error}"
    );
}

fn assert_acc_accepted(version: OpenAccVersion, source: &str) {
    OpenAccConfig::exact(version, c23(), SourceForm::Pragma)
        .unwrap()
        .parser()
        .parse(source)
        .expect("syntax must be accepted from its introduction onward");
}

#[test]
fn openmp_argument_and_modifier_floors_are_intersected() {
    assert_omp_rejected(OpenMpVersion::V2_5, "#pragma omp for schedule(auto)");
    assert_omp_accepted(OpenMpVersion::V3_0, "#pragma omp for schedule(auto)");

    assert_omp_rejected(
        OpenMpVersion::V3_0,
        "#pragma omp parallel reduction(max: x)",
    );
    assert_omp_accepted(
        OpenMpVersion::V3_1,
        "#pragma omp parallel reduction(max: x)",
    );

    assert_omp_rejected(OpenMpVersion::V4_0, "#pragma omp for ordered(2)");
    assert_omp_accepted(OpenMpVersion::V4_5, "#pragma omp for ordered(2)");

    assert_omp_rejected(
        OpenMpVersion::V5_0,
        "#pragma omp parallel proc_bind(primary)",
    );
    assert_omp_accepted(
        OpenMpVersion::V5_1,
        "#pragma omp parallel proc_bind(primary)",
    );

    assert_omp_rejected(
        OpenMpVersion::V5_2,
        "#pragma omp target firstprivate(target, saved: x)",
    );
    assert_omp_accepted(
        OpenMpVersion::V6_0,
        "#pragma omp target firstprivate(target, saved: x)",
    );
}

#[test]
fn openmp_host_specific_default_values_use_real_floors() {
    assert_omp_rejected(OpenMpVersion::V5_0, "#pragma omp parallel default(private)");
    assert_omp_accepted(OpenMpVersion::V5_1, "#pragma omp parallel default(private)");
    assert_omp_rejected(
        OpenMpVersion::V5_0,
        "#pragma omp parallel default(firstprivate)",
    );
    assert_omp_accepted(
        OpenMpVersion::V5_1,
        "#pragma omp parallel default(firstprivate)",
    );
}

#[test]
fn openmp_map_and_defaultmap_subsyntax_has_independent_floors() {
    assert_omp_rejected(OpenMpVersion::V4_5, "#pragma omp target map(close, to: a)");
    assert_omp_accepted(OpenMpVersion::V5_0, "#pragma omp target map(close, to: a)");

    assert_omp_rejected(
        OpenMpVersion::V5_0,
        "#pragma omp target map(present, to: a)",
    );
    assert_omp_accepted(
        OpenMpVersion::V5_1,
        "#pragma omp target map(present, to: a)",
    );

    for source in [
        "#pragma omp target map(self, to: a)",
        "#pragma omp target map(storage: a)",
        "#pragma omp target map(ref_ptr, to: a)",
        "#pragma omp target map(delete, from: a)",
        "#pragma omp target defaultmap(storage:scalar)",
        "#pragma omp target defaultmap(self:scalar)",
    ] {
        assert_omp_rejected(OpenMpVersion::V5_2, source);
        assert_omp_accepted(OpenMpVersion::V6_0, source);
    }

    assert_omp_accepted(OpenMpVersion::V4_5, "#pragma omp target map(delete: a)");
    assert_omp_accepted(OpenMpVersion::V6_0, "#pragma omp target map(delete: a)");
}

#[test]
fn openmp_uses_allocators_preserves_history_and_versions_new_forms() {
    let historical =
        "#pragma omp target uses_allocators(my_alloc(my_traits), other_alloc(other_traits))";
    assert_omp_accepted(OpenMpVersion::V5_0, historical);
    assert_omp_accepted(OpenMpVersion::V6_0, historical);

    let modifier_form = "#pragma omp target uses_allocators(traits(my_traits), memspace(omp_high_bw_mem_space): my_alloc)";
    assert_omp_rejected(OpenMpVersion::V5_1, modifier_form);
    assert_omp_accepted(OpenMpVersion::V5_2, modifier_form);

    let multiple_specs = "#pragma omp target uses_allocators(traits(first_traits): first_alloc; memspace(omp_low_lat_mem_space): second_alloc)";
    assert_omp_rejected(OpenMpVersion::V5_2, multiple_specs);
    assert_omp_accepted(OpenMpVersion::V6_0, multiple_specs);

    let directive_modifier =
        "#pragma omp target uses_allocators(target, traits(my_traits): my_alloc)";
    for version in [OpenMpVersion::V5_2, OpenMpVersion::V6_0] {
        assert!(
            OpenMpConfig::exact(version, c23(), SourceForm::Pragma)
                .unwrap()
                .parser()
                .parse(directive_modifier)
                .is_err()
        );
    }
}

#[test]
fn removed_master_spellings_remain_cumulative() {
    assert_omp_accepted(OpenMpVersion::V1_0, "#pragma omp master");
    assert_omp_accepted(OpenMpVersion::V6_0, "#pragma omp master");

    assert_omp_rejected(OpenMpVersion::V4_5, "#pragma omp parallel master");
    assert_omp_accepted(OpenMpVersion::V5_0, "#pragma omp parallel master");
    assert_omp_accepted(OpenMpVersion::V6_0, "#pragma omp parallel master");

    assert_omp_accepted(
        OpenMpVersion::V6_0,
        "#pragma omp parallel proc_bind(master)",
    );
}

#[test]
fn openmp_clause_usage_additions_are_versioned() {
    let cases = [
        (
            OpenMpVersion::V4_0,
            OpenMpVersion::V4_5,
            "#pragma omp target private(x)",
        ),
        (
            OpenMpVersion::V4_5,
            OpenMpVersion::V5_0,
            "#pragma omp atomic hint(1)",
        ),
        (
            OpenMpVersion::V4_5,
            OpenMpVersion::V5_0,
            "#pragma omp simd if(x)",
        ),
        (
            OpenMpVersion::V4_5,
            OpenMpVersion::V5_0,
            "#pragma omp taskwait depend(in: x)",
        ),
        (
            OpenMpVersion::V5_0,
            OpenMpVersion::V5_1,
            "#pragma omp target thread_limit(4)",
        ),
        (
            OpenMpVersion::V5_0,
            OpenMpVersion::V5_1,
            "#pragma omp taskwait nowait",
        ),
        (
            OpenMpVersion::V5_0,
            OpenMpVersion::V5_1,
            "#pragma omp flush seq_cst",
        ),
        (
            OpenMpVersion::V5_1,
            OpenMpVersion::V5_2,
            "#pragma omp teams if(x)",
        ),
        (
            OpenMpVersion::V5_1,
            OpenMpVersion::V5_2,
            "#pragma omp scope firstprivate(x)",
        ),
        (
            OpenMpVersion::V5_2,
            OpenMpVersion::V6_0,
            "#pragma omp target default(firstprivate)",
        ),
        (
            OpenMpVersion::V5_2,
            OpenMpVersion::V6_0,
            "#pragma omp parallel message(\"stop\")",
        ),
    ];
    for (before, introduced, source) in cases {
        assert_omp_rejected(before, source);
        assert_omp_accepted(introduced, source);
    }
}

#[test]
fn typed_data_motion_and_declare_target_payloads_have_independent_floors() {
    assert_omp_accepted(OpenMpVersion::V4_0, "#pragma omp target update to(value)");

    let mapper = "#pragma omp target update to(mapper(default): value)";
    assert_omp_rejected(OpenMpVersion::V4_5, mapper);
    assert_omp_accepted(OpenMpVersion::V5_0, mapper);

    let present = "#pragma omp target update to(present: value)";
    assert_omp_rejected(OpenMpVersion::V5_0, present);
    assert_omp_accepted(OpenMpVersion::V5_1, present);

    let iterator = "#pragma omp target update to(iterator(int i=0:n): value[i])";
    assert_omp_rejected(OpenMpVersion::V5_0, iterator);
    assert_omp_accepted(OpenMpVersion::V5_1, iterator);

    let explicit_destroy = "#pragma omp depobj(handle) destroy(handle)";
    assert_omp_rejected(OpenMpVersion::V5_1, explicit_destroy);
    assert_omp_accepted(OpenMpVersion::V5_2, explicit_destroy);
    assert_omp_accepted(OpenMpVersion::V5_0, "#pragma omp depobj(handle) destroy");

    let automap = "!$omp declare target enter(automap: value)";
    assert_fortran_omp_rejected(OpenMpVersion::V5_2, automap);
    assert_fortran_omp_accepted(OpenMpVersion::V6_0, automap);
    assert!(
        OpenMpConfig::new(c23(), SourceForm::Pragma)
            .unwrap()
            .parser()
            .parse("#pragma omp declare target enter(automap: value)")
            .is_err()
    );
}

#[test]
fn openacc_payload_modifier_floors_are_intersected() {
    assert_acc_rejected(
        OpenAccVersion::V2_6,
        "#pragma acc parallel copyin(readonly: a)",
    );
    assert_acc_accepted(
        OpenAccVersion::V2_7,
        "#pragma acc parallel copyin(readonly: a)",
    );

    assert_acc_rejected(
        OpenAccVersion::V2_7,
        "#pragma acc parallel copyout(zero: a)",
    );
    assert_acc_accepted(
        OpenAccVersion::V3_0,
        "#pragma acc parallel copyout(zero: a)",
    );

    for source in [
        "#pragma acc parallel copy(always, capture: a)",
        "#pragma acc parallel copyin(alwaysin, capture: a)",
        "#pragma acc parallel copyout(alwaysout, capture: a)",
    ] {
        assert_acc_rejected(OpenAccVersion::V3_3, source);
        assert_acc_accepted(OpenAccVersion::V3_4, source);
    }
}

#[test]
fn openacc_new_argument_shapes_are_typed_and_versioned() {
    assert_acc_rejected(OpenAccVersion::V3_2, "#pragma acc parallel num_gangs(2, 3)");
    assert_acc_accepted(OpenAccVersion::V3_3, "#pragma acc parallel num_gangs(2, 3)");

    assert_acc_rejected(
        OpenAccVersion::V3_2,
        "#pragma acc parallel loop gang(dim: 2)",
    );
    assert_acc_accepted(
        OpenAccVersion::V3_3,
        "#pragma acc parallel loop gang(dim: 2)",
    );

    assert_acc_rejected(
        OpenAccVersion::V3_3,
        "#pragma acc parallel loop collapse(force: 2)",
    );
    assert_acc_accepted(
        OpenAccVersion::V3_4,
        "#pragma acc parallel loop collapse(force: 2)",
    );
}

#[test]
fn openacc_clause_usage_additions_are_versioned() {
    let cases = [
        (
            OpenAccVersion::V2_0,
            OpenAccVersion::V2_5,
            "#pragma acc kernels num_gangs(2)",
        ),
        (
            OpenAccVersion::V2_6,
            OpenAccVersion::V2_7,
            "#pragma acc parallel self(x)",
        ),
        (
            OpenAccVersion::V2_6,
            OpenAccVersion::V2_7,
            "#pragma acc host_data if(x)",
        ),
        (
            OpenAccVersion::V2_7,
            OpenAccVersion::V3_0,
            "#pragma acc wait if(x)",
        ),
        (
            OpenAccVersion::V3_1,
            OpenAccVersion::V3_2,
            "#pragma acc data copy(a) async",
        ),
        (
            OpenAccVersion::V3_3,
            OpenAccVersion::V3_4,
            "#pragma acc atomic if(x)",
        ),
    ];
    for (before, introduced, source) in cases {
        assert_acc_rejected(before, source);
        assert_acc_accepted(introduced, source);
    }
}
