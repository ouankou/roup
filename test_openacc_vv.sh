#!/bin/bash
#
# test_openacc_vv.sh - Round-trip OpenACC directives from OpenACCV-V through ROUP
#
# This script validates ROUP by:
# 1. Cloning the OpenACCV-V test suite (on-demand)
# 2. Finding all C/C++/Fortran source files
# 3. Extracting OpenACC directives (no preprocessing needed)
# 4. Round-tripping each directive through ROUP's parser
# 5. Comparing normalized input vs output
# 6. Reporting pass/fail statistics
#
# Usage:
#   ./test_openacc_vv.sh                        # Auto-clone to target/openacc_vv
#   OPENACC_VV_PATH=/path ./test_openacc_vv.sh  # Use existing clone
#

set -euo pipefail

# Configuration
REPO_URL="https://github.com/OpenACCUserGroup/OpenACCV-V"
REPO_PATH="${OPENACC_VV_PATH:-target/openacc_vv}"
TESTS_DIR="Tests"
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

# Normalize a pragma by removing OpenACC's optional formatting
normalize_pragma() {
    local pragma="$1"
    # Remove commas (optional in OpenACC), remove space before '(' (optional),
    # collapse multiple spaces, trim, lowercase
    echo "$pragma" | sed 's/,//g' | sed 's/ (/(/g' | sed 's/[[:space:]]\+/ /g' | sed 's/^[[:space:]]*//;s/[[:space:]]*$//' | tr '[:upper:]' '[:lower:]'
}

echo "========================================="
echo "  OpenACCV-V Round-Trip Validation"
echo "========================================="
echo ""

# Check for required tools
echo "Checking for required tools..."
for tool in cargo; do
    if ! command -v "$tool" &>/dev/null; then
        echo -e "${RED}Error: $tool not found in PATH${NC}"
        exit 1
    fi
done
echo -e "${GREEN}✓${NC} All required tools found"
echo ""

# Ensure OpenACCV-V repository exists
if [ ! -d "$REPO_PATH" ]; then
    echo "OpenACCV-V not found at $REPO_PATH"
    echo "Cloning from $REPO_URL..."
    git clone --depth 1 "$REPO_URL" "$REPO_PATH" || {
        echo -e "${RED}Failed to clone OpenACCV-V${NC}"
        exit 1
    }
    echo -e "${GREEN}✓${NC} Cloned successfully"
    echo ""
elif [ ! -d "$REPO_PATH/$TESTS_DIR" ]; then
    echo -e "${RED}Error: $REPO_PATH exists but $TESTS_DIR/ not found${NC}"
    exit 1
else
    echo "Using existing OpenACCV-V at $REPO_PATH"
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

# Find all C/C++/Fortran source files
echo "Finding test files in $REPO_PATH/$TESTS_DIR..."
mapfile -t source_files < <(find "$REPO_PATH/$TESTS_DIR" -type f \( \
    -name "*.c" -o -name "*.cpp" -o -name "*.cc" -o -name "*.cxx" -o \
    -name "*.f" -o -name "*.for" -o -name "*.f90" -o -name "*.f95" -o -name "*.f03" -o \
    -name "*.F" -o -name "*.F90" -o -name "*.F95" -o -name "*.F03" \
    \) | sort)
total_files=${#source_files[@]}
echo "Found $total_files files"
echo ""

echo "Processing files..."
echo ""

# Process each file
for file in "${source_files[@]}"; do
    # Read file content
    if ! content=$(cat "$file" 2>/dev/null); then
        continue
    fi

    # Extract directives based on file type
    ext="${file##*.}"
    case "$ext" in
        c|cpp|cc|cxx)
            # C/C++: Extract #pragma acc directives
            mapfile -t pragmas < <(echo "$content" | grep -E '^[[:space:]]*#pragma[[:space:]]+acc' || true)
            ;;
        f|for|f90|f95|f03|F|F90|F95|F03)
            # Fortran: Extract !$acc, c$acc, *$acc directives
            mapfile -t pragmas < <(echo "$content" | grep -iE '^[[:space:]]*[!cC*]\$acc' || true)
            ;;
        *)
            continue
            ;;
    esac

    if [ ${#pragmas[@]} -eq 0 ]; then
        continue
    fi

    files_with_pragmas=$((files_with_pragmas + 1))

    # Process each pragma
    for pragma in "${pragmas[@]}"; do
        total_pragmas=$((total_pragmas + 1))

        # Normalize original pragma
        original_normalized=$(normalize_pragma "$pragma")

        # Round-trip through ROUP
        if ! roundtrip=$(echo "$pragma" | "$ROUNDTRIP_BIN" --acc 2>/dev/null); then
            parse_errors=$((parse_errors + 1))
            failed=$((failed + 1))
            failure_files+=("$file")
            failure_pragmas+=("$pragma")
            failure_reasons+=("Parse error")
            continue
        fi

        # Normalize round-tripped pragma
        roundtrip_normalized=$(normalize_pragma "$roundtrip")

        # Compare
        if [ "$original_normalized" = "$roundtrip_normalized" ]; then
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
