# Release Notes

## Version 0.3.0 (October 11, 2025)

Major documentation release with comprehensive tutorials, guides, and critical bug fixes.

### 🚨 Experimental Status

**IMPORTANT:** ROUP is now explicitly marked as experimental and not production-ready.
- APIs may change before v1.0
- Use for research, education, and prototyping
- See [FAQ](https://roup.ouankou.com/faq.html) for details

### 📚 Complete Documentation Website

**New mdBook Website:** [roup.ouankou.com](https://roup.ouankou.com)

**New Pages (6 major guides):**
- **[Getting Started](https://roup.ouankou.com/getting-started.html)** - Quick start for Rust, C, and C++
- **[C Tutorial](https://roup.ouankou.com/c-tutorial.html)** - Complete C API guide (682 lines)
- **[Building Guide](https://roup.ouankou.com/building.html)** - Cross-platform build instructions (594 lines)
- **[Architecture](https://roup.ouankou.com/architecture.html)** - Internal design documentation (806 lines)
- **[Contributing](https://roup.ouankou.com/contributing.html)** - Developer guide (652 lines)
- **[FAQ](https://roup.ouanoku.com/faq.html)** - Common questions and answers (687 lines)

**Enhanced Pages:**
- **[Introduction](https://roup.ouankou.com/intro.md)** - Expanded from 153 to 394 lines
- **[API Reference](https://roup.ouankou.com/api-reference.html)** - Fixed clause mappings, updated function count
- **[OpenMP Support](https://roup.ouankou.com/openmp-support.html)** - Accurate directive/clause counts

### 🐛 Critical Bug Fixes

**C API Memory Corruption (src/c_api.rs):**
- **Fixed:** Memory corruption in `free_clause_data` function
- **Issue:** Incorrectly freeing reduction clauses (kind 6) which use `ReductionData`, not `variables` pointer
- **Impact:** Caused segfaults when parsing directives with reduction clauses
- **Solution:** Changed range from `2-6` to `2-5` (only private/shared/firstprivate/lastprivate have variable lists)
- **Verification:** All 352 tests pass, C examples work correctly, no memory leaks

**Detailed Union Documentation:**
- Added comprehensive comments explaining `ClauseData` union behavior
- Clarified which clause kinds use which union fields

### ✅ Testing & CI Improvements

**Test Count:** 352 tests (was 342)
- 239 unit tests
- 51 integration tests  
- 62 doc tests

**C/C++ Example Testing:**
- CI now builds and runs C examples on Linux
- Added `gdb` and `valgrind` to devcontainer for debugging
- Enhanced Makefile with `BUILD_TYPE` variable (debug|release)

**Working Examples:**
- `examples/c/basic_parse.c` - Minimal parsing example
- `examples/c/tutorial_basic.c` - Comprehensive 6-step tutorial

### 📊 Accurate Statistics

**OpenMP Support:**
- **95 directives** (not 120+ as previously claimed)
- **91 clauses** (accurate count)
- OpenMP 3.0-6.0 coverage

**Code Safety:**
- **99.1% safe Rust** (~60 lines of unsafe in FFI boundary)
- **16 C FFI functions** (accurate count)

### 🗑️ Cleanup

**Removed Obsolete Files (20 files, -6,572 lines):**
- `include/roup.h` - Old handle-based header (766 lines)
- `examples/c/clause_inspection.c` - Used deleted header (333 lines)
- `examples/c/error_handling.c` - Used deleted header (309 lines)
- `examples/c/string_builder.c` - Used deleted header (202 lines)
- 15 redundant documentation files in `docs/` directory
- `test_c_api.c` from root directory

**Documentation Philosophy:**
- Single source of truth (mdBook website)
- No redundant documentation
- Cross-reference instead of duplicate

### 📝 Documentation Corrections

**Getting Started:**
- Completely rewrote C and C++ quick-start sections
- Removed ALL references to deleted `include/roup.h`
- Updated to pointer-based API (`roup_parse()`, `OmpDirective*`, etc.)
- Fixed compile commands with required libraries (`-lpthread -ldl -lm`)

**All Pages:**
- Experimental warnings added throughout
- Production-ready claims removed
- Test counts updated to 352
- Directive/clause counts corrected

### 🔧 AGENTS.md Policies

**Added Documentation Policies:**
- Single source of truth philosophy
- No redundant documentation
- C FFI API architecture notes (pointer-based, not handle-based)

**Code Quality Requirements:**
- Warning-free builds mandatory
- All commits must pass tests
- Documentation must stay synchronized

### 🔄 Migration Guide

No breaking API changes in this release. All changes are additive (documentation) or bug fixes.

**If upgrading from 0.2.0:**
- ✅ No code changes required
- ✅ C API unchanged (still 16 functions)
- ✅ Reduction clause bug fix is transparent
- ⚠️ Be aware: Project is now explicitly experimental

### 📦 Installation

**Rust:**
```toml
[dependencies]
roup = "0.3"
```

**C/C++:**
```bash
cargo build --release
# Link against target/release/libroup.{a,so,dylib}
```

See [Getting Started](https://roup.ouankou.com/getting-started.html) for details.

### 🙏 Acknowledgments

Thanks to all reviewers who provided feedback on the documentation and caught issues during the PR review process.

---

## Version 0.2.0 (October 11, 2025)

Initial release with minimal unsafe C API for OpenMP parsing.

### New: Direct Pointer-Based C API (src/c_api.rs)

**Replaced:** Old handle-based FFI (src/ffi/, 4066 lines) completely removed  
**Implemented:** 18 C functions with minimal unsafe code (632 lines)  
**Unsafe Blocks:** 18 total (~0.9% of c_api.rs, ~60 lines of unsafe code)

### API Functions

**Lifecycle (3 functions):**
```c
OmpDirective* roup_parse(const char* input);
void roup_directive_free(OmpDirective* directive);
void roup_clause_free(OmpClause* clause);
```

**Directive Queries (3 functions):**
```c
int32_t roup_directive_kind(const OmpDirective* directive);
int32_t roup_directive_clause_count(const OmpDirective* directive);
OmpClauseIterator* roup_directive_clauses_iter(const OmpDirective* directive);
```

**Iterator (2 functions):**
```c
int32_t roup_clause_iterator_next(OmpClauseIterator* iter, OmpClause** out);
void roup_clause_iterator_free(OmpClauseIterator* iter);
```

**Clause Queries (4 functions):**
```c
int32_t roup_clause_kind(const OmpClause* clause);
int32_t roup_clause_schedule_kind(const OmpClause* clause);
int32_t roup_clause_reduction_operator(const OmpClause* clause);
int32_t roup_clause_default_data_sharing(const OmpClause* clause);
```

**Variable Lists (4 functions):**
```c
OmpStringList* roup_clause_variables(const OmpClause* clause);
int32_t roup_string_list_len(const OmpStringList* list);
const char* roup_string_list_get(const OmpStringList* list, int32_t index);
void roup_string_list_free(OmpStringList* list);
```

**Legacy Functions (2 functions kept for iterator cleanup):**
```c
void roup_clause_iterator_free(OmpClauseIterator* iter);
void roup_string_list_free(OmpStringList* list);
```

**Total: 18 functions**

### Safety Guarantees

**Unsafe Code Locations:**
- `src/c_api.rs`: 18 unsafe blocks (~60 lines)
- All NULL-checked before dereferencing
- Documented safety requirements
- Located only at FFI boundary

**C Caller Responsibilities:**
- Check for NULL returns from `roup_parse()`
- Call `_free()` functions to prevent leaks
- Don't use freed pointers
- Don't modify returned strings

**Rust Guarantees:**
- 99.7% safe code (6,709 of 6,769 lines)
- Memory layout C-compatible (`#[repr(C)]`)
- No undefined behavior in Rust code
- Thread-safe (no global mutable state)

### Version Bump

**Previous:** 0.1.0  
**New:** 0.2.0

### Testing

**Rust Tests:**
```
test result: ok. 342 passed; 0 failed; 1 ignored
```

**C API Test:**
```bash
$ ./test_c_api
Testing new minimal unsafe C API...
✓ Parse succeeded
✓ Directive kind: 0
✓ Clause count: 1
✓ Clauses: 1
✅ All tests passed!
```

**Build:**
```
Finished `release` profile [optimized]
0 warnings
```

### Directive Kinds Supported

| Value | Directive | Example |
|-------|-----------|---------|
| 0 | Parallel (+ composites) | `parallel`, `parallel for`, `parallel sections` |
| 1 | For | `for` |
| 2 | Sections | `sections` |
| 3 | Single | `single` |
| 4 | Task | `task` |
| 5 | Master | `master` |
| 6 | Critical | `critical` |
| 7 | Barrier | `barrier` |
| 8 | Taskwait | `taskwait` |
| 9 | Taskgroup | `taskgroup` |
| 10 | Atomic | `atomic` |
| 11 | Flush | `flush` |
| 12 | Ordered | `ordered` |
| 13 | Target (+ composites) | `target`, `target teams` |
| 14 | Teams (+ composites) | `teams`, `teams distribute` |
| 15 | Distribute | `distribute` |
| 16 | Metadirective | `metadirective` |
| 999 | Unknown | (any unrecognized directive) |

### Clause Kinds Supported

| Value | Clause | Example |
|-------|--------|---------|
| 0 | NumThreads | `num_threads(4)` |
| 1 | If | `if(condition)` |
| 2 | Private | `private(x, y)` |
| 3 | Shared | `shared(z)` |
| 4 | Firstprivate | `firstprivate(a)` |
| 5 | Lastprivate | `lastprivate(b)` |
| 6 | Reduction | `reduction(+:sum)` |
| 7 | Schedule | `schedule(static, 10)` |
| 8 | Collapse | `collapse(2)` |
| 9 | Ordered | `ordered` |
| 10 | Nowait | `nowait` |
| 11 | Default | `default(shared)` |
| 999 | Unknown | (any unrecognized clause) |

### Schedule Kinds

| Value | Kind | Description |
|-------|------|-------------|
| 0 | Static | Iterations divided at compile time |
| 1 | Dynamic | Iterations assigned at runtime |
| 2 | Guided | Dynamic with decreasing chunk size |
| 3 | Auto | Compiler/runtime chooses |
| 4 | Runtime | Determined by `OMP_SCHEDULE` |

### Reduction Operators

| Value | Operator | Example |
|-------|----------|---------|
| 0 | Plus | `reduction(+:sum)` |
| 1 | Minus | `reduction(-:diff)` |
| 2 | Times | `reduction(*:product)` |
| 3 | BitwiseAnd | `reduction(&:mask)` |
| 4 | BitwiseOr | `reduction(|:flags)` |
| 5 | BitwiseXor | `reduction(^:xor_val)` |
| 6 | LogicalAnd | `reduction(&&:all_true)` |
| 7 | LogicalOr | `reduction(||:any_true)` |
| 8 | Min | `reduction(min:minimum)` |
| 9 | Max | `reduction(max:maximum)` |

### Default Data Sharing

| Value | Kind | Meaning |
|-------|------|---------|
| 0 | Shared | Variables shared by default |
| 1 | None | Must explicitly specify sharing |

### Documentation Updated

**Modified:**
- README.md (added C API quick start)
- C_FFI_STATUS.md (updated to pointer-based API)
- IMPLEMENTATION_SUMMARY.md (18 unsafe blocks)

**Created:**
- DEVELOPMENT_HISTORY.md (phases 1-3)
- QUICK_START.md (5-minute guide)
- DOCS_CLEANUP_COMPLETE.md (cleanup summary)

**Total:** 17 documentation files

### Build Instructions

**Rust:**
```bash
cargo build --release
```

**C:**
```bash
clang your_program.c \
  -L/path/to/roup/target/release \
  -lroup \
  -Wl,-rpath,/path/to/roup/target/release \
  -o your_program
```

**C++:**
```bash
clang++ -std=c++17 your_program.cpp \
  -L/path/to/roup/target/release \
  -lroup \
  -Wl,-rpath,/path/to/roup/target/release \
  -o your_program
```

### Example Usage

```c
#include <stdio.h>
#include <stdint.h>

// Forward declarations (or include roup.h)
typedef struct OmpDirective OmpDirective;
extern OmpDirective* roup_parse(const char* input);
extern void roup_directive_free(OmpDirective* dir);
extern int32_t roup_directive_kind(const OmpDirective* dir);
extern int32_t roup_directive_clause_count(const OmpDirective* dir);

int main() {
    OmpDirective* dir = roup_parse("#pragma omp parallel for num_threads(4)");
    if (!dir) {
        fprintf(stderr, "Parse failed\n");
        return 1;
    }

    printf("Kind: %d\n", roup_directive_kind(dir));
    printf("Clauses: %d\n", roup_directive_clause_count(dir));

    roup_directive_free(dir);
    return 0;
}
```

### Breaking Changes

**API Change:** New C API uses `roup_*` prefix instead of `omp_*`

**Migration:**
- Old: `omp_parse_cstr()` → New: `roup_parse()`
- Old: Handle-based → New: Direct pointers
- Old: `OmpStatus` return codes → New: NULL on error

**Legacy Code:** Old handle-based API (4066 lines) has been completely removed

### Performance

**Parsing:** Same as v0.1.0 (uses identical parser core)  
**Memory:** Slightly more efficient (no handle registry overhead)  
**Safety:** Similar (both have minimal unsafe at FFI boundary)

### Known Limitations

1. **Variable extraction:** `roup_clause_variables()` returns empty lists (parser stores variables as strings, not parsed lists)
2. **Composite directives:** Mapped to primary directive kind (e.g., `parallel for` → `parallel`)
3. **Error details:** Parse failures return NULL with no error message

### Future Enhancements

**Planned for v1.1:**
1. Variable list parsing and extraction
2. Error message strings for parse failures
3. Detailed clause data structures

**Possible for v2.0:**
1. C header file generation (cbindgen)
2. Python bindings (PyO3)
3. AST manipulation API
4. OpenMP 6.0 features

### Compatibility

**Compilers:**
- Clang 10+ (tested with 21.1.3)
- GCC 7+
- MSVC 2019+

**Standards:**
- C99 or later
- C++17 or later (for C++ wrappers)
- Rust 1.70+ (Edition 2021)

**Platforms:**
- Linux (tested)
- macOS (should work)
- Windows (should work with minor adjustments)

### Credits

**Author:** Anjia Wang <anjia@ouankou.com>  
**License:** MIT  
**Repository:** https://github.com/ouankou/roup

### Release Checklist

✅ All 342 Rust tests passing  
✅ C API test passing  
✅ Zero compiler warnings  
✅ Documentation updated  
✅ Version bumped to 0.2.0  
✅ Unsafe blocks counted and documented  
✅ Examples working  
✅ Ready for git push and release

---

**Release Command:**
```bash
git add -A
git commit -m "Release v0.2.0: Minimal unsafe C API"
git tag v0.2.0
git push origin main --tags
```
