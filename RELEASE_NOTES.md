# Release Notes

## 0.4.0 – Current release
- Completed the OpenMP 6.0 keyword catalogue in the parser and updated the
  keyword registry tests (`tests/openmp_keyword_coverage.rs`,
  `tests/openmp_support_matrix.rs`).
- Exercised the new loop transformation and meta-directive grammar through
  dedicated integration suites (`tests/openmp_loop_transformations.rs`,
  `tests/openmp_metadirective.rs`).
- Expanded language aware parsing and validation for Fortran input, including
  continuation handling (`tests/openmp_fortran.rs`,
  `tests/openmp_fortran_sentinels.rs`).
- Refined the ompparser compatibility layer in `compat/ompparser/` so the build
  script and tests work out of the box on Linux, macOS, and Windows runners.
- Simplified the documentation set and refreshed the mdBook landing pages.

## 0.3.0 – Documentation refresh
- Introduced the public documentation site backed by the mdBook sources in
  `docs/book/` and linked tutorials for Rust, C, C++, and Fortran examples.
- Added comprehensive developer documentation describing the architecture,
  testing workflow, and compatibility story.
- Formalised the experimental status of the project and aligned the README with
  the pointer-based C API.

## 0.2.0 – Pointer based C API
- Replaced the previous handle based FFI with the direct pointer interface now
  implemented in `src/c_api.rs`.
- Added helpers for iterating over clauses, accessing schedule information, and
  freeing heap allocated data from C.
- Established the initial `cargo test` integration suites and example builds for
  the new API.
