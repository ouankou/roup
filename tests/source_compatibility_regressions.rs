use roup::api::{OpenAccConfig, OpenMpConfig};
use roup::ast::{
    AccClauseKind, AccClausePayload, AccGangArgument, AccSizeExpression, OmpClauseKind,
    OmpDirectiveKind, OmpDirectiveParameter, OmpSelectorDeviceTrait, OmpSelectorEntry,
    OmpxPayloadItem,
};
use roup::diagnostic::DiagnosticCode;
use roup::host::{
    BinaryOp, ExprKind, LegacyFortranUnaryOp, Literal, MemberAccess, SizeofOperand, UnaryOp,
};
use roup::ir::{
    ClauseData, ClauseItem, Expression, OmpArrayShapingSubscript, OmpDependence, OmpDistDataPolicy,
    OmpLocator, OriginalSharing, ReductionModifier,
};
use roup::validation::{
    AccClauseSite, AccExpressionSite, IntegerEvaluation, OmpClauseSite, OmpExpressionSite,
    SemanticFacts,
};
use roup::version::{
    CStandard, CppStandard, FortranStandard, HostLanguageProfile, OpenAccVersion, OpenMpVersion,
    SourceForm,
};

fn c() -> roup::api::OpenMpParser {
    OpenMpConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .expect("valid C parser configuration")
        .with_ompparser_extensions()
        .parser()
}

fn cpp() -> roup::api::OpenMpParser {
    OpenMpConfig::new(
        HostLanguageProfile::Cpp(CppStandard::Cpp23),
        SourceForm::Pragma,
    )
    .expect("valid C++ parser configuration")
    .with_ompparser_extensions()
    .parser()
}

fn strict_cpp() -> roup::api::OpenMpParser {
    OpenMpConfig::new(
        HostLanguageProfile::Cpp(CppStandard::Cpp23),
        SourceForm::Pragma,
    )
    .expect("valid strict C++ parser configuration")
    .parser()
}

fn assert_cpp_this_member(expression: &Expression, expected_member: &str) {
    let ExprKind::Member {
        base,
        access,
        member,
    } = &expression.ast().kind
    else {
        panic!("expected a typed C++ member expression");
    };
    assert_eq!(*access, MemberAccess::Arrow);
    assert_eq!(member.as_str(), expected_member);
    assert!(matches!(base.kind, ExprKind::This));
}

fn fortran() -> roup::api::OpenMpParser {
    OpenMpConfig::new(
        HostLanguageProfile::Fortran(FortranStandard::Fortran2023),
        SourceForm::FortranFree,
    )
    .expect("valid Fortran parser configuration")
    .with_ompparser_extensions()
    .parser()
}

fn acc() -> roup::api::OpenAccParser {
    OpenAccConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .expect("valid OpenACC parser configuration")
        .with_accparser_extensions()
        .parser()
}

fn acc_cpp() -> roup::api::OpenAccParser {
    OpenAccConfig::new(
        HostLanguageProfile::Cpp(CppStandard::Cpp23),
        SourceForm::Pragma,
    )
    .expect("valid C++ OpenACC parser configuration")
    .with_accparser_extensions()
    .parser()
}

fn strict_acc_cpp() -> roup::api::OpenAccParser {
    OpenAccConfig::new(
        HostLanguageProfile::Cpp(CppStandard::Cpp23),
        SourceForm::Pragma,
    )
    .expect("valid strict C++ OpenACC parser configuration")
    .parser()
}

fn c_exact(version: OpenMpVersion) -> roup::api::OpenMpParser {
    OpenMpConfig::exact(
        version,
        HostLanguageProfile::C(CStandard::C23),
        SourceForm::Pragma,
    )
    .expect("valid exact OpenMP compatibility configuration")
    .with_ompparser_extensions()
    .parser()
}

fn acc_exact(version: OpenAccVersion) -> roup::api::OpenAccParser {
    OpenAccConfig::exact(
        version,
        HostLanguageProfile::C(CStandard::C23),
        SourceForm::Pragma,
    )
    .expect("valid exact OpenACC compatibility configuration")
    .with_accparser_extensions()
    .parser()
}

