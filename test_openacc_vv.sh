#!/bin/bash
#
# test_openacc_vv.sh - Round-trip OpenACC directives from OpenACCV-V through ROUP
#
# This script validates ROUP by:
# 1. Cloning the OpenACCV-V test suite (on-demand)
# 2. Extracting all C/C++/Fortran OpenACC directives
# 3. Round-tripping each directive through ROUP's parser
# 4. Normalizing directives and comparing round-tripped output
# 5. Reporting pass/fail statistics
#
# Usage:
#   ./test_openacc_vv.sh                          # Auto-clone to target/openacc_vv
#   OPENACC_VV_PATH=/path ./test_openacc_vv.sh    # Use existing clone
#   PYTHON=python ./test_openacc_vv.sh
#

set -euo pipefail

REPO_URL="https://github.com/OpenACCUserGroup/OpenACCV-V"
REPO_PATH="${OPENACC_VV_PATH:-target/openacc_vv}"
TESTS_DIR="Tests"
PYTHON="${PYTHON:-python3}"
MAX_DISPLAY_FAILURES=10

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "========================================="
echo "  OpenACCV-V Round-Trip Validation"
echo "========================================="
echo

echo "Checking for required tools..."
for tool in cargo "$PYTHON"; do
    if ! command -v "$tool" &>/dev/null; then
        echo -e "${RED}Error: $tool not found in PATH${NC}"
        exit 1
    fi
done
echo -e "${GREEN}✓${NC} All required tools found"
echo

if [ ! -d "$REPO_PATH" ]; then
    echo "OpenACCV-V not found at $REPO_PATH"
    echo "Cloning from $REPO_URL..."
    git clone --depth 1 "$REPO_URL" "$REPO_PATH" || {
        echo -e "${RED}Failed to clone OpenACCV-V${NC}"
        exit 1
    }
    echo -e "${GREEN}✓${NC} Cloned successfully"
    echo
elif [ ! -d "$REPO_PATH/$TESTS_DIR" ]; then
    echo -e "${RED}Error: $REPO_PATH exists but $TESTS_DIR/ not found${NC}"
    exit 1
else
    echo "Using existing OpenACCV-V at $REPO_PATH"
    echo
fi

echo "Building roup_roundtrip binary..."
cargo build --quiet --bin roup_roundtrip || {
    echo -e "${RED}Failed to build roup_roundtrip${NC}"
    exit 1
}
echo -e "${GREEN}✓${NC} Binary built"
echo

ROUNDTRIP_BIN="./target/debug/roup_roundtrip"

REPORT=$(mktemp)
if "$PYTHON" scripts/openacc_vv_runner.py \
    --tests-dir "$REPO_PATH/$TESTS_DIR" \
    --roundtrip-bin "$ROUNDTRIP_BIN" \
    --max-failures "$MAX_DISPLAY_FAILURES" \
    > "$REPORT" 2>&1; then
    cat "$REPORT"
    rm -f "$REPORT"
    exit 0
else
    cat "$REPORT"
    echo -e "${RED}✗ Some directives failed to round-trip${NC}"
    rm -f "$REPORT"
    exit 1
fi
