use roup::api::{OpenMpConfig, OpenMpParser};
use roup::ast::OmpReductionIdentifier;
use roup::ir::{ClauseData, ClauseItem, ReductionModifier};
use roup::version::{CStandard, HostLanguageProfile, SourceForm};

fn parser() -> OpenMpParser {
    OpenMpConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .unwrap()
        .parser()
}

fn item_name(item: &ClauseItem) -> &str {
    match item {
        ClauseItem::Identifier(identifier) => identifier.as_str(),
        ClauseItem::Variable(variable) => variable.expression().source(),
        ClauseItem::FortranCommonBlock(name) => name.as_str(),
        ClauseItem::Expression(expression) => expression.source(),
        ClauseItem::OmpparserTrailingSlash(identifier) => identifier.as_str(),
    }
}

#[test]
fn reduction_modifiers_operators_and_items_are_typed() {
    let parsed = parser()
        .parse(
            "#pragma omp parallel for reduction(task,inscan,+:total) reduction(^:checksum) reduction(&&:all_true)",
        )
        .expect("valid reductions");
    let clauses = parsed.directive().clauses();

    let expected = [
        (
            vec![ReductionModifier::Task, ReductionModifier::Inscan],
            OmpReductionIdentifier::Add,
            "total",
        ),
        (vec![], OmpReductionIdentifier::BitwiseXor, "checksum"),
        (vec![], OmpReductionIdentifier::LogicalAnd, "all_true"),
    ];

    for (clause, (expected_modifiers, expected_operator, expected_item)) in
        clauses.iter().zip(expected)
    {
        let ClauseData::Reduction {
            modifiers,
            operator,
            items,
            ..
        } = clause.payload()
        else {
            panic!("reduction clause must have a reduction payload");
        };
        assert_eq!(modifiers, &expected_modifiers);
        assert_eq!(operator, &expected_operator);
        assert_eq!(
            items.iter().map(item_name).collect::<Vec<_>>(),
            [expected_item]
        );
    }
}

#[test]
fn user_defined_reduction_identifiers_are_validated_identifiers() {
    let parsed = parser()
        .parse(
            "#pragma omp parallel reduction(user_addition:accumulator) reduction(task,custom_reducer:list)",
        )
        .expect("valid user-defined reductions");
    let clauses = parsed.directive().clauses();

    for (index, (expected_operator, expected_item)) in
        [("user_addition", "accumulator"), ("custom_reducer", "list")]
            .into_iter()
            .enumerate()
    {
        let ClauseData::Reduction {
            operator, items, ..
        } = clauses[index].payload()
        else {
            panic!("expected reduction payload");
        };
        assert!(matches!(
            operator,
            OmpReductionIdentifier::Name(identifier)
                if identifier.qualified_name().is_some_and(|name|
                    !name.global
                        && name.segments.len() == 1
                        && name.segments[0].as_str() == expected_operator)
        ));
        assert_eq!(
            items.iter().map(item_name).collect::<Vec<_>>(),
            [expected_item]
        );
    }
}

#[test]
fn malformed_reduction_grammar_is_never_carried_as_raw_text() {
    for source in [
        "#pragma omp parallel reduction(+:)",
        "#pragma omp parallel reduction(bogus,+:x)",
        "#pragma omp parallel reduction(+ x)",
        "#pragma omp parallel reduction(user-name:x)",
    ] {
        assert!(parser().parse(source).is_err(), "{source} must be rejected");
    }
}
