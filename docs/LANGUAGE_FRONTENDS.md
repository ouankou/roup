# Language-Aware OpenMP Clause Parsing

The `language_frontends` module provides the minimal C/C++/Fortran awareness
required to fully parse OpenMP pragmas and populate the IR with structured
data. It focuses on clause payloads (lists of identifiers, array sections,
and simple expressions) instead of attempting to implement entire host
language grammars.

## Feature Flag

The implementation lives behind the `language_frontends` Cargo feature. The
feature is enabled by default:

```toml
[features]
default = ["language_frontends"]
language_frontends = []
```

Disable the feature to fall back to the legacy, string-only behaviour:

```bash
cargo build --no-default-features --features ""
```

When disabled, clause payloads degrade gracefully to basic identifier lists
so existing callers continue to work.

## Supported Syntax

With the feature enabled the parser understands the following constructs
inside clause payloads:

* **C/C++ array sections** – `array[lower:length[:stride]]` including chained
  sections such as `matrix[0:N][i]`.
* **Fortran array sections** – `array(lower:upper[:stride])` and rank-
  separated lists (e.g. `field(1:n, :)`).
* **Language-aware item lists** – `private`, `firstprivate`, `shared`,
  `depend`, `map`, and `linear` clauses now yield `ClauseItem::Variable`
  entries when array sections are present.
* **Expression handling** – the array section parts are converted into
  `Expression` values using the existing expression infrastructure. Complex
  sub-expressions automatically fall back to the `Unparsed` representation.

These improvements ensure that downstream consumers can reason about the IR
without re-parsing clause strings.

## Limitations

* `linear` modifiers (`modifier(list): step`) are still reported as
  unsupported.
* Mapper syntax inside `map` clauses is not yet recognised.
* Fortran derived type member access (`foo%bar`) is treated as a generic
  identifier.

Future work can extend the module without affecting the default parser or the
compatibility mode used when the feature is disabled.
