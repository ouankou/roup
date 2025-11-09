# ROUPMPPARSER Compatibility Layer - Status and Rules

## Goal

Make ROUP a drop-in replacement for ompparser library with 100% test compatibility (currently 46%, target 100%).

## Current Status

**Test Results:** 708/1527 passing (46%)
- Builtin tests: 5/136 passing (4%)
- Atomic tests: 28/30 passing (93%)
- Allocate tests: 9/10 passing (90%)

**Critical Problems:**
1. Directive parameters (e.g., `critical(name)`) completely dropped
2. Combined directives parsed incorrectly (e.g., `parallel for` → `parallel`)
3. Fortran parsing failures (indentation issues)
4. Generic clause handling broken (many clauses fall through to Generic/Unknown)
5. Memory corruption bug: allocator clause duplicates when align clause comes first

## HARD RULES - NEVER VIOLATE

### Rule 1: NO String/Number Comparisons Outside Parser Layer

**WRONG - NEVER DO THIS:**
```rust
// In C API or IR layer - FORBIDDEN
match identifier.name() {
    "read" => clause_kind::READ,      // STRING COMPARISON - WRONG!
    "write" => clause_kind::WRITE,
    _ => ...
}

if clause_kind == 47 { ... }           // HARDCODED NUMBER - WRONG!
```

**CORRECT:**
```rust
// Parser layer ONLY - string to enum
match name {
    "read" => Some(OpenMpClause::Read),  // OK - parser layer
    _ => None
}

// IR/C API layers - enum to enum
match clause.variant {
    Some(OpenMpClause::Read) => Ok(ClauseData::Read),  // OK - enum only
    _ => ...
}
```

### Rule 2: Use Symbolic Constants Only

**WRONG:**
```cpp
if (kind == 47) { ... }                    // HARDCODED NUMBER
case 42: return OMPC_seq_cst;              // HARDCODED NUMBER
```

**CORRECT:**
```cpp
if (kind == ROUP_CLAUSE_KIND_READ) { ... }              // SYMBOLIC CONSTANT
case ROUP_CLAUSE_KIND_SEQ_CST: return OMPC_seq_cst;     // SYMBOLIC CONSTANT
```

### Rule 3: String Conversion Happens ONCE

**Flow:**
1. **Parser** (src/parser/openmp.rs): `"read"` → `OpenMpClause::Read` (ONLY place with strings)
2. **IR** (src/ir/convert.rs): `OpenMpClause::Read` → `ClauseData::Read` (enum → enum)
3. **C API** (src/c_api.rs): `ClauseData::Read` → `clause_kind::READ` (enum → constant)
4. **Compat** (compat_impl.cpp): `ROUP_CLAUSE_KIND_READ` → `OMPC_read` (constant → constant)

**No strings or numbers anywhere except step 1.**

## Architecture Layers

### Layer 1: Parser (src/parser/)
- Parses text into AST
- Stores `variant: Option<OpenMpClause>` on each Clause
- `OpenMpClause::from_name()` converts string → enum (ONLY string comparison point)

### Layer 2: IR Conversion (src/ir/convert.rs)
- Converts AST → IR using `clause.variant` (enum matching ONLY)
- Creates `ClauseData` enum variants
- NO string comparisons allowed

### Layer 3: C API (src/c_api.rs)
- Converts IR `ClauseData` → C-compatible `OmpClause`
- Pattern match on ClauseData enum variants
- Maps to `clause_kind::*` constants
- NO string comparisons allowed

### Layer 4: Compat Layer (compat/ompparser/src/compat_impl.cpp)
- Converts ROUP types → ompparser types
- Uses `ROUP_CLAUSE_KIND_*` → `OMPC_*` mappings
- Calls ompparser APIs to construct directives
- NO hardcoded numbers allowed

## Critical Issues

### Issue 1: Directive Parameters Dropped

**Problem:**
```c
#pragma omp critical(test1) hint(test2)
→ #pragma omp critical
```
Name parameter `(test1)` completely lost.

**Root Cause:**
- Parser stores `parameter: Option<Cow<'a, str>>` in Directive struct
- IR conversion ignores parameter field
- OmpDirective struct has no parameter field
- Compat layer never receives parameter

**Solution:**
1. Add `parameter: *const c_char` to OmpDirective struct (src/c_api.rs:156)
2. Add `pub extern "C" fn roup_directive_parameter()` to C API
3. In compat layer, call `roup_directive_parameter()` and pass to ompparser
4. For critical: call `OpenMPCriticalDirective::setCriticalName()`
5. Update all directive types that use parameters (critical, atomic, etc.)

### Issue 2: Combined Directives Parsed Wrong

**Problem:**
```c
#pragma omp parallel for
→ DirectiveKind::Parallel (wrong - should handle both)
```

