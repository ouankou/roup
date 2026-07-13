use roup::api::{OpenMpConfig, ParsedOpenMpDirective};
use roup::ast::OmpClauseKind;
use roup::diagnostic::{Diagnostic, DiagnosticCode};
use roup::ir::{
    AdjustArgsModifier, ClauseData, DependType, OmpAppendOperation, OmpDependence,
    OmpDoacrossIteration, OmpDoacrossOffset, OmpInteropType, OmpLocator, OmpParameterListItem,
};
use roup::validation::{
    AssociationKind, DependObjectState, IntegerEvaluation, OmpClauseItemSite, OmpClauseSite,
    OmpExpressionSite, SemanticFacts,
};
use roup::version::{CStandard, CppStandard, HostLanguageProfile, OpenMpVersion, SourceForm};

fn parse_c(version: OpenMpVersion, source: &str) -> Result<ParsedOpenMpDirective, Diagnostic> {
    OpenMpConfig::exact(
        version,
        HostLanguageProfile::C(CStandard::C23),
        SourceForm::Pragma,
    )
    .expect("valid C parser configuration")
    .parser()
    .parse(source)
}

fn parse_cpp(version: OpenMpVersion, source: &str) -> Result<ParsedOpenMpDirective, Diagnostic> {
    OpenMpConfig::exact(
        version,
        HostLanguageProfile::Cpp(CppStandard::Cpp23),
        SourceForm::Pragma,
    )
    .expect("valid C++ parser configuration")
    .parser()
    .parse(source)
}

#[test]
fn historical_adjust_and_append_forms_remain_typed_in_openmp_six() {
    let adjust_source = "#pragma omp declare variant(fast) match(construct={parallel}) adjust_args(nothing: first, second)";
    for version in [
        OpenMpVersion::V5_1,
        OpenMpVersion::V5_2,
        OpenMpVersion::V6_0,
    ] {
        let parsed = parse_c(version, adjust_source)
            .unwrap_or_else(|error| panic!("OpenMP {version} rejected adjust_args: {error}"));
        assert!(matches!(
            parsed.directive().clauses()[1].payload(),
            ClauseData::AdjustArgs {
                operation: AdjustArgsModifier::Nothing,
                parameters,
            } if matches!(parameters.as_slice(), [
                OmpParameterListItem::Named(_),
                OmpParameterListItem::Named(_),
            ])
        ));
    }

    let append_source = "#pragma omp declare variant(fast) match(construct={parallel}) append_args(interop(target, targetsync))";
    for version in [
        OpenMpVersion::V5_1,
        OpenMpVersion::V5_2,
        OpenMpVersion::V6_0,
    ] {
        let parsed = parse_c(version, append_source)
            .unwrap_or_else(|error| panic!("OpenMP {version} rejected append_args: {error}"));
        assert!(matches!(
            parsed.directive().clauses()[1].payload(),
            ClauseData::AppendArgs { operations }
                if matches!(operations.as_slice(), [OmpAppendOperation::Interop(modifiers)]
                    if modifiers.interop_types == [OmpInteropType::Target, OmpInteropType::Targetsync]
                        && modifiers.preferences.is_empty())
        ));
    }
}

