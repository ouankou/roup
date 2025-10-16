//! Error types shared across IR conversion stages.
//!
//! This module centralizes error definitions so they can be reused by
//! language-specific helpers and the conversion pipeline.
//!
//! ## Design goals
//!
//! - **Single source of truth** for conversion errors
//! - **Reusability** across submodules (e.g. language frontends)
//! - **Friendly Display implementation** for diagnostics

/// Error type for conversion failures when building the IR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversionError {
    /// Unknown directive name encountered during conversion.
    UnknownDirective(String),
    /// Unknown clause name encountered during conversion.
    UnknownClause(String),
    /// Clause text could not be interpreted according to the spec.
    InvalidClauseSyntax(String),
    /// Feature recognised by the parser but not yet supported in the IR.
    Unsupported(String),
}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConversionError::UnknownDirective(name) => {
                write!(f, "Unknown directive: {name}")
            }
            ConversionError::UnknownClause(name) => {
                write!(f, "Unknown clause: {name}")
            }
            ConversionError::InvalidClauseSyntax(msg) => {
                write!(f, "Invalid clause syntax: {msg}")
            }
            ConversionError::Unsupported(msg) => {
                write!(f, "Unsupported feature: {msg}")
            }
        }
    }
}

impl std::error::Error for ConversionError {}
