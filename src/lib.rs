//! # Rust-based OpenMP/OpenACC Unified Parser (ROUP)
//!
//! ROUP is a standalone, unified parser for OpenMP and OpenACC, designed as an
//! extensible framework for directive-based programming interfaces.
//!
//! ## Learning from This Project
//!
//! This codebase is organized to teach Rust programming concepts step-by-step:
//!
//! 1. **Basics**: Structs, enums, lifetimes, pattern matching
//! 2. **Intermediate**: Modules, traits, HashMap/Option, builder pattern
//! 3. **Advanced**: Parser combinators using nom, function pointers, registries
//! 4. **IR Layer**: Semantic representation, enums for polymorphism, FFI design
//!
//! Study the git history to see how the project evolved!

// ============================================================================
// Module Organization
// ============================================================================
//
// This library is organized into focused modules:
//
// - `lexer`: Tokenization using nom parser combinators
// - `parser`: Directive and clause parsing infrastructure
// - `ir`: Intermediate representation (semantic layer)
// - `ffi`: Foreign function interface (C API, 100% safe Rust)
//
// Each module teaches different Rust concepts while building a working parser.

pub mod ffi;
pub mod ir;
pub mod lexer;
pub mod parser;
