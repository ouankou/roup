# Building Guide

This guide covers building ROUP and integrating it into your projects across different languages and platforms.

---

## Quick Start

```bash
# Clone the repository
git clone https://github.com/ouankou/roup.git
cd roup

# Build the library (release mode, optimized)
cargo build --release

# Run tests to verify
cargo test

# Build is complete! Library at: target/release/libroup.{a,so,dylib,dll}
```

**Next steps:**
- **Rust users**: Add ROUP as a dependency (see below)
- **C users**: Link against `libroup.a` (see [C Tutorial](./c-tutorial.md))
- **C++ users**: Use RAII wrappers (see [C++ Tutorial](./cpp-tutorial.md))

---

## Rust Integration

### Option 1: Using crates.io (Recommended)

Add to your `Cargo.toml`:

```toml
[dependencies]
roup = "0.5"
```

Then use in your code:

```rust,ignore
use roup::parser::parse;

fn main() {
    let result = parse("#pragma omp parallel for");
    match result {
        Ok(directive) => println!("Parsed: {:?}", directive),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

### Option 2: Using Git Dependency

```toml
[dependencies]
roup = { git = "https://github.com/ouankou/roup.git" }
```

### Option 3: Local Development

```toml
[dependencies]
roup = { path = "../roup" }
```

### Building Your Rust Project

```bash
# Standard cargo commands
cargo build           # Debug build
cargo build --release # Optimized build
cargo test            # Run tests
cargo doc --open      # Generate and view docs
```

See [Rust Tutorial](./rust-tutorial.md) for complete usage examples.

---

## C Integration

### Prerequisites

- **Rust toolchain** (to build libroup)
- **C compiler**: GCC, Clang, or MSVC
- **Build tools**: Make or CMake (optional)

### Step 1: Build the ROUP Library

```bash
cd /path/to/roup
cargo build --release
```

This creates:
- **Linux**: `target/release/libroup.{a,so}`
- **macOS**: `target/release/libroup.{a,dylib}`
- **Windows**: `target/release/roup.{lib,dll}`

### Step 2: Create Header File

Create `roup_ffi.h` with function declarations (see [C Tutorial](./c-tutorial.md#step-1-setup-and-compilation) for complete header).

### Step 3: Compile Your C Program

#### Using GCC/Clang (Linux/macOS)

```bash
# Static linking
gcc -o myapp main.c \
    -I/path/to/roup_ffi.h \
    -L/path/to/roup/target/release \
    -lroup \
    -lpthread -ldl -lm

# Dynamic linking with rpath
gcc -o myapp main.c \
    -I/path/to/roup_ffi.h \
    -L/path/to/roup/target/release \
    -lroup \
    -Wl,-rpath,/path/to/roup/target/release \
    -lpthread -ldl -lm
```

#### Using CMake

```cmake
cmake_minimum_required(VERSION 3.10)
project(MyApp C)

# Add ROUP library
add_library(roup STATIC IMPORTED)
set_target_properties(roup PROPERTIES
    IMPORTED_LOCATION "/path/to/roup/target/release/libroup.a"
)

# Create executable
add_executable(myapp main.c)
target_include_directories(myapp PRIVATE "/path/to/roup_ffi.h")
target_link_libraries(myapp roup pthread dl m)
```

#### Using MSVC (Windows)

```cmd
REM Build ROUP library first
cargo build --release

REM Compile C program
cl.exe main.c /I"C:\path\to\roup" ^
    /link "C:\path\to\roup\target\release\roup.lib" ^
    ws2_32.lib userenv.lib
```

### Step 4: Run Your Program

```bash
# If using static linking
./myapp

# If using dynamic linking without rpath
LD_LIBRARY_PATH=/path/to/roup/target/release ./myapp

