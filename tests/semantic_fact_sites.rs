use roup::api::{OpenAccConfig, OpenAccParser, OpenMpConfig, OpenMpParser};
use roup::ast::{AccClauseKind, OmpClauseKind};
use roup::diagnostic::DiagnosticCode;
use roup::validation::{
    AccClauseSite, AccExpressionSite, IntegerEvaluation, OmpClauseItemSite, OmpClauseSite,
    OmpExpressionSite, SemanticFacts,
};
use roup::version::{
    CStandard, CppStandard, FortranStandard, HostLanguageProfile, OpenMpVersion, SourceForm,
};

fn omp() -> OpenMpParser {
    OpenMpConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .unwrap()
        .parser()
}

fn omp_exact(version: OpenMpVersion) -> OpenMpParser {
    OpenMpConfig::exact(
        version,
        HostLanguageProfile::C(CStandard::C23),
        SourceForm::Pragma,
    )
    .unwrap()
    .parser()
}

fn acc() -> OpenAccParser {
    OpenAccConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .unwrap()
        .parser()
}

fn omp_cpp() -> OpenMpParser {
    OpenMpConfig::new(
        HostLanguageProfile::Cpp(CppStandard::Cpp23),
        SourceForm::Pragma,
    )
    .unwrap()
    .parser()
}

fn omp_fortran() -> OpenMpParser {
    OpenMpConfig::new(
        HostLanguageProfile::Fortran(FortranStandard::Fortran2023),
        SourceForm::FortranFree,
    )
    .unwrap()
    .parser()
}

#[test]
fn permutation_facts_are_keyed_by_individual_expression() {
    let source = "#pragma omp interchange permutation(outer_position, inner_position)";
    let clause = OmpClauseSite::new(OmpClauseKind::Permutation, 0);
    let outer = OmpExpressionSite::new(clause, 0);
    let inner = OmpExpressionSite::new(clause, 1);

    omp()
        .parse(source)
        .expect("standalone parsing does not guess values");
    let error = omp()
        .parse_with_facts(source, &SemanticFacts::new())
        .unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::MissingSemanticFact);

    let incomplete =
        SemanticFacts::new().with_integer_evaluation(outer, IntegerEvaluation::NonNegative(2));
    let error = omp().parse_with_facts(source, &incomplete).unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::MissingSemanticFact);

    let valid = incomplete
        .clone()
        .with_integer_evaluation(inner, IntegerEvaluation::NonNegative(1));
    omp().parse_with_facts(source, &valid).unwrap();

    let duplicate = incomplete.with_integer_evaluation(inner, IntegerEvaluation::NonNegative(2));
    let error = omp().parse_with_facts(source, &duplicate).unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::InvalidClause);
}

#[test]
fn counts_facts_use_source_list_positions_including_omp_fill() {
    let source = "#pragma omp split counts(prefix, omp_fill, suffix)";
    let clause = OmpClauseSite::new(OmpClauseKind::Counts, 0);
    let prefix = OmpExpressionSite::new(clause, 0);
    let suffix = OmpExpressionSite::new(clause, 2);
    let valid = SemanticFacts::new()
        .with_integer_evaluation(prefix, IntegerEvaluation::NonNegative(3))
        .with_integer_evaluation(suffix, IntegerEvaluation::NonNegative(0));
    omp().parse_with_facts(source, &valid).unwrap();

    let negative = valid
        .clone()
        .with_integer_evaluation(suffix, IntegerEvaluation::Negative);
    let error = omp().parse_with_facts(source, &negative).unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::InvalidClause);
}

#[test]
fn sizes_constant_requirement_is_version_specific() {
    let source = "#pragma omp tile sizes(tile_width)";
    let site = OmpExpressionSite::new(OmpClauseSite::new(OmpClauseKind::Sizes, 0), 0);

    let positive_runtime = SemanticFacts::new().with_positive_integer_expression(site, true);
    let error = omp_exact(OpenMpVersion::V5_2)
        .parse_with_facts(source, &positive_runtime)
        .unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::MissingSemanticFact);

    let old_constant =
        SemanticFacts::new().with_integer_evaluation(site, IntegerEvaluation::NonNegative(8));
    omp_exact(OpenMpVersion::V5_2)
        .parse_with_facts(source, &old_constant)
        .unwrap();

    omp_exact(OpenMpVersion::V6_0)
        .parse_with_facts(source, &positive_runtime)
        .unwrap();

    let invalid = SemanticFacts::new().with_positive_integer_expression(site, false);
    let error = omp_exact(OpenMpVersion::V6_0)
        .parse_with_facts(source, &invalid)
        .unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::InvalidExpressionType);
}

