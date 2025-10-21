# ROUP accparser compatibility layer

This directory provides a drop-in replacement for the original [accparser](https://github.com/ouankou/accparser) library, powered by ROUP's OpenACC parser.

## Quick start

```bash
./build.sh            # configure, build, and run the compat tests
```

The script initialises the submodule, builds ROUP in release mode, compiles `libaccparser.so`, and executes the bundled tests.

## Requirements

- Rust toolchain (for building ROUP)
- CMake 3.10+
- C++ compiler (g++ or clang++)
- Git for submodules

No ANTLR dependency is required.

## Manual build

```bash
git submodule update --init --recursive
cd ../.. && cargo build --release && cd compat/accparser
mkdir -p build && cd build
cmake ..
make
ctest --output-on-failure
```

Link `build/libaccparser.so` (or the corresponding static library) into existing applications. Header files live under `compat/accparser/accparser/src`.

## Feature parity

- Covers the same directive and clause surface as the upstream accparser while using ROUP's keyword tables.【F:docs/OPENACC_SUPPORT.md†L1-L53】
- Preserves alias spellings when round-tripping directives through `OpenACCIR::toString`.【F:compat/accparser/tests/comprehensive_test.cpp†L1-L320】
- Shares numeric identifiers with the C API so aliases and canonical names compare identically.【F:tests/openacc_c_api.rs†L9-L76】

## Testing

The compatibility tests mirror the main suite:

```bash
cd compat/accparser/build
LD_LIBRARY_PATH=. ./accparser_example
LD_LIBRARY_PATH=. ./comprehensive_test
ctest
```

## Support

Report issues at <https://github.com/ouankou/roup/issues>. Documentation for the compatibility layer lives in the book chapter [`docs/book/src/accparser-compat.md`](../../docs/book/src/accparser-compat.md).
