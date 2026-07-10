use roup::api::{OpenAccConfig, ParsedOpenAccDirective};
use roup::ast::{
    AccClauseKind, AccClausePayload, AccDataModifier, AccDirectiveKind, AccGangArgument,
    AccReductionOperator, AccSizeExpression, AccVectorModifier,
};
use roup::diagnostic::Diagnostic;
use roup::version::{CStandard, HostLanguageProfile, SourceForm};

fn parse(source: &str) -> Result<ParsedOpenAccDirective, Diagnostic> {
    OpenAccConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .expect("valid C configuration")
        .parser()
        .parse(source)
}

#[test]
fn standardized_directive_families_build_typed_ast() {
    let cases = [
        ("#pragma acc parallel", AccDirectiveKind::Parallel),
        ("#pragma acc serial", AccDirectiveKind::Serial),
        ("#pragma acc kernels", AccDirectiveKind::Kernels),
        ("#pragma acc data copy(a)", AccDirectiveKind::Data),
        (
            "#pragma acc enter data copyin(a)",
            AccDirectiveKind::EnterData,
        ),
        (
            "#pragma acc exit data delete(a)",
            AccDirectiveKind::ExitData,
        ),
        (
            "#pragma acc host_data use_device(ptr)",
            AccDirectiveKind::HostData,
        ),
        ("#pragma acc loop gang", AccDirectiveKind::Loop),
        (
            "#pragma acc kernels loop independent",
            AccDirectiveKind::KernelsLoop,
        ),
        (
            "#pragma acc parallel loop gang",
            AccDirectiveKind::ParallelLoop,
        ),
        ("#pragma acc serial loop seq", AccDirectiveKind::SerialLoop),
        ("#pragma acc atomic update", AccDirectiveKind::Atomic),
        ("#pragma acc cache(arr[0])", AccDirectiveKind::Cache),
        ("#pragma acc wait(1)", AccDirectiveKind::Wait),
        ("#pragma acc declare create(a)", AccDirectiveKind::Declare),
        ("#pragma acc routine gang", AccDirectiveKind::Routine),
        (
            "#pragma acc init device_type(default)",
            AccDirectiveKind::Init,
        ),
        ("#pragma acc shutdown", AccDirectiveKind::Shutdown),
        ("#pragma acc set device_num(0)", AccDirectiveKind::Set),
        ("#pragma acc update host(a)", AccDirectiveKind::Update),
    ];

    for (source, expected) in cases {
        let parsed = parse(source).unwrap_or_else(|error| panic!("rejected {source:?}: {error}"));
        assert_eq!(parsed.directive().kind(), expected, "{source}");
    }
}

#[test]
fn standardized_clause_families_build_expected_typed_kinds() {
    let cases = [
        ("#pragma acc parallel async", AccClauseKind::Async),
        ("#pragma acc parallel wait(1)", AccClauseKind::Wait),
        ("#pragma acc parallel num_gangs(4)", AccClauseKind::NumGangs),
        (
            "#pragma acc parallel num_workers(8)",
            AccClauseKind::NumWorkers,
        ),
        (
            "#pragma acc parallel vector_length(128)",
            AccClauseKind::VectorLength,
        ),
        ("#pragma acc loop gang", AccClauseKind::Gang),
        ("#pragma acc loop worker", AccClauseKind::Worker),
        ("#pragma acc loop vector", AccClauseKind::Vector),
        ("#pragma acc loop seq", AccClauseKind::Seq),
        ("#pragma acc loop independent", AccClauseKind::Independent),
        ("#pragma acc loop auto", AccClauseKind::Auto),
        ("#pragma acc loop collapse(2)", AccClauseKind::Collapse),
        ("#pragma acc loop tile(8)", AccClauseKind::Tile),
        (
            "#pragma acc loop device_type(nvidia)",
            AccClauseKind::DeviceType,
        ),
        ("#pragma acc parallel if(condition)", AccClauseKind::If),
        (
            "#pragma acc parallel default(present)",
            AccClauseKind::Default,
        ),
        (
            "#pragma acc parallel firstprivate(x)",
            AccClauseKind::Firstprivate,
        ),
        (
            "#pragma acc parallel reduction(+:sum)",
            AccClauseKind::Reduction,
        ),
        ("#pragma acc parallel self", AccClauseKind::SelfClause),
        ("#pragma acc routine bind(foo)", AccClauseKind::Bind),
        ("#pragma acc routine nohost", AccClauseKind::NoHost),
        (
            "#pragma acc set default_async(queue0)",
            AccClauseKind::DefaultAsync,
        ),
        ("#pragma acc set device_num(1)", AccClauseKind::DeviceNum),
        ("#pragma acc data copy(a)", AccClauseKind::Copy),
        ("#pragma acc data copyin(a)", AccClauseKind::CopyIn),
        ("#pragma acc data copyout(a)", AccClauseKind::CopyOut),
        ("#pragma acc data create(a)", AccClauseKind::Create),
        ("#pragma acc exit data delete(a)", AccClauseKind::Delete),
        ("#pragma acc data no_create(a)", AccClauseKind::NoCreate),
        ("#pragma acc data present(a)", AccClauseKind::Present),
        ("#pragma acc parallel private(a)", AccClauseKind::Private),
        ("#pragma acc declare link(a)", AccClauseKind::Link),
        (
            "#pragma acc parallel deviceptr(ptr)",
            AccClauseKind::DevicePtr,
        ),
        (
            "#pragma acc declare device_resident(x)",
            AccClauseKind::DeviceResident,
        ),
        ("#pragma acc enter data attach(ptr)", AccClauseKind::Attach),
        ("#pragma acc exit data detach(ptr)", AccClauseKind::Detach),
        ("#pragma acc exit data finalize", AccClauseKind::Finalize),
        ("#pragma acc update host(arr)", AccClauseKind::SelfClause),
        ("#pragma acc update device(arr)", AccClauseKind::Device),
        (
            "#pragma acc host_data use_device(ptr)",
            AccClauseKind::UseDevice,
        ),
        ("#pragma acc host_data if_present", AccClauseKind::IfPresent),
        ("#pragma acc atomic read", AccClauseKind::Read),
        ("#pragma acc atomic write", AccClauseKind::Write),
        ("#pragma acc atomic capture", AccClauseKind::Capture),
        ("#pragma acc atomic update", AccClauseKind::Update),
    ];

    for (source, expected) in cases {
        let parsed = parse(source).unwrap_or_else(|error| panic!("rejected {source:?}: {error}"));
        assert_eq!(
            parsed
                .directive()
                .clauses()
                .last()
                .map(|clause| clause.kind()),
            Some(expected),
            "{source}"
        );
    }
}

