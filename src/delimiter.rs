//! Shared, strict delimiter scanning for directive payloads.
//!
//! Delimiters that occur inside host-language literals or comments are never
//! structural directive syntax.  Keeping that rule here prevents the clause,
//! semantic, and directive parsers from gradually acquiring incompatible
//! quote/comment workarounds.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CommentStyle {
    /// Recognize block comments only.  This is the unambiguous common subset
    /// used after a directive's outer parentheses have already been bounded.
    Block,
    /// Recognize C/C++ block comments and `//` line comments.
    CLike,
    /// Recognize Fortran `!` line comments.  `//` remains the concatenation
    /// operator in this mode.
    Fortran,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DelimiterError {
    EmptyEntry {
        separator: char,
    },
    UnterminatedQuote {
        quote: char,
        offset: usize,
    },
    UnterminatedBlockComment {
        offset: usize,
    },
    UnmatchedClosing {
        delimiter: char,
        offset: usize,
    },
    MismatchedClosing {
        opening: char,
        expected: char,
        actual: char,
        offset: usize,
    },
    UnclosedDelimiter {
        opening: char,
        expected: char,
    },
}

impl fmt::Display for DelimiterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyEntry { separator } => {
                write!(
                    formatter,
                    "list separated by '{separator}' contains an empty entry"
                )
            }
            Self::UnterminatedQuote { quote, offset } => {
                write!(formatter, "unclosed {quote} quote at byte {offset}")
            }
            Self::UnterminatedBlockComment { offset } => {
                write!(formatter, "unclosed block comment at byte {offset}")
            }
            Self::UnmatchedClosing { delimiter, offset } => {
                write!(
                    formatter,
                    "unmatched closing delimiter '{delimiter}' at byte {offset}"
                )
            }
            Self::MismatchedClosing {
                opening,
                expected,
                actual,
                offset,
            } => write!(
                formatter,
                "mismatched delimiter '{actual}' at byte {offset}: '{opening}' must be closed by '{expected}'"
            ),
            Self::UnclosedDelimiter { opening, expected } => write!(
                formatter,
                "unclosed delimiter '{opening}', expected '{expected}'"
            ),
        }
    }
}

/// Return byte-indexed source characters that are outside literals/comments.
///
/// Backslash escapes are accepted in both quote forms for C/C++, and doubled
/// delimiters are accepted in both quote forms for Fortran.  Supporting both
/// escape mechanisms here is conservative: the typed host lexer still decides
/// whether the literal itself is valid for the configured language profile.
fn code_chars(
    input: &str,
    comment_style: CommentStyle,
) -> Result<Vec<(usize, char)>, DelimiterError> {
    let mut result = Vec::with_capacity(input.chars().count());
    let mut chars = input.char_indices().peekable();

    while let Some((index, character)) = chars.next() {
        if matches!(character, '\'' | '"') {
            let opening_offset = index;
            let quote = character;
            let mut closed = false;
            while let Some((_, quoted)) = chars.next() {
                if quoted == '\\' {
                    // A terminal escape cannot close the literal.  Let the
                    // common unterminated-literal error below report it.
                    if chars.next().is_none() {
                        break;
                    }
                    continue;
                }
                if quoted == quote {
                    if chars.peek().is_some_and(|(_, next)| *next == quote) {
                        chars.next();
                    } else {
                        closed = true;
                        break;
                    }
                }
            }
            if !closed {
                return Err(DelimiterError::UnterminatedQuote {
                    quote,
                    offset: opening_offset,
                });
            }
            continue;
        }

        if character == '/' && chars.peek().is_some_and(|(_, next)| *next == '*') {
            let opening_offset = index;
            chars.next();
            let mut closed = false;
            while let Some((_, comment_character)) = chars.next() {
                if comment_character == '*' && chars.peek().is_some_and(|(_, next)| *next == '/') {
                    chars.next();
                    closed = true;
                    break;
                }
            }
            if !closed {
                return Err(DelimiterError::UnterminatedBlockComment {
                    offset: opening_offset,
                });
            }
            continue;
        }

        if comment_style == CommentStyle::CLike
            && character == '/'
            && chars.peek().is_some_and(|(_, next)| *next == '/')
        {
            chars.next();
            for (_, comment_character) in chars.by_ref() {
                if matches!(comment_character, '\n' | '\r') {
                    break;
                }
            }
            continue;
        }

        if comment_style == CommentStyle::Fortran
            && character == '!'
            // A continued directive can still contain its next `!$omp` or
            // `!$acc` sentinel when the low-level parser is exercised
            // directly instead of through `LogicalSource`.
            && chars.peek().is_none_or(|(_, next)| *next != '$')
        {
            for (_, comment_character) in chars.by_ref() {
                if matches!(comment_character, '\n' | '\r') {
                    break;
                }
            }
            continue;
        }

        result.push((index, character));
    }

    Ok(result)
}

