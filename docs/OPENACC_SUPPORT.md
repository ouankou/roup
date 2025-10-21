# OpenACC coverage

ROUP implements the full OpenACC 3.4 directive and clause surface. The mdBook chapter [`book/src/openacc/openacc-3-4-directives-clauses.md`](book/src/openacc/openacc-3-4-directives-clauses.md) catalogues every keyword with references back to the official [OpenACC 3.4 specification](https://www.openacc.org/sites/default/files/inline-files/OpenACC-3.4.pdf).

## Directive support matrix

| Category            | Directives                                                                   |
| ------------------- | ---------------------------------------------------------------------------- |
| Compute             | `parallel`, `serial`, `kernels`                                              |
| Loop                | `loop`, `parallel loop`, `serial loop`, `kernels loop`                       |
| Data                | `data`, `enter data`, `exit data`, `host data` / `host_data`                 |
| Synchronisation     | `atomic`, `cache`, `wait`                                                    |
| Declaration         | `declare`, `routine`                                                         |
| Runtime             | `init`, `shutdown`, `set`, `update`                                          |
| Terminators & other | `end <directive>` with full multi-word spellings                             |

All directive spellings, including synonyms such as `host data`/`host_data` and `enter data`/`enter_data`, are registered in the parser and propagated through the Rust and C APIs. The integration suite in `tests/openacc_keyword_coverage.rs` exercises every directive string and validates round-tripping through `Directive::to_pragma_string`.【F:tests/openacc_keyword_coverage.rs†L6-L164】

## Clause coverage

| Category              | Clauses and aliases                                                                                                        |
| --------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| Parallelism & control | `async`, `wait`, `num_gangs`, `num_workers`, `vector_length`, `gang`, `worker`, `vector`, `seq`, `independent`, `auto`      |
| Conditionals          | `if`, `if_present`, `self`, `default`, `default_async`                                                                     |
| Data movement         | `copy`, `copyin`, `copyout`, `create`, `delete`, `present`, `no_create`, `device`, `deviceptr`, `device_resident`, `host`   |
| Pointer management    | `attach`, `detach`, `link`, `use_device`                                                                                   |
| Sharing & reductions  | `private`, `firstprivate`, `reduction`                                                                                     |
| Loop transforms       | `collapse`, `tile`                                                                                                         |
| Device specialisation | `device_type`, alias `dtype`                                                                                               |
| Atomic modifiers      | `read`, `write`, `capture`, `update`                                                                                       |
| Synonym aliases       | `pcopy`, `present_or_copy`, `pcopyin`, `present_or_copyin`, `pcopyout`, `present_or_copyout`, `pcreate`, `present_or_create` |

Alias spellings share numeric identifiers in the C API so existing code can treat them as synonyms while retaining the original source text. The parser-level coverage tests validate every clause and alias, and the C API tests assert that aliases collapse to the same integer IDs as their canonical forms.【F:tests/openacc_keyword_coverage.rs†L101-L164】【F:tests/openacc_c_api.rs†L9-L76】

## Compatibility layer parity

The accparser bridge (`compat/accparser/`) exercises the same keyword tables, ensuring the drop-in replacement emits identical directive kinds and preserves alias spellings. The extended `compat/accparser/tests/comprehensive_test.cpp` suite covers mixed alias usage, host-data spacing variants, dtype shorthands, and atomic update round-tripping through `OpenACCIR::toString`.【F:compat/accparser/tests/comprehensive_test.cpp†L1-L320】

## Regression protection

Keyword registration lives alongside round-trip tests in `tests/openacc_roundtrip.rs`, so any change to the registry or numeric mappings fails CI immediately. These tests cover cache directives, alias preservation, dtype handling, and atomic update parsing.【F:tests/openacc_roundtrip.rs†L1-L123】
