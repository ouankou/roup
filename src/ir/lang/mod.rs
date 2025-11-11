//! Language-aware helpers for parsing clause items.
//!
//! The core IR conversion logic treats clause payloads generically.  This
//! module provides just enough C/C++/Fortran awareness to interpret
//! OpenMP-specific constructs such as array sections while keeping the
//! feature easy to disable via [`ParserConfig`].

use super::{
    ArraySection, ClauseItem, ConversionError, Expression, Identifier, Language, ParserConfig,
    Variable,
};

/// Parse a comma separated list of clause items using language aware rules.
pub fn parse_clause_item_list(
    content: &str,
    config: &ParserConfig,
) -> Result<Vec<ClauseItem>, ConversionError> {
    if !config.language_semantics_enabled() {
        return Ok(fallback_identifier_list(content));
    }

    let language = config.language();
    let segments = split_top_level(content, ',', &[('[', ']'), ('(', ')')]);
    let mut items = Vec::new();

    for raw in segments {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }

        // If it contains a top-level '[' then treat as C-like variable with array sections
        // Detect C-like array sections by splitting on top-level '[' (pairs empty
        // so '[' is recognized unless inside quotes/templates).
        if split_top_level(trimmed, '[', &[] as &[(char, char)]).len() > 1 {
            let variable = parse_c_like_variable(trimmed, config)?;
            items.push(ClauseItem::Variable(variable));
            continue;
        }

        // Fortran form: name(...) with parentheses. Use a simple contains +
        // ends_with check and let parse_fortran_variable handle nesting properly.
        if matches!(language, Language::Fortran) && trimmed.contains('(') && trimmed.ends_with(')')
        {
            if let Some(variable) = parse_fortran_variable(trimmed, config)? {
                items.push(ClauseItem::Variable(variable));
                continue;
            }
        }

        if looks_like_identifier(trimmed) {
            items.push(ClauseItem::Identifier(Identifier::new(trimmed)));
            continue;
        }

        items.push(ClauseItem::Expression(Expression::new(trimmed, config)));
    }

    Ok(items)
}

fn fallback_identifier_list(content: &str) -> Vec<ClauseItem> {
    content
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| ClauseItem::Identifier(Identifier::new(s)))
        .collect()
}

fn parse_c_like_variable(value: &str, config: &ParserConfig) -> Result<Variable, ConversionError> {
    let name_end = value.find('[').ok_or_else(|| {
        ConversionError::InvalidClauseSyntax(format!("expected array designator in `{value}`"))
    })?;

    let name = value[..name_end].trim();
    if name.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(format!(
            "missing variable name before array section in `{value}`"
        )));
    }

    let mut sections = Vec::new();
    let mut rest = &value[name_end..];
    while rest.starts_with('[') {
        let (content, remaining) = extract_bracket_content(rest, '[', ']')?;
        let section = parse_array_section(content, config, false)?; // C uses lower:length
        sections.push(section);
        rest = remaining.trim_start();
    }

    if !rest.trim().is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(format!(
            "unexpected trailing characters `{}` in `{}`",
            rest.trim(),
            value
        )));
    }

    Ok(Variable::with_sections(name, sections))
}

fn parse_fortran_variable(
    value: &str,
    config: &ParserConfig,
) -> Result<Option<Variable>, ConversionError> {
    let trimmed = value.trim();
    let Some(paren_pos) = trimmed.find('(') else {
        return Ok(None);
    };

    let name = trimmed[..paren_pos].trim();
    if name.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(format!(
            "missing variable name before parenthesis in `{value}`"
        )));
    }

    let section_part = &trimmed[paren_pos..];
    let (content, remainder) = extract_bracket_content(section_part, '(', ')')?;
    if !remainder.trim().is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(format!(
            "unexpected trailing characters `{}` in `{}`",
            remainder.trim(),
            value
        )));
    }

    let mut sections = Vec::new();
    if !content.trim().is_empty() {
        for dimension in split_top_level(content, ',', &[('(', ')'), ('[', ']')]) {
            let section = parse_array_section(dimension, config, true)?; // Fortran uses lower:upper
            sections.push(section);
        }
    }

    if sections.is_empty() {
        return Ok(Some(Variable::new(name)));
    }

    Ok(Some(Variable::with_sections(name, sections)))
}

