# Roup C Examples

This directory contains example C programs demonstrating the Roup OpenMP parser C FFI.

## Overview

The Roup library provides a 100% safe Rust-based OpenMP parser with a comprehensive C FFI. These examples show how to use the API from C code.

## Examples

### 1. basic_parse.c
**Basic parsing and querying**
- Parse OpenMP directives from strings
- Query directive properties (kind, location, clause count)
- Iterate through clauses using cursors
- Basic resource management

**Topics covered:**
- `omp_parse()` - Parse directives
- `omp_take_last_parse_result()` - Extract results
- `omp_directive_kind()`, `omp_directive_clause_count()` - Query directives
- `omp_directive_clauses_cursor()` - Create iterator
- `omp_cursor_next()`, `omp_cursor_current()` - Cursor operations
- `omp_parse_result_free()` - Cleanup

### 2. clause_inspection.c
**Detailed clause inspection**
- Query clause types
- Use typed accessors (num_threads, schedule, reduction, default)
- Handle list clauses (private, shared, etc.)
- Extract string values from clauses

**Topics covered:**
- `omp_clause_type()` - Get clause type
- `omp_clause_num_threads_value()` - num_threads accessor
- `omp_clause_schedule_kind()`, `omp_clause_schedule_chunk_size()` - schedule accessors
- `omp_clause_reduction_operator()`, `omp_clause_reduction_identifier()` - reduction accessors
- `omp_clause_default_kind()` - default accessor
- `omp_clause_item_count()`, `omp_clause_item_at()` - list clause accessors
- `omp_str_len()`, `omp_str_copy_to_buffer()` - String extraction
- `omp_str_free()` - String cleanup

### 3. string_builder.c
**String building API**
- Create strings from scratch
- Build strings incrementally
- String operations (length, capacity, clear)
- Convert between C strings and handles
- Byte-level manipulation

**Topics covered:**
- `omp_str_new()` - Create empty string
- `omp_str_from_cstr()` - Create from C string
- `omp_str_push_cstr()` - Append C string
- `omp_str_push_bytes()` - Append raw bytes
- `omp_str_len()`, `omp_str_capacity()` - Query string properties
- `omp_str_is_empty()` - Check if empty
- `omp_str_clear()` - Clear content
- `omp_str_copy_to_buffer()` - Extract to C buffer
- `omp_str_free()` - Cleanup

### 4. error_handling.c
**Error handling and resource cleanup**
- Check return status codes
- Validate handles
- Handle parse errors
- Proper cleanup on error paths
- Use helper macros

**Topics covered:**
- `OmpStatus` enum - All error codes
- `INVALID_HANDLE` constant - Invalid handle detection
- `OMP_IS_VALID()`, `OMP_IS_INVALID()` macros - Handle validation
- `OMP_CHECK()`, `OMP_CHECK_GOTO()` macros - Error checking
- Error types: `OMP_INVALID_HANDLE`, `OMP_NULL_POINTER`, `OMP_OUT_OF_BOUNDS`, `OMP_PARSE_ERROR`, `OMP_TYPE_MISMATCH`
- goto cleanup pattern for error handling

## Building

### Prerequisites
- GCC or compatible C compiler
- Rust toolchain (to build the Roup library)
- Linux/Unix environment

### Build All Examples

```bash
make all
```

This will:
1. Build the Rust library (`libroup.so`) if needed
2. Compile all C examples

### Build Individual Examples

```bash
make basic_parse
make clause_inspection
make string_builder
make error_handling
```

### Manual Build

```bash
# First, build the Rust library
cd ../..
cargo build --lib

# Then build a C example
cd examples/c
gcc -Wall -Wextra -I../../include -o basic_parse basic_parse.c \
    -L../../target/debug -lroup -lpthread -ldl -lm
```

## Running

### Run All Examples

```bash
make run-all
```

### Run Individual Examples

```bash
LD_LIBRARY_PATH=../../target/debug ./basic_parse
LD_LIBRARY_PATH=../../target/debug ./clause_inspection
LD_LIBRARY_PATH=../../target/debug ./string_builder
LD_LIBRARY_PATH=../../target/debug ./error_handling
```

**Note:** The `LD_LIBRARY_PATH` is required to find `libroup.so` at runtime.

## API Reference

See `include/roup.h` for the complete C API documentation.

### Key Concepts

**Handles:**
- All resources are represented as opaque `Handle` (uint64_t) values
- `INVALID_HANDLE` (0) indicates an invalid or null resource
- Always check if a handle is valid before using it

**Status Codes:**
- All functions return `OmpStatus`
- `OMP_SUCCESS` (0) indicates success
- Always check the return value before using output parameters

**Memory Management:**
- Strings must be freed with `omp_str_free()`
- Parse results must be freed with `omp_parse_result_free()`
- Clauses must be freed with `omp_clause_free()` (or freed automatically with parse result)
- Cursors must be freed with `omp_cursor_free()`
- The array returned by `omp_take_last_parse_result()` must be freed with `free()`

**Thread Safety:**
- The global registry is thread-safe (protected by mutex)
- Handles are valid across threads
- Multiple threads can parse and query concurrently

## Example Output

### basic_parse
```
=== Roup OpenMP Parser - Basic Example ===

Example 1: Simple parallel directive
Input: "#pragma omp parallel"

Parsed 1 directive(s)

Directive: parallel
  Location: line 1, column 0
  Language: C
  Clauses: 0
...
```

### clause_inspection
```
=== Clause Inspection Example ===

Example 1: num_threads clause
Input: "#pragma omp parallel num_threads(omp_get_max_threads())" 

  Clause: num_threads
    Value: omp_get_max_threads()
...
```

## Clean Up

```bash
make clean
```

This removes all compiled executables.

## Troubleshooting

### Library Not Found
If you see `error while loading shared libraries: libroup.so`:
- Make sure you've built the Rust library: `cargo build --lib`
- Set `LD_LIBRARY_PATH`: `export LD_LIBRARY_PATH=../../target/debug`

### Compilation Errors
- Check that `include/roup.h` exists
- Make sure the Rust library is built first
- Verify GCC version supports C99 or later

### Segmentation Fault
- Always check return status before using output parameters
- Validate handles with `OMP_IS_VALID()` before use
- Don't use handles after freeing them
- Don't free the same handle twice

## Further Reading

- **C API Header:** `include/roup.h` - Complete API documentation
- **Rust Documentation:** Run `cargo doc --open` for Rust API docs
- **OpenMP Support:** See `docs/OPENMP_SUPPORT.md` for supported directives/clauses

## License

Same as the parent Roup project (MIT/Apache-2.0).
