# ROUP

<div align="center">

**Rust-based OpenMP & OpenACC Parser**

*Safe, fast, and comprehensive directive parsing*

[Get Started](./getting-started.md) · [Tutorials](./rust-tutorial.md) · [API Reference](./api-reference.md) · [GitHub](https://github.com/ouankou/roup)

</div>

---

## What is ROUP?

ROUP is an **experimental** parser for OpenMP **and** OpenACC directives, written in safe Rust with C, C++, and Fortran bindings. Parse pragmas like `#pragma omp parallel for` or `!$acc parallel` into structured data that your tools can analyze, transform, and process.

> **⚠️ Experimental Status**: ROUP is under active development and not yet production-ready. APIs may change, and some OpenMP features are still being implemented. Use in research and experimental projects.

**Perfect for:**
- 🔧 **Compiler research** - Experiment with OpenMP parsing in compilers
- 🔍 **Analysis prototypes** - Build experimental linters and analyzers  
- 🎓 **Researchers** - Study parallelization patterns and test new ideas
- 📚 **Educators** - Teaching tool for parallel programming concepts

---

## Why ROUP?

### 🚀 Fast & Lightweight

- Zero-copy lexer and hand-written parsers
- Standalone library (no LLVM/ANTLR dependencies)
- Compatibility shims for ompparser and accparser

### 🛡️ Safe & Reliable

- Memory safety guaranteed except for the narrow FFI boundary
- Extensive automated tests plus OpenMP_VV/OpenACCV-V validation and compat ctests
- NULL-safe C API with defensive checks

### 📚 Comprehensive Directive Support

- OpenMP 3.0–6.0 directives, clauses, combined forms, and metadirectives
- OpenACC 3.4 directives, clauses, aliases, and end-paired constructs
- Canonical keyword handling and clause alias preservation
- Interactive parser debugger (`roup_debug`) for OpenMP/OpenACC (C and Fortran sentinels)

### 🔌 Multi-Language APIs

| Language | API Style | Memory Management | Status |
|----------|-----------|-------------------|--------|
| **Rust** | Native | Automatic (ownership) | ✅ |
| **C** | Pointer-based | Manual (malloc/free pattern) | ✅ |
| **C++** | RAII wrappers | Automatic (destructors) | ✅ |
| **Fortran** | C interop | Manual (via iso_c_binding) | ✅ (via C API) |

---

## Quick Example

### Parse in 3 Lines (Rust)

```rust,ignore
use roup::parser::openmp;

let parser = openmp::parser();
let (_, directive) = parser.parse("#pragma omp parallel for num_threads(4)").unwrap();
// Access directive information
println!("Directive: {}", directive.name);  // Output: Directive: parallel for
println!("Found {} clauses", directive.clauses.len());  // Output: Found 1 clauses
// Iterate through clauses
for clause in &directive.clauses {
    println!("  Clause: {}", clause.name);  // Output:   Clause: num_threads
}
```

### Parse in C

```c
OmpDirective* dir = roup_parse("#pragma omp parallel for num_threads(4)");
printf("Clauses: %d\n", roup_directive_clause_count(dir));
roup_directive_free(dir);
```

### Parse in C++ (with RAII)

```cpp
roup::Directive dir("#pragma omp parallel for num_threads(4)");
std::cout << "Clauses: " << dir.clause_count() << "\n";
// Automatic cleanup!
```

### Parse Fortran

```fortran
! Free-form Fortran
directive_ptr = roup_parse_with_language("!$OMP PARALLEL PRIVATE(A)", &
                                          ROUP_LANG_FORTRAN_FREE)
```

[See full examples →](./getting-started.md)

---

## Feature Highlights

### 🎯 Comprehensive Coverage

<details>
<summary><b>Parallel Constructs</b> - 20+ directives</summary>

- `parallel` - Basic parallel regions
- `parallel for` - Combined parallel + worksharing
- `parallel sections` - Parallel sections
- `parallel master` - Parallel master thread
- `parallel loop` - OpenMP 5.0+ parallel loop
- And more...

</details>

<details>
<summary><b>Work-Sharing</b> - 10+ directives</summary>

- `for` / `do` - Loop worksharing
- `sections` / `section` - Code sections
- `single` - Execute once
- `workshare` - Fortran worksharing
- `loop` - Generic loop construct

</details>

<details>
<summary><b>Tasking</b> - 15+ directives</summary>

- `task` - Explicit tasks
- `taskloop` - Loop-based tasks
- `taskgroup` - Task synchronization
- `taskwait` - Wait for tasks
- `taskyield` - Yield to other tasks
- Dependency clauses: `depend`, `priority`, `detach`

</details>

<details>
<summary><b>Device Offloading</b> - 25+ directives</summary>

- `target` - Offload to device
- `target data` - Device data management
- `target enter/exit data` - Data transfer
- `target update` - Synchronize data
- `teams` - Multiple thread teams
- `distribute` - Distribute iterations

</details>

<details>
<summary><b>SIMD</b> - 10+ directives</summary>

- `simd` - SIMD loops
- `declare simd` - Vectorizable functions
- `distribute simd` - Combined distribute + SIMD
- Various alignment and vectorization clauses

</details>

<details>
<summary><b>Advanced (OpenMP 5.0+)</b></summary>

- `metadirective` - Context-sensitive directives
- `declare variant` - Function variants
- `loop` - Generic loop construct
- `scan` - Prefix scan operations
- `assume` - Compiler assumptions

</details>

[Full OpenMP Support Matrix →](./openmp-support.md)

### 🔍 Rich Clause Support

**92+ clause types including:**

| Category | Clauses |
|----------|---------|
| **Data Sharing** | `private`, `shared`, `firstprivate`, `lastprivate` |
| **Reductions** | `reduction(+:x)`, `reduction(min:y)`, custom operators |
| **Scheduling** | `schedule(static)`, `schedule(dynamic,100)`, `collapse(3)` |
| **Control** | `if(condition)`, `num_threads(8)`, `proc_bind(close)` |
| **Device** | `map(to:x)`, `device(2)`, `defaultmap(tofrom:scalar)` |
| **Dependencies** | `depend(in:x)`, `depend(out:y)`, `depend(inout:z)` |

[Complete clause reference →](./api-reference.md)
| **Edge cases** | Handled (fuzzing tested) | Likely has bugs |
| **Spec compliance** | Verified | Uncertain |

**Verdict:** Unless you have very specific needs, use ROUP.

---

## Safety Guarantees

ROUP prioritizes safety without compromising usability:

### Memory Safety

- ✅ **No buffer overflows** - Rust prevents at compile time
- ✅ **No use-after-free** - Ownership system enforces
- ✅ **No double-free** - Checked at FFI boundary
- ✅ **No memory leaks** - RAII and destructors
- ✅ **No data races** - Thread-safe parsing

### API Safety

**Rust API:**
- 100% memory-safe by construction
- Impossible to trigger undefined behavior

**C API:**
- NULL checks before all pointer operations
- Returns safe defaults on error (-1, NULL)
- Validates UTF-8 encoding
- Documents all safety contracts

## Getting Started

Choose your language:

<div style="display: grid; grid-template-columns: repeat(3, 1fr); gap: 1rem;">

<div style="padding: 1rem; border: 1px solid #ddd; border-radius: 4px;">

### 🦀 Rust

**Install:**
```toml
[dependencies]
roup = "0.7"
```

**Learn:**
- [Rust Tutorial](./rust-tutorial.md)
- [API Docs](./api-reference.md)

</div>

<div style="padding: 1rem; border: 1px solid #ddd; border-radius: 4px;">

### 🔧 C

**Build:**
```bash
cargo build --release
```

**Learn:**
- [C Tutorial](./c-tutorial.md)
- [Building Guide](./building.md)

</div>

<div style="padding: 1rem; border: 1px solid #ddd; border-radius: 4px;">

### ⚙️ C++

**Build:**
```bash
cargo build --release
```

**Learn:**
- [C++ Tutorial](./cpp-tutorial.md)
- [RAII Wrappers](./cpp-tutorial.md#step-2-create-raii-wrappers-modern-c)

</div>

</div>

[Quick Start Guide →](./getting-started.md)

---

## Community

- **GitHub**: [ouankou/roup](https://github.com/ouankou/roup)
- **Issues**: [Bug reports](https://github.com/ouankou/roup/issues)
- **Discussions**: [Questions & ideas](https://github.com/ouankou/roup/discussions)
- **Contributing**: [How to contribute](./contributing.md)

---

## License

ROUP is open source under the **MIT License**.

Copyright © 2024-2025 Anjia Wang

---

## Next Steps

- 📖 [Read the Getting Started guide](./getting-started.md)
- 🦀 [Try the Rust tutorial](./rust-tutorial.md)
- 🔧 [Try the C tutorial](./c-tutorial.md)
- 📚 [Browse the API reference](./api-reference.md)
- 🏗️ [Learn the architecture](./architecture.md)
- ❓ [Check the FAQ](./faq.md)


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
```rust,ignore
use roup::parser::openmp;

let parser = openmp::parser();
let input = "#pragma omp parallel for num_threads(4) private(i)";
match parser.parse(input) {
    Ok((_, directive)) => {
        println!("Directive: {:?}", directive.kind);
        println!("Clauses: {}", directive.clauses.len());
    }
    Err(e) => eprintln!("Parse error: {:?}", e),
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

```text
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
- **Minimal unsafe** - confined to the ROUP FFI modules, well-documented
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

All `unsafe` code is isolated to the FFI modules (`src/c_api.rs` and `src/c_api/openacc.rs`), documented with safety requirements, and NULL-checked before dereferencing. Parser, AST, IR, lexer, and debugger modules forbid unsafe Rust.

---

## Getting Started

Want to experiment with ROUP? Check out our tutorials:

- **[C++ Tutorial](./cpp-tutorial.md)** - Build an experimental application with C++17
- **[Rust API Docs](./api-reference.md)** - Complete API reference

Or jump straight to the code:
- [GitHub Repository](https://github.com/ouankou/roup)
- [Getting Started Guide](./getting-started.md)

---

## License

ROUP is open source under the **MIT License**.

**Copyright © 2024-2025 Anjia Wang**
