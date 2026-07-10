//! Strict host-language expressions used by the semantic IR.
//!
//! Every `Expression` owns a fully classified [`crate::host`] syntax tree.
//! Unsupported or malformed input is an error: there is no string-only mode,
//! opaque node, or best-effort fallback.  Source text is retained solely as
//! backing storage for the byte spans carried by the tree; rendering always
//! walks typed syntax.

use std::fmt;

use crate::host::{self, HostLanguage};
use crate::source::{SourceError, Span};
use crate::version::{
    CStandard, CppStandard, FortranStandard, HostLanguageProfile, OpenMpVersion, VersionPolicy,
};

pub use crate::host::{
    BinaryOp as BinaryOperator, Expr as ExpressionAst, ExprKind as ExpressionKind,
    UnaryOp as UnaryOperator,
};

/// Configuration shared by IR payload parsers.
///
/// The host language is a closed, known value shared with version policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParserConfig {
    profile: HostLanguageProfile,
    openmp_version_policy: VersionPolicy<OpenMpVersion>,
    structural_nesting_depth: u16,
}

/// Maximum recursion depth for recursively nested typed directive structures.
///
/// This bounds nested metadirective variants, nested applied directives, and
/// recursive braced initializers. Inputs that would exceed the limit are hard
/// errors; the parser never relies on a process stack overflow as validation.
pub const MAX_STRUCTURAL_NESTING_DEPTH: u16 = 32;

impl ParserConfig {
    #[must_use]
    pub const fn new(profile: HostLanguageProfile) -> Self {
        Self {
            profile,
            openmp_version_policy: VersionPolicy::Any,
            structural_nesting_depth: 0,
        }
    }

    #[must_use]
    pub(crate) const fn with_openmp_version_policy(
        mut self,
        policy: VersionPolicy<OpenMpVersion>,
    ) -> Self {
        self.openmp_version_policy = policy;
        self
    }

    #[must_use]
    pub(crate) const fn openmp_version_policy(self) -> VersionPolicy<OpenMpVersion> {
        self.openmp_version_policy
    }

    /// Enter one recursively nested typed structure.
    pub(crate) const fn enter_nested_structure(mut self) -> Result<Self, &'static str> {
        if self.structural_nesting_depth >= MAX_STRUCTURAL_NESTING_DEPTH {
            return Err("typed directive structure nesting limit exceeded");
        }
        self.structural_nesting_depth += 1;
        Ok(self)
    }

    #[must_use]
    pub const fn c() -> Self {
        Self::new(HostLanguageProfile::C(CStandard::C23))
    }

    #[must_use]
    pub const fn cpp() -> Self {
        Self::new(HostLanguageProfile::Cpp(CppStandard::Cpp23))
    }

    #[must_use]
    pub const fn fortran() -> Self {
        Self::new(HostLanguageProfile::Fortran(FortranStandard::Fortran2023))
    }

    #[must_use]
    pub const fn from_language(language: HostLanguage) -> Self {
        Self::new(HostLanguageProfile::latest(language))
    }

    #[must_use]
    pub const fn profile(self) -> HostLanguageProfile {
        self.profile
    }

    #[must_use]
    pub const fn host_language(self) -> HostLanguage {
        match self.profile {
            HostLanguageProfile::C(_) => HostLanguage::C,
            HostLanguageProfile::Cpp(_) => HostLanguage::Cpp,
            HostLanguageProfile::Fortran(_) => HostLanguage::Fortran,
        }
    }

    #[must_use]
    pub const fn language(self) -> HostLanguage {
        match self.profile {
            HostLanguageProfile::C(_) => HostLanguage::C,
            HostLanguageProfile::Cpp(_) => HostLanguage::Cpp,
            HostLanguageProfile::Fortran(_) => HostLanguage::Fortran,
        }
    }
}

impl From<HostLanguage> for ParserConfig {
    fn from(language: HostLanguage) -> Self {
        Self::from_language(language)
    }
}

/// A fully parsed host-language expression.
#[derive(Debug, Clone)]
pub struct Expression {
    source: Box<str>,
    profile: HostLanguageProfile,
    ast: host::Expr,
}

impl Expression {
    /// Parse an expression using the explicit language in `config`.
    pub fn new(source: impl Into<String>, config: &ParserConfig) -> Result<Self, ExpressionError> {
        Self::parse_with_profile(source, config.profile())
    }

    /// Parse an expression for a concrete C, C++, or Fortran host language.
    pub fn parse(
        source: impl Into<String>,
        language: HostLanguage,
    ) -> Result<Self, ExpressionError> {
        Self::parse_with_profile(source, HostLanguageProfile::latest(language))
    }

    /// Parse using an exact host-language standard profile.
    pub fn parse_with_profile(
        source: impl Into<String>,
        profile: HostLanguageProfile,
    ) -> Result<Self, ExpressionError> {
        let source = source.into().into_boxed_str();
        let ast = host::parse_expression_with_profile(&source, profile)?;
        Ok(Self {
            source,
            profile,
            ast,
        })
    }

