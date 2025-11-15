# Compat Layer & Roup IR Fix Plan

## Objectives
- Enforce the contract: **after the parsing layer there must be zero raw string/number/char parsing.** The parser produces a fully structured, enum-based AST that every downstream layer consumes as-is.
- Ensure each OpenMP directive and clause is represented by its own enum variant (no grouping).
- Keep the `ompparser` submodule untouched while making `compat` a drop-in replacement that passes all 1,527 upstream tests via `cmake`, `make -j`, and `ctest -j`.

## Constraints & Risks
- Any existing post-parse string inspection in `src/c_api.rs`, `compat/ompparser/src/*.cpp`, or related helpers must be removed instead of patched.
- Parser refactor must avoid regressions in directive semantics across C/C++/Fortran front ends.
- Enum explosion increases serialization/deserialization complexity; ensure FFI layout remains stable.
- OpenMP and OpenACC constructs must never share enums; every directive/clause/modifier/keyword per language gets its own unique enum (e.g., `ROUP_OMPD_parallel`, `ROUP_OMPC_shared`, `ROUP_ACCD_loop`, `ROUP_ACCC_shared`).
- All code references enums symbolically; numeric literals are prohibited.
- Clause normalization (e.g., `shared(a) shared(b)` → `shared(a, b)`) must be configurable so tests can enable/disable it.
- Unsafe Rust limited strictly to the C FFI boundary.

## Action Plan
1. **Audit Current Data Flow (0.5d)**
   - Map every place that inspects raw strings/numbers after parsing.
   - Document missing enums/fields preventing structured transfer.
   - Owners: parser (`src/parser/openmp.rs`), IR definitions (`src/ir/*.rs`), C API (`src/c_api.rs`), compat bridge (`compat/ompparser/src/compat_impl.cpp`).

2. **Design Enum-Based AST (0.75d)**
   - Specify directive enum variants for every OpenMP and OpenACC directive independently with language-specific prefixes (no sharing).
   - Define clause/modifier enums with payload structs capturing all necessary operands.
   - Introduce a configuration knob for clause normalization that downstream layers can toggle per test group.
   - Produce a schema doc describing structs/enums, default values, invariants, and the normalization switch.

3. **Implement Parser Changes (1.5d)**
   - Update `src/parser/openmp.rs` to populate the new enums.
   - Introduce helper builders to keep parsing logic readable.
   - Remove legacy fields carrying raw parameter strings.

4. **Update IR & Storage (0.75d)**
   - Adjust IR modules (e.g., `src/ir/directive.rs`, `src/ir/clause.rs`) to hold the new enums.
   - Ensure serialization/deserialization (if any) handles the richer data.

5. **Revise C API Surface (0.75d)**
   - In `src/c_api.rs`, expose getters/setters for enum-based data; remove any string parsing or concatenation logic.
   - Update FFI-safe structs to mirror the new AST while keeping ABI stability (consider version bump if needed).
   - Keep unsafe Rust confined to the immediate FFI glue and document safety invariants.

6. **Rewrite Compat Layer Consumption (1.0d)**
   - Refactor `compat/ompparser/src/compat_impl.cpp` to switch entirely to structured enums and clause payloads.
   - Delete any residual parsing utilities or ad-hoc formatting code.
   - Handle all directive kinds (declare mapper/simd/reduction, loop bind, in_reduction, end directive spacing) purely via structured data.
   - Support per-test-group clause normalization toggles that mirror `ompparser`/`accparser` behavior so their unparsers receive exactly the AST they expect.

7. **Language-Specific Stabilization (0.5d)**
   - Ensure Fortran directives receive the same structured treatment to eliminate segfaults (atomic, critical, declare target, distribute).
   - Validate directive names with spaces/underscores per `openmp_vv` and `openmp_examples` suites in `ompparser`, and mirror the `openacc_vv` coverage from the `accparser` submodule for OpenACC; support exactly what those suites exercise, no more, no less.

8. **Validation & Regression Testing (continuous, final full day)**
   - Run targeted suites (`ctest -R declare`, `ctest -R fortran`, `ctest -R openacc`) after each milestone.
   - Update `test.sh` so the compat-layer `ctest` (all 1,527 OpenMP tests plus the AccParser suite) runs before later sections, reports failure rates, and aborts immediately on any failure; the rest of the script executes only if compat suites pass 100%.
   - Final verification: from a clean clone run `compat/ompparser` (and `compat/accparser` as applicable) build steps and ensure all upstream tests pass without touching the submodules.

## Deliverables
- Updated parser/IR/C API/compat code with zero post-parse raw string ops.
- Schema documentation for the new enum-based AST.
- Test logs demonstrating full ctest pass on fresh checkout.

## Adherence
- This plan replaces all previous documents. Future work must reference these steps and not reintroduce string parsing shortcuts.
