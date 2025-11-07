use roup::parser::{ClauseKind, Parser};

fn parser() -> Parser {
    Parser::default()
}

#[test]
fn parses_metadirective_with_all_context_selectors() {
    let parser = parser();
    let source = "#pragma omp metadirective \
        when(device={kind(cpu), isa(avx512f), arch(gen)}, implementation={vendor(llvm), extension(match_extension), atomic_default_mem_order(seq_cst)}, user={condition(iterations>0)}, construct={parallel, for, simd}: parallel for schedule(static,1)) \
        when(device={kind(gpu)}, construct={teams, distribute}, implementation={extension(distributed)}, user={condition(flag != 0)}: teams distribute parallel for) \
        default(parallel for reduction(task,inscan,+:sum))";

    let (rest, directive) = parser
        .parse(source)
        .expect("metadirective with context selectors should parse");

    assert!(rest.trim().is_empty(), "expected full consumption of input");
    assert_eq!(directive.name, "metadirective");
    assert_eq!(directive.clauses.len(), 3);

    let first_when = &directive.clauses[0];
    assert_eq!(first_when.name, "when");
    assert!(matches!(first_when.kind, ClauseKind::Parenthesized(_)));
    if let ClauseKind::Parenthesized(body) = &first_when.kind {
        assert_eq!(
            body.as_ref(),
            "device={kind(cpu), isa(avx512f), arch(gen)}, implementation={vendor(llvm), extension(match_extension), atomic_default_mem_order(seq_cst)}, user={condition(iterations>0)}, construct={parallel, for, simd}: parallel for schedule(static,1)"
        );
    }

    let second_when = &directive.clauses[1];
    assert_eq!(second_when.name, "when");
    if let ClauseKind::Parenthesized(body) = &second_when.kind {
        assert_eq!(
            body.as_ref(),
            "device={kind(gpu)}, construct={teams, distribute}, implementation={extension(distributed)}, user={condition(flag != 0)}: teams distribute parallel for"
        );
    } else {
        panic!("expected parenthesized clause for second when");
    }

    let default_clause = &directive.clauses[2];
    assert_eq!(default_clause.name, "default");
    if let ClauseKind::Parenthesized(body) = &default_clause.kind {
        assert_eq!(body.as_ref(), "parallel for reduction(task,inscan,+:sum)");
    } else {
        panic!("expected parenthesized clause for default");
    }
}

#[test]
fn parses_metadirective_with_nested_directives_and_qualifiers() {
    let parser = parser();
    let source = "#pragma omp metadirective \
        when(construct={target, teams, distribute, parallel for}, device={kind(nohost), device_num(1)}, implementation={vendor(amd), extension(quirk_mode)}, user={condition((iterations & 1) == 0)}: target teams distribute parallel for collapse(2) reduction(default, &:acc) nowait) \
        when(implementation={atomic_default_mem_order(seq_cst)}, device={isa(avx2), arch(x86_64)}, user={condition(flag)}, construct={parallel, simd}: parallel for simd schedule(dynamic,4) reduction(^:checksum)) \
        default(nothing)";

    let (rest, directive) = parser
        .parse(source)
        .expect("metadirective with nested directives should parse");

    assert!(rest.trim().is_empty());
    assert_eq!(directive.name, "metadirective");
    assert_eq!(directive.clauses.len(), 3);

    for clause in &directive.clauses {
        assert!(
            matches!(clause.kind, ClauseKind::Parenthesized(_)),
            "all clauses should be parenthesized"
        );
    }

    let bodies: Vec<&str> = directive
        .clauses
        .iter()
        .map(|clause| match &clause.kind {
            ClauseKind::Parenthesized(body) => body.as_ref(),
            ClauseKind::Bare => panic!("expected parenthesized clause"),
            _ => panic!("unexpected clause kind"),
        })
        .collect();

    assert_eq!(
        bodies[0],
        "construct={target, teams, distribute, parallel for}, device={kind(nohost), device_num(1)}, implementation={vendor(amd), extension(quirk_mode)}, user={condition((iterations & 1) == 0)}: target teams distribute parallel for collapse(2) reduction(default, &:acc) nowait"
    );
    assert_eq!(
        bodies[1],
        "implementation={atomic_default_mem_order(seq_cst)}, device={isa(avx2), arch(x86_64)}, user={condition(flag)}, construct={parallel, simd}: parallel for simd schedule(dynamic,4) reduction(^:checksum)"
    );
    assert_eq!(bodies[2], "nothing");
}
