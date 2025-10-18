# Constants architecture

The C bindings expose numeric directive and clause identifiers. Rather than
maintaining those numbers in multiple places, the project generates
`roup_constants.h` from the authoritative tables in `src/c_api.rs`.

- `build.rs` (or `cargo run --bin gen`) parses the match arms in
  `directive_name_to_kind` and `convert_clause` using `syn`.
- The script writes the header and a checksum so CI can confirm the committed
  file is current.
- C and C++ code include `roup_constants.h` and use the macros in switches.

When a directive or clause is added, edit `src/c_api.rs` only and rebuild. Never
change `roup_constants.h` by hand—the next build will overwrite it.
