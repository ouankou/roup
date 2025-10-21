# ROUP ompparser compatibility layer

This directory builds a ROUP-powered `libompparser` that preserves the original headers and IR while replacing the parser implementation.

## Requirements

- Rust toolchain (`cargo`)
- CMake ≥ 3.10
- A C++17 compiler

## Build and test

```bash
./build.sh        # initialises the submodule, builds libroup, compiles the shim, and runs ctest
```

To drive CMake manually:

```bash
git submodule update --init --recursive
cargo build --release
cd compat/ompparser
mkdir -p build && cd build
cmake ..
make
ctest --output-on-failure
```

Set `LD_LIBRARY_PATH` (Linux) or `DYLD_LIBRARY_PATH` (macOS) so the runtime loader can find `libompparser` and `libroup`.

## Linking

```bash
g++ app.cpp \
    -I/path/to/roup/compat/ompparser/ompparser/include \
    -L/path/to/roup/compat/ompparser/build -lompparser -lroup \
    -o app
```

The compatibility layer retains the `parseOpenMP` entry point and directive/clause enums from ompparser. Behavioural notes and API references live in [`docs/book/src/ompparser-compat.md`](../../docs/book/src/ompparser-compat.md).

## Troubleshooting

- Ensure the submodule is initialised if headers are missing.
- Build the core crate (`cargo build --release`) before invoking CMake.
- Export the appropriate library search path if executables cannot load the rebuilt libraries.
