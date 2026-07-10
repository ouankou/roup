use roup::api::{OpenAccConfig, ParsedOpenAccDirective};
use roup::ast::{
    AccBindTarget, AccClausePayload, AccDeviceType, AccGangArgument, AccReductionOperator,
    AccVectorClause, AccWorkerClause,
};
use roup::diagnostic::Diagnostic;
use roup::version::{CStandard, FortranStandard, HostLanguageProfile, OpenAccVersion, SourceForm};

fn parse_c(source: &str) -> Result<ParsedOpenAccDirective, Diagnostic> {
    OpenAccConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .expect("valid C OpenACC configuration")
        .parser()
        .parse(source)
}

fn parse_fortran(source: &str) -> Result<ParsedOpenAccDirective, Diagnostic> {
    OpenAccConfig::new(
        HostLanguageProfile::Fortran(FortranStandard::Fortran2023),
        SourceForm::FortranFree,
    )
    .expect("valid Fortran OpenACC configuration")
    .parser()
    .parse(source)
}

fn parse_c_exact(
    version: OpenAccVersion,
    source: &str,
) -> Result<ParsedOpenAccDirective, Diagnostic> {
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
fn worker_and_vector_have_only_bare_or_one_expression_states() {
    let bare_worker = parse_c("#pragma acc loop worker").expect("bare worker must parse");
    assert!(matches!(
        bare_worker.directive().clauses()[0].payload(),
        AccClausePayload::Worker(AccWorkerClause::Bare)
    ));

    let worker = parse_c("#pragma acc loop worker(num: count)").expect("worker count must parse");
    let AccClausePayload::Worker(AccWorkerClause::Num(count)) =
        worker.directive().clauses()[0].payload()
    else {
        panic!("expected a typed worker num argument");
    };
    assert_eq!(count.to_string(), "count");

    let vector = parse_c("#pragma acc loop vector(width)").expect("vector expression must parse");
    let AccClausePayload::Vector(AccVectorClause::Expression(width)) =
        vector.directive().clauses()[0].payload()
    else {
        panic!("expected a typed vector expression argument");
    };
    assert_eq!(width.to_string(), "width");

    for source in [
        "#pragma acc loop worker(first, second)",
        "#pragma acc loop worker(num: first, second)",
        "#pragma acc loop vector(first, second)",
        "#pragma acc loop vector(length: first, second)",
    ] {
        assert!(parse_c(source).is_err(), "unexpectedly accepted {source:?}");
    }
}

#[test]
fn routine_parallelism_clauses_use_their_routine_specific_shapes() {
    for source in [
        "#pragma acc routine",
        "#pragma acc routine gang",
        "#pragma acc routine gang(dim: 2)",
        "#pragma acc routine worker",
        "#pragma acc routine vector",
        "#pragma acc routine seq",
    ] {
        parse_c(source).unwrap_or_else(|error| panic!("rejected {source:?}: {error}"));
    }

    let gang = parse_c("#pragma acc routine gang(dim: 2)").unwrap();
    assert!(matches!(
        gang.directive().clauses()[0].payload(),
        AccClausePayload::Gang(value)
            if matches!(value.arguments(), [AccGangArgument::Dim(_)])
    ));

    for source in [
        "#pragma acc routine gang(4)",
        "#pragma acc routine gang(num: 4)",
        "#pragma acc routine gang(static: 4)",
        "#pragma acc routine gang(dim: 2, num: 4)",
        "#pragma acc routine worker(4)",
        "#pragma acc routine worker(num: 4)",
        "#pragma acc routine vector(4)",
        "#pragma acc routine vector(length: 4)",
    ] {
        assert!(parse_c(source).is_err(), "unexpectedly accepted {source:?}");
    }
}

#[test]
fn routine_level_presence_remains_cumulatively_accepted() {
    for version in [OpenAccVersion::V2_0, OpenAccVersion::V3_4] {
        for source in [
            "#pragma acc routine",
            "#pragma acc routine gang",
            "#pragma acc routine worker",
            "#pragma acc routine vector",
            "#pragma acc routine seq",
        ] {
            parse_c_exact(version, source)
                .unwrap_or_else(|error| panic!("OpenACC {version} rejected {source:?}: {error}"));
        }
    }
}

#[test]
fn num_gangs_has_a_directive_specific_typed_arity() {
    let kernels = parse_c("#pragma acc kernels num_gangs(teams)").unwrap();
    assert!(matches!(
        kernels.directive().clauses()[0].payload(),
        AccClausePayload::NumGangs(values) if values.len() == 1
    ));
    assert!(parse_c("#pragma acc kernels num_gangs(x, y)").is_err());

    let parallel = parse_c("#pragma acc parallel num_gangs(x, y, z)").unwrap();
    assert!(matches!(
        parallel.directive().clauses()[0].payload(),
        AccClausePayload::NumGangs(values) if values.len() == 3
    ));
    assert!(parse_c("#pragma acc parallel num_gangs(a,,b)").is_err());
    assert!(parse_c("#pragma acc parallel num_gangs(a,b,c,d)").is_err());
}

#[test]
fn device_type_wildcard_is_not_the_identifier_any() {
    let wildcard = parse_c("#pragma acc parallel device_type(*)").unwrap();
    assert!(matches!(
        wildcard.directive().clauses()[0].payload(),
        AccClausePayload::DeviceType(values)
            if matches!(values.as_slice(), [AccDeviceType::Wildcard])
    ));
    assert!(parse_c("#pragma acc parallel device_type(*, gpu)").is_err());

    let named = parse_c("#pragma acc parallel device_type(any)").unwrap();
    assert!(matches!(
        named.directive().clauses()[0].payload(),
        AccClausePayload::DeviceType(values)
            if matches!(values.as_slice(), [AccDeviceType::Named(name)] if name.as_str() == "any")
    ));
}

#[test]
fn reductions_are_closed_and_host_language_specific() {
    let c = parse_c("#pragma acc parallel reduction(&: value)").unwrap();
    assert!(matches!(
        c.directive().clauses()[0].payload(),
        AccClausePayload::Reduction(value)
            if value.operator() == &AccReductionOperator::BitAnd
    ));
    for source in [
        "#pragma acc parallel reduction(-: value)",
        "#pragma acc parallel reduction(.and.: value)",
        "#pragma acc parallel reduction(iand: value)",
        "#pragma acc parallel reduction(custom: value)",
    ] {
        assert!(parse_c(source).is_err(), "unexpectedly accepted {source:?}");
    }

    let fortran = parse_fortran("!$acc parallel reduction(.and.: value)").unwrap();
    assert!(matches!(
        fortran.directive().clauses()[0].payload(),
        AccClausePayload::Reduction(value)
            if value.operator() == &AccReductionOperator::FortAnd
    ));
    assert!(parse_fortran("!$acc parallel reduction(&&: value)").is_err());
    assert!(parse_fortran("!$acc parallel reduction(-: value)").is_err());
}

#[test]
fn scalar_expression_clauses_reject_only_top_level_comma_lists() {
    assert!(parse_c("#pragma acc parallel if(a, b)").is_err());
    parse_c("#pragma acc parallel if((a, b))").unwrap();
    parse_c("#pragma acc parallel num_workers(select(a, b))").unwrap();
}

#[test]
fn tile_rejects_obviously_invalid_sizes() {
    for source in [
        "#pragma acc loop tile(0)",
        "#pragma acc loop tile(-1)",
        "#pragma acc loop tile(1.5)",
    ] {
        assert!(parse_c(source).is_err(), "unexpectedly accepted {source:?}");
    }
    parse_c("#pragma acc loop tile(*, tile_size)").unwrap();
}

#[test]
fn bind_is_a_closed_name_or_string_literal_target() {
    let named = parse_c("#pragma acc routine bind(Device_Entry)").expect("name bind must parse");
    assert!(matches!(
        named.directive().clauses()[0].payload(),
        AccClausePayload::Bind(AccBindTarget::Name(name)) if name.as_str() == "Device_Entry"
    ));

    let string =
        parse_c("#pragma acc routine bind(\"device_entry\")").expect("string bind must parse");
    assert!(matches!(
        string.directive().clauses()[0].payload(),
        AccClausePayload::Bind(AccBindTarget::StringLiteral(literal))
            if literal.value == "device_entry"
    ));

    let fortran = parse_fortran("!$acc routine BIND(DEVICE_ENTRY)")
        .expect("Fortran bind keyword and name must parse case-insensitively");
    assert!(matches!(
        fortran.directive().clauses()[0].payload(),
        AccClausePayload::Bind(AccBindTarget::Name(name)) if name.as_str() == "device_entry"
    ));

    for source in [
        "#pragma acc routine BIND(device_entry)",
        "#pragma acc routine bind(a + b)",
        "#pragma acc routine bind(make_name())",
        "#pragma acc routine bind(42)",
        "#pragma acc routine bind('x')",
        "#pragma acc routine bind(first, second)",
    ] {
        assert!(parse_c(source).is_err(), "unexpectedly accepted {source:?}");
    }
}
