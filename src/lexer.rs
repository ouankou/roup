#![forbid(unsafe_code)]

/// Lexer module - tokenizes OpenMP pragma directives
///
/// Learning Rust: Parser Combinators with nom
/// ===========================================
/// nom is a parser combinator library
/// - Parsers are functions that consume input and return results
/// - Combine small parsers to build complex ones
/// - Type: IResult<Input, Output, Error>
///
/// Learning Rust: Type Aliases
/// ============================
/// IResult is nom's result type for parser functions
/// IResult<&str, &str> means:
/// - Input: &str (string slice to parse)
/// - Output: &str (parsed token)
/// - Returns: Ok((remaining_input, parsed_output)) or Err
use std::borrow::Cow;
use std::fmt;

use nom::IResult;

use crate::source::{SourceError, Span};

/// Learning Rust: Importing from External Crates
/// ===============================================
/// nom::bytes::complete::tag - matches exact strings
/// nom::bytes::complete::take_while1 - matches while predicate is true
use nom::bytes::complete::{tag, take_while1};

/// Language format for parsing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    /// C/C++ language with #pragma omp
    C,
    /// Fortran free-form with !$OMP sentinel
    FortranFree,
    /// Fortran fixed-form with !$OMP or C$OMP in columns 1-6
    FortranFixed,
}

/// Validate that every physical line break followed by more directive text is
/// continued according to the selected source form. A final line terminator
/// and trailing whitespace are accepted; a parser must never wander onto an
/// unrelated source line through generic whitespace skipping.
pub(crate) fn validate_logical_line(
    input: &str,
    language: Language,
    fortran_prefix: &str,
) -> Result<(), usize> {
    let mut offset = 0usize;
    while offset < input.len() {
        let relative_newline = input[offset..].find(['\n', '\r']);
        let line_end = relative_newline.map_or(input.len(), |relative| offset + relative);

        if offset != 0 && matches!(language, Language::FortranFree | Language::FortranFixed) {
            let line = &input[offset..line_end];
            let leading = line.len() - line.trim_start_matches([' ', '\t']).len();
            let after_indent = &line[leading..];
            if match_fortran_continuation_sentinel(after_indent).is_some()
                && match_fortran_continuation_sentinel_with_prefix(after_indent, fortran_prefix)
                    .is_none()
            {
                return Err(offset + leading);
            }
        }

        let Some(_) = relative_newline else {
            return Ok(());
        };
        let newline_end = if input.as_bytes()[line_end] == b'\r' {
            if input.as_bytes().get(line_end + 1) != Some(&b'\n') {
                return Err(line_end);
            }
            line_end + 2
        } else {
            line_end + 1
        };

        if input[newline_end..].trim().is_empty() {
            return Ok(());
        }

        let line = &input[offset..line_end];
        let continued = match language {
            Language::C => line.as_bytes().last() == Some(&b'\\'),
            Language::FortranFree | Language::FortranFixed => {
                fortran_line_has_trailing_continuation(line, fortran_prefix)
            }
        };
        if !continued {
            return Err(line_end);
        }
        offset = newline_end;
    }
    Ok(())
}

fn fortran_line_has_trailing_continuation(line: &str, prefix: &str) -> bool {
    let leading = line.len() - line.trim_start_matches([' ', '\t']).len();
    let after_indent = &line[leading..];
    let sentinel_length =
        match_fortran_continuation_sentinel_with_prefix(after_indent, prefix).unwrap_or(0);
    let content = &after_indent[sentinel_length..];

    let mut quote: Option<char> = None;
    let mut chars = content.char_indices().peekable();
    let mut code_end = content.len();

    while let Some((index, character)) = chars.next() {
        if let Some(delimiter) = quote {
            if character == delimiter {
                if chars.peek().is_some_and(|(_, next)| *next == delimiter) {
                    chars.next();
                } else {
                    quote = None;
                }
            }
            continue;
        }

        match character {
            '\'' | '"' => quote = Some(character),
            '!' => {
                code_end = index;
                break;
            }
            _ => {}
        }
    }

    content[..code_end].trim_end().ends_with('&')
}

