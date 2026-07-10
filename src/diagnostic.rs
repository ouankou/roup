//! Stable structured diagnostics returned by strict parsing and validation.

use crate::source::Span;
use std::error::Error;
use std::fmt;

/// Stable machine-readable diagnostic identifiers.
///
/// Numeric values are part of the public interface and must never be reused.
/// New codes should receive a new value in the appropriate category.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum DiagnosticCode {
    InvalidConfiguration = 1000,
    IncompatibleSourceForm = 1001,

    UnexpectedEndOfInput = 2000,
    UnexpectedToken = 2001,
    TrailingInput = 2002,
    UnterminatedComment = 2003,
    InvalidContinuation = 2004,
    InvalidSentinel = 2005,
    InvalidIdentifier = 2100,
    InvalidLiteral = 2101,
    InvalidExpression = 2102,
    InvalidTypeName = 2103,
    InvalidLocator = 2104,

    InvalidDirective = 3000,
    MissingDirectiveParameter = 3001,
    InvalidClause = 3002,
    ClauseNotAllowed = 3003,
    MissingRequiredClause = 3004,
    DuplicateClause = 3005,
    ConflictingClauses = 3006,
    InvalidModifier = 3007,
    EmptyList = 3008,
    InvalidSelector = 3009,

    VersionAmbiguity = 4000,
    NotAvailableInVersion = 4001,
    NoCompatibleVersion = 4002,
    CannotRenderForVersion = 4003,

    MissingContext = 5000,
    MissingSemanticFact = 5001,
    InvalidNesting = 5002,
    MismatchedEndDirective = 5003,
    InvalidAssociation = 5004,
    InvalidDeclarationPosition = 5005,
    InvalidExpressionType = 5006,
    ConstantExpressionRequired = 5007,
}

impl DiagnosticCode {
    /// Returns the stable symbolic spelling used in logs and serialized output.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidConfiguration => "invalid-configuration",
            Self::IncompatibleSourceForm => "incompatible-source-form",
            Self::UnexpectedEndOfInput => "unexpected-end-of-input",
            Self::UnexpectedToken => "unexpected-token",
            Self::TrailingInput => "trailing-input",
            Self::UnterminatedComment => "unterminated-comment",
            Self::InvalidContinuation => "invalid-continuation",
            Self::InvalidSentinel => "invalid-sentinel",
            Self::InvalidIdentifier => "invalid-identifier",
            Self::InvalidLiteral => "invalid-literal",
            Self::InvalidExpression => "invalid-expression",
            Self::InvalidTypeName => "invalid-type-name",
            Self::InvalidLocator => "invalid-locator",
            Self::InvalidDirective => "invalid-directive",
            Self::MissingDirectiveParameter => "missing-directive-parameter",
            Self::InvalidClause => "invalid-clause",
            Self::ClauseNotAllowed => "clause-not-allowed",
            Self::MissingRequiredClause => "missing-required-clause",
            Self::DuplicateClause => "duplicate-clause",
            Self::ConflictingClauses => "conflicting-clauses",
            Self::InvalidModifier => "invalid-modifier",
            Self::EmptyList => "empty-list",
            Self::InvalidSelector => "invalid-selector",
            Self::VersionAmbiguity => "version-ambiguity",
            Self::NotAvailableInVersion => "not-available-in-version",
            Self::NoCompatibleVersion => "no-compatible-version",
            Self::CannotRenderForVersion => "cannot-render-for-version",
            Self::MissingContext => "missing-context",
            Self::MissingSemanticFact => "missing-semantic-fact",
            Self::InvalidNesting => "invalid-nesting",
            Self::MismatchedEndDirective => "mismatched-end-directive",
            Self::InvalidAssociation => "invalid-association",
            Self::InvalidDeclarationPosition => "invalid-declaration-position",
            Self::InvalidExpressionType => "invalid-expression-type",
            Self::ConstantExpressionRequired => "constant-expression-required",
        }
    }

    /// Returns the stable numeric code used by non-Rust consumers.
    #[must_use]
    pub const fn number(self) -> u16 {
        self as u16
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A secondary source location associated with a diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelatedSpan {
    span: Span,
    message: Box<str>,
}

impl RelatedSpan {
    #[must_use]
    pub fn new(span: Span, message: impl Into<Box<str>>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }

    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// A complete, structured hard error.
///
/// Diagnostics always carry one primary source span. Related spans identify
/// earlier declarations, opening constructs, or conflicting clauses without
/// changing which error was reported first.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    code: DiagnosticCode,
    message: Box<str>,
    primary: Span,
    related: Vec<RelatedSpan>,
}

impl Diagnostic {
    /// Creates a diagnostic at its primary source span.
    #[must_use]
    pub fn new(code: DiagnosticCode, primary: Span, message: impl Into<Box<str>>) -> Self {
        Self {
            code,
            message: message.into(),
            primary,
            related: Vec::new(),
        }
    }

    /// Returns the stable machine-readable code.
    #[must_use]
    pub const fn code(&self) -> DiagnosticCode {
        self.code
    }

    /// Returns the human-readable explanation.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the source span where the problem was first detected.
    #[must_use]
    pub const fn primary_span(&self) -> Span {
        self.primary
    }

    /// Returns related locations in insertion order.
    #[must_use]
    pub fn related_spans(&self) -> &[RelatedSpan] {
        &self.related
    }

    /// Adds a related location and returns the updated diagnostic.
    #[must_use]
    pub fn with_related(mut self, span: Span, message: impl Into<Box<str>>) -> Self {
        self.related.push(RelatedSpan::new(span, message));
        self
    }

    /// Adds a related location in-place.
    pub fn push_related(&mut self, span: Span, message: impl Into<Box<str>>) {
        self.related.push(RelatedSpan::new(span, message));
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}[{}] at {}: {}",
            self.code,
            self.code.number(),
            self.primary,
            self.message
        )
    }
}

