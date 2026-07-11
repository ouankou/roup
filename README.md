# ROUP

ROUP is a strict, safe Rust parser for OpenMP and OpenACC directives in C,
C++, and Fortran source forms.

[![Documentation](https://img.shields.io/badge/docs-roup.ouankou.com-blue)](https://roup.ouankou.com)

## Design

- Successful parses contain canonical typed directive, clause, selector,
  locator, and host-expression data.
- Unknown syntax, malformed payloads, invalid combinations, and trailing input
  are structured hard errors.
- Exact specification modes reject features introduced later but continue to
  accept standardized historical syntax, including syntax later deprecated or
  removed.
- Specification aliases are recognized at the grammar boundary and lowered to
  one semantic AST shape.
- The complete Rust parser forbids unsafe code and builds independently of the
  optional C ABI.

## Rust quick start

```toml
[dependencies]
roup = "0.8"
```

```rust
use roup::api::OpenMpConfig;
use roup::version::{CStandard, HostLanguageProfile, SourceForm};

let parser = OpenMpConfig::new(
    HostLanguageProfile::C(CStandard::C23),
    SourceForm::Pragma,
)?
.parser();

let parsed = parser.parse("#pragma omp parallel private(value)")?;
assert_eq!(parsed.directive().kind().as_str(), "parallel");
assert_eq!(parsed.directive().clauses().len(), 1);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Build only the safe Rust parser with `cargo build -p roup`.

## Optional C ABI and adapters

`crates/roup-capi` provides a separate opaque-handle ABI with one audited
foreign-memory boundary. Its checked-in header is
[`crates/roup-capi/include/roup.h`](crates/roup-capi/include/roup.h). The ABI
exposes typed fields and child nodes; it deliberately has no generic
whole-payload string operation.

```bash
cargo build --release -p roup-capi
```

[`compat/ompparser`](compat/ompparser) and
[`compat/accparser`](compat/accparser) build drop-in C++ compatibility libraries
against that ABI. Their upstream repositories are pinned git submodules, and
the compatibility criterion is their complete test suites registered unchanged:
currently 1,534 ompparser tests and 918 accparser tests, in addition to five
repository-owned contract/audit tests.

## Validation

The repository gate is fail-fast and never skips a missing prerequisite:

```bash
git submodule update --init --recursive
./test.sh
```

It checks formatting, safety invariants, the standalone publishable Rust
package, Clippy, Rust tests and documentation, the C ABI/header,
C/C++/Fortran examples, and every test in both pinned upstream compatibility
suites without fixture rewriting, filtering, disabled tests, or allowed
failures.

## License

BSD-3-Clause. See [LICENSE](LICENSE).
