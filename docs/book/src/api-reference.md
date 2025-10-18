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

## Translation API

ROUP supports **bidirectional translation** between C/C++ and Fortran OpenMP directive syntax. This is useful for automatically porting benchmarks and code between languages.

### Rust Translation API

The `roup::ir::translate` module provides high-level translation functions:

```rust,ignore
use roup::ir::translate::{translate_c_to_fortran, translate_fortran_to_c};

// C/C++ → Fortran
let fortran = translate_c_to_fortran("#pragma omp parallel for private(i)")?;
// Result: "!$omp parallel do private(i)"

// Fortran → C/C++
let c_code = translate_fortran_to_c("!$omp parallel do schedule(static, 4)")?;
// Result: "#pragma omp parallel for schedule(static, 4)"
```

**Advanced API** (returns IR for further processing):

```rust,ignore
use roup::ir::{translate::translate_c_to_fortran_ir, Language, ParserConfig};

let config = ParserConfig::with_parsing(Language::C);
let ir = translate_c_to_fortran_ir("#pragma omp parallel for", config)?;

// Can query or modify the IR
println!("Language: {:?}", ir.language());
println!("Fortran: {}", ir.to_string_for_language(Language::Fortran));
```

### Supported Translations

#### Loop Directive Mapping

The translation correctly maps **loop directive names** between languages:

| C/C++ | Fortran | Notes |
|-------|---------|-------|
| `for` | `do` | Basic loop construct |
| `for simd` | `do simd` | SIMD loop |
| `parallel for` | `parallel do` | Parallel loop |
| `parallel for simd` | `parallel do simd` | Parallel SIMD loop |
| `distribute parallel for` | `distribute parallel do` | Distributed parallel loop |
| `distribute parallel for simd` | `distribute parallel do simd` | All combined forms |
| `teams distribute parallel for` | `teams distribute parallel do` | Complex nesting |
| `target teams distribute parallel for simd` | `target teams distribute parallel do simd` | Maximum nesting |

**Complete list**: All 12 loop directive variants are supported in both directions.

#### Sentinel Translation

| C/C++ | Fortran |
|-------|---------|
| `#pragma omp` | `!$omp` (free-form) |

**Note**: Fixed-form Fortran sentinels (`c$omp`, `*$omp`) are supported for parsing but output uses free-form only.

#### Clause Preservation

✅ **All clauses are preserved as-is** — The OpenMP standard defines clauses identically across languages:

```text
Input (C):
#pragma omp parallel for private(i,j) schedule(static, 4) collapse(2)

Output (Fortran) - clauses unchanged:
!$omp parallel do private(i,j) schedule(static, 4) collapse(2)
```

### Limitations

The translation focuses on **directive syntax only**. The following are **intentionally not translated**:

| Feature | Status | Reason |
|---------|--------|--------|
| **Expressions in clauses** | ❌ Not translated | Language-specific syntax (e.g., `arr[i]` vs `arr(i)`) requires full expression parsing |
| **Variable names** | ✅ Preserved | Variable identifiers work across languages |
| **Surrounding code** | ❌ Not translated | Only directive lines are processed |
| **Fixed-form Fortran output** | ❌ Not supported | Free-form `!$omp` is the modern standard |
| **Comments/whitespace** | ❌ Not preserved | Output is normalized |

**Example limitation:**
```c
// Input (C)
#pragma omp parallel for reduction(+:sum) if(n > 1000)

// Output (Fortran) - expressions NOT translated
!$omp parallel do reduction(+:sum) if(n > 1000)
//                                   ^^^^^^^^^^^^
//                                   Still C syntax! Manual fix needed.
```

**Recommendation**: Use translation for directive structure, then manually adjust expressions for target language.

### C Translation API

The C API provides `roup_convert_language()` for language conversion:

```c
// Convert C to Fortran
const char* c_input = "#pragma omp parallel for private(x)";
char* fortran_output = roup_convert_language(
    c_input,
    ROUP_LANG_C,              // from language
    ROUP_LANG_FORTRAN_FREE    // to language
);

if (fortran_output != NULL) {
    printf("Fortran: %s\n", fortran_output);
    // Result: "!$omp parallel do private(x)"

    // IMPORTANT: Free the returned string
    roup_string_free(fortran_output);
}
```

**Language codes:**
```c
#define ROUP_LANG_C             0
#define ROUP_LANG_FORTRAN_FREE  2
#define ROUP_LANG_FORTRAN_FIXED 3
```

**Function signature:**
```c
// Convert directive between languages
// Returns NULL on error (must free with roup_string_free on success)
char* roup_convert_language(
    const char* input,
    int32_t from_language,
    int32_t to_language
);

// Free string returned by roup_convert_language
void roup_string_free(char* ptr);
```

**Error handling:** Returns `NULL` if:
- Input is NULL or empty
- Language code is invalid
- Parsing fails
- Conversion fails

### Translation Error Handling

**Rust API:**
```rust,ignore
use roup::ir::translate::{translate_c_to_fortran, TranslationError};

match translate_c_to_fortran(input) {
    Ok(fortran) => println!("Translated: {}", fortran),
    Err(TranslationError::EmptyInput) => eprintln!("Input is empty"),
    Err(TranslationError::ParseError(msg)) => eprintln!("Parse failed: {}", msg),
    Err(TranslationError::ConversionError(err)) => eprintln!("Conversion failed: {}", err),
}
```

**C API:**
```c
char* result = roup_convert_language(input, ROUP_LANG_C, ROUP_LANG_FORTRAN_FREE);
if (result == NULL) {
    fprintf(stderr, "Translation failed\n");
    return 1;
}
// Use result...
roup_string_free(result);
```

### Use Cases

1. **Benchmark porting** - Convert OpenMP benchmarks between languages
2. **Multi-language codebases** - Generate equivalent directives for polyglot projects
3. **Documentation** - Show directive equivalents across languages
4. **Code generation** - Generate language-specific directives from templates
5. **Migration tools** - Assist in language migration projects

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
- [Getting Started Guide](./getting-started.md) - Get started in 5 minutes