impl Error for Diagnostic {}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(source: &str, start: usize, end: usize) -> Span {
        Span::new(source, start, end).expect("test span must be valid")
    }

    #[test]
    fn diagnostic_codes_have_stable_names_and_numbers() {
        assert_eq!(DiagnosticCode::UnexpectedToken.number(), 2001);
        assert_eq!(DiagnosticCode::UnexpectedToken.as_str(), "unexpected-token");
        assert_eq!(DiagnosticCode::NoCompatibleVersion.number(), 4002);
        assert_eq!(
            DiagnosticCode::NoCompatibleVersion.to_string(),
            "no-compatible-version"
        );
    }

    #[test]
    fn diagnostic_exposes_primary_error_data() {
        let source = "#pragma omp mystery";
        let primary = span(source, 12, 19);
        let diagnostic = Diagnostic::new(
            DiagnosticCode::InvalidDirective,
            primary,
            "directive `mystery` is not standardized",
        );

        assert_eq!(diagnostic.code(), DiagnosticCode::InvalidDirective);
        assert_eq!(diagnostic.primary_span(), primary);
        assert_eq!(
            diagnostic.message(),
            "directive `mystery` is not standardized"
        );
        assert!(diagnostic.related_spans().is_empty());
        assert!(diagnostic.to_string().contains("invalid-directive[3000]"));
    }

    #[test]
    fn related_spans_preserve_source_order_supplied_by_validator() {
        let source = "default(shared) default(private)";
        let first = span(source, 0, 15);
        let second = span(source, 16, source.len());
        let mut diagnostic = Diagnostic::new(
            DiagnosticCode::DuplicateClause,
            second,
            "`default` may appear only once",
        )
        .with_related(first, "first `default` clause appears here");
        diagnostic.push_related(second, "duplicate clause appears here");

        assert_eq!(diagnostic.related_spans().len(), 2);
        assert_eq!(diagnostic.related_spans()[0].span(), first);
        assert_eq!(
            diagnostic.related_spans()[0].message(),
            "first `default` clause appears here"
        );
        assert_eq!(diagnostic.related_spans()[1].span(), second);
    }

    #[test]
    fn diagnostics_are_standard_errors() {
        fn takes_error(_: &(dyn Error + 'static)) {}

        let diagnostic = Diagnostic::new(
            DiagnosticCode::UnexpectedEndOfInput,
            Span::point("", 0).expect("valid end-of-input point"),
            "expected a directive",
        );
        takes_error(&diagnostic);
    }
}
