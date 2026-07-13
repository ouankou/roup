use roup::api::OpenMpConfig;
use roup::ast::OmpClauseKind;
use roup::diagnostic::DiagnosticCode;
use roup::validation::{
    IntegerEvaluation, OmpClauseSite, OmpDirectivePath, OmpExpressionSite, OmpNestedDirectiveRole,
    SemanticFacts,
};
use roup::version::{CStandard, HostLanguageProfile, SourceForm};

fn parser() -> roup::api::OpenMpParser {
    OpenMpConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .unwrap()
        .parser()
}

fn nested_expression_site(
    role: OmpNestedDirectiveRole,
    child_index: u32,
    kind: OmpClauseKind,
) -> OmpExpressionSite {
    let path = OmpDirectivePath::root()
        .child(role, child_index)
        .expect("one nested path segment must fit");
    OmpExpressionSite::new(OmpClauseSite::at(path, kind, 0), 0)
}

#[test]
fn nested_metadirective_and_construct_facts_are_required() {
    let variant = "#pragma omp metadirective when(device={kind(cpu)}: for collapse(n))";
    let error = parser()
        .parse_with_facts(variant, &SemanticFacts::new())
        .unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::MissingSemanticFact);

    let collapse = nested_expression_site(
        OmpNestedDirectiveRole::MetadirectiveVariant,
        0,
        OmpClauseKind::Collapse,
    );
    parser()
        .parse_with_facts(
            variant,
            &SemanticFacts::new()
                .with_integer_evaluation(collapse, IntegerEvaluation::NonNegative(2)),
        )
        .unwrap();

    let construct = "#pragma omp metadirective when(construct={simd(simdlen(n))}: parallel)";
    let error = parser()
        .parse_with_facts(construct, &SemanticFacts::new())
        .unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::MissingSemanticFact);

    let simdlen = nested_expression_site(
        OmpNestedDirectiveRole::ConstructSelector,
        0,
        OmpClauseKind::Simdlen,
    );
    parser()
        .parse_with_facts(
            construct,
            &SemanticFacts::new()
                .with_integer_evaluation(simdlen, IntegerEvaluation::NonNegative(4)),
        )
        .unwrap();
}

#[test]
fn sibling_nested_directives_do_not_share_facts() {
    let source = "#pragma omp metadirective when(device={kind(cpu)}: for collapse(a)) when(device={kind(gpu)}: for collapse(b))";
    let first = nested_expression_site(
        OmpNestedDirectiveRole::MetadirectiveVariant,
        0,
        OmpClauseKind::Collapse,
    );
    let incomplete =
        SemanticFacts::new().with_integer_evaluation(first, IntegerEvaluation::NonNegative(2));
    let error = parser().parse_with_facts(source, &incomplete).unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::MissingSemanticFact);

    let second = nested_expression_site(
        OmpNestedDirectiveRole::MetadirectiveVariant,
        1,
        OmpClauseKind::Collapse,
    );
    parser()
        .parse_with_facts(
            source,
            &incomplete.with_integer_evaluation(second, IntegerEvaluation::NonNegative(3)),
        )
        .unwrap();
}

#[test]
fn applied_directive_facts_use_the_applied_directive_path() {
    let source =
        "#pragma omp tile sizes(8, 8) apply(grid(1, 2): interchange permutation(m, 1), reverse)";
    let error = parser()
        .parse_with_facts(source, &SemanticFacts::new())
        .unwrap_err();
    assert_eq!(error.code(), DiagnosticCode::MissingSemanticFact);

    let path = OmpDirectivePath::root()
        .child(OmpNestedDirectiveRole::AppliedDirective, 4096)
        .unwrap();
    let m = OmpExpressionSite::new(OmpClauseSite::at(path, OmpClauseKind::Permutation, 0), 0);
    parser()
        .parse_with_facts(
            source,
            &SemanticFacts::new().with_integer_evaluation(m, IntegerEvaluation::NonNegative(2)),
        )
        .unwrap();
}
