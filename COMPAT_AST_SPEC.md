# Enum-Based AST Specification (Plan Step 2)

This document defines the enum-based AST that replaces every post-parse string/number operation. The design satisfies:
- Unique enum per language and keyword (`ROUP_OMPD_*`, `ROUP_OMPC_*`, `ROUP_ACCD_*`, `ROUP_ACCC_*`, etc.)
- No raw strings/numbers outside the parsing layer
- Configurable clause normalization matching `ompparser` / `accparser`
- Minimal unsafe Rust confined to FFI shims

## 1. Top-Level Objects

### 1.1 Language Enum
```rust
#[repr(C)]
pub enum RoupLanguage {
    OpenMP,
    OpenACC,
}
```

### 1.2 Directive Wrapper
```rust
#[repr(C)]
pub struct RoupDirective {
    pub language: RoupLanguage,
    pub source: SourceLocation,
    pub omp: Option<OmpDirective>,
    pub acc: Option<AccDirective>,
}
```
- Exactly one of `omp` or `acc` is `Some`.
- Each directive struct carries kind enum, modifier flags, parameter AST, and clause list.

## 2. OpenMP AST

### 2.1 Directive Kind
```rust
#[repr(C)]
pub enum OmpDirectiveKind {
    ROUP_OMPD_parallel,
    ROUP_OMPD_parallel_for,
    ROUP_OMPD_parallel_for_simd,
    // ... one variant per OpenMP directive in ompparser/openmp_vv suites
    ROUP_OMPD_end_parallel,             // end directives get their own variants
    ROUP_OMPD_declare_mapper,
    ROUP_OMPD_declare_simd,
    // etc.
}
```
- Naming mirrors ompparser tokens (spaces converted to `_`).
- End directives stay explicit (`ROUP_OMPD_end_do_simd`, etc.).
- Parser populates these enums directly; no string lookups elsewhere.

### 2.2 Clause Kind
```rust
#[repr(C)]
pub enum OmpClauseKind {
    ROUP_OMPC_private(OmpIdentifierList),
    ROUP_OMPC_shared(OmpIdentifierList),
    ROUP_OMPC_in_reduction(InReductionClause),
    ROUP_OMPC_bind(BindClause),
    ROUP_OMPC_schedule(ScheduleClause),
    ROUP_OMPC_map(MapClause),
    ROUP_OMPC_reduction(ReductionClause),
    // ... one variant per clause in ompparser suite
}
```
- Clause payload structs carry structured data (operators, modifiers, identifier lists, expressions).
- No `kind: i32` or raw `arguments: *const c_char`.

### 2.3 Normalization Hooks
- Clause lists stored as `Vec<OmpClauseKind>`.
- A `ClauseNormalizationMode` enum determines whether compatible clauses merge.
```rust
pub enum ClauseNormalizationMode {
    Disabled,
    MergeVariableLists,    // e.g., shared(a) shared(b) -> shared(a, b)
    OmpParserParity,       // mimic ompparser default
}
```
- `ParserConfig` gains `clause_normalization: ClauseNormalizationMode`.
- Compat layer sets the mode based on the active test group before invoking the parser.

## 3. OpenACC AST

### 3.1 Directive Kind
```rust
#[repr(C)]
pub enum AccDirectiveKind {
    ROUP_ACCD_parallel,
    ROUP_ACCD_parallel_loop,
    ROUP_ACCD_kernels,
    ROUP_ACCD_serial_loop,
    ROUP_ACCD_wait,
    ROUP_ACCD_end,
    // ... exactly the directives covered by accparser/openacc_vv
}
```

### 3.2 Clause Kind
```rust
#[repr(C)]
pub enum AccClauseKind {
    ROUP_ACCC_copyin(CopyClause),
    ROUP_ACCC_copyout(CopyClause),
    ROUP_ACCC_create(CreateClause),
    ROUP_ACCC_default(AccDefaultClause),
    ROUP_ACCC_wait(AccWaitClause),
    ROUP_ACCC_reduction(AccReductionClause),
    // ... one variant per clause keyword
}
```

### 3.3 Language Separation
- No OpenMP clause/dir enum values appear in the OpenACC namespace and vice versa.
- Numeric constants generated for headers use non-overlapping ranges automatically.

## 4. Parser Output → IR → C API Flow

1. `src/parser` emits `RoupDirective` instances directly (no intermediate strings). This replaces the current `Directive`/`Clause` textual structs for compatibility builds.
2. `src/ir` stores the enums without additional parsing. Clause/identifier lists become dedicated structs (e.g., `IdentifierList { items: Vec<Identifier> }`).
3. `src/c_api.rs` exposes `#[repr(C)]` wrappers mirroring the enums; only conversion is pointer ownership.
4. `compat/ompparser` / `compat/accparser` walk the enums and instantiate ompparser/accparser IR nodes 1:1. All helper functions that previously massaged strings are removed.

## 5. Normalization Configuration

- `ParserConfig` gets:
```rust
pub struct ParserConfig {
    // existing fields...
    pub clause_normalization: ClauseNormalizationMode,
}
```
- Default: `ClauseNormalizationMode::OmpParserParity`.
- `compat` exposes APIs to toggle the mode per test group (e.g., `roup_config_set_clause_normalization(ROUP_NORMALIZE_OMP)` / `ROUP_NORMALIZE_DISABLED`).
- Both compat layers propagate the mode so that ompparser/accparser receive ASTs matching their expected normalization behavior.

## 6. Unsafe Rust Boundaries

- Only the external `#[no_mangle]` functions manipulating raw pointers remain unsafe.
- All AST creation/conversion happens in safe Rust using enums and typed payloads.

## 7. Follow-Up Tasks
- Generate updated headers (`src/roup_constants.h`) reflecting the new enums with explicit prefixes.
- Update build scripts and tests to assert that every `DirectiveName`/`ClauseName` variant has a matching enum entry with a documented numeric ID.