pub(crate) fn split_top_level<'a>(
    input: &'a str,
    separator: char,
    pairs: &[(char, char)],
    comment_style: CommentStyle,
) -> Result<Vec<&'a str>, DelimiterError> {
    let characters = code_chars(input, comment_style)?;
    let mut segments = Vec::new();
    let mut start = 0;
    let mut stack: Vec<(char, char)> = Vec::new();
    let mut ternary_depth = 0usize;
    let track_angles = pairs.contains(&('<', '>'));

    for (position, &(index, character)) in characters.iter().enumerate() {
        let previous = characters[..position]
            .iter()
            .rev()
            .map(|(_, character)| *character)
            .find(|character| !character.is_whitespace());
        let next = characters[position + 1..]
            .iter()
            .map(|(_, character)| *character)
            .find(|character| !character.is_whitespace());

        if track_angles && character == '<' {
            if stack.last().is_none_or(|(_, close)| *close == '>')
                && is_template_start(previous, next)
            {
                stack.push(('<', '>'));
            }
        } else if track_angles && character == '>' {
            if stack.last().is_some_and(|(_, close)| *close == '>') && next != Some('=') {
                stack.pop();
            } else if stack.is_empty() {
                return Err(DelimiterError::UnmatchedClosing {
                    delimiter: character,
                    offset: index,
                });
            }
        } else if let Some(&(opening, closing)) = pairs
            .iter()
            .find(|(opening, _)| *opening == character && *opening != '<')
        {
            stack.push((opening, closing));
        } else if pairs
            .iter()
            .any(|(opening, closing)| *closing == character && *opening != '<')
        {
            pop_delimiter(&mut stack, character, index)?;
        }

        if separator == ':' && stack.is_empty() {
            match character {
                '?' => {
                    ternary_depth += 1;
                    continue;
                }
                ':' if ternary_depth > 0 => {
                    ternary_depth -= 1;
                    continue;
                }
                _ => {}
            }
        }

        if character == separator && stack.is_empty() {
            if separator == ':' && (previous == Some(':') || next == Some(':')) {
                continue;
            }
            push_segment(input, separator, start, index, &mut segments)?;
            start = index + character.len_utf8();
        }
    }

    if let Some(&(opening, expected)) = stack.last() {
        return Err(DelimiterError::UnclosedDelimiter { opening, expected });
    }
    push_segment(input, separator, start, input.len(), &mut segments)?;
    Ok(segments)
}

pub(crate) fn find_top_level_delimiter(
    input: &str,
    delimiter: char,
    from_end: bool,
    comment_style: CommentStyle,
) -> Result<Option<usize>, DelimiterError> {
    let characters = code_chars(input, comment_style)?;
    let mut stack: Vec<(char, char)> = Vec::new();
    let mut ternary_depth = 0usize;
    let mut result = None;

    for (position, &(index, character)) in characters.iter().enumerate() {
        let previous = position.checked_sub(1).map(|prior| characters[prior].1);
        let next = characters.get(position + 1).map(|(_, next)| *next);

        match character {
            '(' => stack.push(('(', ')')),
            '[' => stack.push(('[', ']')),
            '{' => stack.push(('{', '}')),
            ')' | ']' | '}' => pop_delimiter(&mut stack, character, index)?,
            '?' if delimiter == ':' && stack.is_empty() => {
                ternary_depth += 1;
                continue;
            }
            ':' if delimiter == ':' && stack.is_empty() && ternary_depth > 0 => {
                ternary_depth -= 1;
                continue;
            }
            _ => {}
        }

        if character == delimiter && stack.is_empty() {
            if delimiter == ':' && (previous == Some(':') || next == Some(':')) {
                continue;
            }
            if from_end || result.is_none() {
                result = Some(index);
            }
        }
    }

    if let Some(&(opening, expected)) = stack.last() {
        return Err(DelimiterError::UnclosedDelimiter { opening, expected });
    }
    Ok(result)
}

