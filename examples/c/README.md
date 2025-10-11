# Roup C Examples

This directory contains example C programs demonstrating the Roup OpenMP parser C API.

## Overview

The Roup library provides a 100% safe Rust-based OpenMP parser with a minimal unsafe C FFI. The C API uses direct pointers with explicit memory management for simplicity and efficiency.

## API Design

**Memory Model:** Direct pointers (no handle registry)  
**Error Handling:** NULL pointers on failure  
**Resource Management:** Explicit free functions  
**Function Prefix:** `roup_*` (not `omp_*`)

### Key Principles

1. **Minimal Unsafe:** Only 18 unsafe blocks (~60 lines of code)
2. **Explicit Ownership:** Caller owns all returned pointers
3. **NULL-Safe:** Always check for NULL before dereferencing
4. **No Hidden State:** No global registry or hidden allocations
5. **C-Compatible:** Standard C malloc/free patterns

## Available Example

### tutorial_basic.c
**Comprehensive tutorial demonstrating all API features**

**Topics covered:**
- Basic parsing with `roup_parse()`
- Directive queries: `roup_directive_kind()`, `roup_directive_clause_count()`
- Clause iteration with `roup_directive_clauses_iter()` and `roup_clause_iterator_next()`
- Clause queries: `roup_clause_kind()`, `roup_clause_schedule_kind()`, `roup_clause_reduction_operator()`
- Variable lists: `roup_clause_variables()`, `roup_string_list_*()` functions
- Error handling (NULL checks)
- Memory management (explicit free functions)

**Features demonstrated:**
- 6 comprehensive steps
- Multiple directive types
- Complex clause inspection
- Proper resource cleanup
- Error handling patterns

## Building

### Prerequisites
- C compiler (gcc, clang, or compatible)
- Rust toolchain (to build the Roup library)
- Linux/Unix environment (tested on Ubuntu 24.04)

### Build the Tutorial

```bash
# Step 1: Build the Rust library (release mode recommended)
cd /workspaces/roup
cargo build --release

# Step 2: Compile the C tutorial
cd examples/c
clang -std=c11 -Wall -Wextra \
    -I../../target/release \
    tutorial_basic.c \
    -L../../target/release \
    -lroup \
    -o tutorial

# Or use gcc:
gcc -std=c11 -Wall -Wextra \
    -I../../target/release \
    tutorial_basic.c \
    -L../../target/release \
    -lroup \
    -o tutorial
```

### Quick Build

```bash
# All-in-one command
cargo build --release && \
cd examples/c && \
clang tutorial_basic.c \
    -I../../target/release \
    -L../../target/release \
    -lroup \
    -o tutorial
```

## Running

```bash
# Set library path and run
cd examples/c
LD_LIBRARY_PATH=../../target/release ./tutorial
```

**Note:** The `LD_LIBRARY_PATH` is required so the system can find `libroup.so` at runtime.

## Expected Output

```
╔════════════════════════════════════════════════════════════╗
║       OpenMP Parser C Tutorial (Minimal Unsafe API)       ║
╚════════════════════════════════════════════════════════════╝

STEP 1: Parse a Simple OpenMP Directive
✅ Parse succeeded!
Directive: 0x6015fd921a60 (non-NULL pointer)
Kind: 0 (PARALLEL)
Clauses: 0
✓ Memory freed

STEP 2: Parse Directive with Multiple Clauses
✅ Parse succeeded!
Directive: PARALLEL
Clauses: 3

STEP 3: Iterate Through Clauses
✅ Parse succeeded!
1. NUM_THREADS (kind=0)
2. DEFAULT (kind=11)
3. NOWAIT (kind=10)

STEP 4: Query Clause-Specific Data
✅ Parse succeeded!
1. SCHEDULE → kind=1 (dynamic)
2. REDUCTION → operator=0 (+)

STEP 5: Error Handling (NULL Checks)
Testing invalid syntax...
✅ Correctly returned NULL for invalid input
Testing empty string...
✅ Correctly returned NULL for empty input
Testing NULL pointer query...
✅ Correctly returned -1 for NULL directive

STEP 6: Multiple Directive Types
Testing various directive types...
✓ PARALLEL (kind=0)
✓ FOR (kind=1)
✓ TASK (kind=4)
✓ BARRIER (kind=7)
✓ TARGET (kind=13)
✓ TEAMS (kind=14)

✅ Tutorial completed successfully!
```

