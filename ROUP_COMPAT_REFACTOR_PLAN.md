# ROUP → compat Refactor Plan (enum AST, no post-parse strings)

## Goals (unchanged)
- Single parse → typed enum AST (OpenMP/OpenACC); no post-parse string/number parsing.
- Drop-in compat: keep upstream ompparser/accparser ABI/headers (`parseOpenMP`/`parseOpenACC` only), no submodule edits.
- 100% `ctest` pass (OpenMP + OpenACC) and `test.sh` gating.
- `unsafe` only at FFI boundary.

## Current Status (Nov 2025)
- AST/IR: Unknown clauses are fatal; device modifiers and depobj_update dependencies are typed enums; requires/uses_allocators/reduction/device/defaultmap/map/depend/allocator/etc. are structured. Metadirective selectors are stored as `ClauseData::MetadirectiveSelector` with a `raw` string; typed selector parsing is not implemented yet.
- C API (runtime): Clause conversions are AST-based; typed getters exist for reduction, defaultmap, uses_allocators, requires (modifier list), device (modifier + expr), depobj_update (dependence), device_type, bind/order modifiers, map/defaultmap kinds, allocators, grainsize/num_tasks strict+expr. Unknown clauses panic. Metadirective selectors still surface as raw argument strings (currently empty in compat due to missing wiring).
- Constants/header: Legacy `convert_clause` removed. `clause_name_to_kind_for_constants` drives constants generation; `roup_constants.h` regenerated via `cargo build`. (H1 complete; hard-coded block removed and build passing.)
- Compat: Most OpenMP clause handlers now consume typed getters (bind, order, atomic_default_mem_order, grainsize/num_tasks strict+expr, device_type, requires, uses_allocators, device, depobj_update, map/defaultmap, etc.). Variant/metadirective selectors still rely on `arguments` text and currently come through empty, causing `builtin_metadirective` ctest failures. OpenACC compat still string-based.
- Harness: `test.sh` runs the full suite but ompparser `ctest` currently fails (builtin_metadirective selectors rendered empty). Gating order change (compat-first, fail fast) is pending.

## Immediate Work (priority order)
1) **Metadirective selectors (S1):** Wire selector text through C API so compat gets non-empty arguments; then replace raw strings with typed selector parsing/getters and update compat accordingly.
2) **Normalization & gating:** Wire normalization toggles to match ompparser/accparser expectations per test group; update `test.sh` to run compat `ctest` first, report failures, and abort early.
3) **Coverage audit/tests:** Ensure every `ClauseName` is supported or explicitly rejected; add regression tests for new getters (requires/device/depobj_update/uses_allocators/defaultmap, etc.). Add validation for new enums in `ir/validate.rs` and fix `ClauseData::Display` gaps.
4) **OpenACC sweep:** Remove remaining string parsing from accparser compat once getters are confirmed sufficient; mirror the OpenMP cleanup.

## Action Plan (small, copy/pasteable tasks)
### A. (done) Constants/header generation off legacy
- A1–A12: Mapping helper added, all ClauseName variants wired, OpenACC returns UNKNOWN_KIND, panic-on-miss installed, constants_gen wired to helper, convert_clause removed, header regenerated, comments updated, fmt/check clean.

### B. Add missing C API getters
- B1) ✅ Add getter `roup_clause_bind_modifier` (bind enum) in `src/c_api.rs`.
- B2) ✅ Add getter `roup_clause_order_kind` (order enum).
- B3) ✅ Add getter `roup_clause_atomic_default_mem_order`.
- B4) ✅ Add getters for grainsize: strict flag + expression string.
- B5) ✅ Add getters for num_tasks: strict flag + expression string.
- B6) ✅ Add getter `roup_clause_device_type_kind`; add name/helper only if compat requests it.
- B7) ✅ Add name helpers for new enums (order, atomic_default_mem_order, grainsize/num_tasks, device_type).

