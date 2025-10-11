# ROUP: Rust-based OpenMP/OpenACC Unified Parser

**Safe, fast, and extensible directive parser with multi-language support**

---

## What is ROUP?

ROUP is a standalone parser for OpenMP and OpenACC directives, written in Rust. It's designed as an extensible framework that can be integrated into compilers, analysis tools, and development environments.

### Key Features

- ✅ **99.1% Safe Rust** - Only 18 unsafe blocks (~60 lines) at FFI boundary
- ✅ **Multi-Language APIs** - Native Rust, C, and C++ interfaces
- ✅ **OpenMP 5.0+ Support** - 15+ directives, 50+ clauses
- ✅ **Production Ready** - 355 tests, zero warnings, cross-platform
- ✅ **Modern C++** - C++17 RAII wrappers for automatic memory management
- ✅ **Well Documented** - Comprehensive tutorials and examples

---

## Why ROUP?

### For Compiler Developers
- Drop-in OpenMP/OpenACC parser component
- Well-tested, battle-hardened parsing logic
- Easy FFI integration from any language

### For Tool Builders
- Analyze OpenMP code without a full compiler
- Build linters, formatters, and code analyzers
- Extract parallelization patterns from codebases

### For Researchers
- Study directive usage patterns
- Prototype new directive extensions
- Educational tool for learning parallel programming

---

## Quick Example

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

// Forward declarations
typedef struct OmpDirective OmpDirective;
extern OmpDirective* roup_parse(const char* input);
extern int32_t roup_directive_clause_count(const OmpDirective* dir);
extern void roup_directive_free(OmpDirective* dir);

int main() {
    OmpDirective* dir = roup_parse("#pragma omp parallel for num_threads(4)");
    if (dir) {
        printf("Clauses: %d\n", roup_directive_clause_count(dir));
        roup_directive_free(dir);
    }
    return 0;
}
```

### C++
```cpp
#include <iostream>
#include <memory>

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
    int clause_count() const { 
        return ptr_ ? roup_directive_clause_count(ptr_) : 0; 
    }
};

int main() {
    Directive dir("#pragma omp parallel for num_threads(4)");
    if (dir.valid()) {
        std::cout << "Clauses: " << dir.clause_count() << "\n";
    }
    return 0;
}
```

---

## Architecture

ROUP uses a clean, modular architecture:

```
┌─────────────────────────────────────────┐
│         Application Layer               │
│  (Your compiler/tool/analyzer)          │
└─────────────────┬───────────────────────┘
                  │
      ┌───────────┼───────────┐
      │           │           │
      ▼           ▼           ▼
┌─────────┐ ┌─────────┐ ┌─────────┐
│ Rust API│ │  C API  │ │ C++ API │
│         │ │         │ │ (RAII)  │
└─────────┘ └─────────┘ └─────────┘
      │           │           │
      └───────────┼───────────┘
                  │
                  ▼
         ┌────────────────┐
         │  Core Parser   │
         │  (nom-based)   │
         └────────────────┘
                  │
      ┌───────────┼───────────┐
      ▼           ▼           ▼
┌─────────┐ ┌─────────┐ ┌─────────┐
│  Lexer  │ │Directive│ │ Clause  │
│         │ │ Parser  │ │ Parser  │
└─────────┘ └─────────┘ └─────────┘
```

**Key Design Principles:**
- **Safe by default** - Rust's ownership system prevents memory errors
- **Zero-copy parsing** - Uses string slices, not allocations
- **Minimal unsafe** - FFI boundary only, well-documented
- **Extensible** - Easy to add new directives and clauses

---

## OpenMP Support

ROUP currently supports **OpenMP 5.0+** with comprehensive coverage:

### Supported Directives (15+)
- `parallel` - Parallel regions
- `for` - Worksharing loops
- `sections`, `single` - Worksharing constructs
- `task`, `taskwait`, `taskgroup` - Tasking
- `target`, `teams`, `distribute` - Device offloading
- `barrier`, `critical`, `atomic` - Synchronization
- `metadirective` - Dynamic selection
- And more...

### Supported Clauses (50+)
- **Data sharing:** `private`, `shared`, `firstprivate`, `lastprivate`
- **Parallelism control:** `num_threads`, `if`, `proc_bind`
- **Worksharing:** `schedule`, `collapse`, `nowait`
- **Reductions:** `reduction` with 10+ operators (+, *, min, max, etc.)
- **Device:** `map`, `device`, `defaultmap`
- **Dependencies:** `depend`, `in`, `out`, `inout`
- And more...

See the [OpenMP Support Matrix](https://github.com/ouankou/roup/blob/main/docs/OPENMP_SUPPORT.md) for the complete list.

---

## Safety Guarantees

ROUP is built with safety as a core principle:

| Metric | Value | Notes |
|--------|-------|-------|
| **Safe Rust** | 99.1% | All core logic is safe |
| **Unsafe blocks** | 18 | Only at FFI boundary |
| **Unsafe lines** | ~60 | Well-documented, NULL-checked |
| **Memory leaks** | 0 | Rust's RAII prevents leaks |
| **Segfaults** | 0 | Ownership system prevents use-after-free |

All `unsafe` code is:
- **Documented** with safety requirements
- **NULL-checked** before dereferencing
- **Isolated** to `src/c_api.rs`
- **Tested** with 355+ tests

---

## Testing

ROUP has comprehensive test coverage:

- **239 doc tests** - Examples in documentation are auto-tested
- **116 integration tests** - Real-world usage scenarios
- **355 total tests** - All passing, zero warnings
- **Cross-platform** - Tested on Linux, macOS, Windows

Test categories:
- ✅ Basic directives (parallel, for, task, teams, target)
- ✅ Complex features (reductions, metadirectives, nesting)
- ✅ Edge cases (comments, whitespace, error handling)
- ✅ Roundtrip parsing (parse → format → parse)
- ✅ FFI safety (C and C++ examples)

---

## Getting Started

Ready to use ROUP in your project? Check out our tutorials:

- **[C++ Tutorial](./cpp-tutorial.md)** - Build a real application with C++17
- **[Rust API Docs](./api-reference.md)** - Complete API reference

Or jump straight to the code:
- [GitHub Repository](https://github.com/ouankou/roup)
- [Quick Start Guide](https://github.com/ouankou/roup/blob/main/docs/QUICK_START.md)

---

## License

ROUP is open source under the **MIT License**.

**Copyright © 2024-2025 Anjia Wang**
