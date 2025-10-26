#!/bin/bash
#
# test_openacc_vv.sh - Round-trip OpenACC directives from OpenACCV-V through ROUP
#
# This script validates ROUP by:
# 1. Cloning the OpenACCV-V test suite (on-demand)
# 2. Finding all C/C++/Fortran source files
# 3. Preprocessing with appropriate compilers (clang for C/C++, flang/gfortran for Fortran)
# 4. Extracting OpenACC directives from preprocessed output
# 5. Round-tripping each directive through ROUP's parser
# 6. Comparing normalized input vs output
# 7. Reporting pass/fail statistics
#
# Usage:
#   ./test_openacc_vv.sh                        # Auto-clone to target/openacc_vv
#   OPENACC_VV_PATH=/path ./test_openacc_vv.sh  # Use existing clone
#   CLANG=clang-15 ./test_openacc_vv.sh         # Use specific C/C++ compiler
#   FC=gfortran ./test_openacc_vv.sh            # Use specific Fortran compiler
#   PARALLEL_JOBS=8 ./test_openacc_vv.sh        # Control parallel execution
#

set -euo pipefail

# Configuration
REPO_URL="https://github.com/OpenACCUserGroup/OpenACCV-V"
REPO_PATH="${OPENACC_VV_PATH:-target/openacc_vv}"
TESTS_DIR="Tests"
CLANG="${CLANG:-}"  # Auto-detect if not specified
CLANG_FORMAT="${CLANG_FORMAT:-clang-format}"
FC="${FC:-}"  # Auto-detect if not specified
MAX_DISPLAY_FAILURES=10
# Fallback for systems without nproc (e.g., macOS)
if command -v nproc >/dev/null 2>&1; then
    DEFAULT_JOBS=$(nproc)
elif command -v getconf >/dev/null 2>&1; then
    DEFAULT_JOBS=$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo "1")
else
    DEFAULT_JOBS=1
fi
PARALLEL_JOBS="${PARALLEL_JOBS:-$DEFAULT_JOBS}"

# Detect C/C++ compiler if not specified (prefer clang, then gcc)
detect_c_compiler() {
    if [ -n "$CLANG" ]; then
        if command -v "$CLANG" &>/dev/null; then
            echo "$CLANG"
            return 0
        else
            echo ""
            return 1
        fi
    fi

    # Try clang first
    if command -v clang &>/dev/null; then
        echo "clang"
        return 0
    fi

    # Fall back to gcc
    if command -v gcc &>/dev/null; then
        echo "gcc"
        return 0
    fi

    echo ""
    return 1
}

# Detect Fortran compiler if not specified (prefer flang, then gfortran)
detect_fortran_compiler() {
    if [ -n "$FC" ]; then
        if command -v "$FC" &>/dev/null; then
            echo "$FC"
            return 0
        else
            echo ""
            return 1
        fi
    fi

    # Try flang first (LLVM Fortran)
    if command -v flang &>/dev/null; then
        echo "flang"
        return 0
    fi

    # Fall back to gfortran
    if command -v gfortran &>/dev/null; then
        echo "gfortran"
        return 0
    fi

    echo ""
    return 1
}

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
echo "  OpenACCV-V Round-Trip Validation"
echo "========================================="
echo ""

# Check for required tools
echo "Checking for required tools..."
if ! command -v cargo &>/dev/null; then
    echo -e "${RED}Error: cargo not found in PATH${NC}"
    exit 1
fi

# Detect compilers
C_COMPILER=$(detect_c_compiler)
FORTRAN_COMPILER=$(detect_fortran_compiler || echo "")

if [ -z "$C_COMPILER" ]; then
    echo -e "${RED}Error: No C/C++ compiler found (tried clang, gcc)${NC}"
    exit 1
fi

if ! command -v "$CLANG_FORMAT" &>/dev/null; then
    echo -e "${YELLOW}Warning: $CLANG_FORMAT not found, using basic normalization${NC}"
    CLANG_FORMAT=""
fi

# Report what we found
echo -e "${GREEN}✓${NC} Required tools found:"
echo "  C/C++ compiler:     $C_COMPILER"
if [ -n "$FORTRAN_COMPILER" ]; then
    echo "  Fortran compiler:   $FORTRAN_COMPILER"
else
    echo -e "  Fortran compiler:   ${YELLOW}none (Fortran files will be skipped)${NC}"
fi
if [ -n "$CLANG_FORMAT" ]; then
    echo "  Code formatter:     $CLANG_FORMAT"
