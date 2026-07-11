# ROUP accparser compatibility layer

This adapter builds the accparser C++ AST from ROUP's opaque, typed C ABI. It
uses the pinned accparser IR sources without ANTLR.

The contract is strict internally while preserving accparser's public behavior:

- input must contain an OpenACC pragma or Fortran sentinel;
- callers may use accparser's default language behavior or explicitly select
  C, C++, or Fortran with `setLang`; an explicit profile must agree with the
  source form;
- parse, schema, conversion, and representation failures throw exceptions;
- no error produces a partial or fallback AST;
- every typed C-ABI field must be consumed exactly once;
- directive and clause kinds are classified only by exhaustive C-ABI ordinal
  tables, and closed payload enums are consumed only as numeric tags;
- no source prefix is fabricated and no rendered clause is reparsed;
- clause merging, ordering, and source spelling follow the pinned accparser
  public contract; and
- source locations come directly from ROUP byte/line/column spans.

## Build

Build the optional C ABI explicitly before configuring CMake:

```bash
cargo build --locked --release -p roup-capi
cmake -S compat/accparser -B compat/accparser/build
cmake --build compat/accparser/build -j
ctest --test-dir compat/accparser/build --output-on-failure --no-tests=error
```

`compat/accparser/build.sh` performs the same ordered build. CMake never invokes
Cargo or mutates the Rust target tree.

Requirements are CMake 3.20 or newer, a C++17 compiler, and a Rust toolchain
when building `roup-capi`. The pure `roup` Rust parser does not depend on this
adapter or on the C ABI crate.

The compatibility audit is recorded in [STRICT_AUDIT.md](STRICT_AUDIT.md).
CMake registers the pinned upstream test directory unchanged. At the currently
recorded submodule revision this is 918 upstream builtin and OpenACC-VV tests.
The repository adds only `strict_contract` and the no-string-enum source audit,
for 920 mandatory tests in total. No upstream source, fixture, reference output,
expected result, or test registration is rewritten, filtered, disabled, or
allowed to fail.
