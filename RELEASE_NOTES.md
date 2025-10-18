# Release Notes

## 0.3.1 (Unreleased)

- Completes OpenMP 6.0 coverage by wiring the remaining combined directive variants into the IR.
- Extends the round-trip test suite to exercise the new variants and helper predicates.
- Refreshes documentation to emphasise the experimental status of the project.

## 0.3.0 (2025-10-11)

- Launched the documentation site built with mdBook, including detailed tutorials for Rust, C and C++ users.
- Fixed memory management issues in the C API reduction clause handling code.
- Cleaned up legacy headers and examples in favour of the pointer-based FFI.
- Documented repository policies around testing, documentation ownership and experimental guarantees.

## 0.2.0 (2025-10-11)

- Introduced the pointer-based C API with a minimal `unsafe` surface and iterator helpers.
- Added conversion routines for clause data, string lists and directive classification.
- Established continuous integration coverage for the new API and example binaries.
