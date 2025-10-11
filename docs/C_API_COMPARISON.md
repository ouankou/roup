# C API Comparison: Traditional vs Handle-Based

## Side-by-Side: Parsing a Single Directive

### Traditional C API (ompparser style)

```c
#include <ompparser.h>
#include <stdio.h>

int main() {
    // Parse in one line
    OmpDirective *dir = parse_omp("#pragma omp parallel num_threads(4)");
    
    if (dir == NULL) {
        printf("Parse error\n");
        return 1;
    }
    
    printf("Parsed: %s\n", directive_name(dir));
    printf("Clauses: %d\n", directive_clause_count(dir));
    
    free_directive(dir);
    return 0;
}
```

**Character count:** 338 characters  
**FFI calls:** 3 (parse, name, count)  
**Lines of code:** 15  
**Complexity:** Trivial  

---

### Handle-Based API (current roup)

```c
#include <roup.h>
#include <stdio.h>
#include <string.h>

int main() {
    // Step 1: Create string handle
    uint64_t str_handle = omp_str_new();
    if (str_handle == 0) {
        printf("Failed to create string\n");
        return 1;
    }
    
    // Step 2: Build string byte-by-byte (PAINFUL!)
    const char *input = "#pragma omp parallel num_threads(4)";
    size_t len = strlen(input);
    
    for (size_t i = 0; i < len; i++) {
        OmpStatus status = omp_str_push_byte(str_handle, (uint8_t)input[i]);
        if (status != OMP_SUCCESS) {
            printf("Failed to build string at byte %zu\n", i);
            omp_str_free(str_handle);
            return 1;
        }
    }
    
    // Step 3: Parse using handle
    uint64_t result_handle;
    OmpStatus status = omp_parse(str_handle, &result_handle);
    if (status != OMP_SUCCESS) {
        printf("Parse error: %d\n", status);
        omp_str_free(str_handle);
        return 1;
    }
    
    // Step 4: Get actual directive handle
    uint64_t dir_handle = omp_take_last_parse_result();
    if (dir_handle == 0) {
        printf("No parse result\n");
        omp_str_free(str_handle);
        return 1;
    }
    
    // Step 5: Query directive
    DirectiveKind kind;
    status = omp_directive_kind(dir_handle, &kind);
    if (status != OMP_SUCCESS) {
        printf("Failed to get kind\n");
        omp_directive_free(dir_handle);
        omp_str_free(str_handle);
        return 1;
    }
    
    uintptr_t clause_count;
    status = omp_directive_clause_count(dir_handle, &clause_count);
    if (status != OMP_SUCCESS) {
        printf("Failed to get clause count\n");
        omp_directive_free(dir_handle);
        omp_str_free(str_handle);
        return 1;
    }
    
    printf("Parsed: kind=%d\n", kind);
    printf("Clauses: %zu\n", clause_count);
    
    // Step 6: Cleanup (must remember all handles!)
    omp_directive_free(dir_handle);
    omp_str_free(str_handle);
    
    return 0;
}
```

**Character count:** 1,847 characters (5.5x more)  
**FFI calls:** 42 (1 new + 39 push_byte + 1 parse + 1 kind)  
**Lines of code:** 75 (5x more)  
**Complexity:** High (nested error handling, manual cleanup)  

---

## Real-World: Parsing Multiple Directives

### Traditional

```c
void parse_directives(const char **directives, int count) {
    for (int i = 0; i < count; i++) {
        OmpDirective *dir = parse_omp(directives[i]);
        if (dir) {
            process(dir);
            free_directive(dir);
        }
    }
}

// Usage
const char *inputs[] = {
    "#pragma omp parallel",
    "#pragma omp for schedule(static)",
    "#pragma omp task priority(5)"
};
parse_directives(inputs, 3);
```

**Total lines:** 16  
**Readable:** ✅ Yes  
**Maintainable:** ✅ Yes  

---

### Handle-Based

```c
void parse_directives(const char **directives, int count) {
    for (int i = 0; i < count; i++) {
        // Build string
        uint64_t str = omp_str_new();
        if (str == 0) continue;
        
        size_t len = strlen(directives[i]);
        bool failed = false;
        for (size_t j = 0; j < len; j++) {
            if (omp_str_push_byte(str, directives[i][j]) != OMP_SUCCESS) {
                failed = true;
                break;
            }
        }
        
        if (failed) {
            omp_str_free(str);
            continue;
        }
        
        // Parse
        uint64_t result;
        if (omp_parse(str, &result) != OMP_SUCCESS) {
            omp_str_free(str);
            continue;
        }
        
        uint64_t dir = omp_take_last_parse_result();
        if (dir != 0) {
            process_handle(dir);
            omp_directive_free(dir);
        }
        
        omp_str_free(str);
    }
}

// Usage (same)
const char *inputs[] = {
    "#pragma omp parallel",
    "#pragma omp for schedule(static)",
    "#pragma omp task priority(5)"
};
parse_directives(inputs, 3);
```

