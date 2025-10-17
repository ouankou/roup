//! C/C++ to Fortran directive translation
//!
//! This module provides high-level utilities for translating OpenMP directives
//! between C/C++ and Fortran syntax. This is useful for automatically porting
//! benchmarks and code between languages.
//!
//! ## Supported Translations
//!
//! - **Sentinel translation**: `#pragma omp` ↔ `!$omp`
//! - **Loop directive names**: `for` ↔ `do`, including all combined forms
//! - **Clause preservation**: All clauses are preserved as-is (OpenMP standard)
//!
//! ## Limitations
//!
//! - **Expression translation**: Expressions within clauses are NOT translated.
//!   C syntax like `arr[i]` remains unchanged; manual adjustment needed for Fortran.
//! - **Surrounding code**: Only directive lines are translated, not actual source code.
//! - **Fixed-form Fortran**: Only free-form `!$omp` output is supported.
//!
//! ## Examples
//!
//! ```
//! use roup::ir::translate::translate_c_to_fortran;
//!
//! let c_pragma = "#pragma omp parallel for private(i) schedule(static, 4)";
//! let fortran = translate_c_to_fortran(c_pragma)?;
//! assert_eq!(fortran, "!$omp parallel do private(i) schedule(static, 4)");
//! # Ok::<(), roup::ir::translate::TranslationError>(())
//! ```

use std::fmt;

use super::{convert_directive, DirectiveIR, Language, ParserConfig, SourceLocation};
use crate::lexer::Language as LexerLanguage;
use crate::parser::openmp;

/// Errors that can occur during directive translation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranslationError {
    /// Input string was empty or contained only whitespace
    EmptyInput,
    /// Parser failed to recognize the directive
    ParseError(String),
    /// Semantic conversion failed (unknown directive/clause)
    ConversionError(super::ConversionError),
}

impl fmt::Display for TranslationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TranslationError::EmptyInput => write!(f, "input directive is empty"),
            TranslationError::ParseError(msg) => write!(f, "failed to parse directive: {}", msg),
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

/// Translate a C/C++ OpenMP directive to Fortran
///
/// Parses the input using the C parser, converts to IR, changes language to Fortran,
/// and returns the Fortran representation.
///
/// ## Example
///
/// ```
/// use roup::ir::translate::translate_c_to_fortran;
///
/// let input = "#pragma omp target teams distribute parallel for simd";
/// let output = translate_c_to_fortran(input)?;
/// assert_eq!(output, "!$omp target teams distribute parallel do simd");
/// # Ok::<(), roup::ir::translate::TranslationError>(())
/// ```
///
/// ## Errors
///
/// Returns `TranslationError` if:
/// - Input is empty
/// - Parsing fails (malformed directive)
/// - Conversion fails (unsupported construct)
pub fn translate_c_to_fortran(input: &str) -> Result<String, TranslationError> {
    let config = ParserConfig::with_parsing(Language::C);
    let ir = translate_c_to_fortran_ir(input, config)?;
    Ok(ir.to_string_for_language(Language::Fortran))
}

/// Translate a C/C++ OpenMP directive to Fortran, returning the IR
///
/// Like `translate_c_to_fortran`, but returns the `DirectiveIR` for further processing.
/// This is useful when you need to inspect or modify the directive programmatically.
///
/// ## Example
///
/// ```
/// use roup::ir::{translate::translate_c_to_fortran_ir, Language, ParserConfig};
///
/// let config = ParserConfig::with_parsing(Language::C);
/// let ir = translate_c_to_fortran_ir("#pragma omp parallel for", config)?;
/// // The IR keeps its original language (C), translation happens at rendering
/// assert_eq!(ir.language(), Language::C);
/// assert_eq!(ir.to_string_for_language(Language::Fortran), "!$omp parallel do");
/// # Ok::<(), roup::ir::translate::TranslationError>(())
/// ```
pub fn translate_c_to_fortran_ir(
    input: &str,
    config: ParserConfig,
) -> Result<DirectiveIR, TranslationError> {
    if input.trim().is_empty() {
        return Err(TranslationError::EmptyInput);
    }

    // Parse the C/C++ directive with language-aware parser
    let parser = openmp::parser().with_language(LexerLanguage::C);
    let (rest, directive) = parser
        .parse(input)
        .map_err(|err| TranslationError::ParseError(format!("{:?}", err)))?;

    // Check for unparsed input
    if !rest.trim().is_empty() {
        return Err(TranslationError::ParseError(format!(
            "unparsed trailing input: {}",
            rest.trim()
        )));
    }

    // Convert to IR with C language context
    // Note: We keep the language as C in the IR; translation happens at rendering time
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)?;

    Ok(ir)
}

