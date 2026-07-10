use roup::api::{OpenAccConfig, ParsedOpenAccDirective};
use roup::ast::{AccCacheItem, AccDirectiveParameter, AccEndKind};
use roup::diagnostic::Diagnostic;
use roup::version::{CStandard, FortranStandard, HostLanguageProfile, OpenAccVersion, SourceForm};

fn parse(source: &str) -> Result<ParsedOpenAccDirective, Diagnostic> {
    OpenAccConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .expect("valid C configuration")
        .parser()
        .parse(source)
}

fn parse_fortran(source: &str) -> Result<ParsedOpenAccDirective, Diagnostic> {
    OpenAccConfig::new(
        HostLanguageProfile::Fortran(FortranStandard::Fortran2023),
        SourceForm::FortranFree,
    )
    .expect("valid Fortran configuration")
    .parser()
    .parse(source)
}

fn parse_fortran_exact(
    version: OpenAccVersion,
    source: &str,
) -> Result<ParsedOpenAccDirective, Diagnostic> {
    OpenAccConfig::exact(
        version,
        HostLanguageProfile::Fortran(FortranStandard::Fortran2023),
        SourceForm::FortranFree,
    )
    .expect("valid Fortran configuration")
    .parser()
    .parse(source)
}

#[test]
fn cache_parameter_is_typed_once_at_the_ast_boundary() {
    let parsed = parse("#pragma acc cache(readonly: values[index], tile[0:n])")
        .expect("typed cache must parse");
    let Some(AccDirectiveParameter::Cache(cache)) = parsed.directive().parameter() else {
        panic!("expected typed cache parameter");
    };
    assert!(cache.readonly());
    assert!(matches!(cache.items()[0], AccCacheItem::ArrayElement(_)));
    assert!(matches!(
        cache.items()[1],
        AccCacheItem::ContiguousSubarray(_)
    ));
    assert_eq!(cache.items()[0].variable().to_string(), "values[index]");
    assert_eq!(cache.items()[1].variable().to_string(), "tile[0:n]");
}

#[test]
fn wait_modifiers_and_expressions_are_parsed_without_raw_duplicates() {
    let parsed = parse(
        "#pragma acc wait(devnum: flag ? first_device : second_device: queues: first_queue, choose ? second_queue : third_queue)",
    )
    .expect("typed wait must parse");
    let Some(AccDirectiveParameter::Wait(wait)) = parsed.directive().parameter() else {
        panic!("expected typed wait parameter");
    };
    assert!(wait.devnum().is_some());
    assert_eq!(wait.queues().len(), 2);

    let bare = parse("#pragma acc wait").expect("bare wait is valid");
    assert!(bare.directive().parameter().is_none());
}

#[test]
fn malformed_cache_and_wait_parameters_are_hard_errors() {
    for source in [
        "#pragma acc cache()",
        "#pragma acc cache(readonly:)",
        "#pragma acc cache(unknown: value)",
        "#pragma acc cache(readonly: value,)",
        "#pragma acc cache(scalar)",
        "#pragma acc cache(values[0:n:2])",
        "#pragma acc cache(call(values))",
        "#pragma acc wait()",
        "#pragma acc wait(devnum: 1)",
        "#pragma acc wait(devnum:: queue)",
        "#pragma acc wait(devnum: 1:)",
        "#pragma acc wait(queues:)",
        "#pragma acc wait(unknown: queue)",
        "#pragma acc wait(queue,)",
    ] {
        assert!(parse(source).is_err(), "unexpectedly accepted {source:?}");
    }
}

#[test]
fn modifier_keyword_case_follows_the_host_language() {
    assert!(parse("#pragma acc cache(READONLY: value[index])").is_err());
    assert!(parse("#pragma acc wait(QUEUES: queue)").is_err());

    for source in [
        "!$acc cache(READONLY: value(index))",
        "!$acc wait(QUEUES: queue)",
        "!$acc wait(DEVNUM: device: QUEUES: queue)",
    ] {
        parse_fortran(source).unwrap_or_else(|error| panic!("rejected {source:?}: {error}"));
    }
}

