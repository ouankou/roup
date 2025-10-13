# ompparser Compatibility Layer

⚠️ **Experimental Feature** - This compatibility layer is under active development.

ROUP provides a drop-in compatibility layer for projects using [ompparser](https://github.com/ouankou/ompparser), allowing you to switch to ROUP's expected-to-be faster, safer Rust-based parser without changing your code.

## What is it?

A compatibility layer that provides:
- **Same API** as ompparser - no code changes needed
- **Drop-in replacement** via `libompparser.so` 
- **ROUP backend** - expected-to-be faster, safer parsing in Rust
- **Reuses ompparser methods** - toString(), generateDOT(), etc. (zero duplication)

## Quick Start

### One-Command Build

```bash
cd compat/ompparser
./build.sh
```

The script will:
1. Check prerequisites (git, cmake, gcc, cargo)
2. Initialize ompparser submodule
3. Build ROUP core library
4. Build `libompparser.so` (size varies by build configuration)
5. Run all 46 tests

### Manual Build

```bash
# 1. Initialize ompparser submodule
git submodule update --init --recursive

# 2. Build ROUP core
cd /path/to/roup
cargo build --release

# 3. Build compatibility layer
cd compat/ompparser
mkdir -p build && cd build
cmake ..
make

# 4. Run tests
ctest --output-on-failure
```

## Usage

### Drop-in Replacement

Install system-wide and use exactly like original ompparser:

```bash
# Install
cd compat/ompparser/build
sudo make install
sudo ldconfig

# Use (unchanged!)
g++ mycompiler.cpp -lompparser -o mycompiler
```

### Code Example

Your existing ompparser code works without changes:

```cpp
#include <OpenMPIR.h>
#include <iostream>

int main() {
    // Parse OpenMP directive
    OpenMPDirective* dir = parseOpenMP("omp parallel num_threads(4)", nullptr);
    
    if (dir) {
        // Use ompparser methods (all work!)
        std::cout << "Kind: " << dir->getKind() << std::endl;
        std::cout << "String: " << dir->toString() << std::endl;
        
        // Access clauses
        auto* clauses = dir->getAllClauses();
        std::cout << "Clauses: " << clauses->size() << std::endl;
        
        delete dir;
    }
    
    return 0;
}
```

### CMake Integration

**Option 1: pkg-config**
```cmake
find_package(PkgConfig REQUIRED)
pkg_check_modules(OMPPARSER REQUIRED ompparser)

target_link_libraries(your_app ${OMPPARSER_LIBRARIES})
target_include_directories(your_app PRIVATE ${OMPPARSER_INCLUDE_DIRS})
```

**Option 2: Direct linking**
```cmake
target_link_libraries(your_app
    ${PATH_TO_ROUP}/compat/ompparser/build/libompparser.so
)
```

## What's Included

### libompparser.so

Single self-contained library with:
- **ROUP parser** (statically embedded) - Rust-based, safe parsing
- **ompparser methods** - toString, generateDOT, etc.
- **Compatibility wrapper** - Seamless integration layer
- **Self-contained** - No libroup.so dependency (system libs via libc)

### Comprehensive Testing

46 tests covering:
- **Basic directives**: parallel, for, sections, single, task, barrier, taskwait, critical, master
- **Clauses**: num_threads, private, shared, reduction, schedule, if, nowait, etc.
- **String generation**: toString(), generatePragmaString()
- **Error handling**: null input, invalid directives, malformed pragmas
- **Memory management**: allocations, deletion, reuse
- **Language modes**: C, C++, Fortran via setLang()

Run tests:
```bash
cd compat/ompparser/build
ctest --output-on-failure
```

## Architecture

```text
Your Application (OpenMP directives to parse)
    ↓
compat_impl.cpp (~190 lines) - Minimal wrapper
    ↓
ROUP C API (roup_parse, roup_directive_kind, etc.)
    ↓
ROUP Rust Parser (safe parser core)
    ↓
Returns: OpenMPDirective with ompparser methods
```

**Key Design**:
- Reuses 90% of ompparser code (no duplication)
- Git submodule approach - automatic ompparser upgrades
- Minimal unsafe code (~60 lines, 0.9%), all at FFI boundary

## Known Limitations

### 1. Combined Directives ⚠️

Combined directives like `parallel for` are currently parsed as the first directive only.

**Example**:
```cpp
parseOpenMP("omp parallel for", nullptr)
// Returns: OMPD_parallel (should be OMPD_parallel_for)
```

**Status**: ROUP core limitation, tracked for future improvement.

**Workaround**: Tests document expected behavior with clear warnings.

### 2. Clause Parameters 🔄

Basic clause detection works, but parameter extraction not yet implemented.

**Example**:
```cpp
parseOpenMP("omp parallel num_threads(4)", nullptr)
// Detects: num_threads clause ✅
// Extracts "4": ❌ (TODO)
```

**Status**: Planned wrapper enhancement using ROUP's clause expression API.

## Documentation

Complete documentation in `compat/ompparser/`:

- **[README.md](https://github.com/ouankou/roup/blob/main/compat/ompparser/README.md)** - Complete compatibility layer guide with build instructions and examples

For detailed ROUP API documentation, see [API Reference](./api-reference.md).

## Requirements

- **Rust toolchain** (for ROUP core)
- **CMake 3.10+**
- **C++11 compiler** (gcc/clang)
- **Git** (for submodule management)

## CI/CD

The compatibility layer is tested automatically via GitHub Actions (`.github/workflows/build.yml`):

```yaml
- Tests ROUP core (always)
- Tests compat layer (if submodule initialized)
- Verifies library builds successfully
- Validates drop-in functionality
- Checks constants synchronization (checksum validation)
```

## FAQ

**Q: Do I need to change my code?**  
A: No! It's a drop-in replacement with the same API.

**Q: What if I don't need compat layer?**  
A: ROUP works perfectly standalone. The compat layer is optional.

**Q: How do I get ompparser upgrades?**  
A: `git submodule update --remote` pulls latest ompparser automatically.

**Q: What about performance?**  
A: ROUP is expected to be faster than original ompparser due to Rust optimizations.

**Q: Is it stable?**  
A: ⚠️ Experimental stage - thoroughly tested (46 tests) but under active development.

## Support

- **Issues**: [GitHub Issues](https://github.com/ouankou/roup/issues)
- **Discussions**: [GitHub Discussions](https://github.com/ouankou/roup/discussions)
- **Email**: See [Contributing Guide](./contributing.md)