fn parse_array_section(
    value: &str,
    config: &ParserConfig,
    is_fortran: bool,
) -> Result<ArraySection, ConversionError> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return Ok(ArraySection::all());
    }

    let parts = split_top_level(trimmed, ':', &[('(', ')'), ('[', ']')]);
    match parts.len() {
        1 => {
            let expr = Expression::new(parts[0], config);
            Ok(ArraySection::single_index(expr))
        }
        2 => {
            let lower = if parts[0].is_empty() {
                None
            } else {
                Some(Expression::new(parts[0], config))
            };

            // Fortran uses lower:upper, C uses lower:length
            let length = if parts[1].is_empty() {
                None
            } else if is_fortran {
                // Convert Fortran upper bound to length: length = upper - lower + 1
                let upper = Expression::new(parts[1], config);
                Some(fortran_upper_to_length(lower.as_ref(), upper, None, config))
            } else {
                Some(Expression::new(parts[1], config))
            };
            Ok(ArraySection::new(lower, length, None))
        }
        3 => {
            let lower = if parts[0].is_empty() {
                None
            } else {
                Some(Expression::new(parts[0], config))
            };

            let stride = if parts[2].is_empty() {
                None
            } else {
                Some(Expression::new(parts[2], config))
            };

            // Fortran uses lower:upper:stride, C uses lower:length:stride
            let length = if parts[1].is_empty() {
                None
            } else if is_fortran {
                // Convert Fortran upper bound to length with stride: ((upper - lower) / stride) + 1
                let upper = Expression::new(parts[1], config);
                Some(fortran_upper_to_length(
                    lower.as_ref(),
                    upper,
                    stride.as_ref(),
                    config,
                ))
            } else {
                Some(Expression::new(parts[1], config))
            };

            Ok(ArraySection::new(lower, length, stride))
        }
        _ => Err(ConversionError::InvalidClauseSyntax(format!(
            "too many ':' separators in array section `{value}`"
        ))),
    }
}

/// Convert Fortran upper bound to length expression with optional stride
///
/// Fortran array sections use lower:upper[:stride] syntax where both bounds are inclusive.
/// ArraySection stores length, so we convert:
/// - Without stride: length = upper - lower + 1
/// - With stride: length = ((upper - lower) / stride) + 1
///
/// # Examples
///
/// - `A(5:10)` → lower=5, upper=10 → length = (10 - 5 + 1) = 6 elements
/// - `A(:10)` → lower=implicit 1, upper=10 → length = 10 elements
/// - `A(1:N)` → lower=1, upper=N → length = N elements
/// - `A(1:10:2)` → lower=1, upper=10, stride=2 → length = ((10 - 1) / 2) + 1 = 5 elements
fn fortran_upper_to_length(
    lower: Option<&Expression>,
    upper: Expression,
    stride: Option<&Expression>,
    config: &ParserConfig,
) -> Expression {
    match (lower, stride) {
        (Some(lower_expr), Some(stride_expr)) => {
            // With explicit lower and stride: ((upper - lower) / stride) + 1
            Expression::new(
                format!(
                    "((({}) - ({})) / ({}) + 1)",
                    upper.as_str(),
                    lower_expr.as_str(),
                    stride_expr.as_str()
                ),
                config,
            )
        }
        (Some(lower_expr), None) => {
            // With explicit lower, no stride: (upper - lower + 1)
            Expression::new(
                format!("(({}) - ({}) + 1)", upper.as_str(), lower_expr.as_str()),
                config,
            )
        }
        (None, Some(stride_expr)) => {
            // No lower (implicit 1), with stride: ((upper - 1) / stride) + 1
            Expression::new(
                format!(
                    "((({}) - 1) / ({}) + 1)",
                    upper.as_str(),
                    stride_expr.as_str()
                ),
                config,
            )
        }
        (None, None) => {
            // No lower (implicit 1), no stride: upper - 1 + 1 = upper
            upper
        }
    }
}