### C. Compat rewrites off strings
C1) ✅ In `compat/ompparser/src/compat_impl.cpp`, switch bind clause to the bind getter (remove string parsing).
C2) ✅ Switch order clause handling to the order getter.
C3) ✅ Switch atomic_default_mem_order handling to the new getter.
C4) ✅ Switch grainsize handling to getter (strict + expr).
C5) ✅ Switch num_tasks handling to getter (strict + expr).
C6) ✅ Switch device_type handling to getter.
C7) ✅ Scan remaining `args`-based clause parsing in compat and replace with typed getters where applicable; remaining `args` uses are for clauses with no typed payload getters (e.g., if-condition string, default variant).

### D. IR/display/validation tweaks
- D1) ✅ Add `ClauseData::Display` formatting for depobj_update (prints dependence).
- D2) ✅ Add validation in `src/ir/validate.rs` for device modifier and depobj_update (context/value checks).
- D3) ✅ Add parser/IR tests for device modifiers (ancestor/device_num/plain).
- D4) ✅ Add parser/IR tests for depobj_update (dependence variants).
- D5) ✅ Add parser tests for bind/order/atomic_default_mem_order/grainsize/num_tasks strict once getters exist.
- D6) ✅ Ensure `src/ir/mod.rs` re-exports new enums used by the C API (bind/order/grainsize/num_tasks/device/depobj/uses_allocators, etc.).

### E. Normalization & harness
- E1) ✅ Expose normalization toggle via env (`ROUP_NORMALIZE_CLAUSES`) in C API; plumbed through OpenMP and OpenACC parse entry points.
- E2) ✅ Add tests for normalization on/off (e.g., shared(a)+shared(b) merge vs. not).
- E3) ✅ Update `test.sh` to gate on ompparser `ctest` (fails fast on compat errors).

### F. Cleanup & verification
- F1) ✅ Reduction modifier constants retained and used in C API masks (no unused warnings).
- F2) ✅ No dead_code warnings remain (clippy clean).
- F3) ✅ Run `cargo fmt`, `cargo check`, `cargo test` (covered by `test.sh`).
- F4) ✅ Run compat `ctest -j`, capture fail list for follow-up (gated in `test.sh`; currently fails on builtin_metadirective selectors).
- F5) TODO Update docs/plan once compat parsing is string-free; residual string parsing remains for variant/implementation-defined clauses in compat (`format_clause_args`, `populate_variant_clause`, etc.; see S1–S5).

## Risks / Notes
- Mapping omissions in `clause_name_to_kind_for_constants` would leave bad IDs in `roup_constants.h`; failures should trigger panics during generation.
- Remaining compat string parsing can diverge from the AST and break `ctest` despite typed payloads.
- Normalization behavior not plumbed may cause mismatches with ompparser expectations.

## New Micro Tasks (to reach pure enum AST + passing 1527 `ctest`)
### G. Eliminate remaining post-parse parsing (done for OpenMP compat)
- G1) ✅ Add a typed getter for `if` condition expression (C API) and refactor compat to use it instead of `args`.
- G2) ✅ Add a typed getter for default clause variant body (C API) so compat can avoid parsing `args` into a nested directive; update compat to consume it.
- G3) ✅ Scan generic clause handling in compat (the fallback that uses `args` for list clauses) and replace with typed getters where the AST already exposes variables; leave only clauses with no payload.
- G4) ✅ Audit `format_clause_args`/`add_list_expressions` usage; remove any path that re-parses clause text when the AST already supplied item lists (generic fallback now uses AST variables/arguments only).
- G5) ✅ Map atomic_default_mem_order codes strictly to the ompparser enum set; fail fast on unsupported codes instead of collapsing them.

### H. Complete enum/name coverage & codegen hygiene
- H1) ✅ Replace hard-coded clause kind IDs in `src/c_api.rs` with generated/constants-driven mapping; keep a single mapping function.
  - H1.1) ✅ Normalize generated constant names to match existing identifiers (e.g., COPYIN, atomic read/write/update/capture).
  - H1.2) ✅ Ensure a single `clause_name_to_kind_for_constants` exists; remove duplicates.
  - H1.3) ✅ Remove the broken `clause_kinds.rs` include; keep mapping as the generator source of truth.
  - H1.4) ✅ Update `constants_gen`/`build.rs` fallback normalization; header regeneration succeeds.
  - H1.5) ✅ Rebuild and rerun `ctest` to confirm no regressions (all suites passing).
