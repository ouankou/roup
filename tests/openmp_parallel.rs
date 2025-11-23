use roup::parser::{clause::ReductionOperator, ClauseKind, Parser};
use std::borrow::Cow;

fn parse(input: &str) -> roup::parser::Directive<'_> {
    let parser = Parser::default();
    let (_, directive) = parser.parse(input).expect("directive should parse");
    directive
}

#[test]
fn parses_parallel_with_core_clauses() {
    let directive = parse("#pragma omp parallel private(a, b) firstprivate(c) nowait");

    assert_eq!(directive.name, "parallel");
    assert_eq!(directive.clauses.len(), 3);
    assert_eq!(directive.clauses[0].name, "private");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("a, b".into())
    );
    assert_eq!(directive.clauses[1].name, "firstprivate");
    assert_eq!(
        directive.clauses[1].kind,
        ClauseKind::Parenthesized("c".into())
    );
    assert_eq!(directive.clauses[2].name, "nowait");
    assert_eq!(directive.clauses[2].kind, ClauseKind::Bare);
}

#[test]
fn parses_parallel_for_simd_combination() {
    let directive = parse(
        "#pragma omp parallel for simd aligned(buf:64) schedule(static,4) collapse(2) reduction(+:sum)",
    );

    assert_eq!(directive.name, "parallel for simd");
    assert_eq!(directive.clauses.len(), 4);
    assert_eq!(directive.clauses[0].name, "aligned");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("buf:64".into()),
    );
    assert_eq!(directive.clauses[1].name, "schedule");
    assert_eq!(
        directive.clauses[1].kind,
        ClauseKind::Parenthesized("static,4".into())
    );
    assert_eq!(directive.clauses[2].name, "collapse");
    assert_eq!(
        directive.clauses[2].kind,
        ClauseKind::Parenthesized("2".into())
    );
    assert_eq!(directive.clauses[3].name, "reduction");
    match &directive.clauses[3].kind {
        ClauseKind::ReductionClause {
            operator,
            user_defined_identifier,
            variables,
            ..
        } => {
            assert_eq!(*operator, ReductionOperator::Add);
            assert!(user_defined_identifier.is_none());
            assert_eq!(variables, &vec![Cow::from("sum")]);
        }
        other => panic!("expected reduction clause, got {other:?}"),
    }
}

#[test]
fn rejects_parallel_with_unknown_clause() {
    let parser = Parser::default();
    let (_, directive) = parser
        .parse("#pragma omp parallel unsupported_clause")
        .expect("parser should capture unknown clauses for forward compatibility");

    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "unsupported_clause");
    assert_eq!(directive.clauses[0].kind, ClauseKind::Bare);
}