fn split_top_level<'a>(input: &'a str, separator: char, pairs: &[(char, char)]) -> Vec<&'a str> {
    let mut segments = Vec::new();
    let mut start = 0;
    let mut stack: Vec<char> = Vec::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut angle_depth = 0usize;

    let chars: Vec<(usize, char)> = input.char_indices().collect();
    let mut i = 0;

    while i < chars.len() {
        let (idx, ch) = chars[i];
        let prev = if i > 0 { Some(chars[i - 1].1) } else { None };
        let next = chars.get(i + 1).map(|(_, c)| *c);

        // Track quote state
        match ch {
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            _ if in_single_quote || in_double_quote => {
                // Inside quotes, skip delimiter checking
            }
            '<' if stack.is_empty() => {
                // Detect C++ templates
                if is_template_start(prev, next) {
                    angle_depth += 1;
                }
            }
            '>' if angle_depth > 0 && stack.is_empty() => {
                // Close template - handle >> as two closings for nested templates
                // Skip only if this is part of >>= or >= (comparison operators)
                if matches!(next, Some('=')) && !matches!(prev, Some('>')) {
                    // This is >= comparison, skip it
                } else {
                    // This is either a single > or part of >> (nested template closing)
                    // Both should decrement angle_depth
                    angle_depth = angle_depth.saturating_sub(1);
                }
            }
            _ if !in_single_quote && !in_double_quote => {
                // Track paired delimiters
                if let Some(&(_, close)) = pairs.iter().find(|(open, _)| *open == ch) {
                    stack.push(close);
                } else if stack.last().copied() == Some(ch) {
                    stack.pop();
                }
            }
            _ => {}
        }

        // Split at separator only if not inside any structure
        if ch == separator
            && stack.is_empty()
            && angle_depth == 0
            && !in_single_quote
            && !in_double_quote
        {
            segments.push(input[start..idx].trim());
            start = idx + ch.len_utf8();
        }

        i += 1;
    }

    segments.push(input[start..].trim());
    segments
}

/// Check if '<' is likely a template start rather than comparison operator.
fn is_template_start(prev: Option<char>, next: Option<char>) -> bool {
    let is_prev_valid =
        matches!(prev, Some(c) if c.is_alphanumeric() || c == '_' || c == ':' || c == '>');
    let is_next_disqualifier = matches!(next, Some('=') | Some('<') | Some('>'));
    is_prev_valid && !is_next_disqualifier
}

/// Split the input once at the first top-level occurrence of `delimiter`.
///
/// Returns None if the delimiter is not found at the top level.
pub fn split_once_top_level(input: &str, delimiter: char) -> Option<(&str, &str)> {
    find_top_level_delimiter(input, delimiter, false).map(|idx| {
        let next = idx + delimiter.len_utf8();
        (&input[..idx], &input[next..])
    })
}

/// Split the input once at the last top-level occurrence of `delimiter`.
///
/// Returns None if the delimiter is not found at the top level.
pub fn rsplit_once_top_level(input: &str, delimiter: char) -> Option<(&str, &str)> {
    find_top_level_delimiter(input, delimiter, true).map(|idx| {
        let next = idx + delimiter.len_utf8();
        (&input[..idx], &input[next..])
    })
}

