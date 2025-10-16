//! Language-aware parsing helpers for clause item lists.
//!
//! This module isolates the small amount of C/C++/Fortran syntax
//! understanding that is required to fully parse OpenMP pragmas.  The
//! rest of the crate continues to treat clause contents as strings.
//!
//! Parsing is intentionally pragmatic: it recognises identifiers and
//! OpenMP array section syntax for the supported host languages and
//! converts them into structured IR (`ClauseItem::Variable`).  Complex
//! expressions are preserved as raw strings inside [`Expression`]
//! nodes so that downstream compilers can perform language-specific
//! analysis.

use std::fmt;

use super::{ArraySection, ClauseItem, Expression, Identifier, Language, ParserConfig, Variable};
use crate::lexer;

/// Error returned when language-aware parsing fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageParseError {
    message: String,
}

impl LanguageParseError {
    /// Create a new parse error with the provided message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Borrow the error message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for LanguageParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for LanguageParseError {}

/// Parse a comma separated list of clause items for the configured language.
#[cfg(feature = "language-parsing")]
pub fn parse_clause_items(
    content: &str,
    config: &ParserConfig,
) -> Result<Vec<ClauseItem>, LanguageParseError> {
    let collapsed = lexer::collapse_line_continuations(content);
    let normalized = collapsed.trim();

    if normalized.is_empty() {
        return Ok(Vec::new());
    }

    let language = normalize_language(config.language);
    let expression_config = ParserConfig {
        parse_expressions: config.parse_expressions,
        language,
    };

    let mut items = Vec::new();
    for item in split_top_level(normalized, ',') {
        if item.is_empty() {
            continue;
        }

        let clause_item = match language {
            Language::C | Language::Cpp => parse_cxx_item(&item, &expression_config)?,
            Language::Fortran => parse_fortran_item(&item, &expression_config)?,
            Language::Unknown => unreachable!("language normalization ensures known host language"),
        };
        items.push(clause_item);
    }

    Ok(items)
}

/// Fallback parser used when the `language-parsing` feature is disabled.
#[cfg(not(feature = "language-parsing"))]
pub fn parse_clause_items(
    content: &str,
    _config: &ParserConfig,
) -> Result<Vec<ClauseItem>, LanguageParseError> {
    let collapsed = lexer::collapse_line_continuations(content);
    let normalized = collapsed.trim();

    if normalized.is_empty() {
        return Ok(Vec::new());
    }

    let items = normalized
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| ClauseItem::Identifier(Identifier::new(s)))
        .collect();

    Ok(items)
}

#[cfg(feature = "language-parsing")]
fn parse_cxx_item(item: &str, config: &ParserConfig) -> Result<ClauseItem, LanguageParseError> {
    let trimmed = item.trim();
    if trimmed.is_empty() {
        return Err(LanguageParseError::new("empty clause item"));
    }

    let mut name_end = trimmed.len();
    if let Some(idx) = trimmed.find('[') {
        name_end = idx;
    }

    let name = trimmed[..name_end].trim();
    if name.is_empty() {
        return Err(LanguageParseError::new(
            "expected identifier before array section",
        ));
    }

    let mut sections = Vec::new();
    let mut rest = &trimmed[name_end..];
    loop {
        let trimmed_rest = rest.trim_start();
        if !trimmed_rest.starts_with('[') {
            break;
        }

        let (section_body, after) = extract_group(trimmed_rest, '[', ']')?;
        sections.push(parse_section_parts(&section_body, config)?);
        rest = after;
    }

    if !rest.trim().is_empty() {
        return Err(LanguageParseError::new(
            "unexpected tokens after array section",
        ));
    }

    if sections.is_empty() {
        Ok(ClauseItem::Identifier(Identifier::new(name)))
    } else {
        Ok(ClauseItem::Variable(Variable::with_sections(
            name, sections,
        )))
    }
}

