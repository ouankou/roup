//! # OpenMP Intermediate Representation (IR)
//!
//! This module provides a **semantic** representation of OpenMP directives and clauses.
//! Unlike the parser module which deals with syntax, the IR focuses on the **meaning**
//! of the parsed constructs.
//!
//! ## Design Philosophy
//!
//! The IR layer serves as a bridge between the parser (syntax) and compilers (semantics):
//!
//! ```text
//! Input String → Parser → Directive (syntax) → IR → DirectiveIR (semantics) → Compiler
//! ```
//!
//! ## Key Differences: Parser vs IR
//!
//! | Aspect | Parser | IR |
//! |--------|--------|-----|
//! | **Focus** | Syntax preservation | Semantic meaning |
//! | **Clause data** | `"private(a, b)"` as string | List of identifiers `["a", "b"]` |
//! | **Expressions** | Unparsed strings | Optionally parsed AST |
//! | **Validation** | Minimal | Comprehensive |
//! | **Use case** | Parsing | Compilation, analysis |
//!
//! ## Learning Path
//!
//! This module is designed to teach Rust concepts incrementally:
//!
//! 1. **Basic types**: Structs, enums, Copy trait
//! 2. **Advanced enums**: Enums with data, pattern matching
//! 3. **Lifetime management**: References, ownership
//! 4. **Trait implementation**: Display, conversion traits
//! 5. **Error handling**: Result types, custom errors
//! 6. **FFI preparation**: repr(C), opaque types
//!
//! ## Module Organization
//!
//! - `types`: Basic types (SourceLocation, Language, etc.)
//! - `expression`: Expression representation (parsed or unparsed)
//! - `clause_data`: Semantic clause data structures
//! - `directive_ir`: Complete directive representation
//! - `conversion`: Convert parser types to IR
//! - `display`: Pretty-printing IR back to pragmas
//! - `translate`: C/C++ ↔ Fortran directive translation

// Re-export main types
pub use builder::DirectiveBuilder;
pub use clause::{
    AtomicOp, ClauseData, ClauseItem, DefaultKind, DefaultmapBehavior, DefaultmapCategory,
    DependType, DeviceType, LastprivateModifier, LinearModifier, MapType, MemoryOrder, OrderKind,
    ProcBind, ReductionOperator, RequireModifier, ScheduleKind, ScheduleModifier,
    UsesAllocatorBuiltin, UsesAllocatorKind, UsesAllocatorSpec,
};
pub use convert::convert_directive;
pub use directive::{DirectiveIR, DirectiveKind};
pub use error::ConversionError;
pub use expression::{
    BinaryOperator, Expression, ExpressionAst, ExpressionKind, ParserConfig, UnaryOperator,
};
pub use types::{Language, SourceLocation};
pub use validate::{ValidationContext, ValidationError};
pub use variable::{ArraySection, Identifier, Variable};

mod builder;
mod clause;
pub mod convert;
mod directive;
mod error;
mod expression;
mod lang;
pub mod translate;
mod types;
pub mod validate;
mod variable;