fi
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

# Function to process a single file
process_file() {
    local file="$1"
    local temp_dir="$2"
    # Use hash of full file path to avoid collisions (e.g., file-name.c vs file_name.c)
    local file_hash
    if command -v sha1sum >/dev/null 2>&1; then
        file_hash=$(echo -n "$file" | sha1sum | awk '{print $1}')
    elif command -v shasum >/dev/null 2>&1; then
        file_hash=$(echo -n "$file" | shasum | awk '{print $1}')
    elif command -v md5sum >/dev/null 2>&1; then
        file_hash=$(echo -n "$file" | md5sum | awk '{print $1}')
    else
        # Fallback: use full path with character substitution (less safe but functional)
        file_hash=$(echo "$file" | sed 's/[^a-zA-Z0-9]/_/g')
    fi
    local file_id="$file_hash"
    local result_file="$temp_dir/result_$file_id"

    # Detect file type
    local ext="${file##*.}"
    local is_fortran=0
    case "$ext" in
        f|for|f90|f95|f03|F|F90|F95|F03)
            is_fortran=1
            ;;
    esac

    local preprocessed=""
    local pragmas=()

    if [ $is_fortran -eq 1 ]; then
        # Fortran file - use Fortran compiler
        if [ -z "$FORTRAN_COMPILER" ]; then
            # No Fortran compiler available, skip this file
            echo "0 0 0 0" > "$result_file"
            return
        fi

        # Preprocess with Fortran compiler
        preprocessed=$("$FORTRAN_COMPILER" -E -P -I"$(dirname "$file")" "$file" 2>/dev/null || true)

        if [ -z "$preprocessed" ]; then
            echo "0 0 0 0" > "$result_file"
            return
        fi

        # Extract Fortran directives (!$acc, c$acc, *$acc - case insensitive)
        # Normalize to lowercase for consistent processing
        mapfile -t pragmas < <(echo "$preprocessed" | grep -iE '^[[:space:]]*[!cC*]\$acc' | tr '[:upper:]' '[:lower:]' || true)
    else
        # C/C++ file - use C compiler
        preprocessed=$("$C_COMPILER" -E -P -CC -I"$(dirname "$file")" "$file" 2>/dev/null || true)

        if [ -z "$preprocessed" ]; then
            echo "0 0 0 0" > "$result_file"
            return
        fi

        # Extract pragmas (lines starting with #pragma acc, with optional leading whitespace)
        mapfile -t pragmas < <(echo "$preprocessed" | grep -E '^[[:space:]]*#pragma[[:space:]]+acc' || true)
    fi

    if [ ${#pragmas[@]} -eq 0 ]; then
        echo "0 0 0 0" > "$result_file"
        return
    fi

    local file_pragmas=${#pragmas[@]}
    local file_passed=0
    local file_failed=0
    local file_parse_errors=0

    # Process each pragma
    for pragma in "${pragmas[@]}"; do
        if [ $is_fortran -eq 1 ]; then
            # Fortran: normalize by converting to lowercase and removing extra spaces
            local original_normalized=$(echo "$pragma" | tr '[:upper:]' '[:lower:]' | sed 's/[[:space:]][[:space:]]*/ /g' | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')

            # Round-trip through ROUP (auto-detects Fortran from sentinel)
            if ! roundtrip=$(echo "$pragma" | "$ROUNDTRIP_BIN" --acc 2>/dev/null); then
                file_parse_errors=$((file_parse_errors + 1))
                file_failed=$((file_failed + 1))
                echo "$file|$pragma|Parse error" >> "$temp_dir/failures_$file_id"
                continue
            fi

            # Normalize round-tripped output
            local roundtrip_normalized=$(echo "$roundtrip" | tr '[:upper:]' '[:lower:]' | sed 's/[[:space:]][[:space:]]*/ /g' | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')

            # Compare
            if [ "$original_normalized" = "$roundtrip_normalized" ]; then
                file_passed=$((file_passed + 1))
            else
                file_failed=$((file_failed + 1))
                echo "$file|$pragma|Mismatch: got '$roundtrip'" >> "$temp_dir/failures_$file_id"
            fi
        else
            # C/C++: normalize with clang-format if available, otherwise basic normalization
            if [ -n "$CLANG_FORMAT" ]; then
                # Remove commas between clauses, then format
                local original_formatted=$(echo "$pragma" | sed 's/),[[:space:]][[:space:]]*/) /g' | "$CLANG_FORMAT" 2>/dev/null || echo "$pragma")
            else
                # Basic normalization: remove commas between clauses, collapse spaces, trim, lowercase
                local original_formatted=$(echo "$pragma" | sed 's/),[[:space:]][[:space:]]*/) /g' | sed 's/ (/(/g' | sed 's/[[:space:]][[:space:]]*/ /g' | sed 's/^[[:space:]]*//;s/[[:space:]]*$//' | tr '[:upper:]' '[:lower:]')
            fi

            # Round-trip through ROUP
            if ! roundtrip=$(echo "$pragma" | "$ROUNDTRIP_BIN" --acc 2>/dev/null); then
                file_parse_errors=$((file_parse_errors + 1))
                file_failed=$((file_failed + 1))
                echo "$file|$pragma|Parse error" >> "$temp_dir/failures_$file_id"
                continue
            fi

            # Normalize round-tripped pragma
            if [ -n "$CLANG_FORMAT" ]; then
                # Remove commas between clauses, then format
                local roundtrip_formatted=$(echo "$roundtrip" | sed 's/),[[:space:]][[:space:]]*/) /g' | "$CLANG_FORMAT" 2>/dev/null || echo "$roundtrip")
            else
                # Basic normalization: remove commas between clauses, collapse spaces, trim, lowercase
                local roundtrip_formatted=$(echo "$roundtrip" | sed 's/),[[:space:]][[:space:]]*/) /g' | sed 's/ (/(/g' | sed 's/[[:space:]][[:space:]]*/ /g' | sed 's/^[[:space:]]*//;s/[[:space:]]*$//' | tr '[:upper:]' '[:lower:]')
            fi

            # Compare
            if [ "$original_formatted" = "$roundtrip_formatted" ]; then
                file_passed=$((file_passed + 1))
            else
                file_failed=$((file_failed + 1))
                echo "$file|$pragma|Mismatch: got '$roundtrip'" >> "$temp_dir/failures_$file_id"
            fi
        fi
    done

    # Output: has_pragmas total_pragmas passed failed parse_errors
    echo "1 $file_pragmas $file_passed $file_failed $file_parse_errors" > "$result_file"
}

export -f process_file
export ROUNDTRIP_BIN C_COMPILER FORTRAN_COMPILER CLANG_FORMAT

echo "Processing files in parallel (using $PARALLEL_JOBS jobs)..."
echo ""

# Create temporary directory for results
temp_dir=$(mktemp -d)
trap "rm -rf $temp_dir" EXIT

# Process files in parallel (use null-terminated input and positional args to avoid
# filename splitting and shell interpolation of special characters). Use -I {} so
# the positional arguments to the child shell are in the correct order: '{}' -> $1
# and "$temp_dir" -> $2.
printf '%s\0' "${source_files[@]}" | xargs -0 -P "$PARALLEL_JOBS" -I {} bash -c 'process_file "$1" "$2"' _ {} "$temp_dir"

# Collect results
for file in "${source_files[@]}"; do
    # Use same hash generation as in process_file
    file_hash=""
    if command -v sha1sum >/dev/null 2>&1; then
        file_hash=$(echo -n "$file" | sha1sum | awk '{print $1}')
    elif command -v shasum >/dev/null 2>&1; then
        file_hash=$(echo -n "$file" | shasum | awk '{print $1}')
    elif command -v md5sum >/dev/null 2>&1; then
        file_hash=$(echo -n "$file" | md5sum | awk '{print $1}')
    else
        file_hash=$(echo "$file" | sed 's/[^a-zA-Z0-9]/_/g')
    fi
    file_id="$file_hash"
    result_file="$temp_dir/result_$file_id"

    if [ -f "$result_file" ]; then
        read -r has_pragmas file_pragmas file_passed file_failed file_parse_errors < "$result_file"

        if [ "$has_pragmas" -eq 1 ]; then
            files_with_pragmas=$((files_with_pragmas + 1))
        fi

        total_pragmas=$((total_pragmas + file_pragmas))
        passed=$((passed + file_passed))
        failed=$((failed + file_failed))
        parse_errors=$((parse_errors + file_parse_errors))
    fi
done

# Read failure details from all per-file failure logs
# Enable nullglob to handle case where no failure files exist
shopt -s nullglob
for failure_file in "$temp_dir"/failures_*; do
    while IFS='|' read -r file pragma reason; do
        failure_files+=("$file")
        failure_pragmas+=("$pragma")
        failure_reasons+=("$reason")
    done < "$failure_file"
done
shopt -u nullglob

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
