# Getting Started

Get up and running with ROUP in 5 minutes. This guide covers installation, basic usage, and your first OpenMP parse for **Rust**, **C**, and **C++**.

---

## Installation

### Prerequisites

- **Rust** 1.70+ ([Install Rust](https://rustup.rs/))
- **C/C++ Compiler**: Clang 10+ or GCC 9+ (for C/C++ usage)

### Build ROUP

```bash
git clone https://github.com/ouankou/roup.git
cd roup
cargo build --release
```

This creates the library at:
- **Linux**: `target/release/libroup.so`
- **macOS**: `target/release/libroup.dylib`
- **Windows**: `target/release/roup.dll`

---

## Quick Start: Rust

### 1. Add Dependency

Add to your `Cargo.toml`:

```toml
[dependencies]
roup = "0.2"
```

Or use the git version:

```toml
[dependencies]
roup = { git = "https://github.com/ouankou/roup.git" }
```

### 2. Parse Your First Directive

```rust
use roup::parser::openmp::parse_openmp_directive;
use roup::lexer::Language;

fn main() {
    let input = "#pragma omp parallel for num_threads(4)";
    
    match parse_openmp_directive(input, Language::C) {
        Ok(directive) => {
            println!("✓ Successfully parsed: {:?}", directive.kind);
            println!("  Clauses: {}", directive.clauses.len());
            
            for clause in &directive.clauses {
                println!("  - {:?}", clause);
            }
        }
        Err(e) => {
            eprintln!("✗ Parse error: {}", e);
        }
    }
}
```

### 3. Run

```bash
cargo run
```

**Output:**
```
✓ Successfully parsed: ParallelFor
  Clauses: 1
  - NumThreads(Expr { value: "4", .. })
```

**Next:** See the [Rust Tutorial](./rust-tutorial.md) for advanced usage.

---

## Quick Start: C

### 1. Include the Header

The C API is defined in `include/roup.h`:

```c
#include "include/roup.h"
#include <stdio.h>

int main() {
    // Parse a directive
    Handle directive;
    OmpStatus status = omp_parse_cstr(
        "#pragma omp parallel num_threads(4)", 
        OMP_LANG_C, 
        &directive
    );
    
    if (status != OMP_SUCCESS) {
        fprintf(stderr, "Parse failed with status: %d\n", status);
        return 1;
    }
    
    // Query directive kind
    int32_t kind;
    omp_directive_kind_ptr(directive, &kind);
    printf("Directive kind: %d\n", kind);
    
    // Query clause count
    uintptr_t count;
    omp_directive_clause_count_ptr(directive, &count);
    printf("Clause count: %zu\n", count);
    
    // Clean up
    omp_directive_free(directive);
    
    return 0;
}
```

### 2. Compile

```bash
clang example.c \
  -I./include \
  -L./target/release \
  -lroup \
  -Wl,-rpath,./target/release \
  -o example
```

**On macOS:**
```bash
clang example.c \
  -I./include \
  -L./target/release \
  -lroup \
  -Wl,-rpath,@executable_path/../target/release \
  -o example
```

### 3. Run

```bash
./example
```

**Output:**
```
Directive kind: 0
Clause count: 1
```

**Next:** See the [C Tutorial](./c-tutorial.md) for complete examples with error handling.

---

## Quick Start: C++

### 1. Create Your Program

Modern C++17 with RAII wrappers (see [C++ Tutorial](./cpp-tutorial.md) for full implementation):

```cpp
#include "include/roup.h"
#include <iostream>
#include <memory>

// Simple RAII wrapper
class Directive {
    Handle handle_;
public:
    explicit Directive(const char* input) {
        omp_parse_cstr(input, OMP_LANG_C, &handle_);
    }
    
    ~Directive() {
        if (handle_ != INVALID_HANDLE) {
            omp_directive_free(handle_);
        }
    }
    
    // Delete copy, allow move
    Directive(const Directive&) = delete;
    Directive& operator=(const Directive&) = delete;
    Directive(Directive&& other) noexcept 
        : handle_(other.handle_) {
        other.handle_ = INVALID_HANDLE;
    }
    
    bool valid() const { 
        return handle_ != INVALID_HANDLE; 
    }
    
    int32_t kind() const {
        int32_t k = -1;
        omp_directive_kind_ptr(handle_, &k);
        return k;
    }
    
    size_t clause_count() const {
        uintptr_t count = 0;
        omp_directive_clause_count_ptr(handle_, &count);
        return count;
    }
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

### 2. Compile

```bash
clang++ -std=c++17 example.cpp \
  -I./include \
  -L./target/release \
  -lroup \
  -Wl,-rpath,./target/release \
  -o example
```

### 3. Run

```bash
./example
```

**Output:**
```
Directive kind: 28
Clause count: 1
```

**Next:** See the [C++ Tutorial](./cpp-tutorial.md) for a complete real-world example.

---

## Understanding Directive Kinds

The C/C++ API returns integer discriminants for directive kinds. Here are the most common ones:

| Value | Directive | Example |
|-------|-----------|---------|
| 0 | `parallel` | `#pragma omp parallel` |
| 1 | `for` | `#pragma omp for` |
| 5 | `task` | `#pragma omp task` |
| 15 | `target` | `#pragma omp target` |
| 21 | `teams` | `#pragma omp teams` |
| 28 | `parallel for` | `#pragma omp parallel for` |
| 61 | `metadirective` | `#pragma omp metadirective` |

**See the complete enum mapping in the [API Reference](./api-reference.md#directive-kinds-directivekind-enum).**

---

## What's Supported?

### Directives (74 total)
✅ Parallelism: `parallel`, `for`, `sections`, `single`, `master`  
✅ Tasking: `task`, `taskwait`, `taskgroup`, `taskloop`  
✅ Device offloading: `target`, `teams`, `distribute`  
✅ Synchronization: `barrier`, `critical`, `atomic`  
✅ Advanced: `metadirective`, `declare variant`, `loop`  

### Clauses (92 total)
✅ Scheduling: `schedule`, `collapse`, `ordered`  
✅ Data-sharing: `private`, `shared`, `firstprivate`, `lastprivate`  
✅ Reductions: `reduction(+|-|*|&|&&|min|max:vars)`  
✅ Device clauses: `device`, `map`, `is_device_ptr`  
✅ Control: `if`, `num_threads`, `default`  

**For the complete support matrix, see [OpenMP Support](./openmp-support.md)** (coming soon).

---

## Troubleshooting

### Library Not Found at Runtime

**Linux:**
```bash
export LD_LIBRARY_PATH=$PWD/target/release:$LD_LIBRARY_PATH
./example
```

Or use `-Wl,-rpath` during compilation (recommended):
```bash
clang example.c -L./target/release -lroup -Wl,-rpath,$PWD/target/release
```

**macOS:**
```bash
export DYLD_LIBRARY_PATH=$PWD/target/release:$DYLD_LIBRARY_PATH
./example
```

### Parse Returns Invalid Handle

Check your OpenMP syntax:

```c
// ✅ Valid
omp_parse_cstr("#pragma omp parallel", OMP_LANG_C, &handle);

// ✅ Valid - Fortran syntax
omp_parse_cstr("!$omp parallel", OMP_LANG_FORTRAN, &handle);

// ❌ Invalid - missing 'omp'
omp_parse_cstr("#pragma parallel", OMP_LANG_C, &handle);
```

Always check the return status:
```c
OmpStatus status = omp_parse_cstr(input, OMP_LANG_C, &handle);
if (status != OMP_SUCCESS) {
    fprintf(stderr, "Parse error: %d\n", status);
}
```

### C++ Compilation Errors

Make sure you're using C++17:

```bash
# ✅ Correct
clang++ -std=c++17 example.cpp ...

# ❌ Wrong - C++11 lacks required features
clang++ -std=c++11 example.cpp ...
```

---

## Next Steps

### Learn More
- **[Rust Tutorial](./rust-tutorial.md)** - Idiomatic Rust patterns (coming soon)
- **[C Tutorial](./c-tutorial.md)** - Complete C examples (coming soon)
- **[C++ Tutorial](./cpp-tutorial.md)** - Real-world application
- **[API Reference](./api-reference.md)** - Complete C/Rust API docs

### Examples
- `examples/c/` - 5 complete C examples
- `examples/cpp/` - 3 complete C++ examples with RAII

### Advanced Topics
- Error handling strategies
- Iterator patterns for clauses
- Thread safety considerations
- Performance optimization

---

## Summary

**Rust:**
```bash
cargo add roup
# Write code, then:
cargo run
```

**C:**
```bash
cargo build --release
clang example.c -I./include -L./target/release -lroup -Wl,-rpath,./target/release
./example
```

**C++:**
```bash
cargo build --release
clang++ -std=c++17 example.cpp -I./include -L./target/release -lroup -Wl,-rpath,./target/release
./example
```

**You're now ready to parse OpenMP directives!** 🎉
