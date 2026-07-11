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
- release builds and the complete, unchanged pinned upstream ctest suites for
  the `ompparser` and `accparser` adapters;
- C, C++, and Fortran examples.

All native prerequisites and both parser submodules are required. In
particular, `clang-format` is part of the upstream C/C++ round-trip oracle and
is required explicitly rather than relying on a runner-image default. A
missing prerequisite is an error.

At the currently pinned revisions, the ompparser build registers all 1,534
upstream tests plus three local contract tests (1,537 total), and the accparser
build registers all 918 upstream tests plus two local contract/audit tests (920
total). The compatibility gate does not rewrite fixtures or expected output,
select a reduced manifest, disable tests, or allow failures. CTest runs in
verbose mode so CI records every invoked test command. After each suite,
`scripts/verify_ctest_execution.py` hard-fails unless the CTest JUnit receipt
contains every expected test exactly once with no failure, error, or skip.

The complete Ubuntu job can be reproduced with an act-compatible Ubuntu 26.04
runner image:

```bash
act pull_request -W .github/workflows/ci.yml -j test \
  --matrix rust:stable -P ubuntu-26.04=<ubuntu-26.04-act-image>
```

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

The minimum supported Rust version is 1.88, the highest minimum required by
the complete Rust dependency and tooling graph, including the required pinned
mdBook. CI has two full-suite jobs: Rust 1.88 and the current stable release on
Ubuntu 26.04. Both run the complete `test.sh` gate. It also runs the basic Rust
build gate with both toolchains on Windows 2025 and macOS 26; those
jobs run formatting, standalone and workspace checks, Clippy, and workspace
tests, but do not duplicate the native compatibility or documentation suites.
On pushes to `main`, the stable Ubuntu job packages the documentation that gate
already built; no separate job rebuilds it.
