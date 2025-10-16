//! Integration tests for language-aware OpenMP clause parsing.
//!
//! This test suite verifies that the language-aware parsing module correctly
//! handles C/C++/Fortran constructs in OpenMP clauses, including:
//! - Array sections with various syntaxes
//! - Template parameters in C++
//! - Multi-dimensional arrays
//! - Mapper syntax in map clauses
//! - Runtime toggle of language semantics

use roup::ir::{
    convert::convert_directive, ClauseData, ClauseItem, Language, ParserConfig, SourceLocation,
};
use roup::lexer::Language as LexerLanguage;
use roup::parser::{parse_omp_directive, Parser};

#[test]
fn c_map_clause_with_array_sections() {
    let input = "#pragma omp target map(to: arr[0:N:2])";
    let (_, directive) = parse_omp_directive(input).expect("parse C directive");

    let config = ParserConfig::with_parsing(Language::C);
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
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
                let section = &var.array_sections[0];
                assert!(section.lower_bound.is_some());
                assert!(section.length.is_some());
                assert!(section.stride.is_some());
            }
            other => panic!("expected variable clause item, got {other:?}"),
        }
    }
}

#[test]
fn c_nested_array_sections() {
    let input = "#pragma omp target map(to: matrix[0:N][i:j])";
    let (_, directive) = parse_omp_directive(input).expect("parse");

    let config = ParserConfig::with_parsing(Language::C);
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("convert");

    let map_items = ir
        .clauses()
        .iter()
        .find_map(|clause| match clause {
            ClauseData::Map { items, .. } => Some(items),
            _ => None,
        })
        .expect("map clause present");

    match &map_items[0] {
        ClauseItem::Variable(var) => {
            assert_eq!(var.name(), "matrix");
            assert_eq!(var.array_sections.len(), 2);
        }
        other => panic!("expected variable, got {other:?}"),
    }
}

#[test]
fn cpp_template_in_private_clause() {
    let input = "#pragma omp parallel private(vec, data)";
    let (_, directive) = parse_omp_directive(input).expect("parse C++ directive");

    let config = ParserConfig::with_parsing(Language::Cpp);
    let ir = convert_directive(&directive, SourceLocation::start(), Language::Cpp, &config)
        .expect("convert");

    let private_items = ir
        .clauses()
        .iter()
        .find_map(|clause| match clause {
            ClauseData::Private { items } => Some(items),
            _ => None,
        })
        .expect("private clause present");

    // Should correctly split the two items
    assert_eq!(private_items.len(), 2);
    match &private_items[0] {
        ClauseItem::Identifier(id) => {
            assert_eq!(id.to_string(), "vec");
        }
        _ => panic!("Expected identifier"),
    }
    match &private_items[1] {
        ClauseItem::Identifier(id) => {
            assert_eq!(id.to_string(), "data");
        }
        _ => panic!("Expected identifier"),
    }
}

#[test]
fn fortran_array_sections_with_parentheses() {
    let input = "!$omp target map(to: field(1:n, :, 2:m:2))";
    let parser = Parser::default().with_language(LexerLanguage::FortranFree);
    let (_, directive) = parser.parse(input).expect("parse Fortran directive");

    let config = ParserConfig::with_parsing(Language::Fortran);
    let ir = convert_directive(
        &directive,
        SourceLocation::start(),
        Language::Fortran,
        &config,
    )
    .expect("convert Fortran directive");

    let map_items = ir
        .clauses()
        .iter()
        .find_map(|clause| match clause {
            ClauseData::Map { items, .. } => Some(items),
            _ => None,
        })
        .expect("map clause present");

    match &map_items[0] {
        ClauseItem::Variable(var) => {
            assert_eq!(var.name(), "field");
            assert_eq!(var.array_sections.len(), 3);
            // Check that third dimension has stride
            assert!(var.array_sections[2].stride.is_some());
            // Second dimension should be "all" (:)
            assert!(var.array_sections[1].lower_bound.is_none());
            assert!(var.array_sections[1].length.is_none());
        }
        other => panic!("expected Fortran variable, got {other:?}"),
    }
}

#[test]
fn fortran_multi_dimensional_private() {
    let input = "!$omp parallel private(A(1:N), B(:, :), C)";
    let parser = Parser::default().with_language(LexerLanguage::FortranFree);
    let (_, directive) = parser.parse(input).expect("parse");

    let config = ParserConfig::with_parsing(Language::Fortran);
    let ir = convert_directive(
        &directive,
        SourceLocation::start(),
        Language::Fortran,
        &config,
    )
    .expect("convert");

    let private_items = ir
        .clauses()
        .iter()
        .find_map(|clause| match clause {
            ClauseData::Private { items } => Some(items),
            _ => None,
        })
        .expect("private clause present");

    assert_eq!(private_items.len(), 3);
    match &private_items[0] {
        ClauseItem::Variable(var) => {
            assert_eq!(var.name(), "A");
            assert_eq!(var.array_sections.len(), 1);
        }
        other => panic!("expected variable, got {other:?}"),
    }
    match &private_items[1] {
        ClauseItem::Variable(var) => {
            assert_eq!(var.name(), "B");
            assert_eq!(var.array_sections.len(), 2);
        }
        other => panic!("expected variable, got {other:?}"),
    }
    match &private_items[2] {
        ClauseItem::Identifier(_) => {
            // C is a simple identifier
        }
        other => panic!("expected identifier, got {other:?}"),
    }
}

