use roup::parser::{openacc, parse_acc_directive, ClauseKind};

#[test]
fn parses_basic_parallel_loop() {
    let input = "#pragma acc parallel loop gang vector tile(32)";
    let (_, directive) = parse_acc_directive(input).expect("should parse");

    assert_eq!(directive.name, "parallel loop");
    assert_eq!(directive.clauses.len(), 3);
    assert_eq!(directive.clauses[0].name, "gang");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::GangClause {
            modifier: None,
            variables: vec![]
        }
    );
    assert_eq!(directive.clauses[1].name, "vector");
    assert_eq!(
        directive.clauses[1].kind,
        ClauseKind::VectorClause {
            modifier: None,
            variables: vec![]
        }
    );
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

    assert_eq!(directive.name, "wait");
    assert_eq!(directive.parameter, Some("(1)".into()));
    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "async");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("2".into())
    );

    let roundtrip = directive.to_pragma_string_with_prefix("#pragma acc");
    assert_eq!(roundtrip, "#pragma acc wait(1) async(2)");
}

#[test]
fn roundtrip_cache_directive_with_clauses() {
    let parser = openacc::parser();
    let input = "#pragma acc cache(arr[0:10]) async(3)";
    let (_, directive) = parser.parse(input).expect("should parse");

    assert_eq!(directive.name, "cache");
    assert_eq!(directive.parameter, Some("(arr[0: 10])".into()));
    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "async");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("3".into())
    );

    let roundtrip = directive.to_pragma_string_with_prefix("#pragma acc");
    assert_eq!(roundtrip, "#pragma acc cache(arr[0: 10]) async(3)");
}

#[test]
fn parses_alias_clauses_without_renaming() {
    let parser = openacc::parser();
    let input = "#pragma acc data pcopy(a) present_or_copy(b) pcreate(c) present_or_create(d)";
    let (_, directive) = parser.parse(input).expect("should parse");

    let names: Vec<_> = directive.clauses.iter().map(|c| c.name.as_ref()).collect();
    assert_eq!(
        names,
        vec!["pcopy", "present_or_copy", "pcreate", "present_or_create"]
    );

    for clause in &directive.clauses {
        assert!(matches!(
            clause.kind,
            ClauseKind::Parenthesized(_)
                | ClauseKind::VariableList(_)
                | ClauseKind::CopyinClause { .. }
                | ClauseKind::CopyoutClause { .. }
                | ClauseKind::CreateClause { .. }
        ));
    }

    let roundtrip = directive.to_pragma_string_with_prefix("#pragma acc");
    assert_eq!(roundtrip, input);
}

#[test]
fn parses_dtype_alias_on_loop_directive() {
    let parser = openacc::parser();
    let input = "#pragma acc loop dtype(*) vector";
    let (_, directive) = parser.parse(input).expect("should parse");

    assert_eq!(directive.clauses.len(), 2);
    assert_eq!(directive.clauses[0].name, "dtype");
    assert!(matches!(
        directive.clauses[0].kind,
        ClauseKind::VariableList(_) | ClauseKind::Parenthesized(_)
    ));
    assert_eq!(directive.clauses[1].name, "vector");
    assert_eq!(
        directive.clauses[1].kind,
        ClauseKind::VectorClause {
            modifier: None,
            variables: vec![]
        }
    );

    let roundtrip = directive.to_pragma_string_with_prefix("#pragma acc");
    assert_eq!(roundtrip, input);
}

#[test]
fn parses_atomic_update_as_bare_clause() {
    let parser = openacc::parser();
    let input = "#pragma acc atomic update";
    let (_, directive) = parser.parse(input).expect("should parse");

    assert_eq!(directive.name, "atomic");
    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "update");
    assert_eq!(directive.clauses[0].kind, ClauseKind::Bare);

    let roundtrip = directive.to_pragma_string_with_prefix("#pragma acc");
    assert_eq!(roundtrip, input);
}
