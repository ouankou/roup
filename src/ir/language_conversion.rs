//! Utilities for converting directives between language syntaxes.
//!
//! This module provides helpers that parse a directive in one host language
//! (C/C++ or Fortran) and pretty-print it using the canonical syntax of
//! another language. It is primarily used to support workflows that need to
//! translate OpenMP pragmas across language front-ends.

use std::fmt;

use crate::lexer::Language as LexerLanguage;
use crate::parser::openmp;

use super::convert::{convert_directive, ConversionError};
use super::{Language, ParserConfig, SourceLocation};

/// Errors that can occur while converting between language syntaxes.
#[derive(Debug)]
pub enum LanguageConversionError {
    /// The directive text could not be parsed in the input language.
    ParseError(String),
    /// Conversion to the IR failed (usually due to unsupported clauses).
    Conversion(ConversionError),
}

impl fmt::Display for LanguageConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LanguageConversionError::ParseError(msg) => {
                write!(f, "failed to parse directive: {}", msg)
            }
            LanguageConversionError::Conversion(err) => {
                write!(f, "failed to convert directive: {}", err)
            }
        }
    }
}

impl std::error::Error for LanguageConversionError {}

fn parser_language(language: Language) -> LexerLanguage {
    match language {
        Language::Fortran => LexerLanguage::FortranFree,
        _ => LexerLanguage::C,
    }
}

/// Convert a directive string between language syntaxes.
///
/// The directive must include the sentinel (e.g. `#pragma omp` or `!$OMP`).
/// Expressions are preserved verbatim; this function focuses on directive and
/// clause syntax.
pub fn convert_directive_language(
    input: &str,
    from: Language,
    to: Language,
) -> Result<String, LanguageConversionError> {
    let parser = openmp::parser().with_language(parser_language(from));
    let (_, directive) = parser
        .parse(input)
        .map_err(|err| LanguageConversionError::ParseError(format!("{:?}", err)))?;

    let config = ParserConfig::with_parsing(from);
    let ir = convert_directive(&directive, SourceLocation::start(), from, &config)
        .map_err(LanguageConversionError::Conversion)?;

    Ok(ir.display_as(to).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_parallel_for_to_fortran() {
        let input = "#pragma omp parallel for private(i)";
        let converted = convert_directive_language(input, Language::C, Language::Fortran)
            .expect("conversion should succeed");
        assert_eq!(converted, "!$omp parallel do private(i)");
    }

    #[test]
    fn preserves_complex_clauses() {
        let input = "#pragma omp target teams distribute parallel for simd schedule(static, 4)";
        let converted = convert_directive_language(input, Language::C, Language::Fortran)
            .expect("conversion should succeed");
        assert_eq!(
            converted,
            "!$omp target teams distribute parallel do simd schedule(static, 4)"
        );
    }

    #[test]
    fn reports_parse_errors() {
        let err = convert_directive_language("not a pragma", Language::C, Language::Fortran)
            .expect_err("conversion should fail");
        assert!(matches!(err, LanguageConversionError::ParseError(_)));
    }
}
