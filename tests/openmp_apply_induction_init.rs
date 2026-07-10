use roup::api::{OpenMpConfig, ParsedOpenMpDirective};
use roup::ast::{OmpDirectiveKind, OmpInductionIdentifier};
use roup::diagnostic::Diagnostic;
use roup::ir::{
    ClauseData, ClauseItem, DepobjUpdateDependence, OmpApplyLoopKind, OmpForeignRuntimeIdentifier,
    OmpInductionModifier, OmpInteropType, OmpPreferenceSelector, OmpPreferenceSpecification,
};
use roup::version::{
    CStandard, CppStandard, FortranStandard, HostLanguageProfile, OpenMpVersion, SourceForm,
};

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

fn parse_profile(
    profile: HostLanguageProfile,
    form: SourceForm,
    source: &str,
) -> Result<ParsedOpenMpDirective, Diagnostic> {
    OpenMpConfig::exact(OpenMpVersion::V6_0, profile, form)
        .unwrap()
        .parser()
        .parse(source)
}

#[test]
fn apply_retains_loop_selection_and_complete_applied_directives() {
    let parsed = parse(
        OpenMpVersion::V6_0,
        "#pragma omp tile sizes(8, 8) apply(grid(1, 2): interchange permutation(2, 1), reverse)",
    )
    .expect("typed apply clause");
    let ClauseData::Apply {
        loop_modifier: Some(modifier),
        applied_directives,
    } = parsed.directive().clauses()[1].payload()
    else {
        panic!("apply must have a typed modifier and directives");
    };
    assert_eq!(modifier.kind, OmpApplyLoopKind::Grid);
    assert_eq!(
        modifier
            .indices
            .iter()
            .map(|index| index.source())
            .collect::<Vec<_>>(),
        ["1", "2"]
    );
    assert_eq!(
        applied_directives
            .iter()
            .map(|directive| directive.kind())
            .collect::<Vec<_>>(),
        [OmpDirectiveKind::Interchange, OmpDirectiveKind::Reverse]
    );
    assert_eq!(applied_directives[0].clauses().len(), 1);
}

#[test]
fn apply_rejects_the_old_pseudo_transform_payloads() {
    for source in [
        "#pragma omp tile sizes(4) apply(label: reverse)",
        "#pragma omp tile sizes(4) apply(grid(): reverse)",
        "#pragma omp tile sizes(4) apply(grid: reverse,)",
        "#pragma omp tile sizes(4) apply(grid: @not_a_directive)",
    ] {
        assert!(
            parse(OpenMpVersion::V6_0, source).is_err(),
            "unexpectedly accepted {source:?}"
        );
    }
}

#[test]
fn induction_is_one_typed_specification_not_an_ordered_expression_bag() {
    let parsed = parse(
        OpenMpVersion::V6_0,
        "#pragma omp parallel for induction(strict, step(delta), *: index, state.member)",
    )
    .expect("typed induction clause");
    let ClauseData::Induction {
        modifier,
        step,
        identifier,
        items,
    } = parsed.directive().clauses()[0].payload()
    else {
        panic!("induction must have a dedicated payload");
    };
    assert_eq!(*modifier, Some(OmpInductionModifier::Strict));
    assert_eq!(step.source(), "delta");
    assert!(matches!(identifier, OmpInductionIdentifier::Multiply));
    assert!(matches!(items[0], ClauseItem::Identifier(_)));
    assert!(matches!(items[1], ClauseItem::Variable(_)));

    for source in [
        "#pragma omp parallel for induction(step(a), step(b), *: x)",
        "#pragma omp parallel for induction(step(a): x)",
        "#pragma omp parallel for induction(*: x)",
        "#pragma omp parallel for induction(strict, relaxed, step(a), *: x)",
        "#pragma omp parallel for induction(step(a), *, next: x)",
        "#pragma omp parallel for induction(step(a), *: x + 1)",
    ] {
        assert!(
            parse(OpenMpVersion::V6_0, source).is_err(),
            "unexpectedly accepted {source:?}"
        );
    }
}

#[test]
fn induction_reuses_the_full_declared_induction_identifier_grammar() {
    let cpp = parse_profile(
        HostLanguageProfile::Cpp(CppStandard::Cpp23),
        SourceForm::Pragma,
        "#pragma omp parallel for induction(step(delta), ns::next<int>: index)",
    )
    .expect("qualified C++ induction identifier");
    assert!(matches!(
        cpp.directive().clauses()[0].payload(),
        ClauseData::Induction {
            identifier: OmpInductionIdentifier::Name(_),
            ..
        }
    ));

    let fortran = parse_profile(
        HostLanguageProfile::Fortran(FortranStandard::Fortran2023),
        SourceForm::FortranFree,
        "!$omp parallel do induction(step(delta), .advance.: index)",
    )
    .expect("Fortran defined induction operator");
    assert!(matches!(
        fortran.directive().clauses()[0].payload(),
        ClauseData::Induction {
            identifier: OmpInductionIdentifier::DefinedOperator(name),
            ..
        } if name.as_str() == "advance"
    ));
}