#[test]
fn openacc_tile_facts_are_keyed_by_nonautomatic_list_entry() {
    let source = "#pragma acc loop tile(*, tile_width)";
    let site = AccExpressionSite::new(AccClauseSite::new(AccClauseKind::Tile, 0), 1);
    let error = acc()
        .parse_with_facts(source, &SemanticFacts::new())
        .unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::MissingSemanticFact);

    let valid =
        SemanticFacts::new().with_acc_integer_evaluation(site, IntegerEvaluation::NonNegative(16));
    acc().parse_with_facts(source, &valid).unwrap();

    let invalid =
        SemanticFacts::new().with_acc_integer_evaluation(site, IntegerEvaluation::NonNegative(0));
    let error = acc().parse_with_facts(source, &invalid).unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::InvalidClause);
}

#[test]
fn message_type_is_a_required_typed_fact_for_nonliteral_expressions() {
    let source = "#pragma omp error message(runtime_message)";
    let site = OmpExpressionSite::new(OmpClauseSite::new(OmpClauseKind::Message, 0), 0);
    let error = omp()
        .parse_with_facts(source, &SemanticFacts::new())
        .unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::MissingSemanticFact);

    let invalid = SemanticFacts::new().with_string_expression(site, false);
    let error = omp().parse_with_facts(source, &invalid).unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::InvalidExpressionType);

    let typed = SemanticFacts::new().with_string_expression(site, true);
    let error = omp().parse_with_facts(source, &typed).unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::MissingSemanticFact);

    let valid = typed.with_constant_expression(site, true);
    omp().parse_with_facts(source, &valid).unwrap();
    omp()
        .parse_with_facts(
            "#pragma omp error at(execution) message(runtime_message)",
            &SemanticFacts::new().with_string_expression(site, true),
        )
        .expect("execution-time messages need a string type but need not be constant");
    omp()
        .parse_with_facts(
            "#pragma omp error message(\"compile-time\")",
            &SemanticFacts::new(),
        )
        .expect("the typed string literal needs no external type fact");
}

#[test]
fn potential_lvalue_facts_are_keyed_by_locator_entry() {
    let source = "#pragma omp target update to(make_first(), make_second())";
    let clause = OmpClauseSite::new(OmpClauseKind::To, 0);
    let first = roup::validation::OmpLocatorSite::new(clause, 0);
    let second = roup::validation::OmpLocatorSite::new(clause, 1);

    omp_cpp()
        .parse(source)
        .expect("standalone parsing preserves syntactically ambiguous C++ locators");
    let error = omp_cpp()
        .parse_with_facts(source, &SemanticFacts::new())
        .unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::MissingSemanticFact);

    let incomplete = SemanticFacts::new().with_lvalue_locator(first, true);
    let error = omp_cpp().parse_with_facts(source, &incomplete).unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::MissingSemanticFact);

    let invalid = incomplete.clone().with_lvalue_locator(second, false);
    let error = omp_cpp().parse_with_facts(source, &invalid).unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::InvalidLocator);

    let valid = incomplete.with_lvalue_locator(second, true);
    omp_cpp().parse_with_facts(source, &valid).unwrap();
}

#[test]
fn automap_requires_an_allocatable_fact_for_every_item() {
    let source = "!$omp declare target enter(automap: first, second)";
    let clause = OmpClauseSite::new(OmpClauseKind::Enter, 0);
    let first = OmpClauseItemSite::new(clause, 0);
    let second = OmpClauseItemSite::new(clause, 1);
    let positioned = SemanticFacts::new().with_declaration_position(true);

    let error = omp_fortran()
        .parse_with_facts(source, &positioned)
        .unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::MissingSemanticFact);

    let incomplete = positioned.clone().with_allocatable_item(first, true);
    let error = omp_fortran()
        .parse_with_facts(source, &incomplete)
        .unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::MissingSemanticFact);

    let invalid = incomplete.clone().with_allocatable_item(second, false);
    let error = omp_fortran()
        .parse_with_facts(source, &invalid)
        .unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::InvalidClause);

    let valid = incomplete.with_allocatable_item(second, true);
    omp_fortran().parse_with_facts(source, &valid).unwrap();
}