- H2) ✅ Suppress the `dead_code` lint on `clause_name_to_kind_for_constants` (kept for generator parsing only).

### I. Validation, display, and tests
- I1) Add `ClauseData::Display` for `depobj_update`.
- I2) Add validation in `src/ir/validate.rs` for device modifier context and depobj_update dependence.
- I3) Add parser tests for device modifiers (ancestor/device_num/plain), depobj_update variants, bind/order/atomic_default_mem_order/grainsize/num_tasks strictness.
- I4) Add unit tests for new C API getters (bind/order/atomic_default_mem_order/grainsize/num_tasks/defaultmap/uses_allocators/depobj_update/requires/device/device_type).

### J. Normalization and toggles
- J1) Expose normalization toggle via C API and thread it through compat entry points.
- J2) Add tests for normalization on/off (e.g., shared(a)/shared(b) merge vs keep).

### K. Harness and verification
- K1) Update `test.sh` to run compat `ctest` first, abort on failures, and report counts.
- K2) Run full `ctest -j` for ompparser and accparser; triage and fix any failures until 1527/1527 pass.

### L. Final cleanup
- L1) Remove any remaining `args`-based parsing code paths in compat that can be replaced with typed getters; assert/panic on unsupported clauses rather than silently parsing strings.
- L2) Ensure no raw number/string operations post-parse in C API or compat; add asserts where needed.
- L3) Delete or annotate leftover legacy helpers (if any) and rerun `cargo fmt`/`cargo check`/`cargo test`.

### M. OpenACC parity sweep
- M1) Audit `compat/accparser` for any `arguments`/string parsing; replace with typed getters (async/wait/bind/device/device_type/default_async/defaultmap/data clauses, etc.).
- M2) Ensure OpenACC clause/directive ID mappings are fully enum-based and do not reuse OpenMP enums.
- M3) Add missing OpenACC C API getters if compat needs them to avoid string parsing.

### N. Clause-by-clause coverage audit
- N1) Cross-check ompparser/accparser test matrices (openmp_vv, openacc_vv, examples) against our ClauseName/Directive lists; add any missing clauses/directives or explicit fatal errors.
- N2) Verify every compat clause handler uses typed data; no fallback `args` for clauses that have payloads.
- N3) Add any missing AST variants/getters surfaced by the audit.

