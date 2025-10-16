use roup::parser::{
    openmp::{OpenMpClause, OpenMpDirective},
    ClauseKind, ClauseRule, Parser,
};

fn parser() -> Parser {
    Parser::default()
}

#[derive(Clone, Copy)]
enum ExpectedClauseKind {
    Bare,
    Parenthesized,
}

#[test]
fn parses_all_registered_directives() {
    let parser = parser();

    for directive in OpenMpDirective::ALL {
        let source = format!("#pragma omp {}", directive.as_str());
        let result = parser.parse(&source);
        assert!(
            result.is_ok(),
            "directive `{}` should be accepted by the parser",
            directive.as_str()
        );

        let (_, parsed) = result.unwrap();
        assert_eq!(parsed.name, directive.as_str());
    }
}

#[test]
fn parses_all_registered_clauses() {
    let parser = parser();

    for clause in OpenMpClause::ALL {
        let variants = match clause.rule() {
            ClauseRule::Bare => vec![(clause.name().to_string(), ExpectedClauseKind::Bare)],
            ClauseRule::Parenthesized => vec![(
                format!("{}(value)", clause.name()),
                ExpectedClauseKind::Parenthesized,
            )],
            ClauseRule::Flexible => vec![
                (clause.name().to_string(), ExpectedClauseKind::Bare),
                (
                    format!("{}(value)", clause.name()),
                    ExpectedClauseKind::Parenthesized,
                ),
            ],
            ClauseRule::Custom(_) => vec![(
                format!("{}(value)", clause.name()),
                ExpectedClauseKind::Parenthesized,
            )],
            ClauseRule::Unsupported => {
                panic!("clause `{}` should not report Unsupported", clause.name())
            }
        };

        for (clause_text, expected_kind) in variants {
            let source = format!("#pragma omp parallel {clause_text}");
            let result = parser.parse(&source);
            assert!(
                result.is_ok(),
                "clause `{}` should be accepted by the parser",
                clause.name()
            );

            let (_, directive) = result.unwrap();
            assert_eq!(
                directive.clauses.len(),
                1,
                "expected exactly one clause for `{}`",
                clause.name()
            );
            assert_eq!(directive.clauses[0].name, clause.name());

            match (expected_kind, &directive.clauses[0].kind) {
                (ExpectedClauseKind::Bare, ClauseKind::Bare) => {}
                (ExpectedClauseKind::Parenthesized, ClauseKind::Parenthesized(_)) => {}
                _ => panic!(
                    "clause `{}` did not parse with the expected kind",
                    clause.name()
                ),
            }
        }
    }
}