#[cfg(feature = "language-parsing")]
fn parse_fortran_item(item: &str, config: &ParserConfig) -> Result<ClauseItem, LanguageParseError> {
    let trimmed = item.trim();
    if trimmed.is_empty() {
        return Err(LanguageParseError::new("empty clause item"));
    }

    let mut chars = trimmed.chars();
    let mut name_end = trimmed.len();
    while let Some(ch) = chars.next() {
        if ch == '(' {
            name_end = trimmed.len() - chars.as_str().len() - ch.len_utf8();
            break;
        }
    }

    if name_end == trimmed.len() {
        return Ok(ClauseItem::Identifier(Identifier::new(trimmed)));
    }

    let name = trimmed[..name_end].trim();
    if name.is_empty() {
        return Err(LanguageParseError::new(
            "expected identifier before Fortran array section",
        ));
    }

    let (_, rest) = trimmed.split_at(name_end);
    let (section_body, after) = extract_group(rest.trim_start(), '(', ')')?;
    if !after.trim().is_empty() {
        return Err(LanguageParseError::new(
            "unexpected tokens after array reference",
        ));
    }

    let mut sections = Vec::new();
    for dim in split_top_level(section_body.trim(), ',') {
        if dim.is_empty() {
            continue;
        }
        sections.push(parse_fortran_section(&dim, config)?);
    }

    if sections.is_empty() {
        Ok(ClauseItem::Identifier(Identifier::new(name)))
    } else {
        Ok(ClauseItem::Variable(Variable::with_sections(
            name, sections,
        )))
    }
}

#[cfg(feature = "language-parsing")]
fn parse_section_parts(
    section: &str,
    config: &ParserConfig,
) -> Result<ArraySection, LanguageParseError> {
    let trimmed = section.trim();
    if trimmed.is_empty() {
        return Ok(ArraySection::all());
    }

    let parts: Vec<&str> = trimmed.split(':').collect();

    match parts.len() {
        1 => {
            let expr = Expression::new(parts[0].trim(), config);
            Ok(ArraySection::single_index(expr))
        }
        2 => {
            let lower = if parts[0].trim().is_empty() {
                None
            } else {
                Some(Expression::new(parts[0].trim(), config))
            };
            let length = if parts[1].trim().is_empty() {
                None
            } else {
                Some(Expression::new(parts[1].trim(), config))
            };
            Ok(ArraySection::new(lower, length, None))
        }
        3 => {
            let lower = if parts[0].trim().is_empty() {
                None
            } else {
                Some(Expression::new(parts[0].trim(), config))
            };
            let length = if parts[1].trim().is_empty() {
                None
            } else {
                Some(Expression::new(parts[1].trim(), config))
            };
            let stride = if parts[2].trim().is_empty() {
                None
            } else {
                Some(Expression::new(parts[2].trim(), config))
            };
            Ok(ArraySection::new(lower, length, stride))
        }
        _ => Err(LanguageParseError::new("too many ':' in array section")),
    }
}

#[cfg(feature = "language-parsing")]
fn parse_fortran_section(
    section: &str,
    config: &ParserConfig,
) -> Result<ArraySection, LanguageParseError> {
    let trimmed = section.trim();
    if trimmed.is_empty() {
        return Ok(ArraySection::all());
    }

    let parts: Vec<&str> = trimmed.split(':').collect();
    match parts.len() {
        1 => {
            let expr = Expression::new(parts[0].trim(), config);
            Ok(ArraySection::single_index(expr))
        }
        2 => {
            let lower = if parts[0].trim().is_empty() {
                None
            } else {
                Some(Expression::new(parts[0].trim(), config))
            };
            let upper = if parts[1].trim().is_empty() {
                None
            } else {
                Some(Expression::new(parts[1].trim(), config))
            };
            Ok(ArraySection::new(lower, upper, None))
        }
        3 => {
            let lower = if parts[0].trim().is_empty() {
                None
            } else {
                Some(Expression::new(parts[0].trim(), config))
            };
            let upper = if parts[1].trim().is_empty() {
                None
            } else {
                Some(Expression::new(parts[1].trim(), config))
            };
            let stride = if parts[2].trim().is_empty() {
                None
            } else {
                Some(Expression::new(parts[2].trim(), config))
            };
            Ok(ArraySection::new(lower, upper, stride))
        }
        _ => Err(LanguageParseError::new("too many ':' in Fortran selector")),
    }
}

#[cfg(feature = "language-parsing")]
fn extract_group(
    input: &str,
    open: char,
    close: char,
) -> Result<(String, &str), LanguageParseError> {
    let mut chars = input.char_indices();
    match chars.next() {
        Some((_, ch)) if ch == open => {}
        _ => {
            return Err(LanguageParseError::new(
                "expected group to start with delimiter",
            ));
        }
    }

    let mut depth = 1usize;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    while let Some((idx, ch)) = chars.next() {
        match ch {
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            _ if in_single_quote || in_double_quote => {}
            _ if ch == open => {
                depth += 1;
            }
            _ if ch == close => {
                depth -= 1;
                if depth == 0 {
                    let content = input[1..idx].to_string();
                    let rest = &input[idx + ch.len_utf8()..];
                    return Ok((content, rest));
                }
            }
            _ => {}
        }
    }

    Err(LanguageParseError::new("unterminated delimited group"))
}