#[test]
fn historical_aliases_canonicalize_without_losing_source_spelling() {
    let source = "#pragma acc data pcopy(a) present_or_copyin(b) pcreate(c)";
    let parsed = parse(source).expect("standardized aliases must parse");
    let clauses = parsed.directive().clauses();

    assert_eq!(clauses[0].kind(), AccClauseKind::Copy);
    assert_eq!(clauses[0].span().slice(source), Ok("pcopy"));
    assert_eq!(clauses[1].kind(), AccClauseKind::CopyIn);
    assert_eq!(clauses[1].span().slice(source), Ok("present_or_copyin"));
    assert_eq!(clauses[2].kind(), AccClauseKind::Create);
    assert_eq!(clauses[2].span().slice(source), Ok("pcreate"));
}

#[test]
fn modifiers_are_typed_and_follow_c_case_rules() {
    let parsed = parse("#pragma acc parallel copyin(readonly: a)").unwrap();
    let AccClausePayload::Copy(copy) = parsed.directive().clauses()[0].payload() else {
        panic!("expected copyin payload");
    };
    assert_eq!(copy.modifiers(), &[AccDataModifier::Readonly]);

    let parsed = parse("#pragma acc data copyout(zero: b)").unwrap();
    let AccClausePayload::Copy(copy) = parsed.directive().clauses()[0].payload() else {
        panic!("expected copyout payload");
    };
    assert_eq!(copy.modifiers(), &[AccDataModifier::Zero]);

    let parsed = parse("#pragma acc parallel loop gang(num: 4)").unwrap();
    let AccClausePayload::Gang(gang) = parsed.directive().clauses()[0].payload() else {
        panic!("expected gang payload");
    };
    assert!(matches!(gang.arguments(), [AccGangArgument::Num(_)]));

    let parsed = parse("#pragma acc parallel loop vector(length: 32)").unwrap();
    let AccClausePayload::Vector(vector) = parsed.directive().clauses()[0].payload() else {
        panic!("expected vector payload");
    };
    assert_eq!(vector.modifier(), Some(AccVectorModifier::Length));

    let parsed = parse("#pragma acc loop reduction(min: x)").unwrap();
    let AccClausePayload::Reduction(reduction) = parsed.directive().clauses()[0].payload() else {
        panic!("expected reduction payload");
    };
    assert_eq!(reduction.operator(), &AccReductionOperator::Min);

    for malformed in [
        "#pragma acc parallel copyin(ReadOnly: a)",
        "#pragma acc parallel loop gang(NUM: 4)",
        "#pragma acc loop reduction(MIN: x)",
    ] {
        assert!(parse(malformed).is_err(), "accepted {malformed:?}");
    }
}

#[test]
fn tile_and_static_gang_sizes_preserve_automatic_entries_as_typed_data() {
    let parsed = parse("#pragma acc loop tile(*, 8)").unwrap();
    let AccClausePayload::Tile(sizes) = parsed.directive().clauses()[0].payload() else {
        panic!("expected typed tile payload");
    };
    assert!(sizes[0].is_automatic());
    assert_eq!(sizes[1].expression().unwrap().to_string(), "8");

    let parsed = parse("#pragma acc loop gang(4, dim: 2, static:*)").unwrap();
    let AccClausePayload::Gang(gang) = parsed.directive().clauses()[0].payload() else {
        panic!("expected typed gang payload");
    };
    assert!(matches!(
        gang.arguments()[0],
        AccGangArgument::Positional(_)
    ));
    assert!(matches!(gang.arguments()[1], AccGangArgument::Dim(_)));
    assert_eq!(
        gang.arguments()[2],
        AccGangArgument::Static(AccSizeExpression::Automatic)
    );
}

#[test]
fn malformed_payloads_and_unknown_clauses_are_hard_errors() {
    for malformed in [
        "#pragma acc parallel copy()",
        "#pragma acc parallel reduction(+:)",
        "#pragma acc parallel async()",
        "#pragma acc parallel gang()",
        "#pragma acc parallel typo_clause(value)",
        "#pragma acc host data use_device(ptr)",
        "#pragma acc data copy(a,,b)",
        "#pragma acc loop tile()",
        "#pragma acc loop tile(,8)",
        "#pragma acc loop tile(8,)",
        "#pragma acc loop tile(8], 4)",
        "#pragma acc loop gang(static:)",
        "#pragma acc loop gang(static:*,)",
        "#pragma acc loop gang(num:*)",
        "#pragma acc loop gang(4, num: 8)",
        "#pragma acc loop gang(dim: 1, dim: 2)",
        "#pragma acc loop gang(static: 4, static: *)",
        "#pragma acc loop gang(unknown: 4)",
    ] {
        assert!(parse(malformed).is_err(), "accepted {malformed:?}");
    }
}
