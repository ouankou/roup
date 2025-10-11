# Rust-based OpenMP/OpenACC Unified Parser (ROUP)

ROUP is a standalone, unified parser for OpenMP and OpenACC, developed using Rust. It is designed as an extensible framework that can be expanded to support additional directive-based programming interfaces.

**🚀 Quick Start:** See [`docs/QUICK_START.md`](docs/QUICK_START.md) for 5-minute setup in Rust, C, or C++

## Features

- ✅ **Multi-Language Support:** Native Rust API + C/C++ FFI
- ✅ **OpenMP 5.0+:** 15+ directives, 50+ clauses
- ✅ **Safe by Default:** 99.75% safe Rust code
- ✅ **Production Ready:** 342 tests, zero warnings
- ✅ **Modern C++:** C++17 support with RAII wrappers
- ✅ **Well Documented:** Comprehensive tutorials and API docs

## Language Support

| Language | API Style | Tutorial |
|----------|-----------|----------|
| **Rust** | Idiomatic Rust types | Built-in docs (`cargo doc --open`) |
| **C** | Standard C API (pointers) | [`examples/c/tutorial_basic.c`](examples/c/tutorial_basic.c) (450+ lines, 6 steps) |
| **C++** | C++17 with RAII wrappers | [`examples/cpp/tutorial_basic.cpp`](examples/cpp/tutorial_basic.cpp) (600+ lines, 6 steps) |

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
clang example.c -L./target/release -lroup -Wl,-rpath,./target/release -o example
```

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
clang++ -std=c++17 example.cpp -L./target/release -lroup -Wl,-rpath,./target/release -o example
```

## Build and Test

```bash
# Build library (Rust + C FFI)
cargo build --release

# Run Rust tests (342 tests)
cargo test

# Build C tutorial
cd examples/c
clang tutorial_basic.c -L../../target/release -lroup -Wl,-rpath,../../target/release -o tutorial
./tutorial

# Build C++ tutorial
cd examples/cpp
clang++ -std=c++17 tutorial_basic.cpp -L../../target/release -lroup -Wl,-rpath,../../target/release -o tutorial
./tutorial
```

## OpenMP Support

ROUP supports **OpenMP 5.0+** with 15+ directives and 50+ clauses.

**Directives:** `parallel`, `for`, `sections`, `single`, `task`, `taskwait`, `target`, `teams`, `distribute`, `barrier`, `critical`, `atomic`, `metadirective`, and more.

**Clauses:** `private`, `shared`, `reduction`, `schedule`, `num_threads`, `if`, `map`, `device`, `default`, and many others.

See [`docs/OPENMP_SUPPORT.md`](docs/OPENMP_SUPPORT.md) for the complete feature matrix.

## Documentation

| Document | Description |
|----------|-------------|
| [`QUICK_START.md`](docs/QUICK_START.md) | 5-minute setup for Rust/C/C++ |
| [`C_FFI_STATUS.md`](docs/C_FFI_STATUS.md) | Complete C API reference |
| [`TUTORIAL_BUILDING_AND_RUNNING.md`](docs/TUTORIAL_BUILDING_AND_RUNNING.md) | Detailed build instructions |
| [`IMPLEMENTATION_SUMMARY.md`](docs/IMPLEMENTATION_SUMMARY.md) | Implementation details |
| [`DEVELOPMENT_HISTORY.md`](docs/DEVELOPMENT_HISTORY.md) | Project evolution (Phases 1-3) |
| [`examples/c/tutorial_basic.c`](examples/c/tutorial_basic.c) | C tutorial (450+ lines, 6 steps) |
| [`examples/cpp/tutorial_basic.cpp`](examples/cpp/tutorial_basic.cpp) | C++ tutorial (600+ lines, 6 steps) |

## Project Structure

```
src/
├── lib.rs              - Rust API + C API exports
├── c_api.rs            - C API with minimal unsafe (18 functions, 632 lines)
├── lexer.rs            - Tokenizer using nom
└── parser/             - Parser modules
    ├── mod.rs          - Parser entry points
    ├── clause.rs       - Clause parsing
    ├── directive.rs    - Directive parsing
    └── openmp.rs       - OpenMP-specific definitions
examples/
├── c/                  - C tutorial (tutorial_basic.c, 450+ lines)
└── cpp/                - C++ tutorial (tutorial_basic.cpp, 600+ lines)
tests/                  - 342 Rust integration tests (10 files)
docs/                   - Comprehensive documentation (15+ files)
```

## Safety

ROUP is written in **99.1% safe Rust** with only 18 unsafe blocks (~60 lines).

The minimal `unsafe` code exists only in the C API (`src/c_api.rs`) for:
- Converting C strings to Rust (`CStr::from_ptr`)
- Converting between Rust `Box` and C raw pointers (`Box::into_raw`, `Box::from_raw`)
- Dereferencing output pointers (NULL-checked)

All unsafe code is:
- **NULL-checked** before use
- **Documented** with safety requirements
- **Isolated** to the C API boundary
- **Tested** with both C and C++ tutorials

See [`docs/MINIMAL_UNSAFE_SUMMARY.md`](docs/MINIMAL_UNSAFE_SUMMARY.md) for detailed analysis.

## Compiler Requirements

| Language | Compiler | Version | Standard |
|----------|----------|---------|----------|
| Rust | rustc | 1.70+ | Edition 2021 |
| C | Clang | 10+ | C99 |
| C++ | Clang++ | 10+ | C++17 |

**Note:** GCC also works—just replace `clang`/`clang++` with `gcc`/`g++`.

## Testing

```bash
# Rust tests (342 tests)
cargo test

# Expected output:
# test result: ok. 342 passed; 0 failed; 1 ignored
```

**Test Coverage:**
- Basic directives (parallel, for, task, teams, target)
- Complex features (reductions, metadirectives, nesting)
- Edge cases (comments, whitespace, errors)
- Roundtrip parsing (parse → format → parse)

## C FFI API

ROUP exports **18 C functions** for integration with C/C++ projects:

**Lifecycle:**
- `roup_parse()` - Parse directive
- `roup_directive_free()` - Free directive
- `roup_clause_free()` - Free clause

**Queries:**
- `roup_directive_kind()` - Get directive type
- `roup_directive_clause_count()` - Clause count
- `roup_clause_kind()` - Get clause type
- `roup_clause_schedule_kind()` - Schedule details
- `roup_clause_reduction_operator()` - Reduction operator
- And more...

See [`docs/C_FFI_STATUS.md`](docs/C_FFI_STATUS.md) for complete API reference.

## Learning Resources

**For Rust Learners:**
This project demonstrates Rust concepts from basics to advanced:
1. **Basics:** Structs, enums, lifetimes, pattern matching
2. **Intermediate:** Modules, traits, collections, builder pattern
3. **Advanced:** Parser combinators, FFI, unsafe boundaries

Study the commit history and [`docs/DEVELOPMENT_HISTORY.md`](docs/DEVELOPMENT_HISTORY.md) to see how the project evolved!

## License

MIT License - see LICENSE file for details.
