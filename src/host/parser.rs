use crate::host::ast::{
    ArraySection, AssignmentOp, BinaryOp, Expr, ExprKind, FortranArgument, HostLanguage, Literal,
    MemberAccess, PostfixOp, QualifiedName, SectionSemantics, Subscript, UnaryOp,
};
use crate::host::lexer::{LexError, LexErrorKind, Lexer, Token, TokenKind};
use crate::source::{SourceError, Span};
use crate::version::{FortranStandard, HostLanguageProfile};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub span: Span,
    pub kind: ParseErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    Lexical(LexErrorKind),
    UnexpectedToken {
        expected: &'static str,
        found: &'static str,
    },
    TrailingToken(&'static str),
    EmptySubscript,
    TooManySectionColons,
    InvalidAssignmentTarget,
    InvalidIncrementTarget,
    UnsupportedConstruct(&'static str),
    NotAvailableInProfile(&'static str),
    NestingLimitExceeded,
    InvalidSpan(SourceError),
    InternalInvariant(&'static str),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ParseErrorKind::Lexical(error) => error.fmt(f),
            ParseErrorKind::UnexpectedToken { expected, found } => {
                write!(f, "expected {expected}, found {found}")
            }
            ParseErrorKind::TrailingToken(found) => {
                write!(f, "unexpected trailing token {found}")
            }
            ParseErrorKind::EmptySubscript => f.write_str("subscript cannot be empty"),
            ParseErrorKind::TooManySectionColons => {
                f.write_str("array section has more than two colon separators")
            }
            ParseErrorKind::InvalidAssignmentTarget => {
                f.write_str("left side of assignment is not assignable")
            }
            ParseErrorKind::InvalidIncrementTarget => {
                f.write_str("increment/decrement operand is not assignable")
            }
            ParseErrorKind::UnsupportedConstruct(feature) => {
                write!(f, "unsupported expression construct: {feature}")
            }
            ParseErrorKind::NotAvailableInProfile(feature) => {
                write!(
                    f,
                    "{feature} is not available in the configured host language standard"
                )
            }
            ParseErrorKind::NestingLimitExceeded => {
                f.write_str("expression nesting limit exceeded")
            }
            ParseErrorKind::InvalidSpan(error) => write!(f, "invalid parser source span: {error}"),
            ParseErrorKind::InternalInvariant(message) => {
                write!(f, "parser invariant failed: {message}")
            }
        }
    }
}

impl std::error::Error for ParseError {}

impl From<LexError> for ParseError {
    fn from(error: LexError) -> Self {
        Self {
            span: error.span,
            kind: ParseErrorKind::Lexical(error.kind),
        }
    }
}

pub fn parse_expression(source: &str, language: HostLanguage) -> Result<Expr, ParseError> {
    Parser::new(source, language)?.parse()
}

pub fn parse_expression_with_profile(
    source: &str,
    profile: HostLanguageProfile,
) -> Result<Expr, ParseError> {
    Parser::with_profile(source, profile)?.parse()
}

/// Strict Pratt parser over typed tokens.
pub struct Parser<'a> {
    source: &'a str,
    profile: HostLanguageProfile,
    language: HostLanguage,
    tokens: Vec<Token>,
    cursor: usize,
    recursion_depth: u16,
}

impl<'a> Parser<'a> {
    pub fn new(source: &'a str, language: HostLanguage) -> Result<Self, ParseError> {
        Self::with_profile(source, HostLanguageProfile::latest(language))
    }

    pub fn with_profile(source: &'a str, profile: HostLanguageProfile) -> Result<Self, ParseError> {
        let language = profile.language();
        let tokens = Lexer::with_profile(source, profile).tokenize()?;
        Ok(Self {
            source,
            profile,
            language,
            tokens,
            cursor: 0,
            recursion_depth: 0,
        })
    }

    pub fn parse(mut self) -> Result<Expr, ParseError> {
        let expression = self.parse_binding_power(0)?;
        if !matches!(self.current().kind, TokenKind::End) {
            return Err(ParseError {
                span: self.current().span,
                kind: ParseErrorKind::TrailingToken(self.current().kind.description()),
            });
        }
        Ok(expression)
    }

