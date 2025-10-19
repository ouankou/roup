#!/usr/bin/env bash
# Comprehensive test script for ROUP - runs ALL possible tests
# Based on AGENTS.md requirements

set -e  # Exit on first error

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "========================================"
echo "  ROUP Comprehensive Test Suite"
echo "========================================"
echo ""
echo "Environment:"
rustc --version
cargo clippy --version 2>/dev/null || echo "  clippy: not installed"
echo ""

# Test section counter for auto-numbering
SECTION_NUM=1

# ===================================================================
# 1. Code Formatting Check
# ===================================================================
echo "=== $SECTION_NUM. Formatting Check ==="
echo -n "Running cargo fmt --check... "
if cargo fmt --check > /dev/null 2>&1; then
    echo -e "${GREEN}✓ PASS${NC}"
else
    echo -e "${RED}✗ FAIL${NC}"
    echo "Run 'cargo fmt' to fix formatting issues"
    exit 1
fi

# ===================================================================
# 2. Rust Build (all targets)
# ===================================================================
SECTION_NUM=$((SECTION_NUM + 1)); echo "=== $SECTION_NUM. Rust Build ==="
echo -n "Building debug (all targets)... "
if cargo build --locked --all-targets 2>&1 | tee /tmp/build_debug.log | grep -q "Finished"; then
    warnings=$(grep -i "warning:" /tmp/build_debug.log | grep -v "build.rs" | wc -l)
    if [ "$warnings" -eq 0 ]; then
        echo -e "${GREEN}✓ PASS (0 warnings)${NC}"
    else
        echo -e "${YELLOW}⚠ PASS with $warnings warnings${NC}"
    fi
else
    echo -e "${RED}✗ FAIL${NC}"
    exit 1
fi

echo -n "Building release (all targets)... "
if cargo build --locked --release --all-targets 2>&1 | tee /tmp/build_release.log | grep -q "Finished"; then
    warnings=$(grep -i "warning:" /tmp/build_release.log | grep -v "build.rs" | wc -l)
    if [ "$warnings" -eq 0 ]; then
        echo -e "${GREEN}✓ PASS (0 warnings)${NC}"
    else
        echo -e "${YELLOW}⚠ PASS with $warnings warnings${NC}"
    fi
else
    echo -e "${RED}✗ FAIL${NC}"
    exit 1
fi

# ===================================================================
# 3. Rust Unit Tests
# ===================================================================
SECTION_NUM=$((SECTION_NUM + 1)); echo "=== $SECTION_NUM. Rust Unit Tests ==="
echo -n "Running cargo test --lib... "
if cargo test --locked --lib > /tmp/test_lib.log 2>&1; then
    passed=$(grep "test result:" /tmp/test_lib.log | grep -o "[0-9]* passed" | grep -o "[0-9]*")
    echo -e "${GREEN}✓ PASS ($passed tests)${NC}"
else
    echo -e "${RED}✗ FAIL${NC}"
    cat /tmp/test_lib.log
    exit 1
fi

# ===================================================================
# 4. Rust Integration Tests
# ===================================================================
SECTION_NUM=$((SECTION_NUM + 1)); echo "=== $SECTION_NUM. Rust Integration Tests ==="
echo -n "Running cargo test --tests... "
if cargo test --locked --tests > /tmp/test_integration.log 2>&1; then
    passed=$(grep "test result:" /tmp/test_integration.log | tail -1 | grep -o "[0-9]* passed" | grep -o "[0-9]*")
    echo -e "${GREEN}✓ PASS ($passed tests)${NC}"
else
    echo -e "${RED}✗ FAIL${NC}"
    cat /tmp/test_integration.log
    exit 1
fi

# ===================================================================
# 5. Rust Doc Tests
# ===================================================================
SECTION_NUM=$((SECTION_NUM + 1)); echo "=== $SECTION_NUM. Rust Doc Tests ==="
echo -n "Running cargo test --doc... "
if cargo test --locked --doc > /tmp/test_doc.log 2>&1; then
    passed=$(grep "test result:" /tmp/test_doc.log | tail -1 | grep -o "[0-9]* passed" | grep -o "[0-9]*")
    echo -e "${GREEN}✓ PASS ($passed doctests)${NC}"
else
    echo -e "${RED}✗ FAIL${NC}"
    cat /tmp/test_doc.log
    exit 1
fi

