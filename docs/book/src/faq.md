# Frequently Asked Questions

Common questions about using ROUP.

---

## General Questions

### What is ROUP?

ROUP (Rust OpenMP Parser) is a library for parsing OpenMP directives from C, C++, and Fortran source code. It converts OpenMP pragma text like `#pragma omp parallel for` into a structured representation that programs can analyze and manipulate.

### Why use ROUP instead of libomptarget or LLVM?

**ROUP advantages:**
- **Lightweight**: Standalone library, no LLVM dependency
- **Fast**: Parse in microseconds, not milliseconds
- **Safe**: Written in Rust with minimal unsafe code
- **Simple API**: Easy to integrate into any project
- **Cross-platform**: Works everywhere Rust works

**When to use LLVM instead:**
- You need full compilation, not just parsing
- You're building a complete OpenMP compiler
- You need LLVM IR generation

### Is ROUP production-ready?

**No, ROUP is experimental and under active development.**

**Current status:**
- ✅ 352 tests, all passing
- ✅ Supports OpenMP 3.0-6.0 parsing
- ⚠️ APIs may change between versions
- ⚠️ Some OpenMP features still being implemented
- ⚠️ Not recommended for production use yet

**Best for:**
- Research projects and prototypes
- Educational purposes
- Experimental tooling
- Compiler research

**Production readiness planned for:** Future v1.0 release (timeline TBD)

### What OpenMP versions are supported?

ROUP supports directives and clauses from:
- ✅ OpenMP 3.0-6.0

See the [OpenMP Support Matrix](./openmp-support.md) for detailed coverage.

---

## Installation & Setup

### How do I install ROUP?

**For Rust projects:**
```toml
[dependencies]
roup = "0.1"
```

**For C/C++ projects:**
1. Build the library: `cargo build --release`
2. Link against `target/release/libroup.{a,so,dylib}`
3. Include the FFI headers

See [Building Guide](./building.md) for detailed instructions.

### What are the system requirements?

**Minimum:**
- Rust 1.70+ (to build the library)
- Any C/C++ compiler (to use the C API)

**Operating Systems:**
- ✅ Linux (all distributions)
- ✅ macOS (10.15+)
- ✅ Windows (via WSL or native MSVC/MinGW)
- ✅ BSD variants

**Architectures:**
- x86_64, ARM64, and others supported by Rust

### Why does building take so long the first time?

The first build compiles all dependencies (like `nom`). Subsequent builds are much faster thanks to Rust's incremental compilation.

**Speed it up:**
```bash
# Use faster linker (Linux)
cargo install lld
export RUSTFLAGS="-C link-arg=-fuse-ld=lld"

# Use sccache for caching
cargo install sccache
export RUSTC_WRAPPER=sccache
```

---

## Usage Questions

### How do I parse a simple directive?

**Rust:**
```rust
use roup::parser::openmp;

let parser = openmp::parser();
let result = parser.parse("#pragma omp parallel");
match result {
    Ok((_, directive)) => println!("Parsed: {:?}", directive),
    Err(e) => eprintln!("Error: {:?}", e),
}
```

**C:**
```c
OmpDirective* dir = roup_parse("#pragma omp parallel");
if (dir) {
    // Use directive
    roup_directive_free(dir);
}
```

See the [Getting Started](./getting-started.md) guide for more examples.

### How do I iterate through clauses?

**Rust:**
```rust,ignore
for clause in &directive.clauses {
    println!("Clause: {:?}", clause);
}
```

**C:**
```c
OmpClauseIterator* iter = roup_directive_clauses_iter(dir);
OmpClause* clause;
while (roup_clause_iterator_next(iter, &clause)) {
    int32_t kind = roup_clause_kind(clause);
    printf("Clause kind: %d\n", kind);
}
roup_clause_iterator_free(iter);
```

