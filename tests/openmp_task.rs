use roup::parser::{ClauseKind, Parser};

fn parse(input: &str) -> roup::parser::Directive<'_> {
    let parser = Parser::default();
    let (_, directive) = parser.parse(input).expect("directive should parse");
    directive
}

#[test]
fn parses_task_with_dependencies() {
    let directive = parse(
        "#pragma omp task if(inbranch) final(true) priority(3) depend(inout:buf) detach(evt)",
    );

    assert_eq!(directive.name, "task");
    assert_eq!(directive.clauses.len(), 5);
    assert_eq!(directive.clauses[0].name, "if");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("inbranch")
    );
    assert_eq!(directive.clauses[1].name, "final");
    assert_eq!(directive.clauses[1].kind, ClauseKind::Parenthesized("true"));
    assert_eq!(directive.clauses[2].name, "priority");
    assert_eq!(directive.clauses[2].kind, ClauseKind::Parenthesized("3"));
    assert_eq!(directive.clauses[3].name, "depend");
    assert_eq!(
        directive.clauses[3].kind,
        ClauseKind::Parenthesized("inout:buf")
    );
    assert_eq!(directive.clauses[4].name, "detach");
    assert_eq!(directive.clauses[4].kind, ClauseKind::Parenthesized("evt"));
}

#[test]
fn parses_taskloop_simd_with_grainsize() {
    let directive = parse(
        "#pragma omp taskloop simd grainsize(4) num_tasks(16) reduction(max:max_val) shared(out)",
    );

    assert_eq!(directive.name, "taskloop simd");
    assert_eq!(directive.clauses.len(), 4);
    assert_eq!(directive.clauses[0].name, "grainsize");
    assert_eq!(directive.clauses[0].kind, ClauseKind::Parenthesized("4"));
    assert_eq!(directive.clauses[1].name, "num_tasks");
    assert_eq!(directive.clauses[1].kind, ClauseKind::Parenthesized("16"));
    assert_eq!(directive.clauses[2].name, "reduction");
    assert_eq!(
        directive.clauses[2].kind,
        ClauseKind::Parenthesized("max:max_val")
    );
    assert_eq!(directive.clauses[3].name, "shared");
    assert_eq!(directive.clauses[3].kind, ClauseKind::Parenthesized("out"));
}
