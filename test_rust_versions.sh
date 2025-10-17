#!/usr/bin/env bash
# Test against multiple Rust versions to catch version-specific issues
#
# ROUP follows MSRV + stable approach (Rust ecosystem best practice):
#   - MSRV (Minimum Supported Rust Version): 1.85.0
#   - Stable: Latest stable release
#
# This script automatically reads the Rust version list from the CI config file
# (.github/workflows/ci.yml), ensuring local testing always matches CI exactly.
#
# Usage:
#   ./test_rust_versions.sh              # Auto-read versions from CI config (MSRV + stable)
#   ./test_rust_versions.sh 1.85 stable  # Test specific versions
#   ./test_rust_versions.sh 1.85 1.86 stable  # Test intermediate versions too
#
# How it works:
#   - Parses .github/workflows/ci.yml to extract the version matrix
#   - Tests each version by installing it via rustup
#   - Runs critical checks: format, clippy, build, tests
#   - Restores your original Rust version when done
#
# Single source of truth: .github/workflows/ci.yml
# When you update the CI matrix, this script automatically uses the same versions.

set -e

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Read Rust versions from CI config (single source of truth)
# This ensures script and CI stay in sync automatically by parsing the actual CI matrix
get_rust_versions_from_ci() {
    local ci_file=".github/workflows/ci.yml"

    if [ ! -f "$ci_file" ]; then
        echo "WARNING: CI config not found, using fallback MSRV + stable" >&2
        echo "1.85 stable"
        return
    fi

    # Extract version array from CI YAML
    # Looks for: version: ["1.85", "stable"]
    local versions=$(grep -A 1 "version:" "$ci_file" | \
                    grep -oP '\[.*\]' | \
                    tr -d '[]"' | \
                    tr ',' ' ')

    if [ -z "$versions" ]; then
        echo "WARNING: Could not parse versions from CI config, using fallback MSRV + stable" >&2
        echo "1.85 stable"
        return
    fi

    echo "$versions"
}

# Use provided versions or read from CI config
if [ $# -gt 0 ]; then
    VERSIONS=("$@")
else
    VERSIONS=($(get_rust_versions_from_ci))
fi

echo "========================================"
echo "  Rust Version Compatibility Test"
echo "========================================"
echo ""
echo "Testing against Rust versions: ${VERSIONS[*]}"
echo ""

# Check if rustup is installed
if ! command -v rustup &> /dev/null; then
    echo -e "${RED}✗ rustup not found${NC}"
    echo "  This script requires rustup to manage Rust versions"
    echo "  Install: https://rustup.rs/"
    exit 1
fi

# Save current toolchain
ORIGINAL_TOOLCHAIN=$(rustup show active-toolchain | cut -d' ' -f1)
echo "Current toolchain: $ORIGINAL_TOOLCHAIN"
echo ""

PASSED=0
FAILED=0
FAILED_VERSIONS=()

# Test each version
for VERSION in "${VERSIONS[@]}"; do
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}Testing Rust $VERSION${NC}"
    echo -e "${BLUE}========================================${NC}"

    # Install version if needed
    echo -n "Installing/updating Rust $VERSION... "
    if rustup toolchain install "$VERSION" &> /tmp/rustup_install.log; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${RED}✗ FAILED${NC}"
        cat /tmp/rustup_install.log
        ((FAILED++))
        FAILED_VERSIONS+=("$VERSION (install failed)")
        continue
    fi

    # Switch to this version
    rustup override set "$VERSION" > /dev/null

    # Show version info
    echo "  rustc: $(rustc --version)"
    echo "  clippy: $(cargo clippy --version 2>/dev/null || echo 'not available')"
    echo ""

    # Run critical tests
    echo "Running critical checks:"

    # 1. Format check
    echo -n "  1. Format check... "
    if cargo fmt --check &> /tmp/fmt_$VERSION.log; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${RED}✗ FAILED${NC}"
        cat /tmp/fmt_$VERSION.log
        ((FAILED++))
        FAILED_VERSIONS+=("$VERSION (format)")
        rustup override unset > /dev/null
        continue
    fi

    # 2. Clippy
    echo -n "  2. Clippy lints... "
    if cargo clippy --all-targets -- -D warnings &> /tmp/clippy_$VERSION.log; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${RED}✗ FAILED${NC}"
        echo ""
        echo "Clippy errors for Rust $VERSION:"
        cat /tmp/clippy_$VERSION.log | grep -A 3 "error:" | head -50
        echo ""
        ((FAILED++))
        FAILED_VERSIONS+=("$VERSION (clippy)")
        rustup override unset > /dev/null
        continue
    fi

    # 3. Build
    echo -n "  3. Build... "
    if cargo build --all-targets &> /tmp/build_$VERSION.log; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${RED}✗ FAILED${NC}"
        cat /tmp/build_$VERSION.log
        ((FAILED++))
        FAILED_VERSIONS+=("$VERSION (build)")
        rustup override unset > /dev/null
        continue
    fi

    # 4. Tests
    echo -n "  4. Tests... "
    if cargo test --all-targets &> /tmp/test_$VERSION.log; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${RED}✗ FAILED${NC}"
        cat /tmp/test_$VERSION.log | tail -50
        ((FAILED++))
        FAILED_VERSIONS+=("$VERSION (tests)")
        rustup override unset > /dev/null
        continue
    fi

    echo -e "${GREEN}✓ Rust $VERSION: ALL CHECKS PASSED${NC}"
    ((PASSED++))

    # Reset to original
    rustup override unset > /dev/null
    echo ""
done

# Restore original toolchain
echo "Restoring original toolchain: $ORIGINAL_TOOLCHAIN"
rustup override set "$ORIGINAL_TOOLCHAIN" > /dev/null 2>&1 || rustup default "$ORIGINAL_TOOLCHAIN"
rustup override unset > /dev/null 2>&1

# Summary
echo ""
echo "========================================"
echo "  Summary"
echo "========================================"
echo "Tested versions: ${#VERSIONS[@]}"
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo ""

if [ $FAILED -gt 0 ]; then
    echo -e "${RED}Failed versions:${NC}"
    for ver in "${FAILED_VERSIONS[@]}"; do
        echo "  - $ver"
    done
    echo ""
    echo -e "${YELLOW}Note:${NC} Logs available in /tmp/*_<version>.log"
    exit 1
else
    echo -e "${GREEN}✓ All tested versions PASSED!${NC}"
    echo ""
    echo "Your code is compatible with Rust ${VERSIONS[*]}"
    exit 0
fi
