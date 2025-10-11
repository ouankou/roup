# C Tutorial

This tutorial demonstrates how to use ROUP's **minimal unsafe pointer-based C API** for parsing OpenMP directives. The API uses direct C pointers following standard malloc/free patterns familiar to C programmers.

> **API Design**: Direct pointers (`*mut OmpDirective`, `*mut OmpClause`) with manual memory management. No global state, no handles.
>
> **Source**: `src/c_api.rs` - 16 FFI functions, ~60 lines of unsafe code

---

## Prerequisites

Before starting, ensure you have:
- C compiler (GCC, Clang, or MSVC)
- ROUP library compiled (see [Building Guide](./building.md))
- Basic understanding of malloc/free patterns

**Example code**: See `examples/c/tutorial_basic.c` for a complete working example (433 lines).

---

## Step 1: Setup and Compilation

### Project Structure

```
my-project/
├── src/
│   └── main.c
├── include/
│   └── roup_ffi.h      # Forward declarations
└── libroup.a            # Built from cargo build
```

### Forward Declarations

Create `include/roup_ffi.h` with the C API declarations:

```c
#ifndef ROUP_FFI_H
#define ROUP_FFI_H

#include <stdint.h>

// Opaque types (defined in Rust)
typedef struct OmpDirective OmpDirective;
typedef struct OmpClause OmpClause;
typedef struct OmpClauseIterator OmpClauseIterator;
typedef struct OmpStringList OmpStringList;

// Lifecycle functions
extern OmpDirective* roup_parse(const char* input);
extern void roup_directive_free(OmpDirective* directive);
extern void roup_clause_free(OmpClause* clause);

// Directive queries
extern int32_t roup_directive_kind(const OmpDirective* directive);
extern int32_t roup_directive_clause_count(const OmpDirective* directive);
extern OmpClauseIterator* roup_directive_clauses_iter(const OmpDirective* directive);

// Iterator functions
extern int32_t roup_clause_iterator_next(OmpClauseIterator* iter, OmpClause** out);
extern void roup_clause_iterator_free(OmpClauseIterator* iter);

// Clause queries
extern int32_t roup_clause_kind(const OmpClause* clause);
extern int32_t roup_clause_schedule_kind(const OmpClause* clause);
extern int32_t roup_clause_reduction_operator(const OmpClause* clause);
extern int32_t roup_clause_default_data_sharing(const OmpClause* clause);

// Variable lists
extern OmpStringList* roup_clause_variables(const OmpClause* clause);
extern int32_t roup_string_list_len(const OmpStringList* list);
extern const char* roup_string_list_get(const OmpStringList* list, int32_t index);
extern void roup_string_list_free(OmpStringList* list);

#endif // ROUP_FFI_H
```

### Compilation

**Option 1: Using GCC/Clang**
```bash
# Build ROUP library
cargo build --release

# Compile C program
gcc -o my_app src/main.c \
    -I include \
    -L target/release \
    -lroup \
    -lpthread -ldl -lm
```

**Option 2: Using CMake**
```cmake
cmake_minimum_required(VERSION 3.10)
project(roup_example C)

add_executable(my_app src/main.c)
target_include_directories(my_app PRIVATE include)
target_link_libraries(my_app ${CMAKE_SOURCE_DIR}/target/release/libroup.a pthread dl m)
```

---

## Step 2: Parse a Simple Directive

Let's start with the most basic operation: parsing a simple directive.

```c
#include <stdio.h>
#include "roup_ffi.h"

int main(void) {
    const char* input = "#pragma omp parallel";
    
    // Parse the directive
    OmpDirective* dir = roup_parse(input);
    
    // Check for errors (NULL = parse failed)
    if (!dir) {
        fprintf(stderr, "Parse failed!\n");
        return 1;
    }
    
    printf("✅ Parse succeeded!\n");
    
    // IMPORTANT: Free the directive
    roup_directive_free(dir);
    
    return 0;
}
```

**Key Points:**
- `roup_parse()` returns a pointer or `NULL` on error
- Always check for `NULL` before using the directive
- **Always call `roup_directive_free()`** to prevent memory leaks

---

## Step 3: Query Directive Properties

After parsing, you can query the directive's properties:

```c
#include <stdio.h>
#include "roup_ffi.h"

int main(void) {
    const char* input = "#pragma omp parallel for num_threads(4)";
    
    OmpDirective* dir = roup_parse(input);
    if (!dir) {
        return 1;
    }
    
    // Query directive properties
    int32_t kind = roup_directive_kind(dir);
    int32_t clause_count = roup_directive_clause_count(dir);
    
    printf("Directive kind: %d\n", kind);
    printf("Clause count: %d\n", clause_count);
    
    roup_directive_free(dir);
    return 0;
}
```

**Output:**
```
Directive kind: 28
Clause count: 1
```

