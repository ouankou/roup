use roup::api::{OpenMpConfig, OpenMpParser, ParsedOpenMpDirective};
use roup::ast::{
    OmpInitializerValue, OmpReductionInitializer, OmpSelectorDeviceTrait, OmpSelectorEntry,
    OmpSelectorNameListKind, OmpSelectorTraitValue,
};
use roup::ir::{ClauseData, ClauseItem};
use roup::version::{CStandard, FortranStandard, HostLanguageProfile, SourceForm};

fn parser() -> OpenMpParser {
    OpenMpConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .expect("valid C parser configuration")
        .parser()
}

fn parse(source: &str) -> ParsedOpenMpDirective {
    parser()
        .parse(source)
        .unwrap_or_else(|error| panic!("failed to parse {source:?}: {error}"))
}

#[test]
fn commas_and_delimiters_inside_comments_do_not_split_variable_lists() {
    parse("#pragma omp parallel private/* clause trivia */(a)");
    let private = parse("#pragma omp parallel private(a /*, ) ] } : ?*/, b)");
    let ClauseData::Private { items } = private.directive().clauses()[0].payload() else {
        panic!("expected a private payload");
    };
    assert!(matches!(
        items.as_slice(),
        [ClauseItem::Identifier(a), ClauseItem::Identifier(b)]
            if a.as_str() == "a" && b.as_str() == "b"
    ));

    let mapped = parse("#pragma omp target map(to: a /*, ) ] } : ?*/, b)");
    let ClauseData::Map { locators, .. } = mapped.directive().clauses()[0].payload() else {
        panic!("expected a map payload");
    };
    assert_eq!(locators.len(), 2);
}

#[test]
fn directive_keywords_and_selector_separators_treat_comments_as_trivia() {
    parse("#pragma omp target/* keyword trivia */enter data map(to: value)");
    parse(
        "#pragma omp metadirective when(device /* = not a separator */ = {kind(cpu)} /* : not a separator */ : parallel)",
    );

    assert!(
        parser()
            .parse("#pragma omp target,enter data map(to: value)")
            .is_err(),
        "punctuation between directive keywords must not be repaired"
    );
}

#[test]
fn selector_delimiters_inside_string_literals_are_data() {
    for (source, expected) in [
        (
            "#pragma omp metadirective when(device={kind(\"c}pu\")}: parallel)",
            "c}pu",
        ),
        (
            "#pragma omp metadirective when(device={kind(\"c)pu\")}: parallel)",
            "c)pu",
        ),
    ] {
        let parsed = parse(source);
        let ClauseData::MetadirectiveSelector { selector } =
            parsed.directive().clauses()[0].payload()
        else {
            panic!("expected a typed selector");
        };
        let OmpSelectorEntry::Device { traits, .. } = &selector.entries()[0] else {
            panic!("expected a device selector");
        };
        assert!(matches!(
            &traits[0],
            OmpSelectorDeviceTrait::NameList(value)
                if value.kind() == OmpSelectorNameListKind::Kind
                    && matches!(value.properties(), [OmpSelectorTraitValue::StringLiteral(literal)]
                    if literal.value == expected)
        ));
    }
}

#[test]
fn braced_initializers_ignore_braces_inside_string_literals() {
    let parsed = parse(
        "#pragma omp declare reduction(text : item_t : omp_out = omp_in) initializer(omp_priv = {\"}\"})",
    );
    let Some(roup::ast::OmpDirectiveParameter::DeclareReduction(reduction)) =
        parsed.directive().parameter()
    else {
        panic!("expected a declare-reduction parameter");
    };
    assert!(matches!(
        reduction.initializer(),
        Some(OmpReductionInitializer::CAssignment(OmpInitializerValue::Braced(
            initializer
        ))) if initializer.elements().len() == 1
    ));
}

#[test]
fn uses_allocators_traits_reject_non_variable_expressions() {
    assert!(parser()
        .parse("#pragma omp target uses_allocators(traits(f(\")\")): my_allocator)")
        .is_err());
}

#[test]
fn iterator_ranges_use_top_level_colons_only() {
    let parsed = parse(
        "#pragma omp target map(iterator(int i=flag ? 0 : 1 /* : , ) */:n:f(\":\", 1)), to: a)",
    );
    let ClauseData::Map { iterators, .. } = parsed.directive().clauses()[0].payload() else {
        panic!("expected map payload");
    };
    assert_eq!(iterators.len(), 1);
    assert_eq!(iterators[0].start().to_string(), "flag ? 0 : 1");
    assert_eq!(iterators[0].end().to_string(), "n");
    assert_eq!(iterators[0].step().unwrap().to_string(), "f(\":\", 1)");
}

#[test]
fn malformed_quotes_comments_and_delimiters_are_hard_errors() {
    for source in [
        "#pragma omp parallel private(a /* unterminated, b)",
        "#pragma omp parallel private(a // closing parenthesis is commented out)",
        "#pragma omp parallel private(\"unterminated, b)",
        "#pragma omp parallel private(a], b)",
        "#pragma omp target map(to: a /* unterminated, b)",
        "#pragma omp metadirective when(device={kind(\"c}pu)}: parallel)",
        "#pragma omp metadirective when(device={kind(\"c)pu)}: parallel)",
        "#pragma omp declare reduction(text : item_t : omp_out = omp_in) initializer(omp_priv = {\"}\")",
        "#pragma omp target uses_allocators(traits(f(\")\")): my_allocator",
        "#pragma omp target map(iterator(int i=0:n:f(\":\", 1), to: a)",
    ] {
        assert!(parser().parse(source).is_err(), "accepted {source:?}");
    }
}

#[test]
fn fortran_doubled_quotes_shield_delimiters() {
    let parser = OpenMpConfig::new(
        HostLanguageProfile::Fortran(FortranStandard::Fortran2023),
        SourceForm::FortranFree,
    )
    .expect("valid Fortran parser configuration")
    .parser();
    parser
        .parse("!$omp parallel if('a'')' == 'a'')')")
        .expect("a delimiter in a Fortran doubled-quote string is not syntax");

    parser
        .parse("!$omp parallel ! trailing Fortran comment")
        .expect("Fortran exclamation comments are trailing trivia");
    assert!(
        parser.parse("!$omp parallel // not a comment").is_err(),
        "Fortran concatenation syntax must not be erased as a C++ comment"
    );
    assert!(
        parser
            .parse("!$omp parallel private/* not Fortran trivia */(a)")
            .is_err(),
        "C block comments must not leak into Fortran directive syntax"
    );
}
