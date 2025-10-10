use std::fmt;

/// Learning Rust: Collections - HashMap
/// =====================================
/// HashMap is Rust's hash table (like unordered_map in C++)
/// - Key-value pairs
/// - O(1) average lookup
/// - Not in prelude, must import from std::collections
use std::collections::HashMap;

/// Represents the different kinds of clauses in OpenMP directives
#[derive(Debug, PartialEq, Eq)]
pub enum ClauseKind<'a> {
    /// A clause without parameters, e.g., "nowait"
    Bare,
    /// A clause with parenthesized content, e.g., "private(a, b)"
    Parenthesized(&'a str),
}

/// Represents a single clause in an OpenMP directive
#[derive(Debug, PartialEq, Eq)]
pub struct Clause<'a> {
    /// The name of the clause (e.g., "private", "nowait")
    pub name: &'a str,
    /// The kind/type of this clause
    pub kind: ClauseKind<'a>,
}

impl<'a> Clause<'a> {
    /// Creates a new bare clause (no parameters)
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

impl<'a> fmt::Display for Clause<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            ClauseKind::Bare => write!(f, "{}", self.name),
            ClauseKind::Parenthesized(value) => write!(f, "{}({})", self.name, value),
        }
    }
}

/// A registry that stores parsing rules for different clause types
///
/// Learning Rust: Owned vs Borrowed Data
/// ======================================
/// HashMap<&'static str, ClauseRule>
/// - Keys are &'static str (string literals - live forever)
/// - Values are &'static str descriptions (owned by the HashMap)
/// - The HashMap owns its data and will clean up when dropped
pub struct ClauseRegistry {
    rules: HashMap<&'static str, &'static str>,
}

impl ClauseRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        ClauseRegistry {
            rules: HashMap::new(),
        }
    }

    /// Register a clause name with a description
    ///
    /// Learning Rust: Mutable Methods
    /// ===============================
    /// &mut self allows the method to modify the struct
    /// Required for HashMap::insert which changes the map
    pub fn register(&mut self, name: &'static str, description: &'static str) {
        self.rules.insert(name, description);
    }

    /// Check if a clause is registered
    ///
    /// Learning Rust: Option Type
    /// ==========================
    /// HashMap::get returns Option<&V>:
    /// - Some(&value) if key exists
    /// - None if key doesn't exist
    /// 
    /// Option is Rust's way of handling "nullable" values safely!
    /// No null pointer exceptions - compiler forces you to handle None
    pub fn get_description(&self, name: &str) -> Option<&str> {
        // Learning Rust: Copying vs Moving
        // =================================
        // get() returns Option<&&str>, map() converts to Option<&str>
        // The * dereferences to copy the &str (which is just 2 words)
        self.rules.get(name).map(|&desc| desc)
    }

    /// Check if a clause exists in the registry
    pub fn contains(&self, name: &str) -> bool {
        self.rules.contains_key(name)
    }

    /// Get the number of registered clauses
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Create a builder to construct a ClauseRegistry fluently
    ///
    /// Learning Rust: Builder Pattern
    /// ===============================
    /// Builder pattern creates complex objects step-by-step
    /// - Separates construction from representation
    /// - Enables method chaining for fluent API
    /// - Common in Rust (e.g., std::process::Command)
    pub fn builder() -> ClauseRegistryBuilder {
        ClauseRegistryBuilder::new()
    }
}

/// Builder for constructing a ClauseRegistry
///
/// Learning Rust: Builder Pattern Implementation
/// ==============================================
/// The builder:
/// 1. Accumulates configuration
/// 2. Provides a fluent interface (method chaining)
/// 3. build() consumes the builder and creates the final object
pub struct ClauseRegistryBuilder {
    rules: HashMap<&'static str, &'static str>,
}

impl ClauseRegistryBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        ClauseRegistryBuilder {
            rules: HashMap::new(),
        }
    }

    /// Register a clause (returns Self for chaining)
    ///
    /// Learning Rust: Method Chaining
    /// ===============================
    /// Returning 'Self' allows: builder.register(...).register(...)
    /// This is more ergonomic than calling register() multiple times
    pub fn register(mut self, name: &'static str, description: &'static str) -> Self {
        self.rules.insert(name, description);
        self // Return self for chaining
    }

    /// Build the final ClauseRegistry
    ///
    /// Learning Rust: Consuming Methods
    /// =================================
    /// Takes 'self' (not &self) - consumes the builder
    /// After build(), the builder is gone (moved)
    /// Prevents using the builder after construction
    pub fn build(self) -> ClauseRegistry {
        ClauseRegistry {
            rules: self.rules,
        }
    }
}

/// Learning Rust: Default Trait
/// =============================
/// Default trait provides a default constructor
/// Enables: ClauseRegistryBuilder::default()
/// Required by some APIs
impl Default for ClauseRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_bare_clause() {
        let clause = Clause::bare("nowait");
        assert_eq!(clause.name, "nowait");
        assert_eq!(clause.kind, ClauseKind::Bare);
    }

    #[test]
    fn creates_parenthesized_clause() {
        let clause = Clause::parenthesized("private", "a, b");
        assert_eq!(clause.name, "private");
        if let ClauseKind::Parenthesized(value) = clause.kind {
            assert_eq!(value, "a, b");
        } else {
            panic!("Expected Parenthesized clause");
        }
    }

    #[test]
    fn display_formats_clauses() {
        assert_eq!(Clause::bare("nowait").to_string(), "nowait");
        assert_eq!(
            Clause::parenthesized("private", "x, y").to_string(),
            "private(x, y)"
        );
    }

    #[test]
    fn creates_empty_registry() {
        let registry = ClauseRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn registers_and_retrieves_clauses() {
        let mut registry = ClauseRegistry::new();
        
        // Learning Rust: Method Chaining with Mutation
        // =============================================
        // Each register() call mutates the registry
        registry.register("private", "Data privatization clause");
        registry.register("shared", "Data sharing clause");
        
        assert_eq!(registry.len(), 2);
        assert!(registry.contains("private"));
        assert!(registry.contains("shared"));
        assert!(!registry.contains("unknown"));
        
        // Learning Rust: Pattern Matching on Option
        // ==========================================
        // Must handle both Some and None cases
        match registry.get_description("private") {
            Some(desc) => assert_eq!(desc, "Data privatization clause"),
            None => panic!("Expected to find 'private' clause"),
        }
        
        // Using if let for single case
        if let Some(desc) = registry.get_description("shared") {
            assert_eq!(desc, "Data sharing clause");
        }
        
        // None case
        assert_eq!(registry.get_description("nonexistent"), None);
    }

    #[test]
    fn builder_creates_registry_fluently() {
        // Learning Rust: Fluent Builder Pattern
        // ======================================
        // Method chaining creates readable configuration
        let registry = ClauseRegistry::builder()
            .register("private", "Privatize variables")
            .register("shared", "Share variables")
            .register("nowait", "Don't wait for completion")
            .build();

        assert_eq!(registry.len(), 3);
        assert!(registry.contains("private"));
        assert!(registry.contains("shared"));
        assert!(registry.contains("nowait"));
        
        // Learning Rust: Ownership After build()
        // =======================================
        // After build(), the builder is consumed (moved)
        // Can't use it again - compiler prevents this!
        // This is enforced at compile time - no runtime cost!
    }

    #[test]
    fn builder_can_be_created_via_default() {
        // Default trait enables using ::default()
        let registry = ClauseRegistryBuilder::default()
            .register("reduction", "Reduction operation")
            .build();
        
        assert_eq!(registry.len(), 1);
        assert!(registry.contains("reduction"));
    }
}