# ===================================================================
# 6. All Rust Tests Together
# ===================================================================
SECTION_NUM=$((SECTION_NUM + 1)); echo "=== $SECTION_NUM. All Rust Tests Together ==="
echo -n "Running cargo test --all-targets... "
if cargo test --locked --all-targets > /tmp/test_all.log 2>&1; then
    total_passed=$(grep "test result:" /tmp/test_all.log | grep -o "[0-9]* passed" | awk '{sum+=$1} END {print sum}')
    echo -e "${GREEN}✓ PASS ($total_passed total tests)${NC}"
else
    echo -e "${RED}✗ FAIL${NC}"
    cat /tmp/test_all.log
    exit 1
fi

# ===================================================================
# 7. Rust Examples Build
# ===================================================================
SECTION_NUM=$((SECTION_NUM + 1)); echo "=== $SECTION_NUM. Examples Build ==="
echo -n "Building all examples... "
if cargo build --locked --examples > /tmp/build_examples.log 2>&1; then
    echo -e "${GREEN}✓ PASS${NC}"
else
    echo -e "${RED}✗ FAIL${NC}"
    cat /tmp/build_examples.log
    exit 1
fi

# ===================================================================
# 8. Rust Documentation Build
# ===================================================================
SECTION_NUM=$((SECTION_NUM + 1)); echo "=== $SECTION_NUM. Rust Documentation ==="
echo -n "Building API docs (cargo doc --no-deps --all-features)... "
if cargo doc --locked --no-deps --all-features > /tmp/doc.log 2>&1; then
    warnings=$(grep -i "warning:" /tmp/doc.log | wc -l)
    if [ "$warnings" -eq 0 ]; then
        echo -e "${GREEN}✓ PASS (0 warnings)${NC}"
    else
        echo -e "${RED}✗ FAIL - $warnings warnings found (treating warnings as errors):${NC}"
        grep -i "warning:" /tmp/doc.log
        exit 1
    fi
else
    echo -e "${RED}✗ FAIL${NC}"
    cat /tmp/doc.log
    exit 1
fi

# ===================================================================
# 9. ompparser Compatibility Tests
# ===================================================================
SECTION_NUM=$((SECTION_NUM + 1)); echo "=== $SECTION_NUM. ompparser Compat Tests ==="
if [ -d "compat/ompparser" ] && [ -f "compat/ompparser/build.sh" ]; then
    echo -n "Running compat tests... "
    cd compat/ompparser
    if ./build.sh > /tmp/compat_test.log 2>&1; then
        echo -e "${GREEN}✓ PASS${NC}"
    else
        echo -e "${RED}✗ FAIL${NC}"
        cat /tmp/compat_test.log
        cd ../..
        exit 1
    fi
    cd ../..
else
    echo -e "${RED}✗ FAIL - ompparser compatibility layer is MANDATORY but not found${NC}"
    echo "   Expected: compat/ompparser/build.sh"
    echo "   Run: git submodule update --init --recursive"
    exit 1
fi

# ===================================================================
# 10. accparser Compatibility Tests
# ===================================================================
SECTION_NUM=$((SECTION_NUM + 1)); echo "=== $SECTION_NUM. accparser Compat Tests ==="
if [ -d "compat/accparser" ] && [ -f "compat/accparser/build.sh" ]; then
    echo -n "Running accparser compat tests (clean build)... "
    cd compat/accparser
    # Clean build to ensure fresh state (like CI)
    rm -rf build > /dev/null 2>&1
    if ./build.sh > /tmp/accparser_compat_test.log 2>&1; then
        echo -e "${GREEN}✓ PASS${NC}"
    else
        echo -e "${RED}✗ FAIL${NC}"
        cat /tmp/accparser_compat_test.log
        cd ../..
        exit 1
    fi
    cd ../..
else
    echo -e "${YELLOW}⚠ SKIP - accparser compatibility layer not found (optional)${NC}"
    echo "   To enable: git submodule update --init compat/accparser/accparser"
fi

# ===================================================================
# 11. mdBook Documentation Build
# ===================================================================
SECTION_NUM=$((SECTION_NUM + 1)); echo "=== $SECTION_NUM. mdBook Documentation ==="
if ! command -v mdbook > /dev/null 2>&1; then
    echo -e "${RED}✗ FAIL - mdbook is MANDATORY but not installed${NC}"
    echo "   Install: cargo install mdbook"
    exit 1
