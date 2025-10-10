// ROUP: Rust-based OpenMP/OpenACC Unified Parser
// This is the entry point for the library

/// Represents the different kinds of clauses in OpenMP directives
/// 
/// Learning Rust: Enums
/// ====================
/// Enums in Rust can hold data! This is more powerful than C/C++ enums.
/// Each variant can have different associated data.
#[derive(Debug, PartialEq, Eq)]
pub enum ClauseKind<'a> {
    /// A clause without parameters, e.g., "nowait"
    Bare,
    /// A clause with parenthesized content, e.g., "private(a, b)"
    /// The &'a str is a string slice (borrowed reference) with lifetime 'a
    Parenthesized(&'a str),
}

/// Represents a single clause in an OpenMP directive
///
/// Learning Rust: Structs
/// ======================
/// Structs group related data together (like C structs)
/// But in Rust, they can have methods, traits, and lifetimes
#[derive(Debug, PartialEq, Eq)]
pub struct Clause<'a> {
    /// The name of the clause (e.g., "private", "nowait")
    pub name: &'a str,
    /// The kind/type of this clause
    pub kind: ClauseKind<'a>,
}

/// Represents a complete OpenMP directive
#[derive(Debug, PartialEq, Eq)]
pub struct Directive<'a> {
    /// The name of the directive (e.g., "parallel", "for")
    pub name: &'a str,
    /// List of clauses attached to this directive
    pub clauses: Vec<Clause<'a>>,
}

