//! Language-aware helpers for clause syntax.
//!
//! Clause items are parsed exactly once by the host-language expression
//! parser. Classification into identifier, variable designator, or general
//! expression is a structural inspection of that tree. No string rendering,
//! synthetic expression generation, or second array-section representation is
//! involved.

use super::{ClauseItem, ConversionError, Expression, Identifier, ParserConfig, Variable};
use crate::delimiter::{self, CommentStyle};
use crate::host::{ExprKind, HostLanguage, QualifiedName};

/// Parse a comma separated list of clause items using language aware rules.
pub fn parse_clause_item_list(
    content: &str,
    config: &ParserConfig,
) -> Result<Vec<ClauseItem>, ConversionError> {
    let segments = split_top_level(content, ',', &[('[', ']'), ('(', ')')])?;
    let mut items = Vec::with_capacity(segments.len());

    for raw in segments {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(ConversionError::InvalidClauseSyntax(
                "clause item lists must not contain empty entries".to_string(),
            ));
        }

        if let Some(common_block) = parse_fortran_common_block(trimmed, config)? {
            items.push(ClauseItem::FortranCommonBlock(common_block));
            continue;
        }

        if config.source_extensions()
            && let Some(identifier) = trimmed.strip_suffix('/')
            && !identifier.is_empty()
            && !identifier.contains('/')
        {
            items.push(ClauseItem::OmpparserTrailingSlash(Identifier::new(
                identifier,
            )?));
            continue;
        }

        let expression = Expression::new_with_legacy_qualified_value(trimmed, config)?;
        items.push(classify_clause_item(expression)?);
    }

    Ok(items)
}

fn parse_fortran_common_block(
    source: &str,
    config: &ParserConfig,
) -> Result<Option<Identifier>, ConversionError> {
    if config.host_language() != HostLanguage::Fortran {
        return Ok(None);
    }

    if !source.starts_with('/') && !source.ends_with('/') {
        return Ok(None);
    }
    let Some(name) = source
        .strip_prefix('/')
        .and_then(|inner| inner.strip_suffix('/'))
        .map(str::trim)
        .filter(|name| !name.is_empty() && !name.contains('/'))
    else {
        return Err(ConversionError::InvalidClauseSyntax(
            "malformed Fortran named common block; expected `/name/`".to_string(),
        ));
    };

    Identifier::new(name.to_ascii_lowercase())
        .map(Some)
        .map_err(ConversionError::from)
}

fn classify_clause_item(expression: Expression) -> Result<ClauseItem, ConversionError> {
    if let ExprKind::Name(QualifiedName {
        global: false,
        segments,
    }) = &expression.ast().kind
        && segments.len() == 1
    {
        return Ok(ClauseItem::Identifier(segments[0].clone()));
    }

    if Variable::is_designator_expression(&expression) {
        return Variable::from_expression(expression)
            .map(ClauseItem::Variable)
            .map_err(|error| ConversionError::InvalidClauseSyntax(error.to_string()));
    }

    Ok(ClauseItem::Expression(expression))
}

pub(crate) fn split_top_level<'a>(
    input: &'a str,
    separator: char,
    pairs: &[(char, char)],
) -> Result<Vec<&'a str>, ConversionError> {
    delimiter::split_top_level(input, separator, pairs, CommentStyle::Block)
        .map_err(invalid_split_syntax)
}

fn invalid_split_syntax(error: impl std::fmt::Display) -> ConversionError {
    ConversionError::InvalidClauseSyntax(error.to_string())
}

/// Split the input once at the first top-level occurrence of `delimiter`.
///
/// Returns `Ok(None)` if the delimiter is not found at the top level and an
/// error if the input contains malformed nesting or an unclosed quote.
pub fn split_once_top_level(
    input: &str,
    delimiter: char,
) -> Result<Option<(&str, &str)>, ConversionError> {
    Ok(
        find_top_level_delimiter(input, delimiter, false)?.map(|idx| {
            let next = idx + delimiter.len_utf8();
            (&input[..idx], &input[next..])
        }),
    )
}

