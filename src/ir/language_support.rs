use super::{ArraySection, ClauseItem, Expression, Identifier, Language, ParserConfig, Variable};

/// Parse a comma-separated list of clause items using language-specific rules.
///
/// When language support is disabled via [`ParserConfig::with_language_support`],
/// the parser falls back to treating every entry as a plain identifier.
pub(crate) fn parse_clause_item_list(content: &str, config: &ParserConfig) -> Vec<ClauseItem> {
    if !config.language_support_enabled() {
        return fallback_identifier_list(content);
    }

    split_top_level(content, ',')
        .into_iter()
        .filter_map(|item| parse_clause_item(item.trim(), config))
        .collect()
}

fn fallback_identifier_list(content: &str) -> Vec<ClauseItem> {
    content
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| ClauseItem::Identifier(Identifier::new(s)))
        .collect()
}

fn parse_clause_item(raw: &str, config: &ParserConfig) -> Option<ClauseItem> {
    if raw.is_empty() {
        return None;
    }

    if let Some(var) = parse_variable(raw, config) {
        return Some(ClauseItem::Variable(var));
    }

    if looks_like_identifier(raw) {
        return Some(ClauseItem::Identifier(Identifier::new(raw)));
    }

    Some(ClauseItem::Expression(Expression::new(raw, config)))
}

fn parse_variable(raw: &str, config: &ParserConfig) -> Option<Variable> {
    match config.language {
        Language::Fortran => parse_fortran_variable(raw, config),
        Language::C | Language::Cpp | Language::Unknown => parse_c_variable(raw, config),
    }
}

fn parse_c_variable(raw: &str, config: &ParserConfig) -> Option<Variable> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut name_end = trimmed.len();
    for (idx, ch) in trimmed.char_indices() {
        match ch {
            '[' => {
                name_end = idx;
                break;
            }
            '(' => {
                // function-style syntax – treat as expression instead of variable
                return None;
            }
            ' ' | '\t' => {
                name_end = idx;
                break;
            }
            _ => {}
        }
    }

    let name = trimmed[..name_end].trim_end();
    if name.is_empty() || !looks_like_identifier(name) {
        return None;
    }

    let mut sections = Vec::new();
    let mut rest = trimmed[name_end..].trim_start();

    while rest.starts_with('[') {
        let (inside, remainder) = extract_enclosed(rest, '[', ']')?;
        sections.push(parse_c_array_section(inside, config));
        rest = remainder.trim_start();
    }

    if sections.is_empty() {
        None
    } else {
        Some(Variable::with_sections(name, sections))
    }
}

fn parse_fortran_variable(raw: &str, config: &ParserConfig) -> Option<Variable> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut name_end = trimmed.len();
    for (idx, ch) in trimmed.char_indices() {
        match ch {
            '(' => {
                name_end = idx;
                break;
            }
            ' ' | '\t' => {
                name_end = idx;
                break;
            }
            _ => {}
        }
    }

    let name = trimmed[..name_end].trim_end();
    if name.is_empty() || !looks_like_identifier(name) {
        return None;
    }

    let mut sections = Vec::new();
    let rest = trimmed[name_end..].trim_start();

    if rest.starts_with('(') {
        let (inside, _) = extract_enclosed(rest, '(', ')')?;
        for dim in split_top_level(inside, ',') {
            sections.push(parse_fortran_section(dim.trim(), config));
        }
    }

    if sections.is_empty() {
        None
    } else {
        Some(Variable::with_sections(name, sections))
    }
}

fn parse_c_array_section(content: &str, config: &ParserConfig) -> ArraySection {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return ArraySection::all();
    }

    let parts = split_top_level(trimmed, ':');
    match parts.len() {
        0 => ArraySection::all(),
        1 => ArraySection::single_index(Expression::new(trimmed, config)),
        2 => ArraySection::new(
            parse_optional_expression(parts[0], config),
            parse_optional_expression(parts[1], config),
            None,
        ),
        3 => ArraySection::new(
            parse_optional_expression(parts[0], config),
            parse_optional_expression(parts[1], config),
            parse_optional_expression(parts[2], config),
        ),
        _ => ArraySection::new(None, Some(Expression::new(trimmed, config)), None),
    }
}

