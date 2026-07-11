use roup::api::{OpenMpConfig, ParsedOpenMpDirective};
use roup::ast::{
    OmpClauseKind, OmpSelectorDeviceTrait, OmpSelectorEntry, OmpSelectorExtensionProperty,
    OmpSelectorImplementationTraitKind, OmpSelectorNameListKind, OmpSelectorTraitValue,
};
use roup::diagnostic::{Diagnostic, DiagnosticCode};
use roup::ir::{ClauseData, MemoryOrder, RequireModifier};
use roup::validation::{
    IntegerEvaluation, LogicalEvaluation, OmpClauseSite, OmpExpressionSite, SemanticFacts,
    validate_openmp_with_facts,
};
use roup::version::{CStandard, HostLanguageProfile, OpenMpVersion, SourceForm, VersionPolicy};

fn parse(version: OpenMpVersion, source: &str) -> Result<ParsedOpenMpDirective, Diagnostic> {
    OpenMpConfig::exact(
        version,
        HostLanguageProfile::C(CStandard::C23),
        SourceForm::Pragma,
    )
    .unwrap()
    .parser()
    .parse(source)
}

fn selector(parsed: &ParsedOpenMpDirective) -> &roup::ast::OmpSelector {
    let ClauseData::MetadirectiveSelector { selector } = parsed.directive().clauses()[0].payload()
    else {
        panic!("expected a typed selector payload");
    };
    selector
}

fn expression_site(index: usize) -> OmpExpressionSite {
    OmpExpressionSite::new(OmpClauseSite::new(OmpClauseKind::When, 0), index)
}

#[test]
fn device_and_target_device_are_distinct_sets_with_versioned_traits() {
    let source = "#pragma omp metadirective when(device={kind(cpu)}, target_device={device_num(dev), arch(sm_90)}: parallel)";
    let error = parse(OpenMpVersion::V5_0, source).expect_err("target_device starts in 5.1");
    assert_eq!(error.code(), DiagnosticCode::NotAvailableInVersion);

    let parsed = parse(OpenMpVersion::V5_1, source).expect("5.1 target_device selector");
    assert!(matches!(
        selector(&parsed).entries()[0],
        OmpSelectorEntry::Device { .. }
    ));
    assert!(matches!(
        selector(&parsed).entries()[1],
        OmpSelectorEntry::TargetDevice { .. }
    ));

    let uid = "#pragma omp metadirective when(target_device={uid(\"gpu-0\")}: parallel)";
    assert_eq!(
        parse(OpenMpVersion::V5_2, uid).unwrap_err().code(),
        DiagnosticCode::NotAvailableInVersion
    );
    let parsed = parse(OpenMpVersion::V6_0, uid).expect("uid starts in 6.0");
    let OmpSelectorEntry::TargetDevice { traits } = &selector(&parsed).entries()[0] else {
        panic!("expected target_device");
    };
    assert!(matches!(
        &traits[0],
        OmpSelectorDeviceTrait::Uid(OmpSelectorTraitValue::StringLiteral(literal))
            if literal.value == "gpu-0"
    ));
}

#[test]
fn name_list_traits_group_properties_and_reject_every_duplicate_form() {
    let parsed = parse(
        OpenMpVersion::V6_0,
        "#pragma omp metadirective when(device={kind(cpu, \"gpu\"), isa(avx2, sse4)}, implementation={vendor(llvm, \"gnu\"), extension(cuda, hip)}: parallel)",
    )
    .expect("grouped name-list traits");
    let OmpSelectorEntry::Device { traits } = &selector(&parsed).entries()[0] else {
        panic!("expected device selector");
    };
    let OmpSelectorDeviceTrait::NameList(kind) = &traits[0] else {
        panic!("expected one grouped kind trait");
    };
    assert_eq!(kind.kind(), OmpSelectorNameListKind::Kind);
    assert_eq!(kind.properties().len(), 2);

    for source in [
        "#pragma omp metadirective when(device={kind(cpu), kind(gpu)}: parallel)",
        "#pragma omp metadirective when(device={isa(avx2), isa(sse4)}: parallel)",
        "#pragma omp metadirective when(device={kind(cpu, cpu)}: parallel)",
        "#pragma omp metadirective when(device={kind(cpu, \"cpu\")}: parallel)",
        "#pragma omp metadirective when(device={kind(any), isa(avx2)}: parallel)",
        "#pragma omp metadirective when(construct={parallel, parallel}: parallel)",
    ] {
        assert!(
            parse(OpenMpVersion::V6_0, source).is_err(),
            "accepted {source}"
        );
    }
}

