use roup::ir::{
    convert::convert_directive, ClauseData, ClauseItem, Language as IrLanguage, ParserConfig,
    SourceLocation,
};
use roup::lexer::Language as ParserLanguage;
use roup::parser::Parser;

#[test]
fn c_map_clause_tracks_array_sections() {
    let input = "#pragma omp target map(to: arr[0:N:2])";
    let parser = Parser::default();
    let (_, directive) = parser.parse(input).expect("parse C directive");

    let config = ParserConfig::with_parsing(IrLanguage::C);
    let ir = convert_directive(&directive, SourceLocation::start(), IrLanguage::C, &config)
        .expect("convert to IR");

    let map_clause = ir
        .clauses()
        .iter()
        .find(|clause| matches!(clause, ClauseData::Map { .. }))
        .expect("map clause present");

    if let ClauseData::Map { items, .. } = map_clause {
        match &items[0] {
            ClauseItem::Variable(var) => {
                assert_eq!(var.name(), "arr");
                assert_eq!(var.array_sections.len(), 1);
            }
            other => panic!("expected variable clause item, got {:?}", other),
        }
    }
}

#[test]
fn fortran_map_clause_uses_parentheses_sections() {
    let input = "!$omp target map(to: array(1:n))";
    let parser = Parser::default().with_language(ParserLanguage::FortranFree);
    let (_, directive) = parser.parse(input).expect("parse Fortran directive");

    let config = ParserConfig::with_parsing(IrLanguage::Fortran);
    let ir = convert_directive(
        &directive,
        SourceLocation::start(),
        IrLanguage::Fortran,
        &config,
    )
    .expect("convert Fortran directive");

    let map_clause = ir
        .clauses()
        .iter()
        .find(|clause| matches!(clause, ClauseData::Map { .. }))
        .expect("map clause present");

    if let ClauseData::Map { items, .. } = map_clause {
        match &items[0] {
            ClauseItem::Variable(var) => {
                assert_eq!(var.name(), "array");
                assert_eq!(var.array_sections.len(), 1);
            }
            other => panic!("expected Fortran variable, got {:?}", other),
        }
    }
}

#[test]
fn language_support_can_be_disabled() {
    let input = "#pragma omp target map(to: arr[0:N])";
    let parser = Parser::default();
    let (_, directive) = parser.parse(input).expect("parse directive");

    let config = ParserConfig::with_parsing(IrLanguage::C).with_language_support(false);
    let ir = convert_directive(&directive, SourceLocation::start(), IrLanguage::C, &config)
        .expect("convert with disabled language support");

    let map_clause = ir
        .clauses()
        .iter()
        .find(|clause| matches!(clause, ClauseData::Map { .. }))
        .expect("map clause present");

    if let ClauseData::Map { items, .. } = map_clause {
        assert!(matches!(items[0], ClauseItem::Identifier(_)));
    }
}
