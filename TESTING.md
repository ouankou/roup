# Testing Guide

This document describes the testing infrastructure and how to ensure your changes work across all supported Rust versions.

## Quick Start

```bash
# Run all local tests (current Rust version)
./test.sh

# Test MSRV and stable (recommended before PR)
./test_rust_versions.sh

# Test specific versions
./test_rust_versions.sh 1.85 stable
```

## Rust Version Support Policy

### MSRV + Stable Approach

ROUP follows the standard Rust ecosystem practice of testing **MSRV (Minimum Supported Rust Version) + stable**:

- **MSRV: 1.85.0**
  - Minimum version supporting edition2024 (required for mdBook dependencies)
  - Released February 2025
  - Set in `Cargo.toml` as `rust-version = "1.85.0"`
  - Only bumped when new language features or tooling requirements needed

- **Stable: Latest stable release**
  - Ensures compatibility with current Rust ecosystem
  - Catches new lints and API changes early

**Why This Approach?**

- ✅ **Industry standard**: Most Rust crates use MSRV + stable
- ✅ **edition2024 support**: 1.85 is first stable version with edition2024
- ✅ **Lower maintenance**: Only 2 versions to track instead of 6
- ✅ **Clear compatibility promise**: Users know minimum version required
- ✅ **Fewer CI minutes**: 2×3 = 6 jobs instead of 6×3 = 18

**CI Matrix:**
- **Rust versions:** 1.85 (MSRV), stable (2 versions)
- **Operating systems:** Ubuntu 24.04, Windows 2025, macOS 15 (3 OSes)
- **Total combinations:** 6 jobs

### When to Bump MSRV

MSRV should only be bumped when:
- ✅ You need a new language feature not available in current MSRV
- ✅ A critical dependency requires a newer Rust version
- ✅ Ubuntu LTS updates to a newer default Rust version

MSRV should NOT be bumped for:
- ❌ Clippy lint changes (fix the code instead)
- ❌ "Nice to have" features
- ❌ Following the latest Rust version

**When bumping MSRV:**
1. Update `Cargo.toml`: `rust-version = "1.XX.0"`
2. Update `.github/workflows/ci.yml`: `version: ["1.XX", "stable"]`
3. Update this documentation
4. Document the reason in the commit message

## Rust Version Configuration

### CI Config - Single Source of Truth

The **CI workflow file** (`.github/workflows/ci.yml`) is the **single source of truth** for Rust versions. The `test_rust_versions.sh` script **automatically parses** the CI config to extract the version list, ensuring local testing always matches CI exactly.

**How it works:**

```bash
# test_rust_versions.sh automatically reads from CI config
$ ./test_rust_versions.sh

# Internally, it parses:
# .github/workflows/ci.yml
#   version: ["1.85", "stable"]
#
# And tests those exact versions locally
```

**When updating MSRV:**

1. Update **TWO places** (kept in sync):
   ```yaml
   # .github/workflows/ci.yml
   version: ["1.85", "stable"]
   ```

   ```toml
   # Cargo.toml
   rust-version = "1.85.0"
   ```

2. Run `./test_rust_versions.sh` - automatically picks up the new version:
   ```bash
   $ ./test_rust_versions.sh
   Testing against Rust versions: 1.85 stable
   ```

3. Update this TESTING.md documentation.

## Test Scripts

### test.sh - Comprehensive Local Testing

Runs all 19 test categories on your current Rust version:

1. Code formatting (`cargo fmt --check`)
2. Debug build
3. Release build
4. Unit tests
5. Integration tests
6. Doc tests
7. All tests together
8. Examples build
9. API documentation
10. ompparser compatibility
11. mdBook build
12. mdBook tests
13. C examples (build + run)
14. C++ examples (build + run)
15. Fortran examples
16. Header verification
17. Warning check (zero tolerance)
18. Clippy lints (zero tolerance)
19. All features test
20. Benchmarks

**Zero-Tolerance Policy:**
- Any warning = FAIL
- Missing required files = FAIL
- All tests are MANDATORY

### test_rust_versions.sh - Multi-Version Testing

