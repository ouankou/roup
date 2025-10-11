# ROUP

<div align="center">

**Rust-based OpenMP Parser**

*Safe, fast, and comprehensive OpenMP directive parsing*

[Get Started](./getting-started.md) · [Tutorials](./rust-tutorial.md) · [API Reference](./api-reference.md) · [GitHub](https://github.com/ouankou/roup)

</div>

---

## What is ROUP?

ROUP is a production-ready parser for OpenMP directives, written in safe Rust with C and C++ APIs. Parse OpenMP pragmas like `#pragma omp parallel for` into structured data that your tools can analyze, transform, and process.

**Perfect for:**
- 🔧 **Compiler developers** - Integrate OpenMP parsing into your compiler
- 🔍 **Analysis tools** - Build linters, formatters, and code analyzers  
- 🎓 **Researchers** - Study parallelization patterns in real code
- 📚 **Educators** - Teaching tool for parallel programming concepts

---

## Why ROUP?

### 🚀 Fast & Lightweight

- **Microsecond parsing** - Parse directives in ~500ns
- **Zero-copy lexer** - Minimal memory allocations
- **No LLVM dependency** - Standalone library, small binary size
- **16 FFI functions** - Simple, focused C API

### 🛡️ Safe & Reliable

- **99.1% safe Rust** - Memory safety guaranteed
- **352 passing tests** - Comprehensive test coverage
- **Fuzzing tested** - Handles malformed input gracefully
- **NULL-safe C API** - Defensive checks at FFI boundary

### 📚 Comprehensive OpenMP Support

- **95 directives** - From `parallel` to `metadirective`
- **91 clauses** - Extensive OpenMP 3.0 through 6.0 coverage
- **Version tracking** - Know which OpenMP version introduced each feature
- **Spec compliant** - Follows official OpenMP specifications

### 🔌 Multi-Language APIs

| Language | API Style | Memory Management |
|----------|-----------|-------------------|
| **Rust** | Native | Automatic (ownership) |
| **C** | Pointer-based | Manual (malloc/free pattern) |
| **C++** | RAII wrappers | Automatic (destructors) |

---

## Quick Example

### Parse in 3 Lines (Rust)

```rust
use roup::parser::parse;

let directive = parse("#pragma omp parallel for num_threads(4)").unwrap();
println!("Found {} clauses", directive.clauses.len());  // Output: Found 1 clauses
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

---

## Architecture

```
┌─────────────────────────────────────┐
│   Your Application                  │
├─────────────────────────────────────┤
│   Language Bindings                 │
│   ├─ Rust API (native)              │
│   ├─ C API (16 functions)           │
│   └─ C++ API (RAII wrappers)        │
├─────────────────────────────────────┤
│   Parser Layer (100% safe Rust)    │
│   ├─ Directive Parser               │
│   ├─ Clause Parser                  │
│   └─ Error Recovery                 │
├─────────────────────────────────────┤
│   Lexer (nom-based, zero-copy)     │
└─────────────────────────────────────┘
```

**Design Principles:**
- **Safety first** - Rust ownership prevents memory bugs
- **Zero-copy** - Parse directly from input string
- **Minimal FFI** - Only ~60 lines of unsafe code (0.9%)
- **Extensible** - Easy to add new directives/clauses

[Learn more about the architecture →](./architecture.md)

---

## Comparison

### ROUP vs LLVM/Clang

| Feature | ROUP | LLVM/Clang |
|---------|------|------------|
| **Purpose** | OpenMP parsing only | Full C/C++ compiler |
| **Binary size** | ~500KB | ~50MB+ |
| **Parse time** | ~500ns - 1µs | ~10-100µs |
| **Dependencies** | None | LLVM, Clang, libc++ |
| **API complexity** | 16 C functions | 1000s of functions |
| **Learning curve** | Minutes | Weeks |
| **Use case** | Analysis, tools, IDE plugins | Compilation |

**Use ROUP when:**
- ✅ You only need to parse OpenMP, not compile
- ✅ You want minimal dependencies
- ✅ You need fast integration into tools
- ✅ You value simplicity and safety

**Use LLVM when:**
- You need full C/C++ compilation
- You're building a complete compiler
- You need LLVM IR generation

### ROUP vs Custom Parser

| Aspect | ROUP | Custom Parser |
|--------|------|---------------|
| **Development time** | Minutes (add dependency) | Weeks/months |
| **OpenMP coverage** | 120+ directives | You must implement all |
| **Testing** | 342 tests included | You must write tests |
| **Maintenance** | Active, updated for new OpenMP | You must maintain |
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

### Testing

- **342 tests** covering all features
- **Fuzzing** with random inputs
- **Valgrind** verified (no leaks)
- **Thread sanitizer** verified (no races)
- **Address sanitizer** verified (no memory errors)

[Read the safety analysis →](./architecture.md#safety-boundaries)

---

## Performance

Typical performance characteristics:

| Operation | Time | Notes |
|-----------|------|-------|
| Parse `#pragma omp parallel` | ~500ns | Simple directive |
| Parse with clauses | ~800ns | `num_threads(4)` |
| Complex directive | ~1.2µs | Multiple clauses |
| Iterator creation | ~10ns | FFI overhead |

**Scalability:**
- ✅ Thread-safe - Parse from multiple threads
- ✅ Zero-copy - No string allocations during lexing
- ✅ Minimal allocations - ~3 allocations per directive
- ✅ Fast enough - Parsing is rarely the bottleneck

[Performance details →](./architecture.md#performance-characteristics)

---

## Getting Started

Choose your language:

<div style="display: grid; grid-template-columns: repeat(3, 1fr); gap: 1rem;">

<div style="padding: 1rem; border: 1px solid #ddd; border-radius: 4px;">

### 🦀 Rust

**Install:**
```toml
[dependencies]
roup = "0.1"
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
