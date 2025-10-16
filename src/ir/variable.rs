//! Variable and identifier representation
//!
//! This module defines types for representing identifiers and variables
//! in OpenMP clauses. The key distinction:
//!
//! - **Identifier**: Simple name (e.g., `my_var`, `omp_default_mem_alloc`)
//! - **Variable**: Name with optional array sections (e.g., `arr[0:N]`, `mat[i][j:k]`)
//!
//! ## Learning Objectives
//!
//! - **Nested structures**: Variable contains Vec of ArraySection
//! - **Composition**: ArraySection uses Expression type
//! - **Semantic clarity**: Types document intent
//! - **String slices**: Using `&'a str` for zero-copy references
//!
//! ## Design Philosophy
//!
//! OpenMP clauses often work with variables that may be:
//! 1. Simple scalars: `private(x, y, z)`
//! 2. Array sections: `map(to: arr[0:N])`
//! 3. Struct members: `private(point.x)` (not yet supported)
//!
//! We model this with clear types that preserve the original syntax
//! while providing semantic structure.

use std::fmt;

use super::Expression;

// ============================================================================
// Identifier: Simple names
// ============================================================================

/// A simple identifier (not an expression, not a variable with sections)
///
/// Used for names that appear in various contexts:
/// - Variable names: `x`, `my_var`
/// - Function names: `my_function`
/// - Allocator names: `omp_default_mem_alloc`
/// - Mapper names: `my_mapper`
/// - User-defined reduction operators: `my_reduction_op`
///
/// ## Learning: Newtype Pattern
///
/// This is a "newtype" - a struct with a single field that wraps another type.
/// Why not just use `&str` directly?
///
/// 1. **Type safety**: Can't accidentally pass an expression where identifier expected
/// 2. **Semantic clarity**: Code documents intent
/// 3. **Future extension**: Can add validation, normalization, etc.
/// 4. **Zero cost**: Compiler optimizes away the wrapper
///
/// ## Example
///
/// ```
/// use roup::ir::Identifier;
///
/// let id = Identifier::new("my_var");
/// assert_eq!(id.name(), "my_var");
/// assert_eq!(format!("{}", id), "my_var");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Identifier {
    name: String,
}

impl Identifier {
    /// Create a new identifier
    ///
    /// The name is trimmed of whitespace.
    ///
    /// ## Example
    ///
    /// ```
    /// use roup::ir::Identifier;
    ///
    /// let id = Identifier::new("  my_var  ");
    /// assert_eq!(id.name(), "my_var");
    /// ```
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            name: name.trim().to_string(),
        }
    }

    /// Get the identifier name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the identifier as a string slice
    pub fn as_str(&self) -> &str {
        &self.name
    }
}

impl From<&str> for Identifier {
    fn from(s: &str) -> Self {
        Identifier::new(s)
    }
}

