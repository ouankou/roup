# Release Notes

## 0.5.0 (2025-10-18)

### Highlights

- **Directive translation:** bidirectional C/C++ ↔ Fortran conversion is available through `translate::translate()` in Rust and
  `roup_translate_c_to_fortran()` / `roup_translate_fortran_to_c()` in the C API. Fixed- and free-form Fortran sentinels are
  handled automatically.
- **OpenMP_VV parity:** round-trip validation now covers the entire OpenMP_VV corpus via `test_openmp_vv.sh` and the
  `roup_roundtrip` helper binary, requiring a 100% match in CI.
- **Fortran sentinels:** short (`!$`) and traditional (`!$OMP`) sentinels are accepted in both free- and fixed-form layouts with
  column enforcement for fixed-form code.

### Parser and testing updates

- Added directive-specific parsers for constructs with bespoke syntax such as `declare mapper`, `scan inclusive`, and
  `groupprivate`.
- Clause parsing for `nowait` and `safesync` accepts optional arguments, and comments between directive names and content are
  preserved.
- Integration tests now include translation round-trips, Fortran sentinel variants, and the OpenMP_VV corpus, bringing the
  automated suite beyond 600 checks.
- CI focuses on MSRV (1.85) and stable toolchains across Linux, macOS, and Windows.

### Documentation and tooling

- Added translation API coverage and OpenMP_VV guidance to the documentation set.
- Consolidated build/test instructions and error-reporting improvements in the helper scripts.

### Breaking changes

None. Rust, C, and C++ interfaces remain source-compatible.

## 0.4.0 (2025-10-11)

- Completed OpenMP 6.0 coverage, including 26 new combined and atomic directive variants.
- Parser, IR, and display layers updated to understand every variant with matching helper utilities.
- Test suite now exceeds 600 cases, covering round-trips, keyword coverage, and compatibility checks.
- Documentation refreshed across the book and README to reflect the broader coverage and test counts.
- No breaking API changes: all additions are backward compatible for Rust, C, and C++ callers.

## 0.3.0 (2025-10-11)

- Launched the mdBook documentation site with end-to-end tutorials, architecture notes, and contribution guidance.
- Fixed a reduction-clause memory bug in the C API and tightened documentation around the pointer-based design.
- Clarified the experimental status across the project and aligned statistics (directive counts, clause counts, test totals).
- Continuous-integration jobs now compile and run the shipped C examples.

## 0.2.0 (2025-10-11)

- Replaced the legacy handle-based FFI with a direct pointer model implemented in Rust (`src/c_api.rs`).
- Delivered a minimal unsafe boundary (~60 lines) while keeping the remainder of the crate safe.
- Added lifecycle, iterator, clause-query, and variable-list helpers for C and C++ consumers (18 functions total).
- Validated the new API with 342 automated tests and a dedicated C smoke test.