#[test]
fn openmp_six_argument_positions_ranges_and_preferences_have_typed_shapes() {
    let adjusted = parse_c(
        OpenMpVersion::V6_0,
        "#pragma omp declare variant(fast) match(construct={parallel}) adjust_args(need_device_ptr: 1, 3:5, 7:)",
    )
    .expect("OpenMP 6 positional adjustment list");
    let ClauseData::AdjustArgs {
        operation,
        parameters,
    } = adjusted.directive().clauses()[1].payload()
    else {
        panic!("expected adjust_args payload");
    };
    assert_eq!(*operation, AdjustArgsModifier::NeedDevicePtr);
    assert!(matches!(
        parameters[0],
        OmpParameterListItem::Position(position) if position.get() == 1
    ));
    assert!(matches!(
        &parameters[1],
        OmpParameterListItem::Range(range)
            if matches!((range.lower(), range.upper()), (Some(lower), Some(upper))
                if lower.to_string() == "3" && upper.to_string() == "5")
    ));
    assert!(matches!(
        &parameters[2],
        OmpParameterListItem::Range(range)
            if matches!((range.lower(), range.upper()), (Some(lower), None)
                if lower.to_string() == "7")
    ));

    let open_lower = parse_c(
        OpenMpVersion::V6_0,
        "#pragma omp declare variant(fast) match(construct={parallel}) adjust_args(nothing: :2)",
    )
    .expect("OpenMP 6 open-lower adjustment range");
    assert!(matches!(
        open_lower.directive().clauses()[1].payload(),
        ClauseData::AdjustArgs { parameters, .. }
            if matches!(parameters.as_slice(), [OmpParameterListItem::Range(range)]
                if matches!((range.lower(), range.upper()), (None, Some(upper))
                    if upper.to_string() == "2"))
    ));

    let future_in_5_2 = parse_c(
        OpenMpVersion::V5_2,
        "#pragma omp declare variant(fast) match(construct={parallel}) adjust_args(nothing: 1)",
    )
    .unwrap_err();
    assert_eq!(future_in_5_2.code(), DiagnosticCode::NotAvailableInVersion);

    let preferred = parse_c(
        OpenMpVersion::V6_0,
        "#pragma omp declare variant(fast) match(construct={parallel}) append_args(interop(target, prefer_type({fr(\"cuda\")})))",
    )
    .expect("OpenMP 6 append preference");
    assert!(matches!(
        preferred.directive().clauses()[1].payload(),
        ClauseData::AppendArgs { operations }
            if matches!(operations.as_slice(), [OmpAppendOperation::Interop(modifiers)]
                if modifiers.interop_types == [OmpInteropType::Target]
                    && modifiers.preferences.len() == 1)
    ));
    let future_in_5_2 = parse_c(
        OpenMpVersion::V5_2,
        "#pragma omp declare variant(fast) match(construct={parallel}) append_args(interop(target, prefer_type({fr(\"cuda\")})))",
    )
    .unwrap_err();
    assert_eq!(future_in_5_2.code(), DiagnosticCode::NotAvailableInVersion);

    parse_c(
        OpenMpVersion::V6_0,
        "#pragma omp declare variant(fast) match(construct={parallel}) adjust_args(need_device_addr: first)",
    )
    .expect_err("C forbids need_device_addr");
    parse_cpp(
        OpenMpVersion::V6_0,
        "#pragma omp declare variant(fast) match(construct={parallel}) adjust_args(need_device_addr: first)",
    )
    .expect("C++ accepts need_device_addr");
}

#[test]
fn malformed_argument_adjustment_never_degrades_to_raw_payloads() {
    for source in [
        "#pragma omp declare variant(fast) match(construct={parallel}) adjust_args(nothing)",
        "#pragma omp declare variant(fast) match(construct={parallel}) adjust_args(unknown: first)",
        "#pragma omp declare variant(fast) match(construct={parallel}) adjust_args(nothing:)",
        "#pragma omp declare variant(fast) match(construct={parallel}) adjust_args(nothing: 0)",
        "#pragma omp declare variant(fast) match(construct={parallel}) adjust_args(nothing: :)",
        "#pragma omp declare variant(fast) match(construct={parallel}) adjust_args(nothing: 4:2)",
        "#pragma omp declare variant(fast) match(construct={parallel}) adjust_args(nothing: first, first)",
        "#pragma omp declare variant(fast) match(construct={parallel}) append_args()",
        "#pragma omp declare variant(fast) match(construct={parallel}) append_args(raw_text)",
        "#pragma omp declare variant(fast) match(construct={parallel}) append_args(interop())",
        "#pragma omp declare variant(fast) match(construct={parallel}) append_args(interop(target, target))",
        "#pragma omp declare variant(fast) match(construct={parallel}) append_args(interop(target) trailing)",
    ] {
        assert!(
            parse_c(OpenMpVersion::V6_0, source).is_err(),
            "malformed argument adjustment unexpectedly parsed: {source}"
        );
    }
}

#[test]
fn depend_objects_all_memory_and_doacross_are_distinct_typed_nodes() {
    let depobj = parse_c(
        OpenMpVersion::V5_0,
        "#pragma omp task depend(depobj: dependency)",
    )
    .expect("OpenMP 5.0 depend object");
    assert!(matches!(
        depobj.directive().clauses()[0].payload(),
        ClauseData::Depend {
            dependence: OmpDependence::Depobjs { objects },
            iterators,
        } if objects.len() == 1 && iterators.is_empty()
    ));

    let all_memory = parse_c(
        OpenMpVersion::V6_0,
        "#pragma omp task depend(out: omp_all_memory)",
    )
    .expect("cumulative omp_all_memory dependence");
    assert!(matches!(
        all_memory.directive().clauses()[0].payload(),
        ClauseData::Depend {
            dependence: OmpDependence::Locators {
                kind: DependType::Out,
                locators,
            },
            ..
        } if locators.as_slice() == [OmpLocator::AllMemory]
    ));
    assert_eq!(
        parse_c(
            OpenMpVersion::V5_0,
            "#pragma omp task depend(out: omp_all_memory)",
        )
        .unwrap_err()
        .code(),
        DiagnosticCode::NotAvailableInVersion
    );

    for version in [OpenMpVersion::V4_5, OpenMpVersion::V6_0] {
        let historical = parse_c(version, "#pragma omp ordered depend(source)")
            .unwrap_or_else(|error| panic!("OpenMP {version} rejected depend(source): {error}"));
        assert!(matches!(
            historical.directive().clauses()[0].payload(),
            ClauseData::Doacross {
                iteration: OmpDoacrossIteration::Current,
                ..
            }
        ));
    }

    let vector = parse_c(
        OpenMpVersion::V6_0,
        "#pragma omp ordered doacross(sink: outer - distance, inner + 2)",
    )
    .expect("typed doacross vector");
    let ClauseData::Doacross {
        iteration: OmpDoacrossIteration::Vector(items),
        ..
    } = vector.directive().clauses()[0].payload()
    else {
        panic!("expected doacross vector");
    };
    assert_eq!(items.len(), 2);
    assert!(matches!(
        &items[0].offset,
        Some(OmpDoacrossOffset::Subtract(offset)) if offset.to_string() == "distance"
    ));
    assert!(matches!(
        &items[1].offset,
        Some(OmpDoacrossOffset::Add(offset)) if offset.to_string() == "2"
    ));
}

