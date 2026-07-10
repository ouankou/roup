//! Strict, typed host-language expression syntax.
//!
//! This module intentionally has no opaque-expression escape hatch.  A parse
//! either produces a fully classified syntax tree or reports the first token
//! that is outside the implemented grammar.

#![forbid(unsafe_code)]

mod ast;
mod lexer;
mod parser;
mod render;
mod type_name;

pub use ast::*;
pub use lexer::{LexError, LexErrorKind, Lexer, Token, TokenKind};
pub use parser::{
    parse_expression, parse_expression_with_profile, ParseError, ParseErrorKind, Parser,
};
pub use render::CanonicalDisplay;
pub use type_name::{Delimiter as TypeNameDelimiter, TypeName, TypeNameError};
