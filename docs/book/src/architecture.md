# Architecture

ROUP has one semantic parser implementation and two delivery layers.

## Safe Rust parser

The workspace root package is the complete parser. It accepts an explicit
dialect version policy, host-language profile, and source form, and returns a
typed OpenMP or OpenACC AST. Every expression and structured clause payload is
parsed before the result is returned. Unknown syntax, invalid combinations,
unsupported host syntax, and trailing input are hard errors.

The root crate builds only an `rlib` and has `#![forbid(unsafe_code)]`. Its
semantic enums have Rust-native layouts and no ABI discriminants. Syntax that
was standardized by an older specification remains accepted by later exact
version modes. Standard aliases are recognized at the parser boundary and map
to one canonical semantic node.

Parsing is organized into four boundaries:

1. The source-form lexer validates the pragma or Fortran sentinel and line
   continuation rules.
2. The grammar recognizes directive and clause syntax without inventing
   defaults for malformed input.
3. Semantic construction creates the typed AST and host-expression trees.
4. Availability and context validation intersect every used feature with the
   configured specification and reject invalid clause, nesting, and association
   combinations.

Diagnostics carry stable codes and checked UTF-8 byte, line, and column spans.

## Optional C ABI

`crates/roup-capi` is a separate workspace package. It depends on the safe Rust
parser, but the parser never depends on it. The ABI uses opaque generational
handles, by-value options, explicit byte buffers, and structured error handles.
All foreign-pointer access is confined to one audited boundary module; every
other ABI module denies unsafe code.

Directive parameters and clause payloads are exposed as typed fields. A missing
typed conversion is a hard error. The repository document
`docs/C_ABI_ARCHITECTURE.md` defines the detailed ownership and layout rules.

## Compatibility adapters

`compat/ompparser` and `compat/accparser` build against the optional C ABI and
construct the corresponding upstream C++ IR directly from typed queries. They
do not link to Rust enum layouts and do not reinterpret canonical strings.
Unsupported conversions are hard errors.

The upstream projects are pinned git submodules. A repository test requires
both worktrees to match their recorded gitlinks, builds each adapter from a
clean CMake directory, and runs the upstream ctest suites.
