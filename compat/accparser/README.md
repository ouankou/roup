# ROUP accparser compatibility layer

Drop-in replacement for [accparser](https://github.com/ouankou/accparser), powered by ROUP. Passes all 914 accparser tests without ANTLR4.

## Quick start

```bash
mkdir build && cd build
cmake ..
make -j
ctest
```

Produces `libaccparser.so` compatible with existing accparser applications.

## Requirements

- Rust toolchain
- CMake 3.10+
- C++ compiler

No ANTLR4 required.

## Build details

CMake auto-builds ROUP and links all 914 accparser tests. Uses submodule headers directly without modification.

## Implementation

- Parsing: ROUP (Rust)
- AST & unparsing: Original accparser classes
- Headers: Direct from submodule (OpenACCParser.h)
- No runtime generation, no ANTLR4 dependency