    fn parse_binding_power(&mut self, minimum: u8) -> Result<Expr, ParseError> {
        // Each grammar recursion uses several Rust call frames.  Keeping this
        // deliberately conservative protects small test/embedded stacks too.
        const MAX_RECURSION_DEPTH: u16 = 64;
        if self.recursion_depth == MAX_RECURSION_DEPTH {
            return Err(ParseError {
                span: self.current().span,
                kind: ParseErrorKind::NestingLimitExceeded,
            });
        }
        self.recursion_depth += 1;
        let result = self.parse_binding_power_inner(minimum);
        self.recursion_depth -= 1;
        result
    }

    fn parse_binding_power_inner(&mut self, minimum: u8) -> Result<Expr, ParseError> {
        let mut left = self.parse_prefix()?;

        loop {
            if self.is_postfix_start() {
                left = self.parse_postfix(left)?;
                continue;
            }

            if self.language != HostLanguage::Fortran
                && matches!(self.current().kind, TokenKind::Question)
            {
                const CONDITIONAL_BP: u8 = 3;
                if CONDITIONAL_BP < minimum {
                    break;
                }
                self.advance();
                let then_expr = self.parse_binding_power(0)?;
                self.consume_expected(|kind| matches!(kind, TokenKind::Colon), "`:`")?;
                // The third operand is an assignment-expression in C/C++, so
                // assignments bind here while a top-level comma does not.
                let else_expr = self.parse_binding_power(2)?;
                let span = self.join(left.span, else_expr.span)?;
                left = Expr::new(
                    span,
                    ExprKind::Conditional {
                        condition: Box::new(left),
                        then_expr: Box::new(then_expr),
                        else_expr: Box::new(else_expr),
                    },
                );
                continue;
            }

            if let Some((op, left_bp, right_bp)) = self.assignment_operator() {
                if left_bp < minimum {
                    break;
                }
                if !is_assignable(&left) {
                    return Err(ParseError {
                        span: left.span,
                        kind: ParseErrorKind::InvalidAssignmentTarget,
                    });
                }
                self.advance();
                let value = self.parse_binding_power(right_bp)?;
                let span = self.join(left.span, value.span)?;
                left = Expr::new(
                    span,
                    ExprKind::Assignment {
                        op,
                        target: Box::new(left),
                        value: Box::new(value),
                    },
                );
                continue;
            }

            let Some((op, left_bp, right_bp)) = self.binary_operator() else {
                break;
            };
            if left_bp < minimum {
                break;
            }
            self.advance();
            let right = self.parse_binding_power(right_bp)?;
            let span = self.join(left.span, right.span)?;
            left = Expr::new(
                span,
                ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            );
        }

        Ok(left)
    }