## C API Reference

### Core Types (Opaque Pointers)

```c
struct OmpDirective;      // Parsed directive
struct OmpClause;         // Individual clause
struct OmpClauseIterator; // Iterator over clauses
struct OmpStringList;     // List of strings (variables)
```

### Lifecycle Functions

```c
// Parse directive from string (returns NULL on error)
OmpDirective* roup_parse(const char* input);

// Free directive (sets all owned clauses to freed state)
void roup_directive_free(OmpDirective* directive);

// Free individual clause (if not owned by directive)
void roup_clause_free(OmpClause* clause);
```

### Directive Queries

```c
// Get directive kind (returns -1 if directive is NULL)
// 0=PARALLEL, 1=FOR, 2=SECTIONS, 4=TASK, 7=BARRIER, etc.
int32_t roup_directive_kind(const OmpDirective* directive);

// Get clause count (returns 0 if directive is NULL)
int32_t roup_directive_clause_count(const OmpDirective* directive);

// Create iterator for clauses (returns NULL if directive is NULL)
OmpClauseIterator* roup_directive_clauses_iter(const OmpDirective* directive);
```

### Iterator Functions

```c
// Advance iterator, store next clause in *out
// Returns 1 if clause retrieved, 0 if end reached
int32_t roup_clause_iterator_next(OmpClauseIterator* iter, OmpClause** out);

// Free iterator
void roup_clause_iterator_free(OmpClauseIterator* iter);
```

### Clause Queries

```c
// Get clause kind (returns -1 if clause is NULL)
// 0=NUM_THREADS, 2=PRIVATE, 6=REDUCTION, 7=SCHEDULE, etc.
int32_t roup_clause_kind(const OmpClause* clause);

// Get schedule kind for SCHEDULE clause (returns -1 if not applicable)
// 0=static, 1=dynamic, 2=guided, 3=auto, 4=runtime
int32_t roup_clause_schedule_kind(const OmpClause* clause);

// Get reduction operator (returns -1 if not applicable)
// 0=+, 1=-, 2=*, 6=&&, 7=||, 8=min, 9=max
int32_t roup_clause_reduction_operator(const OmpClause* clause);

// Get default data-sharing (returns -1 if not applicable)
// 0=shared, 1=none
int32_t roup_clause_default_data_sharing(const OmpClause* clause);
```

### Variable List Functions

```c
// Get variable list from clause (returns NULL if not applicable)
OmpStringList* roup_clause_variables(const OmpClause* clause);

// Get number of variables in list (returns 0 if list is NULL)
int32_t roup_string_list_len(const OmpStringList* list);

// Get variable at index (returns NULL if out of bounds)
// Returned string is valid until list is freed
const char* roup_string_list_get(const OmpStringList* list, int32_t index);

// Free variable list
void roup_string_list_free(OmpStringList* list);
```

## Memory Management Guide

### Basic Pattern

```c
// 1. Parse (allocates memory)
OmpDirective* dir = roup_parse("#pragma omp parallel");

// 2. Use (check NULL first!)
if (dir != NULL) {
    int32_t kind = roup_directive_kind(dir);
    // ... use directive ...
}

// 3. Free (always, even if NULL - it's a no-op)
roup_directive_free(dir);
```

### Iterator Pattern

```c
OmpDirective* dir = roup_parse("#pragma omp parallel num_threads(4)");
if (!dir) return;

// Create iterator
OmpClauseIterator* iter = roup_directive_clauses_iter(dir);

// Iterate
OmpClause* clause;
while (roup_clause_iterator_next(iter, &clause)) {
    int32_t kind = roup_clause_kind(clause);
    // ... process clause ...
    // Don't free clause! It's owned by directive.
}

// Free iterator
roup_clause_iterator_free(iter);

// Free directive (also frees all clauses)
roup_directive_free(dir);
```

### Variable List Pattern

```c
// Get variables from a clause
OmpStringList* vars = roup_clause_variables(clause);
if (vars) {
    int32_t count = roup_string_list_len(vars);
    for (int32_t i = 0; i < count; i++) {
        const char* var = roup_string_list_get(vars, i);
        printf("Variable: %s\n", var);
    }
    roup_string_list_free(vars);  // MUST free!
}
```