#[test]
fn implementation_scores_require_constant_nonnegative_facts() {
    let source = "#pragma omp metadirective when(implementation={vendor(score(weight): llvm, gnu)}: parallel)";
    let parsed = parse(OpenMpVersion::V6_0, source).expect("typed implementation score");
    let OmpSelectorEntry::Implementation { traits } = &selector(&parsed).entries()[0] else {
        panic!("expected implementation selector");
    };
    assert_eq!(
        traits[0].score().map(ToString::to_string).as_deref(),
        Some("weight")
    );

    let facts = SemanticFacts::new()
        .with_constant_expression(expression_site(0), true)
        .with_integer_evaluation(expression_site(0), IntegerEvaluation::NonNegative(7));
    validate_openmp_with_facts(
        parsed.directive(),
        VersionPolicy::Exact(OpenMpVersion::V6_0),
        parsed.directive().span(),
        &facts,
    )
    .expect("nonnegative constant score facts");

    let negative = SemanticFacts::new()
        .with_constant_expression(expression_site(0), true)
        .with_integer_evaluation(expression_site(0), IntegerEvaluation::Negative);
    assert_eq!(
        validate_openmp_with_facts(
            parsed.directive(),
            VersionPolicy::Exact(OpenMpVersion::V6_0),
            parsed.directive().span(),
            &negative,
        )
        .unwrap_err()
        .code(),
        DiagnosticCode::InvalidClause
    );

    for invalid in [
        "#pragma omp metadirective when(device={kind(score(1): cpu)}: parallel)",
        "#pragma omp metadirective when(target_device={device_num(score(1): dev)}: parallel)",
        "#pragma omp metadirective when(construct={score(1): parallel}: parallel)",
        "#pragma omp metadirective when(implementation={vendor(score(-1): llvm)}: parallel)",
        "#pragma omp metadirective when(implementation={vendor(score(1.5): llvm)}: parallel)",
    ] {
        assert!(
            parse(OpenMpVersion::V6_0, invalid).is_err(),
            "accepted {invalid}"
        );
    }
}