#[test]
fn source_compatibility_preserves_explicit_semantic_fact_validation() {
    let omp_source = "#pragma omp parallel num_threads(requested_threads)";
    let omp_site = OmpExpressionSite::new(OmpClauseSite::new(OmpClauseKind::NumThreads, 0), 0);
    c().parse(omp_source)
        .expect("compatibility parsing without facts remains syntax-only");
    assert_eq!(
        c().parse_with_facts(omp_source, &SemanticFacts::new())
            .unwrap_err()
            .code(),
        DiagnosticCode::MissingSemanticFact
    );
    let invalid = SemanticFacts::new().with_positive_integer_expression(omp_site, false);
    assert_eq!(
        c().parse_with_facts(omp_source, &invalid)
            .unwrap_err()
            .code(),
        DiagnosticCode::InvalidExpressionType
    );
    let valid = SemanticFacts::new().with_positive_integer_expression(omp_site, true);
    c().parse_with_facts(omp_source, &valid)
        .expect("compatibility mode must honor valid OpenMP facts");

    let acc_source = "#pragma acc loop tile(*, tile_width)";
    let acc_site = AccExpressionSite::new(AccClauseSite::new(AccClauseKind::Tile, 0), 1);
    acc()
        .parse(acc_source)
        .expect("compatibility parsing without facts remains syntax-only");
    assert_eq!(
        acc()
            .parse_with_facts(acc_source, &SemanticFacts::new())
            .unwrap_err()
            .code(),
        DiagnosticCode::MissingSemanticFact
    );
    let valid = SemanticFacts::new()
        .with_acc_integer_evaluation(acc_site, IntegerEvaluation::NonNegative(16));
    acc()
        .parse_with_facts(acc_source, &valid)
        .expect("compatibility mode must honor valid OpenACC facts");
}

#[test]
fn standard_cpp_scalar_expressions_use_real_this_nodes() {
    for source in [
        "#pragma omp parallel if(this->ready)",
        "#pragma omp parallel num_threads(this->n)",
        "#pragma omp target device(this->device_id)",
        "#pragma omp for schedule(static, this->chunk)",
    ] {
        strict_cpp()
            .parse(source)
            .expect("standard C++ keyword expressions must parse without extensions");
    }

    let parallel = cpp()
        .parse("#pragma omp parallel if(this->ready) num_threads(this->n)")
        .expect("source-compatible scalar expressions must accept C++ host keywords");
    let ClauseData::If { condition } = parallel.directive().clauses()[0].payload() else {
        panic!("expected a typed if clause");
    };
    assert_cpp_this_member(condition, "ready");
    let ClauseData::NumThreads { nthreads, .. } = parallel.directive().clauses()[1].payload()
    else {
        panic!("expected a typed num_threads clause");
    };
    let [nthreads] = nthreads.as_slice() else {
        panic!("expected one typed num_threads expression");
    };
    assert_cpp_this_member(nthreads, "n");

    let target = cpp()
        .parse("#pragma omp target device(this->device_id)")
        .expect("source-compatible device expressions must accept C++ host keywords");
    let ClauseData::Device { device_num, .. } = target.directive().clauses()[0].payload() else {
        panic!("expected a typed device clause");
    };
    assert_cpp_this_member(device_num, "device_id");

    let scheduled = cpp()
        .parse("#pragma omp for schedule(static, this->chunk)")
        .expect("source-compatible schedule chunks must accept C++ host keywords");
    let ClauseData::Schedule {
        chunk_size: Some(chunk_size),
        ..
    } = scheduled.directive().clauses()[0].payload()
    else {
        panic!("expected a typed schedule chunk expression");
    };
    assert_cpp_this_member(chunk_size, "chunk");
}

