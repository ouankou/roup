# ROUP → compat Refactor Plan (enum AST, no post-parse strings)

## Goals (unchanged)
- Single parse → typed enum AST (OpenMP/OpenACC); no post-parse string/number parsing.
- Drop-in compat: keep upstream ompparser/accparser ABI/headers (`parseOpenMP`/`parseOpenACC` only), no submodule edits.
- 100% `ctest` pass (OpenMP + OpenACC) and `test.sh` gating.
- `unsafe` only at FFI boundary.

## Current Status (Feb 2026)
- AST/IR: Unknown clauses are fatal; device modifiers and depobj_update dependences are typed enums; requires/uses_allocators/reduction are structured. No Generic fallback.
- C API (runtime): All clauses use AST; typed getters exist for reduction, defaultmap, uses_allocators, requires (modifier list), device (modifier + expr), depobj_update (dependence). Unknown clauses panic. Reduction legacy helpers removed.
- Compat: uses_allocators, requires, device, depobj_update consume typed getters (no string parsing for these). Other clauses (bind, grainsize/num_tasks modifiers, order/device_type/atomic_default_mem_order, etc.) still parse `arguments` strings.
- Legacy: `convert_clause` remains only for header generation; `constants_gen.rs` still scrapes it. Dead-code warnings persist.
- Harness: `test.sh` not yet gating on compat `ctest` first.

## Immediate Work (priority order)
1) **Header generation off legacy:** Add an enum-based clause→kind mapping (or parse literal consts) for `constants_gen.rs`, regenerate `src/roup_constants.h`, then delete `convert_clause` and related dead code.
2) **Finish compat string-free migration:** Expose getters for remaining clause payloads (bind enum, order enum, atomic_default_mem_order enum, grainsize/num_tasks strict modifiers, device_type enum, etc.) and update compat to consume them; remove all `arguments`-based parsing.
3) **Normalization & gating:** Wire normalization toggles to match ompparser/accparser expectations per test group; update `test.sh` to run compat `ctest` first, report failures, and abort early.
4) **Coverage audit/tests:** Ensure every `ClauseName` is supported or explicitly rejected; add regression tests for new getters (requires/device/depobj_update/uses_allocators/defaultmap, etc.). Add validation for new enums in `ir/validate.rs` and fix `ClauseData::Display` gaps (e.g., depobj_update).

## Risks / Notes
- Leaving `constants_gen.rs` tied to `convert_clause` blocks removal of legacy code and risks missing new clauses in `roup_constants.h`.
- Remaining compat string parsing can diverge from the AST and break `ctest` despite typed payloads.
- Normalization behavior not plumbed may cause mismatches with ompparser expectations.
