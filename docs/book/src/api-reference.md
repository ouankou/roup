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

> **Important:** These values match the exact FFI enum discriminants in `include/roup.h`. Use these when decoding return values from `roup_directive_kind()`, `roup_clause_type()`, etc.

#### Directive Kinds (DirectiveKind enum)

| Value | Directive | Value | Directive |
|-------|-----------|-------|-----------|
| 0 | `parallel` | 37 | `teams_distribute` |
| 1 | `for` | 38 | `teams_distribute_simd` |
| 2 | `sections` | 39 | `target_teams_distribute` |
| 3 | `section` | 40 | `target_teams_distribute_simd` |
| 4 | `single` | 41 | `distribute_parallel_for` |
| 5 | `task` | 42 | `distribute_parallel_for_simd` |
| 6 | `master` | 43 | `distribute_simd` |
| 7 | `critical` | 44 | `parallel_for_simd` |
| 8 | `barrier` | 45 | `taskloop_simd` |
| 9 | `taskwait` | 46 | `master_taskloop_simd` |
| 10 | `taskgroup` | 47 | `parallel_master_taskloop_simd` |
| 11 | `atomic` | 48 | `target_parallel_for_simd` |
| 12 | `flush` | 49 | `teams_distribute_parallel_for` |
| 13 | `ordered` | 50 | `teams_distribute_parallel_for_simd` |
| 14 | `simd` | 51 | `target_teams_distribute_parallel_for` |
| 15 | `target` | 52 | `target_teams_distribute_parallel_for_simd` |
| 16 | `target_data` | 53 | `loop` |
| 17 | `target_enter_data` | 54 | `parallel_loop` |
| 18 | `target_exit_data` | 55 | `teams_loop` |
| 19 | `target_update` | 56 | `target_loop` |
| 20 | `declare_target` | 57 | `target_parallel_loop` |
| 21 | `teams` | 58 | `target_teams_loop` |
| 22 | `distribute` | 59 | `masked` |
| 23 | `declare_simd` | 60 | `scope` |
| 24 | `declare_reduction` | 61 | `metadirective` |
| 25 | `taskloop` | 62 | `declare_variant` |
| 26 | `cancel` | 63 | `requires` |
| 27 | `cancellation_point` | 64 | `assume` |
| 28 | `parallel_for` | 65 | `nothing` |
| 29 | `parallel_sections` | 66 | `error` |
| 30 | `parallel_master` | 67 | `scan` |
| 31 | `master_taskloop` | 68 | `depobj` |
| 32 | `parallel_master_taskloop` | 69 | `tile` |
| 33 | `target_parallel` | 70 | `unroll` |
| 34 | `target_parallel_for` | 71 | `allocate` |
| 35 | `target_simd` | 72 | `threadprivate` |
| 36 | `target_teams` | 73 | `declare_mapper` |

#### Clause Kinds (ClauseType enum)

| Value | Clause | Value | Clause |
|-------|--------|-------|--------|
| 0 | `if` | 46 | `detach` |
| 1 | `num_threads` | 47 | `affinity` |
| 2 | `default` | 48 | `bind` |
| 3 | `private` | 49 | `filter` |
| 4 | `firstprivate` | 50 | `allocate` |
| 5 | `lastprivate` | 51 | `allocator` |
| 6 | `shared` | 52 | `uses_allocators` |
| 7 | `reduction` | 53 | `inclusive` |
| 8 | `copyin` | 54 | `exclusive` |
| 9 | `copyprivate` | 55 | `when` |
| 10 | `schedule` | 56 | `match` |
| 11 | `ordered` | 57 | `at` |
| 12 | `nowait` | 58 | `severity` |
| 13 | `collapse` | 59 | `message` |
| 14 | `untied` | 60 | `novariants` |
| 15 | `final` | 61 | `nocontext` |
| 16 | `mergeable` | 62 | `adjust_args` |
| 17 | `depend` | 63 | `append_args` |
| 18 | `priority` | 64 | `full` |
| 19 | `grainsize` | 65 | `partial` |
| 20 | `num_tasks` | 66 | `sizes` |
| 21 | `nogroup` | 67 | `holds` |
| 22 | `threads` | 68 | `absent` |
| 23 | `simd` | 69 | `contains` |
| 24 | `aligned` | 70 | `atomic_default_mem_order` |
| 25 | `linear` | 71 | `dynamic_allocators` |
| 26 | `uniform` | 72 | `reverse_offload` |
| 27 | `inbranch` | 73 | `unified_address` |
| 28 | `notinbranch` | 74 | `unified_shared_memory` |
| 29 | `safelen` | 75 | `compare` |
| 30 | `simdlen` | 76 | `fail` |
| 31 | `device` | 77 | `seq_cst` |
| 32 | `map` | 78 | `acq_rel` |
| 33 | `num_teams` | 79 | `release` |
| 34 | `thread_limit` | 80 | `acquire` |
| 35 | `dist_schedule` | 81 | `relaxed` |
| 36 | `proc_bind` | 82 | `hint` |
| 37 | `defaultmap` | 83 | `update` |
| 38 | `to` | 84 | `capture` |
| 39 | `from` | 85 | `read` |
| 40 | `use_device_ptr` | 86 | `write` |
| 41 | `is_device_ptr` | 87 | `init` |
| 42 | `link` | 88 | `use_device_addr` |
| 43 | `nontemporal` | 89 | `has_device_addr` |
| 44 | `order` | 90 | `enter` |
| 45 | `destroy` | 91 | `doacross` |

#### Schedule Kinds (ScheduleKind enum)

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