> **Note**: Directive kind is an integer from the parser's internal registry. For practical use, you typically care more about the clauses than the exact directive kind. The kind value comes from the order in which directives were registered in `src/parser/openmp.rs` - these internal IDs are not part of the stable API and may change between versions.

---

## Step 4: Iterate Through Clauses

To access individual clauses, use the iterator pattern:

```c
#include <stdio.h>
#include "roup_ffi.h"

const char* clause_name(int32_t kind) {
    switch(kind) {
        case 0: return "num_threads";
        case 1: return "if";
        case 2: return "private";
        case 3: return "shared";
        case 6: return "reduction";
        case 7: return "schedule";
        case 10: return "nowait";
        default: return "unknown";
    }
}

int main(void) {
    const char* input = "#pragma omp parallel num_threads(8) default(shared) nowait";
    
    OmpDirective* dir = roup_parse(input);
    if (!dir) return 1;
    
    // Create iterator
    OmpClauseIterator* iter = roup_directive_clauses_iter(dir);
    if (!iter) {
        roup_directive_free(dir);
        return 1;
    }
    
    // Iterate through clauses
    printf("Clauses:\n");
    OmpClause* clause;
    while (roup_clause_iterator_next(iter, &clause)) {
        int32_t kind = roup_clause_kind(clause);
        printf("  - %s (kind=%d)\n", clause_name(kind), kind);
    }
    
    // Cleanup
    roup_clause_iterator_free(iter);
    roup_directive_free(dir);
    
    return 0;
}
```

**Output:**
```
Clauses:
  - num_threads (kind=0)
  - default (kind=11)
  - nowait (kind=10)
```

**Key Points:**
- `roup_clause_iterator_next()` returns `1` if clause available, `0` when done
- Write the clause pointer to `out` parameter
- Always free the iterator with `roup_clause_iterator_free()`

---

## Step 5: Query Clause-Specific Data

Different clause types have different data. Use type-specific query functions:

### Schedule Clause

```c
OmpClause* clause = /* ... get clause ... */;
if (roup_clause_kind(clause) == 7) {  // SCHEDULE
    int32_t sched = roup_clause_schedule_kind(clause);
    const char* names[] = {"static", "dynamic", "guided", "auto", "runtime"};
    printf("Schedule: %s\n", names[sched]);
}
```

### Reduction Clause

```c
if (roup_clause_kind(clause) == 6) {  // REDUCTION
    int32_t op = roup_clause_reduction_operator(clause);
    const char* ops[] = {"+", "-", "*", "&", "|", "^", "&&", "||", "min", "max"};
    printf("Reduction operator: %s\n", ops[op]);
}
```

### Default Clause

```c
if (roup_clause_kind(clause) == 11) {  // DEFAULT
    int32_t def = roup_clause_default_data_sharing(clause);
    printf("Default: %s\n", def == 0 ? "shared" : "none");
}
```

### Complete Example

```c
#include <stdio.h>
#include "roup_ffi.h"

int main(void) {
    const char* input = "#pragma omp parallel for schedule(static, 10) reduction(+:sum)";
    
    OmpDirective* dir = roup_parse(input);
    if (!dir) return 1;
    
    OmpClauseIterator* iter = roup_directive_clauses_iter(dir);
    if (!iter) {
        roup_directive_free(dir);
        return 1;
    }
    
    OmpClause* clause;
    while (roup_clause_iterator_next(iter, &clause)) {
        int32_t kind = roup_clause_kind(clause);
        
        if (kind == 7) {  // SCHEDULE
            int32_t sched = roup_clause_schedule_kind(clause);
            const char* names[] = {"static", "dynamic", "guided", "auto", "runtime"};
            printf("Schedule: %s\n", names[sched]);
        }
        else if (kind == 6) {  // REDUCTION
            int32_t op = roup_clause_reduction_operator(clause);
            const char* ops[] = {"+", "-", "*", "&", "|", "^", "&&", "||", "min", "max"};
            printf("Reduction: %s\n", ops[op]);
        }
    }
    
    roup_clause_iterator_free(iter);
    roup_directive_free(dir);
    
    return 0;
}
```

**Output:**
```
Schedule: static
Reduction: +
```

---

## Step 6: Access Variable Lists

Clauses like `private(x, y, z)` contain lists of variables:

