# Release Notes

## Version 0.4.0 (Current Development)

- Completed the set of OpenMP 6.0 combined directives and helper methods.
- Expanded round-trip tests that cover parser → IR → display pipelines.
- Updated documentation to describe the new directives and refresh statistics.
- No breaking API changes; new behaviour is additive.

## Version 0.3.0 (2025-10-11)

- Published the mdBook documentation site with tutorials, architecture notes, and contribution guides.
- Clarified the experimental status of the project and refreshed public statistics.
- Fixed a memory safety issue in the C API when releasing reduction clauses.
- Added CI and local scripts to validate the C/C++ examples and documentation builds.

## Version 0.2.0 (2025-10-11)

- Replaced the handle-based C API with a direct pointer interface backed by Rust `#[repr(C)]` types.
- Documented the safety contracts around the ~60 lines of unsafe FFI code.
- Added lifecycle helpers, clause iterators, and accessor functions for clause data.
- Verified the new API with automated Rust tests and a C integration test harness.