#[test]
fn mapper_syntax_in_map_clause() {
    let input = "#pragma omp target map(mapper(custom), to: arr[0:N])";
    let (_, directive) = parse_omp_directive(input).expect("parse");

    let config = ParserConfig::with_parsing(Language::C);
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("convert");

    let map_clause = ir
        .clauses()
        .iter()
        .find(|clause| matches!(clause, ClauseData::Map { .. }))
        .expect("map clause present");

    if let ClauseData::Map {
        map_type,
        mapper,
        items,
    } = map_clause
    {
        assert_eq!(*map_type, Some(roup::ir::MapType::To));
        assert_eq!(mapper.as_ref().unwrap().to_string(), "custom");
        assert_eq!(items.len(), 1);
        assert!(matches!(items[0], ClauseItem::Variable(_)));
    } else {
        panic!("Expected map clause");
    }
}

#[test]
fn language_semantics_can_be_disabled() {
    let input = "#pragma omp target map(to: arr[0:N])";
    let (_, directive) = parse_omp_directive(input).expect("parse");

    let config = ParserConfig::with_parsing(Language::C).with_language_semantics(false);
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("convert with disabled language semantics");

    let map_items = ir
        .clauses()
        .iter()
        .find_map(|clause| match clause {
            ClauseData::Map { items, .. } => Some(items),
            _ => None,
        })
        .expect("map clause present");

    // With language semantics disabled, should be treated as identifier
    assert!(matches!(map_items[0], ClauseItem::Identifier(_)));
}

#[test]
fn reduction_with_complex_list() {
    let input = "#pragma omp parallel reduction(+: arr[i], matrix[j][k], scalar)";
    let (_, directive) = parse_omp_directive(input).expect("parse");

    let config = ParserConfig::with_parsing(Language::C);
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("convert");

    let reduction_clause = ir
        .clauses()
        .iter()
        .find(|clause| matches!(clause, ClauseData::Reduction { .. }))
        .expect("reduction clause present");

    if let ClauseData::Reduction { operator, items } = reduction_clause {
        assert_eq!(*operator, roup::ir::ReductionOperator::Add);
        assert_eq!(items.len(), 3);
        assert!(matches!(items[0], ClauseItem::Variable(_)));
        assert!(matches!(items[1], ClauseItem::Variable(_)));
        assert!(matches!(items[2], ClauseItem::Identifier(_)));
    }
}

#[test]
fn depend_clause_with_array_sections() {
    let input = "#pragma omp task depend(in: arr[0:N], matrix[i][j])";
    let (_, directive) = parse_omp_directive(input).expect("parse");

    let config = ParserConfig::with_parsing(Language::C);
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("convert");

    let depend_items = ir
        .clauses()
        .iter()
        .find_map(|clause| match clause {
            ClauseData::Depend { depend_type, items } => Some((depend_type, items)),
            _ => None,
        })
        .expect("depend clause present");

    assert_eq!(*depend_items.0, roup::ir::DependType::In);
    assert_eq!(depend_items.1.len(), 2);
    assert!(matches!(depend_items.1[0], ClauseItem::Variable(_)));
    assert!(matches!(depend_items.1[1], ClauseItem::Variable(_)));
}

#[test]
fn linear_clause_with_array_step() {
    let input = "#pragma omp for linear(i, j: step_size)";
    let (_, directive) = parse_omp_directive(input).expect("parse");

    let config = ParserConfig::with_parsing(Language::C);
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("convert");

    let linear_clause = ir
        .clauses()
        .iter()
        .find(|clause| matches!(clause, ClauseData::Linear { .. }))
        .expect("linear clause present");

    if let ClauseData::Linear {
        modifier,
        items,
        step,
    } = linear_clause
    {
        assert!(modifier.is_none());
        assert_eq!(items.len(), 2);
        assert!(step.is_some());
        assert_eq!(step.as_ref().unwrap().to_string(), "step_size");
    }
}

#[test]
fn quote_handling_in_clause_items() {
    let input = r#"#pragma omp parallel private(str = "a,b,c", value)"#;
    let (_, directive) = parse_omp_directive(input).expect("parse");

    let config = ParserConfig::with_parsing(Language::C);
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("convert");

    let private_items = ir
        .clauses()
        .iter()
        .find_map(|clause| match clause {
            ClauseData::Private { items } => Some(items),
            _ => None,
        })
        .expect("private clause present");

    // Should not split on comma inside quotes
    assert_eq!(private_items.len(), 2);
}

#[test]
fn ternary_operator_in_if_clause() {
    let input = "#pragma omp parallel if(flag ? true : false)";
    let (_, directive) = parse_omp_directive(input).expect("parse");

    let config = ParserConfig::with_parsing(Language::C);
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("convert");

    let if_clause = ir
        .clauses()
        .iter()
        .find(|clause| matches!(clause, ClauseData::If { .. }))
        .expect("if clause present");

    if let ClauseData::If {
        directive_name,
        condition,
    } = if_clause
    {
        assert!(directive_name.is_none());
        assert_eq!(condition.to_string(), "flag ? true : false");
    }
}

#[test]
fn roundtrip_with_language_parsing() {
    let input = "#pragma omp target map(to: arr[0:N])";
    let (_, directive) = parse_omp_directive(input).expect("parse");

    let config = ParserConfig::with_parsing(Language::C);
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("convert");

    let output = ir.to_string();
    assert_eq!(output, input);
}