```c
#include <stdio.h>
#include "roup_ffi.h"

int main(void) {
    const char* input = "#pragma omp parallel private(x, y, z) shared(a, b)";
    
    OmpDirective* dir = roup_parse(input);
    if (!dir) return 1;
    
    OmpClauseIterator* iter = roup_directive_clauses_iter(dir);
    if (!iter) {
        roup_directive_free(dir);
        return 1;
    }
    
    OmpClause* clause;
    while (roup_clause_iterator_next(iter, &clause)) {
        int32_t kind = roup_clause_kind(clause);
        
        // Get variable list
        OmpStringList* vars = roup_clause_variables(clause);
        if (vars) {
            int32_t len = roup_string_list_len(vars);
            
            const char* kind_name = (kind == 2) ? "private" : "shared";
            printf("%s variables: ", kind_name);
            
            for (int32_t i = 0; i < len; i++) {
                const char* var = roup_string_list_get(vars, i);
                printf("%s%s", var, (i < len - 1) ? ", " : "");
            }
            printf("\n");
            
            roup_string_list_free(vars);
        }
    }
    
    roup_clause_iterator_free(iter);
    roup_directive_free(dir);
    
    return 0;
}
```

**Output:**
```
private variables: x, y, z
shared variables: a, b
```

**Key Points:**
- `roup_clause_variables()` returns a `OmpStringList*` or `NULL`
- Use `roup_string_list_len()` to get the count
- Use `roup_string_list_get(list, index)` to access individual strings
- **Always call `roup_string_list_free()`** when done

---

## Step 7: Error Handling

Robust error handling is crucial. The API returns `NULL` on failure:

```c
#include <stdio.h>
#include "roup_ffi.h"

int main(void) {
    // Test 1: Invalid syntax
    const char* invalid = "#pragma omp INVALID_DIRECTIVE";
    OmpDirective* dir1 = roup_parse(invalid);
    if (!dir1) {
        printf("✓ Invalid syntax correctly rejected\n");
    }
    
    // Test 2: NULL input
    OmpDirective* dir2 = roup_parse(NULL);
    if (!dir2) {
        printf("✓ NULL input correctly rejected\n");
    }
    
    // Test 3: Empty string
    OmpDirective* dir3 = roup_parse("");
    if (!dir3) {
        printf("✓ Empty string correctly rejected\n");
    }
    
    // Test 4: Querying NULL
    int32_t kind = roup_directive_kind(NULL);
    printf("roup_directive_kind(NULL) = %d\n", kind);  // Returns -1
    
    return 0;
}
```

**Error Handling Guidelines:**
1. Always check `roup_parse()` return value for `NULL`
2. Check `roup_directive_clauses_iter()` for `NULL`
3. Query functions return `-1` or safe defaults for `NULL` inputs
4. Free resources even in error paths (if allocated)

---

## Step 8: Complete Example

Here's a complete program that demonstrates all concepts:

```c
#include <stdio.h>
#include <stdlib.h>
#include "roup_ffi.h"

void print_clause_details(OmpClause* clause) {
    int32_t kind = roup_clause_kind(clause);
    
    switch(kind) {
        case 0:
            printf("  - num_threads\n");
            break;
        case 2: {
            printf("  - private(");
            OmpStringList* vars = roup_clause_variables(clause);
            if (vars) {
                int32_t len = roup_string_list_len(vars);
                for (int32_t i = 0; i < len; i++) {
                    printf("%s%s", roup_string_list_get(vars, i), 
                           (i < len - 1) ? ", " : "");
                }
                roup_string_list_free(vars);
            }
            printf(")\n");
            break;
        }
        case 6: {
            printf("  - reduction(");
            int32_t op = roup_clause_reduction_operator(clause);
            const char* ops[] = {"+", "-", "*", "&", "|", "^", "&&", "||", "min", "max"};
            if (op >= 0 && op < 10) {
                printf("%s", ops[op]);
            }
            printf(":...)\n");
            break;
        }
        case 7: {
            printf("  - schedule(");
            int32_t sched = roup_clause_schedule_kind(clause);
            const char* names[] = {"static", "dynamic", "guided", "auto", "runtime"};
            if (sched >= 0 && sched < 5) {
                printf("%s", names[sched]);
            }
            printf(")\n");
            break;
        }
        case 10:
            printf("  - nowait\n");
            break;
        case 11:
            printf("  - default(%s)\n", 
                   roup_clause_default_data_sharing(clause) == 0 ? "shared" : "none");
            break;
        default:
            printf("  - unknown (kind=%d)\n", kind);
            break;
    }
}

int main(void) {
    const char* test_cases[] = {
        "#pragma omp parallel",
        "#pragma omp parallel for num_threads(4) private(i, j)",
        "#pragma omp parallel for schedule(static, 100) reduction(+:sum)",
        "#pragma omp task default(shared) nowait",
        NULL
    };
    
    for (int i = 0; test_cases[i] != NULL; i++) {
        printf("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
        printf("Input: %s\n", test_cases[i]);
        printf("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
        
        OmpDirective* dir = roup_parse(test_cases[i]);
        if (!dir) {
            printf("❌ Parse failed!\n");
            continue;
        }
        
        int32_t clause_count = roup_directive_clause_count(dir);
        printf("Clauses: %d\n", clause_count);
        
        if (clause_count > 0) {
            OmpClauseIterator* iter = roup_directive_clauses_iter(dir);
            if (iter) {
                OmpClause* clause;
                while (roup_clause_iterator_next(iter, &clause)) {
                    print_clause_details(clause);
                }
                roup_clause_iterator_free(iter);
            }
        }
        
        roup_directive_free(dir);
    }
    
    printf("\n✅ All tests completed!\n\n");
    return 0;
}
```

