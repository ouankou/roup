# Testing Guide

ROUP ships two helper scripts that mirror the continuous-integration setup. Run them before submitting a change to ensure the parser, compatibility layers, and documentation stay healthy.

## Essential commands

```bash
./test.sh                 # full local battery on the active toolchain
./test_rust_versions.sh   # mirrors the CI Rust matrix (MSRV + stable)
./test_rust_versions.sh 1.85 stable  # explicit versions when debugging
```

## Supported Rust versions

- **Minimum supported Rust version (MSRV): 1.85.0**
  - Matches the `rust-version` entry in `Cargo.toml`.
  - First stable release with Edition 2024 support required by the docs build.
- **Latest stable**: tracked automatically by CI to catch new lints and regressions.

When bumping the MSRV:

1. Update `Cargo.toml` and `.github/workflows/ci.yml` to reference the new version.
2. Adjust this document and any release notes.
3. Run `./test_rust_versions.sh` to ensure both versions pass locally.

## Script overview

### `test.sh`

Runs every mandatory check on the active toolchain:

- Formatting, clippy (`-D warnings`), and warning sweeps.
- Debug and release builds for all targets.
- Unit, integration, doc, and all-target tests.
- Example builds and runs for Rust, C, C++, and Fortran.
- mdBook build and doctest (`mdbook build/test docs/book`).
- Compatibility layers (`compat/ompparser` and `compat/accparser`).
- Header generation, feature-gated tests, and benchmark compilation.
- OpenMP_VV and OpenACC_VV round-trip validation with 100% required success when prerequisites are installed.

Most sections expect optional tooling (e.g., `clang`, `clang-format`, `mdbook`). Install the tools listed in the script header if a step fails due to missing dependencies.

### `test_rust_versions.sh`

Validates multiple toolchains by parsing the CI workflow for the Rust matrix (currently `1.85` and `stable`). For each version it runs:

1. `cargo fmt --check`
2. `cargo clippy --all-targets -- -D warnings`
3. `cargo build --all-targets`
4. `cargo test --all-targets`

The script restores your original toolchain when it finishes. Pass explicit versions to focus on a subset, e.g. `./test_rust_versions.sh 1.85`.

## Continuous integration

GitHub Actions executes the same checks across six jobs:

- **Rust versions:** `1.85` (MSRV) and `stable`.
- **Operating systems:** Ubuntu 24.04, Windows 2025, and macOS 15.

The build jobs cover formatting, clippy, debug/release builds, full tests, feature-gated tests, benchmark compilation, example builds, and compatibility layers. A follow-up docs job builds the API docs with warnings-as-errors and runs `mdbook build/test` before deploying to GitHub Pages on `main`.

## Validation suites

### OpenMP_VV

`./test_openmp_vv.sh` clones the [OpenMP Validation & Verification](https://github.com/OpenMP-Validation-and-Verification/OpenMP_VV) suite, normalises each pragma with `clang-format`, round-trips it through `roup_roundtrip`, and compares the results. Clang and clang-format must be available in `PATH`. The script fails if any pragma diverges or a parse error occurs.

### OpenACC_VV

`./test_openacc_vv.sh` exercises the [OpenACCV-V](https://github.com/OpenACCUserGroup/OpenACCV-V) suite across C, C++, and Fortran sources. It normalises directives internally, runs `roup_roundtrip --acc`, and reports any mismatches. Python 3 is required for the helper utilities.

## Recommended workflow

1. `cargo fmt`
2. `./test.sh`
3. `./test_rust_versions.sh`

Fix issues as they appear, then rerun the affected command. Keeping the scripts green locally saves cycles when CI runs the same checks.
