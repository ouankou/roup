# ROUP: Rust-based OpenMP Parser

**Safe, fast, and comprehensive OpenMP directive parsing**

[![Docs](https://img.shields.io/badge/docs-roup.ouankou.com-blue)](https://roup.ouankou.com)
[![Tests](https://img.shields.io/badge/tests-352%20passing-green)](https://github.com/ouankou/roup)
[![Safety](https://img.shields.io/badge/unsafe-0.9%25-yellow)](https://roup.ouankou.com/architecture.html#safety-boundaries)
[![Status](https://img.shields.io/badge/status-experimental-orange)](https://github.com/ouankou/roup)

> **⚠️ Experimental**: ROUP is under active development and not production-ready. APIs may change. Use for research and experimentation.

---

## 🚀 Quick Start

**5-Minute Setup:** Visit [roup.ouankou.com/getting-started.html](https://roup.ouankou.com/getting-started.html)

### Rust
```toml
[dependencies]
roup = "0.3"
```

### C/C++
```bash
cargo build --release
# Link against target/release/libroup.{a,so,dylib}
```

**Full guides:** [Building Documentation](https://roup.ouankou.com/building.html)

## Features

- ✅ **Multi-Language Support:** Rust, C, and C++ APIs
- ✅ **OpenMP 3.0-6.0:** 95 directives, 91 clauses
- ✅ **Safe by Default:** 99.1% safe Rust code
- ✅ **Experimental:** 352 tests, active development
- ✅ **Modern C++:** C++17 RAII wrappers
- ✅ **Well Documented:** [Comprehensive website](https://roup.ouankou.com)
- 🔄 **ompparser Compatible:** Drop-in replacement layer ([see below](#ompparser-compatibility))

## Documentation

📚 **Complete documentation at [roup.ouankou.com](https://roup.ouankou.com)**

**Quick links:**
- [Getting Started](https://roup.ouankou.com/getting-started.html) - 5-minute setup
- [Rust Tutorial](https://roup.ouankou.com/rust-tutorial.html) - Complete Rust guide
- [C Tutorial](https://roup.ouankou.com/c-tutorial.html) - C API with examples
- [C++ Tutorial](https://roup.ouankou.com/cpp-tutorial.html) - C++17 RAII wrappers
- [Building Guide](https://roup.ouankou.com/building.html) - Build for any platform
- [API Reference](https://roup.ouankou.com/api-reference.html) - Complete API docs
- [Architecture](https://roup.ouankou.com/architecture.html) - Internal design
- [FAQ](https://roup.ouankou.com/faq.html) - Common questions
- [Contributing](https://roup.ouankou.com/contributing.html) - How to contribute

## Language Support

| Language | API Style | Documentation |
|----------|-----------|---------------|
| **Rust** | Idiomatic Rust | [Rust Tutorial](https://roup.ouankou.com/rust-tutorial.html) |
| **C** | Pointer-based (malloc/free) | [C Tutorial](https://roup.ouankou.com/c-tutorial.html) |
| **C++** | C++17 RAII wrappers | [C++ Tutorial](https://roup.ouankou.com/cpp-tutorial.html) |

**Examples:** See [`examples/c/tutorial_basic.c`](examples/c/tutorial_basic.c) and [`examples/cpp/`](examples/cpp/) directory.

## Quick Examples

### Rust

```rust
use roup::parser::parse;

let input = "#pragma omp parallel for num_threads(4) private(i)";
match parse(input) {
    Ok(directive) => {
        println!("Directive: {:?}", directive.kind);
        println!("Clauses: {}", directive.clauses.len());
    }
    Err(e) => eprintln!("Parse error: {}", e),
}
```

### C

```c
#include <stdio.h>

// Forward declarations from libroup
typedef struct OmpDirective OmpDirective;
OmpDirective* roup_parse(const char* input);
int32_t roup_directive_clause_count(const OmpDirective* dir);
void roup_directive_free(OmpDirective* dir);

int main() {
    OmpDirective* dir = roup_parse("#pragma omp parallel for num_threads(4)");
    if (dir) {
        printf("Clauses: %d\n", roup_directive_clause_count(dir));
        roup_directive_free(dir);
    }
    return 0;
}
```

**Compile:**
```bash
cargo build --release
gcc example.c -L./target/release -lroup -lpthread -ldl -lm -o example
```

**Full guide:** [C Tutorial](https://roup.ouankou.com/c-tutorial.html)

### C++

```cpp
#include <iostream>
#include <memory>

// C FFI declarations
struct OmpDirective;
extern "C" {
    OmpDirective* roup_parse(const char* input);
    int32_t roup_directive_clause_count(const OmpDirective* dir);
    void roup_directive_free(OmpDirective* dir);
}

// RAII wrapper
class Directive {
    OmpDirective* ptr_;
public:
    explicit Directive(const char* input) : ptr_(roup_parse(input)) {}
    ~Directive() { if (ptr_) roup_directive_free(ptr_); }
    bool valid() const { return ptr_ != nullptr; }
    int clause_count() const { return ptr_ ? roup_directive_clause_count(ptr_) : 0; }
};

int main() {
    Directive dir("#pragma omp parallel for num_threads(4)");
    if (dir.valid()) {
        std::cout << "Clauses: " << dir.clause_count() << "\n";
    }
    return 0;
}
```

**Compile:**
```bash
cargo build --release
g++ -std=c++17 example.cpp -L./target/release -lroup -lpthread -ldl -lm -o example
```

**Full guide:** [C++ Tutorial](https://roup.ouankou.com/cpp-tutorial.html)

## Build and Test

```bash
# Build library
cargo build --release

# Run all tests (352 tests)
cargo test

# Build documentation
cargo doc --no-deps --open
```

**Platform-specific instructions:** [Building Guide](https://roup.ouankou.com/building.html)

## OpenMP Support

ROUP supports **OpenMP 3.0 through 6.0** with comprehensive coverage:

- **95 directives**: `parallel`, `for`, `task`, `target`, `teams`, `metadirective`, and more
- **91 clauses**: `private`, `reduction`, `schedule`, `map`, `depend`, and many others
- **Version tracking**: Know which OpenMP version introduced each feature

See the [OpenMP Support Matrix](https://roup.ouankou.com/openmp-support.html) for complete details.

## Project Structure

```
src/
├── lib.rs              - Rust library entry point
├── c_api.rs            - C FFI (16 functions, ~60 lines unsafe)
├── lexer.rs            - Tokenizer using nom
└── parser/             - Parser modules
    ├── mod.rs          - Parser entry points
    ├── clause.rs       - Clause parsing (91 types)
    ├── directive.rs    - Directive parsing (95 types)
    └── openmp.rs       - OpenMP-specific definitions
examples/
├── c/                  - C examples (tutorial_basic.c)
└── cpp/                - C++ examples
tests/                  - 342 integration tests
docs/
└── book/               - mdBook documentation website
```

**Architecture details:** [Architecture Guide](https://roup.ouankou.com/architecture.html)

## Safety

ROUP is written in **99.1% safe Rust** with minimal unsafe code (~60 lines).

**Unsafe code exists only in `src/c_api.rs` for:**
- Reading C strings (`CStr::from_ptr`)
- Writing to output pointers (NULL-checked)
- Converting between Rust `Box` and C raw pointers

**All unsafe code is:**
- ✅ NULL-checked before use
- ✅ Documented with safety contracts
- ✅ Isolated to FFI boundary
- ✅ Thoroughly tested

**Details:** [Architecture - Safety Boundaries](https://roup.ouankou.com/architecture.html#safety-boundaries)

## Compiler Requirements

| Language | Compiler | Version | Standard |
|----------|----------|---------|----------|
| Rust | rustc | 1.70+ | Edition 2021 |
| C | Clang | 10+ | C99 |
| C++ | Clang++ | 10+ | C++17 |

**Note:** GCC also works—just replace `clang`/`clang++` with `gcc`/`g++`.

## Testing

```bash
# Run all tests
cargo test

# Expected output:
# test result: ok. 352 passed; 0 failed
```

**Test Coverage:**
- ✅ All directive types (parallel, for, task, target, metadirective)
- ✅ All clause types (private, reduction, schedule, map, depend)
- ✅ Edge cases (comments, whitespace, malformed input)
- ✅ FFI safety (NULL handling, memory management)
- ✅ Roundtrip parsing (parse → format → parse)

**See also:** [Contributing - Testing Guidelines](https://roup.ouankou.com/contributing.html#testing-guidelines)

## C API

ROUP exports **16 C functions** for seamless C/C++ integration:

**Core functions:**
- `roup_parse()` - Parse OpenMP directive
- `roup_directive_free()` - Free directive
- `roup_directive_kind()` - Get directive type
- `roup_directive_clause_count()` - Get clause count
- `roup_directive_clauses_iter()` - Create clause iterator
- `roup_clause_kind()` - Get clause type
- And more...

**Full API reference:** [API Documentation](https://roup.ouankou.com/api-reference.html#c-api-reference)

## ompparser Compatibility

🔄 **Drop-in replacement for existing compilers**

ROUP provides a compatibility layer for compilers currently using [ompparser](https://github.com/ouankou/ompparser). Switch to ROUP without changing your code:

```cpp
#include <OpenMPIR.h>  // Same header as ompparser

// Same API - works identically
OpenMPDirective* dir = parseOpenMP("#pragma omp parallel", nullptr);
std::cout << dir->toString() << std::endl;
delete dir;
```

**Benefits:**
- ✅ **Zero code changes** - Same API, same behavior
- ✅ **Safer** - 99.1% safe Rust instead of C++ with manual memory management
- ✅ **Faster** - Rust optimizations and modern parser design
- ✅ **Well-tested** - Validated against ompparser's own test suite

**Setup:**

```bash
```bash
# Quick start - one command builds everything
cd compat/ompparser
./BUILD.sh

# Or manual build:
# 1. Initialize submodule
git submodule update --init --recursive

# 2. Build ROUP and compatibility layer
cd compat/ompparser
mkdir build && cd build
cmake .. && make

# 3. Run tests
ctest --output-on-failure
```

**Why submodule?** We use ompparser's actual headers (not copies) to guarantee perfect binary compatibility!

**Documentation:**
- [Compatibility Layer Guide](compat/ompparser/README.md)
- [Full Documentation](docs/book/src/ompparser-compat.md)

**Status:** ⚠️ **Experimental** - 46 tests passing, actively developed

## Contributing

We welcome contributions! Please see our [Contributing Guide](https://roup.ouankou.com/contributing.html) for:

- Development setup
- Code quality standards
- Testing requirements
- Pull request process

**Questions?** Check the [FAQ](https://roup.ouankou.com/faq.html) or [open a discussion](https://github.com/ouankou/roup/discussions).

## Learning Resources

ROUP demonstrates Rust concepts from basics to advanced:

- **Basics:** Structs, enums, pattern matching, ownership
- **Intermediate:** Traits, generics, error handling, modules
- **Advanced:** Parser combinators (nom), FFI, unsafe boundaries

**For learners:**
- Read the [Architecture Guide](https://roup.ouankou.com/architecture.html)
- Study the [examples](examples/) directory
- Check the commit history for evolution

---

## License

MIT License - see [LICENSE](LICENSE) file for details.

**Copyright © 2024-2025 Anjia Wang**
