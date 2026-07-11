# C ABI architecture

ROUP's parser is the safe Rust package at the workspace root. It builds as an
`rlib`, contains no C-facing layout annotations, and forbids unsafe Rust. The
optional `roup-capi` package is the only ABI layer; building or using `roup`
never requires it.

## Boundary and ownership

The public header is [`crates/roup-capi/include/roup.h`](../crates/roup-capi/include/roup.h).
It uses fixed-layout value types, opaque generational handles, explicit
pointer-plus-length input, and caller-owned output buffers. Input is copied and
validated as UTF-8 before it reaches the parser. Output copies are
all-or-nothing: an undersized buffer is left untouched and returns a structured
error.

All foreign-pointer reads and writes are isolated in
`crates/roup-capi/src/boundary.rs`. The rest of the ABI crate denies unsafe
code, while the parser crate forbids it entirely. Every parser, directive,
semantic node, and diagnostic lives in one server-owned generational arena.
The two `u64` words in each public handle are an opaque, padding-free identity;
there is no caller-controlled type tag. The arena's stored-object variant is
checked on every operation before access or release, so cross-family retagging,
stale identities, oversized indices, and fabricated identities are hard errors
rather than nullable-pointer conventions.

## Typed queries

The ABI does not expose Rust enum layouts or numeric discriminants. Directive
and clause kinds are translated deliberately into ABI value structs whose
ordinals have a named constant for every schema entry. Exhaustive Rust matches
perform that translation, so adding a parser enum variant without updating the
ABI is a compile error. Tests also require the Rust schema order and checked C
header to contain exactly the same unique names and ordinals.

Clause and directive-parameter data is queried through typed field descriptors
and typed scalar or list element operations. Closed semantic values use `u32`
or `u32`-list fields with named constants. Structured alternatives use tagged
child nodes. Boolean fields use explicit `*_field_bool` getters returning only
zero or one; they are never exposed through a generic integer-width getter.
Strings are reserved for open lexical leaves such as identifiers,
type and expression renderings, variable designators, and literal contents;
consumers never parse a string to recover a closed enum. The service enforces
this with a private leaf-type whitelist instead of accepting arbitrary
`Display` or `ToString` values.

OpenMP directive-parameter tags follow their actual grammar: `allocate`,
`threadprivate`, `groupprivate`, and historical `declare target` lists have
distinct tags and tagged item families rather than a shared string-list tag.
Qualified names, Fortran common blocks, flush designators, mapper IDs, and C++
template/operator identifiers therefore remain distinguishable. A `declare
variant` parameter exposes its optional `base` and tagged required `function`
separately, and a present `declare simd` parameter always has a required
`function` field. Declare-reduction and declare-induction identifiers,
combiners, recursive initializers, and inductor forms are likewise tagged
rather than flattened and later reparsed.

Predefined OpenMP allocators and user allocator identifiers share one tagged
allocator family. This preserves the built-in/custom distinction without
reserving built-in spellings in an open identifier string.

OpenMP `apply` carries a tagged loop modifier and recursively typed applied
directive nodes. `induction` carries its modifier, typed declared-induction
identifier, step expression, and variable-list nodes as separate fields.
Interop `init` carries numeric interop types and recursive preference
specification/selector nodes; depobj `init` instead carries a tagged dependence,
locator node, and variable. None of these payloads is rendered and reparsed at
the C boundary.

Every OpenMP clause exposes an optional numeric
`ROUP_FIELD_DIRECTIVE_NAME_MODIFIER` containing a named directive ordinal;
the field is identical on top-level and recursively nested clause nodes.
`threadset` and `memscope` are numeric closed values, `looprange` exposes
separate `first` and `count` fields, and map mapper IDs use the tagged mapper
family. Selector string-literal nodes expose decoded content plus numeric
character-encoding and quote-style fields. Standard device-kind and vendor
properties additionally cross as closed numeric selector nodes; implementation
properties outside those standardized sets remain explicit open lexical nodes.
An adapter can therefore reject an unrepresentable open property without
classifying a string or substituting an `unknown` enum.

OpenACC `bind` is exposed as a tagged name-or-string-literal node, including a
numeric character-encoding tag for literals. Device types and reduction
operators are tagged so user-defined names remain open leaves while standard
values remain closed. `worker` and `vector` expose either no fields for the
bare form or one numeric modifier plus one scalar expression; a list-shaped or
partially populated payload cannot cross the ABI.

Directive, clause, and diagnostic span queries return half-open UTF-8 byte
ranges plus one-based line and column positions in the original input. Logical
line parsing retains the physical mapping, including tokens that cross a
continuation.

There is intentionally no generic whole-payload string query. A consumer that
cannot represent a typed field must return a hard conversion error.

## Header maintenance

The checked-in header is the source of truth for C consumers. The ABI crate's
tests compare its layouts and constants with the Rust definitions, and its
build script copies that exact header into Cargo's output directory. There is
no source-scraping constants generator and no generated header in the parser
crate.
