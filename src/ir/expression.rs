//! Expression representation and optional parsing
//!
//! This module provides flexible expression handling:
//! - **Default**: Attempt to parse expressions into structured AST
//! - **Fallback**: Keep expressions as raw strings when parsing fails
//! - **Configurable**: Can disable parsing entirely via ParserConfig
//!
//! ## Learning Objectives
//!
//! - **Enums for alternatives**: Expression can be Parsed OR Unparsed
//! - **Recursive structures**: ExpressionAst contains nested expressions
//! - **Configuration patterns**: ParserConfig controls behavior
//! - **Graceful degradation**: Complex expressions fall back to strings
//! - **Box for indirection**: Breaking recursive type cycles
//!
//! ## Design Philosophy
//!
//! The parser supports C, C++, and Fortran - languages with very different
//! expression syntax. Rather than trying to perfectly parse all expressions,
//! we take a pragmatic approach:
//!
//! 1. Parse common simple patterns (literals, identifiers, binary ops)
//! 2. Fall back to string representation for complex expressions
//! 3. Always preserve the original source text
//! 4. Let the consuming compiler handle language-specific parsing
//!
//! This makes the IR **useful immediately** while allowing incremental
//! improvement of expression parsing over time.

use std::fmt;

use super::Language;

// ============================================================================
// Parser Configuration
// ============================================================================

/// Configuration for IR generation and expression parsing
///
/// This controls how the parser converts syntax to IR, particularly
/// how it handles expressions.
///
/// ## Learning: Configuration Pattern
///
/// Rather than using global state or command-line flags, we pass
/// configuration explicitly. This makes the code:
/// - **Testable**: Easy to test with different configs
/// - **Composable**: Multiple parsers with different configs
/// - **Thread-safe**: No global mutable state
///
/// ## Example
///
/// ```
/// use roup::ir::{ParserConfig, Language};
///
/// // Default: parse expressions
/// let default_config = ParserConfig::default();
/// assert!(default_config.parse_expressions);
///
/// // Custom: disable expression parsing
/// let string_only = ParserConfig {
///     parse_expressions: false,
///     language: Language::C,
///     enable_language_support: true,
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParserConfig {
    /// Whether to attempt parsing expressions into structured form
    ///
    /// - `true` (default): Parse expressions, fall back to string on failure
    /// - `false`: Keep all expressions as raw strings
    pub parse_expressions: bool,

    /// Source language (affects expression parsing rules)
    ///
    /// Different languages have different expression syntax:
    /// - C/C++: `arr[i]`, `*ptr`, `x->y`
    /// - Fortran: `arr(i)`, different operators
    pub language: Language,

    /// Enable best-effort language-specific parsing for clause data
    ///
    /// When enabled, ROUP will attempt to parse variables (including
    /// array sections) and language-specific expressions into the IR.
    /// When disabled, parsing falls back to simple identifier strings.
    pub enable_language_support: bool,
}

impl ParserConfig {
    /// Create a new configuration
    pub const fn new(parse_expressions: bool, language: Language) -> Self {
        Self {
            parse_expressions,
            language,
            enable_language_support: true,
        }
    }

    /// Create config that keeps all expressions as strings
    pub const fn string_only(language: Language) -> Self {
        Self {
            parse_expressions: false,
            language,
            enable_language_support: true,
        }
    }

    /// Create config that parses expressions
    pub const fn with_parsing(language: Language) -> Self {
        Self {
            parse_expressions: true,
            language,
            enable_language_support: true,
        }
    }

    /// Return a new configuration with a different source language
    pub fn with_language(mut self, language: Language) -> Self {
        self.language = language;
        self
    }

    /// Enable or disable language-specific parsing support
    pub fn with_language_support(mut self, enabled: bool) -> Self {
        self.enable_language_support = enabled;
        self
    }

    /// Check if language support is enabled
    pub const fn language_support_enabled(&self) -> bool {
        self.enable_language_support
    }
}

impl Default for ParserConfig {
    /// Default: parse expressions, unknown language
    fn default() -> Self {
        Self {
            parse_expressions: true,
            language: Language::Unknown,
            enable_language_support: true,
        }
    }
}