#[test]
fn cpp_compatibility_preserves_standard_qualified_clause_items() {
    let parsed = cpp()
        .parse("#pragma omp parallel private(ns::x, zero::12)")
        .expect("C++ compatibility parsing must retain standard and legacy qualification");
    let ClauseData::Private { items } = parsed.directive().clauses()[0].payload() else {
        panic!("expected a typed private clause");
    };
    let [
        ClauseItem::Variable(standard),
        ClauseItem::Expression(legacy),
    ] = items.as_slice()
    else {
        panic!("expected one standard variable and one legacy expression");
    };
    let ExprKind::Name(name) = &standard.ast().kind else {
        panic!("standard C++ qualification must remain a qualified-name node");
    };
    assert!(!name.global);
    assert!(
        name.segments
            .iter()
            .map(|segment| segment.as_str())
            .eq(["ns", "x"])
    );
    assert!(matches!(
        &legacy.ast().kind,
        ExprKind::LegacyQualifiedInteger { qualifier, value }
            if qualifier.as_str() == "zero" && value.value == 12
    ));
}

#[test]
fn openacc_source_compatibility_applies_to_expression_payloads() {
    acc()
        .parse("#pragma acc parallel if(ns::ready)")
        .expect_err("source compatibility must not enable C++ scope syntax in C");

    let data = acc()
        .parse("#pragma acc data copyin(readonly: readonly::m) copyout(zero: zero::12)")
        .expect("legacy qualified values remain confined to compatibility clause-item lists");
    let AccClausePayload::Copy(copyin) = data.directive().clauses()[0].payload() else {
        panic!("expected a typed OpenACC copyin clause");
    };
    assert!(matches!(
        copyin.variables(),
        [ClauseItem::Variable(variable)]
            if matches!(
                &variable.ast().kind,
                ExprKind::LegacyQualifiedName { segments }
                    if segments.iter().map(|segment| segment.as_str()).eq(["readonly", "m"])
            )
    ));
    let AccClausePayload::Copy(copyout) = data.directive().clauses()[1].payload() else {
        panic!("expected a typed OpenACC copyout clause");
    };
    assert!(matches!(
        copyout.variables(),
        [ClauseItem::Expression(expression)]
            if matches!(
                &expression.ast().kind,
                ExprKind::LegacyQualifiedInteger { qualifier, value }
                    if qualifier.as_str() == "zero" && value.value == 12
            )
    ));

    let waited = acc()
        .parse("#pragma acc enter data wait(devnum: devnum::devnum::z: queues: queues::a)")
        .expect("legacy qualified values remain supported in compatibility wait arguments");
    let AccClausePayload::Wait(wait) = waited.directive().clauses()[0].payload() else {
        panic!("expected a typed OpenACC wait clause");
    };
    assert!(matches!(
        wait.devnum(),
        Some(expression)
            if matches!(
                &expression.ast().kind,
                ExprKind::LegacyQualifiedName { segments }
                    if segments.iter().map(|segment| segment.as_str()).eq(["devnum", "devnum", "z"])
            )
    ));
    assert!(matches!(
        wait.queues(),
        [expression]
            if matches!(
                &expression.ast().kind,
                ExprKind::LegacyQualifiedName { segments }
                    if segments.iter().map(|segment| segment.as_str()).eq(["queues", "a"])
            )
    ));

    for source in [
        "#pragma acc parallel if(this->ready)",
        "#pragma acc parallel num_gangs(this->gangs)",
        "#pragma acc loop tile(this->tile_size)",
        "#pragma acc loop gang(num: this->gangs)",
    ] {
        strict_acc_cpp()
            .parse(source)
            .expect("standard C++ keyword expressions must parse without extensions");
    }

    let parallel = acc_cpp()
        .parse("#pragma acc parallel if(this->ready) num_gangs(this->gangs)")
        .expect("source-compatible OpenACC scalar and expression-list clauses must parse");
    let AccClausePayload::Expression {
        value: condition, ..
    } = parallel.directive().clauses()[0].payload()
    else {
        panic!("expected a typed OpenACC if expression");
    };
    assert_cpp_this_member(condition, "ready");
    let AccClausePayload::NumGangs(values) = parallel.directive().clauses()[1].payload() else {
        panic!("expected a typed OpenACC num_gangs expression list");
    };
    let [gangs] = values.as_slice() else {
        panic!("expected one num_gangs expression");
    };
    assert_cpp_this_member(gangs, "gangs");

    let tiled = acc_cpp()
        .parse("#pragma acc loop tile(this->tile_size)")
        .expect("source-compatible OpenACC tile expressions must parse");
    let AccClausePayload::Tile(sizes) = tiled.directive().clauses()[0].payload() else {
        panic!("expected a typed OpenACC tile list");
    };
    let [AccSizeExpression::Expression(size)] = sizes.as_slice() else {
        panic!("expected one typed OpenACC tile expression");
    };
    assert_cpp_this_member(size, "tile_size");

    let gang = acc_cpp()
        .parse("#pragma acc loop gang(num: this->gangs)")
        .expect("source-compatible OpenACC gang expressions must parse");
    let AccClausePayload::Gang(gang) = gang.directive().clauses()[0].payload() else {
        panic!("expected a typed OpenACC gang clause");
    };
    let [AccGangArgument::Num(gangs)] = gang.arguments() else {
        panic!("expected one typed OpenACC gang num expression");
    };
    assert_cpp_this_member(gangs, "gangs");
}

