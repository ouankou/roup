# OpenMP typed directive and clause components

ROUP represents OpenMP semantics directly. Directive parameters and clause
payloads are enums and checked record types, not strings that a consumer must
split or parse again.

## Directive parameters

Dedicated parameter variants cover constructs such as:

- distinct `allocate`, `threadprivate`, `groupprivate`, and historical
  `declare target` lists;
- `critical`, `flush`, checked-lvalue `depobj`, and construct-name parameters;
- `declare mapper`, `declare reduction`, `declare simd`, and `declare
  induction` declarations; and
- a `declare variant` target with separate optional base and required variant
  function names.

Storage and historical `declare target` lists admit only whole qualified
variable or procedure names and Fortran named common blocks. Array elements,
array sections, and object members are hard errors because the OpenMP
restrictions do not permit parts of variables in these lists. A present
Fortran `declare simd(proc-name)` parameter always contains a procedure name;
an empty target is not representable.

Historical C++ template-id variant names remain accepted cumulatively from
OpenMP 5.0. A `base-name:` prefix is available for Fortran from OpenMP 5.0 and
for C/C++ from OpenMP 5.2, matching the host-specific historical grammars.

`declare reduction` stores a typed reduction identifier, validated type names,
combiner expression, and optional initializer. `declare induction` stores the
induction identifier and its validated type-specifier list, including paired
variable and step types. A malformed declaration cannot be represented by a
partially populated record.

## Clause payloads

Clause payload variants distinguish semantic families, including:

- checked expressions, identifiers, type names, and locator lists;
- scheduling, ordering, binding, mapping, dependence, and reduction kinds;
- atomic operation and memory-order data;
- mapper, iterator, induction, linear, and allocator records;
- metadirective selectors with typed traits and nested directives;
- transformation trees for `apply`; and
- actual requirement clauses on `requires` rather than a synthesized summary
  string.

Lists are delimiter-aware and their elements are parsed once. A locator list
accepts locator shapes only; it cannot silently retain a general expression.
Nested directives retain spans into the outer physical source, including when
a token crosses a line splice.

## Standard aliases

Historical and alternate standardized spellings map to the same typed kind and
payload. Source-facing tools can recover the written spelling by slicing the
checked name span against the original input. Semantic consumers should use the
canonical kind and payload instead of comparing source text.

Fortran `end allocators` and `end dispatch`, including compact spellings, have
dedicated typed directive kinds introduced in OpenMP 5.2 and participate in
strict opener/end pairing.

## Optional C ABI representation

The optional ABI exposes the same structure through field metadata and owned
child-node handles. Scalar leaves, UTF-8 leaves, lists, and nested records have
distinct value kinds. Directive-specific list tags remain distinct in the ABI,
and `declare variant` exposes `base` and `function` as separate fields. There
is deliberately no operation that returns a whole rendered payload.

If a new Rust payload cannot be represented by these fields, the ABI and both
adapters must be extended. Substituting a raw payload or default value is not a
valid conversion.
