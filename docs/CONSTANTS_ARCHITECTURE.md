# Constants architecture

The C bindings expose numeric directive and clause identifiers. Rather than
maintaining those numbers in multiple places, the project generates
`roup_constants.h` from the authoritative C API mapping tables:
`src/c_api.rs` for OpenMP and `src/c_api/openacc.rs` for OpenACC.

- `build.rs` (or `cargo run --bin gen`) parses the match arms in
  the C API mapping functions using `syn`.
- The script writes the header and a checksum so CI can confirm the committed
  file is current.
- C and C++ code include `roup_constants.h` and use the macros in switches.

When an OpenMP directive or clause is added, update `src/c_api.rs`; when an
OpenACC directive or clause is added, update `src/c_api/openacc.rs`. Rebuild
afterward. Never change `roup_constants.h` by hand because the next build will
overwrite it.