Tests your code against multiple Rust versions to catch version-specific issues **before** CI fails.

**Why This Matters:**

Clippy lints evolve across Rust versions. A lint that passes on stable might fail on MSRV (or vice versa). Examples:
- **Rust 1.85+**: `clippy::needless_lifetimes` became stricter
- **Rust 1.88+**: `clippy::uninlined_format_args` became stricter
- **Your local version**: Might be different from CI matrix

**How It Works:**

1. Automatically parses CI config to get version list
2. Uses `rustup` to install/switch between Rust versions
3. Runs critical checks on each version:
   - Format check
   - Clippy with `-D warnings`
   - Build
   - Tests
4. Reports which versions pass/fail
5. Restores your original Rust version

**Usage:**

```bash
# Test MSRV + stable (mirrors CI matrix)
./test_rust_versions.sh

# Test specific versions
./test_rust_versions.sh 1.85 stable

# Test with custom versions (for debugging)
./test_rust_versions.sh 1.85 1.86 1.87 stable
```

**When to Use:**

- ✅ Before creating a PR
- ✅ After fixing clippy warnings
- ✅ After making lifetime changes
- ✅ When updating dependencies
- ✅ When CI fails on a specific version

## CI Testing

The GitHub Actions CI runs a focused matrix:

**Matrix Dimensions:**
- **Rust versions:** 1.85 (MSRV), stable (2 versions)
- **Operating systems:** Ubuntu 24.04, Windows 2025, macOS 15 (3 OSes)
- **Total combinations:** 6 jobs

**CI Workflow:**

1. **Build Job** (6 parallel jobs):
   - Format check
   - Clippy lints (`-D warnings`)
   - Debug build
   - Release build
   - All tests
   - All features test
   - Benchmark validation
   - C examples (Linux only)
   - C++ examples (Linux only)
   - Fortran examples (Linux only)
   - Header verification (Linux only)
   - ompparser compat (Linux only)

2. **Docs Job** (runs after build):
   - mdBook tests
   - mdBook build
   - API docs (`RUSTDOCFLAGS: "-D warnings"`)
   - Examples build
   - Deploy to GitHub Pages (main branch only)

**Why Multiple OSes?**

- Different OS = Different default compilers, different file paths, different behaviors
- We want to ensure the code works **everywhere**

## Catching Version-Specific Issues

### The Problem

Clippy lints change between Rust versions:
- New lints are added
- Existing lints become stricter
- Some lints are deprecated

**Real Examples:**
- The `needless_lifetimes` lint was introduced/strictened in Rust 1.85
- The `uninlined_format_args` lint became stricter in Rust 1.88

### The Solution

**Local Testing:**
```bash
# Before pushing, test against the CI version range
./test_rust_versions.sh

# If any version fails, fix the issue
# The script will show you the exact error
```

**Understanding Failures:**

When `test_rust_versions.sh` reports a failure:

```
✗ Rust 1.85: FAILED
Clippy errors for Rust 1.85:
error: the following explicit lifetimes could be elided: 'a
  --> src/lexer.rs:255:36
```

This tells you:
1. Which version failed (1.85)
2. What failed (clippy)
3. The exact error and location

**Fixing Version-Specific Issues:**

1. Check the clippy documentation for the lint
2. Apply the suggested fix (often `cargo clippy --fix` works)
3. Re-run `./test_rust_versions.sh` to verify
4. Run `./test.sh` to ensure all other tests still pass

## Best Practices

### Before Committing

```bash
# 1. Format your code
cargo fmt

# 2. Run full test suite
./test.sh

# 3. Test against MSRV and stable
./test_rust_versions.sh
```

### Before Creating a PR

```bash
# Test the full version matrix (mirrors CI)
./test_rust_versions.sh
```

### When CI Fails

1. **Check which version failed** in the CI logs
2. **Install that version locally:**
   ```bash
   rustup install 1.85
   rustup override set 1.85
   ```
3. **Run clippy to see the error:**
   ```bash
   cargo clippy --all-targets -- -D warnings
   ```