### S. Strip remaining compat string parsing (OpenMP)
- S1) Metadirective/variant selectors (currently raw strings and empty in compat):
  - S1.1) ✅ Design and add typed AST structs for selectors (`OmpSelector`, device/impl/user/constructs/nested directive).
  - S1.2) ✅ Add a new `ClauseData` variant carrying `OmpSelector`; update Display/validation/IR re-exports if needed.
  - S1.3) ✅ Parse `when/match/otherwise` into typed selectors (device kinds/isa/arch/device_num, impl vendor/extension, user condition, construct list) with raw preserved.
  - S1.4) ✅ Update `ast_builder` to emit typed selector payloads (not just raw strings).
  - S1.5) ✅ Add C API structs/getters exposing selector fields and nested directive data; ensure `roup_clause_arguments` returns selector text until compat migrates.
  - S1.6) ⭕ Refactor compat to consume typed selectors/nested directive; remove `populate_variant_clause` raw parsing.
    - S1.6.1) Add selector score fields (device/impl/constructs) and nested directive AST to `OmpSelector`; keep `raw` as fallback.
    - S1.6.2) ✅ Expose score metadata and nested directive via C API getters; keep `raw` while transitioning.
    - S1.6.3) ✅ Parse the trailing nested directive in the selector and expose it via `roup_clause_selector_nested_directive`.
    - S1.6.4) Rewrite compat `when/match/otherwise` handlers to use typed selector getters (scores, device/impl/user/constructs, nested directive) and delete `populate_variant_clause`/string parsing.
      - S1.6.4.1) Regenerate/install updated `roup_constants.h` and library so compat sees selector getter prototypes (or fix include paths).
      - S1.6.4.2) Add forward declarations for selector getters in compat if needed, then rebuild.
      - S1.6.4.3) Finish `attach_variant_selector_from_roup`: set scores correctly (fix ISA branch using construct scores), map constructs broadly, and use the typed nested directive handle.
      - S1.6.4.4) Include/import ompparser directive helpers (or add forward decls) so variant directive creation links.
      - S1.6.4.5) Add forward declarations for `map_construct_to_directive`, `convert_roup_directive_to_ompparser`, and any construct factory helpers before use.
      - S1.6.4.6) Extend construct mapping (parallel/for/simd/teams/distribute/target/task/etc.) and implement a factory that builds minimal `OpenMPDirective*` without parsing.
      - S1.6.4.7) Factor out ROUP→ompparser directive conversion into a reusable helper (no parsing) that takes `OmpDirective*` + exprParse and returns an `OpenMPDirective*` by reusing existing clause conversion logic.
        - S1.6.4.7.a) Extract the existing ROUP→ompparser directive construction logic from `parseOpenMP` into a callable helper that takes an `OmpDirective*`.
        - S1.6.4.7.b) Ensure clause conversion is reused (no string parsing), and language/parameters are handled identically.
      - S1.6.4.8) Implement `convert_roup_directive_to_ompparser` using the helper from S1.6.4.7.
      - S1.6.4.9) Clean up `attach_variant_selector_from_roup` to use selector getters only (proper score assignment; no construct-score in ISA branch) and attach nested directives via typed handle/factory.
      - S1.6.4.10) Remove `populate_variant_clause` and any string-based variant parsing once typed path builds and links.
    - S1.6.5) Verify `ctest` metadirective suite passes with typed path only; no use of `args`.
    - S1.6.6) ⭕ Close ompparser parity gaps for selectors (typed-only, no raw strings):
      - Add score support on device_num selectors (AST + C API getter).
      - Add implementation user-defined expression (with score) to selector AST + C API getter.
      - Add target_device selector flag to AST + C API getter.
      - Extend construct selector entries to carry full `OmpDirective` + per-entry score (not just `OmpDirectiveKind`); parse them and expose via C API.
      - Ensure compat consumes these typed construct directives/scores and nested directives without any string parsing.
    - S1.6.7) ⭕ Rehome clause conversion into AST-only path (no raw strings):
      - Reseat the clause-conversion switch in `convert_roup_directive_to_ompparser`/`convert_clause_from_roup` using typed getters only (map enums point-to-point).
      - Drop or map any clause enums not present in ompparser; emit a clear warning when a ROUP clause/directive cannot be mapped (no silent ignore); log path for future upstream PR.
      - Include/forward-declare needed ompparser helpers (schedule/dist_schedule/allocate/defaultmap/device/etc.) so the typed conversion builds.
      - Keep construct instantiation generic or add required subclass includes; avoid any string parsing/instantiate-from-text.
      - Re-run compat build/ctest to validate the typed path end-to-end.
      - Future work: open an upstream ompparser PR for unsupported enums encountered during conversion (track warnings).
  - S1.7) ⭕ Delete `format_clause_args`/`add_list_expressions`/`roup_clause_arguments` if no references remain after S1.6/S3.
- S2) Expose typed payload for implementation-defined requirement/adjust/append/apply args via C API; update compat to consume it and remove `setImplementationDefinedRequirement(args)` and similar raw-`args` paths.
- S3) For each remaining `format_clause_args`/`add_list_expressions` usage, add needed getters (variables/expressions/modifiers) and switch compat off raw `args`; then delete those helpers.
- S4) Ensure uses_allocators user allocator names come from the typed getter; remove any leftover string copies of `args` for allocators.
- S5) After S1–S4, audit for any `roup_clause_arguments` usages; replace with typed getters and remove the function entirely if no references remain.