# Windows
set PATH=C:\path\to\roup\target\release;%PATH%
myapp.exe
```

### Complete Example

See `examples/c/tutorial_basic.c` for a full working example with build instructions.

---

## C++ Integration

C++ programs use the same C API with optional RAII wrappers for automatic memory management.

### Step 1: Build ROUP Library

```bash
cargo build --release
```

### Step 2: Create RAII Wrappers

See [C++ Tutorial - Step 2](./cpp-tutorial.md#step-2-create-raii-wrappers-modern-c) for complete wrapper code.

### Step 3: Compile with C++17

```bash
# Using g++
g++ -o myapp main.cpp \
    -I/path/to/roup_ffi.h \
    -I/path/to/roup_wrapper.hpp \
    -L/path/to/roup/target/release \
    -lroup \
    -std=c++17 \
    -lpthread -ldl -lm

# Using Clang++
clang++ -o myapp main.cpp \
    -I/path/to/roup_ffi.h \
    -I/path/to/roup_wrapper.hpp \
    -L/path/to/roup/target/release \
    -lroup \
    -std=c++17 \
    -lpthread -ldl -lm
```

### CMake for C++

```cmake
cmake_minimum_required(VERSION 3.10)
project(MyApp CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_library(roup STATIC IMPORTED)
set_target_properties(roup PROPERTIES
    IMPORTED_LOCATION "/path/to/roup/target/release/libroup.a"
)

add_executable(myapp main.cpp)
target_include_directories(myapp PRIVATE 
    "/path/to/roup_ffi.h"
    "/path/to/roup_wrapper.hpp"
)
target_link_libraries(myapp roup pthread dl m)
```

---

## Platform-Specific Notes

### Linux

#### Ubuntu/Debian

```bash
# Install build tools
sudo apt-get update
sudo apt-get install build-essential curl git

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build ROUP
cargo build --release
```

#### Fedora/RHEL

```bash
# Install build tools
sudo dnf install gcc git

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build ROUP
cargo build --release
```

**Library location**: `target/release/libroup.{a,so}`

### macOS

```bash
# Install Xcode Command Line Tools
xcode-select --install

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build ROUP
cargo build --release
```

**Library location**: `target/release/libroup.{a,dylib}`

**Note**: On macOS, the dynamic library extension is `.dylib`, not `.so`.

### Windows

#### Using Rust with MSVC

```powershell
# Install Rust (uses MSVC toolchain by default on Windows)
# Download from: https://rustup.rs/

# Install Visual Studio Build Tools
# Download from: https://visualstudio.microsoft.com/downloads/

# Build ROUP
cargo build --release
```

**Library location**: `target\release\roup.{lib,dll}`

#### Using Rust with GNU (MinGW)

```bash
# Install MSYS2 from https://www.msys2.org/
# Then in MSYS2 terminal:

# Install MinGW toolchain
pacman -S mingw-w64-x86_64-gcc mingw-w64-x86_64-rust

# Build ROUP
cargo build --release
```

#### WSL (Recommended for C/C++)

Windows Subsystem for Linux provides a full Linux environment:

```bash
# In WSL (Ubuntu)
sudo apt-get install build-essential
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
cargo build --release
```

---

## Build Configurations

### Debug Build (Development)

```bash
cargo build

# Output: target/debug/libroup.{a,so}
# Features: Debug symbols, assertions, slower but better errors
```

### Release Build (Production)

```bash
cargo build --release

# Output: target/release/libroup.{a,so}
# Features: Optimized, no debug symbols, faster execution
```

### Release with Debug Info

```bash
cargo build --release --config profile.release.debug=true

# Output: target/release/libroup.{a,so}
# Features: Optimized but with debug symbols for profiling
```

### Custom Features

```bash
# Build with all features
cargo build --all-features

# Build with specific feature
cargo build --features "serde"

# Build without default features
cargo build --no-default-features
```

---

## Testing

### Run All Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_parallel_directive

# Run tests for specific module
cargo test parser::
```

### Run Examples

```bash
# List available examples
cargo run --example

# Run specific example (if any exist)
cargo run --example parse_simple
```

### Benchmarks

```bash
# If benchmarks are available
cargo bench
```

---

## Troubleshooting

### "linker `cc` not found"

**Problem**: No C compiler installed.

**Solution**:
```bash
# Linux
sudo apt-get install build-essential

# macOS
xcode-select --install

# Windows
# Install Visual Studio Build Tools
```

### "cannot find -lroup"

**Problem**: Linker can't find the ROUP library.

**Solution**:
```bash
# Verify library exists
ls -lh target/release/libroup.*

# Rebuild if needed
cargo build --release

# Check linker path
gcc ... -L$(pwd)/target/release -lroup
```

### "error while loading shared libraries: libroup.so"

**Problem**: Runtime can't find dynamic library.

**Solution 1 - rpath**:
```bash
gcc ... -Wl,-rpath,/path/to/roup/target/release
```

**Solution 2 - LD_LIBRARY_PATH**:
```bash
export LD_LIBRARY_PATH=/path/to/roup/target/release:$LD_LIBRARY_PATH
./myapp
```

**Solution 3 - Install system-wide**:
```bash
sudo cp target/release/libroup.so /usr/local/lib/
sudo ldconfig
```

### Rust Version Too Old

**Problem**: Compilation fails with version error.

**Solution**:
```bash
# Update Rust toolchain
rustup update stable

# Verify version
rustc --version
# Ensure your Rust version is at least 1.85.0
```

### Windows: "VCRUNTIME140.dll missing"

**Problem**: Missing Visual C++ runtime.

**Solution**: Download and install [Visual C++ Redistributable](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist)

---

## Build Performance Tips

### Faster Incremental Builds

```bash
# Use sccache for caching
cargo install sccache
export RUSTC_WRAPPER=sccache

# Use faster linker (Linux)
sudo apt-get install lld
export RUSTFLAGS="-C link-arg=-fuse-ld=lld"
```

### Parallel Builds

```bash
# Use all CPU cores (default)
cargo build -j $(nproc)

# Limit parallel jobs
cargo build -j 4
```

### Reduce Binary Size

```toml
# Add to Cargo.toml
[profile.release]
opt-level = "z"     # Optimize for size
lto = true          # Link-time optimization
codegen-units = 1   # Better optimization
strip = true        # Remove debug symbols
```

---

## Cross-Compilation

### Linux to Windows

```bash
# Install target
rustup target add x86_64-pc-windows-gnu

# Install MinGW
sudo apt-get install mingw-w64

# Build
cargo build --release --target x86_64-pc-windows-gnu
```

### macOS to Linux

```bash
# Install target
rustup target add x86_64-unknown-linux-gnu

# Build (requires cross-compilation setup)
cargo build --release --target x86_64-unknown-linux-gnu
```

For more complex cross-compilation, consider [cross](https://github.com/cross-rs/cross):

```bash
cargo install cross
cross build --release --target x86_64-unknown-linux-gnu
```

---

## IDE Setup

### Visual Studio Code

Recommended extensions:
- **rust-analyzer** - Language server
- **CodeLLDB** - Debugger
- **crates** - Dependency management

### CLion / IntelliJ IDEA

Install the Rust plugin from JetBrains marketplace.

### Vim/Neovim

Use rust-analyzer with your LSP client (coc.nvim, nvim-lspconfig, etc.)

---

## Next Steps

After building successfully:

- **Rust developers**: See [Rust Tutorial](./rust-tutorial.md)
- **C developers**: See [C Tutorial](./c-tutorial.md)
- **C++ developers**: See [C++ Tutorial](./cpp-tutorial.md)
- **API Reference**: See [API Reference](./api-reference.md)

---

## Getting Help

- **Build issues**: Check [GitHub Issues](https://github.com/ouankou/roup/issues)
- **Questions**: See [FAQ](./faq.md)
- **Examples**: Browse `examples/` directory
- **Documentation**: Read `docs/` directory
