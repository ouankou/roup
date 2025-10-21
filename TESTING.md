# Testing guide

ROUP ships scripts for local verification and mirrors the CI workflow. Use these commands before submitting changes:

```bash
# Format, lint, build, run tests, build docs, and exercise examples on your current toolchain
./test.sh

# Repeat critical checks on the MSRV and stable toolchains
./test_rust_versions.sh

# Target specific toolchains if needed
./test_rust_versions.sh 1.85 stable
```

## Supported Rust versions

- **Minimum supported Rust version (MSRV): 1.85.0** – required for the edition 2024 dependencies in the documentation build.
- **Stable** – the latest stable release.

`./test_rust_versions.sh` reads the toolchain list directly from `.github/workflows/ci.yml`, so updating the workflow automatically keeps local testing in sync. When bumping the MSRV, update `Cargo.toml`, the CI workflow, rerun the scripts, and note the change in your commit message.

## Local scripts

### `test.sh`

Runs the full matrix of checks on the active toolchain:

1. `cargo fmt --check`
2. Debug and release builds
3. Unit, integration, doc, and feature-gated tests
4. Clippy with `-D warnings`
5. Example builds (C, C++, Fortran) and header verification
6. mdBook build and link checks
7. ompparser compatibility build and tests
8. OpenMP_VV and OpenACC_VV round-trip validation (100% required)

The script fails fast on warnings or missing prerequisites.

### `test_rust_versions.sh`

Ensures changes compile and lint cleanly on every toolchain in the CI matrix. For each version it performs:

1. `cargo fmt --check`
2. `cargo clippy --all-targets -- -D warnings`
3. Debug and release builds
4. `cargo test`

The script installs missing toolchains with `rustup`, restores the original default toolchain on exit, and prints a summary with any failing versions.

## Continuous integration

GitHub Actions runs the same checks across the matrix defined in `.github/workflows/ci.yml`:

- **Toolchains:** MSRV (`1.85`) and stable
- **Operating systems:** Ubuntu 24.04, Windows 2025, macOS 15

The build jobs cover formatting, clippy, debug/release builds, and tests. A dedicated docs job rebuilds the mdBook site, API docs, and example binaries and deploys them to GitHub Pages on `main`.

## Specification validation

The project validates its OpenMP and OpenACC support with the upstream compliance suites:

- `test_openmp_vv.sh` parses, unparses, and compares every pragma from the OpenMP_VV repository. The helper binary `roup_roundtrip` performs the round-trip; the script normalises pragmas with `clang`/`clang-format` and requires a 100% match rate.
- `test_openacc_vv.sh` performs the same process for the OpenACCV-V repository, covering both C/C++ and Fortran directives.

Provide `OPENMP_VV_PATH` or `OPENACC_VV_PATH` to reuse existing clones, and set `CLANG`/`CLANG_FORMAT` to select specific toolchain versions.

## Troubleshooting

- **Version-specific lint failures:** run `./test_rust_versions.sh` and inspect the failing section. Install the reported toolchain with `rustup install <version>` to reproduce locally.
- **Missing prerequisites:** the scripts abort with descriptive messages if required tools are absent. Install the missing tool and rerun.
- **Docs rebuild failures:** ensure `mdbook`, `mdbook-linkcheck`, and the Rust documentation build succeed with `cargo doc --no-deps`.

Keeping these scripts green ensures parity with CI and the published documentation site.
