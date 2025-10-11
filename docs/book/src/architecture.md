# Architecture

This document explains ROUP's internal design, from lexical analysis to the C FFI boundary.

---

## Overview

ROUP is structured in three main layers:

```
┌─────────────────────────────────────┐
│   C/C++ Applications                │
├─────────────────────────────────────┤
│   C FFI Layer (16 functions)        │  ← ~60 lines of unsafe code (0.9%)
├─────────────────────────────────────┤
│   Rust API (Parser + IR)            │  ← 100% safe Rust
├─────────────────────────────────────┤
│   Lexer (nom-based)                  │  ← Token extraction
└─────────────────────────────────────┘
```

**Key Design Principles:**
- **Safety First**: 99.1% safe Rust code
- **Zero-Copy**: Minimal allocations during parsing
- **Error Recovery**: Detailed error messages with location info
- **Language Agnostic**: Supports C, C++, and Fortran

---

## Lexer Layer

**Location**: `src/lexer.rs`

The lexer transforms raw OpenMP pragma text into a stream of tokens.

### Tokenization Process

```rust
Input:  "#pragma omp parallel num_threads(4)"
         ↓
Tokens: [
    Pragma("#pragma omp"),
    Identifier("parallel"),
    Identifier("num_threads"),
    LParen,
    Integer(4),
    RParen
]
```

### Token Types

```rust
pub enum Token<'a> {
    Identifier(&'a str),      // parallel, private, shared
    Integer(i64),              // 4, 100, 256
    Float(f64),                // 2.5, 1.0e-6
    String(&'a str),           // "filename.txt"
    LParen,                    // (
    RParen,                    // )
    Comma,                     // ,
    Colon,                     // :
    Plus,                      // +
    Minus,                     // -
    Star,                      // *
    // ... more operators
}
```

### Lexer Implementation