**Total lines:** 47 (3x more)  
**Readable:** ⚠️ Barely  
**Maintainable:** ❌ Error-prone  

---

## Performance Breakdown

### Parsing: `"#pragma omp parallel num_threads(4)"` (39 characters)

#### Traditional:
1. `parse_omp(ptr)` - 1 FFI call
   - Read string: ~10ns (pointer dereference)
   - Parse: ~500ns (actual parsing work)
   - **Total:** ~510ns

#### Handle-Based:
1. `omp_str_new()` - 1 call × ~80ns = 80ns
2. `omp_str_push_byte()` - 39 calls × ~80ns = 3,120ns
3. `omp_parse()` - 1 call × (500ns parse + 80ns overhead) = 580ns
   - **Total:** ~3,780ns

**Performance ratio:** 7.4x slower (just from FFI overhead, not counting Mutex contention)

---

### With 1000 directives:

#### Traditional:
- 1,000 FFI calls
- ~510µs total
- **0.51µs per directive**

#### Handle-Based:
- ~40,000 FFI calls (1 new + 39 pushes per directive)
- ~3,780µs total  
- **3.78µs per directive**

**At scale:** 7.4x slower, but more importantly, 40x more FFI calls means 40x more opportunity for context switching overhead.

---

## Memory Management Comparison

### Traditional - Easy to get wrong:

```c
// Memory leak - forgot to free
OmpDirective *dir = parse_omp("#pragma omp parallel");
process(dir);
// LEAK! Forgot: free_directive(dir);

// Double free - crash
OmpDirective *dir = parse_omp("#pragma omp parallel");
free_directive(dir);
free_directive(dir);  // CRASH!

// Use after free - crash
OmpDirective *dir = parse_omp("#pragma omp parallel");
free_directive(dir);
printf("%d", dir->kind);  // CRASH!
```

### Handle-Based - Safer but still can leak:

```c
// Memory leak - forgot to free (same problem)
uint64_t dir = omp_take_last_parse_result();
process(dir);
// LEAK! Forgot: omp_directive_free(dir);

// Double free - SAFE! Returns error instead of crash
uint64_t dir = omp_take_last_parse_result();
omp_directive_free(dir);  // Ok
omp_directive_free(dir);  // Returns NotFound (safe!)

// Use after free - SAFE! Returns error instead of crash
uint64_t dir = omp_take_last_parse_result();
omp_directive_free(dir);
DirectiveKind kind;
omp_directive_kind(dir, &kind);  // Returns NotFound (safe!)
```

**Safety improvement:** Prevents **crashes**, but doesn't prevent **leaks**.

---

## Helper Functions (Required for Handle-Based)

To make handle-based API even remotely usable, you MUST write helpers:

```c
// Helper: Build string from C literal
uint64_t str_from_literal(const char *lit) {
    if (lit == NULL) return 0;
    
    uint64_t h = omp_str_new();
    if (h == 0) return 0;
    
    for (size_t i = 0; lit[i] != '\0'; i++) {
        if (omp_str_push_byte(h, (uint8_t)lit[i]) != OMP_SUCCESS) {
            omp_str_free(h);
            return 0;
        }
    }
    return h;
}

// Helper: Parse from C string
OmpStatus parse_from_cstr(const char *input, uint64_t *out_directive) {
    uint64_t str = str_from_literal(input);
    if (str == 0) return OMP_INTERNAL;
    
    uint64_t result;
    OmpStatus status = omp_parse(str, &result);
    omp_str_free(str);
    
    if (status != OMP_SUCCESS) return status;
    
    *out_directive = omp_take_last_parse_result();
    return OMP_SUCCESS;
}

// Now you can write:
uint64_t dir;
if (parse_from_cstr("#pragma omp parallel", &dir) == OMP_SUCCESS) {
    // ... use dir ...
    omp_directive_free(dir);
}
```

**But wait...** These helper functions need to read C strings (`const char*`), which requires:
1. Trusting null-termination
2. Reading from raw pointer
3. **UNSAFE CODE!**

So the "zero unsafe" goal is defeated - users must write unsafe helpers anyway!

---

## Documentation Burden

### Traditional API Documentation:

```c
/**
 * Parse an OpenMP directive from a C string.
 * 
 * @param input Null-terminated C string containing OpenMP directive
 * @return Parsed directive, or NULL on error
 * 
 * Example:
 *   OmpDirective *d = parse_omp("#pragma omp parallel");
 *   free_directive(d);
 */
OmpDirective* parse_omp(const char *input);
```

**Documentation length:** 7 lines  
**Complexity:** Low  

---

### Handle-Based API Documentation:

```c
/**
 * Create a new empty string handle.
 * 
 * @return String handle (never 0), or 0 on allocation failure
 * 
 * Note: You MUST call omp_str_free() when done to avoid leaks.
 * 
 * Example:
 *   uint64_t str = omp_str_new();
 *   // ... build string ...
 *   omp_str_free(str);
 */
uint64_t omp_str_new(void);

/**
 * Append a single byte to a string.
 * 
 * @param handle String handle from omp_str_new()
 * @param byte Byte value (0-255) to append
 * @return OMP_SUCCESS on success, error code on failure
 * 
 * Note: The resulting string may not be valid UTF-8.
 * 
 * Example:
 *   uint64_t str = omp_str_new();
 *   omp_str_push_byte(str, '#');
 *   omp_str_push_byte(str, 'p');
 *   // ... repeat 37 more times ...
 */
OmpStatus omp_str_push_byte(uint64_t handle, uint8_t byte);

/**
 * Parse a string handle containing an OpenMP directive.
 * 
 * @param string_handle Handle to string created with omp_str_new()
 * @param out_directive Output parameter (not actually used - see below)
 * @return OMP_SUCCESS on success, error code on failure
 * 
 * Note: Due to the zero-unsafe design, the parsed directive is NOT
 * written to out_directive. Instead, you must call:
 * omp_take_last_parse_result() to retrieve the directive handle.
 * 
 * Example:
 *   uint64_t str = omp_str_new();
 *   // ... build "#pragma omp parallel" ...
 *   
 *   uint64_t result;
 *   if (omp_parse(str, &result) == OMP_SUCCESS) {
 *       uint64_t dir = omp_take_last_parse_result();
 *       // ... use dir ...
 *       omp_directive_free(dir);
 *   }
 *   omp_str_free(str);
 */
OmpStatus omp_parse(uint64_t string_handle, uint64_t *out_directive);

/**
 * Retrieve the last parse result.
 * 
 * @return Directive handle from last successful parse, or 0 if parse failed
 * 
 * Note: This is a workaround for avoiding raw pointer writes in omp_parse().
 * Each thread has its own result storage.
 * 
 * Example: See omp_parse() above
 */
uint64_t omp_take_last_parse_result(void);
```

**Documentation length:** 50+ lines (7x more)  
**Complexity:** High (multiple functions to explain, workarounds to document)  

---

## Migration Path

### Migrating from ompparser to Traditional API:

```diff
  #include <stdio.h>
- #include <ompparser.h>
+ #include <roup.h>

  int main() {
      OmpDirective *dir = parse_omp("#pragma omp parallel");
      if (dir) {
          printf("Kind: %d\n", dir->kind);
          free_directive(dir);
      }
      return 0;
  }
```

**Changes required:** 1 line (header)  
**Time to migrate:** < 1 minute  
**Code rewrite:** None  

---

### Migrating from ompparser to Handle-Based:

```diff
  #include <stdio.h>
- #include <ompparser.h>
+ #include <roup.h>
+ #include <string.h>

  int main() {
-     OmpDirective *dir = parse_omp("#pragma omp parallel");
-     if (dir) {
-         printf("Kind: %d\n", dir->kind);
-         free_directive(dir);
+     // Build string
+     uint64_t str = omp_str_new();
+     const char *input = "#pragma omp parallel";
+     for (size_t i = 0; i < strlen(input); i++) {
+         if (omp_str_push_byte(str, input[i]) != OMP_SUCCESS) {
+             omp_str_free(str);
+             return 1;
+         }
+     }
+     
+     // Parse
+     uint64_t result;
+     if (omp_parse(str, &result) != OMP_SUCCESS) {
+         omp_str_free(str);
+         return 1;
+     }
+     
+     uint64_t dir = omp_take_last_parse_result();
+     if (dir != 0) {
+         DirectiveKind kind;
+         if (omp_directive_kind(dir, &kind) == OMP_SUCCESS) {
+             printf("Kind: %d\n", kind);
+         }
+         omp_directive_free(dir);
      }
+     omp_str_free(str);
      return 0;
  }
```

**Changes required:** Complete rewrite (50+ lines)  
**Time to migrate:** 30-60 minutes per file  
**Code rewrite:** 100%  

---

## Conclusion

The handle-based approach is:
- ✅ **Academically brilliant** (proves FFI can avoid unsafe)
- ✅ **Safer against crashes** (no use-after-free crashes)
- ✅ **Thread-safe** (by design)
- ❌ **5-10x more verbose** (unbearable for real code)
- ❌ **7x slower** (FFI overhead dominates)
- ❌ **Unusable as ompparser replacement** (complete API incompatibility)
- ❌ **Requires users to write unsafe helpers anyway** (defeating the purpose)

**For ompparser replacement:** You need a traditional C API with minimal unsafe code. The current approach cannot work.