#[cfg(feature = "language-parsing")]
fn split_top_level(input: &str, delimiter: char) -> Vec<String> {
    let mut items = Vec::new();
    let mut current = String::new();
    let mut bracket_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut angle_depth = 0usize;
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    let chars: Vec<char> = input.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        let ch = chars[i];
        let prev = if i > 0 { Some(chars[i - 1]) } else { None };
        let next = chars.get(i + 1).copied();

        match ch {
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            _ if in_single_quote || in_double_quote => {}
            '[' => {
                bracket_depth += 1;
            }
            ']' => {
                if bracket_depth > 0 {
                    bracket_depth -= 1;
                }
            }
            '(' => {
                paren_depth += 1;
            }
            ')' => {
                if paren_depth > 0 {
                    paren_depth -= 1;
                }
            }
            '<' => {
                if bracket_depth == 0 && paren_depth == 0 {
                    if is_template_start(prev, next) {
                        angle_depth += 1;
                    }
                }
            }
            '>' => {
                if angle_depth > 0
                    && bracket_depth == 0
                    && paren_depth == 0
                    && !matches!(prev, Some('>'))
                    && !matches!(next, Some('>') | Some('='))
                {
                    angle_depth -= 1;
                }
            }
            _ => {}
        }

        if ch == delimiter
            && bracket_depth == 0
            && paren_depth == 0
            && angle_depth == 0
            && !in_single_quote
            && !in_double_quote
        {
            let trimmed = current.trim();
            if !trimmed.is_empty() {
                items.push(trimmed.to_string());
            }
            current.clear();
        } else {
            current.push(ch);
        }

        i += 1;
    }

    let trimmed = current.trim();
    if !trimmed.is_empty() {
        items.push(trimmed.to_string());
    }

    items
}

#[cfg(feature = "language-parsing")]
fn is_template_start(prev: Option<char>, next: Option<char>) -> bool {
    let is_prev_valid =
        matches!(prev, Some(c) if c.is_alphanumeric() || c == '_' || c == ':' || c == '>');
    let is_next_disqualifier = matches!(next, Some('=') | Some('<') | Some('>'));
    is_prev_valid && !is_next_disqualifier
}

fn normalize_language(language: Language) -> Language {
    match language {
        Language::Unknown => Language::C,
        other => other,
    }
}

#[cfg(all(test, feature = "language-parsing"))]
mod tests {
    use super::*;

    fn config(language: Language) -> ParserConfig {
        ParserConfig {
            parse_expressions: true,
            language,
        }
    }

    #[test]
    fn parses_simple_c_identifiers() {
        let items = parse_clause_items("x, y, z", &config(Language::C)).unwrap();
        assert_eq!(items.len(), 3);
        assert!(matches!(items[0], ClauseItem::Identifier(_)));
    }

    #[test]
    fn parses_c_array_sections() {
        let items = parse_clause_items("arr[0:N], matrix[i][j:k]", &config(Language::C)).unwrap();
        assert_eq!(items.len(), 2);
        assert!(matches!(items[0], ClauseItem::Variable(_)));
        assert!(matches!(items[1], ClauseItem::Variable(_)));
    }

    #[test]
    fn parses_cpp_templates_without_splitting() {
        let items =
            parse_clause_items("std::array<int, 4>[idx], data", &config(Language::Cpp)).unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn parses_fortran_array_sections() {
        let items = parse_clause_items("a(1:n, :), b(:), c", &config(Language::Fortran)).unwrap();
        assert_eq!(items.len(), 3);
        assert!(matches!(items[0], ClauseItem::Variable(_)));
        assert!(matches!(items[1], ClauseItem::Variable(_)));
        assert!(matches!(items[2], ClauseItem::Identifier(_)));
    }

    #[test]
    fn rejects_unmatched_brackets() {
        let err = parse_clause_items("arr[0:N", &config(Language::C)).unwrap_err();
        assert!(err.message().contains("unterminated"));
    }
}
