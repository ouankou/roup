# Release Notes

## Unreleased

- Replaced permissive/repair parsing with a strict typed Rust AST, including
  typed host expressions, variables, literals, selectors, locators, directive
  parameters, and clause-specific payloads. Malformed or unsupported input now
  returns a structured hard error instead of a partial or fallback AST.
- Retained syntax standardized by every earlier OpenMP specification in later
  version modes while canonicalizing specification-defined aliases. Exact
  modes reject only syntax introduced after the selected version.
- Split the complete safe Rust parser from the optional `roup-capi` package.
  The root crate is an `rlib` that forbids unsafe code; foreign pointer access
  is isolated to one audited C-memory boundary.
- Replaced the previous C ABI with opaque generational handles, typed field and
  child-node queries, explicit errors, and adversarial stale/retagged-handle
  tests. This is intentionally API- and ABI-breaking.
- Updated the `ompparser` and `accparser` adapters to consume the typed ABI and
  fail when an upstream IR cannot faithfully represent a parsed payload.
- Replaced round-trip and permissive external-corpus gates with focused strict
  syntax, semantic-fact, ABI, adapter, and cross-language regression tests.
- Made Fortran continuation handling dialect-aware: standard OpenACC sentinels
  are stripped from continued OpenACC directives, while cross-dialect
  continuation sentinels are hard errors.
- Hardened documentation publication as a GitHub Pages Actions artifact
  deployment and moved Linux and macOS CI to the Ubuntu 26.04 and macOS 26
  hosted runners.
- Refreshed the complete Cargo dependency lockfile to the newest available
  releases while retaining the verified Rust 1.88.0 repository MSRV required
  by the complete dependency and tooling graph.
- Migrated both workspace crates to the Rust 2024 edition, including explicit
  C export attributes and preserved mutex-guard drop ordering at the ABI
  service boundary.

> The versioned entries below describe each historical release at the time it
> shipped. They may mention binaries, validation scripts, or FFI designs that
> the breaking unreleased refactor above has intentionally removed. Use the
> current README, book, and checked C header as the present API reference.

## 0.7.0 (2025-11-25)

- **Interactive debugger:** `roup_debug` now traces OpenMP and OpenACC parsing step-by-step (interactive and batch), including Fortran sentinels and EOF handling fixes.
- **OpenACC drop-in:** accparser compatibility layer reaches 100% of the upstream ctest suite with ROUP_ACC_* constants and expression/modifier support.
- **Fortran validation:** Expanded Fortran coverage in validation suites and debugger detection; OpenACC end-paired directives now mapped in compat shims.
- **Keyword canonicalisation:** Enforced canonical keyword forms across parsers and generators; OpenACC tokens preserve original spelling for custom parsers.
- **MSRV:** Bumped to Rust 1.88.0 (aligned with mdBook 0.5.x and dependencies).
- **CI/docs:** mdBook fixes, accparser ctests added to scripts, and consistent unknown sentinel (-1) in generated headers and docs.

## 0.6.0 (2025-10-23)

### OpenACC 3.4 support

- **Full OpenACC 3.4 coverage:** comprehensive directive and clause parsing with alias support documented in [`docs/OPENACC_SUPPORT.md`](docs/OPENACC_SUPPORT.md).
- **accparser compatibility layer:** drop-in replacement for the original accparser library with zero ANTLR dependency, shipped in [`compat/accparser/`](compat/accparser/).
- **OpenACCV-V validation:** 100% round-trip success on the [OpenACCV-V](https://github.com/OpenACCUserGroup/OpenACCV-V) suite with automated testing via `test_openacc_vv.sh`.
- Supports both C/C++ `#pragma acc` and Fortran sentinels (`!$acc`, `c$acc`, `*$acc`) across free and fixed form.

### Parser enhancements

- **Parameter normalization:** directive parameters are now normalized consistently across OpenMP and OpenACC.
- **Scan directive fixes:** proper parsing of `scan inclusive` and `scan exclusive` with argument lists.
- **Improved error handling:** better diagnostics for malformed directives and clauses.

### Testing and validation

- **Parallel validation:** OpenMP_VV and OpenACCV-V tests now run with parallelized xargs for faster CI execution.
- **Hardened scripts:** validation scripts use proper quoting and error handling to prevent shell injection.
- Test suite expanded to cover OpenACC round-trips and cross-language compatibility.

### Documentation

- Streamlined top-level guides (README, TESTING, RELEASE_NOTES).
- Removed temporary analysis and design documents.
- Updated submodules to latest upstream versions (accparser, ompparser).
- Consolidated compatibility layer documentation.

### Breaking changes

None. All Rust, C, and C++ APIs remain backward compatible.

## 0.5.0 (2025-10-18)

### Translation API

- Added a conversion layer between C/C++ and Fortran directives.
- Rust entry point: `translate::translate()` detects the input format.
- C API wrappers: `roup_translate_c_to_fortran()` and `roup_translate_fortran_to_c()`.
- Handles both free-form and fixed-form Fortran, including column-7 sentinels.
- The then-current translation guide was removed with the breaking refactor
  described in the unreleased section above.

### OpenMP_VV validation

- Achieved a 3767/3767 success rate on the [OpenMP_VV](https://github.com/OpenMP-Validation-and-Verification/OpenMP_VV) suite.
- Added `test_openmp_vv.sh` and the `roup_roundtrip` helper for automation.
- Parser gained bespoke handlers for 10 directives that require special casing.
- CI enforces 100% parity through the new validation stage.

### Fortran support

- Recognises both traditional (`!$OMP`) and short (`!$`) sentinels in free and fixed form.
- Includes targeted tests for each sentinel combination.

### Parser changes

- Added directive-specific parsers for allocate, threadprivate, declare target, declare mapper, declare variant, depobj, scan (exclusive/inclusive), cancel, groupprivate, and target_data.
- `nowait` and `safesync` clauses accept optional arguments.
- Directives now tolerate comments between the keyword and arguments.
- Added missing directives: `end assumes`, `master taskloop`, and `master taskloop simd`.

### Testing and CI

- Test count now exceeds 620.
- New suites cover OpenMP_VV round-trips, translation round-trips, Fortran sentinel variations, and directive-specific parsing.
- CI targets MSRV 1.85 and stable across Linux, macOS, and Windows.

### Documentation and internal work

- Expanded translation API and OpenMP_VV documentation.
- Consolidated architecture material across the book and README.
- Introduced the `CUSTOM_PARSER_DIRECTIVES` constant and simplified lexer helpers.
- Improved test scripts with clearer platform requirements and error messages.

### Breaking changes

None. Rust, C, and C++ APIs remain backward compatible.

## 0.4.0 (2025-10-11)

- Completed OpenMP 6.0 coverage, including 26 new combined and atomic directives.
- Parser, IR, and display layers now understand every variant with supporting utilities.
- The test suite surpassed 600 cases, covering round-trips, keyword coverage, and compatibility checks.
- Documentation refresh aligned statistics and clarified the experimental status.

## 0.3.0 (2025-10-11)

- Published the mdBook documentation site with tutorials, architecture notes, and contribution guidance.
- Fixed a reduction-clause memory bug in the C API and clarified the pointer model.
- Synced directive, clause, and test totals across the docs.
- CI now compiles and executes the bundled C examples.

## 0.2.0 (2025-10-11)

- Replaced the handle-based FFI with a direct pointer model in `src/c_api.rs`.
- Reduced the unsafe boundary to roughly 60 lines.
- Added lifecycle, iterator, clause-query, and variable-list helpers for C and C++ consumers.
- Validated the API with 342 automated tests and a dedicated C smoke test.