#[test]
fn standard_cpp_selector_expressions_use_typed_keyword_nodes() {
    for source in [
        "#pragma omp metadirective when(implementation={vendor(score(sizeof(int)): llvm)}: parallel)",
        "#pragma omp metadirective when(target_device={device_num(this->dev)}: parallel)",
    ] {
        strict_cpp()
            .parse(source)
            .expect("standard C++ selector expressions must parse without extensions");
    }

    let parsed = cpp()
        .parse("#pragma omp metadirective when(device={kind(score(sizeof(int)): gpu)}: parallel)")
        .expect("source-compatible selector scores must accept C++ host keywords");
    let ClauseData::MetadirectiveSelector { selector, .. } =
        parsed.directive().clauses()[0].payload()
    else {
        panic!("expected a typed metadirective selector");
    };
    let OmpSelectorEntry::Device { traits } = &selector.entries()[0] else {
        panic!("expected a typed device selector");
    };
    let OmpSelectorDeviceTrait::NameList(kind) = &traits[0] else {
        panic!("expected a typed kind name-list selector");
    };
    let score = kind.score().expect("expected a typed selector score");
    assert!(matches!(
        &score.ast().kind,
        ExprKind::Sizeof(SizeofOperand::Type(_))
    ));

    let target = cpp()
        .parse("#pragma omp metadirective when(target_device={device_num(this->dev)}: parallel)")
        .expect("source-compatible target-device expressions must accept C++ host keywords");
    let ClauseData::MetadirectiveSelector { selector, .. } =
        target.directive().clauses()[0].payload()
    else {
        panic!("expected a typed target-device selector");
    };
    let OmpSelectorEntry::TargetDevice { traits } = &selector.entries()[0] else {
        panic!("expected a typed target_device selector");
    };
    let OmpSelectorDeviceTrait::DeviceNum(device_num) = &traits[0] else {
        panic!("expected a typed device_num selector expression");
    };
    assert_cpp_this_member(device_num, "dev");
}

#[test]
fn fortran_dist_data_suffix_is_case_insensitive_and_typed() {
    c().parse("#pragma omp target update from(a DIST_DATA(DUPLICATE))")
        .expect_err("C compatibility payload keywords must remain case-sensitive");

    for source in [
        "!$omp target update from(a dist_data(duplicate))",
        "!$OMP TARGET UPDATE FROM(a DIST_DATA(DUPLICATE))",
    ] {
        let parsed = fortran()
            .parse(source)
            .expect("Fortran compatibility payload keywords must be case-insensitive");
        let ClauseData::From { locators, .. } = parsed.directive().clauses()[0].payload() else {
            panic!("expected a typed from clause");
        };
        assert!(matches!(
            locators.as_slice(),
            [OmpLocator::Distributed { base, policy }]
                if base.source() == "a" && *policy == OmpDistDataPolicy::Duplicate
        ));
    }
}