    fn parse_prefix(&mut self) -> Result<Expr, ParseError> {
        let token = self.current().clone();
        let unary = match (&token.kind, self.language) {
            (TokenKind::Plus, _) => Some((UnaryOp::Plus, self.prefix_binding_power(false))),
            (TokenKind::Minus, _) => Some((UnaryOp::Minus, self.prefix_binding_power(false))),
            (TokenKind::LogicalNot, HostLanguage::Fortran) => {
                Some((UnaryOp::LogicalNot, self.prefix_binding_power(true)))
            }
            (TokenKind::LogicalNot, _) => {
                Some((UnaryOp::LogicalNot, self.prefix_binding_power(false)))
            }
            (TokenKind::BitwiseNot, HostLanguage::C | HostLanguage::Cpp) => {
                Some((UnaryOp::BitwiseNot, self.prefix_binding_power(false)))
            }
            (TokenKind::Star, HostLanguage::C | HostLanguage::Cpp) => {
                Some((UnaryOp::Dereference, self.prefix_binding_power(false)))
            }
            (TokenKind::Ampersand, HostLanguage::C | HostLanguage::Cpp) => {
                Some((UnaryOp::AddressOf, self.prefix_binding_power(false)))
            }
            (TokenKind::PlusPlus, HostLanguage::C | HostLanguage::Cpp) => {
                Some((UnaryOp::PreIncrement, self.prefix_binding_power(false)))
            }
            (TokenKind::MinusMinus, HostLanguage::C | HostLanguage::Cpp) => {
                Some((UnaryOp::PreDecrement, self.prefix_binding_power(false)))
            }
            _ => None,
        };
        if let Some((op, binding_power)) = unary {
            self.advance();
            let operand = self.parse_binding_power(binding_power)?;
            if matches!(op, UnaryOp::PreIncrement | UnaryOp::PreDecrement)
                && !is_assignable(&operand)
            {
                return Err(ParseError {
                    span: operand.span,
                    kind: ParseErrorKind::InvalidIncrementTarget,
                });
            }
            let span = self.join(token.span, operand.span)?;
            return Ok(Expr::new(
                span,
                ExprKind::Unary {
                    op,
                    operand: Box::new(operand),
                },
            ));
        }

        match token.kind {
            TokenKind::Boolean(value) => {
                self.advance();
                Ok(Expr::new(
                    token.span,
                    ExprKind::Literal(Literal::Boolean(value)),
                ))
            }
            TokenKind::NullPointer => {
                self.advance();
                Ok(Expr::new(
                    token.span,
                    ExprKind::Literal(Literal::NullPointer),
                ))
            }
            TokenKind::Integer(value) => {
                self.advance();
                Ok(Expr::new(
                    token.span,
                    ExprKind::Literal(Literal::Integer(value)),
                ))
            }
            TokenKind::Real(value) => {
                self.advance();
                Ok(Expr::new(
                    token.span,
                    ExprKind::Literal(Literal::Real(value)),
                ))
            }
            TokenKind::Character(value) => {
                self.advance();
                Ok(Expr::new(
                    token.span,
                    ExprKind::Literal(Literal::Character(value)),
                ))
            }
            TokenKind::String(value) => {
                self.advance();
                Ok(Expr::new(
                    token.span,
                    ExprKind::Literal(Literal::String(value)),
                ))
            }
            TokenKind::Identifier(_) | TokenKind::Scope => self.parse_name(),
            TokenKind::LeftParen => {
                self.advance();
                let inner = self.parse_binding_power(0)?;
                let close =
                    self.consume_expected(|kind| matches!(kind, TokenKind::RightParen), "`)`")?;
                let span = self.join(token.span, close.span)?;
                Ok(Expr::new(span, ExprKind::Parenthesized(Box::new(inner))))
            }
            _ => Err(ParseError {
                span: token.span,
                kind: ParseErrorKind::UnexpectedToken {
                    expected: "expression",
                    found: token.kind.description(),
                },
            }),
        }
    }

    fn parse_name(&mut self) -> Result<Expr, ParseError> {
        let start = self.current().span;
        let global = if matches!(self.current().kind, TokenKind::Scope) {
            if self.language != HostLanguage::Cpp {
                return Err(ParseError {
                    span: self.current().span,
                    kind: ParseErrorKind::UnsupportedConstruct(
                        "qualified names are available only in C++",
                    ),
                });
            }
            self.advance();
            true
        } else {
            false
        };

        let mut segments = Vec::new();
        loop {
            let token = self.consume_expected(
                |kind| matches!(kind, TokenKind::Identifier(_)),
                "identifier",
            )?;
            let token_span = token.span;
            let TokenKind::Identifier(identifier) = token.kind else {
                return Err(ParseError {
                    span: token_span,
                    kind: ParseErrorKind::InternalInvariant(
                        "identifier expectation returned a different token",
                    ),
                });
            };
            segments.push(identifier);
            if !matches!(self.current().kind, TokenKind::Scope) {
                let span = self.join(start, token_span)?;
                return Ok(Expr::new(
                    span,
                    ExprKind::Name(QualifiedName { global, segments }),
                ));
            }
            if self.language != HostLanguage::Cpp {
                return Err(ParseError {
                    span: self.current().span,
                    kind: ParseErrorKind::UnsupportedConstruct(
                        "qualified names are available only in C++",
                    ),
                });
            }
            self.advance();
        }
    }

    fn is_postfix_start(&self) -> bool {
        matches!(
            (&self.current().kind, self.language),
            (TokenKind::LeftParen, _)
                | (TokenKind::LeftBracket, HostLanguage::C | HostLanguage::Cpp)
                | (
                    TokenKind::Dot | TokenKind::Arrow,
                    HostLanguage::C | HostLanguage::Cpp
                )
                | (TokenKind::Percent, HostLanguage::Fortran)
                | (
                    TokenKind::PlusPlus | TokenKind::MinusMinus,
                    HostLanguage::C | HostLanguage::Cpp
                )
        )
    }

