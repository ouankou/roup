#!/bin/bash
#
# test_openmp_vv.sh - Round-trip OpenMP pragmas from OpenMP_VV through ROUP
#
# This script validates ROUP by:
# 1. Cloning the OpenMP Validation & Verification test suite (on-demand)
# 2. Preprocessing all C/C++ test files with clang
# 3. Extracting OpenMP pragmas
# 4. Round-tripping each pragma through ROUP's parser
# 5. Comparing normalized input vs output with clang-format
# 6. Reporting pass/fail statistics
#
# Usage:
#   ./test_openmp_vv.sh                        # Auto-clone to target/openmp_vv
#   OPENMP_VV_PATH=/path ./test_openmp_vv.sh   # Use existing clone
#   CLANG=clang-15 ./test_openmp_vv.sh         # Use specific clang version
#

set -euo pipefail

# Configuration
REPO_URL="https://github.com/OpenMP-Validation-and-Verification/OpenMP_VV"
REPO_PATH="${OPENMP_VV_PATH:-target/openmp_vv}"
TESTS_DIR="tests"
CLANG="${CLANG:-clang}"
CLANG_FORMAT="${CLANG_FORMAT:-clang-format}"
MAX_DISPLAY_FAILURES=10

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Statistics
total_files=0
files_with_pragmas=0
total_pragmas=0
passed=0
failed=0
parse_errors=0

# Arrays for failure details
declare -a failure_files=()
declare -a failure_pragmas=()
declare -a failure_reasons=()

echo "========================================="
echo "  OpenMP_VV Round-Trip Validation"
echo "========================================="
echo ""

# Check for required tools
echo "Checking for required tools..."
for tool in "$CLANG" "$CLANG_FORMAT" cargo; do
    if ! command -v "$tool" &>/dev/null; then
        echo -e "${RED}Error: $tool not found in PATH${NC}"
        exit 1
    fi
done
echo -e "${GREEN}✓${NC} All required tools found"
echo ""

# Ensure OpenMP_VV repository exists
if [ ! -d "$REPO_PATH" ]; then
    echo "OpenMP_VV not found at $REPO_PATH"
    echo "Cloning from $REPO_URL..."
    git clone --depth 1 "$REPO_URL" "$REPO_PATH" || {
        echo -e "${RED}Failed to clone OpenMP_VV${NC}"
        exit 1
    }
    echo -e "${GREEN}✓${NC} Cloned successfully"
    echo ""
elif [ ! -d "$REPO_PATH/$TESTS_DIR" ]; then
    echo -e "${RED}Error: $REPO_PATH exists but $TESTS_DIR/ not found${NC}"
    exit 1
else
    echo "Using existing OpenMP_VV at $REPO_PATH"
    echo ""
fi

# Build roup_roundtrip binary
echo "Building roup_roundtrip binary..."
cargo build --quiet --bin roup_roundtrip || {
    echo -e "${RED}Failed to build roup_roundtrip${NC}"
    exit 1
}
echo -e "${GREEN}✓${NC} Binary built"
echo ""

ROUNDTRIP_BIN="./target/debug/roup_roundtrip"

# Find all C/C++ source files
echo "Finding C/C++ test files in $REPO_PATH/$TESTS_DIR..."
mapfile -t source_files < <(find "$REPO_PATH/$TESTS_DIR" -type f \( -name "*.c" -o -name "*.cpp" -o -name "*.cc" -o -name "*.cxx" \) | sort)
total_files=${#source_files[@]}
echo "Found $total_files C/C++ files"
echo ""

echo "Processing files..."
echo ""

# Process each file
for file in "${source_files[@]}"; do
    # Preprocess with clang
    preprocessed=$("$CLANG" -E -P -CC -fopenmp -I"$(dirname "$file")" "$file" 2>/dev/null || true)

    if [ -z "$preprocessed" ]; then
        continue
    fi

    # Extract pragmas (lines starting with #pragma omp, with optional leading whitespace)
    mapfile -t pragmas < <(echo "$preprocessed" | grep -E '^[[:space:]]*#pragma[[:space:]]+omp' || true)

    if [ ${#pragmas[@]} -eq 0 ]; then
        continue
    fi

    files_with_pragmas=$((files_with_pragmas + 1))

    # Process each pragma
    for pragma in "${pragmas[@]}"; do
        total_pragmas=$((total_pragmas + 1))

        # Normalize original pragma with clang-format
        original_formatted=$(echo "$pragma" | "$CLANG_FORMAT" 2>/dev/null || echo "$pragma")

        # Round-trip through ROUP
        if ! roundtrip=$(echo "$pragma" | "$ROUNDTRIP_BIN" 2>/dev/null); then
            parse_errors=$((parse_errors + 1))
            failed=$((failed + 1))
            failure_files+=("$file")
            failure_pragmas+=("$pragma")
            failure_reasons+=("Parse error")
            continue
        fi

        # Normalize round-tripped pragma with clang-format
        roundtrip_formatted=$(echo "$roundtrip" | "$CLANG_FORMAT" 2>/dev/null || echo "$roundtrip")

        # Compare
        if [ "$original_formatted" = "$roundtrip_formatted" ]; then
            passed=$((passed + 1))
        else
            failed=$((failed + 1))
            failure_files+=("$file")
            failure_pragmas+=("$pragma")
            failure_reasons+=("Mismatch: got '$roundtrip'")
        fi
    done
done

echo ""
echo "========================================="
echo "  Results"
echo "========================================="
echo ""
echo "Files processed:        $total_files"
echo "Files with pragmas:     $files_with_pragmas"
echo "Total pragmas:          $total_pragmas"
echo ""

if [ $total_pragmas -eq 0 ]; then
    echo -e "${YELLOW}Warning: No pragmas found to test${NC}"
    exit 0
fi

pass_rate=$(awk "BEGIN {printf \"%.1f\", ($passed * 100.0) / $total_pragmas}")

echo -e "${GREEN}Passed:${NC}                $passed"
echo -e "${RED}Failed:${NC}                $failed"
echo "  Parse errors:         $parse_errors"
echo "  Mismatches:           $((failed - parse_errors))"
echo ""
echo "Success rate:           ${pass_rate}%"
echo ""

# Show failure details
if [ $failed -gt 0 ]; then
    echo "========================================="
    echo "  Failure Details (showing first $MAX_DISPLAY_FAILURES)"
    echo "========================================="
    echo ""

    display_count=0
    for i in "${!failure_files[@]}"; do
        if [ $display_count -ge $MAX_DISPLAY_FAILURES ]; then
            remaining=$((failed - MAX_DISPLAY_FAILURES))
            echo "... and $remaining more failures"
            break
        fi

        echo -e "${YELLOW}[$((i + 1))]${NC} ${failure_files[$i]}"
        echo "    Pragma:  ${failure_pragmas[$i]}"
        echo "    Reason:  ${failure_reasons[$i]}"
        echo ""

        display_count=$((display_count + 1))
    done
fi

# Exit with appropriate code
if [ $failed -eq 0 ]; then
    echo -e "${GREEN}✓ All pragmas round-tripped successfully!${NC}"
    exit 0
else
    echo -e "${RED}✗ Some pragmas failed to round-trip${NC}"
    exit 1
fi