**Root Cause:**
- ROUP parses combined directives as single DirectiveKind
- ompparser expects base directive + modifying clauses
- No mapping between ROUP's combined kinds and ompparser's representation

**Solution:**
1. Add special handling in compat layer for combined directives:
   - `OMPD_parallel_for` → Create `OpenMPParallelDirective` + add implied "for" construct
   - Or: map to ompparser's combined directive classes if they exist
2. Check ompparser source for how it handles combined directives
3. May need to create wrapper directive that contains multiple ompparser directives

### Issue 3: Many Clauses Fall Through to Generic/Unknown

**Problem:**
Many clause types not handled, fall through to Generic/Unknown, then compat layer can't process them.

**Missing Clause Types:**
- map, depend, device, device_type, defaultmap
- use_device_ptr, use_device_addr, is_device_ptr, has_device_addr
- in_reduction, task_reduction
- to, from, link
- And ~50 more

**Solution:**
1. For each missing clause, add IR ClauseData variant (src/ir/clause.rs)
2. Add IR conversion (src/ir/convert.rs)
3. Add C API conversion (src/c_api.rs)
4. Add clause_kind constant (src/constants_gen.rs)
5. Add compat mapping (compat_impl.cpp)

**Pattern to follow:**

```rust
// 1. Add to ClauseData enum (src/ir/clause.rs)
pub enum ClauseData {
    ...
    /// `map(modifier: list)` - Map variables to device
    Map {
        modifier: Option<MapModifier>,
        items: Vec<ClauseItem>,
    },
}

// 2. Add IR conversion (src/ir/convert.rs)
Some(OpenMpClause::Map) => {
    // Parse parenthesized content into modifier + items
    // Return ClauseData::Map { ... }
}

// 3. Add C API conversion (src/c_api.rs)
Map { modifier, items } => OmpClause {
    kind: clause_kind::MAP,
    data: ClauseData {
        items: items as *const Vec<ClauseItem>,
    },
}

// 4. Add constant (src/constants_gen.rs)
("MAP", 52),

// 5. Add mapping (compat_impl.cpp)
case ROUP_CLAUSE_KIND_MAP: return OMPC_map;
```

### Issue 4: Allocator Clause Duplication Bug

**Problem:**
```c
#pragma omp allocate(x) align(3) allocator(my_alloc)
→ #pragma omp allocate (x) align(3) allocator (omp_default_mem_allocmy_alloc)
```
Allocator identifier duplicated with garbage when align comes first.

**Root Cause:**
Memory corruption or state leakage in compat layer clause processing. When align clause is processed, it corrupts state that affects subsequent allocator clause.

**Solution:**
1. Check if `addLangExpr()` appends vs replaces
2. Verify allocator clause handler doesn't free string incorrectly (currently NOT freeing - line 538-539)
3. Check if ompparser reuses clause buffer
4. Add debug logging to trace exact values being set
5. May need to copy string instead of passing pointer

### Issue 5: Fortran Indentation Not Parsed

**Problem:**
```fortran
!$OMP   PARALLEL DO
→ Parse failed: NULL directive
```

**Root Cause:**
Fortran parser expects `!$OMP` prefix without extra spaces, but ompparser tests have indented directives.

**Solution:**
1. Update Fortran parser to handle leading whitespace before `!$OMP`
2. Update parser to handle spaces between `!$OMP` and directive name
3. Or: preprocess input to strip indentation (less ideal)

### Issue 6: Bare Clause Handling Incomplete

**Problem:**
Generic bare clauses (nogroup, untied, mergeable) all map to `clause_kind::GENERIC`, losing specific type.

**Solution:**
1. Add specific constants for each bare clause type
2. Add specific ClauseData variants or expand Bare to include clause-specific enum
3. Update compat layer to handle GENERIC clauses by name

## Missing Features Needed

### 1. Directive Variable Lists
Many directives have variable lists in directive itself, not as clauses:
- `allocate(var1, var2)` - variables in directive, not clause
- `threadprivate(var1, var2)`
- `declare target(var1, var2)`

Currently handled ad-hoc in compat layer (lines 557-620). Need systematic approach.

### 2. Clause Modifiers
Many clauses have modifiers that aren't handled:
- `map(to: x)` - "to" is modifier
- `reduction(+: x)` - "+" is modifier (handled)
- `schedule(static, 10)` - "static" is modifier (handled)
- `depend(in: x)` - "in" is modifier

Need to parse and preserve modifiers in IR.

### 3. Expression Evaluation
Some clauses need expression evaluation:
- `if(condition)` - complex expressions
- `num_threads(expr)`

Currently store as string in Expression. May need to preserve expression tree or evaluate.