    fn parse_postfix(&mut self, expression: Expr) -> Result<Expr, ParseError> {
        match (&self.current().kind, self.language) {
            (TokenKind::LeftParen, HostLanguage::C | HostLanguage::Cpp) => {
                let arguments = self.parse_call_arguments()?;
                let end = self.previous().span;
                let span = self.join(expression.span, end)?;
                Ok(Expr::new(
                    span,
                    ExprKind::Call {
                        callee: Box::new(expression),
                        arguments,
                    },
                ))
            }
            (TokenKind::LeftParen, HostLanguage::Fortran) => {
                let arguments = self.parse_fortran_arguments()?;
                let end = self.previous().span;
                let span = self.join(expression.span, end)?;
                Ok(Expr::new(
                    span,
                    ExprKind::FortranApply {
                        designator: Box::new(expression),
                        arguments,
                    },
                ))
            }
            (TokenKind::LeftBracket, HostLanguage::C | HostLanguage::Cpp) => {
                let subscript = self.parse_c_subscript()?;
                let end = self.previous().span;
                let span = self.join(expression.span, end)?;
                Ok(Expr::new(
                    span,
                    ExprKind::Subscript {
                        base: Box::new(expression),
                        subscript,
                    },
                ))
            }
            (TokenKind::Dot | TokenKind::Arrow, HostLanguage::C | HostLanguage::Cpp) => {
                let access = if matches!(self.current().kind, TokenKind::Dot) {
                    MemberAccess::Dot
                } else {
                    MemberAccess::Arrow
                };
                self.advance();
                let member_token = self.consume_expected(
                    |kind| matches!(kind, TokenKind::Identifier(_)),
                    "member identifier",
                )?;
                let member_span = member_token.span;
                let TokenKind::Identifier(member) = member_token.kind else {
                    return Err(ParseError {
                        span: member_span,
                        kind: ParseErrorKind::InternalInvariant(
                            "member expectation returned a different token",
                        ),
                    });
                };
                let span = self.join(expression.span, member_span)?;
                Ok(Expr::new(
                    span,
                    ExprKind::Member {
                        base: Box::new(expression),
                        access,
                        member,
                    },
                ))
            }
            (TokenKind::Percent, HostLanguage::Fortran) => {
                self.advance();
                let member_token = self.consume_expected(
                    |kind| matches!(kind, TokenKind::Identifier(_)),
                    "component identifier",
                )?;
                let member_span = member_token.span;
                let TokenKind::Identifier(member) = member_token.kind else {
                    return Err(ParseError {
                        span: member_span,
                        kind: ParseErrorKind::InternalInvariant(
                            "component expectation returned a different token",
                        ),
                    });
                };
                let span = self.join(expression.span, member_span)?;
                Ok(Expr::new(
                    span,
                    ExprKind::Member {
                        base: Box::new(expression),
                        access: MemberAccess::FortranComponent,
                        member,
                    },
                ))
            }
            (TokenKind::PlusPlus | TokenKind::MinusMinus, HostLanguage::C | HostLanguage::Cpp) => {
                if !is_assignable(&expression) {
                    return Err(ParseError {
                        span: expression.span,
                        kind: ParseErrorKind::InvalidIncrementTarget,
                    });
                }
                let op_token = self.advance().clone();
                let op = if matches!(op_token.kind, TokenKind::PlusPlus) {
                    PostfixOp::Increment
                } else {
                    PostfixOp::Decrement
                };
                let span = self.join(expression.span, op_token.span)?;
                Ok(Expr::new(
                    span,
                    ExprKind::Postfix {
                        op,
                        operand: Box::new(expression),
                    },
                ))
            }
            _ => Err(ParseError {
                span: self.current().span,
                kind: ParseErrorKind::InternalInvariant(
                    "postfix parser called without a postfix token",
                ),
            }),
        }
    }