/// One validated logical directive plus an exact mapping back to its physical
/// source. Parsers consume `text`; AST ranges are converted with `span`.
pub(crate) struct LogicalSource<'a> {
    original: &'a str,
    text: Cow<'a, str>,
    physical_boundaries: Option<Vec<usize>>,
}

impl<'a> LogicalSource<'a> {
    pub(crate) fn new(
        input: &'a str,
        language: Language,
        fortran_prefix: &str,
    ) -> Result<Self, LogicalSourceError> {
        validate_logical_line(input, language, fortran_prefix)
            .map_err(|offset| LogicalSourceError::InvalidContinuation { offset })?;

        let mut output = String::with_capacity(input.len());
        let mut boundaries = vec![0usize];
        let mut offset = 0usize;
        let mut changed = false;

        while offset < input.len() {
            let continuation_end = match language {
                Language::C => skip_c_line_continuation(input, offset),
                Language::FortranFree | Language::FortranFixed => {
                    skip_fortran_continuation(input, offset)
                }
            };
            if let Some(next) = continuation_end {
                changed = true;
                offset = next;
                continue;
            }

            let next = next_char_boundary(input, offset);
            if changed {
                // The one logical boundary on either side of removed source
                // maps to the first byte that remains after the gap. This keeps
                // starts exact and lets a token spanning a splice cover the
                // whole physical spelling.
                if let Some(boundary) = boundaries.last_mut() {
                    *boundary = offset;
                }
            }
            output.push_str(&input[offset..next]);
            for physical in offset + 1..=next {
                boundaries.push(physical);
            }
            offset = next;
        }

        if !changed {
            return Ok(Self {
                original: input,
                text: Cow::Borrowed(input),
                physical_boundaries: None,
            });
        }

        if let Some(boundary) = boundaries.last_mut() {
            *boundary = input.len();
        }
        Ok(Self {
            original: input,
            text: Cow::Owned(output),
            physical_boundaries: Some(boundaries),
        })
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn span(&self, logical: std::ops::Range<usize>) -> Result<Span, LogicalSourceError> {
        if logical.start > logical.end || logical.end > self.text.len() {
            return Err(LogicalSourceError::LogicalRange {
                start: logical.start,
                end: logical.end,
                source_len: self.text.len(),
            });
        }
        if !self.text.is_char_boundary(logical.start) || !self.text.is_char_boundary(logical.end) {
            return Err(LogicalSourceError::LogicalCharBoundary {
                start: logical.start,
                end: logical.end,
            });
        }

        let (start, end) = match &self.physical_boundaries {
            Some(boundaries) => (boundaries[logical.start], boundaries[logical.end]),
            None => (logical.start, logical.end),
        };
        Span::new(self.original, start, end).map_err(LogicalSourceError::PhysicalSpan)
    }

    pub(crate) fn span_of(&self, slice: &str) -> Result<Span, LogicalSourceError> {
        let text_start = self.text.as_ptr() as usize;
        let text_end = text_start
            .checked_add(self.text.len())
            .ok_or(LogicalSourceError::ForeignSlice)?;
        let slice_start = slice.as_ptr() as usize;
        let slice_end = slice_start
            .checked_add(slice.len())
            .ok_or(LogicalSourceError::ForeignSlice)?;
        if slice_start < text_start || slice_end > text_end {
            return Err(LogicalSourceError::ForeignSlice);
        }
        self.span((slice_start - text_start)..(slice_end - text_start))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LogicalSourceError {
    InvalidContinuation {
        offset: usize,
    },
    LogicalRange {
        start: usize,
        end: usize,
        source_len: usize,
    },
    LogicalCharBoundary {
        start: usize,
        end: usize,
    },
    ForeignSlice,
    PhysicalSpan(SourceError),
}

impl fmt::Display for LogicalSourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidContinuation { offset } => {
                write!(formatter, "physical line at byte {offset} is not continued")
            }
            Self::LogicalRange {
                start,
                end,
                source_len,
            } => write!(
                formatter,
                "logical range {start}..{end} is invalid for {source_len} bytes"
            ),
            Self::LogicalCharBoundary { start, end } => write!(
                formatter,
                "logical range {start}..{end} splits a UTF-8 code point"
            ),
            Self::ForeignSlice => {
                formatter.write_str("source slice does not belong to the logical directive")
            }
            Self::PhysicalSpan(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for LogicalSourceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::PhysicalSpan(error) => Some(error),
            _ => None,
        }
    }
}

/// Check if a character is valid in an identifier
///
/// Learning Rust: Closures and Function Pointers
/// ==============================================
/// This function can be used as a predicate
/// take_while1 accepts: fn(char) -> bool
pub fn is_identifier_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Normalize Fortran identifier to lowercase for case-insensitive matching
#[cfg(test)]
pub fn normalize_fortran_identifier(s: &str) -> String {
    s.to_lowercase()
}

/// Parse "#pragma" keyword
///
/// Learning Rust: Parser Combinators Basics
/// =========================================
/// tag("string") returns a parser function
/// The parser succeeds if input starts with "string"
/// Returns: Ok((remaining, matched)) or Err
pub fn lex_pragma(input: &str) -> IResult<&str, &str> {
    tag("#pragma")(input)
}

/// Parse a dialect keyword following `#pragma`
pub fn lex_dialect_keyword<'a>(input: &'a str, keyword: &str) -> IResult<&'a str, &'a str> {
    let parser = tag(keyword);
    parser(input)
}

