# ROUP accparser compatibility layer

This adapter builds the accparser C++ AST from ROUP's opaque, typed C ABI. It
uses the pinned accparser IR sources without ANTLR.

The contract is strict:

- input must contain an OpenACC pragma or Fortran sentinel;
- callers must select C, C++, or Fortran explicitly with `setLang` before
  parsing, and the selected profile must agree with the source form;
- parse, schema, conversion, and representation failures throw exceptions;
- the adapter never returns null for an error;
- every typed C-ABI field must be consumed exactly once;
- directive and clause kinds are classified only by exhaustive C-ABI ordinal
  tables, and closed payload enums are consumed only as numeric tags;
- no source prefix is fabricated and no rendered clause is reparsed;
- clause occurrences and ordering are preserved rather than merged; and
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

The pinned fixture audit is recorded in [STRICT_AUDIT.md](STRICT_AUDIT.md).
The registered gate consists of repository-owned `strict_contract`,
`lang_flag_test`, and `compat_caller` executables, a source audit forbidding
string-based enum classification, and the explicitly audited atomic C and
Fortran fixture pair.
Strict cache element/subarray, end-kind, and routine-parameter coverage
lives in the repository-owned `strict_contract` test and strict C/Fortran
cache fixture pairs because the upstream cache fixtures contain scalar names
forbidden by the OpenACC specification.
The remaining permissive builtin and OpenACC-VV corpora are retained as
audit inputs but are never discovered or registered by glob. All registered
tests are mandatory and hard-time-limited. Audited input/reference hashes make
fixture drift a configure-time failure requiring explicit re-audit.
