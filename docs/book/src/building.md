# Building

ROUP is a Cargo workspace with two packages:

- `roup`: the complete safe Rust parser (`rlib` only)
- `roup-capi`: an optional C ABI (`rlib`, `staticlib`, and `cdylib`)

Both packages use the Rust 2024 edition. The repository MSRV is Rust 1.88,
which covers the complete required dependency and tooling graph, including the
pinned mdBook 0.5.4 documentation tool.

The root package is the default workspace member, so an ordinary build does
not compile or link the ABI:

```bash
cargo build --locked
```

Build both packages when the ABI is required:

```bash
cargo build --locked --workspace
cargo build --locked --release -p roup-capi
```

The public C header is checked in at `crates/roup-capi/include/roup.h`. The ABI
build copies that exact file to its Cargo output directory; it does not generate
constants by scraping Rust source.

## Compatibility libraries

Initialize the two pinned submodules once:

```bash
git submodule update --init --recursive
```

Then build and test either adapter in a separate directory:

```bash
cargo build --locked --release -p roup-capi

cmake -S compat/ompparser -B target/compat/ompparser -DCMAKE_BUILD_TYPE=Release
cmake --build target/compat/ompparser --parallel
ctest --test-dir target/compat/ompparser --output-on-failure --no-tests=error

cmake -S compat/accparser -B target/compat/accparser -DCMAKE_BUILD_TYPE=Release
cmake --build target/compat/accparser --parallel
ctest --test-dir target/compat/accparser --output-on-failure --no-tests=error
```

CMake treats a missing or mismatched prerequisite as an error. It never changes
the recorded submodule revisions or silently substitutes a previously built
library. Each adapter imports its pinned upstream test directory unchanged; at
the current revisions this is 1,534 ompparser tests and 918 accparser tests,
plus five repository-owned contract/audit tests across the two builds.
