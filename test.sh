#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

submodule_status="$(git submodule status --recursive)"
if [[ -z "$submodule_status" ]]; then
    echo "error: no initialized parser submodules were found" >&2
    exit 1
fi
while IFS= read -r status_line; do
    marker="${status_line:0:1}"
    read -r _revision submodule_path _description <<<"${status_line:1}"
    case "$marker" in
        -|U)
            echo "error: parser submodule ${submodule_path} is missing or conflicted" >&2
            printf '%s\n' "$status_line" >&2
            exit 1
            ;;
        +)
            submodule_head="$(git -C "$submodule_path" rev-parse HEAD)"
            origin_main="$(git -C "$submodule_path" rev-parse refs/remotes/origin/main^{commit})"
            if [[ "$submodule_head" != "$origin_main" ]]; then
                echo "error: parser submodule ${submodule_path} is neither the pinned gitlink nor fetched origin/main" >&2
                printf '%s\n' "$status_line" >&2
                exit 1
            fi
            ;;
        ' ')
            ;;
        *)
            echo "error: unknown parser submodule state for ${submodule_path}" >&2
            printf '%s\n' "$status_line" >&2
            exit 1
            ;;
    esac
done <<<"$submodule_status"

cargo fmt --all --check
./scripts/audit_enum_safety.sh

cargo check --locked -p roup --all-targets
cargo check --locked --workspace --all-targets
cargo package --locked -p roup --allow-dirty
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace --all-targets
cargo bench --locked --workspace --no-run

RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps
mdbook test docs/book
mdbook build docs/book

cargo build --locked --release -p roup-capi
cc -std=c11 -Wall -Wextra -Werror -Icrates/roup-capi/include \
    -fsyntax-only crates/roup-capi/tests/header_c11.c
c++ -std=c++17 -Wall -Wextra -Werror -Icrates/roup-capi/include \
    -fsyntax-only crates/roup-capi/tests/header_cpp17.cpp

cmake -S compat/ompparser -B target/compat/ompparser -DCMAKE_BUILD_TYPE=Release
cmake --build target/compat/ompparser --parallel
ctest --test-dir target/compat/ompparser --output-on-failure --no-tests=error

cmake -S compat/accparser -B target/compat/accparser -DCMAKE_BUILD_TYPE=Release
cmake --build target/compat/accparser --parallel
ctest --test-dir target/compat/accparser --output-on-failure --no-tests=error

make -C examples/c clean all run-all BUILD_TYPE=release
make -C examples/cpp clean all run-all BUILD_TYPE=release
make -C examples/fortran clean all
make -C examples/fortran run-all
