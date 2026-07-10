use roup::api::{OpenMpConfig, OpenMpParser};
use roup::ast::{OmpClauseKind, OmpDirectiveKind};
use roup::ir::{ClauseData, ClauseItem};
use roup::version::{CStandard, HostLanguageProfile, SourceForm};

fn parser() -> OpenMpParser {
    OpenMpConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .unwrap()
        .parser()
}

#[test]
fn comments_between_tokens_do_not_create_ast_nodes() {
    let parsed = parser()
        .parse("#pragma omp parallel /* comment */ private(a) // end-line comment\n")
        .expect("comments between directive tokens are valid");
    let directive = parsed.directive();

    assert_eq!(directive.kind(), OmpDirectiveKind::Parallel);
    assert_eq!(directive.clauses().len(), 1);
    assert_eq!(directive.clauses()[0].kind(), OmpClauseKind::Private);
    let ClauseData::Private { items } = directive.clauses()[0].payload() else {
        panic!("private must have a typed payload");
    };
    assert!(matches!(
        items.as_slice(),
        [ClauseItem::Identifier(identifier)] if identifier.as_str() == "a"
    ));
}

#[test]
fn nested_parentheses_do_not_justify_a_malformed_locator_fallback() {
    for source in [
        "#pragma omp for reduction(max:(f(a), g(b))) private(i)",
        "#pragma omp for reduction(+:make_value())",
        "#pragma omp for private((a + b))",
    ] {
        assert!(
            parser().parse(source).is_err(),
            "non-variable clause items must be hard errors: {source}"
        );
    }
}
