# ompparser Compatibility Layer - Implementation Status

## Current Status

**Test Pass Rate: 4% (5/140 tests passing)**

### ✅ Completed

1. **Complete Directive Kind Mapping (86 directives)**
   - All OpenMP 5.x directives: parallel, for, do, simd, sections, single, task, etc.
   - All combined directives: parallel for, teams distribute parallel for, etc.
   - All target directives: target, target teams, target parallel, etc.
   - All Fortran variants: parallel do, do simd, distribute parallel do, etc.
   - Special directives: loop, scan, tile, unroll, metadirective, etc.
   - File: `src/c_api.rs` directive_name_to_kind() function
   - File: `compat/ompparser/src/compat_impl.cpp` mapRoupToOmpparserDirective()

2. **Test Infrastructure**
   - Integrated all 140 ompparser tests into CMake build system
   - Created run_ompparser_tests.sh script
   - Fixed setLang() linkage (C++ not extern "C")
   - File: `compat/ompparser/CMakeLists.txt`

### 🚧 In Progress

3. **Clause Parameter Extraction**

   **Problem**: Directives are recognized correctly, but clause parameters are missing.

   Example failures:
   ```
   INPUT:    #pragma omp parallel private (a, b)
   OUTPUT:   #pragma omp parallel private
   EXPECTED: #pragma omp parallel private (a, b)
   ```

   **Root Cause**: ROUP C API only exposes clause kinds (int32_t), not clause arguments.

   **Required Changes**:
   - Add C API functions to extract clause arguments (variables, expressions)
   - Expose clause parameter data from `ClauseKind::Parenthesized(String)`
   - Pass parameters to ompparser's `addOpenMPClause()` variadic method

### 📋 Remaining Work

4. **Complete Clause Kind Mappings (90+ clauses)**

   Currently mapped: 12 clauses (num_threads, if, private, shared, etc.)

   Need to add: 80+ additional clauses including:
   - copyin, align, proc_bind, allocate, num_teams, thread_limit
   - linear, safelen, simdlen, aligned, nontemporal
   - uniform, inbranch, notinbranch, dist_schedule, bind
   - device, map, depend, priority, affinity, detach
   - And 60+ more...

   Files to update:
   - `src/c_api.rs` convert_clause() function
   - `compat/ompparser/src/compat_impl.cpp` mapRoupToOmpparserClause()

5. **Clause Parameter Passing**

   For each clause type, extract and pass appropriate parameters:

   - **Variable lists** (private, shared, reduction, etc.)
     - Extract from ROUP clause arguments
     - Parse comma-separated variable names
     - Pass to ompparser as variable list

   - **Expressions** (num_threads, if, collapse, etc.)
     - Extract expression string from ROUP
     - Pass to ompparser's expression parser

   - **Special clause types**:
     - `schedule(kind, chunk)` - schedule policy + optional chunk size
     - `reduction(operator: vars)` - operator + variable list
     - `default(kind)` - data sharing attribute
     - `proc_bind(policy)` - thread affinity policy
     - `map(type: vars)` - map type + variable list
     - And many more...

6. **Add ROUP C API Functions**

   New functions needed in `src/c_api.rs`:
   ```rust
   // Get clause argument string (for Parenthesized clauses)
   pub extern "C" fn roup_clause_arguments(clause: *const OmpClause) -> *const c_char;

   // Free clause argument string
   pub extern "C" fn roup_clause_arguments_free(args: *const c_char);

   // Check if clause has arguments
   pub extern "C" fn roup_clause_has_arguments(clause: *const OmpClause) -> i32;
   ```

7. **Update compat_impl.cpp Clause Handling**

   Current code (simplified):
   ```cpp
   dir->addOpenMPClause(static_cast<int>(clause_kind));
   ```

   Needs to become (example for different clause types):
   ```cpp
   switch (clause_kind) {
       case OMPC_private: {
           const char* args = roup_clause_arguments(roup_clause);
           std::vector<std::string> vars = parseVariableList(args);
           dir->addOpenMPClause(OMPC_private, vars);
           roup_clause_arguments_free(args);
           break;
       }
       case OMPC_num_threads: {
           const char* expr = roup_clause_arguments(roup_clause);
           dir->addOpenMPClause(OMPC_num_threads, expr);
           roup_clause_arguments_free(expr);
           break;
       }
       // ... and so on for each clause type
   }
   ```

## Test Results

### Passing Tests (5/140)
- `compat_basic` - Basic compatibility test
- `ompparser_drop_in` - Drop-in replacement test
- `comprehensive` - Comprehensive test
- `ompparser_barrier` - Barrier directive (no clauses)
- `ompparser_parallel_workshare` - Parallel workshare (minimal clauses)

### Common Failure Patterns
1. **Missing clause parameters**: Clauses detected but arguments not extracted
2. **Unknown clauses**: Clauses not in mapping (returns OMPC_unknown)
3. **Merged clauses**: Multiple identical clauses not merged (e.g., two `private` clauses)

## Implementation Roadmap

### Phase 1: Complete Clause Mappings (Est: 2 hours)
- Generate complete clause mapping for c_api.rs (90+ clauses)
- Update compat_impl.cpp with complete clause enum mapping
- Rebuild and test (expected: still ~4% pass rate)

### Phase 2: Expose Clause Arguments from ROUP (Est: 4 hours)
- Add roup_clause_arguments() C API function
- Modify OmpClause structure to store argument strings
- Update convert_clause() to preserve argument data
- Test argument extraction

### Phase 3: Parse and Pass Clause Parameters (Est: 8 hours)
- Implement parseVariableList() helper in compat_impl.cpp
- Handle each clause type individually (90+ cases)
- Pass correct parameters to addOpenMPClause()
- Handle special cases:
  - Clause merging (multiple private clauses → one with all vars)
  - Expression parsing (num_threads, if, collapse)
  - Complex clauses (reduction, map, schedule)

### Phase 4: Testing and Validation (Est: 4 hours)
- Run all 140 tests
- Fix failures one by one
- Achieve 100% pass rate
- Document any limitations

**Total Estimated Time: 18 hours**

## Files Modified So Far

1. `src/c_api.rs` - Complete directive mapping (86 directives)
2. `compat/ompparser/src/compat_impl.cpp` - Simplified directive mapping
3. `compat/ompparser/src/roup_compat.h` - Fixed setLang() declaration
4. `compat/ompparser/CMakeLists.txt` - Added test suite integration
5. `compat/ompparser/run_ompparser_tests.sh` - Test runner script

## Key Technical Insights

1. **ROUP parser already recognizes all directives** - The parser in `src/parser/openmp.rs` has 132 directive definitions covering all of OpenMP 5.x

2. **Directive mapping is complete** - All 86 directive enum values now match ompparser exactly

3. **The bottleneck is clause parameters** - ROUP parses clauses successfully but the C API doesn't expose their arguments

4. **ompparser's strength is reconstruction** - It has sophisticated toString() methods that can reconstruct pragmas IF given complete clause data

5. **No shortcuts possible** - To achieve 100% pass rate, must implement full parameter extraction for all 90+ clause types

## Next Steps

The immediate next task is to implement Phase 2: expose clause arguments from ROUP C API.

This requires:
1. Modify OmpClause structure to include argument string
2. Add roup_clause_arguments() function
3. Update convert_clause() to store arguments
4. Test argument extraction with simple clauses

Once clause arguments are exposed, Phase 3 can begin: parsing and passing parameters to ompparser.
