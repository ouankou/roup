use roup::parser::{ClauseKind, Parser};

fn parser() -> Parser {
    Parser::default()
}

#[test]
fn parses_reduction_clause_with_modifiers_and_operators() {
    let parser = parser();
    let source = "#pragma omp parallel for reduction(task,inscan,+:total) reduction(^:checksum) reduction(&&:all_true)";

    let (rest, directive) = parser
        .parse(source)
        .expect("parallel for reduction clauses should parse");

    assert!(rest.trim().is_empty());
    assert_eq!(directive.name, "parallel for");
    assert_eq!(directive.clauses.len(), 3);

    let expected = ["task,inscan,+:total", "^:checksum", "&&:all_true"];

    for (clause, expected_body) in directive.clauses.iter().zip(expected.into_iter()) {
        assert_eq!(clause.name, "reduction");
        match &clause.kind {
            ClauseKind::Parenthesized(body) => assert_eq!(body.as_ref(), expected_body),
            ClauseKind::Bare => panic!("reduction clauses should be parenthesized"),
            _ => panic!("unexpected clause kind for reduction"),
        }
    }
}

#[test]
fn parses_reduction_clause_with_user_defined_identifier() {
    let parser = parser();
    let source = "#pragma omp parallel reduction(user_addition:accumulator) reduction(task, custom_reducer:list)";

    let (_, directive) = parser
        .parse(source)
        .expect("reduction clauses with user identifiers should parse");

    assert_eq!(directive.name, "parallel");
    assert_eq!(directive.clauses.len(), 2);

    assert_eq!(directive.clauses[0].name, "reduction");
    match &directive.clauses[0].kind {
        ClauseKind::Parenthesized(body) => {
            assert_eq!(body.as_ref(), "user_addition:accumulator");
        }
        ClauseKind::Bare => panic!("reduction clause should be parenthesized"),
        _ => panic!("unexpected clause kind for reduction"),
    }

    assert_eq!(directive.clauses[1].name, "reduction");
    match &directive.clauses[1].kind {
        ClauseKind::Parenthesized(body) => {
            assert_eq!(body.as_ref(), "task, custom_reducer:list");
        }
        ClauseKind::Bare => panic!("reduction clause should be parenthesized"),
        _ => panic!("unexpected clause kind for reduction"),
    }
}
