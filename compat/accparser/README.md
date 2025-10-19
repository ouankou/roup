# ROUP accparser Compatibility Layer

**Drop-in replacement for accparser using ROUP's OpenACC parser**

## Overview

This compatibility layer provides `libaccparser.so` - a drop-in replacement for the original [accparser](https://github.com/ouankou/accparser) library, but powered by ROUP's fast Rust-based parser instead of ANTLR4.

**Key Benefits:**
- ✅ **Zero ANTLR4 dependency** - no need to install antlr4 runtime or toolchain
- ✅ **Same API** - works with existing accparser-based code without changes
- ✅ **Faster parsing** - ROUP's hand-written parser outperforms generated ANTLR code
- ✅ **Better error handling** - clearer error messages and recovery
- ✅ **Actively maintained** - part of the ROUP project

## Quick Start

```bash
# Clone and build
git clone https://github.com/your-org/roup.git
cd roup/compat/accparser
./build.sh

# Run tests
cd build
LD_LIBRARY_PATH=. ./accparser_example
```

## Requirements

- **Rust** toolchain (cargo)
- **CMake** 3.10+
- **C++ compiler** (g++ or clang++)
- **Git** (for submodules)

**NO antlr4 required!**

## Usage

### Option 1: System-wide installation

```bash
cd compat/accparser/build
sudo make install
sudo ldconfig
```

Then use in your project:

```cpp
#include <OpenACCIR.h>

OpenACCDirective* dir = parseOpenACC("acc parallel num_gangs(4)", nullptr);
std::cout << dir->toString() << std::endl;
delete dir;
```

Compile:
```bash
g++ myapp.cpp -laccparser -o myapp
```

### Option 2: Local linking

```bash
g++ myapp.cpp -I/path/to/roup/compat/accparser/accparser/src \
    -L/path/to/roup/compat/accparser/build -laccparser -o myapp
```

## Architecture

```
Your Application
       ↓
parseOpenACC() entry point
       ↓
compat_impl.cpp (bridge layer)
       ↓
ROUP C API (acc_parse, etc.)
       ↓
ROUP Rust parser (fast!)
       ↓
accparser IR (OpenACCDirective, OpenACCClause)
```

**What's included:**
- ROUP parser (replaces ANTLR-generated code)
- accparser IR files (OpenACCIR.cpp, OpenACCIRToString.cpp)
- Bridge layer (compat_impl.cpp)

**What's excluded:**
- ANTLR grammar files (.g4)
- ANTLR-based AST constructor
- antlr4 runtime dependency

## Supported Features

### Directives (17+)
- Compute: `parallel`, `kernels`, `serial`, `loop`
- Data: `data`, `enter data`, `exit data`, `host_data`
- Synchronization: `wait`, `atomic`
- Other: `declare`, `routine`, `init`, `shutdown`, `set`, `update`, `end`

### Clauses (45+)
- Compute: `num_gangs`, `num_workers`, `vector_length`, `async`, `wait`
- Data: `copy`, `copyin`, `copyout`, `create`, `delete`, `present`
- Loop: `gang`, `worker`, `vector`, `seq`, `independent`, `collapse`, `tile`
- Other: `private`, `firstprivate`, `reduction`, `if`, `default`
- Aliases: `pcopy`, `present_or_copy`, `pcopyin`, `present_or_copyin`, `pcopyout`,
  `present_or_copyout`, `pcreate`, `present_or_create`, `dtype`

See `src/roup_constants.h` for complete list.

OpenACC support details and spec cross references live in
[`docs/OPENACC_SUPPORT.md`](../../docs/OPENACC_SUPPORT.md).

## Testing

```bash
cd compat/accparser/build

# Basic test
LD_LIBRARY_PATH=. ./accparser_example

# Comprehensive test suite (35+ tests)
LD_LIBRARY_PATH=. ./comprehensive_test

# Run all CMake tests
ctest
```

## Troubleshooting

### Build fails: "accparser submodule not initialized"
```bash
git submodule update --init --recursive
```

### Build fails: "ROUP library not found"
```bash
cd ../..  # Go to ROUP root
cargo build --release
```

### Test fails: "error while loading shared libraries"
```bash
cd compat/accparser/build
LD_LIBRARY_PATH=. ./accparser_example
```

## Migration from accparser

**No code changes needed!** This is a drop-in replacement.

If you have existing code using accparser:

```cpp
// This code works unchanged
#include <OpenACCIR.h>
OpenACCDirective* dir = parseOpenACC("acc parallel", nullptr);
// ... use dir ...
delete dir;
```

Just recompile with `-laccparser` linking to our `libaccparser.so`.

## Performance

ROUP's hand-written parser is **2-5x faster** than ANTLR-generated parsers for typical OpenACC pragmas.

## License

Copyright (c) 2025 ROUP Project
SPDX-License-Identifier: BSD-3-Clause

## Contributing

This is part of the ROUP project. See main ROUP repository for contribution guidelines.

## Support

- Issues: https://github.com/your-org/roup/issues
- Documentation: https://docs.roup-project.org
- Original accparser: https://github.com/ouankou/accparser