#[test]
fn fortran_cache_items_distinguish_elements_and_contiguous_subarrays() {
    let parsed = parse_fortran("!$acc cache(VALUES(I), TILE(1:N))")
        .expect("Fortran cache items must parse case-insensitively");
    let Some(AccDirectiveParameter::Cache(cache)) = parsed.directive().parameter() else {
        panic!("expected typed cache parameter");
    };
    assert!(matches!(cache.items()[0], AccCacheItem::ArrayElement(_)));
    assert!(matches!(
        cache.items()[1],
        AccCacheItem::ContiguousSubarray(_)
    ));

    for source in ["!$acc cache(SCALAR)", "!$acc cache(TILE(1:N:2))"] {
        assert!(
            parse_fortran(source).is_err(),
            "unexpectedly accepted {source:?}"
        );
    }
}

#[test]
fn routine_name_presence_has_one_ast_state() {
    let bare = parse("#pragma acc routine").expect("bare routine must parse");
    assert!(bare.directive().parameter().is_none());

    let named = parse("#pragma acc routine(worker_fn)").expect("named routine must parse");
    let Some(AccDirectiveParameter::Routine(routine)) = named.directive().parameter() else {
        panic!("expected typed routine parameter");
    };
    assert_eq!(routine.name().as_str(), "worker_fn");

    for source in [
        "#pragma acc routine()",
        "#pragma acc routine(worker_fn, other)",
    ] {
        assert!(parse(source).is_err(), "unexpectedly accepted {source:?}");
    }
}

#[test]
fn end_parameter_is_restricted_to_standardized_fortran_pairs() {
    let cases = [
        ("atomic", AccEndKind::Atomic),
        ("data", AccEndKind::Data),
        ("host_data", AccEndKind::HostData),
        ("kernels", AccEndKind::Kernels),
        ("kernels loop", AccEndKind::KernelsLoop),
        ("loop", AccEndKind::Loop),
        ("parallel", AccEndKind::Parallel),
        ("parallel loop", AccEndKind::ParallelLoop),
        ("serial", AccEndKind::Serial),
        ("serial loop", AccEndKind::SerialLoop),
    ];
    for (spelling, expected) in cases {
        let source = format!("!$acc end {}", spelling.to_ascii_uppercase());
        let parsed =
            parse_fortran(&source).unwrap_or_else(|error| panic!("rejected {source:?}: {error}"));
        assert_eq!(
            parsed.directive().parameter(),
            Some(&AccDirectiveParameter::End(expected))
        );
    }

    for source in [
        "!$acc end",
        "!$acc end cache",
        "!$acc end declare",
        "!$acc end enter data",
        "!$acc end exit data",
        "!$acc end init",
        "!$acc end routine",
        "!$acc end set",
        "!$acc end shutdown",
        "!$acc end update",
        "!$acc end wait",
    ] {
        assert!(
            parse_fortran(source).is_err(),
            "unexpectedly accepted {source:?}"
        );
    }
}

#[test]
fn end_pair_versions_preserve_the_historical_specification_floors() {
    assert!(parse_fortran_exact(OpenAccVersion::V1_0, "!$acc end parallel").is_ok());
    assert!(parse_fortran_exact(OpenAccVersion::V1_0, "!$acc end atomic").is_err());
    assert!(parse_fortran_exact(OpenAccVersion::V2_0, "!$acc end atomic").is_ok());
    assert!(parse_fortran_exact(OpenAccVersion::V2_5, "!$acc end serial").is_err());
    assert!(parse_fortran_exact(OpenAccVersion::V2_6, "!$acc end serial").is_ok());
}

#[test]
fn cache_readonly_preserves_its_historical_introduction_floor() {
    let source = "!$acc cache(readonly: values(index))";
    assert!(parse_fortran_exact(OpenAccVersion::V2_6, source).is_err());
    assert!(parse_fortran_exact(OpenAccVersion::V2_7, source).is_ok());
}

#[test]
fn wait_directive_devnum_preserves_its_openacc_3_0_floor() {
    let source = "!$acc wait(DEVNUM: device: QUEUES: queue)";
    assert!(parse_fortran_exact(OpenAccVersion::V2_7, source).is_err());
    assert!(parse_fortran_exact(OpenAccVersion::V3_0, source).is_ok());
}