#[test]
fn declare_simd_parameter_facts_are_keyed_by_clause_item() {
    let source = "#pragma omp declare simd uniform(first, second)";
    let clause = OmpClauseSite::new(OmpClauseKind::Uniform, 0);
    let first = OmpClauseItemSite::new(clause, 0);
    let second = OmpClauseItemSite::new(clause, 1);
    let positioned = SemanticFacts::new().with_declaration_position(true);

    let error = omp().parse_with_facts(source, &positioned).unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::MissingSemanticFact);

    let incomplete = positioned.clone().with_procedure_parameter(first, true);
    let error = omp().parse_with_facts(source, &incomplete).unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::MissingSemanticFact);

    let invalid = incomplete.clone().with_procedure_parameter(second, false);
    let error = omp().parse_with_facts(source, &invalid).unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::InvalidClause);

    let valid = incomplete.with_procedure_parameter(second, true);
    omp().parse_with_facts(source, &valid).unwrap();

    let list_source = "#pragma omp declare simd aligned(pointer) linear(index)";
    let aligned = OmpClauseItemSite::new(OmpClauseSite::new(OmpClauseKind::Aligned, 0), 0);
    let linear = OmpClauseItemSite::new(OmpClauseSite::new(OmpClauseKind::Linear, 0), 0);
    let facts = SemanticFacts::new()
        .with_declaration_position(true)
        .with_procedure_parameter(aligned, true)
        .with_procedure_parameter(linear, true)
        .with_linear_item(linear, true);
    omp().parse_with_facts(list_source, &facts).unwrap();
}

#[test]
fn uses_allocators_traits_require_per_spec_semantic_facts() {
    let source = "#pragma omp target uses_allocators(traits(constant_traits): custom_allocator)";
    let item = OmpClauseItemSite::new(OmpClauseSite::new(OmpClauseKind::UsesAllocators, 0), 0);

    let error = omp()
        .parse_with_facts(source, &SemanticFacts::new())
        .unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::MissingSemanticFact);

    let invalid = SemanticFacts::new().with_allocator_traits(item, false);
    let error = omp().parse_with_facts(source, &invalid).unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::InvalidClause);

    let valid = SemanticFacts::new().with_allocator_traits(item, true);
    omp().parse_with_facts(source, &valid).unwrap();

    let missing_old_traits = "#pragma omp target uses_allocators(custom_allocator)";
    let error = omp_exact(OpenMpVersion::V5_1)
        .parse(missing_old_traits)
        .unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::MissingRequiredClause);
    omp_exact(OpenMpVersion::V5_2)
        .parse(missing_old_traits)
        .expect("OpenMP 5.2 made the traits modifier optional for custom allocators");
}

#[test]
fn linear_semantics_are_item_and_step_specific() {
    let source = "#pragma omp declare simd linear(index: step(delta), ref)";
    let clause = OmpClauseSite::new(OmpClauseKind::Linear, 0);
    let item = OmpClauseItemSite::new(clause, 0);
    let step = OmpExpressionSite::new(clause, 0);
    let base = SemanticFacts::new()
        .with_declaration_position(true)
        .with_procedure_parameter(item, true)
        .with_linear_item(item, true)
        .with_integer_expression(step, true);

    let error = omp_cpp().parse_with_facts(source, &base).unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::MissingSemanticFact);

    let invalid = base.clone().with_linear_step(step, false);
    let error = omp_cpp().parse_with_facts(source, &invalid).unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::InvalidClause);

    let valid = base.with_linear_step(step, true);
    omp_cpp().parse_with_facts(source, &valid).unwrap();

    let error = omp_cpp()
        .parse("#pragma omp simd linear(index: step(1), ref)")
        .unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::InvalidModifier);
}

#[test]
fn induction_requires_step_and_item_compatibility_facts() {
    let source = "#pragma omp parallel for induction(strict, step(delta), *: index)";
    let clause = OmpClauseSite::new(OmpClauseKind::Induction, 0);
    let step = OmpExpressionSite::new(clause, 0);
    let item = OmpClauseItemSite::new(clause, 0);

    let error = omp()
        .parse_with_facts(source, &SemanticFacts::new())
        .unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::MissingSemanticFact);

    let incomplete = SemanticFacts::new().with_induction_step(step, true);
    let error = omp().parse_with_facts(source, &incomplete).unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::MissingSemanticFact);

    let invalid = incomplete.clone().with_induction_item(item, false);
    let error = omp().parse_with_facts(source, &invalid).unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::InvalidClause);

    let valid = incomplete.with_induction_item(item, true);
    omp().parse_with_facts(source, &valid).unwrap();
}

