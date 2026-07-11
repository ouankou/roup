//! Strict host-language expressions used by the semantic IR.
//!
//! Every `Expression` owns a fully classified [`crate::host`] syntax tree.
//! Unsupported or malformed input is an error: there is no string-only mode,
//! opaque node, or best-effort fallback.  Source text is retained solely as
//! backing storage for the byte spans carried by the tree; rendering always
//! walks typed syntax.

use std::fmt;

use crate::host::{
    self, ArraySection, AssignmentOp, BinaryOp, CppTemplateArgument, Expr, ExprKind,
    FortranArgument, HostLanguage, MemberAccess, PostfixOp, Subscript, UnaryOp,
};
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
    source_compatibility: bool,
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
            source_compatibility: false,
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

    #[must_use]
    pub(crate) const fn with_source_compatibility(mut self, enabled: bool) -> Self {
        self.source_compatibility = enabled;
        self
    }

    #[must_use]
    pub(crate) const fn source_compatibility(self) -> bool {
        self.source_compatibility
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
    /// Parse an expression using the language profile and source policy in `config`.
    pub fn new(source: impl Into<String>, config: &ParserConfig) -> Result<Self, ExpressionError> {
        Self::parse_configured(source.into(), config)
    }

    fn parse_legacy_qualified_value(source: &str, config: &ParserConfig) -> Option<Self> {
        if !config.source_compatibility() {
            return None;
        }
        let components = source.split("::").collect::<Vec<_>>();
        if components.len() < 2 || components.iter().any(|component| component.is_empty()) {
            return None;
        }
        let kind =
            if components.len() == 2 && components[1].bytes().all(|byte| byte.is_ascii_digit()) {
                host::ExprKind::LegacyQualifiedInteger {
                    qualifier: host::Identifier::new(components[0]).ok()?,
                    value: host::IntegerLiteral {
                        base: host::IntegerBase::Decimal,
                        value: components[1].parse::<u128>().ok()?,
                        suffix: host::IntegerSuffix::None,
                    },
                }
            } else {
                host::ExprKind::Name(host::QualifiedName {
                    global: false,
                    segments: components
                        .into_iter()
                        .map(host::Identifier::new)
                        .collect::<Result<Vec<_>, _>>()
                        .ok()?,
                })
            };
        let source = source.to_owned().into_boxed_str();
        let ast = host::Expr::new(Span::entire(&source), kind);
        Some(Self {
            source,
            profile: config.profile(),
            ast,
        })
    }

    pub(crate) fn new_with_legacy_qualified_value(
        source: &str,
        config: &ParserConfig,
    ) -> Result<Self, ExpressionError> {
        if let Some(expression) = Self::parse_legacy_qualified_value(source, config) {
            return Ok(expression);
        }
        Self::parse_configured(source.to_owned(), config)
    }

    fn parse_legacy_fortran_bracket_designator(
        source: &str,
        config: &ParserConfig,
    ) -> Result<Option<Self>, ExpressionError> {
        if !config.source_compatibility()
            || config.host_language() != HostLanguage::Fortran
            || !source.contains('[')
        {
            return Ok(None);
        }
        let contains_bracket_syntax =
            host::Lexer::for_expression_with_profile(source, config.profile())
                .tokenize()
                .map_err(host::ParseError::from)?
                .iter()
                .any(|token| matches!(token.kind, host::TokenKind::LeftBracket));
        if !contains_bracket_syntax {
            return Ok(None);
        }
        let source = source.to_owned().into_boxed_str();
        let ast = host::parse_expression_with_profile(
            &source,
            HostLanguageProfile::Cpp(CppStandard::Cpp23),
        )?;
        Ok(Some(Self {
            source,
            profile: config.profile(),
            ast,
        }))
    }

    fn parse_legacy_fortran_c_unary_designator(
        source: &str,
        config: &ParserConfig,
    ) -> Result<Option<Self>, ExpressionError> {
        if !config.source_compatibility()
            || config.host_language() != HostLanguage::Fortran
            || !matches!(source.trim_start().chars().next(), Some('*' | '&'))
        {
            return Ok(None);
        }
        let source = source.to_owned().into_boxed_str();
        let ast = host::parse_expression_with_profile(
            &source,
            HostLanguageProfile::Cpp(CppStandard::Cpp23),
        )?;
        Ok(Some(Self {
            source,
            profile: config.profile(),
            ast,
        }))
    }

    fn parse_configured(source: String, config: &ParserConfig) -> Result<Self, ExpressionError> {
        if let Some(expression) =
            Self::parse_legacy_fortran_bracket_designator(source.as_str(), config)?
        {
            return Ok(expression);
        }
        if let Some(expression) =
            Self::parse_legacy_fortran_c_unary_designator(source.as_str(), config)?
        {
            return Ok(expression);
        }
        if config.source_compatibility()
            && host::is_reserved_keyword(config.profile(), source.as_str())
            && let Ok(identifier) = host::Identifier::new(source.as_str())
        {
            let source = source.into_boxed_str();
            let ast = host::Expr::new(
                Span::entire(&source),
                host::ExprKind::Name(host::QualifiedName {
                    global: false,
                    segments: vec![identifier],
                }),
            );
            return Ok(Self {
                source,
                profile: config.profile(),
                ast,
            });
        }
        let source = source.into_boxed_str();
        let ast = if config.source_compatibility() {
            host::parse_expression_source_compatible_with_profile(&source, config.profile())?
        } else {
            host::parse_expression_with_profile(&source, config.profile())?
        };
        Ok(Self {
            source,
            profile: config.profile(),
            ast,
        })
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

    /// Exact source spelling covered by the root of this typed expression.
    ///
    /// This is intended for source-compatible adapters whose public contract
    /// preserves expression spelling. Semantic consumers should continue to
    /// inspect [`Self::ast`] and use [`fmt::Display`] for canonical rendering.
    #[must_use]
    pub fn source_spelling(&self) -> &str {
        self.ast
            .span
            .slice(&self.source)
            .expect("typed expression root span must match its source backing")
    }

    /// Render the typed host expression using the compact token separation
    /// used by the historical directive parsers.
    ///
    /// This walks the classified syntax tree; it does not rescan or normalize
    /// an opaque source string. Literal token spellings come from their checked
    /// AST spans so bases, suffixes, quoting, and case remain lossless.
    #[must_use]
    pub fn compact_source_spelling(&self) -> String {
        let mut rendered = String::new();
        render_compact_expression(&mut rendered, &self.source, self.language(), &self.ast);
        rendered
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

fn render_compact_expression(
    output: &mut String,
    source: &str,
    language: HostLanguage,
    expression: &Expr,
) {
    match &expression.kind {
        ExprKind::Literal(_) => output.push_str(
            expression
                .span
                .slice(source)
                .expect("typed expression leaf span must match its source backing"),
        ),
        ExprKind::Name(name) => {
            if name.global {
                output.push_str("::");
            }
            for (index, segment) in name.segments.iter().enumerate() {
                if index > 0 {
                    output.push_str("::");
                }
                output.push_str(segment.as_str());
            }
        }
        ExprKind::CppTemplateId {
            template,
            arguments,
        } => {
            render_compact_expression(output, source, language, template);
            output.push('<');
            for (index, argument) in arguments.iter().enumerate() {
                if index != 0 {
                    output.push(',');
                }
                match argument {
                    CppTemplateArgument::Type(type_name) => {
                        output.push_str(&type_name.compact_source_spelling());
                    }
                    CppTemplateArgument::Expression(expression)
                    | CppTemplateArgument::Ambiguous { expression, .. } => {
                        render_compact_expression(output, source, language, expression);
                    }
                }
            }
            output.push('>');
        }
        ExprKind::LegacyQualifiedInteger { .. } => output.extend(
            expression
                .span
                .slice(source)
                .expect("typed legacy expression span must match its source backing")
                .chars()
                .filter(|character| !character.is_whitespace()),
        ),
        ExprKind::Parenthesized(inner) => {
            output.push('(');
            render_compact_expression(output, source, language, inner);
            output.push(')');
        }
        ExprKind::Unary { op, operand } => {
            output.push_str(compact_unary_operator(*op, language));
            render_compact_expression(output, source, language, operand);
        }
        ExprKind::FortranDefinedUnary { operator, operand } => {
            output.push('.');
            output.push_str(operator.as_str());
            output.push('.');
            render_compact_expression(output, source, language, operand);
        }
        ExprKind::Binary { op, left, right } => {
            render_compact_expression(output, source, language, left);
            output.push_str(compact_binary_operator(*op, language));
            render_compact_expression(output, source, language, right);
        }
        ExprKind::FortranDefinedBinary {
            operator,
            left,
            right,
        } => {
            render_compact_expression(output, source, language, left);
            output.push('.');
            output.push_str(operator.as_str());
            output.push('.');
            render_compact_expression(output, source, language, right);
        }
        ExprKind::Conditional {
            condition,
            then_expr,
            else_expr,
        } => {
            render_compact_expression(output, source, language, condition);
            output.push('?');
            render_compact_expression(output, source, language, then_expr);
            output.push(':');
            render_compact_expression(output, source, language, else_expr);
        }
        ExprKind::Assignment { op, target, value } => {
            render_compact_expression(output, source, language, target);
            output.push_str(compact_assignment_operator(*op));
            render_compact_expression(output, source, language, value);
        }
        ExprKind::Call { callee, arguments } => {
            render_compact_expression(output, source, language, callee);
            output.push('(');
            for (index, argument) in arguments.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                render_compact_expression(output, source, language, argument);
            }
            output.push(')');
        }
        ExprKind::Subscript { base, subscript } => {
            render_compact_expression(output, source, language, base);
            output.push('[');
            match subscript {
                Subscript::Index(index) => {
                    render_compact_expression(output, source, language, index);
                }
                Subscript::Section(section) => {
                    render_compact_section(output, source, language, section);
                }
            }
            output.push(']');
        }
        ExprKind::Member {
            base,
            access,
            member,
        } => {
            render_compact_expression(output, source, language, base);
            output.push_str(match access {
                MemberAccess::Dot => ".",
                MemberAccess::Arrow => "->",
                MemberAccess::Scope => "::",
                MemberAccess::FortranComponent => "%",
            });
            output.push_str(member.as_str());
        }
        ExprKind::Postfix { op, operand } => {
            render_compact_expression(output, source, language, operand);
            output.push_str(match op {
                PostfixOp::Increment => "++",
                PostfixOp::Decrement => "--",
            });
        }
        ExprKind::FortranApply {
            designator,
            arguments,
        } => {
            render_compact_expression(output, source, language, designator);
            output.push('(');
            for (index, argument) in arguments.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                match argument {
                    FortranArgument::Positional(value) => {
                        render_compact_expression(output, source, language, value);
                    }
                    FortranArgument::Keyword { name, value } => {
                        output.push_str(name.as_str());
                        output.push('=');
                        render_compact_expression(output, source, language, value);
                    }
                    FortranArgument::Section(section) => {
                        render_compact_section(output, source, language, section);
                    }
                }
            }
            output.push(')');
        }
    }
}

fn render_compact_section(
    output: &mut String,
    source: &str,
    language: HostLanguage,
    section: &ArraySection,
) {
    if let Some(lower) = &section.lower {
        render_compact_expression(output, source, language, lower);
    }
    output.push(':');
    if let Some(upper_or_length) = &section.upper_or_length {
        render_compact_expression(output, source, language, upper_or_length);
    }
    if let Some(stride) = &section.stride {
        output.push(':');
        render_compact_expression(output, source, language, stride);
    }
}

const fn compact_unary_operator(operator: UnaryOp, language: HostLanguage) -> &'static str {
    match operator {
        UnaryOp::Plus => "+",
        UnaryOp::Minus => "-",
        UnaryOp::LogicalNot if matches!(language, HostLanguage::Fortran) => ".not.",
        UnaryOp::LogicalNot => "!",
        UnaryOp::BitwiseNot => "~",
        UnaryOp::Dereference => "*",
        UnaryOp::AddressOf => "&",
        UnaryOp::PreIncrement => "++",
        UnaryOp::PreDecrement => "--",
    }
}