/// Find the first (or last) top-level occurrence of a delimiter.
///
/// Respects nesting of parentheses, brackets, braces, and quotes.
/// For colon delimiter, also handles ternary operator (? :) disambiguation
/// and C++ scope operator (::) to avoid false matches.
fn find_top_level_delimiter(
    input: &str,
    delimiter: char,
    from_end: bool,
) -> Result<Option<usize>, ConversionError> {
    delimiter::find_top_level_delimiter(input, delimiter, from_end, CommentStyle::Block)
        .map_err(invalid_split_syntax)
}

/// Extract content from a bracketed section, handling nesting.
///
/// Returns (content, remainder) where content is what's inside the delimiters.
/// Supports any pair of delimiters: `()`, `[]`, `{}`, etc.
///
/// This is used by both the lang module and convert.rs to avoid duplication.
pub(crate) fn extract_bracket_content(
    input: &str,
    open: char,
    close: char,
) -> Result<(&str, &str), ConversionError> {
    if !input.starts_with(open) {
        return Err(ConversionError::InvalidClauseSyntax(format!(
            "expected `{input}` to start with '{open}'"
        )));
    }

    let closing = delimiter::find_matching_delimiter(input, 0, open, close, CommentStyle::Block)
        .map_err(invalid_split_syntax)?
        .ok_or_else(|| {
            ConversionError::InvalidClauseSyntax(format!(
                "unterminated `{open}` block in `{input}`"
            ))
        })?;
    let content_start = open.len_utf8();
    Ok((
        &input[content_start..closing],
        &input[closing + close.len_utf8()..],
    ))
}

pub(crate) fn find_matching_delimiter(
    input: &str,
    open_position: usize,
    open: char,
    close: char,
) -> Result<Option<usize>, ConversionError> {
    delimiter::find_matching_delimiter(input, open_position, open, close, CommentStyle::Block)
        .map_err(invalid_split_syntax)
}

pub(crate) fn find_matching_after_open(
    input: &str,
    open: char,
    close: char,
) -> Result<Option<usize>, ConversionError> {
    delimiter::find_matching_after_open(input, open, close, CommentStyle::Block)
        .map_err(invalid_split_syntax)
}

#[cfg(test)]
mod tests {
    use crate::host::{FortranArgument, SectionSemantics, Subscript};
    use crate::version::HostLanguage;

    use super::*;

    fn config_for(language: HostLanguage) -> ParserConfig {
        ParserConfig::from_language(language)
    }

    #[test]
    fn parses_c_array_sections() {
        let config = config_for(HostLanguage::C);
        let items = parse_clause_item_list("arr[0:N], scalar", &config).unwrap();

        assert_eq!(items.len(), 2);
        match &items[0] {
            ClauseItem::Variable(var) => {
                assert_eq!(var.dimensions(), 1);
                let ExprKind::Subscript { subscript, .. } = &var.ast().kind else {
                    panic!("expected typed subscript")
                };
                let Subscript::Section(section) = subscript else {
                    panic!("expected typed section")
                };
                assert_eq!(section.semantics, SectionSemantics::CLength);
                assert!(section.lower.is_some());
                assert!(section.upper_or_length.is_some());
            }
            other => panic!("expected variable, got {other:?}"),
        }
    }

    #[test]
    fn parses_fortran_parentheses_sections() {
        let config = config_for(HostLanguage::Fortran);
        let items = parse_clause_item_list("A(1:N), B(:, :)", &config).unwrap();
        assert_eq!(items.len(), 2);
        assert!(matches!(items[1], ClauseItem::Variable(_)));
    }

    #[test]
    fn malformed_item_does_not_fall_back_to_an_identifier() {
        let config = ParserConfig::c();
        assert!(parse_clause_item_list("arr[0:@]", &config).is_err());
    }

    #[test]
    fn non_designator_array_base_does_not_fall_back_to_an_expression() {
        let config = ParserConfig::c();
        let items = parse_clause_item_list("make_array()[0:N]", &config).unwrap();
        assert!(matches!(items.as_slice(), [ClauseItem::Expression(_)]));
    }

    #[test]
    fn unsupported_cpp_template_designators_are_hard_errors() {
        let config = config_for(HostLanguage::Cpp);
        for source in [
            "std::map<int, float>[idx], data",
            "std::vector<std::pair<int,int>>, other",
            "std::map<std::string, std::vector<std::pair<int,int>>>, x",
        ] {
            assert!(parse_clause_item_list(source, &config).is_err());
        }
    }

