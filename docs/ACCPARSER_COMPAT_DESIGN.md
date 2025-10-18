# accparser Compatibility Layer - Design Document

**Status**: 🚧 In Progress
**Branch**: `feature/openacc-with-accparser-compat`
**Related Issue**: #67

## Executive Summary

This document details the design and implementation plan for the accparser compatibility layer, which provides a drop-in replacement for [accparser](https://github.com/ouankou/accparser) using ROUP as the backend parser.

### Progress So Far

✅ **Completed**:
- OpenACC parsing infrastructure (30+ clauses, 23+ directives)
- Round-trip parsing tests (2/2 passing)
- All P1 bugs from PR #66 fixed
- Comprehensive accparser and ompparser compat layer analysis

🚧 **In Progress**:
- OpenACC C API functions

📋 **TODO**:
- Complete OpenACC C API implementation
- Create compat/accparser/ directory structure
- Implement compatibility wrapper
- Build system integration
- Test with accparser's built-in test suite

## Background

### What is accparser?

accparser is a standalone OpenACC parser built on ANTLR 4. It:
- Supports both C and Fortran
- Generates OpenACC AST (Abstract Syntax Tree)
- Provides toString() and generateDOT() visualization
- Can translate OpenACC → OpenMP (via ompparser)
- **Problem**: Requires ANTLR 4 (Ubuntu 24.04 ships incompatible versions)

### Why Replace the Parser?

**ANTLR 4 Version Mismatch on Ubuntu 24.04**:
- System antlr4 executable: 4.9
- System libantlr4-runtime: 4.10
- Mismatch causes build failures
- Manual download/installation required

**ROUP Benefits**:
- Zero external dependencies (pure Rust)
- Safe, fast parsing
- Better error messages
- Consistent versioning

### Architecture

```
User Code (uses parseOpenACC, OpenACCDirective API)
    ↓
compat_impl.cpp (compatibility wrapper, ~190 lines)
    ↓
ROUP OpenACC C API (acc_parse, acc_directive_kind, etc.)
    ↓
ROUP Parser (Rust - safe, fast OpenACC parsing)
```

The library reuses accparser's own implementation for toString(), generateDOT(), and other methods (zero code duplication).

## accparser Repository Analysis

### Directory Structure

```
accparser/
├── src/
│   ├── acclexer.g4                  # ANTLR lexer (WILL REPLACE)
│   ├── accparser.g4                 # ANTLR parser (WILL REPLACE)
│   ├── OpenACCASTConstructor.cpp/h  # Builds IR from ANTLR (WON'T NEED)
│   ├── OpenACCIR.cpp/h             # IR structure (WILL REUSE)
│   ├── OpenACCIRToString.cpp       # toString() methods (WILL REUSE)
│   └── OpenACCKinds.h               # Directive/clause enums (WILL REUSE)
├── tests/
│   ├── acc_tester.cpp               # Test driver
│   ├── base/                         # Test cases
│   ├── gpubootcamp/                 # GPU Bootcamp tests
│   └── openacc-users-group/         # Community tests
├── acc_demo.cpp                     # Usage example
└── CMakeLists.txt                   # Build configuration

└── CMakeLists.txt                   # Build configuration
```

### Key API (from OpenACCIR.h)

**Directive Classes**:
```cpp
class OpenACCDirective {
    OpenACCDirectiveKind kind;
    OpenACCBaseLang lang;
    std::vector<OpenACCClause*> *clauses_in_original_order;
    std::map<OpenACCClauseKind, std::vector<OpenACCClause*>*> clauses;

    std::string toString();
    std::string generatePragmaString(std::string prefix = "#pragma acc ");
    OpenACCClause* addOpenACCClause(int, ...);
};

// Special directive classes
class OpenACCCacheDirective : public OpenACCDirective { /* ... */ };
class OpenACCEndDirective : public OpenACCDirective { /* ... */ };
class OpenACCRoutineDirective : public OpenACCDirective { /* ... */ };
class OpenACCWaitDirective : public OpenACCDirective { /* ... */ };
```

**Clause Classes**:
```cpp
class OpenACCClause {
    OpenACCClauseKind kind;
    std::vector<std::string> expressions;

    std::string toString();
    void addLangExpr(std::string expression_string, int line, int col);
};

// Specialized clause classes (with modifiers/parameters)
class OpenACCAsyncClause : public OpenACCClause { /* ... */ };
class OpenACCBindClause : public OpenACCClause { /* ... */ };
class OpenACCCollapseClause : public OpenACCClause { /* ... */ };
// ... ~20 more specialized clause classes
```

**Entry Point** (from acc_demo.cpp):
```cpp
extern OpenACCDirective* parseOpenACC(std::string source);
```

### Enums (from OpenACCKinds.h)

**Directives** (21 total):
```cpp
enum OpenACCDirectiveKind {
    ACCD_atomic, ACCD_cache, ACCD_data, ACCD_declare,
    ACCD_end, ACCD_enter_data, ACCD_exit_data, ACCD_host_data,
    ACCD_init, ACCD_kernels, ACCD_kernels_loop, ACCD_loop,
    ACCD_parallel, ACCD_parallel_loop, ACCD_routine, ACCD_serial,
    ACCD_serial_loop, ACCD_set, ACCD_shutdown, ACCD_update,
    ACCD_wait, ACCD_unknown
};
```

**Clauses** (47 total):
```cpp
enum OpenACCClauseKind {
    ACCC_async, ACCC_attach, ACCC_auto, ACCC_bind, ACCC_capture,
    ACCC_collapse, ACCC_copy, ACCC_copyin, ACCC_copyout, ACCC_create,
    ACCC_default_async, ACCC_default, ACCC_delete, ACCC_detach,
    ACCC_device, ACCC_device_num, ACCC_device_resident, ACCC_device_type,
    ACCC_deviceptr, ACCC_finalize, ACCC_firstprivate, ACCC_gang,
    ACCC_host, ACCC_if, ACCC_if_present, ACCC_independent, ACCC_link,
    ACCC_nohost, ACCC_no_create, ACCC_num_gangs, ACCC_num_workers,
    ACCC_present, ACCC_private, ACCC_reduction, ACCC_read, ACCC_self,
    ACCC_seq, ACCC_tile, ACCC_update, ACCC_use_device, ACCC_vector,
    ACCC_vector_length, ACCC_wait, ACCC_worker, ACCC_write,
    ACCC_unknown
};
```

### Build System (from CMakeLists.txt)

**Current Dependencies**:
```cmake
find_program(NAMES antlr4)  # Line 13 - WILL REMOVE

# Lines 59-65: Generate C++ from ANTLR grammar - WILL REMOVE
add_custom_command(OUTPUT
    ${ACCPARSER_GRAMMAR_TARGET_FILES}
    COMMAND antlr4 ${ACCPARSER_GRAMMAR_FILES} ...
)

# Lines 72-83: Build libaccparser.so
add_library(accparser SHARED
    ${ACCIR_SOURCE_FILES}
    ${ACCPARSER_GRAMMAR_TARGET_FILES}
)
target_link_libraries(accparser antlr4-runtime)  # WILL REMOVE
```

**After ROUP Integration**:
```cmake
# No antlr4 dependency!
set(ROUP_STATIC_LIB "${CMAKE_CURRENT_SOURCE_DIR}/path/to/libroup.a")

add_library(accparser SHARED
    src/compat_impl.cpp                 # ROUP compatibility wrapper
    ${ACCIR_SOURCE_FILES}               # Reuse existing IR/toString/etc
)
target_link_libraries(accparser
    ${ROUP_STATIC_LIB}
    pthread dl  # Required for embedded Rust
)
```

## Implementation Plan

### Phase 1: OpenACC C API (src/c_api.rs)

**Required Functions** (mirroring OpenMP API):
```rust
// Core parsing
#[no_mangle]
pub extern "C" fn acc_parse(input: *const c_char) -> *mut AccDirective;
pub extern "C" fn acc_directive_free(directive: *mut AccDirective);

// Directive queries
pub extern "C" fn acc_directive_kind(directive: *const AccDirective) -> i32;
pub extern "C" fn acc_directive_clause_count(directive: *const AccDirective) -> i32;
pub extern "C" fn acc_directive_clauses_iter(directive: *const AccDirective) -> *mut AccClauseIterator;

// Iterator operations
pub extern "C" fn acc_clause_iterator_next(iter: *mut AccClauseIterator, out: *mut *const AccClause) -> i32;
pub extern "C" fn acc_clause_iterator_free(iter: *mut AccClauseIterator);

// Clause queries
pub extern "C" fn acc_clause_kind(clause: *const AccClause) -> i32;
pub extern "C" fn acc_clause_expressions_count(clause: *const AccClause) -> i32;
pub extern "C" fn acc_clause_expression_at(clause: *const AccClause, index: usize) -> *const c_char;
```

**Data Structures**:
```rust
#[repr(C)]
pub struct AccDirective {
    name: *const c_char,
    clauses: Vec<AccClause>,
}

#[repr(C)]
pub struct AccClause {
    kind: i32,
    expressions: *mut AccStringList,  // For variable lists
}

#[repr(C)]
pub struct AccClauseIterator {
    clauses: Vec<*const AccClause>,
    index: usize,
}

#[repr(C)]
pub struct AccStringList {
    items: Vec<*const c_char>,
}
```

**Directive/Clause Mapping** (constants):
```rust
// build.rs will generate these as #define macros in roup_acc_constants.h
const ACC_DIRECTIVE_PARALLEL: i32 = 0;
const ACC_DIRECTIVE_LOOP: i32 = 1;
const ACC_DIRECTIVE_KERNELS: i32 = 2;
// ... all 21 directives

const ACC_CLAUSE_ASYNC: i32 = 0;
const ACC_CLAUSE_WAIT: i32 = 1;
const ACC_CLAUSE_NUM_GANGS: i32 = 2;
// ... all 47 clauses
```

**Implementation Notes**:
- Map ROUP's OpenACC directive names to integer constants
- Map ROUP's clause names to integer constants
- Handle special directives (cache, wait, end) - may need special handling
- Reuse pattern from ompparser C API (src/c_api.rs:213-632)

### Phase 2: C API Constants Generation (build.rs)

**Update build.rs** to generate `roup_acc_constants.h`:
```rust
// After OpenMP constants, add:
writeln!(header, "\n// OpenACC Directive Kinds")?;
writeln!(header, "#define ACC_DIRECTIVE_PARALLEL 0")?;
writeln!(header, "#define ACC_DIRECTIVE_LOOP 1")?;
// ... all directives

writeln!(header, "\n// OpenACC Clause Kinds")?;
writeln!(header, "#define ACC_CLAUSE_ASYNC 0")?;
writeln!(header, "#define ACC_CLAUSE_WAIT 1")?;
// ... all clauses
```

### Phase 3: Compatibility Layer (compat/accparser/)

**Directory Structure** (mirroring ompparser):
```
compat/accparser/
├── README.md                    # Documentation
├── build.sh                     # One-command build
├── CMakeLists.txt              # Build configuration
├── accparser.pc.in             # pkg-config template
├── src/
│   ├── compat_impl.cpp         # Main wrapper (~190 lines)
│   └── roup_compat.h           # Optional header
├── accparser/                   # Git submodule
│   ├── src/
│   │   ├── OpenACCIR.h         # Reuse headers
│   │   ├── OpenACCIR.cpp       # Reuse implementation
│   │   ├── OpenACCIRToString.cpp
│   │   └── OpenACCKinds.h
│   └── ...
├── examples/
│   └── basic_test.cpp           # Example usage
└── tests/
    └── comprehensive_test.cpp   # Test suite
```

**compat_impl.cpp** (key sections):
```cpp
#include <OpenACCIR.h>
#include <cstring>
#include <string>

// Include ROUP OpenACC constants
#include <roup_acc_constants.h>

extern "C" {
    // ROUP C API forward declarations
    struct AccDirective;
    struct AccClause;
    struct AccClauseIterator;

    AccDirective* acc_parse(const char* input);
    void acc_directive_free(AccDirective* directive);
    int32_t acc_directive_kind(const AccDirective* directive);
    AccClauseIterator* acc_directive_clauses_iter(const AccDirective* directive);
    int32_t acc_clause_iterator_next(AccClauseIterator* iter, const AccClause** out);
    void acc_clause_iterator_free(AccClauseIterator* iter);
    int32_t acc_clause_kind(const AccClause* clause);
}

// Map ROUP kinds to accparser kinds
static OpenACCDirectiveKind mapRoupToAccparserDirective(int32_t roup_kind) {
    switch (roup_kind) {
        case ACC_DIRECTIVE_PARALLEL: return ACCD_parallel;
        case ACC_DIRECTIVE_LOOP: return ACCD_loop;
        case ACC_DIRECTIVE_KERNELS: return ACCD_kernels;
        // ... all 21 directives
        default: return ACCD_unknown;
    }
}

static OpenACCClauseKind mapRoupToAccparserClause(int32_t roup_kind) {
    switch (roup_kind) {
        case ACC_CLAUSE_ASYNC: return ACCC_async;
        case ACC_CLAUSE_WAIT: return ACCC_wait;
        // ... all 47 clauses
        default: return ACCC_unknown;
    }
}

// Main entry point (matches accparser API)
extern "C" {
    OpenACCDirective* parseOpenACC(std::string source) {
        // Call ROUP parser
        AccDirective* roup_dir = acc_parse(source.c_str());
        if (!roup_dir) return nullptr;

        // Get directive kind
        int32_t roup_kind = acc_directive_kind(roup_dir);
        OpenACCDirectiveKind kind = mapRoupToAccparserDirective(roup_kind);

        // Create accparser-compatible directive
        OpenACCDirective* dir = new OpenACCDirective(kind);

        // Convert clauses
        AccClauseIterator* iter = acc_directive_clauses_iter(roup_dir);
        if (iter) {
            const AccClause* roup_clause;
            while (acc_clause_iterator_next(iter, &roup_clause) == 1) {
                int32_t clause_kind_int = acc_clause_kind(roup_clause);
                OpenACCClauseKind clause_kind = mapRoupToAccparserClause(clause_kind_int);

                // Add clause using accparser's API
                dir->addOpenACCClause(static_cast<int>(clause_kind));
            }
            acc_clause_iterator_free(iter);
        }

        acc_directive_free(roup_dir);
        return dir;
    }
}
```

**CMakeLists.txt** (adapted from ompparser):
```cmake
cmake_minimum_required(VERSION 3.10)
project(roup-accparser-compat VERSION 0.1.0 LANGUAGES CXX C)

set(CMAKE_CXX_STANDARD 11)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Submodule check
if(NOT EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/accparser/src/OpenACCIR.h")
    message(FATAL_ERROR "accparser submodule not initialized!")
endif()

# ROUP library
set(ROUP_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../..")
set(ROUP_STATIC_LIB "${ROUP_ROOT}/target/release/libroup.a")

# Check/build ROUP
if(NOT EXISTS "${ROUP_STATIC_LIB}")
    execute_process(
        COMMAND cargo build --release
        WORKING_DIRECTORY "${ROUP_ROOT}"
        RESULT_VARIABLE CARGO_RESULT
    )
    if(NOT CARGO_RESULT EQUAL 0)
        message(FATAL_ERROR "Failed to build ROUP library")
    endif()
endif()

# Reuse accparser implementation (exclude parseOpenACC to avoid clash)
set(ACCPARSER_IR_SRC ${CMAKE_CURRENT_SOURCE_DIR}/accparser/src/OpenACCIR.cpp)
set(ACCPARSER_TOSTRING_SRC ${CMAKE_CURRENT_SOURCE_DIR}/accparser/src/OpenACCIRToString.cpp)

# Rename upstream parseOpenACC to avoid symbol collision
set_source_files_properties(${ACCPARSER_IR_SRC}
    PROPERTIES COMPILE_DEFINITIONS "parseOpenACC=accparser_legacy_parseOpenACC")

# Static compat library
add_library(roup-accparser-compat STATIC
    src/compat_impl.cpp
    ${ACCPARSER_IR_SRC}
    ${ACCPARSER_TOSTRING_SRC}
)

target_include_directories(roup-accparser-compat PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/accparser/src
    ${ROUP_ROOT}/src
)

# Link ROUP statically
if(UNIX AND NOT APPLE)
    set(EXTRA_LIBS pthread dl)
endif()

target_link_libraries(roup-accparser-compat PUBLIC
    ${ROUP_STATIC_LIB}
    ${EXTRA_LIBS}
)

# Shared library (drop-in replacement)
add_library(accparser SHARED
    src/compat_impl.cpp
    ${ACCPARSER_IR_SRC}
    ${ACCPARSER_TOSTRING_SRC}
)

target_include_directories(accparser PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/accparser/src
    ${ROUP_ROOT}/src
)

target_link_libraries(accparser PRIVATE
    ${ROUP_STATIC_LIB}
    ${EXTRA_LIBS}
)

set_target_properties(accparser PROPERTIES
    VERSION 0.1.0
    SOVERSION 0
    OUTPUT_NAME "accparser"
)

# Examples and tests
add_executable(acc_demo
    examples/basic_test.cpp
)
target_link_libraries(acc_demo accparser)

enable_testing()
add_test(NAME basic COMMAND acc_demo)

# Installation
install(TARGETS roup-accparser-compat accparser
    ARCHIVE DESTINATION lib
    LIBRARY DESTINATION lib
)

install(FILES
    ${CMAKE_CURRENT_SOURCE_DIR}/accparser/src/OpenACCIR.h
    ${CMAKE_CURRENT_SOURCE_DIR}/accparser/src/OpenACCKinds.h
    DESTINATION include
)
```

### Phase 4: Testing

**Test Strategy**:
1. **Unit tests**: Test ROUP C API directly
2. **Integration tests**: Test compat layer with simple directives
3. **accparser tests**: Run accparser's built-in test suite
   - tests/base/
   - tests/gpubootcamp/
   - tests/openacc-users-group/

**Success Criteria**:
- All accparser built-in tests pass
- 100% compatibility with accparser API
- Zero antlr4 dependency

### Phase 5: Documentation

**Files to Create**:
1. `compat/accparser/README.md` - Quick start guide
2. `docs/book/src/accparser-compat.md` - Full documentation
3. Update `docs/book/src/SUMMARY.md` - Add accparser chapter

**Topics to Cover**:
- Installation and build
- Drop-in replacement guide
- API compatibility notes
- Migration from ANTLR-based accparser
- Performance comparison
- Troubleshooting

## Special Considerations

### Special Directives

Some directives need special handling:

1. **cache directive**: Stores list of variables in expressions
2. **wait directive**: Can have parenthesized expression list
3. **end directive**: References paired directive
4. **routine directive**: Has optional name parameter

These may require special case handling in the C API or compat layer.

### Clause Modifiers

Some clauses have modifiers that need proper mapping:
- `copyin(readonly: ...)` - modifier + variables
- `copyout(zero: ...)` - modifier + variables
- `create(zero: ...)` - modifier + variables
- `reduction(+: sum)` - operator + variables
- `gang(num: 8)` - parameter
- `vector(length: 128)` - parameter
- `worker(num: 4)` - parameter

The C API needs to expose these modifiers and parameters.

## Risk Mitigation

### Risks

1. **Complex clause modifiers**: OpenACC has more complex clause syntax than OpenMP
2. **Special directive handling**: cache, wait, end, routine need special code
3. **Test coverage**: accparser has extensive test suite - must pass 100%
4. **Build system complexity**: Embedding static Rust library in C++ project

### Mitigation Strategies

1. **Incremental implementation**: Start with simple directives, add complexity gradually
2. **Comprehensive C API**: Expose all necessary data (modifiers, parameters, etc.)
3. **Reference implementation**: Study accparser's ANTLR grammar for edge cases
4. **Continuous testing**: Run accparser tests after each change

## Success Metrics

- ✅ All 20 ROUP test categories passing
- ✅ OpenACC round-trip tests passing (2/2)
- ⏳ OpenACC C API complete and tested
- ⏳ Compat layer compiles and links
- ⏳ Drop-in replacement works with accparser tests
- ⏳ 100% of accparser built-in tests passing
- ⏳ Zero antlr4 dependency
- ⏳ Documentation complete

## Timeline Estimate

| Phase | Effort | Status |
|-------|--------|--------|
| OpenACC Parser | ~2 days | ✅ Complete |
| OpenACC C API | ~2 days | 🚧 In Progress |
| Compat Layer Setup | ~1 day | ⏳ Pending |
| Implementation | ~2 days | ⏳ Pending |
| Testing | ~2 days | ⏳ Pending |
| Documentation | ~1 day | ⏳ Pending |
| **Total** | **~10 days** | **40% Complete** |

## References

### External Resources

- [accparser GitHub](https://github.com/ouankou/accparser)
- [OpenACC Specification](https://www.openacc.org/specification)
- [ANTLR 4](https://www.antlr.org/)

### Internal Resources

- `compat/ompparser/README.md` - Reference implementation
- `src/c_api.rs` - OpenMP C API implementation
- `src/parser/openacc.rs` - OpenACC parser
- Issue #67 - OpenACC support tracking

## Appendix: Comparison with ompparser

| Aspect | ompparser | accparser |
|--------|-----------|-----------|
| Directives | 17 | 21 |
| Clauses | ~12 basic | ~47 with modifiers |
| Special handling | Few | cache, wait, end, routine |
| Clause complexity | Simpler | More complex (modifiers, parameters) |
| C API complexity | Medium | Higher (more data to expose) |
| Test suite size | 46 tests | Larger (multiple test dirs) |
