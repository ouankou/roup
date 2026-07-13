//! Validated host-language identifiers and variable designators.
//!
//! A variable is represented by the same parsed expression tree used by every
//! other clause payload.  ROUP therefore has one authoritative representation
//! for names, member access, subscripts, and Fortran section triplets; it does
//! not copy those nodes into a second array-section model or retain a raw name.

use std::fmt;

use crate::host::{Expr, ExprKind, FortranArgument, HostLanguage, QualifiedName, UnaryOp};

use super::{Expression, ExpressionError, ParserConfig};

pub use crate::host::{Identifier, IdentifierError};

/// A fully parsed expression whose root is a host-language variable
/// designator.
///
/// Construction is checked.  Literal, arithmetic, conditional, assignment,
/// and C/C++ call expressions cannot be smuggled into a locator list through
/// this type.  Fortran application syntax remains a designator because the
/// language deliberately leaves `a(i)` ambiguous between an array reference
/// and a function reference until name resolution.
#[derive(Debug, Clone, PartialEq)]
pub struct Variable {
    expression: Expression,
}

/// A fully parsed expression whose root is a host-language lvalue.
///
/// This is intentionally distinct from [`Variable`]. OpenMP locations such as
/// a C/C++ `depobj` argument also admit dereference expressions (`*pointer`),
/// while ordinary variable lists do not. Construction validates the typed host
/// tree and never retains an unchecked source string.
#[derive(Debug, Clone, PartialEq)]
pub struct LValue {
    expression: Expression,
}

impl LValue {
    /// Parse and validate one lvalue expression.
    pub fn parse(source: impl Into<String>, config: &ParserConfig) -> Result<Self, LValueError> {
        Self::from_expression(Expression::new(source, config)?)
    }

    /// Validate an expression that has already been parsed.
    pub fn from_expression(expression: Expression) -> Result<Self, LValueError> {
        let valid = match expression.language() {
            HostLanguage::C | HostLanguage::Cpp => is_c_or_cpp_lvalue(expression.ast()),
            HostLanguage::Fortran => Variable::is_designator_expression(&expression),
        };
        if valid {
            Ok(Self { expression })
        } else {
            Err(LValueError::NotLValue)
        }
    }

    /// The authoritative parsed expression.
    #[must_use]
    pub const fn expression(&self) -> &Expression {
        &self.expression
    }

    /// The authoritative host-language syntax tree.
    #[must_use]
    pub const fn ast(&self) -> &Expr {
        self.expression.ast()
    }

    /// Whether any subscript in this lvalue is an array section.
    #[must_use]
    pub fn has_array_section(&self) -> bool {
        designator_has_array_section(self.ast())
    }
}

impl fmt::Display for LValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.expression.fmt(formatter)
    }
}

/// Failure to construct a checked lvalue expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LValueError {
    Expression(ExpressionError),
    NotLValue,
}

impl fmt::Display for LValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expression(error) => error.fmt(formatter),
            Self::NotLValue => formatter.write_str("expression is not an lvalue"),
        }
    }
}

impl std::error::Error for LValueError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Expression(error) => Some(error),
            Self::NotLValue => None,
        }
    }
}

impl From<ExpressionError> for LValueError {
    fn from(error: ExpressionError) -> Self {
        Self::Expression(error)
    }
}

fn is_c_or_cpp_lvalue(expression: &Expr) -> bool {
    match &expression.kind {
        ExprKind::Name(_)
        | ExprKind::LegacyQualifiedName { .. }
        | ExprKind::Member { .. }
        | ExprKind::Subscript { .. } => true,
        ExprKind::Parenthesized(inner) => is_c_or_cpp_lvalue(inner),
        ExprKind::Unary {
            op: UnaryOp::Dereference,
            ..
        } => true,
        ExprKind::Literal(_)
        | ExprKind::This
        | ExprKind::Sizeof(_)
        | ExprKind::CppTemplateId { .. }
        | ExprKind::LegacyQualifiedInteger { .. }
        | ExprKind::LegacyFortranSubscript { .. }
        | ExprKind::LegacyFortranUnaryDesignator { .. }
        | ExprKind::Unary { .. }
        | ExprKind::FortranDefinedUnary { .. }
        | ExprKind::Binary { .. }
        | ExprKind::FortranDefinedBinary { .. }
        | ExprKind::Conditional { .. }
        | ExprKind::Assignment { .. }
        | ExprKind::Call { .. }
        | ExprKind::Postfix { .. }
        | ExprKind::FortranApply { .. } => false,
    }
}

