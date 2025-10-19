# OpenACC coverage

ROUP implements the complete OpenACC 3.4 directive and clause surface. The
canonical keyword catalogue lives in the mdBook chapter
[`docs/book/src/openacc/openacc-3-4-directives-clauses.md`](book/src/openacc/openacc-3-4-directives-clauses.md),
which cross-references every entry against the [OpenACC Application Programming
Interface Version 3.4](https://www.openacc.org/sites/default/files/inline-files/OpenACC-3.4.pdf)
specification.

## Directive support matrix

| Category            | Directives                                                                 |
| ------------------- | -------------------------------------------------------------------------- |
| Compute             | `parallel`, `serial`, `kernels`                                            |
| Loop                | `loop`, `parallel loop`, `serial loop`, `kernels loop`                     |
| Data                | `data`, `enter data`, `exit data`, `host_data` (space and underscore forms) |
| Synchronisation     | `atomic`, `cache`, `wait`                                                  |
| Declaration         | `declare`, `routine`                                                       |
| Runtime             | `init`, `shutdown`, `set`, `update`                                        |
| Terminators & other | `end <directive>` with full multi-word names                               |

All directive spellings (including synonyms such as `host data`, `host_data`,
`enter data`, and `enter_data`) are registered in the parser and mirrored into
the C API and the accparser compatibility shim. The new
`tests/openacc_keyword_coverage.rs` integration suite exercises every directive
string and validates round-tripping through `Directive::to_pragma_string`.【F:tests/openacc_keyword_coverage.rs†L6-L71】【F:tests/openacc_keyword_coverage.rs†L101-L164】

## Clause coverage

| Category                | Clauses and aliases                                                                                                       |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| Parallelism & control   | `async`, `wait`, `num_gangs`, `num_workers`, `vector_length`, `gang`, `worker`, `vector`, `seq`, `independent`, `auto`    |
| Conditionals            | `if`, `if_present`, `self`, `default`, `default_async`                                                                    |
| Data movement           | `copy`, `copyin`, `copyout`, `create`, `delete`, `present`, `no_create`, `device`, `deviceptr`, `device_resident`, `host` |
| Pointer management      | `attach`, `detach`, `link`, `use_device`                                                                                  |
| Sharing & reductions    | `private`, `firstprivate`, `reduction`                                                                                    |
| Loop transforms         | `collapse`, `tile`                                                                                                        |
| Device specialisation   | `device_type`, alias `dtype`                                                                                              |
| Atomic modifiers        | `read`, `write`, `capture`, `update`                                                                                      |
| Synonym aliases         | `pcopy`, `present_or_copy`, `pcopyin`, `present_or_copyin`, `pcopyout`, `present_or_copyout`, `pcreate`, `present_or_create` |

Clause registration keeps the original surface spelling. Alias spellings share
the same numeric identifiers in the C API so existing code can treat them as
synonyms while retaining the original source text. The parser-level coverage
suite validates every clause and alias (including atomic update as a bare
clause), and the C API tests assert that aliases collapse to the same integer
kind IDs as their canonical forms.【F:tests/openacc_keyword_coverage.rs†L101-L164】【F:tests/openacc_c_api.rs†L9-L76】

## Compatibility layer parity

The accparser bridge exercises the new coverage, ensuring the C++ drop-in
replacement emits the same directive kinds and serialises aliases without
normalisation. The extended `compat/accparser/tests/comprehensive_test.cpp`
suite now exercises mixed alias usage, host-data spacing variants, dtype
shorthands, and atomic update round-tripping through `OpenACCIR::toString`.【F:compat/accparser/tests/comprehensive_test.cpp†L1-L320】

## Regression protection

The end-to-end round-trip and C API tests sit alongside the existing OpenACC
round-trip checks, so any future change to the keyword registry or numeric
mappings will fail CI before shipping. These tests complement the existing
round-trip scenarios in `tests/openacc_roundtrip.rs` and cover cache
directives, alias preservation, dtype handling, and atomic update parsing.【F:tests/openacc_roundtrip.rs†L1-L123】
