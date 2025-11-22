use roup::parser::{clause::ReductionOperator, ClauseKind, Parser};
use std::borrow::Cow;

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
        ClauseKind::Parenthesized("inbranch".into())
    );
    assert_eq!(directive.clauses[1].name, "final");
    assert_eq!(
        directive.clauses[1].kind,
        ClauseKind::Parenthesized("true".into())
    );
    assert_eq!(directive.clauses[2].name, "priority");
    assert_eq!(
        directive.clauses[2].kind,
        ClauseKind::Parenthesized("3".into())
    );
    assert_eq!(directive.clauses[3].name, "depend");
    assert_eq!(
        directive.clauses[3].kind,
        ClauseKind::Parenthesized("inout:buf".into())
    );
    assert_eq!(directive.clauses[4].name, "detach");
    assert_eq!(
        directive.clauses[4].kind,
        ClauseKind::Parenthesized("evt".into())
    );
}

#[test]
fn parses_taskloop_simd_with_grainsize() {
    let directive = parse(
        "#pragma omp taskloop simd grainsize(4) num_tasks(16) reduction(max:max_val) shared(out)",
    );

    assert_eq!(directive.name, "taskloop simd");
    assert_eq!(directive.clauses.len(), 4);
    assert_eq!(directive.clauses[0].name, "grainsize");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("4".into())
    );
    assert_eq!(directive.clauses[1].name, "num_tasks");
    assert_eq!(
        directive.clauses[1].kind,
        ClauseKind::Parenthesized("16".into())
    );
    assert_eq!(directive.clauses[2].name, "reduction");
    match &directive.clauses[2].kind {
        ClauseKind::ReductionClause {
            operator,
            user_defined_identifier,
            variables,
            ..
        } => {
            assert_eq!(*operator, ReductionOperator::Max);
            assert!(user_defined_identifier.is_none());
            assert_eq!(variables, &vec![Cow::from("max_val")]);
        }
        other => panic!("expected reduction clause, got {other:?}"),
    }
    assert_eq!(directive.clauses[3].name, "shared");
    assert_eq!(
        directive.clauses[3].kind,
        ClauseKind::Parenthesized("out".into())
    );
}
