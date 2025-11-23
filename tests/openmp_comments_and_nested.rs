use roup::parser::{clause::ReductionOperator, ClauseKind, Parser};
use std::borrow::Cow;

fn parse(input: &str) -> roup::parser::Directive<'_> {
    let parser = Parser::default();
    let (_, directive) = parser.parse(input).expect("directive should parse");
    directive
}

#[test]
fn parses_clause_with_nested_parentheses() {
    // content contains nested parentheses which should be preserved inside the clause
    let directive = parse("#pragma omp for reduction(max:(f(a), g(b))) private(i)");

    assert_eq!(directive.name, "for");
    assert_eq!(directive.clauses.len(), 2);
    assert_eq!(directive.clauses[0].name, "reduction");
    match &directive.clauses[0].kind {
        ClauseKind::ReductionClause {
            operator,
            user_defined_identifier,
            variables,
            ..
        } => {
            assert_eq!(*operator, ReductionOperator::Max);
            assert!(user_defined_identifier.is_none());
            assert_eq!(variables, &vec![Cow::from("(f(a), g(b))")]);
        }
        other => panic!("expected reduction clause, got {other:?}"),
    }
    assert_eq!(
        directive.clauses[1].kind,
        ClauseKind::Parenthesized("i".into())
    );
}

#[test]
fn parses_pragma_with_comments_inside() {
    // comments (C and C++ style) should be allowed inside the pragma between tokens
    let directive = parse("#pragma omp parallel /* comment */ private(a) // end-line comment\n");

    assert_eq!(directive.name, "parallel");
    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "private");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("a".into())
    );
}
