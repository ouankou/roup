use roup::parser::{ClauseKind, Parser};

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
    assert_eq!(
        directive.clauses[3].kind,
        ClauseKind::Parenthesized("+:sum".into()),
    );
}

#[test]
fn rejects_parallel_with_unknown_clause() {
    let parser = Parser::default();
    let result = parser.parse("#pragma omp parallel unsupported_clause");

    assert!(result.is_err());
}