impl Variable {
    /// Parse and validate one variable designator.
    pub fn parse(source: impl Into<String>, config: &ParserConfig) -> Result<Self, VariableError> {
        Self::from_expression(Expression::new(source, config)?)
    }

    /// Validate an expression that has already been parsed.
    pub fn from_expression(expression: Expression) -> Result<Self, VariableError> {
        if Self::is_designator_expression(&expression) {
            Ok(Self { expression })
        } else {
            Err(VariableError::NotDesignator)
        }
    }

    #[must_use]
    pub(crate) fn is_designator_expression(expression: &Expression) -> bool {
        is_designator(expression.ast())
    }

    /// The authoritative parsed expression.
    #[must_use]
    pub const fn expression(&self) -> &Expression {
        &self.expression
    }

    /// The authoritative host-language syntax tree.
    #[must_use]
    pub const fn ast(&self) -> &Expr {
        self.expression.ast()
    }

    /// Return the identifier for an unqualified, unsubscripted name.
    #[must_use]
    pub fn simple_identifier(&self) -> Option<&Identifier> {
        let ExprKind::Name(QualifiedName {
            global: false,
            segments,
        }) = &self.ast().kind
        else {
            return None;
        };
        (segments.len() == 1).then(|| &segments[0])
    }

    /// The unqualified base identifier of this designator.
    ///
    /// For example, this returns `a` for `a.member[lower:length]`.  A
    /// qualified C++ name has no single local base identifier and returns
    /// `None`.
    #[must_use]
    pub fn root_identifier(&self) -> Option<&Identifier> {
        designator_root(self.ast())
    }

    /// Whether this designator has no subscript or Fortran argument list.
    #[must_use]
    pub fn is_scalar(&self) -> bool {
        designator_rank(self.ast()) == 0
    }

    /// Number of explicitly represented subscript dimensions.
    #[must_use]
    pub fn dimensions(&self) -> usize {
        designator_rank(self.ast())
    }

    /// Whether any dimension is represented by an array section rather than
    /// a single element subscript.
    #[must_use]
    pub fn has_array_section(&self) -> bool {
        designator_has_array_section(self.ast())
    }
}

impl fmt::Display for Variable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.expression.fmt(formatter)
    }
}

/// Failure to construct a checked variable designator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariableError {
    Expression(ExpressionError),
    NotDesignator,
}

impl fmt::Display for VariableError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expression(error) => error.fmt(formatter),
            Self::NotDesignator => formatter.write_str("expression is not a variable designator"),
        }
    }
}

impl std::error::Error for VariableError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Expression(error) => Some(error),
            Self::NotDesignator => None,
        }
    }
}

impl From<ExpressionError> for VariableError {
    fn from(error: ExpressionError) -> Self {
        Self::Expression(error)
    }
}

fn is_designator(expression: &Expr) -> bool {
    match &expression.kind {
        ExprKind::Name(_) | ExprKind::LegacyQualifiedName { .. } => true,
        ExprKind::Parenthesized(inner) => is_designator(inner),
        ExprKind::Member { base, .. }
        | ExprKind::Subscript { base, .. }
        | ExprKind::LegacyFortranSubscript { base, .. } => is_designator(base),
        ExprKind::LegacyFortranUnaryDesignator { operand, .. } => is_designator(operand),
        ExprKind::FortranApply {
            designator,
            arguments,
        } => {
            is_designator(designator)
                && arguments.iter().all(|argument| match argument {
                    FortranArgument::Positional(value) | FortranArgument::Keyword { value, .. } => {
                        !matches!(value.kind, ExprKind::Assignment { .. })
                    }
                    FortranArgument::Section(_) => true,
                })
        }
        ExprKind::Literal(_)
        | ExprKind::This
        | ExprKind::Sizeof(_)
        | ExprKind::CppTemplateId { .. }
        | ExprKind::LegacyQualifiedInteger { .. }
        | ExprKind::Unary { .. }
        | ExprKind::FortranDefinedUnary { .. }
        | ExprKind::Binary { .. }
        | ExprKind::FortranDefinedBinary { .. }
        | ExprKind::Conditional { .. }
        | ExprKind::Assignment { .. }
        | ExprKind::Call { .. }
        | ExprKind::Postfix { .. } => false,
    }
}

fn designator_rank(expression: &Expr) -> usize {
    match &expression.kind {
        ExprKind::Parenthesized(inner) | ExprKind::Member { base: inner, .. } => {
            designator_rank(inner)
        }
        ExprKind::Subscript { base, .. } | ExprKind::LegacyFortranSubscript { base, .. } => {
            designator_rank(base) + 1
        }
        ExprKind::LegacyFortranUnaryDesignator { operand, .. } => designator_rank(operand),
        ExprKind::FortranApply {
            designator,
            arguments,
        } => designator_rank(designator) + arguments.len(),
        _ => 0,
    }
}

