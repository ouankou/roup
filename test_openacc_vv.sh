#!/bin/bash
#
# test_openacc_vv.sh - Validate OpenACC directives from OpenACCV-V using ROUP
#
# This script performs the following steps:
# 1. Clones the OpenACCV-V repository (if not already present)
# 2. Builds the `roup_roundtrip_acc` binary
# 3. Runs all directives through the round-trip validator
# 4. Prints a human-readable summary and writes a JSON report
#
# Usage:
#   ./test_openacc_vv.sh
#   OPENACC_VV_PATH=/path ./test_openacc_vv.sh
#

set -euo pipefail

REPO_URL="https://github.com/OpenACCUserGroup/OpenACCV-V"
REPO_PATH="${OPENACC_VV_PATH:-target/openacc_vv}"
TESTS_DIR="Tests"
ROUNDTRIP_BIN="./target/debug/roup_roundtrip_acc"
REPORT_PATH="target/openacc_vv_report.json"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

mkdir -p "$(dirname "$REPORT_PATH")"

cat <<'BANNER'
=========================================
  OpenACCV-V Round-Trip Validation
=========================================
BANNER

# Check prerequisites
for tool in cargo git python3; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo -e "${RED}Error: required tool '$tool' not found in PATH${NC}"
        exit 1
    fi
done

# Ensure repository exists
if [ ! -d "$REPO_PATH" ]; then
    echo "Cloning OpenACCV-V into $REPO_PATH..."
    git clone --depth 1 "$REPO_URL" "$REPO_PATH"
    echo -e "${GREEN}✓${NC} Clone complete"
else
    echo "Using existing OpenACCV-V repository at $REPO_PATH"
fi

if [ ! -d "$REPO_PATH/$TESTS_DIR" ]; then
    echo -e "${RED}Error: Tests directory not found at $REPO_PATH/$TESTS_DIR${NC}"
    exit 1
fi

# Build round-trip binary
echo "Building roup_roundtrip_acc binary..."
cargo build --quiet --bin roup_roundtrip_acc
if [ ! -x "$ROUNDTRIP_BIN" ]; then
    echo -e "${RED}Error: round-trip binary not found at $ROUNDTRIP_BIN${NC}"
    exit 1
fi

echo -e "${GREEN}✓${NC} Binary built"

echo "Running OpenACC directive validation..."
set +e
python3 scripts/run_openacc_vv.py \
    --tests-dir "$REPO_PATH/$TESTS_DIR" \
    --binary "$ROUNDTRIP_BIN" \
    --json-output "$REPORT_PATH"
status=$?
set -e

echo ""
echo "JSON report written to $REPORT_PATH"

exit $status