// ============================================================================
// Expression Types
// ============================================================================

/// An expression that may be parsed or unparsed
///
/// This is the core type for representing expressions in the IR.
/// It gracefully handles both structured and unstructured forms.
///
/// ## Learning: Enums for Polymorphism
///
/// Instead of inheritance (like in C++), Rust uses enums to represent
/// "one of several types". This is more explicit and type-safe.
///
/// ## Learning: Box for Recursion
///
/// The `Parsed` variant contains `Box<ExpressionAst>` instead of
/// `ExpressionAst` directly. Why? Because `ExpressionAst` itself
/// contains `Expression` values (recursion!).
///
/// Without `Box`, the type would have infinite size. `Box` provides
/// indirection through a heap pointer, breaking the cycle.
///
/// ## Example
///
/// ```
/// use roup::ir::{Expression, ParserConfig};
///
/// let config = ParserConfig::default();
///
/// // Simple expression gets parsed
/// let simple = Expression::new("42", &config);
/// assert!(simple.is_parsed());
///
/// // Complex expression falls back to string
/// let complex = Expression::new("sizeof(struct foo)", &config);
/// // May or may not be parsed depending on parser capability
///
/// // With parsing disabled, always unparsed
/// let config_no_parse = ParserConfig::string_only(roup::ir::Language::C);
/// let expr = Expression::new("N * 2", &config_no_parse);
/// assert!(!expr.is_parsed());
/// assert_eq!(expr.as_str(), "N * 2");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// Expression was successfully parsed into structured form
    ///
    /// The compiler can analyze the AST structure for optimization,
    /// validation, or transformation.
    Parsed(Box<ExpressionAst>),

    /// Expression kept as raw string
    ///
    /// This happens when:
    /// - Expression parsing is disabled
    /// - Expression is too complex for the parser
    /// - Parser doesn't support this language construct yet
    ///
    /// The compiler must parse this string according to the source language.
    Unparsed(String),
}

impl Expression {
    /// Create a new expression, attempting to parse if enabled
    ///
    /// ## Example
    ///
    /// ```
    /// use roup::ir::{Expression, ParserConfig, Language};
    ///
    /// let config = ParserConfig::default();
    /// let expr = Expression::new("100", &config);
    /// assert_eq!(expr.as_str(), "100");
    /// ```
    pub fn new(raw: impl Into<String>, config: &ParserConfig) -> Self {
        let raw = raw.into();
        let trimmed = raw.trim().to_string();

        // If parsing disabled, return unparsed
        if !config.parse_expressions {
            return Expression::Unparsed(trimmed);
        }

        // Try to parse based on language
        match parse_expression(&trimmed, config.language) {
            Ok(ast) => Expression::Parsed(Box::new(ast)),
            Err(_) => Expression::Unparsed(trimmed),
        }
    }

    /// Create an unparsed expression directly
    ///
    /// Useful when you know parsing will fail or you want to bypass it.
    pub fn unparsed(raw: impl Into<String>) -> Self {
        Expression::Unparsed(raw.into())
    }

    /// Get the raw string representation
    ///
    /// This always works, whether the expression is parsed or not.
    /// The original source is always preserved.
    pub fn as_str(&self) -> &str {
        match self {
            Expression::Parsed(ast) => &ast.original_source,
            Expression::Unparsed(s) => s,
        }
    }

    /// Check if expression was successfully parsed
    pub const fn is_parsed(&self) -> bool {
        matches!(self, Expression::Parsed(_))
    }

