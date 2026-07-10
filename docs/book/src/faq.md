# FAQ

## Does the Rust parser require a C toolchain?

No. `cargo build -p roup` builds the complete parser as safe Rust. The optional
`roup-capi` package and C++ compatibility adapters are separate consumers.

## What does an exact specification version mean?

It is an introduction ceiling. Syntax first standardized after the selected
version is rejected. Standardized older syntax remains accepted even if a
newer specification deprecated or removed it, which keeps maintained legacy
code parseable.

## Are spelling aliases preserved?

No. Standard aliases are accepted and canonicalized into one semantic AST
shape. Version compatibility is computed from typed parser provenance. The
checked name span can still select the exact spelling in the original source.

## What happens to unsupported host-language expressions?

Parsing fails with an `invalid-expression` diagnostic. A successful result
always contains a classified host-expression tree.

## Where is unsafe Rust used?

Only the optional C ABI's audited byte-copy boundary contains unsafe code. The
root parser forbids unsafe Rust, and all other ABI modules deny it.

## How are C ABI objects owned?

Parser, directive, child-node, and diagnostic objects share one server-owned
generational arena. Each padding-free two-word handle identifies exactly one
stored object, whose internal variant is checked before every access or
release. Each successful handle is released exactly once with its matching
release operation. Invalid, stale, fabricated, and wrong-kind handles are hard
errors.

## Why is there no clause payload string function?

Clause payloads are structured data rather than one scalar value. Consumers
query typed fields and must report an unsupported conversion directly.
