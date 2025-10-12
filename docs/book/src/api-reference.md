# API Reference

ROUP provides comprehensive APIs for Rust, C, and C++.

---

## Rust API Documentation

The complete Rust API documentation is auto-generated from the source code using `rustdoc`.

**[→ View Rust API Documentation](./api/roup/index.html)**

### Key Modules

- **`roup::parser`** - Main parsing functions
  - `parse()` - Parse OpenMP directive from string
  - `parse_with_config()` - Parse with custom configuration
  
- **`roup::ir::directive`** - Directive types and structures
  - `DirectiveIR` - Main directive structure
  - `DirectiveKind` - Enum of directive types
  
- **`roup::ir::clause`** - Clause types and data
  - `Clause` - Clause structure
  - `ClauseKind` - Enum of clause types
  - `ScheduleKind`, `ReductionOperator`, etc.

- **`roup::ir::types`** - Common types
  - `Language` - Source language (C, C++, Fortran)
  - `SourceLocation` - Position in source code

### Quick Links

- [Parse Functions](./api/roup/parser/index.html)
- [Directive Types](./api/roup/ir/directive/index.html)
- [Clause Types](./api/roup/ir/clause/index.html)

---

## C API Reference

ROUP exports 16 C functions for FFI integration, providing a minimal C API with unsafe pointer operations only at the FFI boundary. All functions use direct C pointers (`*mut OmpDirective`, `*mut OmpClause`) following a standard malloc/free pattern.

**Source**: `src/c_api.rs` (~60 lines of unsafe code at FFI boundary, ~0.9% of file)

### Lifecycle Functions

```c
// Parse OpenMP directive from C string
OmpDirective* roup_parse(const char* input);

// Free directive (required after parsing)
void roup_directive_free(OmpDirective* directive);

// Free clause (usually not needed - owned by directive)
void roup_clause_free(OmpClause* clause);
```

### Directive Query Functions

```c
// Get directive kind (0=parallel, 1=for, etc.)
int32_t roup_directive_kind(const OmpDirective* directive);

// Get number of clauses
int32_t roup_directive_clause_count(const OmpDirective* directive);

// Create clause iterator
OmpClauseIterator* roup_directive_clauses_iter(const OmpDirective* directive);
```

### Iterator Functions

```c
// Get next clause from iterator
// Returns 1 if clause available, 0 if done
int32_t roup_clause_iterator_next(OmpClauseIterator* iter, OmpClause** out);

// Free iterator
void roup_clause_iterator_free(OmpClauseIterator* iter);
```

### Clause Query Functions

```c
// Get clause kind (0=num_threads, 2=private, etc.)
int32_t roup_clause_kind(const OmpClause* clause);

// Get schedule kind (0=static, 1=dynamic, etc.)
int32_t roup_clause_schedule_kind(const OmpClause* clause);

// Get reduction operator (0=+, 1=-, 2=*, etc.)
int32_t roup_clause_reduction_operator(const OmpClause* clause);

// Get default data sharing (0=shared, 1=none)
int32_t roup_clause_default_data_sharing(const OmpClause* clause);
```

### Variable List Functions

```c
// Get variable list from clause (e.g., private(x, y, z))
OmpStringList* roup_clause_variables(const OmpClause* clause);

// Get length of string list
int32_t roup_string_list_len(const OmpStringList* list);

// Get string at index
const char* roup_string_list_get(const OmpStringList* list, int32_t index);

// Free string list
void roup_string_list_free(OmpStringList* list);
```

### Mapping Tables

> **Important:** These values are defined in `src/c_api.rs`. The C API uses a **simple subset** of OpenMP clauses with straightforward integer mapping.

#### Directive Kinds

The C API provides `roup_directive_kind()` which returns an integer representing the directive type. The specific mapping depends on the parser's internal directive registry.

**Common directive types** (from parser):
- Parallel constructs: `parallel`, `parallel for`, `parallel sections`
- Work-sharing: `for`, `sections`, `single`, `workshare`
- Tasking: `task`, `taskloop`, `taskgroup`, `taskwait`
- Device: `target`, `target data`, `target update`, `teams`
- Synchronization: `barrier`, `critical`, `atomic`, `ordered`
- SIMD: `simd`, `declare simd`, `distribute`
- Advanced: `metadirective`, `declare variant`, `assume`

For a complete list of all 120+ supported directives with version compatibility, see the [OpenMP Support Matrix](./openmp-support.md).

#### Clause Kinds (Integer Discriminants)

The C API supports 12 common clause types with simple integer mapping:

