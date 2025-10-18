# Getting Started

This guide shows how to compile ROUP, add the crate to a Rust project, and link
the C/C++ bindings.

## Prerequisites

- Rust 1.85 or newer (`rustup` is recommended).
- A C/C++ toolchain (clang or GCC) when using the FFI bindings.
- Optional: a Fortran compiler for the example programs in `examples/fortran/`.

Clone and build the project:

```bash
git clone https://github.com/ouankou/roup.git
cd roup
cargo build --release
```

The build produces `libroup.so` on Linux, `libroup.dylib` on macOS, and
`roup.dll` on Windows inside `target/release/`.

## Rust quick start

Add ROUP to your `Cargo.toml`:

```toml
[dependencies]
roup = "0.5"
```

Example program:

```rust,ignore
use roup::parser::openmp;
use roup::lexer::Language;

fn main() {
    let input = "#pragma omp parallel for num_threads(4)";
    let parser = openmp::parser();

    match parser.parse_with_language(input, Language::C) {
        Ok((_, directive)) => {
            println!("directive: {}", directive.name);
            println!("clauses: {}", directive.clauses.len());
        }
        Err(err) => eprintln!("parse error: {err:?}"),
    }
}
```

Run it with `cargo run`.

## C quick start

Write a small program that calls the C API:

```c
#include <stdint.h>
#include <stdio.h>

struct OmpDirective;

struct OmpDirective* roup_parse(const char* input);
int32_t roup_directive_clause_count(const struct OmpDirective* dir);
void roup_directive_free(struct OmpDirective* dir);

int main(void) {
    struct OmpDirective* directive = roup_parse("#pragma omp parallel num_threads(4)");
    if (!directive) {
        fputs("parse failed\n", stderr);
        return 1;
    }

    printf("clause count: %d\n", roup_directive_clause_count(directive));
    roup_directive_free(directive);
    return 0;
}
```

Compile against the release build of ROUP:

```bash
cargo build --release
clang example.c \
  -L./target/release \
  -lroup -lpthread -ldl -lm \
  -Wl,-rpath,./target/release \
  -o example
./example
```

macOS users can replace the rpath with
`-Wl,-rpath,@executable_path/../target/release`.

## C++ quick start

The C API can be wrapped with RAII helpers:

```cpp
#include <cstdint>
#include <iostream>

extern "C" {
struct OmpDirective;
OmpDirective* roup_parse(const char* input);
int32_t roup_directive_clause_count(const OmpDirective* dir);
void roup_directive_free(OmpDirective* dir);
}

class Directive {
public:
    explicit Directive(const char* input) : ptr_(roup_parse(input)) {}
    ~Directive() { if (ptr_) roup_directive_free(ptr_); }
    Directive(const Directive&) = delete;
    Directive& operator=(const Directive&) = delete;
    Directive(Directive&& other) noexcept : ptr_(other.ptr_) { other.ptr_ = nullptr; }

    bool valid() const { return ptr_ != nullptr; }
    int32_t clause_count() const { return ptr_ ? roup_directive_clause_count(ptr_) : 0; }

private:
    OmpDirective* ptr_;
};

int main() {
    Directive directive("#pragma omp parallel for num_threads(4)");
    if (!directive.valid()) {
        std::cerr << "parse failed\n";
        return 1;
    }

    std::cout << "clauses: " << directive.clause_count() << "\n";
}
```

Compile with clang++ or g++ in the same way as the C example.

## Next steps

- Explore the complete examples in `examples/` (C, C++, Fortran).
- Read the [Rust tutorial](./rust-tutorial.md) for more detailed use cases.
- Consult the [Testing guide](../../TESTING.md) before contributing changes.