    #[test]
    fn parses_nested_array_sections() {
        let config = config_for(HostLanguage::C);
        let items = parse_clause_item_list("matrix[0:N][i:j]", &config).unwrap();
        assert_eq!(items.len(), 1);
        match &items[0] {
            ClauseItem::Variable(var) => {
                assert_eq!(var.dimensions(), 2);
            }
            other => panic!("expected nested array sections, got {other:?}"),
        }
    }

    #[test]
    fn handles_fortran_multi_dimensional_arrays() {
        let config = config_for(HostLanguage::Fortran);
        let items = parse_clause_item_list("field(1:n, :, 2:m:2)", &config).unwrap();
        assert_eq!(items.len(), 1);
        match &items[0] {
            ClauseItem::Variable(var) => {
                assert_eq!(var.dimensions(), 3);
                let ExprKind::FortranApply { arguments, .. } = &var.ast().kind else {
                    panic!("expected Fortran designator")
                };
                let FortranArgument::Section(section) = &arguments[2] else {
                    panic!("expected section triplet")
                };
                assert!(section.stride.is_some());
            }
            other => panic!("expected Fortran multi-dimensional array, got {other:?}"),
        }
    }

    #[test]
    fn fortran_variable_contains_typed_dimensions() {
        let config = config_for(HostLanguage::Fortran);
        let items = parse_clause_item_list("val(a,b,c)", &config).unwrap();
        match &items[0] {
            ClauseItem::Variable(var) => {
                assert_eq!(var.dimensions(), 3);
            }
            other => panic!("expected typed Fortran variable, got {other:?}"),
        }
    }

    #[test]
    fn split_once_respects_parentheses() {
        // Colon is inside parentheses, should not be found at top level
        let result = super::split_once_top_level("map(to: arr)", ':').unwrap();
        assert_eq!(result, None);

        // Colon is outside parentheses
        let result = super::split_once_top_level("type: value(with:colon)", ':').unwrap();
        assert_eq!(result, Some(("type", " value(with:colon)")));
    }

    #[test]
    fn split_ignores_colon_in_ternary() {
        let result = super::split_once_top_level("x = a ? b : c, y", ',').unwrap();
        assert_eq!(result, Some(("x = a ? b : c", " y")));
    }

