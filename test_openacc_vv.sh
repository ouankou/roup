#!/bin/bash
#
# test_openacc_vv.sh - Round-trip OpenACC pragmas from OpenACCV-V through ROUP
#
# This script validates ROUP by:
# 1. Cloning the OpenACCV-V validation suite (on-demand)
# 2. Preprocessing all C/C++ test files with clang
# 3. Extracting OpenACC directives from C/C++ and Fortran tests
# 4. Round-tripping each directive through ROUP's parser
# 5. Comparing normalized input vs output (clang-format for C/C++, awk for Fortran)
# 6. Reporting pass/fail statistics
#
# Usage:
#   ./test_openacc_vv.sh                          # Auto-clone to target/openacc_vv
#   OPENACC_VV_PATH=/path ./test_openacc_vv.sh   # Use existing clone
#   CLANG=clang-16 ./test_openacc_vv.sh          # Use specific clang version

set -euo pipefail

REPO_URL="https://github.com/OpenACCUserGroup/OpenACCV-V"
REPO_PATH="${OPENACC_VV_PATH:-target/openacc_vv}"
TESTS_DIR="Tests"
CLANG="${CLANG:-clang}"
CLANG_FORMAT="${CLANG_FORMAT:-clang-format}"
ROUNDTRIP_BIN="./target/debug/roup_roundtrip"
MAX_DISPLAY_FAILURES=10

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

total_files=0
files_with_directives=0
total_directives=0
passed=0
failed=0
parse_errors=0

declare -a failure_files=()
declare -a failure_directives=()
declare -a failure_reasons=()

echo "========================================="
echo "  OpenACCV-V Round-Trip Validation"
echo "========================================="
echo ""

echo "Checking for required tools..."
for tool in "$CLANG" "$CLANG_FORMAT" cargo git; do
    if ! command -v "$tool" &>/dev/null; then
        echo -e "${RED}Error: $tool not found in PATH${NC}"
        exit 1
    fi
done
echo -e "${GREEN}✓${NC} All required tools found"
echo ""

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

echo "Building roup_roundtrip binary..."
cargo build --quiet --bin roup_roundtrip || {
    echo -e "${RED}Failed to build roup_roundtrip${NC}"
    exit 1
}
echo -e "${GREEN}✓${NC} Binary built"
echo ""

echo "Finding test files in $REPO_PATH/$TESTS_DIR..."
mapfile -t source_files < <(find "$REPO_PATH/$TESTS_DIR" -type f \( -name "*.c" -o -name "*.cpp" -o -name "*.h" -o -name "*.F90" -o -name "*.f90" -o -name "*.F" -o -name "*.f" \) | sort)
total_files=${#source_files[@]}
echo "Found $total_files files"
echo ""

echo "Processing files..."
echo ""

for file in "${source_files[@]}"; do
    has_directive=false

    case "$file" in
        *.c|*.cpp|*.h)
            preprocessed=$("$CLANG" -E -P -CC -I"$(dirname "$file")" "$file" 2>/dev/null || true)
            if [ -z "$preprocessed" ]; then
                continue
            fi

            mapfile -t pragmas < <(echo "$preprocessed" | grep -E '^[[:space:]]*#pragma[[:space:]]+acc' || true)
            if [ ${#pragmas[@]} -eq 0 ]; then
                continue
            fi
            has_directive=true

            for pragma in "${pragmas[@]}"; do
                total_directives=$((total_directives + 1))

                if ! roundtrip_output=$(printf '%s
' "$pragma" | ROUP_DIALECT=openacc ROUP_LANGUAGE=c "$ROUNDTRIP_BIN" 2>&1); then
                    parse_errors=$((parse_errors + 1))
                    failed=$((failed + 1))
                    failure_files+=("$file")
                    failure_directives+=("$pragma")
                    failure_reasons+=("Parse error: $roundtrip_output")
                    continue
                fi

                roundtrip="$roundtrip_output"

                if ! original_formatted=$(printf '%s
' "$pragma" | "$CLANG_FORMAT" 2>/dev/null); then
                    original_formatted="$pragma"
                fi

                if ! roundtrip_formatted=$(printf '%s
' "$roundtrip" | "$CLANG_FORMAT" 2>/dev/null); then
                    roundtrip_formatted="$roundtrip"
                fi

                if [ "$original_formatted" = "$roundtrip_formatted" ]; then
                    passed=$((passed + 1))
                else
                    failed=$((failed + 1))
                    failure_files+=("$file")
                    failure_directives+=("$pragma")
                    clean_roundtrip=$(printf '%s' "$roundtrip" | tr '\n' ' ')
                    failure_reasons+=("Mismatch: got '$clean_roundtrip'")
                fi
            done
            ;;
        *.F90|*.f90|*.F|*.f)
            mapfile -t directives < <(grep -i '^[[:space:]]*!\$acc' "$file" || true)
            if [ ${#directives[@]} -eq 0 ]; then
                continue
            fi
            has_directive=true

            for directive in "${directives[@]}"; do
                total_directives=$((total_directives + 1))

                if ! roundtrip_output=$(printf '%s
' "$directive" | ROUP_DIALECT=openacc ROUP_LANGUAGE=fortran-free "$ROUNDTRIP_BIN" 2>&1); then
                    parse_errors=$((parse_errors + 1))
                    failed=$((failed + 1))
                    failure_files+=("$file")
                    failure_directives+=("$directive")
                    failure_reasons+=("Parse error: $roundtrip_output")
                    continue
                fi

                roundtrip="$roundtrip_output"
                original_normalized=$(printf '%s
' "$directive" | awk '{ $1=$1; print }')
                roundtrip_normalized=$(printf '%s
' "$roundtrip" | awk '{ $1=$1; print }')

                if [ "$original_normalized" = "$roundtrip_normalized" ]; then
                    passed=$((passed + 1))
                else
                    failed=$((failed + 1))
                    failure_files+=("$file")
                    failure_directives+=("$directive")
                    clean_roundtrip=$(printf '%s' "$roundtrip" | tr '\n' ' ')
                    failure_reasons+=("Mismatch: got '$clean_roundtrip'")
                fi
            done
            ;;
        *)
            ;;
    esac

    if [ "$has_directive" = true ]; then
        files_with_directives=$((files_with_directives + 1))
    fi
done

echo ""
echo "========================================="
echo "  Results"
echo "========================================="
echo ""
echo "Files processed:        $total_files"
echo "Files with directives:  $files_with_directives"
echo "Total directives:       $total_directives"
echo ""

if [ $total_directives -eq 0 ]; then
    echo -e "${YELLOW}Warning: No directives found to test${NC}"
    exit 0
fi

pass_rate=$(awk "BEGIN {printf \"%.1f\", ($passed * 100.0) / $total_directives}")

echo -e "${GREEN}Passed:${NC}                $passed"
echo -e "${RED}Failed:${NC}                $failed"
echo "  Parse errors:         $parse_errors"
echo "  Mismatches:           $((failed - parse_errors))"
echo ""
echo "Success rate:           ${pass_rate}%"
echo ""

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
        echo "    Directive: ${failure_directives[$i]}"
        echo "    Reason:    ${failure_reasons[$i]}"
        echo ""

        display_count=$((display_count + 1))
    done
fi

if [ $failed -eq 0 ]; then
    echo -e "${GREEN}✓ All directives round-tripped successfully!${NC}"
    exit 0
else
    echo -e "${RED}✗ Some directives failed to round-trip${NC}"
    exit 1
fi