/// Parse "omp" keyword (OpenMP default)
#[cfg(test)]
pub fn lex_omp(input: &str) -> IResult<&str, &str> {
    lex_dialect_keyword(input, "omp")
}

/// Parse Fortran free-form sentinel with a specific prefix ("!$OMP", "!$ACC", etc.)
///
/// Supports leading whitespace before the sentinel (common for indented code):
/// - "!$OMP PARALLEL" -> matches with prefix="omp"
/// - "!$ACC PARALLEL" -> matches with prefix="acc"
/// - "    !$OMP PARALLEL" -> matches (leading spaces consumed)
/// - "  \t!$OMP DO" -> matches (mixed whitespace consumed)
pub fn lex_fortran_free_sentinel_with_prefix<'a>(
    input: &'a str,
    prefix: &str,
) -> IResult<&'a str, &'a str> {
    // Skip optional leading whitespace (common in indented Fortran code)
    let (after_space, _) = skip_space_and_comments(input)?;

    // Try full form first (!$omp, !$acc, etc.)
    let expected = format!("!${prefix}");
    if after_space
        .get(..expected.len())
        .is_some_and(|s| s.eq_ignore_ascii_case(&expected))
    {
        return Ok((
            &after_space[expected.len()..],
            &after_space[..expected.len()],
        ));
    }

    // Try short form (!$) for OpenMP and OpenACC
    let lower = prefix.to_ascii_lowercase();
    if (lower == "omp" || lower == "acc")
        && after_space
            .get(..2)
            .is_some_and(|s| s.eq_ignore_ascii_case("!$"))
    {
        return Ok((&after_space[2..], &after_space[..2]));
    }

    Err(nom::Err::Error(nom::error::Error::new(
        input,
        nom::error::ErrorKind::Tag,
    )))
}

/// Parse Fortran free-form sentinel "!$OMP" (case-insensitive)
#[cfg(test)]
pub fn lex_fortran_free_sentinel(input: &str) -> IResult<&str, &str> {
    lex_fortran_free_sentinel_with_prefix(input, "omp")
}

/// Parse Fortran fixed-form sentinel with a specific prefix
/// ("!$OMP", "C$OMP", "!$ACC", "C$ACC", etc.) or short forms in columns 1-6 (case-insensitive)
///
/// Supports both long-form (!$OMP, C$OMP, *$OMP) and short-form (!$, C$, *$) sentinels.
/// Supports leading whitespace before the sentinel:
/// - "!$OMP PARALLEL" -> matches with prefix="omp"
/// - "!$ACC PARALLEL" -> matches with prefix="acc"
/// - "    !$OMP PARALLEL" -> matches (leading spaces consumed)
/// - "C$OMP DO" -> matches
/// - "*$OMP END PARALLEL" -> matches
/// - "!$ PARALLEL" -> matches (short form)
pub fn lex_fortran_fixed_sentinel_with_prefix<'a>(
    input: &'a str,
    prefix: &str,
) -> IResult<&'a str, &'a str> {
    // Skip optional leading whitespace (common in indented Fortran code)
    let (after_space, _) = skip_space_and_comments(input)?;

    // Check for both long-form (!$omp, c$omp, *$omp) and short-form (!$, c$, *$) sentinels
    if let Some(len) = match_fortran_sentinel_with_prefix(after_space, prefix) {
        Ok((&after_space[len..], &after_space[..len]))
    } else {
        Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )))
    }
}

