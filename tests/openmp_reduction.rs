use roup::parser::{
    clause::{ReductionModifier, ReductionOperator},
    ClauseKind, Parser,
};
use std::borrow::Cow;

type ReductionCase = (
    Vec<ReductionModifier>,
    ReductionOperator,
    Option<Cow<'static, str>>,
    Vec<Cow<'static, str>>,
);

fn parser() -> Parser {
    Parser::default()
}

#[test]
fn parses_reduction_clause_with_modifiers_and_operators() {
    let parser = parser();
    let source = "#pragma omp parallel for reduction(task,inscan,+:total) reduction(^:checksum) reduction(&&:all_true)";

    let (rest, directive) = parser
        .parse(source)
        .expect("parallel for reduction clauses should parse");

    assert!(rest.trim().is_empty());
    assert_eq!(directive.name, "parallel for");
    assert_eq!(directive.clauses.len(), 3);

    let expected: [ReductionCase; 3] = [
        (
            vec![ReductionModifier::Task, ReductionModifier::Inscan],
            ReductionOperator::Add,
            None,
            vec![Cow::from("total")],
        ),
        (
            Vec::new(),
            ReductionOperator::BitXor,
            None,
            vec![Cow::from("checksum")],
        ),
        (
            Vec::new(),
            ReductionOperator::LogAnd,
            None,
            vec![Cow::from("all_true")],
        ),
    ];

    for (clause, (modifiers, operator, user_id, vars)) in
        directive.clauses.iter().zip(expected.into_iter())
    {
        assert_eq!(clause.name, "reduction");
        match &clause.kind {
            ClauseKind::ReductionClause {
                modifiers: parsed_mods,
                operator: parsed_op,
                user_defined_identifier,
                variables,
                ..
            } => {
                assert_eq!(parsed_mods, &modifiers);
                assert_eq!(*parsed_op, operator);
                let parsed_user = user_defined_identifier.as_ref().map(|id| id.to_string());
                let expected_user = user_id.as_ref().map(|id| id.to_string());
                assert_eq!(parsed_user, expected_user);
                assert_eq!(variables, &vars);
            }
            other => panic!("unexpected clause kind for reduction: {other:?}"),
        }
    }
}

#[test]
fn parses_reduction_clause_with_user_defined_identifier() {
    let parser = parser();
    let source = "#pragma omp parallel reduction(user_addition:accumulator) reduction(task, custom_reducer:list)";

    let (_, directive) = parser
        .parse(source)
        .expect("reduction clauses with user identifiers should parse");

    assert_eq!(directive.name, "parallel");
    assert_eq!(directive.clauses.len(), 2);

    assert_eq!(directive.clauses[0].name, "reduction");
    match &directive.clauses[0].kind {
        ClauseKind::ReductionClause {
            modifiers,
            operator,
            user_defined_identifier,
            variables,
            ..
        } => {
            assert!(modifiers.is_empty());
            assert_eq!(*operator, ReductionOperator::UserDefined);
            assert_eq!(
                user_defined_identifier.as_ref().map(|id| id.as_ref()),
                Some("user_addition")
            );
            assert_eq!(variables, &vec![Cow::from("accumulator")]);
        }
        other => panic!("unexpected clause kind for reduction: {other:?}"),
    }

    assert_eq!(directive.clauses[1].name, "reduction");
    match &directive.clauses[1].kind {
        ClauseKind::ReductionClause {
            modifiers,
            operator,
            user_defined_identifier,
            variables,
            ..
        } => {
            assert_eq!(modifiers, &vec![ReductionModifier::Task]);
            assert_eq!(*operator, ReductionOperator::UserDefined);
            assert_eq!(
                user_defined_identifier.as_ref().map(|id| id.as_ref()),
                Some("custom_reducer")
            );
            assert_eq!(variables, &vec![Cow::from("list")]);
        }
        other => panic!("unexpected clause kind for reduction: {other:?}"),
    }
}
