# Quick Start Guide

Get up and running with Roup in 5 minutes.

---

## Choose Your Language

- [Rust](#rust-quick-start) - Safe, idiomatic Rust API
- [C](#c-quick-start) - Standard C API with minimal overhead
- [C++](#c-quick-start) - Modern C++17 with RAII wrappers

---

## Rust Quick Start

### 1. Add Dependency

```toml
[dependencies]
roup = "0.1.0"
```

### 2. Parse and Query

```rust
use roup::parser::parse;

fn main() {
    let input = "#pragma omp parallel for num_threads(4)";
    
    match parse(input) {
        Ok(directive) => {
            println!("Directive: {:?}", directive.kind);
            println!("Clauses: {}", directive.clauses.len());
        }
        Err(e) => eprintln!("Parse error: {}", e),
    }
}
```

### 3. Run

```bash
cargo build
cargo run
```

**Output:**
```
Directive: Parallel
Clauses: 2
```

**Learn More:** See `src/lib.rs` for full Rust API

---

## C Quick Start

### 1. Build the Library

```bash
cd /workspaces/roup
cargo build --release
```

This creates `target/release/libroup.so` (or `.dylib` on macOS, `.dll` on Windows).

### 2. Create Your Program

**example.c:**
```c
#include <stdio.h>
#include <stdint.h>

// Forward declarations
typedef struct OmpDirective OmpDirective;
OmpDirective* roup_parse(const char* input);
void roup_directive_free(OmpDirective* directive);
int32_t roup_directive_kind(const OmpDirective* directive);
int32_t roup_directive_clause_count(const OmpDirective* directive);

int main() {
    const char* input = "#pragma omp parallel for num_threads(4)";
    
    OmpDirective* dir = roup_parse(input);
    if (dir == NULL) {
        fprintf(stderr, "Parse failed\n");
        return 1;
    }
    
    printf("Directive kind: %d\n", roup_directive_kind(dir));
    printf("Clause count: %d\n", roup_directive_clause_count(dir));
    
    roup_directive_free(dir);
    return 0;
}
```

### 3. Compile and Run

```bash
clang example.c \
  -L/workspaces/roup/target/release \
  -lroup \
  -Wl,-rpath,/workspaces/roup/target/release \
  -o example

./example
```

**Output:**
```
Directive kind: 0
Clause count: 2
```

**Learn More:** See `examples/c/tutorial_basic.c` for comprehensive tutorial (265 lines, 8 steps)

---

## C++ Quick Start

### 1. Build the Library

```bash
cd /workspaces/roup
cargo build --release
```

### 2. Create Your Program

**example.cpp:**
```cpp
#include <iostream>
#include <memory>
#include <cstdint>

// Forward declarations
struct OmpDirective;
extern "C" {
    OmpDirective* roup_parse(const char* input);
    void roup_directive_free(OmpDirective* directive);
    int32_t roup_directive_kind(const OmpDirective* directive);
    int32_t roup_directive_clause_count(const OmpDirective* directive);
}

// RAII wrapper
class Directive {
    OmpDirective* ptr_;
public:
    explicit Directive(const char* input) : ptr_(roup_parse(input)) {}
    ~Directive() { if (ptr_) roup_directive_free(ptr_); }
    
    // Delete copy, allow move
    Directive(const Directive&) = delete;
    Directive& operator=(const Directive&) = delete;
    Directive(Directive&& other) noexcept : ptr_(other.ptr_) { other.ptr_ = nullptr; }
    Directive& operator=(Directive&& other) noexcept {
        if (this != &other) {
            if (ptr_) roup_directive_free(ptr_);
            ptr_ = other.ptr_;
            other.ptr_ = nullptr;
        }
        return *this;
    }
    
    bool valid() const { return ptr_ != nullptr; }
    int32_t kind() const { return ptr_ ? roup_directive_kind(ptr_) : -1; }
    int32_t clause_count() const { return ptr_ ? roup_directive_clause_count(ptr_) : 0; }
};

int main() {
    Directive dir("#pragma omp parallel for num_threads(4)");
    
    if (!dir.valid()) {
        std::cerr << "Parse failed\n";
        return 1;
    }
    
    std::cout << "Directive kind: " << dir.kind() << "\n";
    std::cout << "Clause count: " << dir.clause_count() << "\n";
    
    return 0;
}  // Automatic cleanup via RAII
```

### 3. Compile and Run

```bash
clang++ -std=c++17 example.cpp \
  -L/workspaces/roup/target/release \
  -lroup \
  -Wl,-rpath,/workspaces/roup/target/release \
  -o example

./example
```

**Output:**
```
Directive kind: 0
Clause count: 2
```

**Learn More:** See `examples/cpp/tutorial_basic.cpp` for comprehensive tutorial (450 lines, 6 steps, modern C++17)

---

## Common Directive Kinds (Returned by `roup_directive_kind()`)

| Value | Directive | Example |
|-------|-----------|---------|
| 0 | Parallel | `#pragma omp parallel` |
| 1 | For | `#pragma omp for` |
| 2 | Sections | `#pragma omp sections` |
| 3 | Single | `#pragma omp single` |
| 4 | Task | `#pragma omp task` |
| 5 | Target | `#pragma omp target` |
| 6 | Teams | `#pragma omp teams` |
| 7 | Distribute | `#pragma omp distribute` |

See `C_FFI_STATUS.md` for complete enum mapping.

---

## What's Supported?

✅ **Directives (15+):**
- Parallelism: `parallel`, `for`, `sections`, `single`, `master`
- Tasking: `task`, `taskwait`, `taskgroup`, `taskyield`
- Device: `target`, `teams`, `distribute`
- Sync: `barrier`, `critical`, `atomic`
- Advanced: `metadirective`, `declare variant`

✅ **Clauses (50+):**
- Scheduling: `schedule(static|dynamic|guided|auto|runtime)`
- Data: `private`, `shared`, `firstprivate`, `lastprivate`
- Reduction: `reduction(+|-|*|&|&&|etc:var_list)`
- Devices: `device(expr)`, `map(to|from|tofrom:var_list)`
- Control: `if(expr)`, `num_threads(expr)`, `default(shared|none)`

See `OPENMP_SUPPORT.md` for complete feature matrix.

---

## Next Steps

### For C Developers
📖 **Full Tutorial:** `examples/c/tutorial_basic.c`
- 8 comprehensive steps
- Error handling patterns
- Iterator usage
- 11 example constructs

### For C++ Developers
📖 **Full Tutorial:** `examples/cpp/tutorial_basic.cpp`
- Modern C++17 features
- RAII wrappers
- Exception handling
- `std::optional` and `std::string_view`

### For Rust Developers
📖 **API Docs:** Run `cargo doc --open`
- Full type documentation
- Example code
- Safety guarantees

### Build Instructions
📖 **Detailed Guide:** `docs/TUTORIAL_BUILDING_AND_RUNNING.md`
- Copy-paste ready commands
- Troubleshooting tips
- Platform-specific notes

---

## Compiler Requirements

| Language | Compiler | Version | Standard |
|----------|----------|---------|----------|
| Rust | rustc | 1.70+ | Edition 2021 |
| C | Clang | 10+ | C99 |
| C++ | Clang++ | 10+ | C++17 |

**Why Clang?** Better diagnostics, LLVM compatibility, modern C++ support.

GCC works too, just replace `clang`/`clang++` with `gcc`/`g++`.

---

## Troubleshooting

### "Library not found" Error

```bash
# Add library path to LD_LIBRARY_PATH (Linux)
export LD_LIBRARY_PATH=/workspaces/roup/target/release:$LD_LIBRARY_PATH

# Or use -Wl,-rpath during compilation (recommended)
clang example.c -L/path/to/lib -lroup -Wl,-rpath,/path/to/lib
```

### Parse Returns NULL

Check your OpenMP syntax:
```c
// ✅ Good
"#pragma omp parallel"

// ❌ Bad - missing 'omp'
"#pragma parallel"

// ❌ Bad - typo
"#pragma omp paralel"
```

### Compilation Errors

Make sure you're using C++17:
```bash
# ✅ Correct
clang++ -std=c++17 example.cpp ...

# ❌ Wrong - C++11 doesn't have std::string_view
clang++ -std=c++11 example.cpp ...
```

---

## Getting Help

1. **Check Examples:** `examples/c/` and `examples/cpp/`
2. **Read Tutorials:** `docs/TUTORIAL_BUILDING_AND_RUNNING.md`
3. **API Reference:** `docs/C_FFI_STATUS.md`
4. **Implementation:** `docs/IMPLEMENTATION_SUMMARY.md`
5. **Development History:** `docs/DEVELOPMENT_HISTORY.md`

---

## Summary

**Rust:**
```bash
cargo build && cargo run
```

**C:**
```bash
cargo build --release
clang example.c -L./target/release -lroup -Wl,-rpath,./target/release -o example
./example
```

**C++:**
```bash
cargo build --release
clang++ -std=c++17 example.cpp -L./target/release -lroup -Wl,-rpath,./target/release -o example
./example
```

**That's it!** You're now parsing OpenMP directives. 🎉