#[test]
fn runtime_and_constant_logical_arguments_use_distinct_facts() {
    let if_source = "#pragma omp parallel if(enabled)";
    let if_site = OmpExpressionSite::new(OmpClauseSite::new(OmpClauseKind::If, 0), 0);
    let error = omp()
        .parse_with_facts(if_source, &SemanticFacts::new())
        .unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::MissingSemanticFact);
    let invalid = SemanticFacts::new().with_logical_expression(if_site, false);
    assert_eq!(
        omp()
            .parse_with_facts(if_source, &invalid)
            .unwrap_err()
            .code(),
        DiagnosticCode::InvalidExpressionType
    );
    let valid = SemanticFacts::new().with_logical_expression(if_site, true);
    omp().parse_with_facts(if_source, &valid).unwrap();

    let mergeable_source = "#pragma omp task mergeable(can_merge)";
    let mergeable = OmpExpressionSite::new(OmpClauseSite::new(OmpClauseKind::Mergeable, 0), 0);
    let merely_typed = SemanticFacts::new().with_logical_expression(mergeable, true);
    assert_eq!(
        omp()
            .parse_with_facts(mergeable_source, &merely_typed)
            .unwrap_err()
            .code(),
        DiagnosticCode::MissingSemanticFact
    );
    let evaluated = SemanticFacts::new()
        .with_logical_evaluation(mergeable, roup::validation::LogicalEvaluation::True);
    omp()
        .parse_with_facts(mergeable_source, &evaluated)
        .unwrap();

    omp()
        .parse_with_facts("#pragma omp parallel if(1.5)", &SemanticFacts::new())
        .expect("a C real scalar expression has OpenMP logical type");
    assert!(omp_fortran().parse("!$omp parallel if(1.5)").is_err());
}

#[test]
fn aligned_and_num_teams_expression_properties_are_not_guessed() {
    let aligned_source = "#pragma omp simd aligned(pointer: alignment)";
    let aligned = OmpExpressionSite::new(OmpClauseSite::new(OmpClauseKind::Aligned, 0), 0);
    let runtime_positive = SemanticFacts::new().with_positive_integer_expression(aligned, true);
    assert_eq!(
        omp_exact(OpenMpVersion::V5_2)
            .parse_with_facts(aligned_source, &runtime_positive)
            .unwrap_err()
            .code(),
        DiagnosticCode::MissingSemanticFact
    );
    let invariant = runtime_positive
        .with_region_invariant_expression(aligned, true)
        .with_ultimate_expression(aligned, true);
    omp_exact(OpenMpVersion::V5_2)
        .parse_with_facts(aligned_source, &invariant)
        .unwrap();

    let old_constant =
        SemanticFacts::new().with_integer_evaluation(aligned, IntegerEvaluation::NonNegative(64));
    omp_exact(OpenMpVersion::V5_1)
        .parse_with_facts(aligned_source, &old_constant)
        .unwrap();

    let teams_source = "#pragma omp teams num_teams(lower: upper)";
    let clause = OmpClauseSite::new(OmpClauseKind::NumTeams, 0);
    let lower = OmpExpressionSite::new(clause, 0);
    let upper = OmpExpressionSite::new(clause, 1);
    let bounds = SemanticFacts::new()
        .with_positive_integer_expression(lower, true)
        .with_positive_integer_expression(upper, true)
        .with_ordered_bounds(clause, true);
    assert_eq!(
        omp()
            .parse_with_facts(teams_source, &bounds)
            .unwrap_err()
            .code(),
        DiagnosticCode::MissingSemanticFact
    );
    omp()
        .parse_with_facts(teams_source, &bounds.with_ultimate_expression(lower, true))
        .unwrap();
}

#[test]
fn domain_specific_expression_facts_are_required() {
    let hint_source = "#pragma omp atomic update hint(sync_hint)";
    let hint = OmpExpressionSite::new(OmpClauseSite::new(OmpClauseKind::Hint, 0), 0);
    assert_eq!(
        omp()
            .parse_with_facts(hint_source, &SemanticFacts::new())
            .unwrap_err()
            .code(),
        DiagnosticCode::MissingSemanticFact
    );
    omp()
        .parse_with_facts(
            hint_source,
            &SemanticFacts::new().with_synchronization_hint(hint, true),
        )
        .unwrap();

    let safesync_source = "#pragma omp parallel safesync(width)";
    let safesync = OmpExpressionSite::new(OmpClauseSite::new(OmpClauseKind::Safesync, 0), 0);
    let width = SemanticFacts::new().with_positive_integer_expression(safesync, true);
    assert_eq!(
        omp()
            .parse_with_facts(safesync_source, &width)
            .unwrap_err()
            .code(),
        DiagnosticCode::MissingSemanticFact
    );
    omp()
        .parse_with_facts(
            safesync_source,
            &width.with_safesync_compatible(safesync, true),
        )
        .unwrap();

    let transparent_source = "#pragma omp task transparent(mode)";
    let transparent = OmpExpressionSite::new(OmpClauseSite::new(OmpClauseKind::Transparent, 0), 0);
    assert_eq!(
        omp()
            .parse_with_facts(transparent_source, &SemanticFacts::new())
            .unwrap_err()
            .code(),
        DiagnosticCode::MissingSemanticFact
    );
    omp()
        .parse_with_facts(
            transparent_source,
            &SemanticFacts::new().with_impex_expression(transparent, true),
        )
        .unwrap();
}

