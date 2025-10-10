// ROUP: Rust-based OpenMP/OpenACC Unified Parser
// This is the entry point for the library

/// Learning Rust: Module Declaration
/// ==================================
/// Declare submodules with 'pub mod'
/// The actual code lives in parser/mod.rs (or parser.rs)
pub mod parser;

// Re-export commonly used types for convenience
pub use parser::{Clause, ClauseKind};