    /// The authoritative typed syntax tree.
    #[must_use]
    pub const fn ast(&self) -> &host::Expr {
        &self.ast
    }

    /// The concrete language used to parse and render this expression.
    #[must_use]
    pub const fn language(&self) -> HostLanguage {
        self.profile.language()
    }

    /// Exact host standard used to classify the expression.
    #[must_use]
    pub const fn profile(&self) -> HostLanguageProfile {
        self.profile
    }

    /// Source backing for AST spans.  Semantic consumers should use [`Self::ast`]
    /// and rendering should use [`fmt::Display`].
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Resolve a checked AST span against this expression's source backing.
    pub fn span_text(&self, span: Span) -> Result<&str, SourceError> {
        span.slice(&self.source)
    }

    /// Retain a typed subtree as its own expression without rendering or
    /// reparsing it. The original source backing is shared by cloning so every
    /// span in the subtree remains valid.
    pub(crate) fn subtree(&self, ast: &host::Expr) -> Self {
        Self {
            source: self.source.clone(),
            profile: self.profile,
            ast: ast.clone(),
        }
    }
}

impl PartialEq for Expression {
    fn eq(&self, other: &Self) -> bool {
        self.profile == other.profile && self.ast == other.ast
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.ast.canonical(self.language()).fmt(formatter)
    }
}

/// Failure to select or parse an expression language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpressionError {
    Parse(host::ParseError),
}

impl fmt::Display for ExpressionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => {
                write!(formatter, "invalid expression at {}: {error}", error.span)
            }
        }
    }
}

impl std::error::Error for ExpressionError {}

impl From<host::ParseError> for ExpressionError {
    fn from(error: host::ParseError) -> Self {
        Self::Parse(error)
    }
}

#[cfg(test)]
mod tests {
    use crate::host::{BinaryOp, ExprKind, MemberAccess, Subscript};

    use super::*;

    #[test]
    fn every_ir_language_maps_to_a_concrete_host_language() {
        assert_eq!(
            ParserConfig::from_language(HostLanguage::C),
            ParserConfig::c()
        );
        assert_eq!(
            ParserConfig::from_language(HostLanguage::Cpp),
            ParserConfig::cpp()
        );
        assert_eq!(
            ParserConfig::from_language(HostLanguage::Fortran),
            ParserConfig::fortran()
        );
    }

    #[test]
    fn valid_nested_c_expression_is_fully_typed() {
        let expression =
            Expression::new("flag ? data[index + 1] : call(-x, y)", &ParserConfig::c()).unwrap();
        assert!(matches!(
            expression.ast().kind,
            ExprKind::Conditional { .. }
        ));
        assert!(!format!("{expression}").is_empty());
    }

    #[test]
    fn valid_nested_cpp_expression_is_fully_typed() {
        let expression = Expression::new(
            "::ns::factory(obj.member)->values[lower:length]",
            &ParserConfig::cpp(),
        )
        .unwrap();
        let ExprKind::Subscript { base, subscript } = &expression.ast().kind else {
            panic!("expected subscript root")
        };
        assert!(matches!(subscript, Subscript::Section(_)));
        assert!(matches!(
            base.kind,
            ExprKind::Member {
                access: MemberAccess::Arrow,
                ..
            }
        ));
    }

    #[test]
    fn valid_nested_fortran_expression_is_fully_typed() {
        let expression = Expression::new(
            ".not. array(1:n:2, :)%ready .or. .false.",
            &ParserConfig::fortran(),
        )
        .unwrap();
        assert!(matches!(
            expression.ast().kind,
            ExprKind::Binary {
                op: BinaryOp::LogicalOr,
                ..
            }
        ));
    }

    #[test]
    fn formerly_opaque_or_complex_inputs_are_hard_errors() {
        for source in ["sizeof(struct item)", "a @ b", "call(", "a ? b"] {
            assert!(Expression::new(source, &ParserConfig::c()).is_err());
        }
    }

    #[test]
    fn display_is_canonical_not_the_source_buffer() {
        let expression = Expression::new("  a+b * c  ", &ParserConfig::c()).unwrap();
        assert_eq!(expression.source(), "  a+b * c  ");
        assert_eq!(expression.to_string(), "a + b * c");
        assert_eq!(
            expression.span_text(expression.ast().span).unwrap(),
            "a+b * c"
        );
    }

    #[test]
    fn equality_compares_typed_grouping_not_rendered_source_text() {
        let left_grouped = Expression::new("(a + b) * c", &ParserConfig::c()).unwrap();
        let right_grouped = Expression::new("a + (b * c)", &ParserConfig::c()).unwrap();

        assert_ne!(left_grouped.ast(), right_grouped.ast());
        assert_ne!(left_grouped, right_grouped);
    }
}
