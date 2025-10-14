/// Lexer module - tokenizes OpenMP pragma directives
///
/// Learning Rust: Parser Combinators with nom
/// ===========================================
/// nom is a parser combinator library
/// - Parsers are functions that consume input and return results
/// - Combine small parsers to build complex ones
/// - Type: IResult<Input, Output, Error>

/// Learning Rust: Type Aliases
/// ============================
/// IResult is nom's result type for parser functions
/// IResult<&str, &str> means:
/// - Input: &str (string slice to parse)
/// - Output: &str (parsed token)
/// - Returns: Ok((remaining_input, parsed_output)) or Err
use std::borrow::Cow;

use nom::IResult;

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

impl Default for Language {
    fn default() -> Self {
        Language::C
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

/// Parse "omp" keyword
pub fn lex_omp(input: &str) -> IResult<&str, &str> {
    tag("omp")(input)
}

/// Parse Fortran free-form sentinel "!$OMP" (case-insensitive)
///
/// Supports leading whitespace before the sentinel (common for indented code):
/// - "!$OMP PARALLEL" -> matches
/// - "    !$OMP PARALLEL" -> matches (leading spaces consumed)
/// - "  \t!$OMP DO" -> matches (mixed whitespace consumed)
pub fn lex_fortran_free_sentinel(input: &str) -> IResult<&str, &str> {
    // Skip optional leading whitespace (common in indented Fortran code)
    let (after_space, _) = skip_space_and_comments(input)?;

    // Optimize: check only first 5 characters instead of entire input
    let matches = after_space
        .get(..5)
        .map_or(false, |s| s.eq_ignore_ascii_case("!$omp"));

    if matches {
        Ok((&after_space[5..], &after_space[..5]))
    } else {
        Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )))
    }
}

/// Parse Fortran fixed-form sentinel "!$OMP" or "C$OMP" in columns 1-6 (case-insensitive)
///
/// Supports leading whitespace before the sentinel:
/// - "!$OMP PARALLEL" -> matches
/// - "    !$OMP PARALLEL" -> matches (leading spaces consumed)
/// - "C$OMP DO" -> matches
/// - "*$OMP END PARALLEL" -> matches
pub fn lex_fortran_fixed_sentinel(input: &str) -> IResult<&str, &str> {
    // Skip optional leading whitespace (common in indented Fortran code)
    let (after_space, _) = skip_space_and_comments(input)?;

    // Optimize: check only first 5 characters instead of entire input
    let first_5 = after_space.get(..5);
    let matches = first_5.map_or(false, |s| {
        s.eq_ignore_ascii_case("!$omp")
            || s.eq_ignore_ascii_case("c$omp")
            || s.eq_ignore_ascii_case("*$omp")
    });

    if matches {
        Ok((&after_space[5..], &after_space[..5]))
    } else {
        Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )))
    }
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

/// Parse a directive name (e.g., "parallel", "for")
pub fn lex_directive(input: &str) -> IResult<&str, &str> {
    lex_identifier(input)
}

/// Parse a clause name (e.g., "private", "nowait")
pub fn lex_clause(input: &str) -> IResult<&str, &str> {
    lex_identifier(input)
}