/// Translate a Fortran OpenMP directive to C/C++
///
/// Parses the input using the Fortran parser, converts to IR, changes language to C,
/// and returns the C representation.
///
/// ## Example
///
/// ```
/// use roup::ir::translate::translate_fortran_to_c;
///
/// let input = "!$omp parallel do private(i)";
/// let output = translate_fortran_to_c(input)?;
/// assert_eq!(output, "#pragma omp parallel for private(i)");
/// # Ok::<(), roup::ir::translate::TranslationError>(())
/// ```
pub fn translate_fortran_to_c(input: &str) -> Result<String, TranslationError> {
    let config = ParserConfig::with_parsing(Language::Fortran);
    let ir = translate_fortran_to_c_ir(input, config)?;
    Ok(ir.to_string_for_language(Language::C))
}

/// Translate a Fortran OpenMP directive to C/C++, returning the IR
///
/// Like `translate_fortran_to_c`, but returns the `DirectiveIR` for further processing.
///
/// This function auto-detects the Fortran format (free-form vs fixed-form) by examining
/// the sentinel. Free-form uses `!$omp`, while fixed-form uses `C$OMP`, `c$omp`, or `*$omp`.
pub fn translate_fortran_to_c_ir(
    input: &str,
    config: ParserConfig,
) -> Result<DirectiveIR, TranslationError> {
    if input.trim().is_empty() {
        return Err(TranslationError::EmptyInput);
    }

    // Auto-detect Fortran format by checking the sentinel
    let fortran_lang = detect_fortran_format(input);

    // Parse the Fortran directive with language-aware parser
    let parser = openmp::parser().with_language(fortran_lang);
    let (rest, directive) = parser
        .parse(input)
        .map_err(|err| TranslationError::ParseError(format!("{:?}", err)))?;

    // Check for unparsed input
    if !rest.trim().is_empty() {
        return Err(TranslationError::ParseError(format!(
            "unparsed trailing input: {}",
            rest.trim()
        )));
    }

    // Convert to IR with Fortran language context
    // Note: We keep the language as Fortran in the IR; translation happens at rendering time
    let ir = convert_directive(
        &directive,
        SourceLocation::start(),
        Language::Fortran,
        &config,
    )?;

    Ok(ir)
}

