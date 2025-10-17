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
roup = "0.3"
```

Or use the git version:

```toml
[dependencies]
roup = { git = "https://github.com/ouankou/roup.git" }
```

### 2. Parse Your First Directive

```rust,ignore
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
```text
✓ Successfully parsed: ParallelFor
  Clauses: 1
  - NumThreads(Expr { value: "4", .. })
```

**Next:** See the [Rust Tutorial](./rust-tutorial.md) for advanced usage.

---

## Quick Start: C

### 1. Write Your Program

The C API uses direct pointers (malloc/free pattern):

```c
#include <stdio.h>
#include <stdint.h>

// Forward declarations from libroup
typedef struct OmpDirective OmpDirective;

OmpDirective* roup_parse(const char* input);
int32_t roup_directive_clause_count(const OmpDirective* dir);
void roup_directive_free(OmpDirective* dir);

int main() {
    // Parse a directive
    OmpDirective* directive = roup_parse("#pragma omp parallel num_threads(4)");
    
    if (!directive) {
        fprintf(stderr, "Parse failed\n");
        return 1;
    }
    
    // Query clause count
    int32_t count = roup_directive_clause_count(directive);
    printf("Clause count: %d\n", count);
    
    // Clean up
    roup_directive_free(directive);
    
    return 0;
}
```

### 2. Compile

```bash
# Build ROUP library first
cargo build --release

# Compile your C program
clang example.c \
  -L./target/release \
  -lroup \
  -lpthread -ldl -lm \
  -Wl,-rpath,./target/release \
  -o example
```

**On macOS:**
```bash
clang example.c \
  -L./target/release \
  -lroup \
  -lpthread -ldl \
  -Wl,-rpath,@executable_path/../target/release \
  -o example
```

### 3. Run

```bash
./example
```

**Output:**
```text
Clause count: 1
```

**Next:** See the [C Tutorial](./c-tutorial.md) for complete examples with iteration and error handling.

---

## Quick Start: C++

### 1. Create Your Program

Modern C++17 with RAII wrappers:

```cpp
#include <iostream>
#include <cstdint>

// Forward declarations from libroup C API
struct OmpDirective;
extern "C" {
    OmpDirective* roup_parse(const char* input);
    int32_t roup_directive_clause_count(const OmpDirective* dir);
    void roup_directive_free(OmpDirective* dir);
}

// Simple RAII wrapper
class Directive {
    OmpDirective* ptr_;
public:
    explicit Directive(const char* input) 
        : ptr_(roup_parse(input)) {}
    
    ~Directive() {
        if (ptr_) roup_directive_free(ptr_);
    }
    
    // Delete copy, allow move
    Directive(const Directive&) = delete;
    Directive& operator=(const Directive&) = delete;
    Directive(Directive&& other) noexcept 
        : ptr_(other.ptr_) {
        other.ptr_ = nullptr;
    }
    
    bool valid() const { 
        return ptr_ != nullptr; 
    }
    
    int clause_count() const {
        return ptr_ ? roup_directive_clause_count(ptr_) : 0;
    }
};

int main() {
    Directive dir("#pragma omp parallel for num_threads(4)");
    
    if (!dir.valid()) {
        std::cerr << "Parse failed\n";
        return 1;
    }
    
    std::cout << "Clause count: " << dir.clause_count() << "\n";
    
    return 0;
}  // Automatic cleanup via RAII
```

### 2. Compile

```bash
# Build ROUP library first
cargo build --release

# Compile your C++ program
clang++ -std=c++17 example.cpp \
  -L./target/release \
  -lroup \
  -lpthread -ldl -lm \
  -Wl,-rpath,./target/release \
  -o example
```

### 3. Run

```bash
./example
```

**Output:**
```text
Clause count: 1
```

**Next:** See the [C++ Tutorial](./cpp-tutorial.md) for a complete real-world example.

---

## What's Supported?

### Directive keywords (127 total)
✅ Parallelism: `parallel`, `for`, `sections`, `single`, `master`
✅ Tasking: `task`, `taskwait`, `taskgroup`, `taskloop`
✅ Device offloading: `target`, `teams`, `distribute`
✅ Synchronization: `barrier`, `critical`, `atomic`
✅ Advanced: `metadirective`, `declare variant`, `loop`, and every combined form (e.g. `target teams distribute parallel for simd`)

### Clause keywords (132 total)
✅ Scheduling: `schedule`, `collapse`, `ordered`
✅ Data-sharing: `private`, `shared`, `firstprivate`, `lastprivate`
✅ Reductions: `reduction(+|-|*|&|&&|min|max:vars)`
✅ Device clauses: `device`, `map`, `is_device_ptr`
✅ Control: `if`, `num_threads`, `default`, and new OpenMP 6.0 additions such as `device_safesync`

**For the complete support matrix, see [OpenMP Support](./openmp-support.md).**

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
clang example.c -L./target/release -lroup -lpthread -ldl -lm -Wl,-rpath,$PWD/target/release
```

**macOS:**
```bash
export DYLD_LIBRARY_PATH=$PWD/target/release:$DYLD_LIBRARY_PATH
./example
```

### Parse Returns NULL

Check your OpenMP syntax:

```c
// ✅ Valid
OmpDirective* dir = roup_parse("#pragma omp parallel");

// ✅ Valid - with clauses
dir = roup_parse("#pragma omp parallel num_threads(4)");

// ❌ Invalid - missing 'omp'
dir = roup_parse("#pragma parallel");  // Returns NULL
```

Always check for NULL:
```c
OmpDirective* dir = roup_parse(input);
if (!dir) {
    fprintf(stderr, "Parse error\n");
    return 1;
}
// ... use dir ...
roup_directive_free(dir);
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
clang example.c -L./target/release -lroup -lpthread -ldl -lm -Wl,-rpath,./target/release
./example
```

**C++:**
```bash
cargo build --release
clang++ -std=c++17 example.cpp -L./target/release -lroup -lpthread -ldl -lm -Wl,-rpath,./target/release
./example
```

**You're ready to experiment with OpenMP parsing!** 🎉