/// Find a matching close in a source slice that begins immediately after an
/// already-consumed opening delimiter.
pub(crate) fn find_matching_after_open(
    input: &str,
    opening: char,
    expected: char,
    comment_style: CommentStyle,
) -> Result<Option<usize>, DelimiterError> {
    let characters = code_chars(input, comment_style)?;
    let mut stack = vec![(opening, expected)];

    for &(index, character) in &characters {
        match character {
            '(' => stack.push(('(', ')')),
            '[' => stack.push(('[', ']')),
            '{' => stack.push(('{', '}')),
            ')' | ']' | '}' => {
                pop_delimiter(&mut stack, character, index)?;
                if stack.is_empty() {
                    return Ok(Some(index));
                }
            }
            _ => {}
        }
    }

    Ok(None)
}

pub(crate) fn find_matching_delimiter(
    input: &str,
    open_position: usize,
    opening: char,
    expected: char,
    comment_style: CommentStyle,
) -> Result<Option<usize>, DelimiterError> {
    if input
        .get(open_position..)
        .and_then(|suffix| suffix.chars().next())
        != Some(opening)
    {
        return Ok(None);
    }
    let content_start = open_position + opening.len_utf8();
    find_matching_after_open(&input[content_start..], opening, expected, comment_style)
        .map(|position| position.map(|relative| content_start + relative))
}

fn pop_delimiter(
    stack: &mut Vec<(char, char)>,
    actual: char,
    offset: usize,
) -> Result<(), DelimiterError> {
    match stack.pop() {
        Some((_, expected)) if expected == actual => Ok(()),
        Some((opening, expected)) => Err(DelimiterError::MismatchedClosing {
            opening,
            expected,
            actual,
            offset,
        }),
        None => Err(DelimiterError::UnmatchedClosing {
            delimiter: actual,
            offset,
        }),
    }
}

fn push_segment<'a>(
    input: &'a str,
    separator: char,
    start: usize,
    end: usize,
    segments: &mut Vec<&'a str>,
) -> Result<(), DelimiterError> {
    let item = input[start..end].trim();
    if item.is_empty() {
        return Err(DelimiterError::EmptyEntry { separator });
    }
    segments.push(item);
    Ok(())
}

/// Check if `<` is likely a template start rather than a comparison operator.
fn is_template_start(previous: Option<char>, next: Option<char>) -> bool {
    let previous_can_name_template = matches!(
        previous,
        Some(character)
            if character.is_alphanumeric()
                || character == '_'
                || character == ':'
                || character == '>'
    );
    let next_disqualifies_template = matches!(next, Some('=') | Some('<') | Some('>'));
    previous_can_name_template && !next_disqualifies_template
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shields_quotes_and_comments() {
        assert_eq!(
            split_top_level(
                "a /*,)]}*/, f(\"x,)\"), b",
                ',',
                &[('(', ')'), ('[', ']'), ('{', '}')],
                CommentStyle::CLike,
            )
            .unwrap(),
            ["a /*,)]}*/", "f(\"x,)\")", "b"]
        );
        assert_eq!(
            find_matching_delimiter("{kind(\"c}pu\")}", 0, '{', '}', CommentStyle::Block,).unwrap(),
            Some(13)
        );
    }

    #[test]
    fn recognizes_both_literal_escape_conventions() {
        assert_eq!(
            split_top_level("'a'','',x', y", ',', &[], CommentStyle::Fortran).unwrap(),
            ["'a'','',x'", "y"]
        );
        assert_eq!(
            split_top_level("\"a\\\",x\", y", ',', &[], CommentStyle::CLike).unwrap(),
            ["\"a\\\",x\"", "y"]
        );
    }

    #[test]
    fn template_angles_tolerate_intervening_whitespace_and_comments() {
        let pairs = &[('(', ')'), ('[', ']'), ('<', '>')];
        assert_eq!(
            split_top_level(
                "std::pair /* type */ < int, long >, double",
                ',',
                pairs,
                CommentStyle::CLike,
            )
            .unwrap(),
            ["std::pair /* type */ < int, long >", "double"]
        );
    }

    #[test]
    fn malformed_lexical_and_delimiter_state_is_an_error() {
        for source in ["\"unterminated", "a /* unterminated"] {
            assert!(split_top_level(source, ',', &[], CommentStyle::CLike).is_err());
        }
        assert!(
            split_top_level(
                "a, f(x]",
                ',',
                &[('(', ')'), ('[', ']')],
                CommentStyle::Block,
            )
            .is_err()
        );
    }
}
