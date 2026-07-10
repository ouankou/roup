//! A safe, strict OpenMP and OpenACC directive parser.
//!
//! The configured parsers in [`api`] return fully typed directive and clause
//! trees or one structured hard error. The default version policy accepts the
//! union of standardized historical syntax; exact policies reject only syntax
//! introduced after the selected specification version. Parsing never uses a
//! render-and-reparse path or a string-only expression fallback.
//!
//! The optional opaque-handle C ABI lives in the separate `roup-capi` workspace
//! package. This crate contains no unsafe Rust and can be built and used on its
//! own as the complete parser implementation.

#![forbid(unsafe_code)]

pub mod api;
pub mod ast;
pub mod availability;
mod delimiter;
pub mod diagnostic;
pub mod feature_availability;
pub mod host;
pub mod ir;
mod lexer;
mod parser;
pub mod source;
pub mod validation;
pub mod version;