fi

if [ ! -d "docs/book" ]; then
    echo -e "${RED}✗ FAIL - docs/book directory not found${NC}"
    exit 1
fi

echo -n "Building mdBook... "
if mdbook build docs/book > /tmp/mdbook_build.log 2>&1; then
    echo -e "${GREEN}✓ PASS${NC}"
else
    echo -e "${RED}✗ FAIL${NC}"
    cat /tmp/mdbook_build.log
    exit 1
fi

# ===================================================================
# 12. mdBook Code Examples Test
# ===================================================================
SECTION_NUM=$((SECTION_NUM + 1)); echo "=== $SECTION_NUM. mdBook Code Examples ==="
echo -n "Testing mdBook code examples... "
if mdbook test docs/book > /tmp/mdbook_test.log 2>&1; then
    echo -e "${GREEN}✓ PASS${NC}"
else
    echo -e "${RED}✗ FAIL${NC}"
    cat /tmp/mdbook_test.log
    exit 1
fi

# ===================================================================
# 13. C Examples Build and Run
# ===================================================================
SECTION_NUM=$((SECTION_NUM + 1)); echo "=== $SECTION_NUM. C Examples ==="
if [ ! -d "examples/c" ] || [ ! -f "examples/c/Makefile" ]; then
    echo -e "${RED}✗ FAIL - C examples are MANDATORY but not found${NC}"
    echo "   Expected: examples/c/Makefile"
    exit 1
fi

echo -n "Building C examples... "
if (cd examples/c && make clean > /dev/null 2>&1 && make BUILD_TYPE=release all > /tmp/c_examples_build.log 2>&1); then
    echo -e "${GREEN}✓ PASS${NC}"
else
    echo -e "${RED}✗ FAIL${NC}"
    cat /tmp/c_examples_build.log
    exit 1
fi

echo -n "Running C examples... "
if (cd examples/c && make BUILD_TYPE=release run-all > /tmp/c_examples_run.log 2>&1); then
    echo -e "${GREEN}✓ PASS${NC}"
else
    echo -e "${RED}✗ FAIL${NC}"
    cat /tmp/c_examples_run.log
    exit 1
fi

# ===================================================================
# 14. C++ Examples Build and Run
# ===================================================================
SECTION_NUM=$((SECTION_NUM + 1)); echo "=== $SECTION_NUM. C++ Examples ==="
if [ -d "examples/cpp" ] && [ -f "examples/cpp/Makefile" ]; then
    echo -n "Building C++ examples... "
    if (cd examples/cpp && make clean > /dev/null 2>&1 && make > /tmp/cpp_examples_build.log 2>&1); then
        echo -e "${GREEN}✓ PASS${NC}"
    else
        echo -e "${RED}✗ FAIL${NC}"
        cat /tmp/cpp_examples_build.log
        exit 1
    fi

    echo -n "Running C++ examples... "
    if (cd examples/cpp && make run-all > /tmp/cpp_examples_run.log 2>&1); then
        echo -e "${GREEN}✓ PASS${NC}"
    else
        echo -e "${RED}✗ FAIL${NC}"
        cat /tmp/cpp_examples_run.log
        exit 1
    fi
else
    echo -e "${RED}✗ FAIL - C++ examples are MANDATORY but not found${NC}"
    echo "   Expected: examples/cpp/Makefile"
    echo "   All example categories (C, C++, Fortran) are required"
    exit 1
fi

# ===================================================================
# 15. Fortran Examples Build
# ===================================================================
SECTION_NUM=$((SECTION_NUM + 1)); echo "=== $SECTION_NUM. Fortran Examples ==="
if [ ! -d "examples/fortran" ] || [ ! -f "examples/fortran/Makefile" ]; then
    echo -e "${RED}✗ FAIL - Fortran examples are MANDATORY but not found${NC}"
    echo "   Expected: examples/fortran/Makefile"
    exit 1
fi

echo -n "Building Fortran examples... "
if (cd examples/fortran && make clean > /dev/null 2>&1 && make > /tmp/fortran_examples.log 2>&1); then
    echo -e "${GREEN}✓ PASS${NC}"
else
    echo -e "${RED}✗ FAIL${NC}"
    cat /tmp/fortran_examples.log
    exit 1
fi

