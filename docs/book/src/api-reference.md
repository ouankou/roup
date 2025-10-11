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

ROUP exports 18 C functions for FFI integration. All functions are documented in the rustdoc, but here's a quick reference:

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

#### Directive Kinds

| Value | Directive |
|-------|-----------|
| 0 | `parallel` |
| 1 | `for` |
| 2 | `sections` |
| 3 | `single` |
| 4 | `task` |
| 5 | `master` |
| 6 | `critical` |
| 7 | `barrier` |
| 8 | `taskwait` |
| 9 | `taskgroup` |
| 10 | `atomic` |
| 11 | `flush` |
| 12 | `ordered` |
| 13 | `target` |
| 14 | `teams` |
| 15 | `distribute` |
| 16 | `metadirective` |

#### Clause Kinds

| Value | Clause |
|-------|--------|
| 0 | `num_threads` |
| 1 | `if` |
| 2 | `private` |
| 3 | `shared` |
| 4 | `firstprivate` |
| 5 | `lastprivate` |
| 6 | `reduction` |
| 7 | `schedule` |
| 8 | `collapse` |
| 9 | `ordered` |
| 10 | `nowait` |
| 11 | `default` |

#### Schedule Kinds

| Value | Schedule |
|-------|----------|
| 0 | `static` |
| 1 | `dynamic` |
| 2 | `guided` |
| 3 | `auto` |
| 4 | `runtime` |

#### Reduction Operators

| Value | Operator |
|-------|----------|
| 0 | `+` (sum) |
| 1 | `-` (subtraction) |
| 2 | `*` (product) |
| 3 | `&` (bitwise AND) |
| 4 | `|` (bitwise OR) |
| 5 | `^` (bitwise XOR) |
| 6 | `&&` (logical AND) |
| 7 | `||` (logical OR) |
| 8 | `min` (minimum) |
| 9 | `max` (maximum) |

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
```rust
match parse(input) {
    Ok(directive) => { /* use directive */ },
    Err(e) => eprintln!("Parse error: {}", e),
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
