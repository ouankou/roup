//! Basic IR types: SourceLocation and Language
//!
//! This file introduces fundamental concepts:
//! - **Structs**: Composite data types with named fields
//! - **Copy trait**: Types that can be copied bitwise
//! - **repr(C)**: C-compatible memory layout for FFI
//! - **Derive macros**: Automatic trait implementations
//! - **Documentation**: How to document Rust code properly

use std::fmt;

// ============================================================================
// SourceLocation: Track where code appears in source files
// ============================================================================

/// Source code location information
///
/// Stores the line and column number where a directive or clause appears
/// in the original source file. This is useful for:
///
/// - **Error reporting**: "Error at line 42, column 5"
/// - **IDE integration**: Jump to source location
/// - **Debugging**: Know where IR came from
///
/// ## Learning: The `Copy` Trait
///
/// This struct implements `Copy`, meaning it can be duplicated by simple
/// bitwise copy (like `memcpy` in C). This is efficient for small types.
///
/// Types that implement `Copy` must also implement `Clone`. The difference:
/// - `Copy`: Implicit duplication (assignment creates a copy)
/// - `Clone`: Explicit duplication (call `.clone()` method)
///
/// ## Learning: `repr(C)`
///
/// The `#[repr(C)]` attribute tells Rust to lay out this struct in memory
/// exactly like C would. This is critical for FFI (Foreign Function Interface).
///
/// Without `repr(C)`, Rust might reorder fields for optimization.
/// With `repr(C)`, fields appear in declaration order.
///
/// ## Example
///
/// ```
/// use roup::ir::SourceLocation;
///
/// let loc = SourceLocation { line: 42, column: 5 };
/// let copy = loc; // Implicitly copied due to Copy trait
/// assert_eq!(loc.line, 42);
/// assert_eq!(copy.line, 42); // Original still valid
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct SourceLocation {
    /// Line number (1-indexed, as in editors)
    pub line: u32,

    /// Column number (1-indexed, as in editors)
    pub column: u32,
}

impl SourceLocation {
    /// Create a new source location
    ///
    /// ## Example
    ///
    /// ```
    /// use roup::ir::SourceLocation;
    ///
    /// let loc = SourceLocation::new(10, 5);
    /// assert_eq!(loc.line, 10);
    /// assert_eq!(loc.column, 5);
    /// ```
    pub const fn new(line: u32, column: u32) -> Self {
        Self { line, column }
    }

    /// Create a location at the start of a file
    ///
    /// ## Example
    ///
    /// ```
    /// use roup::ir::SourceLocation;
    ///
    /// let start = SourceLocation::start();
    /// assert_eq!(start.line, 1);
    /// assert_eq!(start.column, 1);
    /// ```
    pub const fn start() -> Self {
        Self::new(1, 1)
    }
}

impl Default for SourceLocation {
    /// Default location is at the start of a file (1, 1)
    fn default() -> Self {
        Self::start()
    }
}

impl fmt::Display for SourceLocation {
    /// Format as "line:column" for error messages
    ///
    /// ## Example
    ///
    /// ```
    /// use roup::ir::SourceLocation;
    ///
    /// let loc = SourceLocation::new(42, 5);
    /// assert_eq!(format!("{}", loc), "42:5");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

// ============================================================================
// Language: Identify source programming language
// ============================================================================

/// Source programming language
///
/// OpenMP supports multiple host languages: C, C++, and Fortran.
/// The IR needs to track the source language because:
///
/// - **Pragma syntax**: C/C++ use `#pragma omp`, Fortran uses `!$omp`
/// - **Expression parsing**: Different languages have different expression syntax
/// - **Type systems**: Languages have different type semantics
/// - **Pretty-printing**: Need to output correct syntax for the language
///
/// ## Learning: Enums as Tagged Unions
///
/// In Rust, `enum` is much more powerful than in C. Each variant can:
/// - Carry no data (like these variants)
/// - Carry data (we'll see this in later types)
/// - Have different types of data per variant
///
/// ## Learning: `repr(C)` on Enums
///
/// For FFI, we need stable discriminant values. `#[repr(C)]` ensures:
/// - Discriminant is a C-compatible integer
/// - Size and alignment match C expectations
/// - Variants have predictable numeric values
///
/// We explicitly assign values (0, 1, 2, 3) so C code can rely on them.
///
/// ## Example
///
/// ```
/// use roup::ir::Language;
///
/// let lang = Language::C;
/// assert_eq!(lang as u32, 0);
///
/// let cpp = Language::Cpp;
/// assert_eq!(cpp as u32, 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub enum Language {
    /// C language
    ///
    /// Uses `#pragma omp` syntax
    C = 0,

    /// C++ language
    ///
    /// Uses `#pragma omp` syntax (same as C)
    Cpp = 1,

    /// Fortran language
    ///
    /// Uses `!$omp` syntax
    Fortran = 2,

    /// Unknown or unspecified language
    ///
    /// Used when language cannot be determined
    Unknown = 3,
}

