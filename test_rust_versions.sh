#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

if ! command -v rustup >/dev/null 2>&1; then
    echo "error: rustup is required to validate multiple Rust toolchains" >&2
    exit 1
fi
if ! command -v rg >/dev/null 2>&1; then
    echo "error: ripgrep is required to read the CI toolchain matrix" >&2
    exit 1
fi

if (( $# > 0 )); then
    versions=("$@")
else
    ci_line="$(rg -m1 '^[[:space:]]*rust:[[:space:]]*\[' .github/workflows/ci.yml)" || {
        echo "error: .github/workflows/ci.yml has no inline rust toolchain matrix" >&2
        exit 1
    }
    version_text="$(sed -E 's/^[^[]*\[([^]]+)\].*$/\1/' <<<"$ci_line" | tr -d '"' | tr ',' ' ')"
    read -r -a versions <<<"$version_text"
    if (( ${#versions[@]} == 0 )); then
        echo "error: the CI rust toolchain matrix is empty" >&2
        exit 1
    fi
fi

for version in "${versions[@]}"; do
    if [[ -z "$version" ]]; then
        echo "error: an empty Rust toolchain was requested" >&2
        exit 1
    fi

    echo "validating Rust $version"
    rustup toolchain install "$version" --profile minimal --component rustfmt,clippy
    cargo "+$version" fmt --all --check
    ./scripts/audit_enum_safety.sh
    cargo "+$version" check --locked -p roup --all-targets
    cargo "+$version" check --locked --workspace --all-targets
    cargo "+$version" clippy --locked --workspace --all-targets -- -D warnings
    cargo "+$version" test --locked --workspace --all-targets
done

echo "all requested Rust toolchains passed"
