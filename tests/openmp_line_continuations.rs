use roup::lexer::Language;
use roup::parser::{ClauseKind, Directive, Parser};

fn parse_with_language(input: &str, language: Language) -> Directive<'_> {
    let parser = Parser::default().with_language(language);
    let (_, directive) = parser.parse(input).expect("directive should parse");
    directive
}

#[test]
fn parses_c_multiline_with_backslash() {
    let directive = parse_with_language(
        concat!(
            "#pragma omp parallel for \\\n",
            "    schedule(dynamic, 4) \\\n",
            "    private(i, \\\n",
            "            j)"
        ),
        Language::C,
    );

    assert_eq!(directive.name, "parallel for");
    assert_eq!(directive.clauses.len(), 2);
    assert_eq!(directive.clauses[0].name, "schedule");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("dynamic, 4".into())
    );
    assert_eq!(directive.clauses[1].name, "private");
    assert_eq!(
        directive.clauses[1].kind,
        ClauseKind::Parenthesized("i, j".into())
    );
}

#[test]
fn parses_c_multiline_with_comments() {
    let directive = parse_with_language(
        concat!(
            "#pragma omp parallel for \\\n",
            "    /* align workers */ schedule(static, 2) \\\n",
            "    // items below\n",
            "    private(a)"
        ),
        Language::C,
    );

    assert_eq!(directive.name, "parallel for");
    assert_eq!(directive.clauses.len(), 2);
    assert_eq!(directive.clauses[0].name, "schedule");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("static, 2".into())
    );
    assert_eq!(directive.clauses[1].name, "private");
    assert_eq!(
        directive.clauses[1].kind,
        ClauseKind::Parenthesized("a".into())
    );
}

#[test]
fn parses_fortran_free_with_ampersand() {
    let directive = parse_with_language(
        concat!("!$omp parallel do &\n", "!$omp private(i, &\n", "!$omp& j)"),
        Language::FortranFree,
    );

    assert_eq!(directive.name, "parallel do");
    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "private");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("i, j".into())
    );
}

#[test]
fn parses_fortran_free_with_comment_continuation() {
    let directive = parse_with_language(
        concat!(
            "!$omp parallel do private(i, & ! trailing comment\n",
            "!$omp& j, k)"
        ),
        Language::FortranFree,
    );

    assert_eq!(directive.name, "parallel do");
    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "private");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("i, j, k".into())
    );
}

#[test]
fn parses_fortran_fixed_with_mixed_sentinels() {
    let directive = parse_with_language(
        concat!(
            "      !$OMP TEAMS DISTRIBUTE &\n",
            "      C$OMP& PARALLEL DO &\n",
            "      !$OMP PRIVATE(I) SHARED(A)"
        ),
        Language::FortranFixed,
    );

    assert_eq!(directive.name, "teams distribute parallel do");
    assert_eq!(directive.clauses.len(), 2);
    assert_eq!(directive.clauses[0].name, "private");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("I".into())
    );
    assert_eq!(directive.clauses[1].name, "shared");
    assert_eq!(
        directive.clauses[1].kind,
        ClauseKind::Parenthesized("A".into())
    );
}
