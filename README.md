# ROUP: Rust OpenMP Parser

ROUP is an experimental parser for OpenMP directives written in Rust.  It exposes a
safe Rust API together with C and C++ bindings so tools can analyse or transform
`#pragma omp` constructs without relying on a full compiler front-end.

> **Status:** active development.  Expect breaking changes while the parser and
> its public APIs continue to evolve.

## Getting started

### Use from Rust
Add the crate to your `Cargo.toml`:

```toml
[dependencies]
roup = "0.4"
```

Example usage:

```rust
use roup::parser::openmp;

fn main() {
    let parser = openmp::parser();
    let input = "#pragma omp parallel for num_threads(4)";

    match parser.parse(input) {
        Ok((_, directive)) => {
            println!("directive: {}", directive.name);
            println!("clauses: {}", directive.clauses.len());
        }
        Err(err) => eprintln!("parse error: {err:?}"),
    }
}
```

### Use from C or C++
Build the library and link against the generated artefacts:

```bash
cargo build --release
# Linux/macOS: target/release/libroup.{so,dylib}
# Windows:     target/release/roup.dll
```

Minimal C program:

```c
#include <stdint.h>
#include <stdio.h>

struct OmpDirective;

struct OmpDirective* roup_parse(const char* input);
int32_t roup_directive_clause_count(const struct OmpDirective* dir);
void roup_directive_free(struct OmpDirective* dir);

int main(void) {
    struct OmpDirective* dir = roup_parse("#pragma omp parallel for num_threads(4)");
    if (!dir) {
        fputs("parse failed\n", stderr);
        return 1;
    }

    printf("clauses: %d\n", roup_directive_clause_count(dir));
    roup_directive_free(dir);
    return 0;
}
```

See `examples/c/` and `examples/cpp/` for complete buildable samples.  Fortran
interop examples live in `examples/fortran/`.

## Building and testing

```bash
# Build the library
cargo build --release

# Run the available Rust tests
cargo test

# Optional helper scripts
./test.sh                 # full project checks (requires mdBook, clang, cmake, etc.)
./test_rust_versions.sh   # run the CI toolchain matrix locally
```

The helper scripts expect the ompparser compatibility submodule to be checked
out (`git submodule update --init --recursive`) and require external tools such
as CMake, a C/C++ toolchain, and mdBook.

## Documentation

The mdBook sources live in `docs/book`.  Build them locally with:

```bash
cargo install mdbook
mdbook build docs/book
```

Generated pages end up in `docs/book/book/`.  The same directory structure is
used by the CI workflow to publish combined mdBook and rustdoc output.

## Project layout

```
src/                 core library and FFI bindings
compat/ompparser/    drop-in replacement layer for the legacy ompparser API
docs/                reference material and mdBook sources
examples/            language specific examples (C, C++, Fortran)
tests/               integration suites that exercise the parser
utils/               helper binaries used in development
```

## License

ROUP is distributed under the terms of the BSD-3-Clause license.  See
[`LICENSE`](LICENSE) for details.
