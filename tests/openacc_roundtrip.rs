use roup::parser::{openacc, parse_acc_directive, ClauseKind};

#[test]
fn parses_basic_parallel_loop() {
    let input = "#pragma acc parallel loop gang vector tile(32)";
    let (_, directive) = parse_acc_directive(input).expect("should parse");

    assert_eq!(directive.name, "parallel loop");
    assert_eq!(directive.clauses.len(), 3);
    assert_eq!(directive.clauses[0].name, "gang");
    assert_eq!(directive.clauses[0].kind, ClauseKind::Bare);
    assert_eq!(directive.clauses[1].name, "vector");
    assert_eq!(directive.clauses[1].kind, ClauseKind::Bare);
    assert_eq!(directive.clauses[2].name, "tile");
    assert_eq!(
        directive.clauses[2].kind,
        ClauseKind::Parenthesized("32".into())
    );

    let roundtrip = directive.to_pragma_string_with_prefix("#pragma acc");
    assert_eq!(roundtrip, "#pragma acc parallel loop gang vector tile(32)");
}

#[test]
fn parses_wait_directive_with_clauses() {
    let parser = openacc::parser();
    let input = "#pragma acc wait(1) async(2)";
    let (_, directive) = parser.parse(input).expect("should parse");

    assert_eq!(directive.name, "wait(1)");
    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "async");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("2".into())
    );

    let roundtrip = directive.to_pragma_string_with_prefix("#pragma acc");
    assert_eq!(roundtrip, "#pragma acc wait(1) async(2)");
}