impl From<String> for Identifier {
    fn from(s: String) -> Self {
        Identifier::new(s)
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

// ============================================================================
// ArraySection: Array slicing specification
// ============================================================================

/// Array section specification: `[lower:length:stride]`
///
/// OpenMP allows specifying portions of arrays using array sections:
/// - `arr[0:N]` - elements 0 through N-1
/// - `arr[i:10:2]` - 10 elements starting at i, every 2nd element
/// - `arr[:]` - all elements
///
/// ## Learning: Optional Fields
///
/// All three parts (lower, length, stride) are optional! This is
/// modeled with `Option<Expression>`:
/// - `Some(expr)` - part is present
/// - `None` - part is omitted
///
/// ## Syntax Examples
///
/// | OpenMP Syntax | lower | length | stride |
/// |---------------|-------|--------|--------|
/// | `arr[0:N]` | Some(0) | Some(N) | None |
/// | `arr[:]` | None | None | None |
/// | `arr[i:10:2]` | Some(i) | Some(10) | Some(2) |
/// | `arr[i]` | Some(i) | None | None |
///
/// ## Example
///
/// ```
/// use roup::ir::{ArraySection, Expression, ParserConfig};
///
/// let config = ParserConfig::default();
///
/// // arr[0:N]
/// let section = ArraySection {
///     lower_bound: Some(Expression::new("0", &config)),
///     length: Some(Expression::new("N", &config)),
///     stride: None,
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ArraySection {
    /// Lower bound (starting index)
    ///
    /// If `None`, starts at beginning (equivalent to 0)
    pub lower_bound: Option<Expression>,

    /// Length (number of elements)
    ///
    /// If `None`, goes to end of dimension
    pub length: Option<Expression>,

    /// Stride (spacing between elements)
    ///
    /// If `None`, defaults to 1 (consecutive elements)
    pub stride: Option<Expression>,
}

impl ArraySection {
    /// Create a new array section with all fields
    pub fn new(
        lower_bound: Option<Expression>,
        length: Option<Expression>,
        stride: Option<Expression>,
    ) -> Self {
        Self {
            lower_bound,
            length,
            stride,
        }
    }

    /// Create an array section for a single index: `arr[i]`
    ///
    /// ## Example
    ///
    /// ```
    /// use roup::ir::{ArraySection, Expression, ParserConfig};
    ///
    /// let config = ParserConfig::default();
    /// let section = ArraySection::single_index(Expression::new("i", &config));
    /// // Represents arr[i]
    /// ```
    pub fn single_index(index: Expression) -> Self {
        Self {
            lower_bound: Some(index),
            length: None,
            stride: None,
        }
    }

    /// Create an array section for all elements: `arr[:]`
    ///
    /// ## Example
    ///
    /// ```
    /// use roup::ir::ArraySection;
    ///
    /// let section = ArraySection::all();
    /// // Represents arr[:]
    /// ```
    pub const fn all() -> Self {
        Self {
            lower_bound: None,
            length: None,
            stride: None,
        }
    }

    /// Check if this represents a single index access
    pub fn is_single_index(&self) -> bool {
        self.lower_bound.is_some() && self.length.is_none() && self.stride.is_none()
    }

    /// Check if this represents all elements
    pub fn is_all(&self) -> bool {
        self.lower_bound.is_none() && self.length.is_none() && self.stride.is_none()
    }
}

impl fmt::Display for ArraySection {
    /// Format as OpenMP array section syntax
    ///
    /// ## Example
    ///
    /// ```
    /// use roup::ir::{ArraySection, Expression, ParserConfig};
    ///
    /// let config = ParserConfig::default();
    /// let section = ArraySection::new(
    ///     Some(Expression::new("0", &config)),
    ///     Some(Expression::new("N", &config)),
    ///     None,
    /// );
    /// assert_eq!(format!("{}", section), "0:N");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Format: [lower:length:stride]
        // Omitted parts are skipped, but colons are preserved

        if let Some(lower) = &self.lower_bound {
            write!(f, "{lower}")?;
        }

        if self.length.is_some() || self.stride.is_some() {
            write!(f, ":")?;
        }

        if let Some(length) = &self.length {
            write!(f, "{length}")?;
        }

        if self.stride.is_some() {
            write!(f, ":")?;
        }

        if let Some(stride) = &self.stride {
            write!(f, "{stride}")?;
        }

        Ok(())
    }
}

// ============================================================================
// Variable: Name with optional array sections
// ============================================================================

/// A variable reference, possibly with array sections
///
/// Variables in OpenMP clauses can be:
/// - Simple: `x`, `my_var`
/// - Array elements: `arr[i]`
/// - Array sections: `arr[0:N]`
/// - Multidimensional: `matrix[i][0:N]`
///
/// ## Learning: Composition
///
/// Notice how `Variable` is built from other IR types:
/// - Uses `&'a str` for the name (borrowed from source)
/// - Uses `Vec<ArraySection>` for subscripts
/// - `ArraySection` uses `Expression`
///
/// This shows how complex structures are built from simple parts.
///
/// ## Example
///
/// ```
/// use roup::ir::{Variable, ArraySection, Expression, ParserConfig};
///
/// let config = ParserConfig::default();
///
/// // Simple variable: x
/// let simple = Variable::new("x");
/// assert_eq!(simple.name(), "x");
/// assert!(simple.is_scalar());
///
/// // Array section: arr[0:N]
/// let array = Variable::with_sections(
///     "arr",
///     vec![ArraySection::new(
///         Some(Expression::new("0", &config)),
///         Some(Expression::new("N", &config)),
///         None,
///     )]
/// );
/// assert!(!array.is_scalar());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Variable {
    /// Variable name
    name: String,