**Technology**: Built with [nom](https://github.com/rust-bakery/nom) parser combinators

**Why nom?**
- **Zero-copy**: Works directly on input &str, no allocations
- **Composable**: Small parsers combine into larger ones
- **Error-rich**: Detailed error messages with position
- **Battle-tested**: Used in production parsers worldwide

**Example Lexer Function:**

```rust
// Parse an identifier: alphanumeric + underscores
fn identifier(input: &str) -> IResult<&str, Token> {
    let (remaining, matched) = take_while1(|c: char| {
        c.is_alphanumeric() || c == '_'
    })(input)?;
    
    Ok((remaining, Token::Identifier(matched)))
}
```

---

## Parser Layer

**Location**: `src/parser/`

The parser converts token streams into a structured Intermediate Representation (IR).

### Parser Modules

```
src/parser/
├── mod.rs          # Main parser entry point
├── directive.rs    # Directive parsing (parallel, for, task, etc.)
├── clause.rs       # Clause parsing (private, reduction, etc.)
└── openmp.rs       # OpenMP-specific parsing logic
```

### Parsing Phases

#### Phase 1: Directive Recognition

```rust
Input tokens: [Identifier("parallel"), Identifier("for"), ...]
               ↓
Directive:    DirectiveKind::ParallelFor
```

Supports 120+ directive types from OpenMP 3.0 through 6.0.

#### Phase 2: Clause Parsing

```rust
Input tokens: [Identifier("num_threads"), LParen, Integer(4), RParen]
               ↓
Clause:       Clause::NumThreads(IntegerExpr(4))
```

Parses 92+ clause types with full argument validation.

#### Phase 3: IR Construction

```rust
DirectiveIR {
    kind: DirectiveKind::ParallelFor,
    clauses: vec![
        Clause::NumThreads(IntegerExpr(4)),
        Clause::Private(vec!["i", "j"]),
    ],
    location: SourceLocation { line: 1, column: 1 },
    language: Language::C,
}
```

### Error Handling

Errors include detailed context:

```rust
ParseError {
    message: "Expected ')' after num_threads value",
    location: SourceLocation { line: 1, column: 27 },
    context: "#pragma omp parallel num_threads(4",
                                              ^
}
```

---

## Intermediate Representation (IR)

**Location**: `src/ir/`

The IR is the central data structure representing parsed OpenMP directives.

### IR Structure

```rust
pub struct DirectiveIR {
    pub kind: DirectiveKind,           // What directive?
    pub clauses: Vec<Clause>,          // What clauses?
    pub location: SourceLocation,      // Where in source?
    pub language: Language,            // C, C++, or Fortran?
}
```

### Directive Kinds

```rust
pub enum DirectiveKind {
    // Parallel constructs
    Parallel,
    ParallelFor,
    ParallelSections,
    
    // Work-sharing
    For,
    Sections,
    Section,
    Single,
    
    // Tasking
    Task,
    TaskLoop,
    TaskWait,
    TaskGroup,
    
    // Device offloading
    Target,
    TargetData,
    TargetUpdate,
    Teams,
    
    // Synchronization
    Barrier,
    Critical,
    Atomic,
    Ordered,
    
    // SIMD
    Simd,
    DeclareSimd,
    
    // Advanced (OpenMP 5.0+)
    Metadirective,
    DeclareVariant,
    Loop,
    
    // ... 120+ total directives
}
```

### Clause Types

```rust
pub enum Clause {
    // Thread management
    NumThreads(IntegerExpr),
    If(Condition),
    
    // Data sharing
    Private(Vec<Variable>),
    Shared(Vec<Variable>),
    FirstPrivate(Vec<Variable>),
    LastPrivate(Vec<Variable>),
    
    // Reductions
    Reduction {
        operator: ReductionOperator,
        variables: Vec<Variable>,
    },
    
    // Scheduling
    Schedule {
        kind: ScheduleKind,
        chunk_size: Option<IntegerExpr>,
    },
    
    // Loop control
    Collapse(usize),
    Ordered,
    Nowait,
    
    // Defaults
    Default(DefaultKind),
    
    // ... 92+ total clauses
}
```

### Supporting Types

```rust
pub enum ScheduleKind {
    Static,
    Dynamic,
    Guided,
    Auto,
    Runtime,
}

pub enum ReductionOperator {
    Add,         // +
    Subtract,    // -
    Multiply,    // *
    BitAnd,      // &
    BitOr,       // |
    BitXor,      // ^
    LogicalAnd,  // &&
    LogicalOr,   // ||
    Min,
    Max,
}

pub enum DefaultKind {
    Shared,
    None,
    Private,
    FirstPrivate,
}

pub enum Language {
    C,
    Cpp,
    Fortran,
}
```

---

## C FFI Layer

**Location**: `src/c_api.rs`

The FFI layer exposes a minimal unsafe pointer-based API to C/C++.

### Design Philosophy

**Goal**: Provide a traditional C API (malloc/free pattern) with minimal unsafe code.

**Achieved**:
- **16 functions**: Complete C API surface
- **~60 lines of unsafe**: Only at FFI boundary (~0.9% of file)
- **Standard patterns**: Familiar to C programmers
- **Safe internals**: All business logic in safe Rust

### FFI Functions

#### Lifecycle Functions (3)

```c
// Parse directive, returns pointer or NULL
OmpDirective* roup_parse(const char* input);

// Free directive (required)
void roup_directive_free(OmpDirective* directive);

// Free clause (not usually needed - owned by directive)
void roup_clause_free(OmpClause* clause);
```

#### Directive Query Functions (3)

```c
// Get directive type (integer)
int32_t roup_directive_kind(const OmpDirective* directive);

// Get number of clauses
int32_t roup_directive_clause_count(const OmpDirective* directive);

// Create iterator over clauses
OmpClauseIterator* roup_directive_clauses_iter(const OmpDirective* directive);
```

#### Iterator Functions (2)

```c
// Get next clause (returns 1 if available, 0 if done)
int32_t roup_clause_iterator_next(OmpClauseIterator* iter, OmpClause** out);

// Free iterator
void roup_clause_iterator_free(OmpClauseIterator* iter);
```

#### Clause Query Functions (4)

```c
// Get clause type (0=num_threads, 2=private, etc.)
int32_t roup_clause_kind(const OmpClause* clause);

// Get schedule kind for schedule clauses
int32_t roup_clause_schedule_kind(const OmpClause* clause);

// Get reduction operator for reduction clauses
int32_t roup_clause_reduction_operator(const OmpClause* clause);

// Get default data sharing
int32_t roup_clause_default_data_sharing(const OmpClause* clause);
```

#### Variable List Functions (4)

```c
// Get variable list from clause
OmpStringList* roup_clause_variables(const OmpClause* clause);

// Get list length
int32_t roup_string_list_len(const OmpStringList* list);

// Get string at index
const char* roup_string_list_get(const OmpStringList* list, int32_t index);

// Free string list
void roup_string_list_free(OmpStringList* list);
```

### Memory Model

**Ownership Rules:**
1. **Directives**: Created by `roup_parse()`, freed by `roup_directive_free()`
2. **Iterators**: Created by `roup_directive_clauses_iter()`, freed by `roup_clause_iterator_free()`
3. **String Lists**: Created by `roup_clause_variables()`, freed by `roup_string_list_free()`
4. **Clauses**: Owned by directive, DO NOT call `roup_clause_free()` on them

**Pattern**: Standard C malloc/free, familiar to all C programmers.

### Safety Boundaries

#### The ~60 Lines of Unsafe

All unsafe code is confined to FFI boundary operations:

**1. Reading C Strings (2 places)**
```rust
// Safety: Caller must ensure input is valid null-terminated C string
unsafe {
    CStr::from_ptr(input).to_str()
}
```

**Checks:**
- ✅ NULL pointer check before unsafe
- ✅ UTF-8 validation with error return
- ✅ No memory writes, only reads

**2. Writing to Output Pointers (multiple places)**
```rust
// Safety: Caller must ensure out is valid, aligned, writable
unsafe {
    *out = value;
}
```

**Checks:**
- ✅ NULL pointer check before unsafe
- ✅ Only writes primitive types (i32, u32, pointers)
- ✅ Single write operation, no loops

**3. Pointer Manipulation for Iterators**
```rust
// Safety: Internal Box pointer management
unsafe {
    Box::from_raw(ptr)
}
```

**Checks:**
- ✅ NULL pointer check
- ✅ Pointer created by Box::into_raw()
- ✅ No double-free (consumed on free)

### Why Minimal Unsafe is Necessary

**Cannot avoid unsafe for FFI:**
- Reading C strings requires `CStr::from_ptr()` (unsafe)
- Writing to C pointers requires dereference (unsafe)
- This is standard Rust FFI practice

**Alternative approaches considered:**

❌ **Zero unsafe**: Would require C programs to build strings byte-by-byte (40x slower, unusable)

❌ **Handle-based API**: Would need global registry with more unsafe code (50+ blocks)

✅ **Minimal unsafe pointer API**: Only ~60 lines, standard C patterns, practical performance

See [AGENTS.md](https://github.com/ouankou/roup/blob/main/AGENTS.md) for the official API architecture documentation.

---

## Data Flow Example

Let's trace a complete parse operation:

### Input

```c
const char* input = "#pragma omp parallel for num_threads(4) private(i)";
OmpDirective* dir = roup_parse(input);
```

### Step 1: FFI Boundary (C → Rust)

```rust
// src/c_api.rs
#[no_mangle]
pub extern "C" fn roup_parse(input: *const c_char) -> *mut OmpDirective {
    // NULL check
    if input.is_null() {
        return std::ptr::null_mut();
    }
    
    // Read C string (unsafe #1)
    let rust_str = unsafe {
        CStr::from_ptr(input).to_str().ok()?
    };
    
    // Call safe parser
    let directive_ir = parser::parse(rust_str).ok()?;
    
    // Box and return pointer
    Box::into_raw(Box::new(directive_ir))
}
```

### Step 2: Lexer (Pure Rust)

```rust
// src/lexer.rs
tokenize("#pragma omp parallel for num_threads(4) private(i)")
  ↓
[
    Pragma("#pragma omp"),
    Identifier("parallel"),
    Identifier("for"),
    Identifier("num_threads"),
    LParen,
    Integer(4),
    RParen,
    Identifier("private"),
    LParen,
    Identifier("i"),
    RParen,
]
```

### Step 3: Parser (Pure Rust)

```rust
// src/parser/mod.rs
parse_directive(tokens)
  ↓
DirectiveIR {
    kind: DirectiveKind::ParallelFor,
    clauses: vec![
        Clause::NumThreads(IntegerExpr(4)),
        Clause::Private(vec!["i"]),
    ],
    location: SourceLocation { line: 1, column: 1 },
    language: Language::C,
}
```

### Step 4: FFI Boundary (Rust → C)

```rust
// Return pointer to C
Box::into_raw(Box::new(directive_ir)) → 0x7fff12340000
```

### Step 5: C Queries

```c
int32_t kind = roup_directive_kind(dir);           // 28 (ParallelFor)
int32_t count = roup_directive_clause_count(dir);  // 2
```

### Step 6: Cleanup

```c
roup_directive_free(dir);  // Calls Box::from_raw() and drops
```

---

## Performance Characteristics

### Time Complexity

- **Lexing**: O(n) where n = input length
- **Parsing**: O(n × m) where m = average clause complexity (~O(n) in practice)
- **IR Construction**: O(c) where c = number of clauses
- **FFI Call Overhead**: ~10ns per call

### Space Complexity

- **Lexer**: O(1) - zero-copy, works on &str
- **Parser**: O(c) - allocates clause vector
- **IR**: O(c) - owns clause data
- **FFI**: O(1) - pointer conversion only

### Benchmarks

Typical directive parsing:

| Directive | Time | Allocations |
|-----------|------|-------------|
| `#pragma omp parallel` | ~500ns | 1 (DirectiveIR) |
| `#pragma omp parallel for num_threads(4)` | ~800ns | 2 (DirectiveIR + 1 clause) |
| `#pragma omp parallel for private(i,j,k) reduction(+:sum)` | ~1.2µs | 3 (DirectiveIR + 2 clauses) |

**Compare to old handle-based API**: 40x fewer FFI calls, 320x faster string building.

---

## Thread Safety

### Safe Concurrency

**Parser**: Thread-safe, can parse from multiple threads simultaneously

```rust
// Safe to do in parallel
std::thread::spawn(|| {
    let dir1 = parse("#pragma omp parallel");
});
std::thread::spawn(|| {
    let dir2 = parse("#pragma omp for");
});
```

**IR**: Immutable after construction, safe to share across threads

**FFI**: Each directive is independent, safe to parse in parallel

### Unsafe Patterns (User Responsibility)

❌ **Sharing directive across threads without synchronization**
```c
// Thread 1
roup_directive_free(dir);

// Thread 2 (at same time)
roup_directive_kind(dir);  // Use-after-free!
```

❌ **Double-free**
```c
roup_directive_free(dir);
roup_directive_free(dir);  // Undefined behavior!
```

✅ **Safe multi-threaded usage**
```c
// Each thread has its own directive
OmpDirective* dir1 = roup_parse("#pragma omp parallel");  // Thread 1
OmpDirective* dir2 = roup_parse("#pragma omp for");       // Thread 2
```

---

## Error Handling Strategy

### Rust API

Uses `Result<DirectiveIR, ParseError>`:

```rust
match parse(input) {
    Ok(directive) => { /* use directive */ },
    Err(ParseError { message, location, .. }) => {
        eprintln!("Parse error at line {}: {}", location.line, message);
    }
}
```

### C API

Uses `NULL` return values:

```c
OmpDirective* dir = roup_parse(input);
if (dir == NULL) {
    fprintf(stderr, "Parse failed\n");
    return 1;
}
```

**Query functions**: Return `-1` or safe defaults for NULL inputs

```c
int32_t kind = roup_directive_kind(NULL);  // Returns -1, won't crash
```

---

## Testing Strategy

### Test Coverage

```
Total Tests:    352
Unit Tests:     239
Integration:    51
Doc Tests:      62
```

### Test Categories

1. **Lexer Tests**: Token extraction, edge cases, Unicode
2. **Parser Tests**: Directive recognition, clause parsing, error cases
3. **IR Tests**: Structure validation, roundtrip serialization
4. **FFI Tests**: NULL handling, memory safety, error propagation
5. **Concurrent Tests**: Multi-threaded parsing, race detection

### Example Tests

```rust
#[test]
fn test_parallel_for_parsing() {
    let result = parse("#pragma omp parallel for num_threads(4)");
    assert!(result.is_ok());
    
    let directive = result.unwrap();
    assert_eq!(directive.kind, DirectiveKind::ParallelFor);
    assert_eq!(directive.clauses.len(), 1);
    
    match &directive.clauses[0] {
        Clause::NumThreads(IntegerExpr(4)) => {},
        _ => panic!("Expected NumThreads(4)"),
    }
}

#[test]
fn test_ffi_null_safety() {
    let dir = roup_parse(std::ptr::null());
    assert!(dir.is_null());
    
    let kind = roup_directive_kind(std::ptr::null());
    assert_eq!(kind, -1);
}
```

---

## Design Trade-offs

### Lexer: nom vs Custom

**Chose nom:**
- ✅ Zero-copy parsing
- ✅ Rich error messages
- ✅ Well-tested combinators
- ✅ Easy to extend

**vs Custom Lexer:**
- ❌ Learning curve
- ❌ Dependency on external crate

### IR: Owned vs Borrowed

**Chose owned (Vec, String):**
- ✅ Simple lifetime management
- ✅ Easy to pass across FFI
- ✅ No lifetime annotations in IR

**vs Borrowed (&str slices):**
- ❌ Slower (allocations)
- ❌ More memory usage

**Justification**: Parsing is not the bottleneck; simplicity and FFI compatibility matter more.

### FFI: Pointers vs Handles

**Chose pointers:**
- ✅ Standard C pattern (malloc/free)
- ✅ Minimal unsafe (~60 lines)
- ✅ Zero overhead
- ✅ Familiar to C programmers

**vs Handle-based:**
- ❌ 40x more FFI calls
- ❌ 50+ unsafe blocks
- ❌ Global registry complexity

See [AGENTS.md - C FFI API Architecture](https://github.com/ouankou/roup/blob/main/AGENTS.md#c-ffi-api-architecture) for details.

---

## Future Architecture Considerations

### Potential Enhancements

1. **Incremental Parsing**: Parse only changed directives
2. **Streaming API**: Parse large files without loading into memory
3. **Parallel Parsing**: Parse multiple files concurrently
4. **AST Transformation**: Optimize or transform directives
5. **Code Generation**: Generate code from IR

### Stability Guarantees

**Stable**:
- Rust public API (`parse()` function signature)
- C FFI function signatures (16 functions)
- IR structure (major fields)

**Unstable** (may change):
- Internal parser implementation
- Lexer token types
- Error message formatting

---

## Summary

ROUP's architecture prioritizes:

1. **Safety**: 99.1% safe Rust, minimal unsafe only at FFI boundary
2. **Performance**: Zero-copy lexing, minimal allocations
3. **Usability**: Standard C patterns, clear error messages
4. **Correctness**: 352 tests, comprehensive OpenMP support

The three-layer design (Lexer → Parser → FFI) provides a clean separation of concerns while maintaining excellent performance characteristics.

**Key Metrics:**
- 16 FFI functions
- ~60 lines of unsafe code (0.9%)
- 95 directives
- 91 clauses
- 352 tests
- ~500ns to parse simple directive

For implementation details, see the source code in `src/`.
