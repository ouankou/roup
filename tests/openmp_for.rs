use roup::parser::{clause::ReductionOperator, ClauseKind, Parser};
use std::borrow::Cow;

fn parse(input: &str) -> roup::parser::Directive<'_> {
    let parser = Parser::default();
    let (_, directive) = parser.parse(input).expect("directive should parse");
    directive
}

#[test]
fn parses_for_with_iteration_clauses() {
    let directive = parse("#pragma omp for schedule(guided,16) ordered(2) private(i, j)");

    assert_eq!(directive.name, "for");
    assert_eq!(directive.clauses.len(), 3);
    assert_eq!(directive.clauses[0].name, "schedule");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("guided,16".into())
    );
    assert_eq!(directive.clauses[1].name, "ordered");
    assert_eq!(
        directive.clauses[1].kind,
        ClauseKind::Parenthesized("2".into())
    );
    assert_eq!(directive.clauses[2].name, "private");
    assert_eq!(
        directive.clauses[2].kind,
        ClauseKind::Parenthesized("i, j".into())
    );
}

#[test]
fn parses_for_simd_with_linear_clause() {
    let directive =
        parse("#pragma omp for simd linear(x:2) safelen(8) simdlen(4) reduction(-:diff)");

    assert_eq!(directive.name, "for simd");
    assert_eq!(directive.clauses.len(), 4);
    assert_eq!(directive.clauses[0].name, "linear");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("x:2".into())
    );
    assert_eq!(directive.clauses[1].name, "safelen");
    assert_eq!(
        directive.clauses[1].kind,
        ClauseKind::Parenthesized("8".into())
    );
    assert_eq!(directive.clauses[2].name, "simdlen");
    assert_eq!(
        directive.clauses[2].kind,
        ClauseKind::Parenthesized("4".into())
    );
    assert_eq!(directive.clauses[3].name, "reduction");
    match &directive.clauses[3].kind {
        ClauseKind::ReductionClause {
            operator,
            user_defined_identifier,
            variables,
            ..
        } => {
            assert_eq!(*operator, ReductionOperator::Sub);
            assert!(user_defined_identifier.is_none());
            assert_eq!(variables, &vec![Cow::from("diff")]);
        }
        other => panic!("expected reduction clause, got {other:?}"),
    }
}

#[test]
fn parses_for_with_bare_ordered_clause() {
    let directive = parse("#pragma omp for ordered nowait");

    assert_eq!(directive.name, "for");
    assert_eq!(directive.clauses.len(), 2);
    assert_eq!(directive.clauses[0].name, "ordered");
    assert_eq!(directive.clauses[0].kind, ClauseKind::Bare);
    assert_eq!(directive.clauses[1].name, "nowait");
    assert_eq!(directive.clauses[1].kind, ClauseKind::Bare);
}