---

## Clause Kind Reference

The C API supports 12 common clause types:

| Kind | Clause | Has Variables | Has Specific Data |
|------|--------|---------------|-------------------|
| 0 | `num_threads` | No | Value (int) |
| 1 | `if` | No | Condition (string) |
| 2 | `private` | Yes | Variable list |
| 3 | `shared` | Yes | Variable list |
| 4 | `firstprivate` | Yes | Variable list |
| 5 | `lastprivate` | Yes | Variable list |
| 6 | `reduction` | Yes | Operator + variables |
| 7 | `schedule` | No | Schedule kind + chunk |
| 8 | `collapse` | No | Depth (int) |
| 9 | `ordered` | No | - |
| 10 | `nowait` | No | - |
| 11 | `default` | No | Sharing kind |
| 999 | Unknown | - | - |

**Schedule Kinds** (for clause kind 7):
- 0 = `static`
- 1 = `dynamic`
- 2 = `guided`
- 3 = `auto`
- 4 = `runtime`

**Reduction Operators** (for clause kind 6):
- 0 = `+`, 1 = `-`, 2 = `*`
- 3 = `&`, 4 = `|`, 5 = `^`
- 6 = `&&`, 7 = `||`
- 8 = `min`, 9 = `max`

**Default Kinds** (for clause kind 11):
- 0 = `shared`
- 1 = `none`
- 2 = `private`
- 3 = `firstprivate`

---

## Memory Management Checklist

✅ **DO:**
- Call `roup_directive_free()` for every successful `roup_parse()`
- Call `roup_clause_iterator_free()` for every `roup_directive_clauses_iter()`
- Call `roup_string_list_free()` for every `roup_clause_variables()`
- Check for `NULL` returns before using pointers

❌ **DON'T:**
- Call `roup_clause_free()` on clauses from iterators (owned by directive)
- Access freed pointers (use-after-free)
- Forget to free in error paths
- Assume parse always succeeds

---

## Performance Tips

1. **Reuse parsed directives** - Don't reparse the same string repeatedly
2. **Minimize FFI crossings** - Batch operations when possible
3. **Avoid unnecessary iteration** - If you only need clause count, don't iterate
4. **Use local variables** - Cache query results instead of calling repeatedly

**Example (inefficient):**
```c
// BAD: Queries kind multiple times
for (int i = 0; i < count; i++) {
    if (roup_clause_kind(clause) == 2) {
        process_private(roup_clause_kind(clause));
    }
}
```

**Example (efficient):**
```c
// GOOD: Cache the kind
int32_t kind = roup_clause_kind(clause);
if (kind == 2) {
    process_private(kind);
}
```

---

## Next Steps

Now that you understand the C API basics:

1. **Build the example** - Compile and run `examples/c/tutorial_basic.c`
2. **Explore directives** - See [OpenMP Support](./openmp-support.md) for all 120+ directives
3. **Advanced usage** - Check [API Reference](./api-reference.md) for complete function details
4. **C++ wrappers** - Read [C++ Tutorial](./cpp-tutorial.md) for RAII wrappers

**Full Example Code**: `examples/c/tutorial_basic.c` (433 lines with detailed comments)

---

## Troubleshooting

### Linker Errors

**Problem**: `undefined reference to roup_parse`

**Solution**: Link against the ROUP static library:
```bash
gcc ... -L target/release -lroup -lpthread -ldl -lm
```

### Parse Always Returns NULL

**Problem**: All parses fail, even valid input

**Solution**:
- Check that the library was built correctly (`cargo build --release`)
- Verify the input string is valid OpenMP syntax
- Ensure the string is null-terminated
- Try the examples first to verify the library works

### Memory Leaks

**Problem**: Valgrind reports memory leaks

**Solution**:
- Ensure every `roup_parse()` has a matching `roup_directive_free()`
- Ensure every `roup_directive_clauses_iter()` has a matching `roup_clause_iterator_free()`
- Ensure every `roup_clause_variables()` has a matching `roup_string_list_free()`

### Segmentation Fault

**Problem**: Program crashes with segfault

**Solution**:
- Check for `NULL` before dereferencing pointers
- Don't access freed pointers
- Don't call `roup_clause_free()` on clauses from iterators

---

**Questions?** Check the [FAQ](./faq.md) or open an issue on [GitHub](https://github.com/ouankou/roup/issues).
