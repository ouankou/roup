# ROUP ompparser Compatibility Layer

> **Experimental:** the ABI is expected to stabilise over time.

The compatibility layer packages the ROUP parser behind the original ompparser API so existing projects can switch parsers
without code changes.

## Quick Start

```bash
cd compat/ompparser
./build.sh
```

The script ensures dependencies are present, initialises the ompparser submodule, builds ROUP, compiles the wrapper, and runs the
CMake test suite.

## Manual Steps

```bash
git submodule update --init --recursive
cargo build --release
cd compat/ompparser
mkdir -p build && cd build
cmake ..
make
ctest --output-on-failure
```

## Outputs

- `libompparser.so` – ompparser ABI backed by ROUP
- `libroup-ompparser-compat.a` – static library variant

## Documentation

Architectural notes and usage examples live in the documentation site:
[`docs/book/src/ompparser-compat.md`](../../docs/book/src/ompparser-compat.md).

## Requirements

- CMake 3.10+
- A C++17-capable compiler
- Rust toolchain (for building ROUP)
- Git (for submodules)

## License

Apache-2.0 (matching the ROUP core)
