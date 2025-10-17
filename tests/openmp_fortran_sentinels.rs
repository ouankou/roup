use roup::lexer::Language;
use roup::parser::{ClauseKind, Directive, Parser};

fn parse_with_language(input: &str, language: Language) -> Directive<'_> {
    let parser = Parser::default().with_language(language);
    let (rest, directive) = parser.parse(input).expect("directive should parse");
    assert!(
        rest.trim().is_empty(),
        "parser left unconsumed input: {rest:?}"
    );
    directive
}

fn parse_fixed(input: &str) -> Directive<'_> {
    parse_with_language(input, Language::FortranFixed)
}

fn parse_free(input: &str) -> Directive<'_> {
    parse_with_language(input, Language::FortranFree)
}

#[test]
fn fortran_fixed_supports_short_form_sentinel() {
    let directive = parse_fixed(concat!(
        "      !$OMP PARALLEL DO &\n",
        "      !$& PRIVATE(I, J)",
    ));

    assert_eq!(directive.name, "parallel do");
    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "private");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("I, J".into()),
    );
}

#[test]
fn fortran_fixed_accepts_alternate_comment_characters() {
    let directive = parse_fixed(concat!(
        "      !$OMP TARGET TEAMS &\n",
        "      C$OMP& DISTRIBUTE &\n",
        "      *$OMP& PARALLEL DO PRIVATE(I)",
    ));

    assert_eq!(directive.name, "target teams distribute parallel do");
    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "private");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("I".into()),
    );
}

#[test]
fn fortran_fixed_handles_mixed_case_and_short_sentinels() {
    let cases = [
        concat!("      !$OMP PARALLEL DO &\n", "      !$OMP& PRIVATE(I)"),
        concat!("      !$OMP PARALLEL DO &\n", "      !$Omp& PRIVATE(I)"),
        concat!("      !$OMP PARALLEL DO &\n", "      !$& PRIVATE(I)"),
        concat!("      !$OMP PARALLEL DO &\n", "      C$OMP& PRIVATE(I)"),
        concat!("      !$OMP PARALLEL DO &\n", "      *$OMP& PRIVATE(I)"),
    ];

    for case in cases {
        let directive = parse_fixed(case);
        assert_eq!(directive.name, "parallel do");
        assert_eq!(directive.clauses.len(), 1);
        assert_eq!(directive.clauses[0].name, "private");
        assert_eq!(
            directive.clauses[0].kind,
            ClauseKind::Parenthesized("I".into()),
        );
    }
}

#[test]
fn fortran_fixed_supports_deep_continuations_with_comments() {
    let directive = parse_fixed(concat!(
        "      !$OMP TARGET &\n",
        "      !$OMP& TEAMS & ! reorganize teams\n",
        "      !$& DISTRIBUTE &\n",
        "      !$OMP& PARALLEL DO PRIVATE(I, J) &\n",
        "      !$OMP& SHARED(A)",
    ));

    assert_eq!(directive.name, "target teams distribute parallel do");
    assert_eq!(directive.clauses.len(), 2);
    assert_eq!(directive.clauses[0].name, "private");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("I, J".into()),
    );
    assert_eq!(directive.clauses[1].name, "shared");
    assert_eq!(
        directive.clauses[1].kind,
        ClauseKind::Parenthesized("A".into()),
    );
}

#[test]
fn fortran_fixed_rejects_missing_sentinel_on_continuation() {
    let parser = Parser::default().with_language(Language::FortranFixed);
    let err = parser.parse(concat!("      !$OMP PARALLEL DO &\n", "      X PRIVATE(I)"));

    assert!(err.is_err());
}

#[test]
fn fortran_free_allows_ampersand_at_line_end_only() {
    let directive = parse_free(concat!("!$omp parallel do &\n", "!$omp private(i, j)",));

    assert_eq!(directive.name, "parallel do");
    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "private");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("i, j".into()),
    );
}

#[test]
fn fortran_free_allows_ampersand_at_line_start_only() {
    let directive = parse_free(concat!("!$omp parallel do\n", "& private(i, j)",));

    assert_eq!(directive.name, "parallel do");
    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "private");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("i, j".into()),
    );
}

#[test]
fn fortran_free_supports_ampersand_on_both_lines() {
    let directive = parse_free(concat!("!$omp parallel do &\n", "& private(i, j)",));

    assert_eq!(directive.name, "parallel do");
    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "private");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("i, j".into()),
    );
}

#[test]
fn fortran_free_supports_multiple_consecutive_continuations() {
    let directive = parse_free(concat!(
        "!$omp target teams distribute parallel do &\n",
        "!$omp& schedule(static, 4) &\n",
        "!$omp& private(i, j) &\n",
        "!$omp& shared(a, b)",
    ));

    assert_eq!(directive.name, "target teams distribute parallel do",);
    assert_eq!(directive.clauses.len(), 3);
    assert_eq!(directive.clauses[0].name, "schedule");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("static, 4".into()),
    );
    assert_eq!(directive.clauses[1].name, "private");
    assert_eq!(
        directive.clauses[1].kind,
        ClauseKind::Parenthesized("i, j".into()),
    );
    assert_eq!(directive.clauses[2].name, "shared");
    assert_eq!(
        directive.clauses[2].kind,
        ClauseKind::Parenthesized("a, b".into()),
    );
}

#[test]
fn fortran_free_preserves_token_boundaries_in_directive_name() {
    let directive = parse_free(concat!("!$omp parallel&\n", "!$omp& do private(i)",));

    assert_eq!(directive.name, "parallel do");
    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "private");
}

#[test]
fn fortran_free_allows_clause_continuation_inside_parentheses() {
    let directive = parse_free(concat!(
        "!$omp parallel do private(i, &\n",
        "& j, &\n",
        "& k)",
    ));

    assert_eq!(directive.name, "parallel do");
    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "private");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("i, j, k".into()),
    );
}

#[test]
fn fortran_free_handles_empty_continuation_lines() {
    let directive = parse_free(concat!("!$omp parallel do private(i, &\n", "&\n", "& j)",));

    assert_eq!(directive.name, "parallel do");
    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "private");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("i, j".into()),
    );
}

#[test]
fn fortran_free_handles_comments_between_continuations() {
    let directive = parse_free(concat!(
        "!$omp parallel do private(i, & ! list part one\n",
        "& j, &\n",
        "& ! comment only line\n",
        "& k)",
    ));

    assert_eq!(directive.name, "parallel do");
    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "private");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("i, j, k".into()),
    );
}

#[test]
fn fortran_free_reports_error_when_missing_continuation_marker() {
    let parser = Parser::default().with_language(Language::FortranFree);
    match parser.parse("!$omp parallel do private(i, &") {
        Ok((rest, _)) => {
            assert!(
                !rest.trim().is_empty(),
                "parser unexpectedly consumed the entire directive"
            );
        }
        Err(_) => {}
    }
}