    /// Get the parsed AST if available
    pub fn as_ast(&self) -> Option<&ExpressionAst> {
        match self {
            Expression::Parsed(ast) => Some(ast),
            Expression::Unparsed(_) => None,
        }
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// Expression AST (Structured Representation)
// ============================================================================

/// Parsed expression abstract syntax tree
///
/// This represents common expression patterns found in OpenMP directives.
/// It's **not** a complete C/C++/Fortran parser, just enough to handle
/// typical OpenMP expressions.
///
/// ## Learning: Recursive Data Structures
///
/// Notice that `ExpressionKind` contains `Box<ExpressionAst>` in several
/// variants. This allows representing nested expressions like:
/// - `(a + b) * c` - BinaryOp containing another BinaryOp
/// - `arr[i][j]` - ArrayAccess containing another ArrayAccess
///
/// ## Example
///
/// ```
/// use roup::ir::{Expression, ParserConfig};
///
/// let config = ParserConfig::default();
/// let expr = Expression::new("42", &config);
///
/// if let Some(ast) = expr.as_ast() {
///     // Can inspect the AST structure
///     println!("Original: {}", ast.original_source);
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ExpressionAst {
    /// Original source text (always preserved)
    pub original_source: String,

    /// Parsed structure (best-effort)
    pub kind: ExpressionKind,
}

/// Common expression patterns in OpenMP directives
///
/// ## Learning: Large Enums with Data
///
/// This enum demonstrates Rust's powerful enum system. Each variant
/// can carry different data:
/// - `IntLiteral(i64)` - carries an integer
/// - `Identifier(String)` - carries an owned string
/// - `BinaryOp { ... }` - carries multiple fields
///
/// This is much more powerful than C enums, which can only be simple tags.
#[derive(Debug, Clone, PartialEq)]
pub enum ExpressionKind {
    /// Integer literal: `42`, `0x10`, `0b1010`
    IntLiteral(i64),

    /// Identifier: `N`, `num_threads`, `my_var`
    Identifier(String),

    /// Binary operation: `a + b`, `N * 2`, `i < 10`
    BinaryOp {
        left: Box<ExpressionAst>,
        op: BinaryOperator,
        right: Box<ExpressionAst>,
    },

    /// Unary operation: `-x`, `!flag`, `*ptr`
    UnaryOp {
        op: UnaryOperator,
        operand: Box<ExpressionAst>,
    },

    /// Function call: `foo(a, b)`, `omp_get_num_threads()`
    Call {
        function: String,
        args: Vec<ExpressionAst>,
    },

    /// Array subscript: `arr[i]`, `matrix[i][j]`
    ArrayAccess {
        array: Box<ExpressionAst>,
        indices: Vec<ExpressionAst>,
    },

    /// Ternary conditional: `cond ? a : b`
    Conditional {
        condition: Box<ExpressionAst>,
        then_expr: Box<ExpressionAst>,
        else_expr: Box<ExpressionAst>,
    },

    /// Parenthesized: `(expr)`
    Parenthesized(Box<ExpressionAst>),

    /// Too complex to parse, kept as string
    ///
    /// This is our escape hatch for expressions that are valid
    /// but not yet supported by the parser.
    Complex(String),
}

/// Binary operators
///
/// ## Learning: repr(C) for C Interop
///
/// We use `#[repr(C)]` so these enum values are compatible with C code.
/// Each variant gets an explicit numeric value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub enum BinaryOperator {
    // Arithmetic
    Add = 0,
    Sub = 1,
    Mul = 2,
    Div = 3,
    Mod = 4,

    // Comparison
    Eq = 10,
    Ne = 11,
    Lt = 12,
    Le = 13,
    Gt = 14,
    Ge = 15,

    // Logical
    And = 20,
    Or = 21,

    // Bitwise
    BitwiseAnd = 30,
    BitwiseOr = 31,
    BitwiseXor = 32,
    ShiftLeft = 33,
    ShiftRight = 34,
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub enum UnaryOperator {
    Negate = 0,     // -x
    LogicalNot = 1, // !x
    BitwiseNot = 2, // ~x
    Deref = 3,      // *ptr (C/C++)
    AddressOf = 4,  // &var (C/C++)
}

// ============================================================================
// Expression Parser (Isolated, Configurable)
// ============================================================================

/// Error type for expression parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
}

/// Parse an expression string into an AST
///
/// This is **isolated** and can be disabled via config.
/// Returns `Err` if expression is too complex or language-specific.
///
/// ## Learning: Error Handling with Result
///
/// Rust doesn't have exceptions. Instead, functions that can fail
/// return `Result<T, E>`:
/// - `Ok(value)` - success
/// - `Err(error)` - failure
///
/// The caller must handle both cases (checked at compile time!).
fn parse_expression(input: &str, language: Language) -> Result<ExpressionAst, ParseError> {
    match language {
        Language::C | Language::Cpp => parse_c_expression(input),
        Language::Fortran => parse_fortran_expression(input),
        Language::Unknown => parse_generic_expression(input),
    }
}

/// Parse C/C++ expression
///
/// Currently falls back to generic parser. In the future, this could
/// handle C/C++-specific constructs like `->`, `sizeof`, etc.
fn parse_c_expression(input: &str) -> Result<ExpressionAst, ParseError> {
    parse_generic_expression(input)
}

/// Parse Fortran expression
///
/// Currently falls back to generic parser. In the future, this could
/// handle Fortran-specific constructs.
fn parse_fortran_expression(input: &str) -> Result<ExpressionAst, ParseError> {
    parse_generic_expression(input)
}

/// Parse simple, language-agnostic expressions
///
/// This handles the most common patterns:
/// - Integer literals: `42`
/// - Identifiers: `N`, `my_var`
/// - Everything else: marked as `Complex`
///
/// This is intentionally simple. Complex parsing can be added later
/// without changing the IR structure.
fn parse_generic_expression(input: &str) -> Result<ExpressionAst, ParseError> {
    let trimmed = input.trim();

    // Try to parse as integer literal
    if let Ok(value) = trimmed.parse::<i64>() {
        return Ok(ExpressionAst {
            original_source: input.to_string(),
            kind: ExpressionKind::IntLiteral(value),
        });
    }

    // Try to parse as identifier
    if is_simple_identifier(trimmed) {
        return Ok(ExpressionAst {
            original_source: input.to_string(),
            kind: ExpressionKind::Identifier(trimmed.to_string()),
        });
    }

    // For everything else, mark as complex
    // The consuming compiler will parse it
    Ok(ExpressionAst {
        original_source: input.to_string(),
        kind: ExpressionKind::Complex(trimmed.to_string()),
    })
}

/// Check if a string is a simple identifier
///
/// An identifier must:
/// - Start with letter or underscore
/// - Contain only letters, digits, or underscores
fn is_simple_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    let mut chars = s.chars();
    let first = chars.next().unwrap();

