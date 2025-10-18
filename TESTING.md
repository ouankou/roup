# Testing Guide

The repository ships two helper scripts:

```bash
./test.sh                # format, lint, build and run the suite on the active toolchain
./test_rust_versions.sh  # repeat the critical checks for the MSRV and stable compilers
```

Both scripts read the Rust version matrix from `.github/workflows/ci.yml`, so local runs use the same compiler list as CI. By
default we support the Minimum Supported Rust Version (MSRV) declared in `Cargo.toml` and the latest stable release.

## Recommended workflow

1. Run `cargo fmt` for quick iterations.
2. Use `./test.sh` before committing to exercise formatting, clippy, builds, tests and documentation.
3. Run `./test_rust_versions.sh` ahead of a pull request or when CI flags a version-specific lint.

Pass `./test_rust_versions.sh` a custom list of toolchains to narrow down an issue, for example:

```bash
./test_rust_versions.sh 1.85 stable
```

## Troubleshooting

- Install `rustup` from <https://rustup.rs/> to manage multiple compilers.
- If a version-specific lint fails, install that toolchain with `rustup toolchain install <version>` and re-run the failing
  command locally.
- The ompparser compatibility tests live in `compat/ompparser/`; run their CMake build when touching that layer.