#[test]
fn fortran_literal_brackets_do_not_select_legacy_designator_grammar() {
    let parsed = fortran()
        .parse("!$omp parallel if(index(s, '[') > 0 .and. flag)")
        .expect("a bracket inside a Fortran character literal is data, not designator syntax");
    let ClauseData::If { condition } = parsed.directive().clauses()[0].payload() else {
        panic!("expected a typed if clause");
    };
    assert!(matches!(
        condition.ast().kind,
        ExprKind::Binary {
            op: BinaryOp::LogicalAnd,
            ..
        }
    ));

    let mapped = fortran()
        .parse("!$omp target map(to: values[0])")
        .expect("a real legacy bracket designator must remain supported");
    let ClauseData::Map { locators, .. } = mapped.directive().clauses()[0].payload() else {
        panic!("expected a typed map clause");
    };
    let expression = match locators.as_slice() {
        [OmpLocator::LValue(value)] => value.expression(),
        [OmpLocator::PotentialLValue(expression)] => expression,
        _ => panic!("expected one typed legacy bracket locator"),
    };
    assert!(matches!(
        expression.ast().kind,
        ExprKind::LegacyFortranSubscript { .. }
    ));

    let pointer = fortran()
        .parse("!$omp target map(to: *ptr)")
        .expect("the named extension dialect retains typed pointer designators");
    let ClauseData::Map { locators, .. } = pointer.directive().clauses()[0].payload() else {
        panic!("expected a typed map clause");
    };
    assert!(matches!(
        locators.as_slice(),
        [OmpLocator::LValue(value)]
            if matches!(value.ast().kind, ExprKind::LegacyFortranUnaryDesignator { .. })
    ));
}

#[test]
fn fortran_pointer_and_bracket_designator_extensions_compose() {
    for (source, expected_op) in [
        (
            "!$omp target map(to: *values[0])",
            LegacyFortranUnaryOp::Dereference,
        ),
        (
            "!$omp target map(to: &values[1:count])",
            LegacyFortranUnaryOp::AddressOf,
        ),
    ] {
        let parsed = fortran()
            .parse(source)
            .expect("legacy pointer and bracket designators must compose");
        let ClauseData::Map { locators, .. } = parsed.directive().clauses()[0].payload() else {
            panic!("expected a typed map clause");
        };
        let expression = match locators.as_slice() {
            [OmpLocator::LValue(value)] => value.expression(),
            [OmpLocator::PotentialLValue(expression)] => expression,
            _ => panic!("expected one typed legacy pointer-subscript locator"),
        };
        let ExprKind::LegacyFortranUnaryDesignator { op, operand } = &expression.ast().kind else {
            panic!("expected a typed legacy pointer designator");
        };
        assert_eq!(*op, expected_op);
        assert!(matches!(
            operand.kind,
            ExprKind::LegacyFortranSubscript { .. }
        ));
    }
}

#[test]
fn source_compatibility_preserves_exact_version_ceilings() {
    let omp_three_one = c_exact(OpenMpVersion::V3_1);
    let error = omp_three_one
        .parse("#pragma omp target")
        .expect_err("source compatibility must not make OpenMP 4.0 syntax available in 3.1");
    assert_eq!(error.code(), DiagnosticCode::NotAvailableInVersion);

    let omp_four = c_exact(OpenMpVersion::V4_0);
    omp_four
        .parse("#pragma omp target")
        .expect("OpenMP 4.0 introduced target");
    let error = omp_four
        .parse("#pragma omp target defaultmap(tofrom: scalar)")
        .expect_err("source compatibility must retain the OpenMP 4.5 clause ceiling");
    assert_eq!(error.code(), DiagnosticCode::NotAvailableInVersion);
    let versioned = c()
        .parse("#pragma omp target defaultmap(tofrom: scalar)")
        .expect("the union policy accepts standardized OpenMP 4.5 syntax");
    assert!(
        !versioned
            .compatible_versions()
            .contains(OpenMpVersion::V4_0)
    );
    assert!(
        versioned
            .compatible_versions()
            .contains(OpenMpVersion::V4_5)
    );

    let omp_six = c_exact(OpenMpVersion::V6_0);
    omp_six
        .parse("#pragma omp master")
        .expect("a ceiling must not reject maintained historical syntax");

    let acc_two_five = acc_exact(OpenAccVersion::V2_5);
    let error = acc_two_five
        .parse("#pragma acc serial")
        .expect_err("source compatibility must not make OpenACC 2.6 syntax available in 2.5");
    assert_eq!(error.code(), DiagnosticCode::NotAvailableInVersion);

    let acc_two_six = acc_exact(OpenAccVersion::V2_6);
    acc_two_six
        .parse("#pragma acc serial")
        .expect("OpenACC 2.6 introduced serial");
    let error = acc_two_six
        .parse("#pragma acc data no_create(a)")
        .expect_err("source compatibility must retain the OpenACC 2.7 clause ceiling");
    assert_eq!(error.code(), DiagnosticCode::NotAvailableInVersion);
    let versioned = acc()
        .parse("#pragma acc data no_create(a)")
        .expect("the union policy accepts standardized OpenACC 2.7 syntax");
    assert!(
        !versioned
            .compatible_versions()
            .contains(OpenAccVersion::V2_6)
    );
    assert!(
        versioned
            .compatible_versions()
            .contains(OpenAccVersion::V2_7)
    );
}

