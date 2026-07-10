# Testing Guide

The repository has two fail-fast validation entry points. Neither script skips
missing tools, missing submodules, malformed configuration, or failing test
families.

## Complete repository gate

```bash
git submodule update --init --recursive
./test.sh
```

`test.sh` validates:

- formatting and the strict AST/unsafe-code audit;
- standalone `roup`, its publishable pure-Rust package, and complete-workspace checks;
- Clippy with warnings denied, Rust tests, benchmark compilation, and docs;
- the checked C header with C11 and C++17 compilers;
- release builds and ctests for the `ompparser` and `accparser` adapters;
- C, C++, and Fortran examples.

All native prerequisites and both parser submodules are required. A missing
prerequisite is an error.

## Rust toolchain gate

```bash
./test_rust_versions.sh
```

Without arguments, the script reads the exact `rust` matrix from
`.github/workflows/ci.yml`. It errors if the matrix cannot be read. Explicit
toolchains may be supplied when needed:

```bash
./test_rust_versions.sh 1.88 stable
```

For every requested toolchain, the script installs Rustfmt and Clippy and runs
the standalone and workspace checks, the safety audit, Clippy, and all tests.
The first failure terminates the run.

## Toolchain policy

The minimum supported Rust version is 1.88. CI tests Rust 1.88 and the current
stable release on Linux, Windows, and macOS. The Linux integration job also
runs the complete native and documentation gate.