    // First character must be letter or underscore
    if !first.is_alphabetic() && first != '_' {
        return false;
    }

    // Remaining characters must be alphanumeric or underscore
    chars.all(|c| c.is_alphanumeric() || c == '_')
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------------
    // ParserConfig tests
    // ------------------------------------------------------------------------

    #[test]
    fn parser_config_default_enables_parsing() {
        let config = ParserConfig::default();
        assert!(config.parse_expressions);
        assert_eq!(config.language, Language::Unknown);
    }

    #[test]
    fn parser_config_string_only_disables_parsing() {
        let config = ParserConfig::string_only(Language::C);
        assert!(!config.parse_expressions);
        assert_eq!(config.language, Language::C);
    }

    #[test]
    fn parser_config_with_parsing_enables_parsing() {
        let config = ParserConfig::with_parsing(Language::Fortran);
        assert!(config.parse_expressions);
        assert_eq!(config.language, Language::Fortran);
    }

    // ------------------------------------------------------------------------
    // Expression tests
    // ------------------------------------------------------------------------

    #[test]
    fn expression_new_parses_integer_literal() {
        let config = ParserConfig::default();
        let expr = Expression::new("42", &config);

        assert!(expr.is_parsed());
        assert_eq!(expr.as_str(), "42");

        if let Some(ast) = expr.as_ast() {
            assert!(matches!(ast.kind, ExpressionKind::IntLiteral(42)));
        } else {
            panic!("Should be parsed");
        }
    }

    #[test]
    fn expression_new_parses_identifier() {
        let config = ParserConfig::default();
        let expr = Expression::new("my_var", &config);

        assert!(expr.is_parsed());
        assert_eq!(expr.as_str(), "my_var");

        if let Some(ast) = expr.as_ast() {
            if let ExpressionKind::Identifier(name) = &ast.kind {
                assert_eq!(name, "my_var");
            } else {
                panic!("Should be identifier");
            }
        }
    }