#[test]
fn source_compatibility_union_policy_retains_parser_extensions() {
    let parsed = c()
        .parse("#pragma omp target teams workdistribute collapse(2)")
        .expect("the ompparser compatibility contract accepts its C workdistribute extension");
    assert_eq!(
        parsed.directive().kind(),
        OmpDirectiveKind::TargetTeamsWorkdistribute
    );
    assert!(
        parsed.compatible_versions().is_empty(),
        "a C-only parser extension must not claim standardized OpenMP compatibility"
    );

    for source in ["!$ompx vendor payload", "!$omp end section"] {
        let parsed = fortran()
            .parse(source)
            .expect("an ompparser directive extension must remain accepted");
        assert!(
            parsed.compatible_versions().is_empty(),
            "a nonstandard directive must not claim standardized OpenMP compatibility: {source}"
        );
    }

    let parsed = acc()
        .parse("#pragma acc routine indirect")
        .expect("the accparser indirect extension must remain accepted");
    assert!(
        parsed.compatible_versions().is_empty(),
        "a nonstandard clause must clear the base directive's standardized compatibility"
    );
}

#[test]
fn exact_version_policy_rejects_nonstandard_compatibility_extensions() {
    let omp_one = OpenMpConfig::exact(
        OpenMpVersion::V1_0,
        HostLanguageProfile::Fortran(FortranStandard::Fortran2023),
        SourceForm::FortranFree,
    )
    .expect("valid exact OpenMP compatibility configuration")
    .with_ompparser_extensions()
    .parser();
    for source in ["!$ompx vendor payload", "!$omp end section"] {
        let error = omp_one
            .parse(source)
            .expect_err("nonstandard syntax has no exact OpenMP version");
        assert_eq!(
            error.code(),
            DiagnosticCode::NotAvailableInVersion,
            "{source}"
        );
    }

    let error = acc_exact(OpenAccVersion::V2_0)
        .parse("#pragma acc routine indirect")
        .expect_err("a nonstandard clause has no exact OpenACC version");
    assert_eq!(error.code(), DiagnosticCode::NotAvailableInVersion);
}

#[test]
fn ompx_payload_is_not_retained_as_an_opaque_string() {
    let parsed = fortran()
        .parse("!$ompx vendor Name(\"GPU0\")")
        .expect("typed OMPX payload must parse");
    assert_eq!(parsed.directive().kind(), OmpDirectiveKind::Ompx);
    let Some(OmpDirectiveParameter::Ompx(payload)) = parsed.directive().parameter() else {
        panic!("expected a typed OMPX parameter");
    };
    assert!(matches!(
        payload.items(),
        [
            OmpxPayloadItem::Identifier(vendor),
            OmpxPayloadItem::Invocation { name, arguments }
        ] if vendor.as_str() == "vendor"
            && name.as_str() == "Name"
            && matches!(arguments.as_slice(), [argument]
                if matches!(argument.ast().kind,
                    ExprKind::Literal(Literal::String(_))))
    ));
}

