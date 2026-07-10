#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

failures=0

reject_matches() {
    local title="$1"
    shift

    local matches
    local status
    set +e
    matches="$(rg "$@")"
    status=$?
    set -e

    if (( status == 0 )); then
        echo "error: $title" >&2
        printf '%s\n' "$matches" >&2
        failures=1
    elif (( status != 1 )); then
        echo "error: audit search failed while checking: $title" >&2
        exit "$status"
    fi
}

if ! rg -q '^#!\[forbid\(unsafe_code\)\]$' src/lib.rs; then
    echo "error: the pure Rust crate must forbid unsafe code at its root" >&2
    failures=1
fi

if ! rg -q '^#!\[deny\(unsafe_code\)\]$' crates/roup-capi/src/lib.rs; then
    echo "error: the C ABI crate must deny unsafe code by default" >&2
    failures=1
fi

if ! rg -q '^#\[allow\(unsafe_code\)\]$' crates/roup-capi/src/lib.rs; then
    echo "error: the C ABI must explicitly isolate its audited unsafe boundary" >&2
    failures=1
fi

if ! rg -q '^trait AbiStringLeaf \{' crates/roup-capi/src/service.rs; then
    echo "error: C ABI string projection must use an explicit lexical-leaf whitelist" >&2
    failures=1
fi

reject_matches \
    "unsafe Rust is only permitted in crates/roup-capi/src/boundary.rs" \
    -n '\bunsafe[[:space:]]*(fn|extern|impl|trait|\{)' src crates/roup-capi/src \
    -g '*.rs' -g '!crates/roup-capi/src/boundary.rs'

reject_matches \
    "foreign pointer access is only permitted in the audited boundary module" \
    -n '(\*const|\*mut|slice::from_raw_parts|ptr::|\.as_ptr\(\))' \
    crates/roup-capi/src -g '*.rs' -g '!boundary.rs'

reject_matches \
    "the pure Rust crate must not expose C/static library artifacts" \
    -n 'crate-type[[:space:]]*=.*(cdylib|staticlib)' Cargo.toml

reject_matches \
    "semantic Rust enums must not carry ABI representations or numeric discriminants" \
    -n '(^[[:space:]]*#\[repr\(|^[[:space:]]*[A-Za-z_][A-Za-z0-9_]*[[:space:]]*=[[:space:]]*-?[0-9])' \
    src/ast src/ir src/host -g '*.rs'

reject_matches \
    "opaque expression fallback nodes are forbidden" \
    -n '(ExpressionKind::Unparsed|\bUnparsed[[:space:]]*\(|\bComplex[[:space:]]*\()' src -g '*.rs'

reject_matches \
    "unknown/default host-language sentinels are forbidden" \
    -n 'Language::Unknown|Unknown[[:space:]]*=[[:space:]]*[0-9]|impl Default for Language' \
    src -g '*.rs'

reject_matches \
    "presentation-only formatting flags are forbidden in semantic AST data" \
    -n '\b(space_after_colon|comma_separated)\b' src/ast src/ir src/parser -g '*.rs'

reject_matches \
    "normalization and render/reparse repair paths are forbidden" \
    -n '(ClauseNormalizationMode|merge_clauses|openmp_normalization|render[^[:space:]]*_and_parse|unparse[^[:space:]]*_and_parse)' \
    src tests -g '*.rs'

reject_matches \
    "C++ adapters must not use raw passthrough payload APIs" \
    -n '(addPassthroughItem|specificationToString)' \
    compat/ompparser/src compat/accparser/src -g '*.cpp' -g '*.h'

reject_matches \
    "adapter language selection must not silently default to C" \
    -n 'current_lang[[:space:]]*=[[:space:]]*(Lang_C|ACC_Lang_C)([^A-Za-z_]|$)' \
    compat/ompparser/src compat/accparser/src -g '*.cpp' -g '*.h'

reject_matches \
    "adapters must classify directive and clause kinds from typed ordinals" \
    -n '(directive_kind|clause_kind)[[:space:]]*\([[:space:]]*const[[:space:]]+std::string|enum_key[[:space:]]*\(' \
    compat/ompparser/src compat/accparser/src -g '*.cpp' -g '*.h'

reject_matches \
    "closed semantic fields must not be read as adapter strings" \
    -n -U '(optional|required)_strings?\([[:space:]]*ROUP_FIELD_(KIND|MODIFIER|MODIFIERS|OPERATOR|DIRECTIVE|DIRECTIVES|BEHAVIOR|CATEGORY|DEPEND_TYPE|MEMORY_ORDER|ALLOCATOR)[[:space:]]*[,)]' \
    compat/ompparser/src compat/accparser/src -g '*.cpp' -g '*.h'

reject_matches \
    "adapter-owned code must not classify values with string-literal equality" \
    -n '((==|!=)[[:space:]]*"[^"]*"|"[^"]*"[[:space:]]*(==|!=))' \
    compat/ompparser/src compat/accparser/src -g '*.cpp' -g '*.h'

reject_matches \
    "C ABI closed semantic fields must use U32 or tagged-node projection" \
    -n -U 'ClauseField::(string|strings|owned_strings)\([[:space:]]*crate::ROUP_FIELD_(KIND|MODIFIER|MODIFIERS|OPERATOR|DIRECTIVE|DIRECTIVES|BEHAVIOR|CATEGORY|DEPEND_TYPE|MEMORY_ORDER|ALLOCATOR)[[:space:]]*[,)]' \
    crates/roup-capi/src/service.rs

reject_matches \
    "C ABI string projection must not accept arbitrary Display/ToString values" \
    -n 'fn (string|strings)[^\n]*(ToString|Display)|impl<[^>]*(ToString|Display)[^>]*>[[:space:]]+AbiStringLeaf' \
    crates/roup-capi/src/service.rs

if ! rg -q 'FieldValue::Bool\(_\).*ROUP_FIELD_VALUE_BOOL' \
    crates/roup-capi/src/service.rs; then
    echo "error: boolean ABI fields must retain their dedicated value kind" >&2
    failures=1
fi

if ! rg -q 'FieldValue::U64\(_\).*ROUP_FIELD_VALUE_U64' \
    crates/roup-capi/src/service.rs; then
    echo "error: u64 ABI fields must retain their dedicated value kind" >&2
    failures=1
fi

if ! rg -q 'fn scalar_field_bool\(' crates/roup-capi/src/service.rs; then
    echo "error: boolean ABI fields require an explicit bool getter" >&2
    failures=1
fi

reject_matches \
    "directive and clause ABI ordinals require exhaustive match projections" \
    -n '\.position\(' \
    crates/roup-capi/src/service.rs

reject_matches \
    "removed in-core C ABI implementation files must not be referenced" \
    -n '(src/c_api|roup_constants\.h|constants_gen|cargo run[^\n]*--bin gen)' \
    Cargo.toml README.md docs examples scripts test.sh .github \
    -g '*.rs' -g '*.toml' -g '*.md' -g '*.c' -g '*.cpp' -g '*.h' -g '*.sh' -g '*.yml' -g 'Makefile' \
    -g '!audit_enum_safety.sh'

if (( failures != 0 )); then
    exit 1
fi

echo "strict AST and safety audit passed"
