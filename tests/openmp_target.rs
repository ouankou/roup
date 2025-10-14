use roup::parser::{ClauseKind, Parser};

fn parse(input: &str) -> roup::parser::Directive<'_> {
    let parser = Parser::default();
    let (_, directive) = parser.parse(input).expect("directive should parse");
    directive
}

#[test]
fn parses_target_with_mapping_clauses() {
    let directive = parse("#pragma omp target if(device) device(0) map(tofrom:array[0:N]) nowait");

    assert_eq!(directive.name, "target");
    assert_eq!(directive.clauses.len(), 4);
    assert_eq!(directive.clauses[0].name, "if");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("device".into())
    );
    assert_eq!(directive.clauses[1].name, "device");
    assert_eq!(
        directive.clauses[1].kind,
        ClauseKind::Parenthesized("0".into())
    );
    assert_eq!(directive.clauses[2].name, "map");
    assert_eq!(
        directive.clauses[2].kind,
        ClauseKind::Parenthesized("tofrom:array[0:N]".into()),
    );
    assert_eq!(directive.clauses[3].name, "nowait");
    assert_eq!(directive.clauses[3].kind, ClauseKind::Bare);
}

#[test]
fn parses_target_teams_distribute_parallel_for_simd() {
    let directive = parse(
        "#pragma omp target teams distribute parallel for simd num_teams(4) thread_limit(128) \
         schedule(dynamic,8) reduction(*:prod) uses_allocators(omp_default_mem_alloc)",
    );

    assert_eq!(directive.name, "target teams distribute parallel for simd");
    assert_eq!(directive.clauses.len(), 5);
    assert_eq!(directive.clauses[0].name, "num_teams");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("4".into())
    );
    assert_eq!(directive.clauses[1].name, "thread_limit");
    assert_eq!(
        directive.clauses[1].kind,
        ClauseKind::Parenthesized("128".into())
    );
    assert_eq!(directive.clauses[2].name, "schedule");
    assert_eq!(
        directive.clauses[2].kind,
        ClauseKind::Parenthesized("dynamic,8".into())
    );
    assert_eq!(directive.clauses[3].name, "reduction");
    assert_eq!(
        directive.clauses[3].kind,
        ClauseKind::Parenthesized("*:prod".into())
    );
    assert_eq!(directive.clauses[4].name, "uses_allocators");
    assert_eq!(
        directive.clauses[4].kind,
        ClauseKind::Parenthesized("omp_default_mem_alloc".into()),
    );
}
