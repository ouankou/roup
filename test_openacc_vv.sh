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
#   PARALLEL_JOBS=8 ./test_openacc_vv.sh        # Control parallel execution
#

set -euo pipefail

# Configuration
REPO_URL="https://github.com/OpenACCUserGroup/OpenACCV-V"
REPO_PATH="${OPENACC_VV_PATH:-target/openacc_vv}"
TESTS_DIR="Tests"
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

    # Read file content
    if ! content=$(cat "$file" 2>/dev/null); then
        echo "0 0 0 0 0" > "$result_file"
        return
    fi

    # Extract directives based on file type
    local ext="${file##*.}"
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
            echo "0 0 0 0 0" > "$result_file"
            return
            ;;
    esac

    if [ ${#pragmas[@]} -eq 0 ]; then
        echo "0 0 0 0 0" > "$result_file"
        return
    fi

    local file_pragmas=${#pragmas[@]}
    local file_passed=0
    local file_failed=0
    local file_parse_errors=0

    # Process each pragma
    for pragma in "${pragmas[@]}"; do
        # Normalize original pragma
        local original_normalized=$(normalize_pragma "$pragma")

        # Round-trip through ROUP
        if ! roundtrip=$(echo "$pragma" | "$ROUNDTRIP_BIN" --acc 2>/dev/null); then
            file_parse_errors=$((file_parse_errors + 1))
            file_failed=$((file_failed + 1))
            echo "$file|$pragma|Parse error" >> "$temp_dir/failures_$file_id"
            continue
        fi

        # Normalize round-tripped pragma
        local roundtrip_normalized=$(normalize_pragma "$roundtrip")

        # Compare
        if [ "$original_normalized" = "$roundtrip_normalized" ]; then
            file_passed=$((file_passed + 1))
        else
            file_failed=$((file_failed + 1))
            echo "$file|$pragma|Mismatch: got '$roundtrip'" >> "$temp_dir/failures_$file_id"
        fi
    done

    # Output: has_pragmas total_pragmas passed failed parse_errors
    echo "1 $file_pragmas $file_passed $file_failed $file_parse_errors" > "$result_file"
}

export -f process_file normalize_pragma
export ROUNDTRIP_BIN

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
