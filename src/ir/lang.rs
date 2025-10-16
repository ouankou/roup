#![cfg(feature = "language_frontends")]

use std::fmt;

use super::{ArraySection, ClauseItem, Expression, Identifier, Language, ParserConfig, Variable};

#[derive(Debug)]
pub enum ClauseItemParseError {
    UnbalancedDelimiter(char),
    TooManySectionParts(String),
}

impl fmt::Display for ClauseItemParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClauseItemParseError::UnbalancedDelimiter(c) => {
                write!(f, "unbalanced delimiter '{}'", c)
            }
            ClauseItemParseError::TooManySectionParts(section) => {
                write!(f, "too many parts in array section '{}'", section)
            }
        }
    }
}

impl std::error::Error for ClauseItemParseError {}

/// Parse a comma separated list of clause items respecting nested delimiters.
pub fn parse_clause_item_list(
    content: &str,
    config: &ParserConfig,
) -> Result<Vec<ClauseItem>, ClauseItemParseError> {
    let mut items = Vec::new();

    for raw_item in split_top_level(content, ',') {
        let item = raw_item.trim();
        if item.is_empty() {
            continue;
        }

        items.push(parse_clause_item(item, config)?);
    }

    Ok(items)
}

/// Split the input once at the first top-level occurrence of `delimiter`.
pub fn split_once_top_level(input: &str, delimiter: char) -> Option<(&str, &str)> {
    find_top_level_delimiter(input, delimiter, false).map(|idx| {
        let next = idx + delimiter.len_utf8();
        (&input[..idx], &input[next..])
    })
}

/// Split the input once at the last top-level occurrence of `delimiter`.
pub fn rsplit_once_top_level(input: &str, delimiter: char) -> Option<(&str, &str)> {
    find_top_level_delimiter(input, delimiter, true).map(|idx| {
        let next = idx + delimiter.len_utf8();
        (&input[..idx], &input[next..])
    })
}