    fn parse_call_arguments(&mut self) -> Result<Vec<Expr>, ParseError> {
        self.consume_expected(|kind| matches!(kind, TokenKind::LeftParen), "`(`")?;
        let mut arguments = Vec::new();
        if matches!(self.current().kind, TokenKind::RightParen) {
            self.advance();
            return Ok(arguments);
        }
        loop {
            arguments.push(self.parse_binding_power(2)?);
            if matches!(self.current().kind, TokenKind::Comma) {
                self.advance();
                if matches!(self.current().kind, TokenKind::RightParen) {
                    return Err(ParseError {
                        span: self.current().span,
                        kind: ParseErrorKind::UnexpectedToken {
                            expected: "expression after `,`",
                            found: self.current().kind.description(),
                        },
                    });
                }
                continue;
            }
            self.consume_expected(|kind| matches!(kind, TokenKind::RightParen), "`)`")?;
            return Ok(arguments);
        }
    }

    fn parse_c_subscript(&mut self) -> Result<Subscript, ParseError> {
        self.consume_expected(|kind| matches!(kind, TokenKind::LeftBracket), "`[`")?;
        let lower = if matches!(
            self.current().kind,
            TokenKind::Colon | TokenKind::RightBracket
        ) {
            None
        } else {
            Some(Box::new(self.parse_binding_power(0)?))
        };

        if matches!(self.current().kind, TokenKind::Colon) {
            self.advance();
            let upper_or_length = if matches!(
                self.current().kind,
                TokenKind::Colon | TokenKind::RightBracket
            ) {
                None
            } else {
                Some(Box::new(self.parse_binding_power(0)?))
            };
            let stride = if matches!(self.current().kind, TokenKind::Colon) {
                self.advance();
                if matches!(self.current().kind, TokenKind::RightBracket) {
                    None
                } else {
                    Some(Box::new(self.parse_binding_power(0)?))
                }
            } else {
                None
            };
            if matches!(self.current().kind, TokenKind::Colon) {
                return Err(ParseError {
                    span: self.current().span,
                    kind: ParseErrorKind::TooManySectionColons,
                });
            }
            self.consume_expected(|kind| matches!(kind, TokenKind::RightBracket), "`]`")?;
            return Ok(Subscript::Section(ArraySection {
                semantics: SectionSemantics::CLength,
                lower,
                upper_or_length,
                stride,
            }));
        }

        let Some(index) = lower else {
            return Err(ParseError {
                span: self.current().span,
                kind: ParseErrorKind::EmptySubscript,
            });
        };
        self.consume_expected(|kind| matches!(kind, TokenKind::RightBracket), "`]`")?;
        Ok(Subscript::Index(index))
    }

    fn parse_fortran_arguments(&mut self) -> Result<Vec<FortranArgument>, ParseError> {
        self.consume_expected(|kind| matches!(kind, TokenKind::LeftParen), "`(`")?;
        let mut arguments = Vec::new();
        if matches!(self.current().kind, TokenKind::RightParen) {
            self.advance();
            return Ok(arguments);
        }

        loop {
            let argument = if matches!(self.current().kind, TokenKind::Colon) {
                FortranArgument::Section(self.parse_fortran_section(None)?)
            } else if let TokenKind::Identifier(name) = &self.current().kind {
                if self
                    .tokens
                    .get(self.cursor + 1)
                    .is_some_and(|token| matches!(token.kind, TokenKind::Equal))
                {
                    if matches!(
                        self.profile,
                        HostLanguageProfile::Fortran(FortranStandard::Fortran77)
                    ) {
                        return Err(ParseError {
                            span: self.current().span,
                            kind: ParseErrorKind::NotAvailableInProfile("Fortran keyword argument"),
                        });
                    }
                    let name = name.clone();
                    self.advance();
                    self.advance();
                    FortranArgument::Keyword {
                        name,
                        value: self.parse_binding_power(0)?,
                    }
                } else {
                    let expression = self.parse_binding_power(0)?;
                    if matches!(self.current().kind, TokenKind::Colon) {
                        FortranArgument::Section(self.parse_fortran_section(Some(expression))?)
                    } else {
                        FortranArgument::Positional(expression)
                    }
                }
            } else {
                let expression = self.parse_binding_power(0)?;
                if matches!(self.current().kind, TokenKind::Colon) {
                    FortranArgument::Section(self.parse_fortran_section(Some(expression))?)
                } else {
                    FortranArgument::Positional(expression)
                }
            };
            arguments.push(argument);

            if matches!(self.current().kind, TokenKind::Comma) {
                self.advance();
                if matches!(self.current().kind, TokenKind::RightParen) {
                    return Err(ParseError {
                        span: self.current().span,
                        kind: ParseErrorKind::UnexpectedToken {
                            expected: "Fortran argument after `,`",
                            found: self.current().kind.description(),
                        },
                    });
                }
                continue;
            }
            self.consume_expected(|kind| matches!(kind, TokenKind::RightParen), "`)`")?;
            return Ok(arguments);
        }
    }

