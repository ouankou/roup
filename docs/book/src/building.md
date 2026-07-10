# Building

ROUP is a Cargo workspace with two packages:

- `roup`: the complete safe Rust parser (`rlib` only)
- `roup-capi`: an optional C ABI (`rlib`, `staticlib`, and `cdylib`)

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
cmake -S compat/ompparser -B target/compat/ompparser -DCMAKE_BUILD_TYPE=Release
cmake --build target/compat/ompparser --parallel
ctest --test-dir target/compat/ompparser --output-on-failure

cmake -S compat/accparser -B target/compat/accparser -DCMAKE_BUILD_TYPE=Release
cmake --build target/compat/accparser --parallel
ctest --test-dir target/compat/accparser --output-on-failure
```

CMake treats a missing or mismatched prerequisite as an error. It never changes
the recorded submodule revisions or silently substitutes a previously built
library.