    #[test]
    fn expression_new_handles_complex_as_complex() {
        let config = ParserConfig::default();
        let expr = Expression::new("a + b * c", &config);

        // Should parse but as Complex kind
        if let Some(ast) = expr.as_ast() {
            assert!(matches!(ast.kind, ExpressionKind::Complex(_)));
        }
    }

    #[test]
    fn expression_with_parsing_disabled_stays_unparsed() {
        let config = ParserConfig::string_only(Language::C);
        let expr = Expression::new("42", &config);

        assert!(!expr.is_parsed());
        assert_eq!(expr.as_str(), "42");
        assert!(expr.as_ast().is_none());
    }

    #[test]
    fn expression_unparsed_creates_unparsed() {
        let expr = Expression::unparsed("anything");

        assert!(!expr.is_parsed());
        assert_eq!(expr.as_str(), "anything");
    }

    #[test]
    fn expression_preserves_original_source() {
        let config = ParserConfig::default();
        let expr = Expression::new("  42  ", &config);

        // Trimmed version is used
        assert_eq!(expr.as_str(), "42");
    }

    #[test]
    fn expression_display_shows_source() {
        let expr = Expression::unparsed("N * 2");
        assert_eq!(format!("{}", expr), "N * 2");
    }

    // ------------------------------------------------------------------------
    // ExpressionAst tests
    // ------------------------------------------------------------------------

    #[test]
    fn parse_generic_expression_handles_integers() {
        let result = parse_generic_expression("123").unwrap();
        assert_eq!(result.original_source, "123");
        assert!(matches!(result.kind, ExpressionKind::IntLiteral(123)));
    }

    #[test]
    fn parse_generic_expression_handles_negative_integers() {
        let result = parse_generic_expression("-456").unwrap();
        // Negative integers are actually parsed successfully by parse::<i64>()
        assert!(matches!(result.kind, ExpressionKind::IntLiteral(-456)));
    }

    #[test]
    fn parse_generic_expression_handles_identifiers() {
        let result = parse_generic_expression("num_threads").unwrap();
        if let ExpressionKind::Identifier(name) = result.kind {
            assert_eq!(name, "num_threads");
        } else {
            panic!("Should be identifier");
        }
    }

    #[test]
    fn parse_generic_expression_handles_complex() {
        let result = parse_generic_expression("a + b").unwrap();
        if let ExpressionKind::Complex(s) = result.kind {
            assert_eq!(s, "a + b");
        } else {
            panic!("Should be complex");
        }
    }

    // ------------------------------------------------------------------------
    // Helper function tests
    // ------------------------------------------------------------------------

    #[test]
    fn is_simple_identifier_accepts_valid_identifiers() {
        assert!(is_simple_identifier("x"));
        assert!(is_simple_identifier("my_var"));
        assert!(is_simple_identifier("_private"));
        assert!(is_simple_identifier("var123"));
        assert!(is_simple_identifier("CamelCase"));
    }

    #[test]
    fn is_simple_identifier_rejects_invalid() {
        assert!(!is_simple_identifier(""));
        assert!(!is_simple_identifier("123var")); // starts with digit
        assert!(!is_simple_identifier("my-var")); // contains hyphen
        assert!(!is_simple_identifier("my var")); // contains space
        assert!(!is_simple_identifier("my+var")); // contains operator
    }

    // ------------------------------------------------------------------------
    // Binary and Unary Operator tests
    // ------------------------------------------------------------------------

    #[test]
    fn binary_operator_has_correct_discriminants() {
        assert_eq!(BinaryOperator::Add as u32, 0);
        assert_eq!(BinaryOperator::Eq as u32, 10);
        assert_eq!(BinaryOperator::And as u32, 20);
        assert_eq!(BinaryOperator::BitwiseAnd as u32, 30);
    }

    #[test]
    fn unary_operator_has_correct_discriminants() {
        assert_eq!(UnaryOperator::Negate as u32, 0);
        assert_eq!(UnaryOperator::LogicalNot as u32, 1);
        assert_eq!(UnaryOperator::AddressOf as u32, 4);
    }
}