/// Detect Fortran format (free-form or fixed-form) based on sentinel
///
/// - Free-form: `!$omp` or `!$OMP`
/// - Fixed-form: `C$OMP`, `c$omp`, `*$omp` (columns 1-6)
fn detect_fortran_format(input: &str) -> LexerLanguage {
    let trimmed = input.trim_start();

    // Check for free-form sentinel (!$omp or !$OMP)
    if trimmed.starts_with("!$") {
        return LexerLanguage::FortranFree;
    }

    // Check for fixed-form sentinels (C$OMP, c$omp, *$omp)
    if trimmed.starts_with("C$") || trimmed.starts_with("c$") || trimmed.starts_with("*$") {
        return LexerLanguage::FortranFixed;
    }

    // Default to free-form if sentinel is not recognized
    LexerLanguage::FortranFree
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_c_to_fortran_basic() {
        let input = "#pragma omp parallel for";
        let output = translate_c_to_fortran(input).unwrap();
        assert_eq!(output, "!$omp parallel do");
    }

    #[test]
    fn test_translate_c_to_fortran_with_clauses() {
        let input = "#pragma omp parallel for private(i) schedule(static, 4)";
        let output = translate_c_to_fortran(input).unwrap();
        assert_eq!(output, "!$omp parallel do private(i) schedule(static, 4)");
    }

    #[test]
    fn test_translate_c_to_fortran_complex() {
        let input = "#pragma omp target teams distribute parallel for simd collapse(2)";
        let output = translate_c_to_fortran(input).unwrap();
        assert_eq!(
            output,
            "!$omp target teams distribute parallel do simd collapse(2)"
        );
    }

    #[test]
    fn test_translate_c_to_fortran_for_only() {
        let input = "#pragma omp for nowait";
        let output = translate_c_to_fortran(input).unwrap();
        assert_eq!(output, "!$omp do nowait");
    }

    #[test]
    fn test_translate_c_to_fortran_non_loop() {
        let input = "#pragma omp parallel";
        let output = translate_c_to_fortran(input).unwrap();
        assert_eq!(output, "!$omp parallel");
    }

    #[test]
    fn test_translate_fortran_to_c_basic() {
        let input = "!$omp parallel do";
        let output = translate_fortran_to_c(input).unwrap();
        assert_eq!(output, "#pragma omp parallel for");
    }

    #[test]
    fn test_translate_fortran_to_c_with_clauses() {
        let input = "!$omp do schedule(dynamic)";
        let output = translate_fortran_to_c(input).unwrap();
        assert_eq!(output, "#pragma omp for schedule(dynamic)");
    }

    #[test]
    fn test_translate_empty_input() {
        let result = translate_c_to_fortran("");
        assert!(matches!(result, Err(TranslationError::EmptyInput)));

        let result = translate_c_to_fortran("   ");
        assert!(matches!(result, Err(TranslationError::EmptyInput)));
    }

    #[test]
    fn test_translate_invalid_input() {
        let result = translate_c_to_fortran("not a pragma");
        assert!(matches!(result, Err(TranslationError::ParseError(_))));
    }

    #[test]
    fn test_translate_c_to_fortran_ir() {
        let config = ParserConfig::with_parsing(Language::C);
        let ir = translate_c_to_fortran_ir("#pragma omp parallel for", config).unwrap();
        // The IR keeps its original language (C), translation happens at rendering
        assert_eq!(ir.language(), Language::C);
        assert_eq!(
            ir.to_string_for_language(Language::Fortran),
            "!$omp parallel do"
        );
    }

    #[test]
    fn test_translate_fortran_fixed_form_to_c() {
        // Test uppercase fixed-form sentinel (C$OMP)
        let input = "C$OMP PARALLEL DO";
        let output = translate_fortran_to_c(input).unwrap();
        assert_eq!(output, "#pragma omp parallel for");

        // Test lowercase fixed-form sentinel (c$omp)
        let input = "c$omp do schedule(static)";
        let output = translate_fortran_to_c(input).unwrap();
        assert_eq!(output, "#pragma omp for schedule(static)");

        // Test asterisk fixed-form sentinel (*$omp)
        let input = "*$omp parallel";
        let output = translate_fortran_to_c(input).unwrap();
        assert_eq!(output, "#pragma omp parallel");
    }

    #[test]
    fn test_detect_fortran_format() {
        // Free-form sentinels
        assert_eq!(
            detect_fortran_format("!$omp parallel"),
            LexerLanguage::FortranFree
        );
        assert_eq!(
            detect_fortran_format("!$OMP PARALLEL"),
            LexerLanguage::FortranFree
        );
        assert_eq!(
            detect_fortran_format("  !$omp do"),
            LexerLanguage::FortranFree
        );

        // Fixed-form sentinels
        assert_eq!(
            detect_fortran_format("C$OMP PARALLEL"),
            LexerLanguage::FortranFixed
        );
        assert_eq!(
            detect_fortran_format("c$omp do"),
            LexerLanguage::FortranFixed
        );
        assert_eq!(
            detect_fortran_format("*$omp parallel"),
            LexerLanguage::FortranFixed
        );
        assert_eq!(
            detect_fortran_format("  C$OMP PARALLEL"),
            LexerLanguage::FortranFixed
        );
    }
}
