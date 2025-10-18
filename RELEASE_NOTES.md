# Release Notes

## 0.4.0 (unreleased)

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
