use roup::api::{OpenMpConfig, OpenMpParser};
use roup::ast::OmpClauseKind;
use roup::diagnostic::DiagnosticCode;
use roup::validation::{
    DependObjectState, DetachEventStatus, InteropObjectState, OmpClauseItemSite, OmpClauseSite,
    OmpExpressionSite, SemanticFacts,
};
use roup::version::{CStandard, HostLanguageProfile, SourceForm};

fn omp() -> OpenMpParser {
    OpenMpConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .unwrap()
        .parser()
}

fn item(kind: OmpClauseKind) -> OmpClauseItemSite {
    OmpClauseItemSite::new(OmpClauseSite::new(kind, 0), 0)
}

fn expression(kind: OmpClauseKind) -> OmpExpressionSite {
    OmpExpressionSite::new(OmpClauseSite::new(kind, 0), 0)
}

#[test]
fn declare_induction_requires_typed_collector_and_inductor_facts() {
    let source = "#pragma omp declare_induction(step : int) inductor(omp_var += omp_step) collector(omp_step * omp_idx)";
    let positioned = SemanticFacts::new().with_declaration_position(true);
    assert_eq!(
        omp()
            .parse_with_facts(source, &positioned)
            .unwrap_err()
            .code(),
        DiagnosticCode::MissingSemanticFact
    );

    let invalid = positioned
        .clone()
        .with_inductor_expression(expression(OmpClauseKind::Inductor), false)
        .with_collector_expression(expression(OmpClauseKind::Collector), true);
    assert_eq!(
        omp().parse_with_facts(source, &invalid).unwrap_err().code(),
        DiagnosticCode::InvalidExpressionType
    );

    let valid = positioned
        .with_inductor_expression(expression(OmpClauseKind::Inductor), true)
        .with_collector_expression(expression(OmpClauseKind::Collector), true);
    omp().parse_with_facts(source, &valid).unwrap();
}

#[test]
fn detach_requires_event_and_encountering_task_facts() {
    let source = "#pragma omp task detach(event)";
    let site = item(OmpClauseKind::Detach);
    assert_eq!(
        omp()
            .parse_with_facts(source, &SemanticFacts::new())
            .unwrap_err()
            .code(),
        DiagnosticCode::MissingSemanticFact
    );

    let wrong_type = SemanticFacts::new()
        .with_detach_event(site, DetachEventStatus::WrongType)
        .with_encountering_final_task(false);
    assert_eq!(
        omp()
            .parse_with_facts(source, &wrong_type)
            .unwrap_err()
            .code(),
        DiagnosticCode::InvalidExpressionType
    );

    let final_task = SemanticFacts::new()
        .with_detach_event(site, DetachEventStatus::Valid)
        .with_encountering_final_task(true);
    assert_eq!(
        omp()
            .parse_with_facts(source, &final_task)
            .unwrap_err()
            .code(),
        DiagnosticCode::InvalidAssociation
    );

    let valid = SemanticFacts::new()
        .with_detach_event(site, DetachEventStatus::Valid)
        .with_encountering_final_task(false);
    omp().parse_with_facts(source, &valid).unwrap();

    assert_eq!(
        omp()
            .parse("#pragma omp task detach(event) private(event)")
            .unwrap_err()
            .code(),
        DiagnosticCode::ConflictingClauses
    );
}

#[test]
fn interop_actions_require_object_state_and_modifiability() {
    let init_source = "#pragma omp interop init(target: object)";
    let init_site = item(OmpClauseKind::Init);
    let typed = SemanticFacts::new()
        .with_interop_object(init_site, InteropObjectState::Uninitialized)
        .with_modifiable_item(init_site, true);
    omp().parse_with_facts(init_source, &typed).unwrap();

    let constant = SemanticFacts::new()
        .with_interop_object(init_site, InteropObjectState::Uninitialized)
        .with_modifiable_item(init_site, false);
    assert_eq!(
        omp()
            .parse_with_facts(init_source, &constant)
            .unwrap_err()
            .code(),
        DiagnosticCode::InvalidClause
    );

    let use_source = "#pragma omp interop use(object)";
    let use_site = item(OmpClauseKind::Use);
    let uninitialized =
        SemanticFacts::new().with_interop_object(use_site, InteropObjectState::Uninitialized);
    assert_eq!(
        omp()
            .parse_with_facts(use_source, &uninitialized)
            .unwrap_err()
            .code(),
        DiagnosticCode::InvalidClause
    );
    omp()
        .parse_with_facts(
            use_source,
            &SemanticFacts::new().with_interop_object(use_site, InteropObjectState::Initialized),
        )
        .unwrap();

    assert_eq!(
        omp()
            .parse("#pragma omp interop init(target: object) use(object)")
            .unwrap_err()
            .code(),
        DiagnosticCode::ConflictingClauses
    );
}

#[test]
fn interop_depend_requires_targetsync_semantics() {
    let target = "#pragma omp interop init(target: object) depend(in: token)";
    let init_site = item(OmpClauseKind::Init);
    let init_facts = SemanticFacts::new()
        .with_interop_object(init_site, InteropObjectState::Uninitialized)
        .with_modifiable_item(init_site, true);
    assert_eq!(
        omp()
            .parse_with_facts(target, &init_facts)
            .unwrap_err()
            .code(),
        DiagnosticCode::ClauseNotAllowed
    );
    omp()
        .parse_with_facts(
            "#pragma omp interop init(targetsync: object) depend(in: token)",
            &init_facts,
        )
        .unwrap();

    let use_source = "#pragma omp interop use(object) depend(in: token)";
    let use_clause = OmpClauseSite::new(OmpClauseKind::Use, 0);
    let use_facts = SemanticFacts::new()
        .with_interop_object(item(OmpClauseKind::Use), InteropObjectState::Initialized)
        .with_interop_targetsync(use_clause, true);
    omp().parse_with_facts(use_source, &use_facts).unwrap();
}

#[test]
fn depobj_actions_require_the_correct_object_state() {
    let init_source = "#pragma omp depobj init(in(token): object)";
    let init_site = item(OmpClauseKind::Init);
    let initialized = SemanticFacts::new()
        .with_depend_object(init_site, DependObjectState::Initialized)
        .with_modifiable_item(init_site, true);
    assert_eq!(
        omp()
            .parse_with_facts(init_source, &initialized)
            .unwrap_err()
            .code(),
        DiagnosticCode::InvalidClause
    );
    let uninitialized = SemanticFacts::new()
        .with_depend_object(init_site, DependObjectState::Uninitialized)
        .with_modifiable_item(init_site, true);
    omp().parse_with_facts(init_source, &uninitialized).unwrap();

    let update_source = "#pragma omp depobj update(inout: object)";
    let update_site = item(OmpClauseKind::DepobjUpdate);
    omp()
        .parse_with_facts(
            update_source,
            &SemanticFacts::new().with_depend_object(update_site, DependObjectState::Initialized),
        )
        .unwrap();

    let historical = "#pragma omp depobj(object) depend(in: token)";
    let depend_site = item(OmpClauseKind::Depend);
    omp()
        .parse_with_facts(
            historical,
            &SemanticFacts::new()
                .with_depend_object(depend_site, DependObjectState::Uninitialized)
                .with_modifiable_item(depend_site, true),
        )
        .unwrap();
}