#[test]
fn allocate_allocator_and_alignment_use_source_ordered_expression_sites() {
    let source = "#pragma omp parallel private(storage) allocate(allocator(selected_allocator), align(requested_alignment): storage)";
    let clause = OmpClauseSite::new(OmpClauseKind::Allocate, 0);
    let allocator = OmpExpressionSite::new(clause, 0);
    let alignment = OmpExpressionSite::new(clause, 1);

    let error = omp()
        .parse_with_facts(source, &SemanticFacts::new())
        .unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::MissingSemanticFact);

    let allocator_only = SemanticFacts::new().with_allocator_handle_expression(allocator, true);
    let error = omp().parse_with_facts(source, &allocator_only).unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::MissingSemanticFact);

    let valid = allocator_only
        .clone()
        .with_integer_evaluation(alignment, IntegerEvaluation::NonNegative(64));
    omp().parse_with_facts(source, &valid).unwrap();

    let wrong_allocator = valid
        .clone()
        .with_allocator_handle_expression(allocator, false);
    assert_eq!(
        omp()
            .parse_with_facts(source, &wrong_allocator)
            .unwrap_err()
            .code(),
        DiagnosticCode::InvalidExpressionType
    );

    let non_power_of_two =
        allocator_only.with_integer_evaluation(alignment, IntegerEvaluation::NonNegative(24));
    assert_eq!(
        omp()
            .parse_with_facts(source, &non_power_of_two)
            .unwrap_err()
            .code(),
        DiagnosticCode::InvalidClause
    );
}

#[test]
fn obvious_scalar_constraint_violations_fail_plain_parse() {
    for source in [
        "#pragma omp parallel num_threads(0)",
        "#pragma omp for schedule(static, 0)",
        "#pragma omp taskloop grainsize(-1)",
        "#pragma omp teams num_teams(0)",
        "#pragma omp task priority(-1)",
        "#pragma omp simd safelen(0)",
        "#pragma omp simd safelen(4) simdlen(8)",
        "#pragma omp for collapse(2) ordered(1)",
        "#pragma omp masked filter(\"thread\")",
        "#pragma omp target device(ancestor: 2)",
        "#pragma omp taskgraph graph_id(\"graph\")",
    ] {
        assert!(
            omp().parse(source).is_err(),
            "accepted an obvious scalar violation in {source:?}"
        );
    }
}

#[test]
fn dynamic_scalar_constraints_require_expression_specific_facts() {
    let source = "#pragma omp parallel num_threads(requested_threads)";
    let site = OmpExpressionSite::new(OmpClauseSite::new(OmpClauseKind::NumThreads, 0), 0);
    let error = omp()
        .parse_with_facts(source, &SemanticFacts::new())
        .unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::MissingSemanticFact);

    let invalid = SemanticFacts::new().with_positive_integer_expression(site, false);
    let error = omp().parse_with_facts(source, &invalid).unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::InvalidExpressionType);

    let valid = SemanticFacts::new().with_positive_integer_expression(site, true);
    omp().parse_with_facts(source, &valid).unwrap();

    let contradictory = SemanticFacts::new()
        .with_integer_evaluation(site, IntegerEvaluation::NonNegative(4))
        .with_positive_integer_expression(site, false);
    let error = omp().parse_with_facts(source, &contradictory).unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::InvalidConfiguration);

    let contradictory = SemanticFacts::new()
        .with_integer_evaluation(site, IntegerEvaluation::NonNegative(4))
        .with_constant_expression(site, false);
    let error = omp().parse_with_facts(source, &contradictory).unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::InvalidConfiguration);

    let priority_source = "#pragma omp task priority(task_priority)";
    let priority = OmpExpressionSite::new(OmpClauseSite::new(OmpClauseKind::Priority, 0), 0);
    let facts =
        SemanticFacts::new().with_integer_evaluation(priority, IntegerEvaluation::NonNegative(7));
    omp().parse_with_facts(priority_source, &facts).unwrap();
}