    /// Array sections (empty for scalar variables)
    ///
    /// Each element represents one dimension:
    /// - `arr[i]` → 1 section
    /// - `matrix[i][j]` → 2 sections
    /// - `tensor[i][j][k]` → 3 sections
    pub array_sections: Vec<ArraySection>,
}

impl Variable {
    /// Create a new variable without array sections (scalar)
    ///
    /// ## Example
    ///
    /// ```
    /// use roup::ir::Variable;
    ///
    /// let var = Variable::new("x");
    /// assert_eq!(var.name(), "x");
    /// assert!(var.is_scalar());
    /// ```
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            name: name.trim().to_string(),
            array_sections: Vec::new(),
        }
    }

    /// Create a variable with array sections
    ///
    /// ## Example
    ///
    /// ```
    /// use roup::ir::{Variable, ArraySection};
    ///
    /// let var = Variable::with_sections(
    ///     "arr",
    ///     vec![ArraySection::all()]
    /// );
    /// assert_eq!(var.name(), "arr");
    /// assert!(!var.is_scalar());
    /// ```
    pub fn with_sections(name: impl Into<String>, sections: Vec<ArraySection>) -> Self {
        let name = name.into();
        Self {
            name: name.trim().to_string(),
            array_sections: sections,
        }
    }

    /// Get the variable name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Check if this is a scalar (no array sections)
    pub fn is_scalar(&self) -> bool {
        self.array_sections.is_empty()
    }

    /// Check if this is an array (has sections)
    pub fn is_array(&self) -> bool {
        !self.array_sections.is_empty()
    }

    /// Get the number of dimensions
    ///
    /// Returns 0 for scalars, 1+ for arrays.
    pub fn dimensions(&self) -> usize {
        self.array_sections.len()
    }
}

impl From<&str> for Variable {
    fn from(name: &str) -> Self {
        Variable::new(name)
    }
}

impl From<String> for Variable {
    fn from(name: String) -> Self {
        Variable::new(name)
    }
}

impl From<Identifier> for Variable {
    fn from(id: Identifier) -> Self {
        Variable::new(id.name())
    }
}

impl fmt::Display for Variable {
    /// Format as OpenMP variable syntax
    ///
    /// ## Example
    ///
    /// ```
    /// use roup::ir::{Variable, ArraySection, Expression, ParserConfig};
    ///
    /// let config = ParserConfig::default();
    ///
    /// // Scalar
    /// let scalar = Variable::new("x");
    /// assert_eq!(format!("{}", scalar), "x");
    ///
    /// // Array section
    /// let array = Variable::with_sections(
    ///     "arr",
    ///     vec![ArraySection::new(
    ///         Some(Expression::new("0", &config)),
    ///         Some(Expression::new("N", &config)),
    ///         None,
    ///     )]
    /// );
    /// assert_eq!(format!("{}", array), "arr[0:N]");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;

        for section in &self.array_sections {
            write!(f, "[{section}]")?;
        }

        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::ParserConfig;

    // ------------------------------------------------------------------------
    // Identifier tests
    // ------------------------------------------------------------------------

    #[test]
    fn identifier_new_trims_whitespace() {
        let id = Identifier::new("  my_var  ");
        assert_eq!(id.name(), "my_var");
    }

    #[test]
    fn identifier_from_str() {
        let id: Identifier = "test".into();
        assert_eq!(id.name(), "test");
    }

    #[test]
    fn identifier_display() {
        let id = Identifier::new("my_var");
        assert_eq!(format!("{id}"), "my_var");
    }

