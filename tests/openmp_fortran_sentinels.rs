use roup::lexer::Language;
use roup::parser::{ClauseKind, Directive, Parser};

fn parse_fixed(input: &str) -> Directive<'_> {
    let parser = Parser::default().with_language(Language::FortranFixed);
    let (_, directive) = parser.parse(input).expect("directive should parse");
    directive
}

fn parse_free(input: &str) -> Directive<'_> {
    let parser = Parser::default().with_language(Language::FortranFree);
    let (_, directive) = parser.parse(input).expect("directive should parse");
    directive
}

#[test]
fn fixed_form_supports_short_form_continuations() {
    let directive = parse_fixed(concat!(
        "      !$OMP PARALLEL DO &\n",
        "      !$& PRIVATE(I) &\n",
        "      !$& SHARED(A)"
    ));

    assert_eq!(directive.name, "parallel do");
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

#[test]
fn fixed_form_supports_alternate_comment_characters() {
    for sentinel in ["C$OMP", "C$", "*$OMP", "*$"] {
        let source = format!(
            "      {sentinel} TEAMS DISTRIBUTE &\n      {sentinel}& PARALLEL DO &\n      {sentinel}& PRIVATE(I)",
        );
        let directive = parse_fixed(&source);

        assert_eq!(directive.name, "teams distribute parallel do");
        assert_eq!(directive.clauses.len(), 1);
        assert_eq!(directive.clauses[0].name, "private");
        assert_eq!(
            directive.clauses[0].kind,
            ClauseKind::Parenthesized("I".into())
        );
    }
}

#[test]
fn fixed_form_accepts_short_sentinel_on_initial_line() {
    let directive = parse_fixed(concat!("      !$ PARALLEL DO &\n", "      !$& PRIVATE(I)"));

    assert_eq!(directive.name, "parallel do");
    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "private");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("I".into())
    );
}

#[test]
fn fixed_form_is_case_insensitive_for_sentinels() {
    let directive = parse_fixed(concat!(
        "      !$Omp TEAMS DISTRIBUTE &\n",
        "      !$oMP& PARALLEL DO &\n",
        "      !$OmP& PRIVATE(I)"
    ));

    assert_eq!(directive.name, "teams distribute parallel do");
    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "private");
}

#[test]
fn free_form_accepts_leading_ampersand_on_continuation_line() {
    let directive = parse_free(concat!("!$omp parallel do &\n", "& private(i, j)"));

    assert_eq!(directive.name, "parallel do");
    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "private");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("i, j".into())
    );
}

#[test]
fn free_form_accepts_ampersand_at_both_ends() {
    let directive = parse_free(concat!(
        "!$omp parallel do &\n",
        "!$omp& private(i, &\n",
        "!$omp& j)"
    ));

    assert_eq!(directive.name, "parallel do");
    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "private");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("i, j".into())
    );
}

#[test]
fn free_form_supports_multiple_consecutive_continuations() {
    let directive = parse_free(concat!(
        "!$omp target teams distribute &\n",
        "!$omp& parallel do &\n",
        "!$omp& schedule(dynamic) &\n",
        "!$omp& private(i, j)"
    ));

    assert_eq!(directive.name, "target teams distribute parallel do");
    assert_eq!(directive.clauses.len(), 2);
    assert_eq!(directive.clauses[0].name, "schedule");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("dynamic".into())
    );
    assert_eq!(directive.clauses[1].name, "private");
}

#[test]
fn free_form_handles_varying_indentation() {
    let directive = parse_free(concat!(
        "    !$omp parallel do &\n",
        "          !$omp& private(i) &\n",
        "!$omp& shared(a)"
    ));

    assert_eq!(directive.name, "parallel do");
    assert_eq!(directive.clauses.len(), 2);
    assert_eq!(directive.clauses[0].name, "private");
    assert_eq!(directive.clauses[1].name, "shared");
}

#[test]
fn free_form_continuation_inside_directive_name() {
    let directive = parse_free(concat!("!$omp parallel&\n", "!$omp& do schedule(static)"));

    assert_eq!(directive.name, "parallel do");
    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "schedule");
}

#[test]
fn free_form_continuation_inside_clause() {
    let directive = parse_free(concat!(
        "!$omp parallel do private(&\n",
        "!$omp& x, &\n",
        "!$omp& y)"
    ));

    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "private");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("x, y".into())
    );
}

#[test]
fn free_form_allows_empty_continuation_lines() {
    let directive = parse_free(concat!(
        "!$omp parallel do &\n",
        "!$omp& &\n",
        "!$omp& private(i)"
    ));

    assert_eq!(directive.name, "parallel do");
    assert_eq!(directive.clauses.len(), 1);
}

#[test]
fn free_form_skips_comment_only_continuation_lines() {
    let directive = parse_free(concat!(
        "!$omp parallel do &\n",
        "!$omp& & ! comment about variables\n",
        "!$omp& private(i, j)"
    ));

    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "private");
}

#[test]
fn free_form_respects_maximum_continuation_depth() {
    let directive = parse_free(concat!(
        "!$omp parallel do &\n",
        "!$omp& private(i, &\n",
        "!$omp& j, &\n",
        "!$omp& k, &\n",
        "!$omp& l)"
    ));

    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "private");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("i, j, k, l".into())
    );
}

#[test]
fn free_form_errors_on_inline_ampersand_without_newline() {
    let parser = Parser::default().with_language(Language::FortranFree);
    match parser.parse("!$omp parallel do & private(i)") {
        Ok((rest, directive)) => {
            assert!(
                rest.starts_with("&"),
                "expected inline ampersand to remain, got {rest:?}"
            );
            assert!(directive.clauses.is_empty());
        }
        Err(err) => {
            panic!("expected parser to leave inline ampersand unparsed, got error: {err:?}")
        }
    }
}
