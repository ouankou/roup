# Testing Guide

The project uses a "MSRV + stable" policy. The minimum supported Rust version is **1.85.0** and the CI matrix also runs on the
latest stable release across Linux, Windows, and macOS.

## Essential Commands

```bash
cargo test                     # Fast local sanity check
./test.sh                      # Full suite: formatting, builds, docs, compat layer
./test_rust_versions.sh        # Run the full check on MSRV and stable
./test_rust_versions.sh 1.85 stable  # Optional explicit list
```

`test_rust_versions.sh` reads `.github/workflows/ci.yml` to stay in sync with the CI configuration. When the MSRV changes, update
`Cargo.toml`, the workflow file, and this document before running the script.

## When to Run What

| Situation | Command |
| --- | --- |
| Local development | `cargo test` |
| Before sending a PR | `./test.sh` |
| Verifying toolchain compatibility | `./test_rust_versions.sh` |

## MSRV Updates

Only bump the MSRV when a new language feature or dependency requires it. After changing the version:

1. Edit `Cargo.toml` (`rust-version = "X.Y.Z"`).
2. Update the CI matrix (`.github/workflows/ci.yml`).
3. Refresh this file and any related documentation.
4. Run `./test_rust_versions.sh` to confirm both toolchains pass.