    fn parse_fortran_section(&mut self, lower: Option<Expr>) -> Result<ArraySection, ParseError> {
        if matches!(
            self.profile,
            HostLanguageProfile::Fortran(FortranStandard::Fortran77)
        ) {
            return Err(ParseError {
                span: self.current().span,
                kind: ParseErrorKind::NotAvailableInProfile("Fortran array section"),
            });
        }
        self.consume_expected(|kind| matches!(kind, TokenKind::Colon), "`:`")?;
        let upper = if matches!(
            self.current().kind,
            TokenKind::Colon | TokenKind::Comma | TokenKind::RightParen
        ) {
            None
        } else {
            Some(Box::new(self.parse_binding_power(0)?))
        };
        let stride = if matches!(self.current().kind, TokenKind::Colon) {
            self.advance();
            if matches!(
                self.current().kind,
                TokenKind::Comma | TokenKind::RightParen
            ) {
                None
            } else {
                Some(Box::new(self.parse_binding_power(0)?))
            }
        } else {
            None
        };
        if matches!(self.current().kind, TokenKind::Colon) {
            return Err(ParseError {
                span: self.current().span,
                kind: ParseErrorKind::TooManySectionColons,
            });
        }
        Ok(ArraySection {
            semantics: SectionSemantics::FortranUpperBound,
            lower: lower.map(Box::new),
            upper_or_length: upper,
            stride,
        })
    }

    fn prefix_binding_power(&self, logical_not: bool) -> u8 {
        match self.language {
            HostLanguage::C | HostLanguage::Cpp => 90,
            HostLanguage::Fortran if logical_not => 7,
            HostLanguage::Fortran => 15,
        }
    }

    fn binary_operator(&self) -> Option<(BinaryOp, u8, u8)> {
        let left = |op, precedence| Some((op, precedence, precedence + 1));
        let right = |op, precedence| Some((op, precedence, precedence));
        match self.language {
            HostLanguage::C | HostLanguage::Cpp => match self.current().kind {
                TokenKind::Comma => left(BinaryOp::Comma, 1),
                TokenKind::LogicalOr => left(BinaryOp::LogicalOr, 4),
                TokenKind::LogicalAnd => left(BinaryOp::LogicalAnd, 6),
                TokenKind::Pipe => left(BinaryOp::BitwiseOr, 8),
                TokenKind::Caret => left(BinaryOp::BitwiseXor, 10),
                TokenKind::Ampersand => left(BinaryOp::BitwiseAnd, 12),
                TokenKind::EqualEqual => left(BinaryOp::Equal, 14),
                TokenKind::NotEqual => left(BinaryOp::NotEqual, 14),
                TokenKind::Less => left(BinaryOp::Less, 16),
                TokenKind::LessEqual => left(BinaryOp::LessEqual, 16),
                TokenKind::Greater => left(BinaryOp::Greater, 16),
                TokenKind::GreaterEqual => left(BinaryOp::GreaterEqual, 16),
                TokenKind::ShiftLeft => left(BinaryOp::ShiftLeft, 18),
                TokenKind::ShiftRight => left(BinaryOp::ShiftRight, 18),
                TokenKind::Plus => left(BinaryOp::Add, 20),
                TokenKind::Minus => left(BinaryOp::Subtract, 20),
                TokenKind::Star => left(BinaryOp::Multiply, 22),
                TokenKind::Slash => left(BinaryOp::Divide, 22),
                TokenKind::Remainder => left(BinaryOp::Remainder, 22),
                _ => None,
            },
            HostLanguage::Fortran => match self.current().kind {
                TokenKind::LogicalEqv => left(BinaryOp::LogicalEqv, 2),
                TokenKind::LogicalNeqv => left(BinaryOp::LogicalNeqv, 2),
                TokenKind::LogicalOr => left(BinaryOp::LogicalOr, 4),
                TokenKind::LogicalAnd => left(BinaryOp::LogicalAnd, 6),
                TokenKind::EqualEqual => left(BinaryOp::Equal, 8),
                TokenKind::NotEqual => left(BinaryOp::NotEqual, 8),
                TokenKind::Less => left(BinaryOp::Less, 8),
                TokenKind::LessEqual => left(BinaryOp::LessEqual, 8),
                TokenKind::Greater => left(BinaryOp::Greater, 8),
                TokenKind::GreaterEqual => left(BinaryOp::GreaterEqual, 8),
                TokenKind::Concat => left(BinaryOp::Concatenate, 10),
                TokenKind::Plus => left(BinaryOp::Add, 12),
                TokenKind::Minus => left(BinaryOp::Subtract, 12),
                TokenKind::Star => left(BinaryOp::Multiply, 14),
                TokenKind::Slash => left(BinaryOp::Divide, 14),
                TokenKind::Power => right(BinaryOp::Power, 16),
                _ => None,
            },
        }
    }

