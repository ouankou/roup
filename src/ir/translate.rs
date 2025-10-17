//! Language translation helpers for OpenMP directives
//!
//! This module provides high level utilities for converting parsed OpenMP
//! pragmas between host languages. The primary use-case today is translating
//! C/C++ `#pragma omp` directives into their Fortran `!$omp` equivalents so
//! existing C benchmarks can be reused by Fortran tooling.
//!
//! # Examples
//!
//! ```rust
//! use roup::ir::translate::translate_c_to_fortran;
//!
//! let output = translate_c_to_fortran("#pragma omp parallel for private(i)")?;
//! assert_eq!(output, "!$omp parallel do private(i)");
//! # Ok::<(), roup::ir::translate::TranslationError>(())
//! ```

use std::fmt;

use super::{convert::convert_directive, DirectiveIR, Language, ParserConfig, SourceLocation};
use crate::parser::parse_omp_directive;

/// Errors that can occur while translating between host languages
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranslationError {
    /// Input string was empty or contained only whitespace
    EmptyInput,
    /// Parser failed to recognise the directive
    ParseError(String),
    /// Semantic conversion failed (unknown directive/clause)
    ConversionError(super::ConversionError),
}

impl fmt::Display for TranslationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TranslationError::EmptyInput => write!(f, "input pragma is empty"),
            TranslationError::ParseError(msg) => write!(f, "failed to parse pragma: {}", msg),
            TranslationError::ConversionError(err) => write!(f, "conversion error: {}", err),
        }
    }
}

impl std::error::Error for TranslationError {}

impl From<super::ConversionError> for TranslationError {
    fn from(err: super::ConversionError) -> Self {
        TranslationError::ConversionError(err)
    }
}

/// Translate a C/C++ OpenMP pragma into its Fortran representation
///
/// This helper parses the input using ROUP's C parser, converts it into the
/// semantic IR, switches the language to Fortran and finally renders the
/// directive back to a string.
pub fn translate_c_to_fortran(input: &str) -> Result<String, TranslationError> {
    let config = ParserConfig::with_parsing(Language::C);
    translate_c_to_fortran_ir(input, config).map(|dir| dir.to_string())
}

/// Translate a C/C++ OpenMP pragma into a Fortran `DirectiveIR`
///
/// `ParserConfig` is accepted explicitly so callers can reuse pre-configured
/// expression parsing settings (for example string-only parsing).
pub fn translate_c_to_fortran_ir(
    input: &str,
    config: ParserConfig,
) -> Result<DirectiveIR, TranslationError> {
    if input.trim().is_empty() {
        return Err(TranslationError::EmptyInput);
    }

    let (rest, directive) = parse_omp_directive(input)
        .map_err(|err| TranslationError::ParseError(format!("{:?}", err)))?;

    if !rest.trim().is_empty() {
        return Err(TranslationError::ParseError(format!(
            "unparsed trailing input: {}",
            rest.trim()
        )));
    }

    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)?;

    Ok(ir.into_language(Language::Fortran))
}