const fn compact_binary_operator(operator: BinaryOp, language: HostLanguage) -> &'static str {
    match operator {
        BinaryOp::Power => "**",
        BinaryOp::Multiply => "*",
        BinaryOp::Divide => "/",
        BinaryOp::Remainder => "%",
        BinaryOp::Add => "+",
        BinaryOp::Subtract => "-",
        BinaryOp::Concatenate => "//",
        BinaryOp::ShiftLeft => "<<",
        BinaryOp::ShiftRight => ">>",
        BinaryOp::Less => "<",
        BinaryOp::LessEqual => "<=",
        BinaryOp::Greater => ">",
        BinaryOp::GreaterEqual => ">=",
        BinaryOp::Equal => "==",
        BinaryOp::NotEqual if matches!(language, HostLanguage::Fortran) => "/=",
        BinaryOp::NotEqual => "!=",
        BinaryOp::BitwiseAnd => "&",
        BinaryOp::BitwiseXor => "^",
        BinaryOp::BitwiseOr => "|",
        BinaryOp::LogicalAnd if matches!(language, HostLanguage::Fortran) => ".and.",
        BinaryOp::LogicalAnd => "&&",
        BinaryOp::LogicalOr if matches!(language, HostLanguage::Fortran) => ".or.",
        BinaryOp::LogicalOr => "||",
        BinaryOp::LogicalEqv => ".eqv.",
        BinaryOp::LogicalNeqv => ".neqv.",
        BinaryOp::Comma => ",",
    }
}

