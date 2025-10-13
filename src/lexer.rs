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

// ============================================================================
// Continuation Handling Overview
// ===========================================================================
//
// The lexer understands OpenMP continuation patterns for the supported languages:
// - C/C++: trailing backslash followed by newline (\n or \r\n)
// - Fortran free-form: trailing '&' and optional leading '&' on continuation lines
// - Fortran fixed-form: repeated sentinels (e.g., C$OMP, !$OMP, *$OMP) with an
//   optional '&' occupying column 6
//
// These rules allow ROUP to accept directives exactly as they appear in source
// files without requiring the caller to flatten or preprocess input.
// ===========================================================================

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

        // Handle C/C++ line continuations with trailing backslash
        if bytes[i] == b'\\' {
            // Support both Unix (\n) and Windows (\r\n) newlines
            if input[i + 1..].starts_with("\r\n") {
                i += 3; // Skip "\\\r\n"
                continue;
            }
            if input[i + 1..].starts_with('\n') {
                i += 2; // Skip "\\\n"
                continue;
            }
        }

        // Handle Fortran continuation markers '&' that appear at end of a line
        if bytes[i] == b'&' {
            let mut j = i + 1;
            let mut saw_newline = false;

            while j < len {
                let ch = input[j..].chars().next().unwrap();
                if ch == '\r' {
                    j += ch.len_utf8();
                    continue;
                }
                if ch == '\n' {
                    saw_newline = true;
                    j += ch.len_utf8();
                    break;
                }
                if ch.is_whitespace() {
                    j += ch.len_utf8();
                    continue;
                }
                break;
            }

            // Continuation if followed by newline (with optional whitespace) or another sentinel
            let sentinel_follows = input[j..]
                .get(..5)
                .map(|s| {
                    s.eq_ignore_ascii_case("!$omp")
                        || s.eq_ignore_ascii_case("c$omp")
                        || s.eq_ignore_ascii_case("*$omp")
                })
                .unwrap_or(false);

            let prev_line_continuation = {
                let before = &input[..i];
                let last_newline = before.rfind('\n');
                let segment = match last_newline {
                    Some(pos) => &before[..pos],
                    None => before,
                };
                segment
                    .trim_end_matches(|ch: char| ch.is_whitespace())
                    .ends_with('&')
            };

            if saw_newline || sentinel_follows || prev_line_continuation {
                i = j;
                continue;
            }

            // Otherwise '&' is significant (e.g., bitwise operator) - stop skipping
            break;
        }

        // Handle Fortran sentinel on continuation lines (e.g., !$OMP& PRIVATE)
        if i != 0 {
            let before = &input[..i];
            let at_line_start = before
                .rfind('\n')
                .or_else(|| before.rfind('\r'))
                .map(|pos| before[pos + 1..].chars().all(|ch| ch.is_ascii_whitespace()))
                .unwrap_or(false);
            let after_ampersand = before
                .chars()
                .rev()
                .find(|ch| !ch.is_ascii_whitespace())
                .map(|ch| ch == '&')
                .unwrap_or(false);

            if at_line_start || after_ampersand {
                let sentinel = input[i..].get(..5);
                let is_fortran_sentinel = sentinel.map_or(false, |s| {
                    s.eq_ignore_ascii_case("!$omp")
                        || s.eq_ignore_ascii_case("c$omp")
                        || s.eq_ignore_ascii_case("*$omp")
                });

                if is_fortran_sentinel {
                    i += 5;

                    // Skip optional whitespace after the sentinel
                    while i < len {
                        let ch = input[i..].chars().next().unwrap();
                        if ch.is_whitespace() && ch != '\n' && ch != '\r' {
                            i += ch.len_utf8();
                        } else {
                            break;
                        }
                    }

                    // Skip optional '&' directly after sentinel
                    if i < len {
                        let ch = input[i..].chars().next().unwrap();
                        if ch == '&' {
                            i += ch.len_utf8();

                            // Skip whitespace after the continuation ampersand
                            while i < len {
                                let ch2 = input[i..].chars().next().unwrap();
                                if ch2.is_whitespace() {
                                    i += ch2.len_utf8();
                                } else {
                                    break;
                                }
                            }
                        }
                    }

                    continue;
                }
            }
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
    fn skips_c_line_continuations() {
        let (rest, _) = skip_space_and_comments("\\\nnext").unwrap();
        assert_eq!(rest, "next");

        let (rest, _) = skip_space_and_comments("\\\r\n  value").unwrap();
        assert_eq!(rest, "value");
    }

    #[test]
    fn skips_fortran_trailing_ampersand() {
        let (rest, _) = skip_space_and_comments("&\n  following").unwrap();
        assert_eq!(rest, "following");

        let (rest, _) = skip_space_and_comments("&   !$OMP next").unwrap();
        assert_eq!(rest, "next");

        let (rest, _) = skip_space_and_comments("&\n& continued").unwrap();
        assert_eq!(rest, "continued");
    }

    #[test]
    fn skips_fortran_continuation_sentinel() {
        let input = "\n!$OMP& private(a)";
        let (rest, _) = skip_space_and_comments(input).unwrap();
        assert_eq!(rest, "private(a)");

        let input = "\n  C$OMP   &   schedule(static)";
        let (rest, _) = skip_space_and_comments(input).unwrap();
        assert_eq!(rest, "schedule(static)");
    }

    #[test]
    fn skip_space1_requires_whitespace() {
        let result = skip_space1_and_comments("no_space");
        assert!(result.is_err());

        let result = skip_space1_and_comments(" has_space");
        assert!(result.is_ok());
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
