# Roup C API Documentation

Complete reference for the Roup OpenMP Parser C Foreign Function Interface (FFI).

## Table of Contents

1. [Overview](#overview)
2. [Quick Start](#quick-start)
3. [Core Concepts](#core-concepts)
4. [API Reference](#api-reference)
   - [String API](#string-api)
   - [Parser API](#parser-api)
   - [Directive Query API](#directive-query-api)
   - [Clause Query API](#clause-query-api)
5. [Error Handling](#error-handling)
6. [Memory Management](#memory-management)
7. [Thread Safety](#thread-safety)
8. [Examples](#examples)
9. [Building and Linking](#building-and-linking)

## Overview

The Roup C FFI provides a safe, ergonomic interface for parsing and querying OpenMP directives from C code. The library is implemented in 100% safe Rust with zero `unsafe` blocks, ensuring memory safety and thread safety.

### Key Features

- **Zero-copy parsing:** Parse OpenMP directives without unnecessary allocations
- **Comprehensive coverage:** Support for 70+ directive kinds and 90+ clause types
- **Type-safe queries:** Typed accessors for clause data (num_threads, schedule, reduction, etc.)
- **Iterator pattern:** Cursor-based iteration over clauses
- **Memory safe:** No raw pointers, all resources managed through opaque handles
- **Thread safe:** Global registry protected by mutex, handles valid across threads
- **Error handling:** Comprehensive status codes for all error conditions

### Design Philosophy

1. **No unsafe code:** The entire implementation is safe Rust
2. **Handle-based:** All resources represented as opaque `Handle` (uint64_t) values
3. **Explicit cleanup:** Resources must be explicitly freed by the caller
4. **Status codes:** All functions return `OmpStatus` for comprehensive error reporting
5. **No hidden state:** All operations are explicit and predictable

## Quick Start

### Basic Example

```c
#include "roup.h"
#include <stdio.h>
#include <stdlib.h>

int main() {
    // 1. Parse OpenMP directive
    Handle result;
    OmpStatus status = omp_parse("#pragma omp parallel num_threads(4)", 
                                  OMP_LANG_C, &result);
    if (status != OMP_SUCCESS) {
        printf("Parse failed\n");
        return 1;
    }
    
    // 2. Extract directive handles
    Handle *directives;
    uintptr_t count;
    omp_take_last_parse_result(&directives, &count);
    
    // 3. Query directive properties
    if (count > 0) {
        DirectiveKind kind;
        omp_directive_kind(directives[0], &kind);
        printf("Directive kind: %d\n", kind);
        
        uintptr_t clause_count;
        omp_directive_clause_count(directives[0], &clause_count);
        printf("Number of clauses: %zu\n", clause_count);
    }
    
    // 4. Clean up
    free(directives);
    omp_parse_result_free(result);
    
    return 0;
}
```

### Build and Run

```bash
# Build the Rust library
cargo build --lib

# Compile the C program
gcc -o example example.c -I./include -L./target/debug -lroup -lpthread -ldl -lm

# Run with library path
LD_LIBRARY_PATH=./target/debug ./example
```

## Core Concepts

### Handles

All resources are represented as opaque `Handle` values (typedef uint64_t). Handles are:
- **Opaque:** Internal implementation hidden from C code
- **Non-zero:** Valid handles are always > 0
- **Unique:** Each resource has a unique handle value
- **Cross-thread:** Valid across thread boundaries

```c
typedef uint64_t Handle;
#define INVALID_HANDLE ((Handle)0)

// Check if handle is valid
if (OMP_IS_VALID(handle)) {
    // Use handle
}

if (OMP_IS_INVALID(handle)) {
    // Handle is invalid
}
```

### Status Codes

All functions return `OmpStatus` to indicate success or failure:

```c
typedef enum {
    OMP_SUCCESS = 0,           // Operation succeeded
    OMP_INVALID_HANDLE = 1,    // Handle not found in registry
    OMP_INVALID_UTF8 = 2,      // String contains invalid UTF-8
    OMP_NULL_POINTER = 3,      // Required pointer is NULL
    OMP_OUT_OF_BOUNDS = 4,     // Index exceeds bounds
    OMP_PARSE_ERROR = 5,       // Parse failed
    OMP_TYPE_MISMATCH = 6,     // Clause type doesn't match
    OMP_EMPTY_RESULT = 7,      // No results available
} OmpStatus;

// Always check status before using output
OmpStatus status = omp_parse(input, lang, &result);
if (status != OMP_SUCCESS) {
    // Handle error
    return status;
}
```

### Resource Types

The FFI manages several resource types:

| Resource Type | Description | Free Function |
|--------------|-------------|---------------|
| String | Mutable string buffer | `omp_str_free()` |
| Parse Result | Array of directives | `omp_parse_result_free()` |
| Directive | Parsed OpenMP directive | `omp_directive_free()` |
| Clause | Directive clause | `omp_clause_free()` |
| Cursor | Clause iterator | `omp_cursor_free()` |

**Important:** Parse results own their directives. When you free a parse result, all associated directives are also freed. You typically don't need to call `omp_directive_free()` or `omp_clause_free()` explicitly.

## API Reference

### String API

The string API provides mutable string buffers for building and manipulating strings.

#### omp_str_new

Create a new empty string.

```c
OmpStatus omp_str_new(Handle *out_handle);
```

**Parameters:**
- `out_handle`: Output parameter for string handle

**Returns:** `OMP_SUCCESS` or `OMP_NULL_POINTER`

**Example:**
```c
Handle str;
if (omp_str_new(&str) == OMP_SUCCESS) {
    // Use string
    omp_str_free(str);
}
```

#### omp_str_from_cstr

Create a string from a null-terminated C string.

```c
OmpStatus omp_str_from_cstr(const char *c_str, Handle *out_handle);
```

**Parameters:**
- `c_str`: Null-terminated C string
- `out_handle`: Output parameter for string handle

**Returns:** `OMP_SUCCESS`, `OMP_NULL_POINTER`, or `OMP_INVALID_UTF8`

**Example:**
```c
Handle str;
if (omp_str_from_cstr("Hello World", &str) == OMP_SUCCESS) {
    // Use string
    omp_str_free(str);
}
```

#### omp_str_push_cstr

Append a C string to an existing string.

```c
OmpStatus omp_str_push_cstr(Handle handle, const char *c_str);
```

**Parameters:**
- `handle`: String handle
- `c_str`: Null-terminated C string to append

**Returns:** `OMP_SUCCESS`, `OMP_INVALID_HANDLE`, `OMP_NULL_POINTER`, or `OMP_INVALID_UTF8`

**Example:**
```c
Handle str;
omp_str_new(&str);
omp_str_push_cstr(str, "Hello");
omp_str_push_cstr(str, " World");
omp_str_free(str);
```

#### omp_str_push_bytes

Append raw bytes to a string.

```c
OmpStatus omp_str_push_bytes(Handle handle, const uint8_t *data, uintptr_t len);
```

**Parameters:**
- `handle`: String handle
- `data`: Pointer to bytes
- `len`: Number of bytes

**Returns:** `OMP_SUCCESS`, `OMP_INVALID_HANDLE`, or `OMP_INVALID_UTF8`

#### omp_str_len

Get the length of a string in bytes.

```c
OmpStatus omp_str_len(Handle handle, uintptr_t *out_len);
```

**Parameters:**
- `handle`: String handle
- `out_len`: Output parameter for length

**Returns:** `OMP_SUCCESS`, `OMP_INVALID_HANDLE`, or `OMP_NULL_POINTER`

**Example:**
```c
uintptr_t len;
if (omp_str_len(str, &len) == OMP_SUCCESS) {
    printf("String length: %zu\n", len);
}
```

#### omp_str_capacity

Get the capacity of a string in bytes.

```c
OmpStatus omp_str_capacity(Handle handle, uintptr_t *out_capacity);
```

**Returns:** `OMP_SUCCESS`, `OMP_INVALID_HANDLE`, or `OMP_NULL_POINTER`

#### omp_str_copy_to_buffer

Copy string contents to a C buffer.

```c
OmpStatus omp_str_copy_to_buffer(Handle handle, char *buffer, 
                                  uintptr_t buffer_size, uintptr_t *out_len);
```

**Parameters:**
- `handle`: String handle
- `buffer`: Output buffer (must have space for length + 1 bytes)
- `buffer_size`: Size of output buffer
- `out_len`: Output parameter for bytes written (excluding null terminator)

**Returns:** `OMP_SUCCESS`, `OMP_INVALID_HANDLE`, `OMP_NULL_POINTER`, or `OMP_OUT_OF_BOUNDS`

**Example:**
```c
uintptr_t len;
omp_str_len(str, &len);

char *buffer = malloc(len + 1);
uintptr_t written;
if (omp_str_copy_to_buffer(str, buffer, len + 1, &written) == OMP_SUCCESS) {
    printf("String: %s\n", buffer);
}
free(buffer);
```

#### omp_str_clear

Clear a string (length becomes 0, capacity unchanged).

```c
OmpStatus omp_str_clear(Handle handle);
```

**Returns:** `OMP_SUCCESS` or `OMP_INVALID_HANDLE`

#### omp_str_is_empty

Check if a string is empty.

```c
OmpStatus omp_str_is_empty(Handle handle, bool *out_is_empty);
```

**Returns:** `OMP_SUCCESS`, `OMP_INVALID_HANDLE`, or `OMP_NULL_POINTER`

#### omp_str_free

Free a string and remove from registry.

```c
OmpStatus omp_str_free(Handle handle);
```

**Returns:** `OMP_SUCCESS` or `OMP_INVALID_HANDLE`

**Important:** After calling this, the handle becomes invalid.

### Parser API

The parser API provides functions to parse OpenMP directives from text.

#### omp_parse

Parse OpenMP directives from input text.

```c
OmpStatus omp_parse(const char *input, Language lang, Handle *out_handle);
```

**Parameters:**
- `input`: Input text containing OpenMP directives
- `lang`: Programming language (`OMP_LANG_C` or `OMP_LANG_FORTRAN`)
- `out_handle`: Output parameter for parse result handle

**Returns:** `OMP_SUCCESS`, `OMP_NULL_POINTER`, `OMP_INVALID_UTF8`, or `OMP_PARSE_ERROR`

**Example:**
```c
Handle result;
OmpStatus status = omp_parse("#pragma omp parallel", OMP_LANG_C, &result);
if (status == OMP_SUCCESS) {
    // Use result
    omp_parse_result_free(result);
}
```

**Supported Languages:**
```c
typedef enum {
    OMP_LANG_C = 0,       // C/C++ (#pragma omp)
    OMP_LANG_FORTRAN = 1, // Fortran (!$omp, c$omp, *$omp)
} Language;
```

#### omp_take_last_parse_result

Extract directive handles from the last parse result.

```c
OmpStatus omp_take_last_parse_result(Handle **out_directives, uintptr_t *out_count);
```

**Parameters:**
- `out_directives`: Output parameter for array of directive handles (caller must free)
- `out_count`: Output parameter for number of directives

**Returns:** `OMP_SUCCESS`, `OMP_NULL_POINTER`, or `OMP_EMPTY_RESULT`

**Important:** The returned array is heap-allocated and must be freed with `free()`.

**Example:**
```c
Handle result;
omp_parse("#pragma omp parallel\n#pragma omp for", OMP_LANG_C, &result);

Handle *directives;
uintptr_t count;
if (omp_take_last_parse_result(&directives, &count) == OMP_SUCCESS) {
    printf("Parsed %zu directives\n", count);
    for (uintptr_t i = 0; i < count; i++) {
        // Use directives[i]
    }
    free(directives);  // Must free the array
}
omp_parse_result_free(result);
```

#### omp_parse_result_free

Free a parse result and all associated directives.

```c
OmpStatus omp_parse_result_free(Handle handle);
```

**Returns:** `OMP_SUCCESS` or `OMP_INVALID_HANDLE`

**Important:** This also frees all directive and clause handles associated with this parse result.

### Directive Query API

The directive query API provides functions to inspect parsed directives.

#### omp_directive_kind

Get the kind of a directive.

```c
OmpStatus omp_directive_kind(Handle handle, DirectiveKind *out_kind);
```

**Parameters:**
- `handle`: Directive handle
- `out_kind`: Output parameter for directive kind

**Returns:** `OMP_SUCCESS`, `OMP_INVALID_HANDLE`, or `OMP_NULL_POINTER`

**Example:**
```c
DirectiveKind kind;
if (omp_directive_kind(dir, &kind) == OMP_SUCCESS) {
    if (kind == OMP_DIR_PARALLEL) {
        printf("Parallel directive\n");
    }
}
```

**Directive Kinds:** See `DirectiveKind` enum in roup.h for all 70+ directive kinds.

#### omp_directive_clause_count

Get the number of clauses in a directive.

```c
OmpStatus omp_directive_clause_count(Handle handle, uintptr_t *out_count);
```

**Returns:** `OMP_SUCCESS`, `OMP_INVALID_HANDLE`, or `OMP_NULL_POINTER`

#### omp_directive_line

Get the source line number of a directive (1-indexed).

```c
OmpStatus omp_directive_line(Handle handle, uintptr_t *out_line);
```

**Returns:** `OMP_SUCCESS`, `OMP_INVALID_HANDLE`, or `OMP_NULL_POINTER`

#### omp_directive_column

Get the source column number of a directive (0-indexed).

```c
OmpStatus omp_directive_column(Handle handle, uintptr_t *out_column);
```

**Returns:** `OMP_SUCCESS`, `OMP_INVALID_HANDLE`, or `OMP_NULL_POINTER`

#### omp_directive_language

Get the programming language of a directive.

```c
OmpStatus omp_directive_language(Handle handle, Language *out_lang);
```

**Returns:** `OMP_SUCCESS`, `OMP_INVALID_HANDLE`, or `OMP_NULL_POINTER`

#### omp_directive_clauses_cursor

Create a cursor for iterating over directive clauses.

```c
OmpStatus omp_directive_clauses_cursor(Handle handle, Handle *out_cursor);
```

**Parameters:**
- `handle`: Directive handle
- `out_cursor`: Output parameter for cursor handle

**Returns:** `OMP_SUCCESS`, `OMP_INVALID_HANDLE`, or `OMP_NULL_POINTER`

**Example:**
```c
Handle cursor;
if (omp_directive_clauses_cursor(dir, &cursor) == OMP_SUCCESS) {
    bool done;
    while (omp_cursor_is_done(cursor, &done) == OMP_SUCCESS && !done) {
        Handle clause;
        if (omp_cursor_current(cursor, &clause) == OMP_SUCCESS) {
            // Use clause
        }
        omp_cursor_next(cursor);
    }
    omp_cursor_free(cursor);
}
```

#### omp_cursor_next

Move cursor to the next clause.

```c
OmpStatus omp_cursor_next(Handle handle);
```

**Returns:** `OMP_SUCCESS` or `OMP_INVALID_HANDLE`

#### omp_cursor_current

Get the current clause from the cursor.

```c
OmpStatus omp_cursor_current(Handle handle, Handle *out_clause);
```

**Parameters:**
- `handle`: Cursor handle
- `out_clause`: Output parameter for clause handle (INVALID_HANDLE if done)

**Returns:** `OMP_SUCCESS`, `OMP_INVALID_HANDLE`, or `OMP_NULL_POINTER`

#### omp_cursor_is_done

Check if the cursor has reached the end.

```c
OmpStatus omp_cursor_is_done(Handle handle, bool *out_is_done);
```

**Returns:** `OMP_SUCCESS`, `OMP_INVALID_HANDLE`, or `OMP_NULL_POINTER`

#### omp_cursor_reset

Reset cursor to the beginning.

```c
OmpStatus omp_cursor_reset(Handle handle);
```

**Returns:** `OMP_SUCCESS` or `OMP_INVALID_HANDLE`

#### omp_cursor_total

Get the total number of clauses in the cursor.

```c
OmpStatus omp_cursor_total(Handle handle, uintptr_t *out_total);
```

**Returns:** `OMP_SUCCESS`, `OMP_INVALID_HANDLE`, or `OMP_NULL_POINTER`

#### omp_cursor_position

Get the current position in the cursor (0-indexed).

```c
OmpStatus omp_cursor_position(Handle handle, uintptr_t *out_position);
```

**Returns:** `OMP_SUCCESS`, `OMP_INVALID_HANDLE`, or `OMP_NULL_POINTER`

#### omp_cursor_free

Free a cursor.

```c
OmpStatus omp_cursor_free(Handle handle);
```

**Returns:** `OMP_SUCCESS` or `OMP_INVALID_HANDLE`

#### omp_directive_free

Free a directive (usually not needed).

```c
OmpStatus omp_directive_free(Handle handle);
```

**Returns:** `OMP_SUCCESS` or `OMP_INVALID_HANDLE`

**Note:** Directives are automatically freed when the parse result is freed.

### Clause Query API

The clause query API provides functions to inspect clause data.

#### omp_clause_at

Get a clause at a specific index.

```c
OmpStatus omp_clause_at(Handle directive_handle, uintptr_t index, Handle *out_clause);
```

**Parameters:**
- `directive_handle`: Directive handle
- `index`: Clause index (0-based)
- `out_clause`: Output parameter for clause handle

**Returns:** `OMP_SUCCESS`, `OMP_INVALID_HANDLE`, `OMP_NULL_POINTER`, or `OMP_OUT_OF_BOUNDS`

**Example:**
```c
Handle clause;
if (omp_clause_at(dir, 0, &clause) == OMP_SUCCESS) {
    // Use clause
}
```

#### omp_clause_type

Get the type of a clause.

```c
OmpStatus omp_clause_type(Handle handle, ClauseType *out_type);
```

**Returns:** `OMP_SUCCESS`, `OMP_INVALID_HANDLE`, or `OMP_NULL_POINTER`

**Clause Types:** See `ClauseType` enum in roup.h for all 90+ clause types.

#### Typed Accessors

These functions extract specific data from clauses. They return `OMP_TYPE_MISMATCH` if the clause type doesn't match.

##### omp_clause_num_threads_value

Get num_threads value (returns string handle with expression).

```c
OmpStatus omp_clause_num_threads_value(Handle handle, Handle *out_value);
```

**Returns:** `OMP_SUCCESS`, `OMP_INVALID_HANDLE`, `OMP_NULL_POINTER`, or `OMP_TYPE_MISMATCH`

**Example:**
```c
// "#pragma omp parallel num_threads(4)"
Handle value;
if (omp_clause_num_threads_value(clause, &value) == OMP_SUCCESS) {
    // value is a string handle containing "4"
    uintptr_t len;
    omp_str_len(value, &len);
    char *buf = malloc(len + 1);
    omp_str_copy_to_buffer(value, buf, len + 1, &len);
    printf("num_threads: %s\n", buf);
    free(buf);
    omp_str_free(value);
}
```

##### omp_clause_default_kind

Get default kind.

```c
OmpStatus omp_clause_default_kind(Handle handle, DefaultKind *out_kind);
```

**Default Kinds:**
```c
typedef enum {
    OMP_DEFAULT_SHARED = 0,
    OMP_DEFAULT_NONE = 1,
    OMP_DEFAULT_PRIVATE = 2,
    OMP_DEFAULT_FIRSTPRIVATE = 3,
} DefaultKind;
```

##### omp_clause_schedule_kind

Get schedule kind.

```c
OmpStatus omp_clause_schedule_kind(Handle handle, ScheduleKind *out_kind);
```

**Schedule Kinds:**
```c
typedef enum {
    OMP_SCHEDULE_STATIC = 0,
    OMP_SCHEDULE_DYNAMIC = 1,
    OMP_SCHEDULE_GUIDED = 2,
    OMP_SCHEDULE_AUTO = 3,
    OMP_SCHEDULE_RUNTIME = 4,
} ScheduleKind;
```

##### omp_clause_schedule_chunk_size

Get schedule chunk size (returns string handle, INVALID_HANDLE if none).

```c
OmpStatus omp_clause_schedule_chunk_size(Handle handle, Handle *out_chunk_size);
```

##### omp_clause_reduction_operator

Get reduction operator.

```c
OmpStatus omp_clause_reduction_operator(Handle handle, ReductionOperator *out_op);
```

**Reduction Operators:**
```c
typedef enum {
    OMP_REDUCTION_ADD = 0,      // +
    OMP_REDUCTION_MULTIPLY = 1, // *
    OMP_REDUCTION_SUBTRACT = 2, // -
    OMP_REDUCTION_AND = 3,      // &
    OMP_REDUCTION_OR = 4,       // |
    OMP_REDUCTION_XOR = 5,      // ^
    OMP_REDUCTION_LAND = 6,     // &&
    OMP_REDUCTION_LOR = 7,      // ||
    OMP_REDUCTION_MIN = 8,      // min
    OMP_REDUCTION_MAX = 9,      // max
    OMP_REDUCTION_CUSTOM = 10,  // Custom identifier
} ReductionOperator;
```

##### omp_clause_reduction_identifier

Get custom reduction identifier (only for CUSTOM operator).

```c
OmpStatus omp_clause_reduction_identifier(Handle handle, Handle *out_identifier);
```

#### List Clause Accessors

These functions work with clauses that contain lists of variables (private, shared, etc.).

##### omp_clause_item_count

Get the number of items in a list clause.

```c
OmpStatus omp_clause_item_count(Handle handle, uintptr_t *out_count);
```

##### omp_clause_item_at

Get an item at a specific index.

```c
OmpStatus omp_clause_item_at(Handle handle, uintptr_t index, Handle *out_item);
```

**Returns:** String handle containing variable name

**Example:**
```c
// "#pragma omp parallel private(x, y, z)"
uintptr_t count;
omp_clause_item_count(clause, &count);  // count = 3

for (uintptr_t i = 0; i < count; i++) {
    Handle item;
    if (omp_clause_item_at(clause, i, &item) == OMP_SUCCESS) {
        // item is string handle containing "x", "y", or "z"
        uintptr_t len;
        omp_str_len(item, &len);
        char *buf = malloc(len + 1);
        omp_str_copy_to_buffer(item, buf, len + 1, &len);
        printf("Variable: %s\n", buf);
        free(buf);
        omp_str_free(item);
    }
}
```

#### Bare Clause Accessors

Some clauses have no arguments (e.g., `nowait`).

##### omp_clause_is_bare

Check if a clause is bare (no arguments).

```c
OmpStatus omp_clause_is_bare(Handle handle, bool *out_is_bare);
```

##### omp_clause_bare_name

Get the bare clause name.

```c
OmpStatus omp_clause_bare_name(Handle handle, Handle *out_name);
```

**Returns:** String handle containing clause name (only if clause is bare)

#### omp_clause_free

Free a clause.

```c
OmpStatus omp_clause_free(Handle handle);
```

**Returns:** `OMP_SUCCESS` or `OMP_INVALID_HANDLE`

**Note:** Clauses are automatically freed when the parse result is freed.

## Error Handling

### Status Codes

All functions return `OmpStatus`. Always check the return value:

```c
OmpStatus status = omp_parse(input, lang, &result);
if (status != OMP_SUCCESS) {
    switch (status) {
        case OMP_NULL_POINTER:
            printf("Null pointer passed\n");
            break;
        case OMP_PARSE_ERROR:
            printf("Parse failed\n");
            break;
        default:
            printf("Error: %d\n", status);
    }
    return 1;
}
```

### Helper Macros

The header provides convenience macros:

#### OMP_CHECK

Check status and return on error:

```c
#define OMP_CHECK(call) do { \
    OmpStatus _status = (call); \
    if (_status != OMP_SUCCESS) { \
        return _status; \
    } \
} while (0)

// Usage:
OmpStatus my_function() {
    Handle result;
    OMP_CHECK(omp_parse(input, lang, &result));
    // ... use result ...
    return OMP_SUCCESS;
}
```

#### OMP_CHECK_GOTO

Check status and goto cleanup label on error:

```c
#define OMP_CHECK_GOTO(call, label) do { \
    OmpStatus _status = (call); \
    if (_status != OMP_SUCCESS) { \
        goto label; \
    } \
} while (0)

// Usage:
OmpStatus my_function() {
    Handle result = INVALID_HANDLE;
    Handle *dirs = NULL;
    
    OMP_CHECK_GOTO(omp_parse(input, lang, &result), cleanup);
    OMP_CHECK_GOTO(omp_take_last_parse_result(&dirs, &count), cleanup);
    
    // ... use dirs ...
    
cleanup:
    if (dirs) free(dirs);
    if (OMP_IS_VALID(result)) omp_parse_result_free(result);
    return OMP_SUCCESS;
}
```

#### OMP_IS_VALID / OMP_IS_INVALID

Check handle validity:

```c
#define OMP_IS_VALID(handle) ((handle) != INVALID_HANDLE)
#define OMP_IS_INVALID(handle) ((handle) == INVALID_HANDLE)

// Usage:
if (OMP_IS_VALID(handle)) {
    // Use handle
}
```

## Memory Management

### Ownership Rules

1. **Parse Results**: You own parse results and must free them with `omp_parse_result_free()`
2. **Directive Arrays**: Arrays returned by `omp_take_last_parse_result()` are heap-allocated and must be freed with `free()`
3. **String Handles**: Strings returned by typed accessors must be freed with `omp_str_free()`
4. **Cursors**: Cursors must be freed with `omp_cursor_free()`
5. **Directives/Clauses**: Usually freed automatically when parse result is freed

### Memory Patterns

#### Pattern 1: Parse and Query

```c
Handle result;
omp_parse(input, lang, &result);

Handle *dirs;
uintptr_t count;
omp_take_last_parse_result(&dirs, &count);

// Use dirs[0..count-1]

free(dirs);                    // Free the array
omp_parse_result_free(result); // Frees all directives/clauses
```

#### Pattern 2: Cursor Iteration

```c
Handle cursor;
omp_directive_clauses_cursor(dir, &cursor);

bool done;
while (omp_cursor_is_done(cursor, &done) == OMP_SUCCESS && !done) {
    Handle clause;
    omp_cursor_current(cursor, &clause);
    // Use clause (don't free - owned by parse result)
    omp_cursor_next(cursor);
}

omp_cursor_free(cursor);  // Free cursor only
```

#### Pattern 3: String Extraction

```c
// Get string from typed accessor
Handle str_handle;
omp_clause_num_threads_value(clause, &str_handle);

// Copy to C buffer
uintptr_t len;
omp_str_len(str_handle, &len);
char *buffer = malloc(len + 1);
omp_str_copy_to_buffer(str_handle, buffer, len + 1, &len);

// Use buffer
printf("%s\n", buffer);

// Clean up
free(buffer);
omp_str_free(str_handle);  // Free the string handle
```

### Common Mistakes

❌ **Using handle after freeing:**
```c
omp_str_free(handle);
omp_str_len(handle, &len);  // ERROR: handle is invalid
```

❌ **Not freeing the directive array:**
```c
Handle *dirs;
omp_take_last_parse_result(&dirs, &count);
// ... use dirs ...
// Missing: free(dirs);
```

❌ **Freeing directives individually:**
```c
for (uintptr_t i = 0; i < count; i++) {
    omp_directive_free(dirs[i]);  // Usually not needed
}
omp_parse_result_free(result);  // This will fail - directives already freed
```

✅ **Correct pattern:**
```c
Handle *dirs;
omp_take_last_parse_result(&dirs, &count);
// ... use dirs (don't free individual directives) ...
free(dirs);                    // Free array only
omp_parse_result_free(result); // This frees all directives
```

## Thread Safety

### Global Registry

All handles are stored in a global registry protected by a mutex (`parking_lot::Mutex`). This provides:

- **Thread-safe allocation:** Multiple threads can create handles concurrently
- **Thread-safe access:** Handles can be used from any thread
- **Thread-safe cleanup:** Multiple threads can free handles concurrently

### Concurrency Patterns

#### Pattern 1: Concurrent Parsing

```c
// Thread 1
Handle result1;
omp_parse("#pragma omp parallel", OMP_LANG_C, &result1);
// ... use result1 ...
omp_parse_result_free(result1);

// Thread 2 (concurrent)
Handle result2;
omp_parse("#pragma omp for", OMP_LANG_C, &result2);
// ... use result2 ...
omp_parse_result_free(result2);
```

#### Pattern 2: Handle Sharing

```c
// Thread 1: Create and share handle
Handle global_result;
omp_parse(input, lang, &global_result);
// Pass global_result to Thread 2

// Thread 2: Query shared handle
DirectiveKind kind;
omp_directive_kind(global_result, &kind);  // Safe

// Thread 1: Clean up when all threads done
omp_parse_result_free(global_result);
```

### Thread Safety Guarantees

✅ **Safe:**
- Creating handles from multiple threads
- Querying handles from multiple threads
- Freeing different handles from multiple threads

⚠️ **Unsafe:**
- Modifying the same string handle from multiple threads
- Freeing the same handle from multiple threads

## Examples

See the `examples/c/` directory for complete examples:

1. **basic_parse.c** - Basic parsing and querying
2. **clause_inspection.c** - Detailed clause inspection
3. **string_builder.c** - String building API
4. **error_handling.c** - Error handling patterns

## Building and Linking

### Build the Rust Library

```bash
cargo build --lib
# Or for release:
cargo build --lib --release
```

This produces:
- Debug: `target/debug/libroup.so` (or `.dylib` on macOS, `.dll` on Windows)
- Release: `target/release/libroup.so`

### Compile C Code

```bash
gcc -o myprogram myprogram.c \
    -I./include \
    -L./target/debug \
    -lroup \
    -lpthread -ldl -lm
```

**Flags:**
- `-I./include`: Include path for `roup.h`
- `-L./target/debug`: Library search path
- `-lroup`: Link Roup library
- `-lpthread -ldl -lm`: Required system libraries

### Run with Library Path

```bash
# Linux
LD_LIBRARY_PATH=./target/debug ./myprogram

# macOS
DYLD_LIBRARY_PATH=./target/debug ./myprogram

# Or install the library system-wide
sudo cp target/release/libroup.so /usr/local/lib/
sudo ldconfig
```

### CMake Example

```cmake
cmake_minimum_required(VERSION 3.10)
project(MyProject C)

# Find the Roup library
find_library(ROUP_LIB roup HINTS ${CMAKE_SOURCE_DIR}/target/debug)

# Add executable
add_executable(myprogram myprogram.c)

# Include directories
target_include_directories(myprogram PRIVATE ${CMAKE_SOURCE_DIR}/include)

# Link libraries
target_link_libraries(myprogram ${ROUP_LIB} pthread dl m)
```

### Pkg-config Support

Create `roup.pc`:

```
prefix=/usr/local
libdir=${prefix}/lib
includedir=${prefix}/include

Name: roup
Description: OpenMP Parser Library
Version: 0.1.0
Libs: -L${libdir} -lroup -lpthread -ldl -lm
Cflags: -I${includedir}
```

Use with pkg-config:

```bash
gcc myprogram.c $(pkg-config --cflags --libs roup) -o myprogram
```

## Troubleshooting

### Library Not Found at Runtime

**Error:** `error while loading shared libraries: libroup.so: cannot open shared object file`

**Solution:** Set library path or install system-wide:
```bash
export LD_LIBRARY_PATH=./target/debug:$LD_LIBRARY_PATH
# Or:
sudo cp target/release/libroup.so /usr/local/lib/
sudo ldconfig
```

### Segmentation Fault

Common causes:
1. Using invalid handle (check with `OMP_IS_VALID`)
2. Not checking return status before using output parameters
3. Using handle after freeing it
4. Freeing same handle twice

**Debug:** Enable Rust backtraces:
```bash
RUST_BACKTRACE=1 ./myprogram
```

### Memory Leaks

Use valgrind to detect leaks:
```bash
valgrind --leak-check=full --show-leak-kinds=all \
    LD_LIBRARY_PATH=./target/debug ./myprogram
```

Common leak sources:
- Not freeing parse results
- Not freeing string handles from typed accessors
- Not freeing cursors
- Not freeing directive arrays from `omp_take_last_parse_result()`

## Performance Tips

1. **Reuse strings:** Create once, clear and reuse instead of allocating new
2. **Batch operations:** Parse multiple directives in one call when possible
3. **Minimize string extraction:** Only call `omp_str_copy_to_buffer()` when needed
4. **Use cursors for iteration:** More efficient than indexed access
5. **Release build:** Use `cargo build --release` for production (10x faster)

## Version Compatibility

This documentation is for Roup v0.1.0. The API may change in future versions.

## License

Same as the parent Roup project: MIT or Apache-2.0.
