# Frequently Asked Questions

## General

### What is ROUP?

ROUP is a Rust library that parses OpenMP directives and exposes the result to
Rust, C, C++, and Fortran consumers.  It focuses on analysing and transforming
existing OpenMP code rather than compiling it.

### Is ROUP production ready?

Not yet.  The project is actively developed and APIs may change between
releases.  Treat the current builds as experimental and review the release notes
for the latest status updates.

### Which OpenMP versions are supported?

The parser tracks the OpenMP 6.0 specification.  Integration tests exercise the
keyword registry, loop-transform directives, meta-directives, and device
constructs.  Unsupported constructs fail with descriptive parse errors instead
of being silently accepted.

## Installation

### How do I install ROUP for Rust?

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
roup = "0.4"
```

### How do I use ROUP from C or C++?

Build the library with `cargo build --release` and link against the generated
artefact (`libroup.so`, `libroup.dylib`, or `roup.dll`).  The C API is declared
in `compat/include/roup.h`, which is generated from `src/c_api.rs` by the build
script.

### What toolchains are required?

- Rust 1.85 or newer (matches the MSRV in `Cargo.toml`).
- A C/C++ compiler when using the FFI bindings.
- Optional: Fortran compiler for the Fortran examples.

## Usage

### How do I parse a directive?

```rust,no_run
use roup::parser::openmp;

let parser = openmp::parser();
let (_, directive) = parser.parse("#pragma omp parallel for").expect("parse");
println!("kind: {}", directive.name);
```

### How do I walk clauses from C?

```c
OmpClauseIterator* it = roup_directive_clauses_iter(dir);
OmpClause* clause = NULL;
while (roup_clause_iterator_next(it, &clause)) {
    printf("clause kind: %d\n", roup_clause_kind(clause));
}
roup_clause_iterator_free(it);
```

## Testing and contributing

### Which tests should I run before sending a change?

At minimum run `cargo test`.  For thorough coverage run `./test.sh` and
`./test_rust_versions.sh` (see `TESTING.md` for details).  These scripts build
examples, execute the ompparser compatibility tests, and verify the
documentation.

### Where can I learn more?

- [Getting started](./getting-started.md)
- [Architecture](./architecture.md)
- [OpenMP support overview](./openmp-support.md)