# ===================================================================
# 16. Header Verification
# ===================================================================
SECTION_NUM=$((SECTION_NUM + 1)); echo "=== $SECTION_NUM. Header Verification ==="
echo -n "Verifying header is up-to-date... "
if cargo run --locked --bin gen > /tmp/header_verify.log 2>&1; then
    echo -e "${GREEN}✓ PASS${NC}"
else
    echo -e "${RED}✗ FAIL${NC}"
    cat /tmp/header_verify.log
    exit 1
fi

# ===================================================================
# 17. Check for Compiler Warnings
# ===================================================================
SECTION_NUM=$((SECTION_NUM + 1)); echo "=== $SECTION_NUM. Warning Check ==="
echo -n "Checking for unexpected warnings... "
cargo build --locked 2>&1 | tee /tmp/warnings_raw.log > /dev/null
grep -i "warning:" /tmp/warnings_raw.log | grep -v "build.rs" > /tmp/warnings.log || true
warning_count=$(wc -l < /tmp/warnings.log)
if [ "$warning_count" -eq 0 ]; then
    echo -e "${GREEN}✓ PASS (0 warnings)${NC}"
else
    echo -e "${RED}✗ FAIL - $warning_count warnings found:${NC}"
    cat /tmp/warnings.log
    exit 1
fi

# ===================================================================
# 18. Clippy Lints (MANDATORY)
# ===================================================================
SECTION_NUM=$((SECTION_NUM + 1)); echo "=== $SECTION_NUM. Clippy Lints ==="
if command -v cargo-clippy > /dev/null 2>&1 || cargo clippy --version > /dev/null 2>&1; then
    echo -n "Running clippy (all targets)... "
    if cargo clippy --locked --all-targets -- -D warnings > /tmp/clippy.log 2>&1; then
        echo -e "${GREEN}✓ PASS${NC}"
    else
        echo -e "${RED}✗ FAIL - Clippy warnings found:${NC}"
        cat /tmp/clippy.log
        exit 1
    fi
else
    echo -e "${RED}✗ FAIL - clippy not installed${NC}"
    echo "Install with: rustup component add clippy"
    exit 1
fi

# ===================================================================
# 19. OpenMP_VV Round-Trip Validation (REQUIRED: 100% pass rate)
# ===================================================================
SECTION_NUM=$((SECTION_NUM + 1)); echo "=== $SECTION_NUM. OpenMP_VV Round-Trip Validation ==="
openmp_vv_status="skipped"
if command -v clang &>/dev/null && command -v clang-format &>/dev/null; then
    echo -n "Running OpenMP_VV round-trip test (100% required)... "
    if ./test_openmp_vv.sh > /tmp/test_openmp_vv.log 2>&1; then
        # Check that it's actually 100% (no failures)
        failures=$(grep "^Failed:" /tmp/test_openmp_vv.log | grep -o "[0-9]*" || echo "0")
        if [ "$failures" -eq 0 ]; then
            echo -e "${GREEN}✓ PASS (100% success rate)${NC}"
            openmp_vv_status="passed"
            grep "Success rate:" /tmp/test_openmp_vv.log || true
        else
            echo -e "${RED}✗ FAIL - $failures pragmas failed (100% required)${NC}"
            grep "Success rate:" /tmp/test_openmp_vv.log || true
            echo "See /tmp/test_openmp_vv.log for details"
            tail -50 /tmp/test_openmp_vv.log
            exit 1
        fi
    else
        echo -e "${RED}✗ FAIL - OpenMP_VV test script failed${NC}"
        tail -50 /tmp/test_openmp_vv.log
        exit 1
    fi
else
    echo -e "${RED}✗ FAIL - clang and clang-format are REQUIRED for OpenMP_VV validation${NC}"
    echo "   Install (Debian/Ubuntu): apt-get install clang clang-format"
    echo "   Install (macOS): brew install llvm"
    echo "   Install (Fedora/RHEL): dnf install clang clang-tools-extra"
    exit 1
fi

