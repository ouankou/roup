# ROUP ompparser compatibility layer

A drop-in replacement for [ompparser](https://github.com/ouankou/ompparser) that swaps in ROUP for directive parsing while
preserving the original C++ interface. The bridge remains experimental but is covered by 46 automated tests.

## Quick start

```bash
./build.sh
```

The helper script verifies `git`, `cmake`, `gcc`, and `cargo`, initialises the ompparser submodule, builds ROUP in release mode,
and produces both the shared and static compatibility libraries before executing the bundled tests.

## Manual build

```bash
# Initialise the submodule
git submodule update --init --recursive

# Build ROUP
cd ../.. && cargo build --release && cd compat/ompparser

# Configure and build the compatibility layer
mkdir -p build && cd build
cmake ..
cmake --build .

# Run the C++ tests
ctest --output-on-failure
```

## Layout

- `build.sh` – single-command build helper
- `CMakeLists.txt` – project configuration
- `src/compat_impl.cpp` – bridge from the ompparser API to the ROUP C API
- `ompparser/` – vendored headers and support code from upstream
- `examples/` – sample integration code
- `tests/` – regression suite mirroring the original project

## Architecture

```
Your code → compat_impl.cpp → ROUP C API → ROUP parser
```

Existing ompparser helpers such as `toString` and `generateDOT` stay intact; only the parser backend changes. Building requires
CMake 3.10+, a C++11 compiler, a Rust toolchain, and Git for submodules.

## Documentation and licence

Additional details live in the [mdBook chapter](../../docs/book/src/ompparser-compat.md). The compatibility layer is distributed
under the Apache-2.0 licence, matching the ROUP core.
