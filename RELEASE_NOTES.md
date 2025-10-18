# Release Notes

## 0.5.0 (2025-10-18)

### Major Features

**Translation API** - C/C++ ↔ Fortran Directive Translation
- Added comprehensive translation infrastructure for converting OpenMP directives between C/C++ and Fortran
- Rust API: `translate::translate()` with automatic format detection
- C API: `roup_translate_c_to_fortran()` and `roup_translate_fortran_to_c()` with explicit format control
- Full support for both free-form and fixed-form Fortran (with proper column-based sentinel positioning)
- Handles language-specific syntax differences (DO vs FOR, array section parentheses vs brackets)
- See [Translation API documentation](https://roup.ouankou.com/api-reference.html#translation-api) for details

**OpenMP_VV Validation** - 100% Round-Trip Pass Rate
- Achieved **100% success rate** (3767/3767 pragmas) on the official [OpenMP Validation & Verification](https://github.com/OpenMP-Validation-and-Verification/OpenMP_VV) test suite
- Validates ROUP against real-world OpenMP code from the comprehensive OpenMP_VV repository
- Automated round-trip testing: parse → unparse → compare with clang-format normalization
- Added `test_openmp_vv.sh` script and `roup_roundtrip` binary for validation
- Enhanced parser with 10 custom directive parsers for non-standard syntax patterns
- Updated test infrastructure to require 100% pass rate in CI

**Enhanced Fortran Support**
- Short-form Fortran sentinels: `!$` (free-form) and `      !$` (fixed-form, must start in column 7)
  - Note: In fixed-form Fortran, the sentinel must begin in column 7 (i.e., six leading spaces before `!$`)
- Comprehensive sentinel variation tests covering all valid OpenMP Fortran comment formats
- Full support for both traditional (`!$OMP`/`      !$OMP`) and short (`!$`/`      !$`) forms

### Parser Improvements

- **Custom directive parsers** for 10 directives with special syntax:
  - `allocate(list)`, `threadprivate(list)`, `declare target(list)`
  - `declare mapper(id)`, `declare variant(func)`, `depobj(obj)`
  - `scan exclusive/inclusive(list)`, `cancel construct-type`
  - `groupprivate(list)`, `target_data` (underscore variant)
- **Flexible clause rules**: `nowait` and `safesync` now support optional arguments
- **Comment handling**: Proper support for comments between directive names and parenthesized content
- **Missing directives added**: `end assumes`, `master taskloop`, `master taskloop simd`

### Testing & Validation

- Test suite now at **620 automated tests** (up from 600+)
- New test categories:
  - OpenMP_VV round-trip validation (3767 real-world pragmas)
  - Translation round-trip tests (C/C++ ↔ Fortran)
  - Fortran sentinel variation tests
  - Custom parser integration tests
- Updated MSRV testing approach: 1.85 (MSRV) + stable only
- Enhanced CI matrix: 6 jobs (2 Rust versions × 3 OSes)

### Documentation

- Comprehensive translation API documentation with examples
- Detailed OpenMP_VV validation documentation in `TESTING.md`
- Updated architecture documentation
- Consolidated and streamlined guides
- All statistics and numbers verified for accuracy

### Internal Improvements

- Constant `CUSTOM_PARSER_DIRECTIVES` for maintainability
- Simplified `parse_parenthesized_content()` using lexer utilities
- Platform-specific installation instructions in test scripts
- Enhanced error messages and debugging support

### Breaking Changes

**None** - All changes are backward compatible for Rust, C, and C++ callers.

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
