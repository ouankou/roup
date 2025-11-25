# ompparser Compatibility Layer

ROUP provides a drop-in compatibility layer for projects using [ompparser](https://github.com/ouankou/ompparser), allowing you to switch to ROUP's faster, safer Rust-based parser without changing your code.

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
5. Run the upstream ompparser ctest suite

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

The shim is validated via the ompparser ctest suite bundled in the submodule. It covers directive kinds (including combined/end-paired forms), clause parameters, unparsing, and language modes.

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

The shim uses the generated `ROUP_OMPD_*`/`ROUP_OMPC_*` constants from `src/roup_constants.h` and preserves the canonical keyword spellings from ROUP. Legacy ompparser enum values are mapped internally.

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