/// Parse Fortran fixed-form sentinel "!$OMP", "C$OMP", or short forms (case-insensitive)
#[cfg(test)]
pub fn lex_fortran_fixed_sentinel(input: &str) -> IResult<&str, &str> {
    lex_fortran_fixed_sentinel_with_prefix(input, "omp")
}

/// Parse an identifier (directive or clause name)
///
/// Learning Rust: Higher-Order Functions
/// ======================================
/// take_while1 is a higher-order function
/// It takes a function (predicate) and returns a parser
/// The parser consumes chars while predicate is true
fn lex_identifier(input: &str) -> IResult<&str, &str> {
    take_while1(is_identifier_char)(input)
}

/// Parse a clause name (e.g., "private", "nowait")
pub fn lex_clause(input: &str) -> IResult<&str, &str> {
    lex_identifier(input)
}

/// Find the closing parenthesis matching an opening parenthesis immediately
/// before `input`. Parentheses inside quoted literals are ignored. Both C-like
/// backslash escapes and Fortran doubled-quote escapes are recognized.
/// Delimiters in comments are also ignored. Malformed lexical state never
/// yields a guessed closing position.
pub(crate) fn find_matching_parenthesis(input: &str, fortran: bool) -> Option<usize> {
    let comment_style = if fortran {
        crate::delimiter::CommentStyle::Fortran
    } else {
        crate::delimiter::CommentStyle::CLike
    };
    crate::delimiter::find_matching_after_open(input, '(', ')', comment_style)
        .ok()
        .flatten()
}

/// Skip whitespace and C-style comments
///
/// Learning Rust: Manual Parsing and Byte Manipulation
/// ====================================================
/// Sometimes you need to go beyond parser combinators!
/// This function manually iterates through bytes for performance
pub fn skip_space_and_comments(input: &str) -> IResult<&str, &str> {
    skip_language_trivia(input, false)
}

/// Skip Fortran whitespace, continuations, and trailing `!` comments.
/// `//` remains an operator and `/* */` remains ordinary invalid syntax.
pub(crate) fn skip_fortran_space_and_comments(input: &str) -> IResult<&str, &str> {
    skip_language_trivia(input, true)
}

fn skip_language_trivia(input: &str, fortran: bool) -> IResult<&str, &str> {
    let mut i = 0;
    let bytes = input.as_bytes();
    let len = bytes.len();

    while i < len {
        let continuation = if fortran {
            skip_fortran_continuation(input, i)
        } else {
            skip_c_line_continuation(input, i)
        };
        if let Some(next_idx) = continuation {
            i = next_idx;
            continue;
        }

        // Learning Rust: Working with Bytes
        // ==================================
        // .as_bytes() converts &str to &[u8] (byte slice)
        // Useful for ASCII operations (faster than chars)
        if bytes[i].is_ascii_whitespace() {
            // Learning Rust: UTF-8 Handling
            // ==============================
            // chars() iterates over Unicode scalar values
            // len_utf8() returns bytes needed for this character
            // Every ASCII whitespace byte is one complete UTF-8 scalar.
            i += 1;
            continue;
        }

        // Handle /* */ comments
        if !fortran && i + 1 < len && &input[i..i + 2] == "/*" {
            if let Some(end) = input[i + 2..].find("*/") {
                i += 2 + end + 2;
                continue;
            } else {
                return Err(nom::Err::Error(nom::error::Error::new(
                    &input[i..],
                    nom::error::ErrorKind::Tag,
                )));
            }
        }

        // Handle // comments
        if !fortran && i + 1 < len && &input[i..i + 2] == "//" {
            if let Some(end) = input[i + 2..].find('\n') {
                i += 2 + end + 1;
            } else {
                i = len;
            }
            continue;
        }

        if fortran && bytes[i] == b'!' {
            if let Some(end) = input[i + 1..].find('\n') {
                i += 1 + end + 1;
            } else {
                i = len;
            }
            continue;
        }

        break;
    }

    // This parser reports only the remaining input; callers do not inspect the
    // consumed whitespace/comment slice.
    Ok((&input[i..], &input[..0]))
}

