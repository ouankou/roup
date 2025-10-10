// ROUP: Rust-based OpenMP/OpenACC Unified Parser
// This is the entry point for the library

/// Learning Rust: Standard Library Imports
/// ========================================
/// 'use' brings items into scope
/// std::fmt - formatting and display traits
use std::fmt;

/// Represents the different kinds of clauses in OpenMP directives
/// 
/// Learning Rust: Enums
/// ====================
/// Enums in Rust can hold data! This is more powerful than C/C++ enums.
/// Each variant can have different associated data.
/// 
/// Learning Rust: Lifetimes
/// ========================
/// The <'a> is a lifetime parameter. It tells Rust how long references live.
/// 
/// Why? Rust prevents dangling pointers at compile time!
/// - &'a str means "a reference to a string that lives for lifetime 'a"
/// - All &'a references in this struct must live at least as long as 'a
/// - The compiler checks this - no runtime overhead!
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
/// 
/// Learning Rust: String Slices (&str)
/// ====================================
/// &str is a "string slice" - a view into string data owned elsewhere
/// - Doesn't own the data (no allocation/deallocation)
/// - Just a pointer + length
/// - Extremely efficient for parsing! We can reference parts of input string
/// 
/// Example: If input is "#pragma omp parallel private(x)"
/// - directive.name would be a slice pointing to "parallel" in the input
/// - No copying needed! Zero-cost abstraction!
/// 
/// Vec<Clause<'a>> is a growable array
/// - Vec owns its data (unlike slices)
/// - Can push/pop elements
/// - All Clause references must live for lifetime 'a
#[derive(Debug, PartialEq, Eq)]
pub struct Directive<'a> {
    /// The name of the directive (e.g., "parallel", "for")
    pub name: &'a str,
    /// List of clauses attached to this directive
    pub clauses: Vec<Clause<'a>>,
}

/// Learning Rust: Methods (impl blocks)
/// =====================================
/// Use 'impl' blocks to add methods to structs
/// - Methods can borrow self (&self), mutate (&mut self), or consume (self)
/// - &self is like 'this' in C++, but explicit!
impl<'a> Clause<'a> {
    /// Creates a new bare clause (no parameters)
    /// 
    /// Learning Rust: Associated Functions
    /// ====================================
    /// Functions without 'self' are associated functions (like static methods)
    /// Called via Clause::bare("nowait"), not instance.bare()
    pub fn bare(name: &'a str) -> Self {
        Clause {
            name,
            kind: ClauseKind::Bare,
        }
    }

    /// Creates a new parenthesized clause
    pub fn parenthesized(name: &'a str, value: &'a str) -> Self {
        Clause {
            name,
            kind: ClauseKind::Parenthesized(value),
        }
    }
}

/// Learning Rust: Trait Implementation (Display)
/// ==============================================
/// Traits are like interfaces - they define behavior
/// Display trait enables formatted output with {}
/// Debug (from #[derive(Debug)]) uses {:?}
impl<'a> fmt::Display for Clause<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Learning Rust: Pattern Matching in Practice
        // ============================================
        // Match on self.kind to decide how to format
        match self.kind {
            ClauseKind::Bare => write!(f, "{}", self.name),
            ClauseKind::Parenthesized(value) => write!(f, "{}({})", self.name, value),
        }
    }
}

impl<'a> fmt::Display for Directive<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#pragma omp {}", self.name)?;
        
        // Learning Rust: Iterators
        // ========================
        // for loops iterate over anything that implements Iterator
        // enumerate() adds index to iteration
        // ? operator propagates errors (returns early if Err)
        for (idx, clause) in self.clauses.iter().enumerate() {
            if idx == 0 {
                write!(f, " ")?;
            } else {
                write!(f, " ")?;
            }
            write!(f, "{}", clause)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Learning Rust: Unit Tests
    /// ==========================
    /// #[cfg(test)] means this module only compiles during testing
    /// #[test] marks a function as a test
    /// Tests live alongside code - encouraged in Rust!
    
    #[test]
    fn creates_bare_clause() {
        let clause = Clause::bare("nowait");
        assert_eq!(clause.name, "nowait");
        
        // Learning Rust: Pattern Matching with match
        // ===========================================
        // Match is like switch but EXHAUSTIVE - must handle all cases!
        // The compiler ensures you don't miss a case
        match clause.kind {
            ClauseKind::Bare => {
                // Success! This is what we expect
            }
            ClauseKind::Parenthesized(_) => {
                panic!("Expected Bare, got Parenthesized");
            }
        }
    }

    #[test]
    fn creates_parenthesized_clause() {
        let clause = Clause::parenthesized("private", "a, b");
        assert_eq!(clause.name, "private");
        
        // Learning Rust: Pattern Matching with if let
        // ============================================
        // When you only care about one variant, use 'if let'
        // More concise than match when you have one case
        if let ClauseKind::Parenthesized(value) = clause.kind {
            assert_eq!(value, "a, b");
        } else {
            panic!("Expected Parenthesized clause");
        }
    }

    #[test]
    fn creates_directive_with_clauses() {
        // Learning Rust: Vec Literals
        // ============================
        // vec! macro creates a Vec from a list of elements
        let directive = Directive {
            name: "parallel",
            clauses: vec![
                Clause::bare("nowait"),
                Clause::parenthesized("private", "x"),
            ],
        };

        assert_eq!(directive.name, "parallel");
        assert_eq!(directive.clauses.len(), 2);
        
        // Learning Rust: Indexing
        // =======================
        // Vec can be indexed like arrays
        // But be careful - panics if index out of bounds!
        assert_eq!(directive.clauses[0].name, "nowait");
        assert_eq!(directive.clauses[1].name, "private");
    }

    #[test]
    fn display_formats_bare_clause() {
        let clause = Clause::bare("nowait");
        // Learning Rust: to_string() and Display
        // =======================================
        // Any type implementing Display gets to_string() for free
        assert_eq!(clause.to_string(), "nowait");
    }

    #[test]
    fn display_formats_parenthesized_clause() {
        let clause = Clause::parenthesized("private", "a, b");
        assert_eq!(clause.to_string(), "private(a, b)");
    }

    #[test]
    fn display_formats_complete_directive() {
        let directive = Directive {
            name: "parallel",
            clauses: vec![
                Clause::parenthesized("private", "x, y"),
                Clause::bare("nowait"),
            ],
        };
        assert_eq!(
            directive.to_string(),
            "#pragma omp parallel private(x, y) nowait"
        );
    }
}