    fn assignment_operator(&self) -> Option<(AssignmentOp, u8, u8)> {
        if self.language == HostLanguage::Fortran {
            return None;
        }
        let op = match self.current().kind {
            TokenKind::Equal => AssignmentOp::Assign,
            TokenKind::PlusEqual => AssignmentOp::AddAssign,
            TokenKind::MinusEqual => AssignmentOp::SubtractAssign,
            TokenKind::StarEqual => AssignmentOp::MultiplyAssign,
            TokenKind::SlashEqual => AssignmentOp::DivideAssign,
            TokenKind::RemainderEqual => AssignmentOp::RemainderAssign,
            TokenKind::ShiftLeftEqual => AssignmentOp::ShiftLeftAssign,
            TokenKind::ShiftRightEqual => AssignmentOp::ShiftRightAssign,
            TokenKind::AmpersandEqual => AssignmentOp::BitwiseAndAssign,
            TokenKind::CaretEqual => AssignmentOp::BitwiseXorAssign,
            TokenKind::PipeEqual => AssignmentOp::BitwiseOrAssign,
            _ => return None,
        };
        Some((op, 2, 2))
    }

    fn current(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.cursor - 1]
    }

    fn advance(&mut self) -> &Token {
        let index = self.cursor;
        if !matches!(self.tokens[index].kind, TokenKind::End) {
            self.cursor += 1;
        }
        &self.tokens[index]
    }

    fn consume_expected(
        &mut self,
        predicate: impl FnOnce(&TokenKind) -> bool,
        expected: &'static str,
    ) -> Result<Token, ParseError> {
        let token = self.current().clone();
        if predicate(&token.kind) {
            self.advance();
            Ok(token)
        } else {
            Err(ParseError {
                span: token.span,
                kind: ParseErrorKind::UnexpectedToken {
                    expected,
                    found: token.kind.description(),
                },
            })
        }
    }

    fn join(&self, first: Span, last: Span) -> Result<Span, ParseError> {
        Span::new(self.source, first.start_byte(), last.end_byte()).map_err(|error| ParseError {
            span: first,
            kind: ParseErrorKind::InvalidSpan(error),
        })
    }
}

