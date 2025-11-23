use roup::parser::{clause::ReductionOperator, ClauseKind, Parser};
use std::borrow::Cow;

fn parse(input: &str) -> roup::parser::Directive<'_> {
    let parser = Parser::default();
    let (_, directive) = parser.parse(input).expect("directive should parse");
    directive
}

#[test]
fn parses_teams_with_reductions() {
    let directive = parse("#pragma omp teams num_teams(8) thread_limit(32) reduction(+:total)");

    assert_eq!(directive.name, "teams");
    assert_eq!(directive.clauses.len(), 3);
    assert_eq!(directive.clauses[0].name, "num_teams");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("8".into())
    );
    assert_eq!(directive.clauses[1].name, "thread_limit");
    assert_eq!(
        directive.clauses[1].kind,
        ClauseKind::Parenthesized("32".into())
    );
    assert_eq!(directive.clauses[2].name, "reduction");
    match &directive.clauses[2].kind {
        ClauseKind::ReductionClause {
            operator,
            user_defined_identifier,
            variables,
            ..
        } => {
            assert_eq!(*operator, ReductionOperator::Add);
            assert!(user_defined_identifier.is_none());
            assert_eq!(variables, &vec![Cow::from("total")]);
        }
        other => panic!("expected reduction clause, got {other:?}"),
    }
}

#[test]
fn parses_teams_distribute_parallel_loop() {
    let directive = parse(
        "#pragma omp teams distribute parallel loop collapse(3) allocate(pmem:buf) order(concurrent)",
    );

    assert_eq!(directive.name, "teams distribute parallel loop");
    assert_eq!(directive.clauses.len(), 3);
    assert_eq!(directive.clauses[0].name, "collapse");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("3".into())
    );
    assert_eq!(directive.clauses[1].name, "allocate");
    assert_eq!(
        directive.clauses[1].kind,
        ClauseKind::Parenthesized("pmem:buf".into())
    );
    assert_eq!(directive.clauses[2].name, "order");
    assert_eq!(
        directive.clauses[2].kind,
        ClauseKind::Parenthesized("concurrent".into())
    );
}

#[test]
fn parses_teams_distribute_with_dist_schedule() {
    let directive = parse("#pragma omp teams distribute dist_schedule(static,4) collapse(2)");

    assert_eq!(directive.name, "teams distribute");
    assert_eq!(directive.clauses.len(), 2);
    assert_eq!(directive.clauses[0].name, "dist_schedule");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("static,4".into()),
    );
    assert_eq!(directive.clauses[1].name, "collapse");
    assert_eq!(
        directive.clauses[1].kind,
        ClauseKind::Parenthesized("2".into())
    );
}
