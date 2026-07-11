# Getting started

## Rust-only parser

Build and test the safe parser without the C ABI:

```bash
cargo build -p roup
cargo test -p roup
```

A parser configuration always names the host-language standard and source
form. `VersionPolicy::Any` accepts the union of standardized historical syntax;
an exact configuration enforces an introduction ceiling.

```rust,ignore
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

## Optional C ABI

Build the ABI explicitly:

```bash
cargo build -p roup-capi --release
```

Include `crates/roup-capi/include/roup.h` and link the generated
`libroup_capi` shared or static library. The ABI copies UTF-8 input, returns
opaque handles, and requires callers to release every successful parser,
directive, and error handle.

## Complete repository validation

`./test.sh` is fail-fast. It requires initialized pinned submodules and all
native toolchains, then checks formatting, lints, Rust tests, documentation,
the C ABI, both compatibility adapters, every test in their pinned upstream
suites, and all language examples. The current compatibility totals are
1,537/1,537 for ompparser and 920/920 for accparser, including five local
contract/audit tests.
