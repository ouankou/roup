# OpenACC coverage

ROUP implements the full keyword surface mandated by the OpenACC 3.4
specification (November 2023). Directive and clause registries were verified
against the official PDF during development, and the automated tests in
`tests/openacc_keyword_coverage.rs` assert that every keyword continues to parse.

## Directive support matrix

| Directive keyword(s) | Status | Notes |
| --- | --- | --- |
| `atomic` | ✅ Supported | Accepts `update`, `read`, `write`, and `capture` clauses. |
| `cache` | ✅ Supported | Custom parser recognises parenthesised variable lists. |
| `data` | ✅ Supported | Allows full data clause set including legacy synonyms. |
| `declare` | ✅ Supported | Works with `create`, `link`, and other declarative clauses. |
| `end` | ✅ Supported | Handles composite names (e.g. `end parallel loop`). |
| `enter data`, `enter_data` | ✅ Supported | Space and underscore spellings normalised. |
| `exit data`, `exit_data` | ✅ Supported | Includes `finalize` clause handling. |
| `host_data`, `host data` | ✅ Supported | Supports `use_device`, `if`, and `if_present`. |
| `init` | ✅ Supported | Works with optional `device_num` modifiers. |
| `kernels`, `kernels loop` | ✅ Supported | Combined form parsed as single directive. |
| `loop` | ✅ Supported | Loop clauses (`gang`, `worker`, `vector`, etc.) registered. |
| `parallel`, `parallel loop` | ✅ Supported | Case-insensitive in Fortran modes. |
| `routine` | ✅ Supported | Accepts binding clauses and execution mode modifiers. |
| `serial`, `serial loop` | ✅ Supported | Fully mirrored from specification text. |
| `set` | ✅ Supported | Exposes `default_async`/`device_type` combinations. |
| `shutdown` | ✅ Supported | Matches spec semantics for queue teardown. |
| `update` | ✅ Supported | Recognises `self`, `device`, `host`, and `if_present`. |
| `wait` | ✅ Supported | Supports both bare and parenthesised forms. |

## Clause coverage

Clauses are grouped by usage for readability. All entries are parsed case-
insensitively in Fortran modes.

### Compute clauses

| Clause keyword | Status | Notes |
| --- | --- | --- |
| `async`, `wait` | ✅ Supported | Flexible rule allows optional queues/device modifiers. |
| `num_gangs`, `num_workers`, `vector_length` | ✅ Supported | Parenthesised integer expressions retained verbatim. |
| `gang`, `worker`, `vector`, `seq` | ✅ Supported | Accept bare form and optional parameter syntax (spec-compliant). |
| `independent`, `auto`, `collapse`, `tile` | ✅ Supported | Tile lists preserved for round-tripping. |
| `device_type`, `bind` | ✅ Supported | Accept either bare tokens or parenthesised device lists. |
| `if`, `default`, `default_async`, `firstprivate`, `private`, `nohost`, `reduction` | ✅ Supported | Default clause honours `none`/`present` options. |
| `read`, `write`, `capture`, `update` (atomic) | ✅ Supported | Mapped to canonical OpenACC clause kinds. |
| `self`, `if_present` | ✅ Supported | Used on `update`/`wait` directives per spec. |

### Data movement clauses

| Clause keyword | Status | Notes |
| --- | --- | --- |
| `copy`, `copyin`, `copyout`, `create`, `delete` | ✅ Supported | Canonical mapping for C API and compat layers. |
| `present`, `no_create`, `host`, `device`, `deviceptr`, `device_num`, `device_resident` | ✅ Supported | Expressions retained exactly as provided. |
| `use_device`, `attach`, `detach` | ✅ Supported | Works on `host_data`, `enter data`, and related constructs. |
| `present_or_copy`, `present_or_copyin`, `present_or_copyout`, `present_or_create` | ✅ Supported | Map to canonical data clause kinds for the C API. |
| `pcopy`, `pcopyin`, `pcopyout`, `pcreate` | ✅ Supported | Legacy synonyms accepted and normalised to canonical semantics. |

### Miscellaneous

| Clause keyword | Status | Notes |
| --- | --- | --- |
| `link` | ✅ Supported | Available on `declare` directives. |
| `finalize` | ✅ Supported | Recognised on `exit data`. |

## Testing guarantees

- `tests/openacc_keyword_coverage.rs` exercises every directive and clause.
- `tests/openacc_roundtrip.rs` covers round-tripping of representative clauses
  including legacy `present_or_*` spellings.
- `compat/accparser/tests/comprehensive_test.cpp` verifies that the accparser
  shim maps the legacy aliases (`pcopy`, etc.) onto the canonical
  `OpenACCClauseKind` values expected by downstream consumers.

Together with the generated `roup_constants.h` mapping, these tests provide a
machine-checked proof that the OpenACC 3.4 keyword set is fully supported.