### 4. Structured Data
Some directives have complex structured data:
- `declare variant(...)` - complex matching rules
- `requires(...)` - multiple requirements

Need proper IR representation.

## Systematic Fix Approach

### Phase 1: Infrastructure (CRITICAL - DO FIRST)
1. Add directive parameter support (critical, atomic, etc.)
2. Fix Fortran indentation parsing
3. Add comprehensive clause kind constants for ALL OpenMP clauses
4. Fix allocator duplication bug

### Phase 2: Clause Coverage (HIGH PRIORITY)
For each missing clause type:
1. Add ClauseData variant
2. Add IR conversion
3. Add C API conversion
4. Add compat mapping
5. Add test

**Priority order (most common first):**
- device, map, depend
- defaultmap, use_device_ptr, is_device_ptr
- in_reduction, task_reduction
- to, from, link

### Phase 3: Combined Directives (MEDIUM PRIORITY)
1. Map ROUP combined directives to ompparser representation
2. Test all combined forms (parallel for, target teams distribute, etc.)

### Phase 4: Advanced Features (LOW PRIORITY)
1. Clause modifiers
2. Complex structured data
3. Expression trees

## Testing Strategy

### Current Test Breakdown
- `builtin_*` tests (136 tests): Basic directive/clause combinations - most fail
- `openmp_vv_*` tests (~1000 tests): Validation suite - many fail
- `openmp_examples_*` tests (~400 tests): Example code - many fail

### Fix Verification
After each fix:
1. Run specific test: `ctest -R "builtin_<directive>"`
2. Check test improvement
3. Run full suite: `ctest 2>&1 | grep "% tests"`
4. Track overall percentage increase

### Debug Commands
```bash
# Test single directive
echo "#pragma omp critical(test)" | ./ompparser/tests/tester /dev/stdin

# Run specific test with output
ctest -R "builtin_critical" --output-on-failure

# Check test file
cat ompparser/tests/builtin/critical.txt

# Rebuild
cargo build --release
cmake --build . --target all
```

## File Locations

### ROUP Core
- `src/parser/openmp.rs` - OpenMP clause definitions, `from_name()` (ONLY string comparison)
- `src/parser/clause.rs` - Clause struct with variant field
- `src/ir/clause.rs` - ClauseData enum (add new variants here)
- `src/ir/convert.rs` - AST → IR conversion (enum matching ONLY)
- `src/c_api.rs` - IR → C conversion (enum → constant ONLY)
- `src/constants_gen.rs` - Clause kind constants (add new constants here)

### Compat Layer
- `compat/ompparser/src/compat_impl.cpp` - Main compat implementation
- `compat/ompparser/build/` - Build directory (cmake)
- `compat/ompparser/ompparser/tests/builtin/` - Test input files

### Generated Files
- `src/roup_constants.h` - Auto-generated C header (don't edit directly)
- `target/release/libroup.a` - ROUP library (compat links against this)

## Common Mistakes to Avoid

### ❌ WRONG: String comparison in C API
```rust
// src/c_api.rs - NEVER DO THIS
match identifier.name() {
    "read" => clause_kind::READ,  // STRING COMPARISON!
    _ => ...
}
```

### ✅ CORRECT: Add ClauseData variant
```rust
// src/ir/clause.rs
pub enum ClauseData {
    Read,  // Add enum variant
    ...
}

// src/c_api.rs
Read => OmpClause {  // Pattern match enum
    kind: clause_kind::READ,
    ...
}
```

### ❌ WRONG: Hardcoded numbers
```cpp
if (kind == 47) { ... }
case 42: return OMPC_seq_cst;
```

### ✅ CORRECT: Symbolic constants
```cpp
if (kind == ROUP_CLAUSE_KIND_READ) { ... }
case ROUP_CLAUSE_KIND_SEQ_CST: return OMPC_seq_cst;
```

### ❌ WRONG: Modifying compat layer without updating IR
```cpp
// Adding special case in compat layer for clause not in IR
// This is a band-aid, not a fix
```

### ✅ CORRECT: Add proper IR support first
```rust
// 1. Add ClauseData variant
// 2. Add IR conversion
// 3. Add C API conversion
// 4. Then add compat handling
```

## Next Steps (Priority Order)

1. **Fix directive parameters** - 30+ tests blocked on this
2. **Add missing clause types** - 100+ tests blocked on this
3. **Fix combined directives** - 50+ tests blocked on this
4. **Fix allocator duplication** - 1 test blocked
5. **Fix Fortran parsing** - 200+ tests blocked
6. **Add clause modifiers** - 50+ tests need this

**Target:** 100% test pass rate (1527/1527)

**Current:** 46% (708/1527)

**Gap:** 819 tests still failing
