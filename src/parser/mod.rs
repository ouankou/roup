/// Parser module for OpenMP directives
///
/// Learning Rust: Module System
/// =============================
/// Modules organize code into logical units
/// - mod.rs is the entry point for the 'parser' module
/// - Can have submodules in separate files
/// - Controls visibility with pub keyword

mod clause;
mod directive;

// Re-export types for convenience
pub use clause::{Clause, ClauseKind};
pub use directive::Directive;

// Learning Rust: pub vs private
// ==============================
// - 'pub' makes items visible outside the module
// - Without 'pub', items are private to the module
// - 'pub use' re-exports items from submodules
//   This lets users do: use roup::parser::Clause
//   Instead of: use roup::parser::clause::Clause
