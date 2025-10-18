# ROUP: Rust-based OpenMP Parser

ROUP is a Rust library and C/C++/Fortran FFI for parsing OpenMP directives. It focuses on predictable behaviour, clear safety
boundaries, and compatibility with existing ompparser-based toolchains.

> **Status:** Experimental. Interfaces may change between releases.

## Installation

### Rust

```toml
[dependencies]
roup = "0.4"
```

### C / C++ / Fortran

```bash
cargo build --release
# Link against target/release/libroup.{a,so,dylib}
```

## Usage

### Rust

```rust
use roup::parser::parse;

match parse("#pragma omp parallel for") {
    Ok(directive) => println!("{} clauses", directive.clauses.len()),
    Err(err) => eprintln!("{}", err),
}
```

### C (excerpt)

```c
#include <stdint.h>
typedef struct OmpDirective OmpDirective;
OmpDirective* roup_parse(const char* input);
int32_t roup_directive_clause_count(const OmpDirective* dir);
void roup_directive_free(OmpDirective* dir);

OmpDirective* dir = roup_parse("#pragma omp parallel for");
if (dir != NULL) {
    int32_t clauses = roup_directive_clause_count(dir);
    roup_directive_free(dir);
}
```

See the [tutorials](https://roup.ouankou.com/) for complete examples in Rust, C, C++, and Fortran.

## Highlights

- Complete registry of OpenMP directive and clause keywords from versions 3.0 through 6.0
- Idiomatic Rust API with FFI bindings for C, C++, and Fortran
- Unsafe code isolated to the FFI boundary with documented contracts
- Compatibility shim that implements the ompparser API on top of ROUP

## Documentation

The mdBook site at [roup.ouankou.com](https://roup.ouankou.com/) is the authoritative source for setup guides, tutorials,
reference material, and architectural notes.

## Testing

```bash
cargo test
./test.sh                 # Full local check (formatting, docs, compat layer)
./test_rust_versions.sh   # Verify MSRV and stable toolchains
```

The crate currently targets Rust 1.85 (MSRV) and the latest stable release.

## ompparser Compatibility

A compatibility layer provides the original ompparser API while delegating parsing to ROUP. Build it with:

```bash
cd compat/ompparser
./build.sh
```

More details are available in [`compat/ompparser/README.md`](compat/ompparser/README.md) and the documentation site.

## Contributing

Issues and pull requests are welcome. Please review the contributing guide on the documentation site before submitting changes.

## License

BSD-3-Clause
