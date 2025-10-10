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

/// Check if a character is valid in an identifier
///
/// Learning Rust: Closures and Function Pointers
/// ==============================================
/// This function can be used as a predicate
/// take_while1 accepts: fn(char) -> bool
pub fn is_identifier_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
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
    fn skip_space1_requires_whitespace() {
        let result = skip_space1_and_comments("no_space");
        assert!(result.is_err());

        let result = skip_space1_and_comments(" has_space");
        assert!(result.is_ok());
    }
}