| Value | Clause | Description | Example |
|-------|--------|-------------|---------|
| 0 | `num_threads` | Thread count | `num_threads(4)` |
| 1 | `if` | Conditional | `if(condition)` |
| 2 | `private` | Private variables | `private(x, y)` |
| 3 | `shared` | Shared variables | `shared(a, b)` |
| 4 | `firstprivate` | Private with init | `firstprivate(z)` |
| 5 | `lastprivate` | Private with final value | `lastprivate(result)` |
| 6 | `reduction` | Reduction operation | `reduction(+:sum)` |
| 7 | `schedule` | Loop scheduling | `schedule(static, 100)` |
| 8 | `collapse` | Loop nesting | `collapse(2)` |
| 9 | `ordered` | Ordered execution | `ordered` |
| 10 | `nowait` | Remove barrier | `nowait` |
| 11 | `default` | Default sharing | `default(shared)` |
| 999 | Unknown | Unrecognized clause | - |

**Note**: The C API intentionally supports a focused subset of clauses for simplicity. The Rust API supports all 92+ OpenMP 6.0 clauses.

#### Schedule Kinds

| Value | Schedule |
|-------|----------|
| 0 | `static` |
| 1 | `dynamic` |
| 2 | `guided` |
| 3 | `auto` |
| 4 | `runtime` |

#### Default Kinds (DefaultKind enum)

| Value | Default |
|-------|---------|
| 0 | `shared` |
| 1 | `none` |
| 2 | `private` |
| 3 | `firstprivate` |

#### Reduction Operators (ReductionOperator enum)

| Value | Operator |
|-------|----------|
| 0 | `+` (add) |
| 1 | `*` (multiply) |
| 2 | `-` (subtract) |
| 3 | `&` (bitwise AND) |
| 4 | `|` (bitwise OR) |
| 5 | `^` (bitwise XOR) |
| 6 | `&&` (logical AND) |
| 7 | `||` (logical OR) |
| 8 | `min` (minimum) |
| 9 | `max` (maximum) |
| 10 | `custom` (user-defined) |

---

## C++ RAII Wrappers

For modern C++17 applications, use the RAII wrappers provided in the [C++ Tutorial](./cpp-tutorial.md#step-2-create-raii-wrappers-modern-c).

**Key classes:**
- `roup::Directive` - Auto-frees directive on destruction
- `roup::ClauseIterator` - Auto-frees iterator on destruction
- `roup::StringList` - Auto-frees string list on destruction

**Example:**
```cpp
#include "roup_wrapper.hpp"

roup::Directive dir("#pragma omp parallel for num_threads(4)");
if (dir) {
    std::cout << "Kind: " << dir.kind() << std::endl;
    std::cout << "Clauses: " << dir.clause_count() << std::endl;
}
// Automatic cleanup when dir goes out of scope
```

---

## Memory Management Rules

### Rust API
- **Automatic** - Rust's ownership system handles everything
- No manual `free()` needed

### C API
- **Manual** - Must call `_free()` functions
- **Directive:** Call `roup_directive_free()` when done
- **Iterator:** Call `roup_clause_iterator_free()` when done
- **String List:** Call `roup_string_list_free()` when done
- **Clauses:** Do NOT free - owned by directive

### C++ RAII API
- **Automatic** - RAII wrappers call `_free()` in destructors
- Exception-safe - cleanup happens even with exceptions

---

## Error Handling

### Rust
```rust,ignore
use roup::parser::openmp;

let parser = openmp::parser();
match parser.parse(input) {
    Ok((_, directive)) => { /* use directive */ },
    Err(e) => eprintln!("Parse error: {:?}", e),
}
```

### C
```c
OmpDirective* dir = roup_parse(input);
if (dir == NULL) {
    fprintf(stderr, "Parse failed\n");
    return 1;
}
// Use dir...
roup_directive_free(dir);
```

### C++
```cpp
roup::Directive dir(input);
if (!dir) {
    std::cerr << "Parse failed\n";
    return 1;
}
// Use dir...
```

---

## Thread Safety

- ✅ **Parsing is thread-safe** - Multiple threads can call `parse()` simultaneously
- ✅ **Read operations are thread-safe** - Query functions are read-only
- ⚠️ **Modification is not thread-safe** - Don't mutate same directive from multiple threads
- ⚠️ **Iterators are single-threaded** - One iterator per thread

---

## Performance Tips

1. **Reuse parsed directives** when possible
2. **Avoid reparsing** the same string repeatedly
3. **Use iterators** instead of random access
4. **Batch operations** to minimize FFI overhead (C/C++)
5. **Profile first** - parsing is usually not the bottleneck

---

## Further Reading

- [Rust API Documentation](./api/roup/index.html) - Complete rustdoc
- [C++ Tutorial](./cpp-tutorial.md) - Real-world C++ examples
- [GitHub Repository](https://github.com/ouankou/roup) - Source code and examples
- [Quick Start Guide](https://github.com/ouankou/roup/blob/main/docs/QUICK_START.md) - Get started in 5 minutes
