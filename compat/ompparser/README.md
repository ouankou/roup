# ROUP ompparser Compatibility Layer

This directory provides a drop-in replacement for the original
[ompparser](https://github.com/ouankou/ompparser) library using ROUP as the
backend parser.  The goal is binary compatibility with existing tools that link
against ompparser while benefiting from the Rust implementation.

## Quick start

```bash
./build.sh
```

The script initialises the submodule, builds ROUP in release mode, compiles the
compatibility shim, and runs the accompanying CMake tests.

## Manual steps

```bash
# 1. Ensure the submodule is available
git submodule update --init --recursive

# 2. Build ROUP
cd ../..
cargo build --release
cd compat/ompparser

# 3. Configure and build the compat layer
mkdir -p build && cd build
cmake ..
cmake --build .

# 4. Execute the tests
ctest --output-on-failure
```

Outputs include `libompparser.so`/`dylib` plus the static convenience archive
`libroup-ompparser-compat.a`.

## Directory layout

- `build.sh` – helper that performs the full build and test flow.
- `CMakeLists.txt` – project configuration.
- `src/compat_impl.cpp` – translation layer that maps ompparser calls to the
  ROUP C API.
- `examples/` – sample programs using the compatibility layer.
- `tests/` – small regression suite executed by CTest.

## Requirements

- CMake 3.10 or later.
- A C++11 or newer compiler.
- A Rust toolchain capable of building ROUP (see the root README).

## Documentation

Further background is available in `docs/book/src/ompparser-compat.md`.
