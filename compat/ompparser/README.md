# ROUP ompparser compatibility layer

This directory builds a ROUP-powered `libompparser` that preserves the original headers and IR while replacing the parser implementation.

## Requirements

- Rust toolchain (`cargo`)
- CMake ≥ 3.20
- A C++17 compiler

## Build and test

```bash
git submodule update --init --recursive
cargo build --locked --release -p roup-capi
cmake -S compat/ompparser -B compat/ompparser/build
cmake --build compat/ompparser/build
ctest --test-dir compat/ompparser/build --output-on-failure --no-tests=error
```

CMake never invokes Cargo. Configuration fails unless the caller has already
built `target/release/libroup_capi.a`; pass `-DROUP_STATIC_LIB=/path/to/archive`
to use another explicitly built archive.

## Linking

```bash
g++ app.cpp \
    -I/path/to/roup/compat/ompparser/ompparser/src \
    -L/path/to/roup/compat/ompparser/build -lompparser \
    -o app
```

The adapter traverses only the opaque, typed ROUP C ABI. It does not depend on
Rust layouts, serialized clause payloads, or a second parser. Malformed input
and unrepresentable target-IR shapes fail through ompparser's established
`parseOpenMP` contract: no partial or fallback AST is returned. Parser
rejections return `nullptr`; invalid pre-parse source-form or language input
remains a hard error. `roup_ompparser_last_error()` returns the thread-local
diagnostic for the most recent call in either case.

Call `setLang(Lang_C)`, `setLang(Lang_Cplusplus)`, or
`setLang(Lang_Fortran)` before parsing. `Lang_unknown` is rejected because a
pragma alone cannot distinguish C from C++, and silently choosing C would hide
host-expression errors. The selected profile must agree with the source form.
Under `Lang_Fortran`, `!$OMP`/`!$` select free form while `C$OMP`/`C$` and
`*$OMP`/`*$` select fixed form.

Directive/clause kinds and every closed semantic choice cross the ABI as
numeric tags; structured values cross as owned semantic-node handles. The
adapter never queries directive or clause names to recover enums, never
compares closed semantic strings, and never reparses a rendered payload.
Strings at this boundary are only identifier, type, expression, or literal
spellings that ompparser's IR itself stores lexically. All typed end ordinals
have explicit paired mappings, including `end allocators` and `end dispatch`.

Some pinned ompparser classes expose only lexical storage for semantics that
ROUP models structurally, including parts of `declare induction`, `induction`,
and `apply`. Such text is produced only in this final compatibility conversion
from already validated typed nodes; it is never carried through the Rust AST or
reparsed to recover semantics. A typed value that the upstream IR cannot
represent is a conversion error.

CTest registers the pinned upstream test directory unchanged. At the currently
recorded submodule revision this is 1,534 upstream tests covering the builtin,
OpenMP-VV, and example surfaces. Three repository-owned ABI/adapter tests are
added, for 1,537 mandatory tests in total. No upstream source, fixture,
reference output, expected result, or test registration is rewritten, filtered,
disabled, or allowed to fail.

## Troubleshooting

- Ensure the submodule is initialised if headers are missing.
- Build `roup-capi` in release mode before invoking CMake.
- Export the appropriate library search path if executables cannot load the rebuilt libraries.
