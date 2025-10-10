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
}
