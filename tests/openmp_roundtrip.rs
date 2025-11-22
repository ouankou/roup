use roup::parser::{openmp, ClauseRule, Parser};

fn parse(input: &str) -> roup::parser::Directive<'_> {
    let parser = Parser::default();
    let (_, directive) = parser.parse(input).expect("directive should parse");
    directive
}

#[test]
fn roundtrips_all_openmp_directives_without_clauses() {
    for directive in openmp::OpenMpDirective::ALL {
        let source = format!("#pragma omp {}", directive.as_str());
        let parsed = parse(&source);
        assert_eq!(
            parsed.to_pragma_string(),
            source,
            "directive: {}",
            directive.as_str()
        );
    }
}

fn sample_clause(clause: openmp::OpenMpClause) -> Option<String> {
    match clause {
        openmp::OpenMpClause::Reduction
        | openmp::OpenMpClause::InReduction
        | openmp::OpenMpClause::TaskReduction => {
            return Some(format!("{}(+:{})", clause.name(), "value"));
        }
        _ => {}
    }
    match clause.rule() {
        ClauseRule::Bare => Some(clause.name().to_string()),
        ClauseRule::Parenthesized | ClauseRule::Flexible => {
            Some(format!("{}(value)", clause.name()))
        }
        ClauseRule::Custom(_) => Some(format!("{}(value)", clause.name())),
        ClauseRule::Unsupported => None,
    }
}

#[test]
fn roundtrips_all_openmp_clauses() {
    for clause in openmp::OpenMpClause::ALL {
        let Some(clause_source) = sample_clause(*clause) else {
            continue;
        };

        let source = format!("#pragma omp parallel {}", clause_source);
        let parsed = parse(&source);

        assert_eq!(
            parsed.to_pragma_string(),
            source,
            "clause: {}",
            clause.name()
        );
    }
}

#[test]
fn parses_new_openmp60_entries() {
    let split = parse("#pragma omp split counts(4)");
    assert_eq!(split.name, "split");
    assert_eq!(split.clauses[0].name, "counts");

    let stripe = parse("#pragma omp stripe");
    assert_eq!(stripe.name, "stripe");
    assert!(stripe.clauses.is_empty());

    let begin_meta = parse("#pragma omp begin metadirective");
    assert_eq!(begin_meta.name, "begin metadirective");
    assert!(begin_meta.clauses.is_empty());

    let has_device_addr = parse("#pragma omp parallel has_device_addr(ptr)");
    assert_eq!(has_device_addr.clauses[0].name, "has_device_addr");
}