/// Find the first (or last) top-level occurrence of a delimiter.
///
/// Respects nesting of parentheses, brackets, braces, and quotes.
/// For colon delimiter, also handles ternary operator (? :) disambiguation
/// and C++ scope operator (::) to avoid false matches.
fn find_top_level_delimiter(input: &str, delimiter: char, from_end: bool) -> Option<usize> {
    let mut depth_paren = 0i32;
    let mut depth_bracket = 0i32;
    let mut depth_brace = 0i32;
    let mut ternary_depth = 0i32;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut result: Option<usize> = None;

    let chars: Vec<(usize, char)> = input.char_indices().collect();

    for i in 0..chars.len() {
        let (idx, ch) = chars[i];
        let next_ch = chars.get(i + 1).map(|(_, c)| *c);
        let prev_ch = if i > 0 { Some(chars[i - 1].1) } else { None };

        // Track quote state
        match ch {
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
                continue;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
                continue;
            }
            _ if in_single_quote || in_double_quote => {
                continue;
            }
            _ => {}
        }

        // Track delimiter depth
        match ch {
            '(' => depth_paren += 1,
            ')' => {
                if depth_paren > 0 {
                    depth_paren -= 1;
                }
            }
            '[' => depth_bracket += 1,
            ']' => {
                if depth_bracket > 0 {
                    depth_bracket -= 1;
                }
            }
            '{' => depth_brace += 1,
            '}' => {
                if depth_brace > 0 {
                    depth_brace -= 1;
                }
            }
            '?' if delimiter == ':' => ternary_depth += 1,
            ':' if delimiter == ':' && ternary_depth > 0 => {
                ternary_depth -= 1;
                continue;
            }
            _ => {}
        }

        if ch == delimiter && depth_paren == 0 && depth_bracket == 0 && depth_brace == 0 {
            // Skip C++ scope operator :: when delimiter is :
            if delimiter == ':' && (prev_ch == Some(':') || next_ch == Some(':')) {
                continue;
            }

            result = Some(idx);
            if !from_end {
                break;
            }
        }
    }

    result
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

    let mut depth = 0;
    let mut start_idx = None;
    for (idx, ch) in input.char_indices() {
        if ch == open {
            depth += 1;
            if depth == 1 {
                start_idx = Some(idx + open.len_utf8());
            }
        } else if ch == close {
            depth -= 1;
            if depth == 0 {
                let start = start_idx.expect("opening bracket must set start index");
                let content = &input[start..idx];
                let rest = &input[idx + close.len_utf8()..];
                return Ok((content, rest));
            }
        }
    }

    Err(ConversionError::InvalidClauseSyntax(format!(
        "unterminated `{open}` block in `{input}`"
    )))
}

fn looks_like_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some(ch) if is_identifier_start(ch) => chars.all(is_identifier_continue),
        _ => false,
    }
}

fn is_identifier_start(ch: char) -> bool {
    ch.is_alphabetic() || ch == '_'
}