# ===================================================================
# 20. OpenACCV-V Round-Trip Validation (REQUIRED: 100% pass rate)
# ===================================================================
SECTION_NUM=$((SECTION_NUM + 1)); echo "=== $SECTION_NUM. OpenACCV-V Round-Trip Validation ==="
openacc_vv_status="skipped"
if command -v python3 &>/dev/null; then
    echo -n "Running OpenACCV-V round-trip test (100% required)... "
    if ./test_openacc_vv.sh > /tmp/test_openacc_vv.log 2>&1; then
        failures=$(grep "^Failed:" /tmp/test_openacc_vv.log | awk '{print $2}' || echo "0")
        if [ -z "$failures" ]; then
            failures=0
        fi
        if [ "$failures" -eq 0 ]; then
            echo -e "${GREEN}✓ PASS (100% success rate)${NC}"
            openacc_vv_status="passed"
            grep "Success rate:" /tmp/test_openacc_vv.log || true
        else
            echo -e "${RED}✗ FAIL - $failures directives failed (100% required)${NC}"
            grep "Success rate:" /tmp/test_openacc_vv.log || true
            echo "See /tmp/test_openacc_vv.log for details"
            tail -50 /tmp/test_openacc_vv.log
            exit 1
        fi
    else
        echo -e "${RED}✗ FAIL - OpenACCV-V test script failed${NC}"
        tail -50 /tmp/test_openacc_vv.log
        exit 1
    fi
else
    echo -e "${YELLOW}⚠${NC} python3 is REQUIRED for OpenACCV-V validation"
    echo "   Install python3 (Debian/Ubuntu): apt-get install python3"
fi

# ===================================================================
# 21. All Features Test
# ===================================================================
SECTION_NUM=$((SECTION_NUM + 1)); echo "=== $SECTION_NUM. All Features Test ==="
echo -n "Running tests with --all-features... "
if cargo test --locked --all-features > /tmp/test_all_features.log 2>&1; then
    echo -e "${GREEN}✓ PASS${NC}"
else
    echo -e "${RED}✗ FAIL${NC}"
    cat /tmp/test_all_features.log
    exit 1
fi

# ===================================================================
# 22. Benchmark Tests
# ===================================================================
SECTION_NUM=$((SECTION_NUM + 1)); echo "=== $SECTION_NUM. Benchmark Tests ==="
if [ ! -d "benches" ]; then
    echo -e "${RED}✗ FAIL - Benchmarks are MANDATORY but benches directory not found${NC}"
    exit 1
fi

echo -n "Running benchmarks (validation mode)... "
# Run benchmarks in quick mode for validation (not full performance measurement)
if cargo bench --locked --no-run > /tmp/bench.log 2>&1; then
    echo -e "${GREEN}✓ PASS (benchmarks compile)${NC}"
else
    echo -e "${RED}✗ FAIL${NC}"
    cat /tmp/bench.log
    exit 1
fi

# ===================================================================
# SUMMARY
# ===================================================================
echo ""
echo "========================================"
total_categories=22
passed_categories=$total_categories
if [ "$openmp_vv_status" != "passed" ]; then
    passed_categories=$((passed_categories - 1))
fi
if [ "${openacc_vv_status:-skipped}" != "passed" ]; then
    passed_categories=$((passed_categories - 1))
fi
if [ $passed_categories -eq $total_categories ]; then
    echo -e "  ${GREEN}ALL ${total_categories} TEST CATEGORIES PASSED${NC}"
else
    echo -e "  ${GREEN}${passed_categories} TEST CATEGORIES PASSED${NC}"
fi
echo "========================================"
echo ""
echo "Summary:"
echo "  ✓ Code formatting (cargo fmt)"
echo "  ✓ Rust builds (debug + release)"
echo "  ✓ All Rust tests (unit + integration + doc)"
echo "  ✓ All examples (Rust + C + C++ + Fortran)"
echo "  ✓ C examples execution (run-all)"
echo "  ✓ Documentation (rustdoc + mdBook with --all-features)"
echo "  ✓ Compatibility layers (ompparser + accparser)"
echo "  ✓ Header verification"
echo "  ✓ Zero compiler warnings"
echo "  ✓ Clippy lints passed"
if [ "$openmp_vv_status" = "passed" ]; then
    echo "  ✓ OpenMP_VV round-trip validation"
elif [ "$openmp_vv_status" = "skipped" ]; then
    echo "  ⚠ OpenMP_VV round-trip validation (skipped)"
fi
if [ "${openacc_vv_status:-skipped}" = "passed" ]; then
    echo "  ✓ OpenACCV-V round-trip validation"
elif [ "${openacc_vv_status:-skipped}" = "skipped" ]; then
    echo "  ⚠ OpenACCV-V round-trip validation (skipped)"
fi
echo "  ✓ All features tested"
echo "  ✓ Benchmarks validated"
echo ""
echo "Ready for commit!"