/// Skip whitespace/comments - requires at least one
pub fn skip_space1_and_comments(input: &str) -> IResult<&str, &str> {
    let (rest, _) = skip_space_and_comments(input)?;

    // Learning Rust: Error Handling in Parsers
    // =========================================
    // Return an error if nothing was consumed
    if rest.len() == input.len() {
        Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Space,
        )))
    } else {
        Ok((rest, &input[..0]))
    }
}

pub(crate) fn skip_fortran_space1_and_comments(input: &str) -> IResult<&str, &str> {
    let (rest, _) = skip_fortran_space_and_comments(input)?;
    if rest.len() == input.len() {
        Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Space,
        )))
    } else {
        Ok((rest, &input[..0]))
    }
}

/// Parse an identifier token (exposed publicly)
pub fn lex_identifier_token(input: &str) -> IResult<&str, &str> {
    lex_identifier(input)
}

/// Check if input contains continuation markers (optimized single-pass)
///
/// Performance optimization (Issue #29): Replaces 2× O(n) scans from
/// `contains('\\') + contains('&')` with single O(n) pass.
///
/// The compiler can vectorize this loop for SIMD performance on modern CPUs.
#[inline]
fn has_continuation_markers(bytes: &[u8]) -> bool {
    for &b in bytes {
        if b == b'\\' || b == b'&' {
            return true;
        }
    }
    false
}

/// Collapse line continuations in input string
///
/// This function is public for benchmarking purposes but should be considered
/// an internal implementation detail.
#[doc(hidden)]
pub fn collapse_line_continuations(input: &str) -> Cow<'_, str> {
    // Performance optimization (Issue #29): Use single-pass byte scan instead of
    // 2× contains() calls to check for continuation markers.

    let bytes = input.as_bytes();
    if !has_continuation_markers(bytes) {
        return Cow::Borrowed(input);
    }

    let mut output = String::with_capacity(input.len());
    let mut idx = 0;
    let len = bytes.len();
    let mut changed = false;

    while idx < len {
        if bytes[idx] == b'\\' {
            if let Some(next) = skip_c_line_continuation(input, idx) {
                changed = true;
                // Translation phase 2 removes exactly backslash-newline. Any
                // whitespace before the backslash or at the start of the next
                // physical line remains source whitespace; synthesizing a
                // separator here would incorrectly turn `parallel\\\nfor`
                // into `parallel for` instead of the token `parallelfor`.
                idx = next;
                continue;
            }
        } else if bytes[idx] == b'&' {
            if let Some(next) = skip_fortran_continuation(input, idx) {
                changed = true;
                // Remove only syntax that belongs to the continuation. Token
                // separation must come from actual whitespace before the
                // trailing marker or after the optional leading marker.
                idx = next;
                continue;
            }
        }

        let next = next_char_boundary(input, idx);
        output.push_str(&input[idx..next]);
        idx = next;
    }

    if changed {
        Cow::Owned(output)
    } else {
        Cow::Borrowed(input)
    }
}

fn next_char_boundary(input: &str, start: usize) -> usize {
    let mut next = start.saturating_add(1).min(input.len());
    while next < input.len() && !input.is_char_boundary(next) {
        next += 1;
    }
    next
}

fn skip_c_line_continuation(input: &str, idx: usize) -> Option<usize> {
    let bytes = input.as_bytes();
    let len = bytes.len();
    if idx >= len || bytes[idx] != b'\\' {
        return None;
    }

    let mut next = idx + 1;

    if next >= len {
        return None;
    }

    match bytes[next] {
        b'\n' => {
            next += 1;
        }
        b'\r' if bytes.get(next + 1) == Some(&b'\n') => {
            next += 2;
        }
        _ => return None,
    }

    Some(next)
}

