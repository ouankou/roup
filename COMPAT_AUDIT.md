# Compat Layer Audit (Plan Step 1)

## Scope
- `src/c_api.rs` (OpenMP C API + shared FFI definitions)
- `src/c_api/openacc.rs` (OpenACC C API)
- `src/ir/convert.rs`, `src/ir/clause.rs`
- `src/parser/directive_kind.rs` plus clause/directive enums
- `compat/ompparser/src/compat_impl.cpp`
- `compat/accparser` bridge expectations
- `test.sh`

## Findings

### Post-parse string/number usage (violates "no raw string/num/char ops after parsing")
1. `src/c_api.rs:124-191` – `OmpDirective`, `OmpClause`, and iterators store C strings + `i32` codes instead of enum payloads. Downstream helpers (e.g., `directive_name_to_kind`, `convert_clause`) allocate `String`s for case-insensitive matching and manually parse clause arguments.
2. `src/c_api/openacc.rs:1-220` – OpenACC bridge stores directive names and clause arguments as `CString`s, extracts routine names via `trim()/strip_prefix`, and exposes `kind`/`modifier`/`operator` as bare `i32` constants.
3. `src/ir/convert.rs:1-200+` – Conversion layer still parses clause text (maps strings → enums) and handles normalization manually, meaning the IR is not yet purely enum-based coming out of the parser.
4. `compat/ompparser/src/compat_impl.cpp:1-220+` – Compat adapter is built entirely around helper functions such as `extract_clause_text`, `format_mapper`, `format_clause_args`, and `normalize_expression` that parse/patch strings before handing them to the `ompparser` IR. This must be deleted once the structured AST exists.

### Shared enums / insufficient language separation
1. `src/parser/directive_kind.rs` – Single `DirectiveName` enum covers OpenMP + OpenACC constructs, so parser cannot emit language-specific enum variants like `ROUP_OMPD_parallel` vs `ROUP_ACCD_parallel`.
2. `src/ir::DirectiveKind` mixes OpenMP constructs only; OpenACC directives are converted elsewhere instead of being part of a unified, language-prefixed enum family.
3. `src/roup_constants.h` (generated from `src/c_api.rs`) currently exposes overlapping numeric constants and does not follow the `ROUP_OMPD_*` / `ROUP_ACCD_*` naming/numbering convention.

### Clause normalization configuration gaps
- No shared configuration object exists to toggle clause normalization. `compat_impl.cpp` hardcodes formatting/merging logic, so we cannot match ompparser/accparser behavior per test group.

### Testing harness gap
- `test.sh:150-230` runs compat `ctest` but does not capture/pass through the failure counts/rates before exiting. Requirement: run the full compat suite first, abort immediately with failure statistics, and only continue to other sections if the suite is clean.

## Next Actions
- Use this audit to drive Step 2 (enum-based AST design + normalization toggles).
- Prepare updates to `test.sh` so compat `ctest` gatekeeping matches the CI requirement.
