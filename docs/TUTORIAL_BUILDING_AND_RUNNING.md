# Complete Tutorial: Building and Using the OpenMP Parser

This guide provides step-by-step instructions for building the parser library and running C/C++ examples. **Just copy and paste each command block!**

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Building the Rust Parser Library](#building-the-rust-parser-library)
3. [Running the C Tutorial](#running-the-c-tutorial)
4. [Running the C++ Tutorial](#running-the-c-tutorial-1)
5. [Troubleshooting](#troubleshooting)

---

## Prerequisites

### Required Software

- **Rust** (1.70+ recommended)
- **Clang** (LLVM toolchain for C examples)
- **Clang++** (LLVM toolchain for C++ examples)
- **Git** (to clone the repository)

### Installing Rust (if not installed)

```bash
# Linux/macOS
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Verify installation
rustc --version
cargo --version
```

### Verifying C/C++ Compilers

```bash
# Check Clang (LLVM)
clang --version
clang++ --version

# Should show version 10.0 or higher
```

---

## Building the Rust Parser Library

### Step 1: Navigate to Repository Root

```bash
# If you're in the examples directory, go back to root
cd /workspaces/roup

# Or if you need to clone:
# git clone https://github.com/ouankou/roup.git
# cd roup
```

### Step 2: Build the Library in Release Mode

```bash
# Build the library (optimized)
cargo build --lib --release

# Verify the build succeeded
ls -lh target/release/libroup.*
```

**Expected output:**
```
-rwxr-xr-x ... libroup.so      (Linux)
-rwxr-xr-x ... libroup.dylib   (macOS)
-rwxr-xr-x ... libroup.dll     (Windows - if using MinGW)
```

### Step 3: Run Rust Tests (Optional - Verify Everything Works)

```bash
# Run all tests
cargo test --lib

# You should see:
# test result: ok. 342 passed; 0 failed; 1 ignored; 0 measured
```

---

## Running the C Tutorial

### Method 1: Quick Build and Run (Copy-Paste This!)

```bash
# Navigate to C examples directory
cd examples/c

# Build the tutorial with Clang
clang -o tutorial_basic \
    tutorial_basic.c \
    -I../../include \
    -L../../target/release \
    -lroup \
    -Wl,-rpath,../../target/release

# Run it!
./tutorial_basic
```

### Method 2: Using Absolute Paths (More Reliable)

```bash
# Navigate to C examples directory
cd examples/c

# Get absolute path to project root
PROJECT_ROOT="$(cd ../.. && pwd)"

# Build with absolute paths using Clang
clang -o tutorial_basic \
    tutorial_basic.c \
    -I"${PROJECT_ROOT}/include" \
    -L"${PROJECT_ROOT}/target/release" \
    -lroup \
    -Wl,-rpath,"${PROJECT_ROOT}/target/release"

# Run it!
./tutorial_basic
```

### Method 3: Using LD_LIBRARY_PATH (Alternative)

```bash
cd examples/c

# Build without rpath
clang -o tutorial_basic \
    tutorial_basic.c \
    -I../../include \
    -L../../target/release \
    -lroup

# Run with LD_LIBRARY_PATH
LD_LIBRARY_PATH=../../target/release ./tutorial_basic
```

### Expected Output

```
=== OpenMP Parser Tutorial (C) ===

Step 1: Parsing a simple directive
-----------------------------------
Input: #pragma omp parallel
✓ Successfully parsed!
  Handle: 1

Step 2: Querying directive properties
--------------------------------------
  Kind: 0 (0 = PARALLEL)
  Clause count: 0
  Source location: line 1, column 1
  Language: 0 (C)

[... more output ...]

=== Tutorial Complete! ===
```

---

## Running the C++ Tutorial

### Method 1: Quick Build and Run (Copy-Paste This!)

```bash
# Navigate to C++ examples directory
cd examples/cpp

# Build the tutorial with Clang++
clang++ -o tutorial_basic \
    tutorial_basic.cpp \
    -I../../include \
    -L../../target/release \
    -lroup \
    -std=c++17 \
    -Wl,-rpath,../../target/release

# Run it!
./tutorial_basic
```

### Method 2: Using Absolute Paths

```bash
# Navigate to C++ examples directory
cd examples/cpp

# Get absolute path to project root
PROJECT_ROOT="$(cd ../.. && pwd)"

# Build with absolute paths using Clang++
clang++ -o tutorial_basic \
    tutorial_basic.cpp \
    -I"${PROJECT_ROOT}/include" \
    -L"${PROJECT_ROOT}/target/release" \
    -lroup \
    -std=c++17 \
    -Wl,-rpath,"${PROJECT_ROOT}/target/release"

# Run it!
./tutorial_basic
```

### Method 3: Using LD_LIBRARY_PATH

```bash
cd examples/cpp

# Build without rpath
clang++ -o tutorial_basic \
    tutorial_basic.cpp \
    -I../../include \
    -L../../target/release \
    -lroup \
    -std=c++17

# Run with LD_LIBRARY_PATH
LD_LIBRARY_PATH=../../target/release ./tutorial_basic
```

### Expected Output

```
╔════════════════════════════════════════════════════╗
║   OpenMP Parser Tutorial (C++)                     ║
║   Complete Guide for Beginners                     ║
╚════════════════════════════════════════════════════╝

Step 1: Basic Parsing
=====================

Parsing: #pragma omp parallel
✓ Successfully parsed!
  Handle: 1
  Kind: PARALLEL
  Clauses: 0
  Location: line 1, column 1
  Language: C/C++

[... more output ...]

╔════════════════════════════════════════════════════╗
║   Tutorial Complete!                               ║
╚════════════════════════════════════════════════════╝
```

---

## All-in-One Build Script

### Create and Run Build Script (Linux/macOS)

Copy this entire block:

```bash
# Create build script
cat > build_tutorials.sh << 'EOF'
#!/bin/bash
set -e

echo "========================================"
echo "Building roup OpenMP Parser Tutorials"
echo "========================================"

# Get project root
PROJECT_ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$PROJECT_ROOT"

# Step 1: Build Rust library
echo ""
echo "Step 1: Building Rust library..."
cargo build --lib --release
echo "✓ Rust library built successfully"

# Step 2: Build C tutorial
echo ""
echo "Step 2: Building C tutorial with Clang..."
cd examples/c
clang -o tutorial_basic \
    tutorial_basic.c \
    -I"${PROJECT_ROOT}/include" \
    -L"${PROJECT_ROOT}/target/release" \
    -lroup \
    -Wl,-rpath,"${PROJECT_ROOT}/target/release"
echo "✓ C tutorial built successfully"
cd "$PROJECT_ROOT"

# Step 3: Build C++ tutorial
echo ""
echo "Step 3: Building C++ tutorial with Clang++..."
cd examples/cpp
clang++ -o tutorial_basic \
    tutorial_basic.cpp \
    -I"${PROJECT_ROOT}/include" \
    -L"${PROJECT_ROOT}/target/release" \
    -lroup \
    -std=c++17 \
    -Wl,-rpath,"${PROJECT_ROOT}/target/release"
echo "✓ C++ tutorial built successfully"
cd "$PROJECT_ROOT"

echo ""
echo "========================================"
echo "Build Complete!"
echo "========================================"
echo ""
echo "Run tutorials with:"
echo "  C tutorial:   ./examples/c/tutorial_basic"
echo "  C++ tutorial: ./examples/cpp/tutorial_basic"
echo ""
EOF

# Make it executable
chmod +x build_tutorials.sh

# Run it!
./build_tutorials.sh
```

### Run the Tutorials

```bash
# Run C tutorial
./examples/c/tutorial_basic

# Run C++ tutorial
./examples/cpp/tutorial_basic
```

---

## Platform-Specific Notes

### Linux

Use the commands as shown above. Install LLVM/Clang if needed:

```bash
# Ubuntu/Debian: Install Clang
sudo apt-get update
sudo apt-get install clang

# Fedora/RHEL
sudo dnf install clang

# Verify installation
clang --version
clang++ --version
```

### macOS

Use the commands as shown, but note:
- Library extension is `.dylib` instead of `.so`
- Install Xcode Command Line Tools if needed:

```bash
xcode-select --install
```

### Windows (WSL/MinGW)

**Recommended: Use WSL (Windows Subsystem for Linux)**

```bash
# In WSL, follow Linux instructions
# Install Clang in WSL:
sudo apt-get install clang
```

**Alternative: MSYS2 with Clang**

```bash
# Install MSYS2 from https://www.msys2.org/
# Then in MSYS2 terminal:
pacman -S mingw-w64-clang-x86_64-toolchain

# Use paths with Windows-style backslashes or forward slashes
```

---

## Troubleshooting

### Error: `cannot find -lroup`

**Problem:** Linker cannot find the library.

**Solution:**

```bash
# Verify library was built
ls -lh target/release/libroup.*

# If it doesn't exist, rebuild:
cargo build --lib --release
```

### Error: `error while loading shared libraries: libroup.so`

**Problem:** Runtime cannot find the library.

**Solution 1: Use rpath (recommended)**

```bash
# Rebuild with rpath
clang -o tutorial_basic tutorial_basic.c \
    -I../../include -L../../target/release -lroup \
    -Wl,-rpath,../../target/release
```

**Solution 2: Use LD_LIBRARY_PATH**

```bash
LD_LIBRARY_PATH=../../target/release ./tutorial_basic
```

**Solution 3: Install library system-wide (advanced)**

```bash
sudo cp target/release/libroup.so /usr/local/lib/
sudo ldconfig
```

### Error: `roup.h: No such file or directory`

**Problem:** Compiler cannot find header file.

**Solution:**

```bash
# Verify header exists
ls -lh include/roup.h

# Use absolute path
PROJECT_ROOT="$(pwd)"
clang -I"${PROJECT_ROOT}/include" ...
```

### Error: `undefined reference to ...`

**Problem:** Library is not being linked.

**Solution:**

```bash
# Ensure -lroup comes AFTER the source file
clang tutorial_basic.c -I../../include -L../../target/release -lroup

# Not: clang -lroup tutorial_basic.c (WRONG ORDER!)
```

### Error: C++ compilation fails with `<iomanip>` not found

**Problem:** C++ standard library not found.

**Solution:**

```bash
# Use clang++ instead of clang
clang++ tutorial_basic.cpp -I../../include -L../../target/release -lroup -std=c++17

# Or install Clang C++ support:
# Ubuntu: sudo apt-get install clang libc++-dev
# macOS: xcode-select --install
```

---

## Quick Reference Card

### Essential Build Commands

```bash
# Build Rust library
cargo build --lib --release

# Build C example with Clang
cd examples/c
clang -o tutorial_basic tutorial_basic.c \
    -I../../include -L../../target/release -lroup \
    -Wl,-rpath,../../target/release

# Build C++ example with Clang++
cd examples/cpp
clang++ -o tutorial_basic tutorial_basic.cpp \
    -I../../include -L../../target/release -lroup \
    -std=c++17 -Wl,-rpath,../../target/release
```

### File Locations

```
roup/
├── include/roup.h              # C/C++ header file
├── target/release/libroup.so   # Compiled library (Linux)
├── examples/c/tutorial_basic.c # C tutorial source
└── examples/cpp/tutorial_basic.cpp # C++ tutorial source
```

### Compiler Flags Explained

| Flag | Purpose |
|------|---------|
| `-I../../include` | Include header directory |
| `-L../../target/release` | Library search path |
| `-lroup` | Link against libroup |
| `-Wl,-rpath,...` | Embed library path in binary |
| `-std=c++17` | Use C++17 standard (C++ only) |

---

## Next Steps

After completing the tutorials:

1. **Read the API documentation** in `include/roup.h`
2. **Check other examples** in `examples/c/` and `examples/cpp/`
3. **Read the safety analysis** in `docs/MINIMAL_UNSAFE_SUMMARY.md`
4. **Explore OpenMP features** in `docs/OPENMP_SUPPORT.md`

---

## Getting Help

- **GitHub Issues:** https://github.com/ouankou/roup/issues
- **Documentation:** See `docs/` directory
- **Examples:** See `examples/` directory

---

**Happy parsing! 🚀**