fn is_assignable(expression: &Expr) -> bool {
    match &expression.kind {
        ExprKind::Name(_) | ExprKind::Member { .. } | ExprKind::Subscript { .. } => true,
        ExprKind::Parenthesized(inner) => is_assignable(inner),
        ExprKind::Unary {
            op: UnaryOp::Dereference,
            ..
        } => true,
        ExprKind::FortranApply { .. } => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_precedence_is_structural() {
        let expression = parse_expression("a + b * c", HostLanguage::C).unwrap();
        let ExprKind::Binary {
            op: BinaryOp::Add,
            right,
            ..
        } = expression.kind
        else {
            panic!("expected addition root")
        };
        assert!(matches!(
            right.kind,
            ExprKind::Binary {
                op: BinaryOp::Multiply,
                ..
            }
        ));
    }

    #[test]
    fn parses_cpp_qualified_postfix_chain_and_section() {
        let expression =
            parse_expression("::ns::f(obj.member, ptr->field)[lo:len]", HostLanguage::Cpp).unwrap();
        assert!(matches!(
            expression.kind,
            ExprKind::Subscript {
                subscript: Subscript::Section(_),
                ..
            }
        ));
    }

    #[test]
    fn parses_conditional_and_right_associative_assignment() {
        let expression = parse_expression("x = flag ? y : z = 2", HostLanguage::C).unwrap();
        assert!(matches!(
            expression.kind,
            ExprKind::Assignment {
                op: AssignmentOp::Assign,
                ..
            }
        ));
    }

    #[test]
    fn fortran_power_binds_more_tightly_than_unary_minus() {
        let expression = parse_expression("-2**2", HostLanguage::Fortran).unwrap();
        let ExprKind::Unary {
            op: UnaryOp::Minus,
            operand,
        } = expression.kind
        else {
            panic!("expected unary minus")
        };
        assert!(matches!(
            operand.kind,
            ExprKind::Binary {
                op: BinaryOp::Power,
                ..
            }
        ));
    }

    #[test]
    fn parses_fortran_component_and_section_triplet() {
        let expression =
            parse_expression("array(1:n:2, :)%field .ge. 0", HostLanguage::Fortran).unwrap();
        assert!(matches!(
            expression.kind,
            ExprKind::Binary {
                op: BinaryOp::GreaterEqual,
                ..
            }
        ));
    }

    #[test]
    fn rejects_unknown_syntax_instead_of_creating_opaque_node() {
        let error = parse_expression("a @ b", HostLanguage::Cpp).unwrap_err();
        assert!(matches!(error.kind, ParseErrorKind::Lexical(_)));
    }

    #[test]
    fn rejects_non_lvalue_assignment() {
        let error = parse_expression("(a + b) = c", HostLanguage::C).unwrap_err();
        assert_eq!(error.kind, ParseErrorKind::InvalidAssignmentTarget);
    }

    #[test]
    fn malformed_inputs_return_errors_for_every_language() {
        let cases = [
            (HostLanguage::C, ""),
            (HostLanguage::C, "a +"),
            (HostLanguage::C, "f(, x)"),
            (HostLanguage::C, "a["),
            (HostLanguage::Cpp, "::"),
            (HostLanguage::Cpp, "object->"),
            (HostLanguage::Cpp, "f(x,)"),
            (HostLanguage::Cpp, "a @ b"),
            (HostLanguage::Fortran, "array("),
            (HostLanguage::Fortran, "array(1::2:3)"),
            (HostLanguage::Fortran, ".unknown. value"),
            (HostLanguage::Fortran, "value /= "),
        ];

        for (language, source) in cases {
            assert!(
                parse_expression(source, language).is_err(),
                "{language:?} parser unexpectedly accepted {source:?}"
            );
        }
    }

    #[test]
    fn fortran_symbolic_not_equal_is_typed() {
        let expression = parse_expression("left /= right", HostLanguage::Fortran).unwrap();
        assert!(matches!(
            expression.kind,
            ExprKind::Binary {
                op: BinaryOp::NotEqual,
                ..
            }
        ));
    }

    #[test]
    fn excessive_nesting_is_an_error_not_a_panic() {
        let source = format!("{}x{}", "(".repeat(300), ")".repeat(300));
        let error = parse_expression(&source, HostLanguage::C).unwrap_err();
        assert_eq!(error.kind, ParseErrorKind::NestingLimitExceeded);
    }

    #[test]
    fn null_pointer_is_a_typed_literal() {
        for language in [HostLanguage::C, HostLanguage::Cpp] {
            let expression = parse_expression("nullptr", language).unwrap();
            assert!(matches!(
                expression.kind,
                ExprKind::Literal(Literal::NullPointer)
            ));
        }
    }

    #[test]
    fn adjacent_stars_remain_two_c_operators() {
        let expression = parse_expression("left **right", HostLanguage::C).unwrap();
        let ExprKind::Binary {
            op: BinaryOp::Multiply,
            right,
            ..
        } = expression.kind
        else {
            panic!("expected multiplication")
        };
        assert!(matches!(
            right.kind,
            ExprKind::Unary {
                op: UnaryOp::Dereference,
                ..
            }
        ));
    }
}