/// Skip whitespace and C-style comments
///
/// Learning Rust: Manual Parsing and Byte Manipulation
/// ====================================================
/// Sometimes you need to go beyond parser combinators!
/// This function manually iterates through bytes for performance
pub fn skip_space_and_comments(input: &str) -> IResult<&str, &str> {
    let mut i = 0;
    let bytes = input.as_bytes();
    let len = bytes.len();

    while i < len {
        // Handle C/C++ line continuations using backslash-newline
        if let Some(next_idx) = skip_c_line_continuation(input, i) {
            i = next_idx;
            continue;
        }

        // Handle Fortran continuation markers using trailing ampersand
        if let Some(next_idx) = skip_fortran_continuation(input, i) {
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
            let ch = input[i..].chars().next().unwrap();
            i += ch.len_utf8();
            continue;
        }

        // Handle /* */ comments
        if i + 1 < len && &input[i..i + 2] == "/*" {
            if let Some(end) = input[i + 2..].find("*/") {
                i += 2 + end + 2;
                continue;
            } else {
                // Unterminated comment - consume to end
                i = len;
                break;
            }
        }

        // Handle // comments
        if i + 1 < len && &input[i..i + 2] == "//" {
            if let Some(end) = input[i + 2..].find('\n') {
                i += 2 + end + 1;
            } else {
                i = len;
            }
            continue;
        }

        break;
    }

    // Return (remaining, consumed) - consumed is empty slice for compatibility
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

/// Parse an identifier token (exposed publicly)
pub fn lex_identifier_token(input: &str) -> IResult<&str, &str> {
    lex_identifier(input)
}

pub(crate) fn collapse_line_continuations<'a>(input: &'a str) -> Cow<'a, str> {
    // P1 Fix: Preserve whitespace when collapsing line continuations to prevent
    // token merging (e.g., "parallel\\\n    for" → "parallel for" not "parallelfor").
    // We insert a space when collapsing unless there's already trailing whitespace.
    //
    // For Fortran continuations, we also preserve a space to maintain token separation.

    if !input.contains('\\') && !input.contains('&') {
        return Cow::Borrowed(input);
    }

    let mut output = String::with_capacity(input.len());
    let mut idx = 0;
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut changed = false;

    while idx < len {
        if bytes[idx] == b'\\' {
            let mut next = idx + 1;
            while next < len && matches!(bytes[next], b' ' | b'\t') {
                next += 1;
            }
            if next < len && (bytes[next] == b'\n' || bytes[next] == b'\r') {
                changed = true;
                if bytes[next] == b'\r' {
                    next += 1;
                    if next < len && bytes[next] == b'\n' {
                        next += 1;
                    }
                } else {
                    next += 1;
                }
                while next < len && matches!(bytes[next], b' ' | b'\t') {
                    next += 1;
                }
                // Insert a space to preserve token separation, but only if
                // the output doesn't already end with whitespace
                if !output.is_empty() && !output.ends_with(|c: char| c.is_whitespace()) {
                    output.push(' ');
                }
                idx = next;
                continue;
            }
        } else if bytes[idx] == b'&' {
            if let Some(next) = skip_fortran_continuation(input, idx) {
                changed = true;
                // For Fortran, also preserve token separation
                if !output.is_empty() && !output.ends_with(|c: char| c.is_whitespace()) {
                    output.push(' ');
                }
                idx = next;
                continue;
            }
        }

        let ch = input[idx..].chars().next().unwrap();
        output.push(ch);
        idx += ch.len_utf8();
    }

    if changed {
        Cow::Owned(output)
    } else {
        Cow::Borrowed(input)
    }
}

fn skip_c_line_continuation(input: &str, idx: usize) -> Option<usize> {
    let bytes = input.as_bytes();
    let len = bytes.len();
    if idx >= len || bytes[idx] != b'\\' {
        return None;
    }

    let mut next = idx + 1;
    while next < len && matches!(bytes[next], b' ' | b'\t') {
        next += 1;
    }

    if next >= len {
        return Some(len);
    }

    match bytes[next] {
        b'\n' => {
            next += 1;
        }
        b'\r' => {
            next += 1;
            if next < len && bytes[next] == b'\n' {
                next += 1;
            }
        }
        _ => return None,
    }

    while next < len && matches!(bytes[next], b' ' | b'\t') {
        next += 1;
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
        return Some(len);
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

    while next < len {
        match bytes[next] {
            b' ' | b'\t' => next += 1,
            b'\r' | b'\n' => {
                next += 1;
            }
            _ => break,
        }
    }

    if let Some(len_sent) = match_fortran_sentinel(&input[next..]) {
        next += len_sent;
        while next < len && matches!(bytes[next], b' ' | b'\t') {
            next += 1;
        }
    }

    if next < len && bytes[next] == b'&' {
        next += 1;
        while next < len && matches!(bytes[next], b' ' | b'\t') {
            next += 1;
        }
    }

    Some(next)
}

fn match_fortran_sentinel(input: &str) -> Option<usize> {
    let candidates = ["!$omp", "c$omp", "*$omp"];
    for candidate in candidates {
        if input.len() >= candidate.len()
            && input[..candidate.len()].eq_ignore_ascii_case(candidate)
        {
            return Some(candidate.len());
        }
    }
    None
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
        let (rest, _) = skip_space_and_comments(input).unwrap();
        assert_eq!(rest, "private(i, j)");
    }

    #[test]
    fn collapse_line_continuations_removes_c_backslash() {
        let collapsed = collapse_line_continuations(concat!("a, \\\n", "    b"));
        assert_eq!(collapsed.as_ref(), "a, b");
    }

    #[test]
    fn collapse_line_continuations_removes_fortran_ampersand() {
        let input = "items( i, &\n!$omp& j )";
        let collapsed = collapse_line_continuations(input);
        assert_eq!(collapsed.as_ref(), "items( i, j )");
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
}