impl Language {
    /// Get the pragma prefix for this language
    ///
    /// Returns the string used to start OpenMP directives.
    ///
    /// ## Example
    ///
    /// ```
    /// use roup::ir::Language;
    ///
    /// assert_eq!(Language::C.pragma_prefix(), "#pragma omp ");
    /// assert_eq!(Language::Cpp.pragma_prefix(), "#pragma omp ");
    /// assert_eq!(Language::Fortran.pragma_prefix(), "!$omp ");
    /// ```
    pub const fn pragma_prefix(self) -> &'static str {
        match self {
            Language::C | Language::Cpp => "#pragma omp ",
            Language::Fortran => "!$omp ",
            Language::Unknown => "#pragma omp ", // Default to C syntax
        }
    }

    /// Check if this language uses C-style syntax
    ///
    /// Both C and C++ use the same OpenMP syntax.
    ///
    /// ## Example
    ///
    /// ```
    /// use roup::ir::Language;
    ///
    /// assert!(Language::C.is_c_family());
    /// assert!(Language::Cpp.is_c_family());
    /// assert!(!Language::Fortran.is_c_family());
    /// ```
    pub const fn is_c_family(self) -> bool {
        matches!(self, Language::C | Language::Cpp)
    }

    /// Check if this language is Fortran
    ///
    /// ## Example
    ///
    /// ```
    /// use roup::ir::Language;
    ///
    /// assert!(!Language::C.is_fortran());
    /// assert!(Language::Fortran.is_fortran());
    /// ```
    pub const fn is_fortran(self) -> bool {
        matches!(self, Language::Fortran)
    }
}

impl Default for Language {
    /// Default to unknown language
    fn default() -> Self {
        Language::Unknown
    }
}

impl fmt::Display for Language {
    /// Format language name for display
    ///
    /// ## Example
    ///
    /// ```
    /// use roup::ir::Language;
    ///
    /// assert_eq!(format!("{}", Language::C), "C");
    /// assert_eq!(format!("{}", Language::Cpp), "C++");
    /// assert_eq!(format!("{}", Language::Fortran), "Fortran");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Language::C => write!(f, "C"),
            Language::Cpp => write!(f, "C++"),
            Language::Fortran => write!(f, "Fortran"),
            Language::Unknown => write!(f, "Unknown"),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------------
    // SourceLocation tests
    // ------------------------------------------------------------------------

    #[test]
    fn source_location_new_creates_correct_location() {
        let loc = SourceLocation::new(42, 5);
        assert_eq!(loc.line, 42);
        assert_eq!(loc.column, 5);
    }

    #[test]
    fn source_location_start_is_one_one() {
        let start = SourceLocation::start();
        assert_eq!(start.line, 1);
        assert_eq!(start.column, 1);
    }

    #[test]
    fn source_location_default_is_start() {
        let default = SourceLocation::default();
        let start = SourceLocation::start();
        assert_eq!(default, start);
    }

    #[test]
    fn source_location_copy_works() {
        let loc1 = SourceLocation::new(10, 20);
        let loc2 = loc1; // Should copy, not move
        assert_eq!(loc1.line, 10); // loc1 still valid
        assert_eq!(loc2.line, 10); // loc2 has same value
    }

    #[test]
    fn source_location_display_formats_correctly() {
        let loc = SourceLocation::new(42, 5);
        assert_eq!(format!("{}", loc), "42:5");
    }

    #[test]
    fn source_location_equality_works() {
        let loc1 = SourceLocation::new(42, 5);
        let loc2 = SourceLocation::new(42, 5);
        let loc3 = SourceLocation::new(42, 6);

        assert_eq!(loc1, loc2);
        assert_ne!(loc1, loc3);
    }

    // ------------------------------------------------------------------------
    // Language tests
    // ------------------------------------------------------------------------

    #[test]
    fn language_has_correct_discriminants() {
        assert_eq!(Language::C as u32, 0);
        assert_eq!(Language::Cpp as u32, 1);
        assert_eq!(Language::Fortran as u32, 2);
        assert_eq!(Language::Unknown as u32, 3);
    }

    #[test]
    fn language_pragma_prefix_c_and_cpp() {
        assert_eq!(Language::C.pragma_prefix(), "#pragma omp ");
        assert_eq!(Language::Cpp.pragma_prefix(), "#pragma omp ");
    }

    #[test]
    fn language_pragma_prefix_fortran() {
        assert_eq!(Language::Fortran.pragma_prefix(), "!$omp ");
    }

    #[test]
    fn language_pragma_prefix_unknown_defaults_to_c() {
        assert_eq!(Language::Unknown.pragma_prefix(), "#pragma omp ");
    }

    #[test]
    fn language_is_c_family() {
        assert!(Language::C.is_c_family());
        assert!(Language::Cpp.is_c_family());
        assert!(!Language::Fortran.is_c_family());
        assert!(!Language::Unknown.is_c_family());
    }

    #[test]
    fn language_is_fortran() {
        assert!(!Language::C.is_fortran());
        assert!(!Language::Cpp.is_fortran());
        assert!(Language::Fortran.is_fortran());
        assert!(!Language::Unknown.is_fortran());
    }

    #[test]
    fn language_display_formats_correctly() {
        assert_eq!(format!("{}", Language::C), "C");
        assert_eq!(format!("{}", Language::Cpp), "C++");
        assert_eq!(format!("{}", Language::Fortran), "Fortran");
        assert_eq!(format!("{}", Language::Unknown), "Unknown");
    }

    #[test]
    fn language_default_is_unknown() {
        assert_eq!(Language::default(), Language::Unknown);
    }

    #[test]
    fn language_copy_works() {
        let lang1 = Language::C;
        let lang2 = lang1; // Should copy, not move
        assert_eq!(lang1, Language::C); // lang1 still valid
        assert_eq!(lang2, Language::C); // lang2 has same value
    }

    #[test]
    fn language_equality_works() {
        assert_eq!(Language::C, Language::C);
        assert_ne!(Language::C, Language::Cpp);
        assert_ne!(Language::Fortran, Language::Unknown);
    }
}