4. **Fix the issue** (try `cargo clippy --fix --allow-dirty` first)
5. **Verify with test_rust_versions.sh:**
   ```bash
   ./test_rust_versions.sh 1.85
   ```
6. **Restore your normal version:**
   ```bash
   rustup override unset
   ```

## Understanding Test Output

### test.sh Output

```
========================================
  ROUP Comprehensive Test Suite
========================================

Environment:
rustc 1.90.0 (1159e78c4 2025-09-14)
clippy 0.1.90 (1159e78c47 2025-09-14)

=== 1. Formatting Check ===
Running cargo fmt --check... ✓ PASS
...
========================================
  ALL 19 TEST CATEGORIES PASSED
========================================
```

The "Environment" section shows which Rust/clippy version you're testing with. This helps identify if issues are version-related.

### test_rust_versions.sh Output

```
========================================
  Rust Version Compatibility Test
========================================

Testing against Rust versions: 1.85 stable

========================================
Testing Rust 1.85
========================================
  rustc: rustc 1.85.0 (a28077b28 2025-02-20)
  clippy: clippy 0.1.85

Running critical checks:
  1. Format check... ✓
  2. Clippy lints... ✗ FAILED

Clippy errors for Rust 1.82:
error: the following explicit lifetimes could be elided: 'a
...

========================================
  Summary
========================================
Tested versions: 2
Passed: 1
Failed: 1

Failed versions:
  - 1.82 (clippy)
```

This clearly shows which version failed and why.

## OpenMP_VV Round-Trip Validation