fn parse_fortran_section(content: &str, config: &ParserConfig) -> ArraySection {
    let trimmed = content.trim();
    if trimmed.is_empty() || trimmed == ":" {
        return ArraySection::all();
    }

    let parts = split_top_level(trimmed, ':');
    match parts.len() {
        0 => ArraySection::all(),
        1 => ArraySection::single_index(Expression::new(trimmed, config)),
        2 => ArraySection::new(
            parse_optional_expression(parts[0], config),
            parse_optional_expression(parts[1], config),
            None,
        ),
        3 => ArraySection::new(
            parse_optional_expression(parts[0], config),
            parse_optional_expression(parts[1], config),
            parse_optional_expression(parts[2], config),
        ),
        _ => ArraySection::new(None, Some(Expression::new(trimmed, config)), None),
    }
}

fn parse_optional_expression(segment: &str, config: &ParserConfig) -> Option<Expression> {
    let trimmed = segment.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(Expression::new(trimmed, config))
    }
}

fn looks_like_identifier(raw: &str) -> bool {
    if raw.is_empty() {
        return false;
    }

    if raw.chars().any(char::is_whitespace) {
        return false;
    }

    let mut rest = raw;

    // Allow global C++ scope operator (::identifier)
    if rest.starts_with("::") {
        rest = &rest[2..];
    }

    // Replace namespace/derived-type separators with dots and validate each segment
    let normalized = rest.replace("::", ".").replace('%', ".").replace("->", ".");

    normalized
        .split('.')
        .filter(|segment| !segment.is_empty())
        .all(is_identifier_segment)
}

fn is_identifier_segment(segment: &str) -> bool {
    let mut chars = segment.chars();
    match chars.next() {
        Some(ch) if ch.is_alphabetic() || ch == '_' => {
            chars.all(|c| c.is_alphanumeric() || c == '_')
        }
        _ => false,
    }
}

fn extract_enclosed<'a>(input: &'a str, open: char, close: char) -> Option<(&'a str, &'a str)> {
    let mut depth = 0;
    let mut start = None;

    for (idx, ch) in input.char_indices() {
        if idx == 0 && ch != open {
            return None;
        }

        if ch == open {
            depth += 1;
            if depth == 1 {
                start = Some(idx + ch.len_utf8());
            }
        } else if ch == close {
            depth -= 1;
            if depth == 0 {
                let start_idx = start?;
                let inside = &input[start_idx..idx];
                let remainder = &input[idx + ch.len_utf8()..];
                return Some((inside, remainder));
            }
        }
    }

    None
}

fn split_top_level(input: &str, delimiter: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut depth_paren: usize = 0;
    let mut depth_bracket: usize = 0;
    let mut depth_brace: usize = 0;

    let mut iter = input.char_indices().peekable();
    while let Some((idx, ch)) = iter.next() {
        match ch {
            '(' => depth_paren += 1,
            ')' => depth_paren = depth_paren.saturating_sub(1),
            '[' => depth_bracket += 1,
            ']' => depth_bracket = depth_bracket.saturating_sub(1),
            '{' => depth_brace += 1,
            '}' => depth_brace = depth_brace.saturating_sub(1),
            _ => {}
        }

        if ch == delimiter && depth_paren == 0 && depth_bracket == 0 && depth_brace == 0 {
            parts.push(&input[start..idx]);
            start = idx + ch.len_utf8();
        }
    }

    parts.push(&input[start..]);
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_c_array_sections() {
        let config = ParserConfig::with_parsing(Language::C);
        let items = parse_clause_item_list("arr[0:N], value", &config);

        assert_eq!(items.len(), 2);
        match &items[0] {
            ClauseItem::Variable(var) => {
                assert_eq!(var.name(), "arr");
                assert_eq!(var.array_sections.len(), 1);
                let section = &var.array_sections[0];
                assert!(section.lower_bound.is_some());
                assert!(section.length.is_some());
            }
            other => panic!("Expected variable, got {:?}", other),
        }
    }

    #[test]
    fn parses_fortran_array_sections() {
        let config = ParserConfig::with_parsing(Language::Fortran);
        let items = parse_clause_item_list("array(1:n:2), scalar", &config);

        assert_eq!(items.len(), 2);
        match &items[0] {
            ClauseItem::Variable(var) => {
                assert_eq!(var.name(), "array");
                assert_eq!(var.array_sections.len(), 1);
                let section = &var.array_sections[0];
                assert!(section.lower_bound.is_some());
                assert!(section.length.is_some());
                assert!(section.stride.is_some());
            }
            other => panic!("Expected Fortran variable, got {:?}", other),
        }
    }

    #[test]
    fn falls_back_to_identifiers_when_disabled() {
        let config = ParserConfig::with_parsing(Language::C).with_language_support(false);
        let items = parse_clause_item_list("arr[0:N]", &config);

        assert_eq!(items.len(), 1);
        assert!(matches!(items[0], ClauseItem::Identifier(_)));
    }
}