fn is_identifier_continue(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_for(language: Language) -> ParserConfig {
        ParserConfig::with_parsing(language)
    }

    #[test]
    fn parses_c_array_sections() {
        let config = config_for(Language::C);
        let items = parse_clause_item_list("arr[0:N], scalar", &config).unwrap();

        assert_eq!(items.len(), 2);
        match &items[0] {
            ClauseItem::Variable(var) => {
                assert_eq!(var.name(), "arr");
                assert_eq!(var.array_sections.len(), 1);
                let section = &var.array_sections[0];
                assert!(section.lower_bound.is_some());
                assert!(section.length.is_some());
            }
            other => panic!("expected variable, got {other:?}"),
        }
    }

    #[test]
    fn parses_fortran_parentheses_sections() {
        let config = config_for(Language::Fortran);
        let items = parse_clause_item_list("A(1:N), B(:, :)", &config).unwrap();
        assert_eq!(items.len(), 2);
        assert!(matches!(items[1], ClauseItem::Variable(_)));
    }

    #[test]
    fn falls_back_to_identifier_when_disabled() {
        let mut config = ParserConfig::with_parsing(Language::C);
        config = config.with_language_semantics(false);
        let items = parse_clause_item_list("arr[0:N]", &config).unwrap();
        assert!(matches!(items[0], ClauseItem::Identifier(_)));
    }

    #[test]
    fn parses_cpp_templates_without_splitting() {
        let config = config_for(Language::Cpp);
        let items = parse_clause_item_list("std::map<int, float>[idx], data", &config).unwrap();
        assert_eq!(items.len(), 2);
        match &items[0] {
            ClauseItem::Variable(var) => {
                // The template part should be preserved in the variable name
                assert!(var.name().contains("std::map"));
                assert_eq!(var.array_sections.len(), 1);
            }
            other => panic!("expected variable with template, got {other:?}"),
        }
    }

    #[test]
    fn parses_nested_cpp_templates() {
        // Nested templates like std::vector<std::pair<int,int>> use >> closing
        let config = config_for(Language::Cpp);
        let items =
            parse_clause_item_list("std::vector<std::pair<int,int>>, other", &config).unwrap();
        assert_eq!(items.len(), 2);
        match &items[0] {
            ClauseItem::Expression(expr) => {
                // Should preserve the entire nested template type
                assert!(expr.as_str().contains("std::vector<std::pair<int,int>>"));
            }
            other => panic!("expected expression with nested template, got {other:?}"),
        }
        match &items[1] {
            ClauseItem::Identifier(id) => {
                assert_eq!(id.as_str(), "other");
            }
            other => panic!("expected identifier 'other', got {other:?}"),
        }
    }

    #[test]
    fn parses_deeply_nested_templates() {
        // Test even deeper nesting: map<string, vector<pair<int,int>>>
        let config = config_for(Language::Cpp);
        let items = parse_clause_item_list(
            "std::map<std::string, std::vector<std::pair<int,int>>>, x",
            &config,
        )
        .unwrap();
        assert_eq!(items.len(), 2);
        match &items[0] {
            ClauseItem::Expression(expr) => {
                assert!(expr.as_str().contains("std::map"));
                assert!(expr.as_str().contains("std::vector"));
                assert!(expr.as_str().contains("std::pair"));
            }
            other => panic!("expected expression with deeply nested template, got {other:?}"),
        }
    }

    #[test]
    fn parses_nested_array_sections() {
        let config = config_for(Language::C);
        let items = parse_clause_item_list("matrix[0:N][i:j]", &config).unwrap();
        assert_eq!(items.len(), 1);
        match &items[0] {
            ClauseItem::Variable(var) => {
                assert_eq!(var.name(), "matrix");
                assert_eq!(var.array_sections.len(), 2);
            }
            other => panic!("expected nested array sections, got {other:?}"),
        }
    }

    #[test]
    fn handles_fortran_multi_dimensional_arrays() {
        let config = config_for(Language::Fortran);
        let items = parse_clause_item_list("field(1:n, :, 2:m:2)", &config).unwrap();
        assert_eq!(items.len(), 1);
        match &items[0] {
            ClauseItem::Variable(var) => {
                assert_eq!(var.name(), "field");
                assert_eq!(var.array_sections.len(), 3);
                // Third dimension should have stride
                assert!(var.array_sections[2].stride.is_some());
            }
            other => panic!("expected Fortran multi-dimensional array, got {other:?}"),
        }
    }

    #[test]
    fn split_once_respects_parentheses() {
        // Colon is inside parentheses, should not be found at top level
        let result = super::split_once_top_level("map(to: arr)", ':');
        assert_eq!(result, None);

        // Colon is outside parentheses
        let result = super::split_once_top_level("type: value(with:colon)", ':');
        assert_eq!(result, Some(("type", " value(with:colon)")));
    }

    #[test]
    fn rsplit_once_finds_last_top_level() {
        let result = super::rsplit_once_top_level("i: j: k", ':');
        assert_eq!(result, Some(("i: j", " k")));
    }

    #[test]
    fn split_ignores_colon_in_ternary() {
        let result = super::split_once_top_level("x = a ? b : c, y", ',');
        assert_eq!(result, Some(("x = a ? b : c", " y")));
    }

    #[test]
    fn split_respects_quotes() {
        let config = config_for(Language::C);
        let items = parse_clause_item_list(r#"str = "a,b,c", value"#, &config).unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn ignores_cpp_scope_operator() {
        // C++ scope operator :: should not be treated as colon delimiter
        let result = super::split_once_top_level("std::vector<int>", ':');
        assert_eq!(result, None); // No top-level colon, only ::

        // Test that actual delimiter after :: still works
        let result = super::split_once_top_level("std::type: value", ':');
        assert_eq!(result, Some(("std::type", " value")));

        // Test parsing map clause with C++ scoped types
        let config = config_for(Language::Cpp);
        let items = parse_clause_item_list("std::vector<int>& buf", &config).unwrap();
        assert_eq!(items.len(), 1);
        match &items[0] {
            ClauseItem::Expression(expr) => {
                // Should preserve the :: in the type name
                assert!(expr.as_str().contains("std::vector"));
            }
            other => panic!("expected expression with C++ scope operator, got {other:?}"),
        }
    }

    #[test]
    fn handles_empty_array_sections() {
        let config = config_for(Language::C);
        let items = parse_clause_item_list("arr[:]", &config).unwrap();
        assert_eq!(items.len(), 1);
        match &items[0] {
            ClauseItem::Variable(var) => {
                let section = &var.array_sections[0];
                assert!(section.lower_bound.is_none());
                assert!(section.length.is_none());
            }
            other => panic!("expected variable with empty section, got {other:?}"),
        }
    }

    #[test]
    fn fortran_converts_upper_bound_to_length() {
        // Fortran: A(5:10) means elements 5,6,7,8,9,10 = 6 elements
        let config = config_for(Language::Fortran);
        let items = parse_clause_item_list("A(5:10)", &config).unwrap();
        assert_eq!(items.len(), 1);

        match &items[0] {
            ClauseItem::Variable(var) => {
                assert_eq!(var.name(), "A");
                assert_eq!(var.array_sections.len(), 1);
                let section = &var.array_sections[0];

                // Lower bound should be 5
                assert_eq!(section.lower_bound.as_ref().unwrap().as_str(), "5");

                // Length should be computed as (10 - 5 + 1)
                assert_eq!(
                    section.length.as_ref().unwrap().as_str(),
                    "((10) - (5) + 1)"
                );
            }
            other => panic!("expected Fortran variable with array section, got {other:?}"),
        }
    }

    #[test]
    fn fortran_implicit_lower_bound_is_1() {
        // Fortran: A(:10) means elements 1-10 = 10 elements
        let config = config_for(Language::Fortran);
        let items = parse_clause_item_list("A(:10)", &config).unwrap();
        assert_eq!(items.len(), 1);

        match &items[0] {
            ClauseItem::Variable(var) => {
                assert_eq!(var.name(), "A");
                let section = &var.array_sections[0];

                // No explicit lower bound
                assert!(section.lower_bound.is_none());

                // Length should be 10 (upper bound when lower is implicit 1)
                assert_eq!(section.length.as_ref().unwrap().as_str(), "10");
            }
            other => panic!("expected Fortran variable, got {other:?}"),
        }
    }

    #[test]
    fn c_uses_length_not_upper_bound() {
        // C: arr[5:10] means 10 elements starting at index 5
        let config = config_for(Language::C);
        let items = parse_clause_item_list("arr[5:10]", &config).unwrap();
        assert_eq!(items.len(), 1);

        match &items[0] {
            ClauseItem::Variable(var) => {
                assert_eq!(var.name(), "arr");
                let section = &var.array_sections[0];

                // Lower bound should be 5
                assert_eq!(section.lower_bound.as_ref().unwrap().as_str(), "5");

                // Length should be 10 (not converted)
                assert_eq!(section.length.as_ref().unwrap().as_str(), "10");
            }
            other => panic!("expected C variable, got {other:?}"),
        }
    }

    #[test]
    fn fortran_accounts_for_stride_in_length() {
        // Fortran: A(1:10:2) means elements 1,3,5,7,9 = 5 elements
        let config = config_for(Language::Fortran);
        let items = parse_clause_item_list("A(1:10:2)", &config).unwrap();
        assert_eq!(items.len(), 1);

        match &items[0] {
            ClauseItem::Variable(var) => {
                assert_eq!(var.name(), "A");
                let section = &var.array_sections[0];

                // Lower bound should be 1
                assert_eq!(section.lower_bound.as_ref().unwrap().as_str(), "1");

                // Stride should be 2
                assert_eq!(section.stride.as_ref().unwrap().as_str(), "2");

                // Length should be ((10 - 1) / 2) + 1 = 5
                assert_eq!(
                    section.length.as_ref().unwrap().as_str(),
                    "(((10) - (1)) / (2) + 1)"
                );
            }
            other => panic!("expected Fortran variable with stride, got {other:?}"),
        }
    }

    #[test]
    fn fortran_stride_with_implicit_lower_bound() {
        // Fortran: A(:10:3) means elements 1,4,7,10 = 4 elements
        let config = config_for(Language::Fortran);
        let items = parse_clause_item_list("A(:10:3)", &config).unwrap();
        assert_eq!(items.len(), 1);

        match &items[0] {
            ClauseItem::Variable(var) => {
                assert_eq!(var.name(), "A");
                let section = &var.array_sections[0];

                // No explicit lower bound (implicit 1)
                assert!(section.lower_bound.is_none());

                // Stride should be 3
                assert_eq!(section.stride.as_ref().unwrap().as_str(), "3");

                // Length should be ((10 - 1) / 3) + 1 = 4
                assert_eq!(
                    section.length.as_ref().unwrap().as_str(),
                    "(((10) - 1) / (3) + 1)"
                );
            }
            other => {
                panic!("expected Fortran variable with implicit lower and stride, got {other:?}")
            }
        }
    }
}
