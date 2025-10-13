# Constants Architecture: Single Source of Truth

## Problem

The ROUP C API uses integer codes for directives (0-16) and clauses (0-11). These codes need to be:
1. **Defined in Rust** (`src/c_api.rs`) - the actual implementation
2. **Available to C/C++** - for use in switch/case statements
3. **Maintained in one place** - to avoid duplication and drift

## Solution: Auto-Generated Header

### Single Source of Truth: `src/c_api.rs`

The constants are defined in two functions:

```rust
fn directive_name_to_kind(name: *const c_char) -> i32 {
    match name_str {
        "parallel" => 0,
        "for" => 1,
        // ... etc
    }
}

fn convert_clause(clause: &Clause) -> OmpClause {
    let (kind, data) = match clause.name.as_ref() {
        "num_threads" => (0, ...),
        "if" => (1, ...),
        // ... etc
    }
}
```

### Auto-Generation: `build.rs` + `src/constants_gen.rs`

The build script (`build.rs`) uses shared logic from `src/constants_gen.rs` for robust AST parsing:

**Dual-Mode Operation:**
- **Build mode** (`cargo build`): Generates header during compilation
- **Verification mode** (`cargo run --bin gen`): Validates existing header in CI

**How it works:**
1. `constants_gen.rs` parses `c_api.rs` as a Rust AST (immune to formatting changes)
2. Finds `directive_name_to_kind()` and `convert_clause()` functions
3. Extracts match arms: `"directive-name" => number`
4. Generates C header with `#define` constants
5. Creates checksum for validation (17 directives × 1000 + 12 clauses = 17012)

**Generated header:** `src/roup_constants.h`

```c
#define ROUP_CONSTANTS_CHECKSUM 17012  // Auto-validation

#define ROUP_DIRECTIVE_PARALLEL 0
#define ROUP_DIRECTIVE_FOR 1
// ... etc

#define ROUP_CLAUSE_NUM_THREADS 0
#define ROUP_CLAUSE_IF 1
// ... etc
```

### Usage in C/C++: `compat/ompparser/src/compat_impl.cpp`

```cpp
#include <roup_constants.h>

switch (roup_kind) {
    case ROUP_DIRECTIVE_PARALLEL: return OMPD_parallel;
    case ROUP_DIRECTIVE_FOR: return OMPD_for;
    // ... etc
}
```

## Maintenance Workflow

When adding a new directive or clause:

1. **Edit `src/c_api.rs` ONLY**:
   - Add new case to `directive_name_to_kind()` or `convert_clause()`
   - Assign next available integer code
   - Example: `"distribute" => 15,`

2. **Run `cargo build`**:
   - `build.rs` (in build mode) automatically parses the updated code
   - Uses shared `constants_gen` module for AST parsing
   - Regenerates `src/roup_constants.h` with new constants
   - C/C++ code automatically gets new `#define` values

3. **CI Validation**:
   - Runs `cargo run --bin gen` to verify header is up-to-date
   - Uses same `constants_gen` logic (zero code duplication)
   - Ensures header matches source code before merging

4. **No manual editing needed**:
   - ✅ Constants extracted via AST parsing (robust, format-independent)
   - ✅ Header auto-generated on every build
   - ✅ CI validates header synchronization
   - ❌ Never edit `roup_constants.h` manually (will be overwritten)
   - ❌ Never edit `build.rs` or `constants_gen.rs` unless changing parsing logic

## Benefits

✅ **No duplication**: Constants defined once in Rust  
✅ **Compile-time constants**: C++ can use in switch/case  
✅ **Fully automated**: AST parsing extracts constants automatically  
✅ **Format-independent**: Uses `syn` crate (proper AST parser, not string matching)  
✅ **Type-safe**: Rust compiler validates the codes  
✅ **Maintainable**: Single source of truth in `c_api.rs`  
✅ **Validated**: Checksum ensures synchronization (17012 = 17 directives + 12 clauses)

## Implementation Details

**Shared Module:** `src/constants_gen.rs` (~320 lines) contains all AST parsing logic:
- Public API: `parse_directive_mappings()`, `parse_clause_mappings()`, `generate_header()`, `calculate_checksum()`, `extract_checksum_from_header()`
- Uses `syn` crate for robust AST parsing
- Recursively searches for match expressions in functions
- Handles unsafe blocks, nested expressions
- Extracts `"string" => number` patterns from match arms
- Immune to code formatting, whitespace, or comment changes

**Dual-Mode build.rs:**
- **Build Script Mode** (during `cargo build`): Detects `OUT_DIR` env var, generates header
- **Standalone Binary** (via `cargo run --bin gen`): Validates existing header against source

**Benefits of Shared Module:**
- ✅ Zero code duplication (~200 lines saved)
- ✅ Single source of truth for parsing logic
- ✅ Same validation in build and CI
- ✅ Robust AST parsing (not fragile string matching)

**Previously:** Manual maintenance required updating both `c_api.rs` and `build.rs`  
**Now:** Edit `c_api.rs` only; constants auto-extracted via AST, validated in CI