ROUP can validate itself against the [OpenMP Validation & Verification (OpenMP_VV)](https://github.com/OpenMP-Validation-and-Verification/OpenMP_VV) test suite by round-tripping every pragma through the parser.

### How It Works

The validation process:

1. **Clone OpenMP_VV** (automatically on first run to `target/openmp_vv`)
2. **Find all C/C++ test files** in the `tests/` directory
3. **Preprocess with clang** to expand macros and includes
4. **Extract OpenMP pragmas** (lines starting with `#pragma omp`)
5. **Round-trip each pragma**:
   - Normalize original with `clang-format`
   - Parse with ROUP → unparse to string
   - Normalize round-tripped version with `clang-format`
   - Compare with `diff`
6. **Report statistics**: total pragmas, pass/fail counts, success rate

### Running the Test

```bash
# Run OpenMP_VV validation (auto-clones repository if needed)
./test_openmp_vv.sh

# Or as part of the full test suite
./test.sh  # Includes OpenMP_VV as section 18
```

### Using Existing Repository

If you already have OpenMP_VV cloned elsewhere:

```bash
# Point to existing clone
OPENMP_VV_PATH=/path/to/OpenMP_VV ./test_openmp_vv.sh

# Use specific clang version
CLANG=clang-15 CLANG_FORMAT=clang-format-15 ./test_openmp_vv.sh
```

### Example Output

```
=========================================
  OpenMP_VV Round-Trip Validation
=========================================

Checking for required tools...
✓ All required tools found

Using existing OpenMP_VV at target/openmp_vv

Building roup_roundtrip binary...
✓ Binary built

Finding C/C++ test files in target/openmp_vv/tests...
Found 247 C/C++ files

Processing files...

=========================================
  Results
=========================================

Files processed:        247
Files with pragmas:     183
Total pragmas:          1247

Passed:                1189
Failed:                58
  Parse errors:         12
  Mismatches:           46

Success rate:           95.3%
```

### Requirements

- **clang** - For preprocessing (macros, includes)
- **clang-format** - For pragma normalization
- **cargo** - To build the `roup_roundtrip` binary
- **git** - To clone OpenMP_VV (if not already present)

The test gracefully skips if clang/clang-format are not available.

### Implementation Details

The validation uses two simple components:

1. **`roup_roundtrip` binary** (~30 lines of Rust):
   - Reads one pragma from stdin
   - Parses and unparses it
   - Prints to stdout
   - Exits with code 1 on parse error

2. **`test_openmp_vv.sh` script** (~200 lines of bash):
   - Orchestrates the entire workflow
   - Uses standard Unix tools (find, grep, diff)
   - Tracks statistics and formats output

This approach is simpler and more maintainable than complex Rust-only solutions.

## OpenACCV-V Round-Trip Validation

The [OpenACCV-V](https://github.com/OpenACCUserGroup/OpenACCV-V) suite exercises OpenACC directives across C, C++, and Fortran sources. ROUP round-trips every directive through the OpenACC parser to guarantee semantic fidelity.

### How It Works

1. **Clone OpenACCV-V** (automatically on first run into `target/openacc_vv`).
2. **Collect directives** from `.c`, `.cpp`, `.F90`, `.F`, and related files under `Tests/`.
3. **Normalize optional punctuation** (commas between clauses, spacing before parentheses) without altering meaning.
4. **Round-trip each directive** using the `roup_roundtrip_acc` binary.
5. **Re-parse canonical output** to verify the structure matches the original directive.
6. **Emit JSON metrics** to `target/openacc_vv_report.json` alongside the console summary.

### Running the Test

```bash
# Run OpenACCV-V validation (auto-clones repository if needed)
./test_openacc_vv.sh

# Or as part of the comprehensive suite
./test.sh  # Includes OpenACCV-V as section 20
```

### Using an Existing Clone

```bash
# Reuse an existing checkout
OPENACC_VV_PATH=/path/to/OpenACCV-V ./test_openacc_vv.sh
```

### Example Output

```
=========================================
  OpenACCV-V Round-Trip Validation
=========================================

Files processed:        1336
Files with directives:  1304
Total directives:       9417

Passed:                9417
Failed:                0
  Parse errors:        0
  Mismatches:          0

Success rate:          100.0%

JSON report written to target/openacc_vv_report.json
```

### Requirements

- **python3** – Directive harvesting and reporting (`scripts/run_openacc_vv.py`).
- **cargo** – Builds the `roup_roundtrip_acc` binary.
- **git** – Clones OpenACCV-V when no local checkout exists.

The script exits with a non-zero status if any directive fails to round-trip.

## FAQ

**Q: Why MSRV + stable instead of testing many versions?**
A: This is the standard Rust ecosystem practice. It's lower maintenance, clearer to users, and sufficient for catching issues. If it works on MSRV and stable, it almost always works on versions in between.

**Q: What is our MSRV and why?**
A: 1.85.0 - it's the first stable Rust version supporting edition2024, which is required by mdBook dependencies (specifically the `ignore` crate 0.4.24+).

**Q: When will MSRV be bumped?**
A: Only when we need new language features or Ubuntu LTS updates its default. Not for clippy lints or "nice to have" features.

**Q: Which versions should I test locally?**
A: Run `./test_rust_versions.sh` which automatically tests MSRV (1.85) + stable.

**Q: Do I need to test all 6 CI jobs locally?**
A: No! `test_rust_versions.sh` tests multiple Rust versions but only on your OS. That's usually sufficient since most issues are version-related, not OS-related.

**Q: What if I don't have rustup?**
A: Install it from https://rustup.rs/ - it's the standard Rust toolchain manager and required for managing multiple versions.

**Q: Can I skip version testing?**
A: You can, but CI will catch the issue later. Testing locally saves you a round-trip to CI (faster feedback).

**Q: What's the difference between test.sh and test_rust_versions.sh?**
- `test.sh`: Comprehensive (19 categories) on YOUR current Rust version
- `test_rust_versions.sh`: Critical checks (4 checks) on MULTIPLE Rust versions (MSRV + stable)

Use both for maximum confidence!

**Q: Can I test intermediate versions like 1.82 or 1.85?**
A: Yes! While CI only tests MSRV + stable, you can test any version locally:
```bash
./test_rust_versions.sh 1.85 1.86 1.87 stable
```
This is useful when debugging version-specific issues.

## Continuous Improvement

This testing infrastructure is designed to catch issues early. If you encounter a new class of version-specific issue:

1. Document it in this file
2. Consider adding a specific check to test_rust_versions.sh
3. Share the knowledge in PR comments

Together we keep the code quality high! 🚀
