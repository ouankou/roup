//! Step-by-step parser debugger for educational and debugging purposes
//!
//! This module provides an interactive debugging interface that allows users to step through
//! the parsing process token by token, seeing exactly what the parser is doing at each step.

mod ast_display;
mod stepper;
mod ui;

pub use ast_display::display_ast_tree;
pub use stepper::{DebugSession, DebugStep, StepKind};
pub use ui::{run_interactive_session, run_non_interactive, UserCommand};

use crate::lexer::Language;
use crate::parser::Dialect;

/// Result type for debugger operations
pub type DebugResult<T> = Result<T, DebugError>;

/// Errors that can occur during debugging
#[derive(Debug)]
pub enum DebugError {
    /// Parser failed to parse the input
    ParseError(String),
    /// Input/output error
    IoError(std::io::Error),
    /// Invalid input
    InvalidInput(String),
}

impl std::fmt::Display for DebugError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DebugError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            DebugError::IoError(e) => write!(f, "I/O error: {}", e),
            DebugError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
        }
    }
}

impl std::error::Error for DebugError {}

impl From<std::io::Error> for DebugError {
    fn from(e: std::io::Error) -> Self {
        DebugError::IoError(e)
    }
}

/// Configuration for the debug session
#[derive(Debug, Clone)]
pub struct DebugConfig {
    pub dialect: Dialect,
    pub language: Language,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            dialect: Dialect::OpenMp,
            language: Language::C,
        }
    }
}

impl DebugConfig {
    pub fn new(dialect: Dialect, language: Language) -> Self {
        Self { dialect, language }
    }

    pub fn openmp() -> Self {
        Self {
            dialect: Dialect::OpenMp,
            language: Language::C,
        }
    }

    pub fn openacc() -> Self {
        Self {
            dialect: Dialect::OpenAcc,
            language: Language::C,
        }
    }
}
