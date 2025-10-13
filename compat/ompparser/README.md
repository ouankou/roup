# ROUP ompparser Compatibility Layer

⚠️ **Experimental Feature** - Under active development  
**Tests**: 46/46 passing (100%) 🎯

Drop-in replacement for [ompparser](https://github.com/ouankou/ompparser) using ROUP as the backend parser.

## Quick Start

```bash
./build.sh
```

The script will:
- ✅ Check prerequisites (git, cmake, gcc, cargo)
- ✅ Initialize ompparser submodule
- ✅ Build ROUP core library
- ✅ Build `libompparser.so` (~5.5MB with static ROUP embedding)
- ✅ Run all 46 tests

## What You Get

- **libompparser.so** - Drop-in replacement with ROUP parser + ompparser methods
- **libroup-ompparser-compat.a** - Static library for ROUP-specific builds

## Manual Build

```bash
# 1. Initialize submodule
git submodule update --init --recursive

# 2. Build ROUP
cd ../.. && cargo build --release && cd compat/ompparser

# 3. Build compat layer
mkdir -p build && cd build
cmake .. && make

# 4. Run tests
ctest --output-on-failure
```

## Documentation

**Full documentation**: See [ompparser Compatibility Layer](../../docs/book/src/ompparser-compat.md) in the ROUP book.

**Files in this directory**:
## Project Structure

- `build.sh` - One-command build script
- `CMakeLists.txt` - Build configuration
- `src/compat_impl.cpp` - Compatibility wrapper (~190 lines)
- `ompparser/` - Submodule (provides headers and implementation sources)
- `examples/` - Example code showing usage
- `tests/` - Test suite (46 tests)

## Architecture

```
Your Code (uses parseOpenMP, OpenMPDirective API)
    ↓
compat_impl.cpp (compatibility wrapper)
    ↓
ROUP C API (roup_parse, roup_directive_kind, etc.)
    ↓
ROUP Parser (Rust - safe, fast parsing)
```

The library reuses ompparser's own implementation for toString(), generateDOT(), and other methods (zero duplication).

## Requirements

- CMake 3.10+
- C++11 compiler
- Rust toolchain (for building ROUP)
- Git (for submodules)

## License

Apache-2.0 (same as ROUP core)
