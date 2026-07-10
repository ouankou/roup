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
Rust layouts, serialized clause payloads, or a second parser. Invalid input,
unsupported target-IR shapes, stale handles, and conversion failures are hard
errors; `parseOpenMP` throws a C++ exception and never returns a fallback AST.

Call `setLang(Lang_C)`, `setLang(Lang_Cplusplus)`, or
`setLang(Lang_Fortran)` before the first parse. The adapter intentionally starts
in `Lang_unknown`, and parsing before explicit language selection is a hard
error. Under `Lang_Fortran`, `!$OMP`/`!$` select free form while
`C$OMP`/`C$` and `*$OMP`/`*$` select fixed form. A pragma or sentinel that
conflicts with the selected base language is a hard error.

Directive/clause kinds and every closed semantic choice cross the ABI as
numeric tags; structured values cross as owned semantic-node handles. The
adapter never queries directive or clause names to recover enums, never
compares closed semantic strings, and never reparses a rendered payload.
Strings at this boundary are only identifier, type, expression, or literal
spellings that ompparser's IR itself stores lexically. All typed end ordinals
have explicit paired mappings, including `end allocators` and `end dispatch`.

Pinned ompparser has no structured representation for a `declare induction`
directive argument: its only API is a raw passthrough string. The compatibility
adapter does not reconstruct or inject that string, so such a conversion throws
until upstream provides a typed API. Its `apply`, `induction`, and
`prefer_type` APIs likewise retain the obsolete pseudo-transform/expression
bags or one raw preference string; the adapter hard-errors for those typed
payloads instead of rendering ROUP's semantic tree back into text. Likewise,
ROUP's open device-kind and
implementation-vendor selector identifiers are rejected because ompparser
offers only closed enums for them; no `unknown` fallback is used.

CTest is the explicit strict compatibility gate. It contains the local ABI and
adapter contract executables plus a fixed manifest of audited pinned fixtures:
atomic, barrier, critical, C++ declare-mapper, end-declare-target, flush, parallel,
scan, taskgroup, taskwait, taskyield, and tile, including the listed Fortran
variants. The atomic and parallel fixtures are transformed only by
configure-time exact replacements recorded in `CMakeLists.txt`; a changed
pinned input or audited fixture hash is a configuration error. The legacy
trailing atomic comma is retained as an explicit hard-negative expectation.

The complete upstream builtin, OpenMP-VV, and example trees remain available as
audit material but are not registered implicitly. They mix permissive invalid
inputs, legacy spelling-oriented output, and whole-source extraction workloads,
so their aggregate result is not an adapter contract. No registered gate test
is disabled or allowed to fail, and every test has a hard timeout.

## Troubleshooting

- Ensure the submodule is initialised if headers are missing.
- Build `roup-capi` in release mode before invoking CMake.
- Export the appropriate library search path if executables cannot load the rebuilt libraries.