    #[test]
    fn identifier_equality() {
        let id1 = Identifier::new("x");
        let id2 = Identifier::new("x");
        let id3 = Identifier::new("y");

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    // ------------------------------------------------------------------------
    // ArraySection tests
    // ------------------------------------------------------------------------

    #[test]
    fn array_section_single_index() {
        let config = ParserConfig::default();
        let section = ArraySection::single_index(Expression::new("i", &config));

        assert!(section.is_single_index());
        assert!(!section.is_all());
        assert!(section.lower_bound.is_some());
        assert!(section.length.is_none());
        assert!(section.stride.is_none());
    }

    #[test]
    fn array_section_all() {
        let section = ArraySection::all();

        assert!(!section.is_single_index());
        assert!(section.is_all());
        assert!(section.lower_bound.is_none());
        assert!(section.length.is_none());
        assert!(section.stride.is_none());
    }

    #[test]
    fn array_section_display_single_index() {
        let config = ParserConfig::default();
        let section = ArraySection::single_index(Expression::new("i", &config));

        assert_eq!(format!("{section}"), "i");
    }

    #[test]
    fn array_section_display_range() {
        let config = ParserConfig::default();
        let section = ArraySection::new(
            Some(Expression::new("0", &config)),
            Some(Expression::new("N", &config)),
            None,
        );

        assert_eq!(format!("{section}"), "0:N");
    }

    #[test]
    fn array_section_display_with_stride() {
        let config = ParserConfig::default();
        let section = ArraySection::new(
            Some(Expression::new("0", &config)),
            Some(Expression::new("N", &config)),
            Some(Expression::new("2", &config)),
        );

        assert_eq!(format!("{section}"), "0:N:2");
    }

    #[test]
    fn array_section_display_all() {
        let section = ArraySection::all();
        assert_eq!(format!("{section}"), "");
    }

    #[test]
    fn array_section_display_omitted_lower() {
        let config = ParserConfig::default();
        let section = ArraySection::new(None, Some(Expression::new("N", &config)), None);

        assert_eq!(format!("{section}"), ":N");
    }

    // ------------------------------------------------------------------------
    // Variable tests
    // ------------------------------------------------------------------------

    #[test]
    fn variable_new_creates_scalar() {
        let var = Variable::new("x");

        assert_eq!(var.name(), "x");
        assert!(var.is_scalar());
        assert!(!var.is_array());
        assert_eq!(var.dimensions(), 0);
    }

    #[test]
    fn variable_with_sections_creates_array() {
        let config = ParserConfig::default();
        let var = Variable::with_sections(
            "arr",
            vec![ArraySection::single_index(Expression::new("i", &config))],
        );

        assert_eq!(var.name(), "arr");
        assert!(!var.is_scalar());
        assert!(var.is_array());
        assert_eq!(var.dimensions(), 1);
    }

    #[test]
    fn variable_multidimensional() {
        let config = ParserConfig::default();
        let var = Variable::with_sections(
            "matrix",
            vec![
                ArraySection::single_index(Expression::new("i", &config)),
                ArraySection::single_index(Expression::new("j", &config)),
            ],
        );

        assert_eq!(var.dimensions(), 2);
    }

    #[test]
    fn variable_from_str() {
        let var: Variable = "x".into();
        assert_eq!(var.name(), "x");
        assert!(var.is_scalar());
    }

    #[test]
    fn variable_from_identifier() {
        let id = Identifier::new("my_var");
        let var: Variable = id.into();
        assert_eq!(var.name(), "my_var");
        assert!(var.is_scalar());
    }

    #[test]
    fn variable_display_scalar() {
        let var = Variable::new("x");
        assert_eq!(format!("{var}"), "x");
    }

    #[test]
    fn variable_display_single_index() {
        let config = ParserConfig::default();
        let var = Variable::with_sections(
            "arr",
            vec![ArraySection::single_index(Expression::new("i", &config))],
        );

        assert_eq!(format!("{var}"), "arr[i]");
    }

    #[test]
    fn variable_display_array_section() {
        let config = ParserConfig::default();
        let var = Variable::with_sections(
            "arr",
            vec![ArraySection::new(
                Some(Expression::new("0", &config)),
                Some(Expression::new("N", &config)),
                None,
            )],
        );

        assert_eq!(format!("{var}"), "arr[0:N]");
    }

    #[test]
    fn variable_display_multidimensional() {
        let config = ParserConfig::default();
        let var = Variable::with_sections(
            "matrix",
            vec![
                ArraySection::single_index(Expression::new("i", &config)),
                ArraySection::new(
                    Some(Expression::new("0", &config)),
                    Some(Expression::new("N", &config)),
                    None,
                ),
            ],
        );

        assert_eq!(format!("{var}"), "matrix[i][0:N]");
    }

    #[test]
    fn variable_trims_name() {
        let var = Variable::new("  arr  ");
        assert_eq!(var.name(), "arr");
    }
}