fn skip_fortran_continuation(input: &str, idx: usize) -> Option<usize> {
    let bytes = input.as_bytes();
    let len = bytes.len();
    if idx >= len || bytes[idx] != b'&' {
        return None;
    }

    let mut next = idx + 1;

    while next < len {
        match bytes[next] {
            b' ' | b'\t' => next += 1,
            b'!' => {
                next += 1;
                while next < len && bytes[next] != b'\n' && bytes[next] != b'\r' {
                    next += 1;
                }
                break;
            }
            b'\n' | b'\r' => break,
            _ => return None,
        }
    }

    if next >= len {
        return None;
    }

    if bytes[next] == b'\r' {
        next += 1;
        if next < len && bytes[next] == b'\n' {
            next += 1;
        }
    } else if bytes[next] == b'\n' {
        next += 1;
    } else {
        return None;
    }

    let continuation_line_start = next;
    while next < len && matches!(bytes[next], b' ' | b'\t') {
        next += 1;
    }

    if let Some(len_sent) = match_fortran_continuation_sentinel(&input[next..]) {
        next += len_sent;
        let whitespace_start = next;
        while next < len && matches!(bytes[next], b' ' | b'\t') {
            next += 1;
        }
        if next < len && bytes[next] == b'&' {
            return Some(next + 1);
        }
        return Some(whitespace_start);
    }

    if next < len && bytes[next] == b'&' {
        return Some(next + 1);
    }

    // Without a sentinel or leading continuation marker, indentation is part
    // of the continued source and may be the only separator between tokens.
    Some(continuation_line_start)
}

/// Match Fortran fixed-form sentinel with a specific prefix
/// Returns the length of the matched sentinel, or None if no match
fn match_fortran_sentinel_with_prefix(input: &str, prefix: &str) -> Option<usize> {
    let mut candidates = Vec::new();
    let lower = prefix.to_ascii_lowercase();
    for sentinel in ["!$", "c$", "*$"] {
        candidates.push(format!("{sentinel}{lower}"));
    }
    // Short forms (!$, c$, *$) are valid for both OpenMP and OpenACC
    if lower == "omp" || lower == "acc" {
        candidates.extend(["!$".to_string(), "c$".to_string(), "*$".to_string()]);
    }

    for candidate in candidates {
        if input.len() >= candidate.len()
            && input[..candidate.len()].eq_ignore_ascii_case(&candidate)
        {
            return Some(candidate.len());
        }
    }
    None
}

/// Match a dialect-specific continuation sentinel only when the sentinel ends
/// at a valid boundary. This prevents a short `!$` match from consuming the
/// start of a different long sentinel such as `!$acc`.
fn match_fortran_continuation_sentinel_with_prefix(input: &str, prefix: &str) -> Option<usize> {
    let length = match_fortran_sentinel_with_prefix(input, prefix)?;
    match input.as_bytes().get(length) {
        None | Some(b' ' | b'\t' | b'&' | b'\r' | b'\n') => Some(length),
        Some(_) => None,
    }
}