const fn compact_assignment_operator(operator: AssignmentOp) -> &'static str {
    match operator {
        AssignmentOp::Assign => "=",
        AssignmentOp::AddAssign => "+=",
        AssignmentOp::SubtractAssign => "-=",
        AssignmentOp::MultiplyAssign => "*=",
        AssignmentOp::DivideAssign => "/=",
        AssignmentOp::RemainderAssign => "%=",
        AssignmentOp::ShiftLeftAssign => "<<=",
        AssignmentOp::ShiftRightAssign => ">>=",
        AssignmentOp::BitwiseAndAssign => "&=",
        AssignmentOp::BitwiseXorAssign => "^=",
        AssignmentOp::BitwiseOrAssign => "|=",
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
    fn configured_constructor_cannot_bypass_source_compatibility() {
        let source = "this->ready";
        assert!(Expression::new(source, &ParserConfig::cpp()).is_err());

        let config = ParserConfig::cpp().with_source_compatibility(true);
        let expression = Expression::new(source, &config)
            .expect("the configured constructor must honor source compatibility");
        let ExprKind::Member {
            base,
            access: MemberAccess::Arrow,
            member,
        } = &expression.ast().kind
        else {
            panic!("expected a typed C++ member expression");
        };
        assert_eq!(member.as_str(), "ready");
        assert!(matches!(
            &base.kind,
            ExprKind::Name(name)
                if !name.global
                    && name.segments.len() == 1
                    && name.segments[0].as_str() == "this"
        ));
    }

    #[test]
    fn compact_cpp_template_arguments_keep_required_token_separators() {
        let expression = Expression::new("factory<unsigned long>()", &ParserConfig::cpp()).unwrap();
        assert_eq!(
            expression.compact_source_spelling(),
            "factory<unsigned long>()"
        );
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
