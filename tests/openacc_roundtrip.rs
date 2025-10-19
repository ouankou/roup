use roup::lexer::Language;
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

#[test]
fn parses_present_or_copy_data_clause() {
    let input = "#pragma acc data present_or_copy(a)";
    let (_, directive) = parse_acc_directive(input).expect("should parse");

    assert_eq!(directive.name, "data");
    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "present_or_copy");
    assert!(matches!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized(_)
    ));

    let roundtrip = directive.to_pragma_string_with_prefix("#pragma acc");
    assert_eq!(roundtrip, "#pragma acc data present_or_copy(a)");
}

#[test]
fn parses_pcopy_alias_clause() {
    let input = "#pragma acc data pcopy(b)";
    let (_, directive) = parse_acc_directive(input).expect("should parse");

    assert_eq!(directive.name, "data");
    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "pcopy");
    assert!(matches!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized(_)
    ));

    let roundtrip = directive.to_pragma_string_with_prefix("#pragma acc");
    assert_eq!(roundtrip, "#pragma acc data pcopy(b)");
}

#[test]
fn fortran_present_or_copy_uppercase_roundtrip() {
    let parser = openacc::parser().with_language(Language::FortranFree);
    let input = "!$ACC DATA PRESENT_OR_COPY(A)";
    let (_, directive) = parser.parse(input).expect("should parse");

    assert_eq!(directive.name, "data");
    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "present_or_copy");

    let roundtrip = directive.to_pragma_string_with_prefix("!$acc");
    assert_eq!(roundtrip, "!$acc data present_or_copy(A)");
}