fn designator_has_array_section(expression: &Expr) -> bool {
    match &expression.kind {
        ExprKind::Parenthesized(inner) | ExprKind::Member { base: inner, .. } => {
            designator_has_array_section(inner)
        }
        ExprKind::Subscript { base, subscript } => {
            matches!(subscript, crate::host::Subscript::Section(_))
                || designator_has_array_section(base)
        }
        ExprKind::LegacyFortranSubscript { base, subscript } => {
            matches!(subscript, crate::host::Subscript::Section(_))
                || designator_has_array_section(base)
        }
        ExprKind::LegacyFortranUnaryDesignator { operand, .. } => {
            designator_has_array_section(operand)
        }
        ExprKind::FortranApply {
            designator,
            arguments,
        } => {
            designator_has_array_section(designator)
                || arguments
                    .iter()
                    .any(|argument| matches!(argument, FortranArgument::Section(_)))
        }
        ExprKind::Name(_) | ExprKind::LegacyQualifiedName { .. } | ExprKind::This => false,
        ExprKind::Literal(_)
        | ExprKind::Sizeof(_)
        | ExprKind::CppTemplateId { .. }
        | ExprKind::LegacyQualifiedInteger { .. }
        | ExprKind::Unary { .. }
        | ExprKind::FortranDefinedUnary { .. }
        | ExprKind::Binary { .. }
        | ExprKind::FortranDefinedBinary { .. }
        | ExprKind::Conditional { .. }
        | ExprKind::Assignment { .. }
        | ExprKind::Call { .. }
        | ExprKind::Postfix { .. } => false,
    }
}

fn designator_root(expression: &Expr) -> Option<&Identifier> {
    match &expression.kind {
        ExprKind::Name(QualifiedName {
            global: false,
            segments,
        }) if segments.len() == 1 => segments.first(),
        ExprKind::Parenthesized(inner)
        | ExprKind::Member { base: inner, .. }
        | ExprKind::Subscript { base: inner, .. }
        | ExprKind::LegacyFortranSubscript { base: inner, .. }
        | ExprKind::LegacyFortranUnaryDesignator { operand: inner, .. }
        | ExprKind::FortranApply {
            designator: inner, ..
        } => designator_root(inner),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::host::{ExprKind, FortranArgument, SectionSemantics, Subscript};

    use super::*;

    #[test]
    fn identifier_construction_is_strict() {
        assert_eq!(Identifier::new("value").unwrap().as_str(), "value");
        assert_eq!(Identifier::new(""), Err(IdentifierError::Empty));
        assert!(Identifier::new(" two").is_err());
        assert!(Identifier::new("a-b").is_err());
    }

    #[test]
    fn c_designator_reuses_host_subscript_tree() {
        let variable = Variable::parse("object.values[lower:length]", &ParserConfig::c()).unwrap();
        assert_eq!(
            variable.root_identifier().map(Identifier::as_str),
            Some("object")
        );
        assert_eq!(variable.dimensions(), 1);
        assert!(!variable.is_scalar());
        let ExprKind::Subscript { subscript, .. } = &variable.ast().kind else {
            panic!("expected subscript designator")
        };
        let Subscript::Section(section) = subscript else {
            panic!("expected array section")
        };
        assert_eq!(section.semantics, SectionSemantics::CLength);
        assert_eq!(variable.to_string(), "object.values[lower:length]");
    }

    #[test]
    fn fortran_designator_preserves_upper_bound_semantics() {
        let variable =
            Variable::parse("array(1:upper:2, :)%field", &ParserConfig::fortran()).unwrap();
        assert_eq!(variable.dimensions(), 2);
        let ExprKind::Member { base, .. } = &variable.ast().kind else {
            panic!("expected component designator")
        };
        let ExprKind::FortranApply { arguments, .. } = &base.kind else {
            panic!("expected Fortran application")
        };
        let FortranArgument::Section(section) = &arguments[0] else {
            panic!("expected section triplet")
        };
        assert_eq!(section.semantics, SectionSemantics::FortranUpperBound);
        assert_eq!(variable.to_string(), "array(1:upper:2, :)%field");
    }

    #[test]
    fn non_designators_are_hard_errors() {
        for source in ["1", "a + b", "flag ? a : b", "call(a)"] {
            assert!(
                Variable::parse(source, &ParserConfig::c()).is_err(),
                "`{source}` must not become a variable"
            );
        }
    }
}
