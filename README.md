# ROUP

Rust-first parser for OpenMP pragmas with optional C and C++ bindings. ROUP is still experimental, but the core parser and FFI
layer are usable today for research tooling and prototype compilers.

## Quick start

### Rust
```toml
[dependencies]
roup = "0.4"
```

### C or C++
```bash
cargo build --release
# Link against target/release/libroup.{a,so,dylib}
```

See the [getting started guide](https://roup.ouankou.com/getting-started.html) for platform specific instructions and build
flags.

## Highlights

- OpenMP 3.0–6.0 directive and clause coverage.
- Safe-by-default Rust implementation with a narrow `unsafe` surface for FFI.
- Optional C and C++ wrappers plus an ompparser-compatible layer.
- Comprehensive documentation at [roup.ouankou.com](https://roup.ouankou.com).

## Documentation map

- [Tutorials](https://roup.ouankou.com/rust-tutorial.html) for Rust, C and C++ users.
- [Architecture notes](https://roup.ouankou.com/architecture.html) covering parser and IR design.
- [OpenMP support matrix](https://roup.ouankou.com/openmp-support.html).
- [Contributing guide](https://roup.ouankou.com/contributing.html) with coding standards and review expectations.

## Development workflow

```bash
# Format, lint, build and test
./test.sh

# Verify against the MSRV and the latest stable compiler
./test_rust_versions.sh
```

The scripts derive the compiler list from `.github/workflows/ci.yml`, keeping local runs aligned with CI. Additional compatibility
checks for the ompparser bridge live under `compat/ompparser/`.

## Support channels

Questions and bug reports are tracked on the [GitHub issue tracker](https://github.com/ouankou/roup/issues). Discussions and
feature proposals are welcome in the repository discussions board.