## Error Handling Best Practices

### Always Check for NULL

```c
OmpDirective* dir = roup_parse(input);
if (dir == NULL) {
    fprintf(stderr, "Parse failed!\n");
    return -1;
}
// ... use dir safely ...
roup_directive_free(dir);
```

### Safe Queries

```c
// Query functions are NULL-safe
int32_t kind = roup_directive_kind(NULL);  // Returns -1
int32_t count = roup_directive_clause_count(NULL);  // Returns 0

// But still check for semantic meaning
if (kind == -1) {
    fprintf(stderr, "Invalid directive\n");
}
```

### Cleanup on Error

```c
OmpDirective* dir = roup_parse(input);
if (!dir) goto cleanup;

OmpClauseIterator* iter = roup_directive_clauses_iter(dir);
if (!iter) goto cleanup;

// ... process ...

cleanup:
    if (iter) roup_clause_iterator_free(iter);
    if (dir) roup_directive_free(dir);
```

## Common Patterns

### Directive Kind Names

```c
const char* directive_name(int32_t kind) {
    switch (kind) {
        case 0: return "PARALLEL";
        case 1: return "FOR";
        case 2: return "SECTIONS";
        case 4: return "TASK";
        case 7: return "BARRIER";
        case 13: return "TARGET";
        case 14: return "TEAMS";
        default: return "UNKNOWN";
    }
}
```

### Clause Kind Names

```c
const char* clause_name(int32_t kind) {
    switch (kind) {
        case 0: return "NUM_THREADS";
        case 2: return "PRIVATE";
        case 3: return "SHARED";
        case 6: return "REDUCTION";
        case 7: return "SCHEDULE";
        case 10: return "NOWAIT";
        case 11: return "DEFAULT";
        default: return "UNKNOWN";
    }
}
```

### Schedule Kind Names

```c
const char* schedule_name(int32_t kind) {
    switch (kind) {
        case 0: return "static";
        case 1: return "dynamic";
        case 2: return "guided";
        case 3: return "auto";
        case 4: return "runtime";
        default: return "unknown";
    }
}
```

## Differences from Old API

If you used the old handle-based API, here are the key changes:

| Old API (Handle-based) | New API (Pointer-based) |
|------------------------|-------------------------|
| `omp_parse_cstr()` | `roup_parse()` |
| `Handle` (uint64_t) | Direct pointers |
| `OmpStatus` return codes | NULL on error |
| `OMP_SUCCESS` checks | NULL checks |
| Handle registry | No registry |
| `omp_directive_free()` | `roup_directive_free()` |

**Migration:** Update function names, replace handle checks with NULL checks, remove status code handling.

## Thread Safety

- **Parsing:** Thread-safe (no shared state)
- **Queries:** Thread-safe (read-only operations)
- **Freeing:** NOT thread-safe (caller must synchronize)

**Best practice:** Each thread should own its parsed directives.

## Troubleshooting

### Library Not Found at Runtime
```
error while loading shared libraries: libroup.so: cannot open shared object file
```

**Solution:**
```bash
export LD_LIBRARY_PATH=/workspaces/roup/target/release:$LD_LIBRARY_PATH
```

### Segmentation Fault

**Common causes:**
1. Using pointer after freeing it
2. Not checking for NULL
3. Freeing same pointer twice
4. Dereferencing NULL from failed parse

**Solution:** Always check for NULL, never use after free.

### Parse Returns NULL

**Possible causes:**
1. Invalid OpenMP syntax
2. Empty or NULL input string
3. Malformed directive

**Solution:** Validate input before parsing.

## Further Reading

- **API Implementation:** `src/c_api.rs` - Full source code
- **C Header:** Generate with cbindgen or see `src/c_api.rs` comments
- **C++ Tutorial:** `examples/cpp/tutorial_basic.cpp` - RAII wrappers
- **OpenMP Support:** `docs/OPENMP_SUPPORT.md` - Supported directives/clauses
- **Rust Documentation:** Run `cargo doc --open` for internal APIs

## License

Same as the parent Roup project (MIT/Apache-2.0).
