# Testing Guide

ROUP ships two shell scripts that mirror the CI workflow. Run them before
opening a pull request.

## Essential commands

```bash
# Full suite on your current toolchain
./test.sh

# Minimum supported Rust version (MSRV) + stable
./test_rust_versions.sh

# Optional: specify exact toolchains
./test_rust_versions.sh 1.85 stable
```

## Rust toolchain policy

- **MSRV:** 1.85.0 (first release with edition 2024 support used by the
  documentation toolchain). The version is recorded in `Cargo.toml` and the CI
  workflow.
- **Stable:** latest stable release. Clippy changes between toolchains, so we
  test MSRV and stable to catch differences early.
- Update the value only when a new language feature or dependency requires it.
  When bumping MSRV, touch `Cargo.toml`, `.github/workflows/ci.yml`, and this
  document.

## Script reference

### `test.sh`

Runs the comprehensive local suite on the active toolchain:

- Formatting, Clippy (`-D warnings`), debug and release builds
- Unit, integration, and doctests
- Example builds (Rust, C, C++)
- mdBook build
- ompparser compatibility checks
- OpenMP_VV and OpenACCV-V round-trip validation (skips when prerequisites are
  unavailable)

### `test_rust_versions.sh`

Validates critical checks across multiple toolchains:

- Reads the toolchain list from `.github/workflows/ci.yml` so local testing stays
  aligned with CI
- Installs the requested toolchains via `rustup`
- Runs formatting, Clippy, `cargo build`, and `cargo test`
- Restores your original toolchain after finishing

Use this script before a pull request or whenever CI reports a
version-specific failure.

## Continuous integration

GitHub Actions runs six jobs:

- **Toolchains:** 1.85 (MSRV) and stable
- **Operating systems:** Ubuntu 24.04, Windows, and macOS

The build job performs the same checks as `test.sh`; the docs job rebuilds the
mdBook and API documentation with warnings denied.

## Validation suites

The round-trip scripts live at the repository root:

- `test_openmp_vv.sh` validates every pragma in the
  [OpenMP Validation & Verification](https://github.com/OpenMP-Validation-and-Verification/OpenMP_VV)
  repository.
- `test_openacc_vv.sh` performs the equivalent pass for the
  [OpenACCV-V](https://github.com/OpenACCUserGroup/OpenACCV-V) suite across both
  C/C++ and Fortran input.

Both scripts clone their upstream repositories into `target/` on first run and
skip gracefully if required tools such as `clang` or `clang-format` are missing.

## FAQ

- **Why MSRV + stable?** It keeps maintenance manageable while covering the two
  toolchains most likely to expose Clippy or compiler differences.
- **Do I need rustup?** Yes. The scripts use `rustup` to install and select the
  toolchains they need.
- **Can I test other versions?** Pass additional toolchains to
  `test_rust_versions.sh`, for example `./test_rust_versions.sh 1.85 1.86 stable`.
