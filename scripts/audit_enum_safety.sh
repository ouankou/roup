#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if ! command -v rg >/dev/null 2>&1; then
    echo "error: ripgrep (rg) is required for enum/safety audit" >&2
    exit 1
fi

failures=0

report_hits() {
    local title="$1"
    local hits="$2"
    if [[ -n "$hits" ]]; then
        echo "error: $title" >&2
        echo "$hits" >&2
        failures=1
    fi
}

missing_forbid=()
for file in src/ast/mod.rs src/debugger/mod.rs src/ir/mod.rs src/lexer.rs src/parser/mod.rs; do
    if ! rg -q '#!\[forbid\(unsafe_code\)\]' "$file"; then
        missing_forbid+=("$file")
    fi
done
if (( ${#missing_forbid[@]} > 0 )); then
    report_hits "safe Rust module missing #![forbid(unsafe_code)]" "$(printf '%s\n' "${missing_forbid[@]}")"
fi

unsafe_hits="$(
    rg -n '\bunsafe\b' src \
        --glob '!**/c_api.rs' \
        --glob '!**/c_api/openacc.rs' \
        --glob '!**/constants_gen.rs' \
        --glob '!**/lib.rs' \
        --glob '!compat/**' \
    | rg -v 'forbid\(unsafe_code\)' || true
)"
report_hits "unsafe outside the FFI/generator allowlist" "$unsafe_hits"

generic_hits="$(
    rg -n 'ClauseData::Generic|IrClauseData::Generic|Generic \{' \
        src/ast src/ir src/parser/ast_builder.rs src/c_api.rs src/c_api/openacc.rs \
        --glob '!compat/**' || true
)"
report_hits "generic clause payload fallback is forbidden after parse" "$generic_hits"

unknown_payload_hits="$(
    rg -n 'DirectiveKind::Unknown|DepobjUpdateDependence::Unknown|DoacrossType::Unknown|SeverityKind::Unknown|AtKind::Unknown|InitKind::Unknown|ApplyTransformKind::Unknown|AccDeviceType::Unknown|AccReductionOperator::Unknown' \
        src/ast src/ir src/parser src/c_api.rs src/c_api/openacc.rs \
        --glob '!compat/**' || true
)"
report_hits "unknown semantic enum payloads are forbidden after parse" "$unknown_payload_hits"

other_payload_hits="$(
    rg -n 'DirectiveName::Other\b|ClauseName::Other\b' \
        src/ast src/ir src/c_api.rs src/c_api/openacc.rs \
        --glob '!compat/**' \
    | rg -v 'src/ir/convert.rs:.*ConversionError::Unknown' \
    | rg -v 'src/c_api.rs:[0-9]+:.*let other = DirectiveName::Other' || true
)"
report_hits "parser Other payloads must not survive into AST/IR/C API semantic paths" "$other_payload_hits"

stringly_hits="$(
    rg -n '==\s*['"'"'"]|!=\s*['"'"'"]|starts_with\(|strip_prefix\(|contains\(|match .*as_str\(\)|to_ascii_lowercase\(\).*as_str\(' \
        src/ast/mod.rs src/ir/convert.rs src/ir/validate.rs \
        --glob '!compat/**' || true
)"
report_hits "string/char semantic checks outside parser boundary" "$stringly_hits"

if (( failures != 0 )); then
    exit 1
fi

echo "enum/safety audit passed"
