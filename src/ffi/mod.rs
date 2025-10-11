//! # FFI (Foreign Function Interface) Layer
//!
//! This module provides a **100% safe Rust** C-compatible API for the OpenMP parser.
//! It uses a handle-based design to avoid raw pointers and `unsafe` code entirely.
//!
//! ## Design Philosophy
//!
//! Traditional FFI approaches use raw pointers (`*const T`, `*mut T`) which require
//! `unsafe` code. Instead, we use **opaque handles** (integer IDs) that reference
//! Rust-owned resources stored in a global registry.
//!
//! ```text
//! Traditional FFI:           Handle-Based FFI (This Design):
//! ┌─────────────┐           ┌─────────────┐
//! │ C Code      │           │ C Code      │
//! │  char* ptr ─┼──────>    │  uint64_t h │ (just a number)
//! └─────────────┘  unsafe   └──────┬──────┘
//!                                   │ safe lookup
//!                            ┌──────▼──────┐
//!                            │ Registry    │
//!                            │  handles    │
//!                            │  -> data    │
//!                            └─────────────┘
//! ```
//!
//! ## Learning Objectives
//!
//! 1. **Zero-unsafe FFI**: Proving FFI doesn't require `unsafe` with clever design
//! 2. **Handle-based APIs**: Opaque integer handles for resource management
//! 3. **Interior mutability**: `Mutex` for thread-safe shared state
//! 4. **Error handling**: Status codes instead of panics across FFI boundary
//! 5. **Resource lifecycle**: Deterministic cleanup without garbage collection
//!
//! ## Key Concepts
//!
//! ### Opaque Handles
//! All resources are identified by `u64` handles:
//! - **String handles**: UTF-8 byte sequences built by C
//! - **AST handles**: Parsed directive trees
//! - **Node handles**: Individual directives/clauses
//! - **Cursor handles**: Iterators over children/clauses
//!
//! ### Global Registry
//! A thread-safe global registry owns all resources:
//! ```ignore
//! static REGISTRY: Mutex<Registry> {
//!     asts: HashMap<u64, DirectiveIR>,
//!     strings: HashMap<u64, ByteString>,
//!     cursors: HashMap<u64, Cursor>,
//! }
//! ```
//!
//! ### No Panics
//! All functions return status codes or 0 for invalid handles.
//! Panics are caught at the boundary (future work: panic hook).
//!
//! ## Module Organization
//!
//! - `registry`: Global handle storage and allocation
//! - `types`: C-compatible enums and status codes
//! - `string`: String building API
//! - `parse`: Parser integration
//! - `directive`: Directive query functions
//! - `clause`: Clause query functions
//! - `cursor`: Iterator pattern implementation

pub mod registry;
pub mod string;
pub mod types;

// Re-export main types
pub use registry::Registry;
pub use string::*;
pub use types::*;