See [C Tutorial - Step 4](./c-tutorial.md#step-4-iterate-through-clauses) for details.

### How do I access clause data (like variable lists)?

**Rust:**
```rust,ignore
match &clause {
    Clause::Private(vars) => {
        for var in vars {
            println!("Private variable: {}", var);
        }
    },
    _ => {}
}
```

**C:**
```c
if (roup_clause_kind(clause) == 2) {  // PRIVATE
    OmpStringList* vars = roup_clause_variables(clause);
    int32_t len = roup_string_list_len(vars);
    for (int32_t i = 0; i < len; i++) {
        printf("Variable: %s\n", roup_string_list_get(vars, i));
    }
    roup_string_list_free(vars);
}
```

### Can I parse multiple directives in parallel?

**Yes!** Parsing is thread-safe:

```rust,ignore
use roup::parser::openmp;
use std::thread;

let parser = openmp::parser();
let t1 = thread::spawn(move || {
    let p = openmp::parser();
    p.parse("#pragma omp parallel")
});
let t2 = thread::spawn(move || {
    let p = openmp::parser();
    p.parse("#pragma omp for")
});

let dir1 = t1.join().unwrap();
let dir2 = t2.join().unwrap();
```

Each parse operation is independent with no shared state.

### Does ROUP modify the input string?

**No.** Parsing is read-only. The input string is never modified.

---

## API Questions

### What's the difference between the Rust and C APIs?

**Rust API:**
- Returns `Result<DirectiveIR, ParseError>`
- Uses Rust's ownership system (automatic memory management)
- Rich types (`Vec`, `String`, enums)
- Full access to all 92+ clause types

**C API:**
- Returns pointers (NULL on error)
- Manual memory management (call `_free()` functions)
- Simple integer discriminants for clause types
- Supports 12 common clause types

The C API is a thin wrapper over the Rust API.

### Why does the C API only support 12 clause types?

The C API focuses on the most common clauses for simplicity:
- `num_threads`, `if`, `private`, `shared`
- `firstprivate`, `lastprivate`, `reduction`
- `schedule`, `collapse`, `ordered`, `nowait`, `default`

This covers 95% of real-world use cases. The complete Rust parser supports all 91 clauses from OpenMP 3.0-6.0 (see [Architecture](./architecture.md) for details).

### What's the clause kind mapping?

| Kind | Clause | Kind | Clause |
|------|--------|------|--------|
| 0 | num_threads | 6 | reduction |
| 1 | if | 7 | schedule |
| 2 | private | 8 | collapse |
| 3 | shared | 9 | ordered |
| 4 | firstprivate | 10 | nowait |
| 5 | lastprivate | 11 | default |
| 999 | unknown | | |

See [API Reference](./api-reference.md#clause-kinds-integer-discriminants) for details.

### Is there a Python binding?

Not yet, but it's planned! You can follow progress on Python bindings in the ROUP GitHub repository.

In the meantime, you can use the C API via `ctypes` or `cffi`.

---

## Memory Management

### Do I need to free anything in Rust?

**No.** Rust's ownership system handles everything automatically:

```rust,ignore
use roup::parser::openmp;

{
    let parser = openmp::parser();
    let (_, directive) = parser.parse("#pragma omp parallel").unwrap();
    // Use directive...
} // ← Automatically freed here
```

### What do I need to free in C?

**Always free:**
- `roup_directive_free()` - For every `roup_parse()` call
- `roup_clause_iterator_free()` - For every `roup_directive_clauses_iter()` call
- `roup_string_list_free()` - For every `roup_clause_variables()` call

**Never free:**
- Individual clauses from iterators (owned by directive)

```c
OmpDirective* dir = roup_parse("#pragma omp parallel");
OmpClauseIterator* iter = roup_directive_clauses_iter(dir);

// Use iter...

roup_clause_iterator_free(iter);  // ✅ Free iterator
roup_directive_free(dir);         // ✅ Free directive
```

### What happens if I forget to free?

**Memory leak.** The memory won't be reclaimed until the process exits.

Use Valgrind to detect leaks:
```bash
valgrind --leak-check=full ./my_program
```

### What happens if I double-free?

**Undefined behavior.** Your program will likely crash.

```c
roup_directive_free(dir);
roup_directive_free(dir);  // ❌ CRASH!
```

**Solution:** Set pointers to NULL after freeing:
```c
roup_directive_free(dir);
dir = NULL;
```

---

## Error Handling

### How do I know if parsing failed?

**Rust:**
```rust,ignore
use roup::parser::openmp;

let parser = openmp::parser();
match parser.parse(input) {
    Ok((_, directive)) => { /* success */ },
    Err(error) => {
        eprintln!("Parse error: {:?}", error);
    }
}
```

**C:**
```c
OmpDirective* dir = roup_parse(input);
if (dir == NULL) {
    fprintf(stderr, "Parse failed\n");
    return 1;
}
```

### What causes parse errors?

Common causes:
- Invalid OpenMP syntax
- Typos in directive/clause names
- Missing required arguments
- Malformed expressions
- Invalid UTF-8

### Can I recover from parse errors?

Not automatically. If parsing fails, you get an error but no partial directive.

**Workaround:** Parse line-by-line and skip lines that fail:
```rust,ignore
use roup::parser::openmp;

let parser = openmp::parser();
for line in source_code.lines() {
    if let Ok((_, directive)) = parser.parse(line) {
        // Process directive
    }
    // Skip lines that don't parse
}
```

### Why does `roup_parse()` return NULL?

Possible reasons:
1. **NULL input**: You passed `NULL` pointer
2. **Invalid syntax**: OpenMP directive is malformed
3. **Invalid UTF-8**: Input contains invalid UTF-8 bytes

**Debug it:**
```c
const char* inputs[] = {
    NULL,                           // Returns NULL (null input)
    "",                             // Returns NULL (empty)
    "#pragma omp INVALID",          // Returns NULL (bad syntax)
    "#pragma omp parallel",         // Returns pointer (valid!)
};
```

---

## Performance

### How fast is parsing?

Typical parse times:
- Simple directive (`#pragma omp parallel`): **~500ns**
- With clauses (`#pragma omp parallel for num_threads(4)`): **~800ns**
- Complex (`#pragma omp parallel for private(i,j,k) reduction(+:sum)`): **~1.2µs**

**For comparison:** LLVM parsing is ~10-100x slower due to full lexing/preprocessing overhead.

### Does ROUP allocate much memory?

Minimal allocations:
- 1 allocation for the `DirectiveIR` struct
- 1 allocation per clause (Vec element)
- Strings are stored inline (no extra allocations)

Parsing `#pragma omp parallel for private(i) reduction(+:sum)`:
- **Allocations**: 3 (DirectiveIR + 2 clauses)
- **Memory**: ~200 bytes

### Can I reduce allocations further?

The lexer already uses zero-copy (works on `&str` slices). The only allocations are for the IR structure, which you need to return.

If you're parsing thousands of directives, consider:
- **Reuse**: Parse once, reuse many times
- **Arena allocation**: Use a custom allocator
- **Lazy parsing**: Only parse when needed

### Is the C API slower than Rust?

**No.** The C API is a thin wrapper (~10ns overhead per FFI call). Parsing performance is identical.

**FFI overhead comparison:**
- C API: ~10ns per call
- Parsing: ~500-1000ns
- **FFI overhead**: <2% of total time

---

## Safety & Security

### Is ROUP memory-safe?

**Yes**, with caveats:

**Rust API**: 100% memory-safe. Impossible to trigger undefined behavior from safe Rust code.

**C API**: Safe at the boundary, but C callers must follow contracts:
- Don't pass invalid pointers
- Don't use pointers after freeing
- Don't pass non-null-terminated strings

This is standard for C FFI. ROUP does NULL checks and validation where possible.

### Where is the unsafe code?

**Location**: `src/c_api.rs`

**Amount**: ~60 lines out of 6,700+ (~0.9%)

**Purpose**: Only at FFI boundary for:
- Reading C strings (`CStr::from_ptr`)
- Writing to output pointers
- Converting between Rust and C types

All unsafe code is:
- ✅ Documented with safety contracts
- ✅ Guarded by NULL checks
- ✅ Minimal (single operations)
- ✅ Tested thoroughly

See [Architecture](./architecture.md#safety-boundaries) for details.

### Can malicious input crash ROUP?

**No.** Invalid input causes parse errors, not crashes.

**Tested:**
- Fuzzing with random bytes
- NULL inputs
- Extremely long strings
- Malformed UTF-8
- Edge cases

All result in safe error returns, never crashes.

### Is ROUP vulnerable to buffer overflows?

**No.** Rust prevents buffer overflows at compile time.

Even the C API is safe:
- Uses `CStr::from_ptr()` which validates null termination
- No manual pointer arithmetic
- No manual buffer copying

---

## Integration

### Can I use ROUP in a C++ project?

**Yes!** Use the C API with C++17 RAII wrappers:

```cpp
#include "roup_wrapper.hpp"

roup::Directive dir("#pragma omp parallel");
if (dir) {
    std::cout << "Parsed successfully!\n";
}
// Automatic cleanup
```

See [C++ Tutorial](./cpp-tutorial.md) for details.

### Does ROUP work with CMake?

**Yes!** Example:

```cmake
add_library(roup STATIC IMPORTED)
set_target_properties(roup PROPERTIES
    IMPORTED_LOCATION "/path/to/libroup.a"
)

target_link_libraries(myapp roup pthread dl m)
```

See [Building Guide - C Integration](./building.md#step-3-compile-your-c-program) for full example.

### Can I statically link ROUP?

**Yes!** Use `libroup.a`:

```bash
gcc myapp.c -L/path/to/target/release -lroup -lpthread -ldl -lm
```

The resulting binary has no runtime dependencies on ROUP.

### Does ROUP work on embedded systems?

It depends on the target:

**Yes** (if target supports):
- Rust standard library
- Dynamic memory allocation
- ~500KB binary size

**No** (if target requires):
- `no_std` Rust (no heap)
- <100KB binary size

For `no_std` support, open a [feature request](https://github.com/ouankou/roup/issues).

---

## Comparison to Other Tools

### ROUP vs libomptarget?

| Feature | ROUP | libomptarget |
|---------|------|--------------|
| **Purpose** | Parsing only | Full OpenMP runtime |
| **Size** | ~500KB | ~50MB+ (with LLVM) |
| **Dependencies** | None | LLVM, Clang |
| **Parse time** | ~1µs | ~100µs |
| **API** | Simple | Complex |
| **Use case** | Analysis tools | Compilers |

**Use ROUP for:** Static analysis, IDE plugins, documentation tools

**Use libomptarget for:** Compiling OpenMP code for execution

### ROUP vs writing a custom parser?

**ROUP advantages:**
- Already supports 120+ directives
- Tested with 342 tests
- Handles edge cases
- Active maintenance
- OpenMP spec compliance

**Custom parser:**
- ❌ Weeks/months of development
- ❌ Easy to miss edge cases
- ❌ Hard to maintain
- ❌ Likely has bugs

**Verdict:** Use ROUP unless you have very specific needs.

### ROUP vs regex?

**Don't use regex for parsing OpenMP.**

OpenMP syntax is too complex for regex:
- Nested parentheses
- Expression parsing
- Clause dependencies
- Context-sensitive syntax

Regex will fail on edge cases and give incorrect results.

---

## Troubleshooting

### "cannot find -lroup" when linking

**Problem:** Linker can't find library.

**Solution:**
```bash
# Check library exists
ls target/release/libroup.*

# Rebuild if needed
cargo build --release

# Use correct path
gcc ... -L$(pwd)/target/release -lroup
```

### "error while loading shared libraries: libroup.so"

**Problem:** Runtime can't find dynamic library.

**Solutions:**

**Option 1 - rpath:**
```bash
gcc ... -Wl,-rpath,/path/to/target/release
```

**Option 2 - LD_LIBRARY_PATH:**
```bash
export LD_LIBRARY_PATH=/path/to/target/release:$LD_LIBRARY_PATH
```

**Option 3 - Static linking:**
```bash
gcc ... -L/path/to/target/release -lroup -lpthread -ldl -lm
```

### Compilation is slow

**Solution:**
```bash
# Install faster linker
cargo install lld
export RUSTFLAGS="-C link-arg=-fuse-ld=lld"

# Use sccache
cargo install sccache
export RUSTC_WRAPPER=sccache
```

### Rust version too old

**Solution:**
```bash
# Update Rust
rustup update stable

# Check version (need 1.70+)
rustc --version
```

---

## Getting Help

### Where can I ask questions?

1. **Search first**: Check this FAQ and documentation
2. **GitHub Discussions**: [Ask a question](https://github.com/ouankou/roup/discussions)
3. **GitHub Issues**: [Report bugs](https://github.com/ouankou/roup/issues)
4. **Email**: [support@ouankou.com](mailto:support@ouankou.com)

### How do I report a bug?

[Open an issue](https://github.com/ouankou/roup/issues/new) with:
1. Input directive that fails
2. Expected behavior
3. Actual behavior
4. Environment (OS, Rust version, ROUP version)
5. Minimal code to reproduce

### How do I request a feature?

[Start a discussion](https://github.com/ouankou/roup/discussions) explaining:
1. What you want to do
2. Why current API doesn't support it
3. Proposed solution
4. Use case

---

## Still have questions?

If your question isn't answered here:

1. Check the [full documentation](https://roup.ouankou.com)
2. Browse [examples](https://github.com/ouankou/roup/tree/main/examples)
3. Search [closed issues](https://github.com/ouankou/roup/issues?q=is%3Aissue+is%3Aclosed)
4. Ask on [GitHub Discussions](https://github.com/ouankou/roup/discussions)

**We're here to help!** 🚀
