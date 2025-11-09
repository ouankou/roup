# OMPParser Compatibility Layer Status

## Overview
This document tracks the progress toward achieving 100% compatibility between ROUP and ompparser test suite (136 tests with 1500+ individual test cases).

## Current Status
**Pass Rate:** 4/136 tests (2.9%)
**Branch:** `feat/ompparser-compat-upgrade`
**Last Updated:** 2025-11-09

## Completed Work

### ✅ Directive Mapping (Complete)
- **C API Enhancement:** Expanded `directive_name_to_kind()` in `src/c_api.rs` to map ALL 132 OpenMP directives to unique integer codes (0-131)
- **Constants Generation:** Updated `src/constants_gen.rs` to include composite directives (e.g., "parallel for" → `ROUP_DIRECTIVE_PARALLEL_FOR`)
- **Compat Layer Mapping:** Updated `compat/ompparser/src/compat_impl.cpp` `mapRoupToOmpparserDirective()` to map 86 supported directives to ompparser enums
- **Result:** All major directive types now properly recognized (improved from 17 → 132 unique directive codes)

### ✅ Basic Infrastructure
- Fixed `setLang()` linkage (removed `extern "C"` to match ompparser C++ API)
- Generated comprehensive `roup_constants.h` with 133 directive constants
- Established mapping framework for all directives

## Remaining Work for 100% Pass Rate

### 1. Clause Parameter Extraction (CRITICAL - ~80% of failures)
**Problem:** Clause parameters are not being extracted from ROUP to ompparser

**Current Behavior:**
```c
// Input:  #pragma omp parallel private(a, b, c)
// Output: #pragma omp parallel private       // ❌ Missing variables!
```

**Required Changes:**
- [ ] Modify ROUP C API `convert_clause()` to export clause parameters
- [ ] Update `OmpClause` struct in `src/c_api.rs` to include variable lists, expressions
- [ ] Implement clause-specific data structures:
  - Variable lists (private, shared, firstprivate, etc.)
  - Reduction operators and variables
  - Schedule kinds and chunk sizes
  - Default clause values (shared/none/private)
  - Map clause types and variables

**Files to Modify:**
- `src/c_api.rs`: `convert_clause()`, `ClauseData` union
- `compat/ompparser/src/compat_impl.cpp`: `parseOpenMPDirective()` clause conversion

### 2. Clause Kind Mapping (Medium Priority)
**Current:** Only 12 clause kinds mapped
**Required:** Map all 79 OpenMP clause kinds

**Action Items:**
- [ ] Expand `convert_clause()` in `src/c_api.rs` to return all clause kinds
- [ ] Update `mapRoupToOmpparserClause()` in compat_impl.cpp
- [ ] Add missing clause kinds (allocate, copyin, proc_bind, etc.)

### 3. Fix Segmentation Faults (~9 tests)
**Cause:** Memory issues in directive creation or clause handling

**Known Issues:**
- Critical directive with named locks
- Flush directive tests
- Directives with complex clause combinations

**Action Items:**
- [ ] Add null pointer checks in compat_impl.cpp
- [ ] Verify OpenMPDirective allocation/deallocation
- [ ] Debug with valgrind for memory leaks

### 4. Default Clause Value Handling
**Problem:** Default clause always shows "shared" instead of actual value

**Example:**
```c
// Input:  #pragma omp parallel default(none)
// Output: #pragma omp parallel default(shared)  // ❌ Wrong value!
```

**Action Items:**
- [ ] Parse default clause value in convert_clause()
- [ ] Map to ompparser DefaultKind enum (shared/none/private/firstprivate)

### 5. Combined Directive Decomposition
Some tests expect ompparser to decompose combined directives:
```c
#pragma omp parallel for → { OMPD_parallel, OMPD_for }
```

**Action Items:**
- [ ] Investigate ompparser's directive decomposition behavior
- [ ] Implement in compat layer if required for tests

## Test Failure Analysis

### Passing Tests (4/136)
1. `atomic_clause_token` - Atomic directive with clause tokens
2. `single_clause_token` - Single directive with clause tokens
3. `aligned_clause_token` - SIMD aligned clause
4. _(one more)_

### High-Impact Failures
| Test Category | Count | Root Cause |
|--------------|-------|------------|
| Clause parameters missing | ~110 | No variable/expression extraction |
| Segmentation faults | ~9 | Memory management issues |
| Wrong clause values | ~10 | Default/schedule/reduction value extraction |
| Unknown directives | ~3 | Directives not in ompparser spec |

## Implementation Priority

### Phase 1: Clause Parameter Extraction (Highest ROI)
Implementing variable list extraction will fix ~80% of remaining failures.

**Estimated Impact:** 4 → 100+ passing tests

**Key Implementation:**
1. Add `variables` field to OmpClause in c_api.rs
2. Extract variables from ROUP's `ClauseKind::VariableList`
3. Convert to ompparser's variable list format

### Phase 2: Clause Value Extraction
Extract schedule kinds, default values, reduction operators

**Estimated Impact:** +20 passing tests

### Phase 3: Segfault Fixes
Debug and fix memory issues

**Estimated Impact:** +9 passing tests

### Phase 4: Edge Cases
Handle special directives and rare clause combinations

**Estimated Impact:** +3 passing tests

## Technical Debt

### Code Quality
- [ ] Add comprehensive error handling
- [ ] Document compat layer architecture
- [ ] Add unit tests for mapping functions

### Performance
- [ ] Profile clause conversion overhead
- [ ] Optimize repeated string allocations

## References
- OMPParser tests: `compat/ompparser/ompparser/tests/` (136 test files)
- ROUP C API: `src/c_api.rs`
- Compat implementation: `compat/ompparser/src/compat_impl.cpp`
- OpenMP spec: OpenMPKinds.h (116 directives, 79 clauses)

## Next Steps
1. Implement clause parameter extraction in `src/c_api.rs`
2. Test with parallel.txt and private/shared/reduction tests
3. Iterate on clause value extraction
4. Fix segmentation faults with valgrind
5. Run full test suite until 100% pass rate

---
**Note:** This is a significant engineering effort requiring deep changes to both ROUP's C API and the compat layer. The directive mapping foundation is now solid; clause parameter extraction is the critical path to 100% compatibility.