/// Split the input by a delimiter while respecting nested parentheses/brackets/braces.
pub fn split_top_level<'a>(input: &'a str, delimiter: char) -> Vec<&'a str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut depth_paren = 0i32;
    let mut depth_bracket = 0i32;
    let mut depth_brace = 0i32;
    let mut ternary_depth = 0i32;

    for (idx, ch) in input.char_indices() {
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
            ':' if delimiter == ':' => {
                if ternary_depth > 0 {
                    ternary_depth -= 1;
                    continue;
                }
            }
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

fn parse_clause_item(
    item: &str,
    config: &ParserConfig,
) -> Result<ClauseItem, ClauseItemParseError> {
    match parse_variable_candidate(item, config)? {
        Some(VariableCandidate { name, sections }) if sections.is_empty() => {
            Ok(ClauseItem::Identifier(Identifier::new(name)))
        }
        Some(VariableCandidate { name, sections }) => Ok(ClauseItem::Variable(
            Variable::with_sections(name, sections),
        )),
        None => {
            if looks_like_identifier(item, config.language) {
                Ok(ClauseItem::Identifier(Identifier::new(item)))
            } else {
                Ok(ClauseItem::Expression(Expression::new(item, config)))
            }
        }
    }
}

struct VariableCandidate {
    name: String,
    sections: Vec<ArraySection>,
}

fn parse_variable_candidate(
    item: &str,
    config: &ParserConfig,
) -> Result<Option<VariableCandidate>, ClauseItemParseError> {
    let language = match config.language {
        Language::Unknown => Language::C,
        other => other,
    };

    match language {
        Language::C | Language::Cpp => parse_c_variable(item, config),
        Language::Fortran => parse_fortran_variable(item, config),
        Language::Unknown => parse_c_variable(item, config),
    }
}

fn parse_c_variable(
    item: &str,
    config: &ParserConfig,
) -> Result<Option<VariableCandidate>, ClauseItemParseError> {
    let trimmed = item.trim();
    let mut chars = trimmed.char_indices();
    let (mut end, mut has_chars) = (0usize, false);

    while let Some((idx, ch)) = chars.next() {
        if !has_chars {
            if !is_c_identifier_start(ch) {
                return Ok(None);
            }
            has_chars = true;
        } else if !is_c_identifier_continue(ch) {
            end = idx;
            break;
        }

        end = idx + ch.len_utf8();
    }

    if !has_chars {
        return Ok(None);
    }

    let name = trimmed[..end].trim().to_string();
    let mut sections = Vec::new();
    let mut rest = trimmed[end..].trim_start();

    while let Some(after_open) = rest.strip_prefix('[') {
        let (inside, after) = extract_delimited(after_open, '[', ']')?;
        let section = parse_section_spec(inside, config)?;
        sections.push(section);
        rest = after.trim_start();
    }

    if rest.is_empty() {
        Ok(Some(VariableCandidate { name, sections }))
    } else {
        Ok(None)
    }
}

fn parse_fortran_variable(
    item: &str,
    config: &ParserConfig,
) -> Result<Option<VariableCandidate>, ClauseItemParseError> {
    let trimmed = item.trim();
    let mut chars = trimmed.char_indices();
    let (mut end, mut has_chars) = (0usize, false);

    while let Some((idx, ch)) = chars.next() {
        if !has_chars {
            if !is_fortran_identifier_start(ch) {
                return Ok(None);
            }
            has_chars = true;
        } else if !is_fortran_identifier_continue(ch) {
            end = idx;
            break;
        }

        end = idx + ch.len_utf8();
    }

    if !has_chars {
        return Ok(None);
    }

    let name = trimmed[..end].trim().to_string();
    let mut sections = Vec::new();
    let mut rest = trimmed[end..].trim_start();

    while let Some(after_open) = rest.strip_prefix('(') {
        let (inside, after) = extract_delimited(after_open, '(', ')')?;

        for dim in split_top_level(inside, ',') {
            let spec = dim.trim();
            if spec.is_empty() {
                sections.push(ArraySection::all());
            } else {
                sections.push(parse_section_spec(spec, config)?);
            }
        }

        rest = after.trim_start();
    }

    if rest.is_empty() {
        Ok(Some(VariableCandidate { name, sections }))
    } else {
        Ok(None)
    }
}

fn parse_section_spec(
    spec: &str,
    config: &ParserConfig,
) -> Result<ArraySection, ClauseItemParseError> {
    let trimmed = spec.trim();

    if trimmed.is_empty() {
        return Ok(ArraySection::all());
    }

    let parts = split_top_level(trimmed, ':');

    match parts.len() {
        1 => {
            let expr = Expression::new(parts[0].trim(), config);
            Ok(ArraySection::single_index(expr))
        }
        2 => {
            let lower = parts[0].trim();
            let length = parts[1].trim();

            let lower_expr = if lower.is_empty() {
                None
            } else {
                Some(Expression::new(lower, config))
            };

            let length_expr = if length.is_empty() {
                None
            } else {
                Some(Expression::new(length, config))
            };

            Ok(ArraySection::new(lower_expr, length_expr, None))
        }
        3 => {
            let lower = parts[0].trim();
            let length = parts[1].trim();
            let stride = parts[2].trim();

            let lower_expr = if lower.is_empty() {
                None
            } else {
                Some(Expression::new(lower, config))
            };

            let length_expr = if length.is_empty() {
                None
            } else {
                Some(Expression::new(length, config))
            };

            let stride_expr = if stride.is_empty() {
                None
            } else {
                Some(Expression::new(stride, config))
            };

            Ok(ArraySection::new(lower_expr, length_expr, stride_expr))
        }
        _ => Err(ClauseItemParseError::TooManySectionParts(
            trimmed.to_string(),
        )),
    }
}

fn extract_delimited<'a>(
    input: &'a str,
    open: char,
    close: char,
) -> Result<(&'a str, &'a str), ClauseItemParseError> {
    let mut depth = 1i32;

    for (idx, ch) in input.char_indices() {
        match ch {
            c if c == open => depth += 1,
            c if c == close => {
                depth -= 1;
                if depth == 0 {
                    let inside = &input[..idx];
                    let rest = &input[idx + ch.len_utf8()..];
                    return Ok((inside, rest));
                }
            }
            _ => {}
        }
    }

    Err(ClauseItemParseError::UnbalancedDelimiter(close))
}

fn looks_like_identifier(input: &str, language: Language) -> bool {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return false;
    }

    let mut chars = trimmed.chars();
    let first = match chars.next() {
        Some(ch) => ch,
        None => return false,
    };

    match language {
        Language::Fortran => {
            if !is_fortran_identifier_start(first) {
                return false;
            }

            chars.all(is_fortran_identifier_continue)
        }
        _ => {
            if !is_c_identifier_start(first) {
                return false;
            }

            chars.all(is_c_identifier_continue)
        }
    }
}

fn is_c_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_c_identifier_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn is_fortran_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_fortran_identifier_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn find_top_level_delimiter(input: &str, delimiter: char, from_end: bool) -> Option<usize> {
    let mut depth_paren = 0i32;
    let mut depth_bracket = 0i32;
    let mut depth_brace = 0i32;
    let mut ternary_depth = 0i32;
    let mut result: Option<usize> = None;

    for (idx, ch) in input.char_indices() {
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
            ':' if delimiter == ':' => {
                if ternary_depth > 0 {
                    ternary_depth -= 1;
                    continue;
                }
            }
            _ => {}
        }

        if ch == delimiter && depth_paren == 0 && depth_bracket == 0 && depth_brace == 0 {
            result = Some(idx);
            if !from_end {
                break;
            }
        }
    }

    if from_end {
        result
    } else {
        result
    }
}