#[test]
fn malformed_dependence_shapes_are_hard_errors() {
    for source in [
        "#pragma omp task depend(in: omp_all_memory)",
        "#pragma omp task depend(out: omp_all_memory, other)",
        "#pragma omp task depend(depobj: dependency[0:1])",
        "#pragma omp task depend(depobj: dependency + 1)",
        "#pragma omp ordered depend(source: outer)",
        "#pragma omp ordered depend(sink)",
        "#pragma omp ordered doacross(source: outer)",
        "#pragma omp ordered doacross(sink: omp_cur_iteration)",
        "#pragma omp ordered doacross(sink: outer * 2)",
        "#pragma omp ordered doacross(sink: outer - -1)",
    ] {
        assert!(
            parse_c(OpenMpVersion::V6_0, source).is_err(),
            "malformed dependence unexpectedly parsed: {source}"
        );
    }
}

#[test]
fn semantic_fact_sites_cover_adjust_depend_objects_and_doacross_offsets() {
    let adjust_source =
        "#pragma omp declare variant(fast) match(construct={parallel}) adjust_args(nothing: first)";
    let adjust_parser = OpenMpConfig::exact(
        OpenMpVersion::V5_1,
        HostLanguageProfile::C(CStandard::C23),
        SourceForm::Pragma,
    )
    .unwrap()
    .parser();
    let missing = adjust_parser
        .parse_with_facts(adjust_source, &SemanticFacts::new())
        .unwrap_err();
    assert!(matches!(
        missing.code(),
        DiagnosticCode::MissingContext | DiagnosticCode::MissingSemanticFact
    ));
    let adjust_clause = OmpClauseSite::new(OmpClauseKind::AdjustArgs, 0);
    let adjust_facts = SemanticFacts::new()
        .with_declaration_position(true)
        .with_procedure_parameter(OmpClauseItemSite::new(adjust_clause, 0), true);
    adjust_parser
        .parse_with_facts(adjust_source, &adjust_facts)
        .expect("complete adjust_args facts");

    let depobj_source = "#pragma omp task depend(depobj: dependency)";
    let depobj_parser = OpenMpConfig::exact(
        OpenMpVersion::V6_0,
        HostLanguageProfile::C(CStandard::C23),
        SourceForm::Pragma,
    )
    .unwrap()
    .parser();
    let depobj_item = OmpClauseItemSite::new(OmpClauseSite::new(OmpClauseKind::Depend, 0), 0);
    assert_eq!(
        depobj_parser
            .parse_with_facts(depobj_source, &SemanticFacts::new())
            .unwrap_err()
            .code(),
        DiagnosticCode::MissingSemanticFact
    );
    let uninitialized =
        SemanticFacts::new().with_depend_object(depobj_item, DependObjectState::Uninitialized);
    assert_eq!(
        depobj_parser
            .parse_with_facts(depobj_source, &uninitialized)
            .unwrap_err()
            .code(),
        DiagnosticCode::InvalidClause
    );
    let initialized =
        SemanticFacts::new().with_depend_object(depobj_item, DependObjectState::Initialized);
    depobj_parser
        .parse_with_facts(depobj_source, &initialized)
        .expect("initialized depend object");

    let doacross_source = "#pragma omp ordered doacross(sink: outer - distance, inner + 2)";
    let doacross_clause = OmpClauseSite::new(OmpClauseKind::Doacross, 0);
    let base_facts = SemanticFacts::new()
        .with_association(AssociationKind::DoacrossLoop, true)
        .with_associated_ordered_parameter(2);
    assert_eq!(
        depobj_parser
            .parse_with_facts(doacross_source, &base_facts)
            .unwrap_err()
            .code(),
        DiagnosticCode::MissingSemanticFact
    );
    let complete = base_facts.with_integer_evaluation(
        OmpExpressionSite::new(doacross_clause, 0),
        IntegerEvaluation::NonNegative(1),
    );
    depobj_parser
        .parse_with_facts(doacross_source, &complete)
        .expect("complete doacross offset facts");
}