/// Match either standardized Fortran directive sentinel. Logical-source
/// validation checks that a matched long sentinel belongs to the parser's
/// selected dialect before this shared continuation stripper runs.
fn match_fortran_continuation_sentinel(input: &str) -> Option<usize> {
    ["omp", "acc"]
        .into_iter()
        .filter_map(|prefix| match_fortran_continuation_sentinel_with_prefix(input, prefix))
        .max()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pragma_keyword() {
        let result = lex_pragma("#pragma omp parallel");
        assert!(result.is_ok());

        // Learning Rust: Destructuring
        // =============================
        // Extract values from tuples using pattern matching
        let (remaining, matched) = result.unwrap();
        assert_eq!(matched, "#pragma");
        assert_eq!(remaining, " omp parallel");
    }

    #[test]
    fn parses_omp_keyword() {
        let (remaining, matched) = lex_omp("omp parallel").unwrap();
        assert_eq!(matched, "omp");
        assert_eq!(remaining, " parallel");
    }

    #[test]
    fn parses_identifiers() {
        let (rest, name) = lex_identifier("parallel private").unwrap();
        assert_eq!(name, "parallel");
        assert_eq!(rest, " private");

        let (rest2, name2) = lex_identifier("private_data(x)").unwrap();
        assert_eq!(name2, "private_data");
        assert_eq!(rest2, "(x)");
    }

    #[test]
    fn identifier_requires_alphanumeric() {
        // Should fail on special characters
        let result = lex_identifier("(invalid");
        assert!(result.is_err());
    }

    #[test]
    fn skips_whitespace() {
        let (rest, _) = skip_space_and_comments("   hello").unwrap();
        assert_eq!(rest, "hello");

        let (rest, _) = skip_space_and_comments("\t\n  world").unwrap();
        assert_eq!(rest, "world");
    }

    #[test]
    fn skips_c_style_comments() {
        let (rest, _) = skip_space_and_comments("/* comment */ code").unwrap();
        assert_eq!(rest, "code");

        let (rest, _) = skip_space_and_comments("/* multi\nline\ncomment */ after").unwrap();
        assert_eq!(rest, "after");
    }

    #[test]
    fn skips_cpp_style_comments() {
        let (rest, _) = skip_space_and_comments("// comment\ncode").unwrap();
        assert_eq!(rest, "code");
    }

    #[test]
    fn skips_mixed_whitespace_and_comments() {
        let input = "  /* comment1 */  \n  // comment2\n  code";
        let (rest, _) = skip_space_and_comments(input).unwrap();
        assert_eq!(rest, "code");
    }

    #[test]
    fn skip_space1_requires_whitespace() {
        let result = skip_space1_and_comments("no_space");
        assert!(result.is_err());

        let result = skip_space1_and_comments(" has_space");
        assert!(result.is_ok());
    }

    #[test]
    fn skip_space_handles_c_line_continuations() {
        let (rest, _) = skip_space_and_comments("\\\n    default(none)").unwrap();
        assert_eq!(rest, "default(none)");
    }

    #[test]
    fn skip_space_handles_fortran_continuations() {
        let input = "&\n!$omp private(i, j)";
        let (rest, _) = skip_fortran_space_and_comments(input).unwrap();
        assert_eq!(rest, "private(i, j)");
    }

    #[test]
    fn trivia_comment_syntax_is_language_specific() {
        assert_eq!(skip_fortran_space_and_comments("! comment").unwrap().0, "");
        assert_eq!(
            skip_fortran_space_and_comments("// operator").unwrap().0,
            "// operator"
        );
        assert_eq!(
            skip_fortran_space_and_comments("/* not a comment */")
                .unwrap()
                .0,
            "/* not a comment */"
        );
    }

    #[test]
    fn collapse_line_continuations_removes_c_backslash() {
        let collapsed = collapse_line_continuations(concat!("a, \\\n", "    b"));
        assert_eq!(collapsed.as_ref(), "a,     b");
    }

    #[test]
    fn c_line_splicing_never_synthesizes_token_separation() {
        let collapsed = collapse_line_continuations("parallel\\\nfor");
        assert_eq!(collapsed.as_ref(), "parallelfor");

        let malformed_extension = collapse_line_continuations("parallel\\  \nfor");
        assert_eq!(malformed_extension.as_ref(), "parallel\\  \nfor");
    }

    #[test]
    fn logical_line_validation_rejects_uncontinued_following_text() {
        assert!(
            validate_logical_line("#pragma omp parallel\nprivate(x)", Language::C, "omp").is_err()
        );
        assert!(validate_logical_line(
            "!$omp parallel\n!$omp private(x)",
            Language::FortranFree,
            "omp",
        )
        .is_err());

        assert!(validate_logical_line("#pragma omp parallel\n  ", Language::C, "omp").is_ok());
        assert!(
            validate_logical_line("#pragma omp parallel \\\n  private(x)", Language::C, "omp",)
                .is_ok()
        );
        assert!(validate_logical_line(
            "!$omp parallel & ! comment\n!$omp& private(x)",
            Language::FortranFree,
            "omp",
        )
        .is_ok());
    }

    #[test]
    fn logical_source_maps_spliced_tokens_to_physical_source() {
        let source = "#pragma omp paral\\\nlel private(α)";
        let logical = LogicalSource::new(source, Language::C, "omp").unwrap();
        assert_eq!(logical.text(), "#pragma omp parallel private(α)");

        let start = logical.text().find("parallel").unwrap();
        let span = logical.span(start..start + "parallel".len()).unwrap();
        assert_eq!(span.slice(source), Ok("paral\\\nlel"));

        let private = &logical.text()[logical.text().find("private").unwrap()..];
        let span = logical.span_of(private).unwrap();
        assert_eq!(span.start().line(), 2);
        assert_eq!(span.slice(source), Ok("private(α)"));
    }

    #[test]
    fn c_continuation_rejects_whitespace_after_backslash() {
        assert!(
            validate_logical_line("#pragma omp parallel\\  \nfor", Language::C, "omp").is_err()
        );
        assert!(validate_logical_line("#pragma omp parallel \\\rfor", Language::C, "omp").is_err());
        assert_eq!(
            collapse_line_continuations("parallel\\\rfor").as_ref(),
            "parallel\\\rfor"
        );
    }

    #[test]
    fn collapse_line_continuations_removes_fortran_ampersand() {
        let input = "items( i, &\n!$omp& j )";
        let collapsed = collapse_line_continuations(input);
        assert_eq!(collapsed.as_ref(), "items( i,  j )");

        let openacc = "copy( a, &\n!$acc& b )";
        let collapsed = collapse_line_continuations(openacc);
        assert_eq!(collapsed.as_ref(), "copy( a,  b )");
    }

    #[test]
    fn fortran_continuation_uses_only_source_token_separation() {
        let separated = collapse_line_continuations("parallel&\n!$omp& do");
        assert_eq!(separated.as_ref(), "parallel do");

        let joined = collapse_line_continuations("parallel&\n!$omp&do");
        assert_eq!(joined.as_ref(), "paralleldo");
    }

    #[test]
    fn parses_fortran_free_sentinel() {
        let (rest, matched) = lex_fortran_free_sentinel("!$OMP parallel").unwrap();
        assert_eq!(matched, "!$OMP");
        assert_eq!(rest, " parallel");

        // Case-insensitive
        let (rest, matched) = lex_fortran_free_sentinel("!$omp PARALLEL").unwrap();
        assert_eq!(matched, "!$omp");
        assert_eq!(rest, " PARALLEL");
    }

    #[test]
    fn parses_fortran_fixed_sentinel() {
        let (rest, matched) = lex_fortran_fixed_sentinel("!$OMP parallel").unwrap();
        assert_eq!(matched, "!$OMP");
        assert_eq!(rest, " parallel");

        let (rest, matched) = lex_fortran_fixed_sentinel("C$OMP parallel").unwrap();
        assert_eq!(matched, "C$OMP");
        assert_eq!(rest, " parallel");

        // Case-insensitive
        let (rest, matched) = lex_fortran_fixed_sentinel("c$omp PARALLEL").unwrap();
        assert_eq!(matched, "c$omp");
        assert_eq!(rest, " PARALLEL");
    }

    #[test]
    fn normalizes_fortran_identifiers() {
        assert_eq!(normalize_fortran_identifier("PARALLEL"), "parallel");
        assert_eq!(normalize_fortran_identifier("Private"), "private");
        assert_eq!(normalize_fortran_identifier("num_threads"), "num_threads");
    }

    #[test]
    fn optimized_single_pass_no_markers() {
        // Test optimized single-pass scan for inputs without continuation markers
        let inputs = vec![
            "parallel",
            "parallel for private(i)",
            "target teams distribute",
            "simd reduction(+:sum) private(i,j,k)",
        ];

        for input in inputs {
            let result = collapse_line_continuations(input);
            // Should return borrowed (no markers detected, no allocation)
            assert!(matches!(result, Cow::Borrowed(_)));
            assert_eq!(result.as_ref(), input);
        }
    }

    #[test]
    fn single_pass_with_markers() {
        // Test that single-pass scan correctly detects markers
        let has_backslash = "parallel \\\n num_threads(4)";
        let has_ampersand = "parallel do &\n!$omp private(i)";

        // Both should be detected and processed
        let r1 = collapse_line_continuations(has_backslash);
        let r2 = collapse_line_continuations(has_ampersand);

        assert!(matches!(r1, Cow::Owned(_)));
        assert!(matches!(r2, Cow::Owned(_)));
    }
}
