# Testing Guide

This document explains how to exercise the test matrix used by the project and
how to mirror the CI configuration locally.

## Core commands

```bash
# Run the default Rust test suite
cargo test

# Run the local helper script (requires clang, cmake, mdBook, etc.)
./test.sh

# Check the CI toolchain matrix (MSRV + stable)
./test_rust_versions.sh
```

The helper scripts expect external tooling (CMake, clang/clang++, a Fortran
compiler, mdBook) and require the ompparser submodule to be present:

```bash
git submodule update --init --recursive
```

## Rust toolchains

The crate declares `rust-version = "1.85.0"` in `Cargo.toml`.  CI tests the
minimum supported Rust version (MSRV) alongside the latest stable release.  The
matrix is defined in `.github/workflows/ci.yml` and re-used by
`test_rust_versions.sh`, so updating the workflow keeps local testing in sync
with CI.

To run the matrix manually:

```bash
./test_rust_versions.sh            # use versions from the workflow
./test_rust_versions.sh 1.85 beta  # specify versions explicitly
```

The script installs the requested toolchains with `rustup`, runs `cargo fmt`,
`cargo clippy -- -D warnings`, builds the project, and executes the tests for
each requested toolchain before restoring your original default toolchain.

## What `test.sh` covers

`test.sh` is the one-stop script used by CI on Linux runners.  It performs the
following checks:

1. Formatting (`cargo fmt --check`).
2. Debug and release builds for all targets.
3. Unit, integration, and documentation tests.
4. Example builds (`cargo build --examples`).
5. Rust documentation (`cargo doc --no-deps --all-features`).
6. ompparser compatibility build and test suite (`compat/ompparser/build.sh`).
7. mdBook build and doctest execution (`docs/book`).
8. C, C++, and Fortran example builds (Makefiles in `examples/`).
9. Header generation check (`cargo run --bin gen`).
10. Warning audit (`cargo build` with log inspection).
11. Clippy with `-D warnings`.
12. `cargo test --all-features`.
13. Benchmark compilation via `cargo bench --no-run`.

The script exits on the first failure and prints the log captured for the
failing step to aid debugging.

## Continuous integration

The CI workflow in `.github/workflows/ci.yml` runs the `test.sh` script on
Ubuntu and executes the lighter set of checks (formatting, clippy, build, tests,
benchmarks) on Windows and macOS.  A dedicated `docs` job then builds the mdBook
and Rust API documentation to ensure documentation changes compile cleanly.