#[test]
fn scored_user_conditions_remain_cumulative_and_openmp_5_0_remains_static() {
    let source =
        "#pragma omp metadirective when(user={condition(score(weight): runtime_flag)}: parallel)";
    for version in [
        OpenMpVersion::V5_0,
        OpenMpVersion::V5_1,
        OpenMpVersion::V5_2,
        OpenMpVersion::V6_0,
    ] {
        let parsed = parse(version, source).unwrap_or_else(|error| {
            panic!("OpenMP {version} rejected scored user syntax: {error}")
        });
        let OmpSelectorEntry::User { score, condition } = &selector(&parsed).entries()[0] else {
            panic!("expected a typed user selector");
        };
        assert_eq!(
            score.as_ref().map(ToString::to_string).as_deref(),
            Some("weight")
        );
        assert_eq!(condition.to_string(), "runtime_flag");
    }

    let parsed = parse(OpenMpVersion::V5_0, source).expect("OpenMP 5.0 scored user selector");
    let dynamic_facts = SemanticFacts::new()
        .with_constant_expression(expression_site(0), true)
        .with_integer_evaluation(expression_site(0), IntegerEvaluation::NonNegative(4))
        .with_logical_expression(expression_site(1), true)
        .with_constant_expression(expression_site(1), false);
    assert_eq!(
        validate_openmp_with_facts(
            parsed.directive(),
            VersionPolicy::Exact(OpenMpVersion::V5_0),
            parsed.directive().span(),
            &dynamic_facts,
        )
        .unwrap_err()
        .code(),
        DiagnosticCode::ConstantExpressionRequired
    );

    let static_facts = dynamic_facts
        .clone()
        .with_constant_expression(expression_site(1), true);
    validate_openmp_with_facts(
        parsed.directive(),
        VersionPolicy::Exact(OpenMpVersion::V5_0),
        parsed.directive().span(),
        &static_facts,
    )
    .expect("OpenMP 5.0 accepts a constant scored user condition");

    let parsed = parse(OpenMpVersion::V5_1, source).expect("OpenMP 5.1 scored user selector");
    validate_openmp_with_facts(
        parsed.directive(),
        VersionPolicy::Exact(OpenMpVersion::V5_1),
        parsed.directive().span(),
        &dynamic_facts,
    )
    .expect("OpenMP 5.1 permits a dynamic scored user condition");

    assert!(
        parse(
            OpenMpVersion::V6_0,
            "#pragma omp metadirective when(user={condition(score(-1): runtime_flag)}: parallel)"
        )
        .is_err()
    );
}

#[test]
fn atomic_default_order_and_requires_properties_are_typed_and_versioned() {
    let base_5_1 = "#pragma omp metadirective when(implementation={atomic_default_mem_order(seq_cst), requires(unified_address)}: parallel)";
    assert_eq!(
        parse(OpenMpVersion::V5_0, base_5_1).unwrap_err().code(),
        DiagnosticCode::NotAvailableInVersion
    );
    parse(OpenMpVersion::V5_1, base_5_1)
        .expect("atomic_default_mem_order and requires selectors start in OpenMP 5.1");

    let source = "#pragma omp metadirective when(implementation={atomic_default_mem_order(score(2): acquire), requires(score(3): unified_address(flag), dynamic_allocators, atomic_default_mem_order(relaxed))}: parallel)";
    assert_eq!(
        parse(OpenMpVersion::V5_2, source).unwrap_err().code(),
        DiagnosticCode::NotAvailableInVersion
    );
    let parsed = parse(OpenMpVersion::V6_0, source).expect("OpenMP 6 selector requirements");
    let OmpSelectorEntry::Implementation { traits } = &selector(&parsed).entries()[0] else {
        panic!("expected implementation selector");
    };
    assert!(matches!(
        traits[0].kind(),
        OmpSelectorImplementationTraitKind::AtomicDefaultMemOrder(MemoryOrder::Acquire)
    ));
    let OmpSelectorImplementationTraitKind::Requires(requirements) = traits[1].kind() else {
        panic!("expected typed requires clause properties");
    };
    assert_eq!(requirements.len(), 3);
    assert!(matches!(
        requirements[0].requirement(),
        RequireModifier::UnifiedAddress
    ));
    assert_eq!(
        requirements[0]
            .required()
            .map(ToString::to_string)
            .as_deref(),
        Some("flag")
    );
    assert!(matches!(
        requirements[2].requirement(),
        RequireModifier::AtomicDefaultMemOrder(MemoryOrder::Relaxed)
    ));

    let facts = SemanticFacts::new()
        .with_integer_evaluation(expression_site(0), IntegerEvaluation::NonNegative(2))
        .with_integer_evaluation(expression_site(1), IntegerEvaluation::NonNegative(3))
        .with_logical_evaluation(expression_site(2), LogicalEvaluation::True);
    validate_openmp_with_facts(
        parsed.directive(),
        VersionPolicy::Exact(OpenMpVersion::V6_0),
        parsed.directive().span(),
        &facts,
    )
    .expect("score and required-condition facts");
}