    #[test]
    fn split_respects_quotes() {
        let config = config_for(HostLanguage::C);
        let items = parse_clause_item_list(r#"str = "a,b,c", value"#, &config).unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn comparisons_are_not_implicitly_treated_as_cpp_templates() {
        let config = config_for(HostLanguage::Cpp);
        let items = parse_clause_item_list("a<b,c>d", &config).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].to_string(), "a < b");
        assert_eq!(items[1].to_string(), "c > d");
    }

    #[test]
    fn angle_tracking_is_enabled_only_when_requested() {
        let items = split_top_level(
            "std::pair<int,float>,long",
            ',',
            &[('(', ')'), ('[', ']'), ('<', '>')],
        )
        .unwrap();
        assert_eq!(items, ["std::pair<int,float>", "long"]);
    }

    #[test]
    fn malformed_top_level_lists_are_hard_errors() {
        let pairs = &[('(', ')'), ('[', ']'), ('{', '}')];
        for source in [
            "",
            "a,,b",
            ",a",
            "a,",
            "f(a,b",
            "a],b",
            "f(a],b",
            r#""unterminated,a"#,
            r#"'unterminated,a"#,
        ] {
            assert!(
                split_top_level(source, ',', pairs).is_err(),
                "{source:?} must be rejected"
            );
        }
        assert!(split_top_level("a>,b", ',', &[('<', '>')]).is_err());
    }

    #[test]
    fn split_once_validates_the_entire_input() {
        for source in ["kind:value(foo", "kind:value]", r#"kind:"value"#] {
            assert!(
                split_once_top_level(source, ':').is_err(),
                "{source:?} must be rejected"
            );
        }
    }

    #[test]
    fn ignores_cpp_scope_operator() {
        // C++ scope operator :: should not be treated as colon delimiter
        let result = super::split_once_top_level("std::vector<int>", ':').unwrap();
        assert_eq!(result, None); // No top-level colon, only ::

        // Test that actual delimiter after :: still works
        let result = super::split_once_top_level("std::type: value", ':').unwrap();
        assert_eq!(result, Some(("std::type", " value")));

        // A supported qualified expression stays typed.
        let config = config_for(HostLanguage::Cpp);
        let items = parse_clause_item_list("std::value + offset", &config).unwrap();
        assert_eq!(items.len(), 1);
        match &items[0] {
            ClauseItem::Expression(expr) => {
                assert_eq!(expr.to_string(), "std::value + offset");
            }
            other => panic!("expected qualified expression, got {other:?}"),
        }
    }

    #[test]
    fn handles_empty_array_sections() {
        let config = config_for(HostLanguage::C);
        let items = parse_clause_item_list("arr[:]", &config).unwrap();
        assert_eq!(items.len(), 1);
        match &items[0] {
            ClauseItem::Variable(var) => assert_eq!(var.to_string(), "arr[:]"),
            other => panic!("expected variable with empty section, got {other:?}"),
        }
    }

    #[test]
    fn fortran_preserves_upper_bound_without_synthetic_reparse() {
        let config = config_for(HostLanguage::Fortran);
        let items = parse_clause_item_list("A(5:10)", &config).unwrap();
        let ClauseItem::Variable(var) = &items[0] else {
            panic!("expected Fortran variable")
        };
        assert_eq!(var.to_string(), "a(5:10)");
        let ExprKind::FortranApply { arguments, .. } = &var.ast().kind else {
            panic!("expected Fortran application")
        };
        let FortranArgument::Section(section) = &arguments[0] else {
            panic!("expected section")
        };
        assert_eq!(section.semantics, SectionSemantics::FortranUpperBound);
        assert_eq!(
            section
                .lower
                .as_ref()
                .map(|expression| expression.canonical(HostLanguage::Fortran).to_string())
                .as_deref(),
            Some("5")
        );
        assert_eq!(
            section
                .upper_or_length
                .as_ref()
                .map(|expression| expression.canonical(HostLanguage::Fortran).to_string())
                .as_deref(),
            Some("10")
        );
    }

    #[test]
    fn fortran_implicit_lower_bound_remains_implicit() {
        let config = config_for(HostLanguage::Fortran);
        let items = parse_clause_item_list("A(:10)", &config).unwrap();
        assert_eq!(items.len(), 1);

        let ClauseItem::Variable(var) = &items[0] else {
            panic!("expected Fortran variable")
        };
        assert_eq!(var.to_string(), "a(:10)");
    }

    #[test]
    fn c_uses_length_not_upper_bound() {
        // C: arr[5:10] means 10 elements starting at index 5
        let config = config_for(HostLanguage::C);
        let items = parse_clause_item_list("arr[5:10]", &config).unwrap();
        assert_eq!(items.len(), 1);

        let ClauseItem::Variable(var) = &items[0] else {
            panic!("expected C variable")
        };
        assert_eq!(var.to_string(), "arr[5:10]");
    }

    #[test]
    fn fortran_accounts_for_stride_in_length() {
        // Fortran: A(1:10:2) means elements 1,3,5,7,9 = 5 elements
        let config = config_for(HostLanguage::Fortran);
        let items = parse_clause_item_list("A(1:10:2)", &config).unwrap();
        assert_eq!(items.len(), 1);

        let ClauseItem::Variable(var) = &items[0] else {
            panic!("expected Fortran variable")
        };
        assert_eq!(var.to_string(), "a(1:10:2)");
    }

    #[test]
    fn fortran_stride_with_implicit_lower_bound() {
        // Fortran: A(:10:3) means elements 1,4,7,10 = 4 elements
        let config = config_for(HostLanguage::Fortran);
        let items = parse_clause_item_list("A(:10:3)", &config).unwrap();
        assert_eq!(items.len(), 1);

        let ClauseItem::Variable(var) = &items[0] else {
            panic!("expected Fortran variable")
        };
        assert_eq!(var.to_string(), "a(:10:3)");
    }
}
