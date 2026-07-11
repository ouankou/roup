use crate::source::Span;
use std::fmt;

use super::type_name::TypeName;

pub use crate::version::HostLanguage;

/// A validated source-language identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Identifier(String);

impl Identifier {
    pub fn new(text: impl Into<String>) -> Result<Self, IdentifierError> {
        let text = text.into();
        let mut chars = text.chars();
        let Some(first) = chars.next() else {
            return Err(IdentifierError::Empty);
        };
        if !(first == '_' || first.is_alphabetic()) {
            return Err(IdentifierError::InvalidStart(first));
        }
        if let Some(ch) = chars.find(|ch| !(*ch == '_' || ch.is_alphanumeric())) {
            return Err(IdentifierError::InvalidContinue(ch));
        }
        Ok(Self(text))
    }

    pub(crate) fn from_lexed(text: &str) -> Self {
        Self(text.to_owned())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentifierError {
    Empty,
    InvalidStart(char),
    InvalidContinue(char),
}

impl fmt::Display for IdentifierError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("identifier must not be empty"),
            Self::InvalidStart(character) => {
                write!(formatter, "identifier cannot start with `{character}`")
            }
            Self::InvalidContinue(character) => {
                write!(formatter, "identifier cannot contain `{character}`")
            }
        }
    }
}

impl std::error::Error for IdentifierError {}

/// C++-style qualified name.  C and Fortran parsers produce one segment.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QualifiedName {
    pub global: bool,
    pub segments: Vec<Identifier>,
}

impl QualifiedName {
    pub fn unqualified(identifier: Identifier) -> Self {
        Self {
            global: false,
            segments: vec![identifier],
        }
    }
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub span: Span,
    pub kind: ExprKind,
}

impl Expr {
    pub fn new(span: Span, kind: ExprKind) -> Self {
        Self { span, kind }
    }
}

/// Expression equality is structural. Source spans identify where a tree came
/// from, but they are not part of the expression's semantic shape.
impl PartialEq for Expr {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

/// A typed C++ template argument.
///
/// Whether a syntactically valid identifier denotes a type or a value depends
/// on C++ name lookup, which is deliberately outside this parser. When both
/// interpretations are valid, the AST retains both instead of guessing.
#[derive(Debug, Clone, PartialEq)]
pub enum CppTemplateArgument {
    Type(TypeName),
    Expression(Box<Expr>),
    Ambiguous {
        type_name: TypeName,
        expression: Box<Expr>,
    },
}

impl CppTemplateArgument {
    /// Returns the type-name interpretation when one is syntactically valid.
    pub fn type_name(&self) -> Option<&TypeName> {
        match self {
            Self::Type(type_name) | Self::Ambiguous { type_name, .. } => Some(type_name),
            Self::Expression(_) => None,
        }
    }

