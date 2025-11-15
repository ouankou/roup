# ROUP Compat Execution Plan

## 1. Objective & Final Goal
Deliver a ROUP parser/IR/C API/compat stack that: (a) emits a fully enum-based AST for every OpenMP/OpenACC directive, clause, modifier, and keyword with language-specific prefixes, (b) contains zero raw string/number/char parsing outside the initial parsing layer, (c) mirrors ompparser/accparser clause normalization behavior per test group, and (d) allows a fresh clone (with submodules) to build `compat/ompparser` and `compat/accparser`, run `cmake && make -j && ctest -j`, and pass all 1,527 ompparser tests plus the accparser suites. `test.sh` will enforce this by running compat `ctest` in full and aborting immediately on failure.

## 2. What Needs Fixing
- **AST/Data Model**: Replace mixed string/int-based directive + clause representations with typed enums for every OpenMP/OpenACC keyword (no shared variants across languages or directives/clauses/modifiers). `src/ast`, `src/ir`, and parser outputs must align with this schema.
- **Parser Layer**: `src/parser/openmp.rs`, `openacc.rs`, and clause helpers must emit the new enums, handle directives with spaces/underscores exactly as ompparser/openacc_vv expect, and populate clause payload structs (identifier lists, expressions, reduction ops, map types, etc.).
- **Normalization Controls**: Implement a configurable normalization switch (enabled/disabled/match-ompparser) so constructs like `shared(a) shared(b)` merge only for the test groups that expect it; expose this through parser config and the C API so compat can toggle it dynamically.
- **C API Surface**: `src/c_api.rs` and `src/c_api/openacc.rs` must forward the structured AST without any ad-hoc parsing, string concatenation, or magic numbers; unsafe Rust restricted to FFI boundary only.
- **Compat Layer Consumption**: `compat/ompparser` and `compat/accparser` need to instantiate ompparser/accparser IR nodes strictly via the enums/payloads. Any helper performing parsing, tokenization, or deduction must be removed; normalization toggles must bridge through.
- **Testing Harness**: `test.sh` must run compat `ctest` (OpenMP + OpenACC) first, report failure counts, and exit immediately on non-zero failures before running other suites.

## 3. Constraints & Assumptions
- Ompparser and accparser submodules remain untouched; all fixes occur in ROUP and compat glue.
- Unsafe Rust is only acceptable for raw-pointer FFI entry points.
- `openmp_vv`, `openmp_examples`, and accparser `openacc_vv` suites define the exact directive/clause universe; ROUP must support exactly those constructs for both C/C++ and Fortran.
- Compat unparsing issues always stem from ROUP emitting an incomplete AST or mis-configuring normalization; compat cannot rely on ROUP text unparsers.
- No code after the parsing layer may introduce temporary string/number-based parsing, even for debugging.
- Each enum value must be referenced symbolically (`ROUP_OMPD_parallel`, etc.); numeric literals are forbidden.

## 4. Challenges & Risks
- **Enum Explosion Complexity**: Hundreds of directive/clause variants increase boilerplate size; risk of missing payload data and ABI drift in C API.
- **Parser Parity**: Ensuring OpenMP/OpenACC grammar coverage (including end directives, Fortran forms, modifiers) matches ompparser/accparser exactly without falling back to textual hacks.
- **Normalization Fidelity**: Mapping ompparser’s normalization behaviors onto ROUP without modifying the submodule requires precise configuration and extensive regression coverage.
- **Compat Integration**: Large refactor to compat C++ code may introduce mismatched IR construction; requires systematic validation using upstream tests.
- **Testing Time**: Full compat `ctest` (~1.5k tests) is time-consuming; workflow must amortize by running focused subsets during development while ensuring `test.sh` always performs the full suite before CI.

## 5. Actionable Schedule & Steps
1. **Schema Finalization (0.5d)**
   - Expand `src/ast/mod.rs` to include full enum sets for OpenMP (`ROUP_OMPD_*`, `ROUP_OMPC_*`, modifier enums) and OpenACC (`ROUP_ACCD_*`, `ROUP_ACCC_*`).
   - Define payload structs for every clause (reduction, in_reduction, map, bind, etc.) plus identifier/expression containers.
   - Output Rust + C header constants via `build.rs`/`src/constants_gen.rs` so enums stay in sync.

2. **Parser Emission Upgrade (1.5d)**
   - Rewrite `src/parser/openmp.rs`, `src/parser/openacc.rs`, and shared clause helpers to construct the new enums directly, including Fortran branches.
   - Add builder utilities for directives/clauses to keep parsing readable.
   - Wire in a `ClauseNormalizationMode` inside parser config but keep it disabled until the compat layer sets it.

3. **IR & Storage Alignment (0.5d)**
   - Update `src/ir/*.rs` modules and any serializer/deserializer to carry the enumized AST without lossy conversions.
   - Ensure the debugger/stepper and internal passes consume the enums.

4. **C API Reshape (0.75d)**
   - Adjust `src/c_api.rs` (and OpenACC variant) to expose the enums and payload structs over FFI with minimal unsafe code.
   - Provide getter APIs for clause payloads (identifier arrays, reduction ops, expressions) so compat never examines strings.
   - Surface normalization config toggles via API.

5. **Compat Layer Refactor (1.0d)**
   - Rebuild `compat/ompparser/src/compat_impl.cpp` and `compat/accparser/src/*.cpp` to instantiate ompparser/accparser IR purely from enums/payloads.
   - Remove any string-based helpers or temporary parsing (e.g., reduction clause parsing, directive token matching).
   - Ensure each language keeps separate enums and per-test-group normalization toggles feed through.

6. **Testing Harness Hardening (0.25d)**
   - Update `test.sh` so compat `ctest` (OpenMP + OpenACC) runs first, prints failure stats, and exits non-zero immediately when any compat test fails.
   - Add optional focused test targets (`ctest -R ...`) for quicker iteration without changing CI behavior.

7. **Stabilization & Verification (continuous, final 1.0d)**
   - After each major step, run relevant subsets (`cargo test parser::openmp`, `ctest -R reduction`, `ctest -R openacc`).
   - Final gate: clean build of compat/ompparser and compat/accparser followed by `ctest -j` for both suites through `test.sh`; success criteria is 100% pass rate.

## 6. Verification Strategy
- **Unit Level**: Extend/adjust Rust unit tests for parser + AST modules to ensure enums and normalization behave as expected.
- **Integration Level**: For each directive/clause family converted, run targeted `ctest` subsets and compare AST dumps to ompparser expectations.
- **Full Regression**: `test.sh` executes: (1) compat `ctest` (OpenMP 1,527 tests + OpenACC suites) and aborts on any failure, (2) remaining ROUP tests only if compat passes. Fail counts and logs from `compat/ompparser/build/Testing/Temporary/LastTest.log` must be reported when failures occur.
- **Acceptance**: Success declared only when a fresh checkout plus submodules can complete `compat/ompparser` + `compat/accparser` build/test flow and `test.sh` exits 0 without manual intervention.