#[test]
fn array_shaping_locator_has_typed_dimensions_and_subscripts() {
    let parsed = c()
        .parse("#pragma omp target update from((([nx][ny+2])a)[0:nx][1])")
        .expect("OpenMP array shaping must parse in source compatibility mode");
    let ClauseData::From { locators, .. } = parsed.directive().clauses()[0].payload() else {
        panic!("expected a typed from clause");
    };
    let [
        OmpLocator::ArrayShaping {
            dimensions,
            base,
            subscripts,
        },
    ] = locators.as_slice()
    else {
        panic!("expected one typed array-shaping locator");
    };
    assert_eq!(dimensions.len(), 2);
    assert_eq!(dimensions[0].compact_source_spelling(), "nx");
    assert_eq!(dimensions[1].compact_source_spelling(), "ny+2");
    assert_eq!(base.compact_source_spelling(), "a");
    assert!(matches!(
        subscripts.as_slice(),
        [
            OmpArrayShapingSubscript::Section {
                lower: Some(_),
                length: Some(_),
                stride: None,
            },
            OmpArrayShapingSubscript::Index(_),
        ]
    ));
}

#[test]
fn dereferenced_depobj_is_a_checked_lvalue() {
    let parsed = c()
        .parse("#pragma omp task depend(depobj: *obj)")
        .expect("dereferenced depend object must parse");
    let ClauseData::Depend { dependence, .. } = parsed.directive().clauses()[0].payload() else {
        panic!("expected a typed depend clause");
    };
    let OmpDependence::Depobjs { objects } = dependence else {
        panic!("expected typed depobj dependence");
    };
    assert!(matches!(
        objects.as_slice(),
        [object] if matches!(object.ast().kind,
            ExprKind::Unary { op: UnaryOp::Dereference, .. })
    ));
}

#[test]
fn historical_original_reduction_modifier_keeps_typed_sharing() {
    let parsed = c()
        .parse("#pragma omp for reduction(original(private),+: sum_v)")
        .expect("historical original(private) modifier must parse");
    let ClauseData::Reduction { modifiers, .. } = parsed.directive().clauses()[0].payload() else {
        panic!("expected a typed reduction clause");
    };
    assert_eq!(
        modifiers.as_slice(),
        [ReductionModifier::Original(OriginalSharing::Private)]
    );
}

#[test]
fn compatibility_keyword_locator_and_redundant_fortran_omp_remain_typed() {
    let mapped = cpp()
        .parse("#pragma omp target map(this->values[0:count])")
        .expect("C++ this locator must parse");
    let ClauseData::Map { locators, .. } = mapped.directive().clauses()[0].payload() else {
        panic!("expected a typed map clause");
    };
    assert!(matches!(
        locators.as_slice(),
        [OmpLocator::LValue(_) | OmpLocator::PotentialLValue(_)]
    ));

    let teams = fortran()
        .parse("!$omp omp teams num_teams(4)")
        .expect("historical redundant omp token must parse");
    assert_eq!(teams.directive().kind(), OmpDirectiveKind::Teams);
}

#[test]
fn ompparser_map_expression_extensions_remain_typed_and_fact_checked() {
    for source in [
        "#pragma omp target map(to: obj.dist_data(x))",
        "#pragma omp target map(to: foo(a, dist_data(b)))",
        "#pragma omp target map(to: foo + dist_data(duplicate))",
    ] {
        let parsed = c()
            .parse(source)
            .expect("ompparser map-expression extensions must remain typed");
        let ClauseData::Map { locators, .. } = parsed.directive().clauses()[0].payload() else {
            panic!("expected a typed map clause");
        };
        assert!(matches!(
            locators.as_slice(),
            [OmpLocator::PotentialLValue(_)]
        ));
        assert_eq!(
            c().parse_with_facts(source, &SemanticFacts::new())
                .unwrap_err()
                .code(),
            DiagnosticCode::MissingSemanticFact
        );
    }

    let strict = OpenMpConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .expect("valid strict OpenMP configuration")
        .parser();
    assert!(
        strict
            .parse("#pragma omp target map(to: foo + dist_data(duplicate))")
            .is_err()
    );
}

#[test]
fn ompparser_firstprivate_modifier_order_is_typed() {
    c().parse(
        "#pragma omp target teams distribute firstprivate(target, saved: e) firstprivate(saved, teams: f) firstprivate(target data: g)",
    )
    .expect("ompparser firstprivate modifier order must parse into typed clauses");
}