    /// Returns the expression interpretation when one is syntactically valid.
    pub fn expression(&self) -> Option<&Expr> {
        match self {
            Self::Expression(expression) | Self::Ambiguous { expression, .. } => Some(expression),
            Self::Type(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Literal(Literal),
    Name(QualifiedName),
    /// A C++ template-id used as an expression designator.
    CppTemplateId {
        template: Box<Expr>,
        arguments: Vec<CppTemplateArgument>,
    },
    /// A token shape accepted by the historical directive parsers even though
    /// it is not a host-language qualified name.
    LegacyQualifiedInteger {
        qualifier: Identifier,
        value: IntegerLiteral,
    },
    Parenthesized(Box<Expr>),
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    /// A user-defined Fortran dotted operator in prefix position.
    FortranDefinedUnary {
        operator: Identifier,
        operand: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    /// A user-defined Fortran dotted operator between two operands.
    FortranDefinedBinary {
        operator: Identifier,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Conditional {
        condition: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },
    Assignment {
        op: AssignmentOp,
        target: Box<Expr>,
        value: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        arguments: Vec<Expr>,
    },
    Subscript {
        base: Box<Expr>,
        subscript: Subscript,
    },
    Member {
        base: Box<Expr>,
        access: MemberAccess,
        member: Identifier,
    },
    Postfix {
        op: PostfixOp,
        operand: Box<Expr>,
    },
    /// In Fortran, `name(...)` cannot be classified as a call versus an array
    /// reference without symbol information.  Keeping the ambiguity explicit
    /// avoids guessing or falling back to text.
    FortranApply {
        designator: Box<Expr>,
        arguments: Vec<FortranArgument>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Boolean(bool),
    NullPointer,
    Integer(IntegerLiteral),
    Real(RealLiteral),
    Character(CharacterLiteral),
    String(StringLiteral),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntegerBase {
    Binary,
    Octal,
    Decimal,
    Hexadecimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CIntegerWidth {
    #[default]
    Default,
    Long,
    LongLong,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CIntegerSuffix {
    pub unsigned: bool,
    pub width: CIntegerWidth,
    pub size_t: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FortranKind {
    Numeric(u32),
    Named(Identifier),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IntegerSuffix {
    None,
    C(CIntegerSuffix),
    Fortran(FortranKind),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IntegerLiteral {
    pub base: IntegerBase,
    pub value: u128,
    pub suffix: IntegerSuffix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RealExponentKind {
    E,
    D,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RealExponent {
    pub kind: RealExponentKind,
    pub negative: bool,
    pub magnitude: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CRealSuffix {
    Float,
    Double,
    LongDouble,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RealSuffix {
    C(CRealSuffix),
    Fortran(Option<FortranKind>),
}

/// An exact decimal real literal.  The mathematical significand is
/// `coefficient * 10^-fractional_digits`; no source spelling or lossy `f64`
/// is carried into the AST.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RealLiteral {
    pub coefficient: u128,
    pub fractional_digits: u32,
    pub exponent: Option<RealExponent>,
    pub suffix: RealSuffix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CharacterEncoding {
    Ordinary,
    Utf8,
    Utf16,
    Utf32,
    Wide,
    Fortran,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CharacterLiteral {
    pub encoding: CharacterEncoding,
    pub value: char,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StringLiteral {
    pub encoding: CharacterEncoding,
    pub delimiter: StringDelimiter,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StringDelimiter {
    SingleQuote,
    DoubleQuote,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Plus,
    Minus,
    LogicalNot,
    BitwiseNot,
    Dereference,
    AddressOf,
    PreIncrement,
    PreDecrement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PostfixOp {
    Increment,
    Decrement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    Power,
    Multiply,
    Divide,
    Remainder,
    Add,
    Subtract,
    Concatenate,
    ShiftLeft,
    ShiftRight,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Equal,
    NotEqual,
    BitwiseAnd,
    BitwiseXor,
    BitwiseOr,
    LogicalAnd,
    LogicalOr,
    LogicalEqv,
    LogicalNeqv,
    Comma,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssignmentOp {
    Assign,
    AddAssign,
    SubtractAssign,
    MultiplyAssign,
    DivideAssign,
    RemainderAssign,
    ShiftLeftAssign,
    ShiftRightAssign,
    BitwiseAndAssign,
    BitwiseXorAssign,
    BitwiseOrAssign,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemberAccess {
    Dot,
    Arrow,
    Scope,
    FortranComponent,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Subscript {
    Index(Box<Expr>),
    Section(ArraySection),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SectionSemantics {
    /// C/C++ OpenMP array section: `lower:length[:stride]`.
    CLength,
    /// Fortran section triplet: `lower:upper[:stride]`.
    FortranUpperBound,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArraySection {
    pub semantics: SectionSemantics,
    pub lower: Option<Box<Expr>>,
    pub upper_or_length: Option<Box<Expr>>,
    pub stride: Option<Box<Expr>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FortranArgument {
    Positional(Expr),
    Keyword { name: Identifier, value: Expr },
    Section(ArraySection),
}