#[test]
fn extension_properties_are_recursive_and_user_maps_to_the_dynamic_context_without_a_dynamic_set() {
    let parsed = parse(
        OpenMpVersion::V6_0,
        "#pragma omp metadirective when(device={vendor_device(prop, nested(\"x\", 4), extent + 1)}, implementation={vendor_impl(score(2): mode(fast), 8)}, user={condition(runtime_flag)}: parallel)",
    )
    .expect("typed extension traits and user condition");
    let OmpSelectorEntry::Device { traits } = &selector(&parsed).entries()[0] else {
        panic!("expected device extension");
    };
    let OmpSelectorDeviceTrait::Extension(extension) = &traits[0] else {
        panic!("expected typed device extension trait");
    };
    assert_eq!(extension.name().as_str(), "vendor_device");
    assert!(matches!(
        &extension.properties()[1],
        OmpSelectorExtensionProperty::Call { name, properties }
            if name.as_str() == "nested" && properties.len() == 2
    ));
    assert!(matches!(
        selector(&parsed).entries()[2],
        OmpSelectorEntry::User { .. }
    ));

    // Expression order includes obvious integer literals so callers have one
    // stable index scheme: nested `4`, `extent + 1`, score, `8`, condition.
    let facts = SemanticFacts::new()
        .with_integer_evaluation(expression_site(1), IntegerEvaluation::NonNegative(5))
        .with_integer_evaluation(expression_site(2), IntegerEvaluation::NonNegative(2))
        .with_logical_expression(expression_site(4), true);
    validate_openmp_with_facts(
        parsed.directive(),
        VersionPolicy::Exact(OpenMpVersion::V6_0),
        parsed.directive().span(),
        &facts,
    )
    .expect("recursive extension and user facts");

    assert!(
        parse(
            OpenMpVersion::V6_0,
            "#pragma omp metadirective when(dynamic={condition(runtime_flag)}: parallel)"
        )
        .is_err()
    );
}

#[test]
fn device_number_requires_integer_and_conforming_device_facts() {
    let parsed = parse(
        OpenMpVersion::V6_0,
        "#pragma omp metadirective when(target_device={device_num(dev)}: parallel)",
    )
    .expect("target device number");
    let missing = validate_openmp_with_facts(
        parsed.directive(),
        VersionPolicy::Exact(OpenMpVersion::V6_0),
        parsed.directive().span(),
        &SemanticFacts::new().with_integer_expression(expression_site(0), true),
    )
    .unwrap_err();
    assert_eq!(missing.code(), DiagnosticCode::MissingSemanticFact);

    let facts = SemanticFacts::new()
        .with_integer_expression(expression_site(0), true)
        .with_conforming_device_number(expression_site(0), true);
    validate_openmp_with_facts(
        parsed.directive(),
        VersionPolicy::Exact(OpenMpVersion::V6_0),
        parsed.directive().span(),
        &facts,
    )
    .expect("conforming device number facts");

    for invalid in [
        "#pragma omp metadirective when(device={device_num(0)}: parallel)",
        "#pragma omp metadirective when(device={uid(\"gpu\")}: parallel)",
        "#pragma omp metadirective when(target_device={device_num(-1)}: parallel)",
        "#pragma omp metadirective when(target_device={uid(a, b)}: parallel)",
        "#pragma omp metadirective when(device={kind(cpu)}, device={isa(avx2)}: parallel)",
        "#pragma omp metadirective when(target_device={kind(gpu)}, target_device={uid(\"x\")}: parallel)",
        "#pragma omp metadirective when(device={vendor_trait(nested())}: parallel)",
        "#pragma omp metadirective when(device={vendor_trait(nested(foo) junk)}: parallel)",
        "#pragma omp metadirective when(implementation={vendor_trait(score(2): nested(foo)}: parallel)",
    ] {
        assert!(
            parse(OpenMpVersion::V6_0, invalid).is_err(),
            "accepted {invalid}"
        );
    }
}