#[test]
fn historical_and_openmp6_prefer_type_syntax_share_typed_semantics() {
    let historical = parse(
        OpenMpVersion::V5_1,
        "#pragma omp interop init(prefer_type(\"cuda\", omp_ifr_hip), target, targetsync: object)",
    )
    .expect("OpenMP 5.1 prefer_type syntax remains accepted");
    let ClauseData::InitInterop {
        interop_types,
        preferences,
        variable,
    } = historical.directive().clauses()[0].payload()
    else {
        panic!("interop init must be typed");
    };
    assert_eq!(
        interop_types,
        &[OmpInteropType::Target, OmpInteropType::Targetsync]
    );
    assert_eq!(preferences.len(), 2);
    assert!(matches!(
        preferences[0],
        OmpPreferenceSpecification::ForeignRuntime(OmpForeignRuntimeIdentifier::StringLiteral(_))
    ));
    assert_eq!(variable.to_string(), "object");
    parse(
        OpenMpVersion::V6_0,
        "#pragma omp interop init(prefer_type(\"cuda\", omp_ifr_hip), target, targetsync: object)",
    )
    .expect("historical prefer_type syntax remains accepted by OpenMP 6 mode");

    let modern = parse(
        OpenMpVersion::V6_0,
        "#pragma omp interop init(prefer_type({fr(\"cuda\"), attr(\"ompx_fast\", \"ompx_vendor\")}), target: object)",
    )
    .expect("OpenMP 6 preference selectors");
    let ClauseData::InitInterop { preferences, .. } = modern.directive().clauses()[0].payload()
    else {
        panic!("interop init must be typed");
    };
    let OmpPreferenceSpecification::Selectors(selectors) = &preferences[0] else {
        panic!("brace preference must retain selectors");
    };
    assert!(matches!(
        selectors[0],
        OmpPreferenceSelector::ForeignRuntime(_)
    ));
    assert!(matches!(
        &selectors[1],
        OmpPreferenceSelector::Attributes(attributes) if attributes.len() == 2
    ));
}

#[test]
fn new_payload_forms_have_openmp6_floors_without_removing_old_syntax() {
    for source in [
        "#pragma omp tile sizes(4) apply(grid: reverse)",
        "#pragma omp parallel for induction(step(1), *: index)",
        "#pragma omp depobj init(in(item): dependence_object)",
        "#pragma omp interop init(prefer_type({fr(\"cuda\")}), target: object)",
    ] {
        assert!(
            parse(OpenMpVersion::V5_2, source).is_err(),
            "OpenMP 6 syntax unexpectedly accepted in 5.2 mode: {source}"
        );
        parse(OpenMpVersion::V6_0, source)
            .unwrap_or_else(|error| panic!("OpenMP 6 syntax rejected: {source}: {error}"));
    }
}

#[test]
fn depobj_init_has_a_typed_dependence_locator_and_variable() {
    let parsed = parse(
        OpenMpVersion::V6_0,
        "#pragma omp depobj init(inout(item): dependence_object)",
    )
    .expect("OpenMP 6 depobj init");
    assert!(parsed.directive().parameter().is_none());
    assert!(matches!(
        parsed.directive().clauses()[0].payload(),
        ClauseData::InitDepobj {
            dependence: DepobjUpdateDependence::Inout,
            variable,
            ..
        } if variable.to_string() == "dependence_object"
    ));

    for source in [
        "#pragma omp depobj init(in(a, b): dep)",
        "#pragma omp depobj init(depobj(a): dep)",
        "#pragma omp interop init(in(a): dep)",
        "#pragma omp interop init(target: object + 1)",
        "#pragma omp interop init(prefer_type({attr(\"fast\")}), target: object)",
    ] {
        assert!(
            parse(OpenMpVersion::V6_0, source).is_err(),
            "unexpectedly accepted {source:?}"
        );
    }
}

#[test]
fn depobj_update_keeps_historical_and_openmp6_argument_sources_distinct() {
    for version in [OpenMpVersion::V5_2, OpenMpVersion::V6_0] {
        let historical = parse(
            version,
            "#pragma omp depobj(dependence_object) update(inout)",
        )
        .unwrap_or_else(|error| panic!("historical depobj update rejected in {version}: {error}"));
        assert!(historical.directive().parameter().is_some());
        assert!(matches!(
            historical.directive().clauses()[0].payload(),
            ClauseData::DepobjUpdate {
                dependence: DepobjUpdateDependence::Inout,
                variable: None,
            }
        ));
    }

    let modern = parse(
        OpenMpVersion::V6_0,
        "#pragma omp depobj update(inout: dependence_object)",
    )
    .expect("OpenMP 6 depobj update must carry its own update variable");
    assert!(modern.directive().parameter().is_none());
    assert!(matches!(
        modern.directive().clauses()[0].payload(),
        ClauseData::DepobjUpdate {
            dependence: DepobjUpdateDependence::Inout,
            variable: Some(variable),
        } if variable.to_string() == "dependence_object"
    ));

    parse(
        OpenMpVersion::V6_0,
        "#pragma omp depobj(dependence_object) update(inout: dependence_object)",
    )
    .expect("an explicit update variable may repeat the historical directive argument");
    assert!(
        parse(
            OpenMpVersion::V5_2,
            "#pragma omp depobj update(inout: dependence_object)",
        )
        .is_err(),
        "the argument-less depobj grammar must retain its OpenMP 6 floor"
    );

    for source in [
        "#pragma omp depobj update(inout)",
        "#pragma omp depobj destroy",
        "#pragma omp depobj depend(in: source)",
        "#pragma omp depobj(dependence_object) update(inout: other_object)",
        "#pragma omp depobj(dependence_object) destroy(other_object)",
        "#pragma omp depobj(dependence_object) init(in(source): dependence_object)",
    ] {
        assert!(
            parse(OpenMpVersion::V6_0, source).is_err(),
            "malformed depobj argument ownership unexpectedly parsed: {source}"
        );
    }

    parse(
        OpenMpVersion::V6_0,
        "#pragma omp depobj init(in(source): first_object) update(out: second_object) destroy(third_object)",
    )
    .expect("the OpenMP 6 required action clause set is not exclusive");
}
