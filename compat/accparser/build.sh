#!/bin/bash
# ROUP accparser Drop-in Replacement - One-Command Build Script
# Usage: ./build.sh

set -e  # Exit on error

echo "======================================"
echo "  ROUP accparser Build Script"
echo "======================================"
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Status functions
status() { echo -e "${GREEN}[✓]${NC} $1"; }
error() { echo -e "${RED}[✗]${NC} $1"; exit 1; }
warn() { echo -e "${YELLOW}[!]${NC} $1"; }

# Get directories
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROUP_ROOT="$SCRIPT_DIR/../.."

echo "Build directory: $SCRIPT_DIR"
echo ""

# Step 1: Check prerequisites
echo "Step 1/5: Checking prerequisites..."

command -v git >/dev/null 2>&1 || error "git not found"
command -v cmake >/dev/null 2>&1 || error "cmake not found (need 3.10+)"
command -v cargo >/dev/null 2>&1 || error "cargo not found (install Rust)"

# Check C++ compiler
if command -v g++ >/dev/null 2>&1; then
    CXX_COMPILER="g++"
elif command -v clang++ >/dev/null 2>&1; then
    CXX_COMPILER="clang++"
    warn "g++ not found, using clang++"
else
    error "No C++ compiler found"
fi

status "All prerequisites found (using $CXX_COMPILER)"
echo ""

# Step 2: Initialize submodule
echo "Step 2/5: Initializing accparser submodule..."

if [ ! -f "$SCRIPT_DIR/accparser/src/OpenACCIR.h" ]; then
    cd "$SCRIPT_DIR"
    git submodule update --init --recursive || error "Failed to initialize submodule"
    status "Submodule initialized"
else
    status "Submodule already initialized"
fi
echo ""

# Step 3: Build ROUP core
echo "Step 3/5: Building ROUP core library..."

cd "$ROUP_ROOT"
if [ ! -f "target/release/libroup.a" ]; then
    cargo build --release || error "Failed to build ROUP"
    status "ROUP built successfully"
else
    warn "ROUP already built (skipping)"
fi
echo ""

# Step 4: Build compatibility layer
echo "Step 4/5: Building libaccparser.so..."

cd "$SCRIPT_DIR"
mkdir -p build
cd build
cmake .. || error "CMake failed"
make -j$(nproc) || error "Build failed"

status "libaccparser.so built successfully"
echo ""

# Step 5: Run tests
echo "Step 5/5: Running tests..."

if [ -f "./accparser_example" ]; then
    LD_LIBRARY_PATH=. ./accparser_example
    status "Tests passed"
else
    warn "Test binary not found"
fi
echo ""

# Summary
echo "======================================"
echo "  Build Complete! 🎉"
echo "======================================"
echo ""
echo "Built files:"
echo "  $SCRIPT_DIR/build/libaccparser.so"
echo "  $SCRIPT_DIR/build/libroup-accparser-compat.a"
echo ""
echo "Next steps:"
echo ""
echo "  1. Test it:"
echo "     cd $SCRIPT_DIR/build"
echo "     LD_LIBRARY_PATH=. ./accparser_example"
echo ""
echo "  2. Install system-wide:"
echo "     cd $SCRIPT_DIR/build"
echo "     sudo make install"
echo "     sudo ldconfig"
echo ""
echo "  3. Use in your project:"
echo "     g++ myapp.cpp -laccparser -o myapp"
echo ""
