# OpenMP 6.0 Support Matrix

This document catalogues the OpenMP 6.0 surface area for C and C++ and records what the ROUP parser currently understands.  The
lists below are derived from the [OpenMP Application Programming Interface Version 6.0 specification](https://www.openmp.org/wp-content/uploads/OpenMP-API-Specification-6-0.pdf).  The focus is on directive
keywords, the standard combined forms, and the clauses that may appear on those directives.  For the canonical directive, clause, and modifier catalogue see the [OpenMP 6.0 directive reference](../docs/book/src/openmp60-directives-clauses.md) and the [directive–clause component index](../docs/book/src/openmp60-directive-clause-components.md).

**Legend**: ✅ Supported in the parser & tests · ❌ Not yet implemented in the parser

All directive and clause keywords from the OpenMP 6.0 specification are registered with the parser and covered by automated tests.
The OpenMP parser exposes 127 directive spellings (the 64 canonical directives plus all OpenMP-standard combined forms and Fortran aliases) and registers all 125 clause keywords.  The `tests/openmp_support_matrix.rs` integration test iterates over these registries to ensure every keyword parses successfully.

## Directive Support (C/C++)

### Core execution and worksharing directives
| Directive | Status | Notes |
| --- | --- | --- |
| `allocate` | ✅ | Registered via the OpenMP directive registry. |
| `allocators` | ✅ | Registered via the OpenMP directive registry. |
| `assume` | ✅ | Registered via the OpenMP directive registry. |
| `assumes` | ✅ | Registered via the OpenMP directive registry. |
| `atomic` | ✅ | Registered via the OpenMP directive registry. |
| `atomic read` | ✅ | Registered via the OpenMP directive registry. |
| `atomic write` | ✅ | Registered via the OpenMP directive registry. |
| `atomic update` | ✅ | Registered via the OpenMP directive registry. |
| `atomic capture` | ✅ | Registered via the OpenMP directive registry. |
| `atomic compare capture` | ✅ | Registered via the OpenMP directive registry. |
| `barrier` | ✅ | Registered via the OpenMP directive registry. |
| `begin assumes` | ✅ | Registered via the OpenMP directive registry. |
| `cancel` | ✅ | Registered via the OpenMP directive registry. |
| `cancellation point` | ✅ | Registered via the OpenMP directive registry. |
| `critical` | ✅ | Registered via the OpenMP directive registry. |
| `declare induction` | ✅ | Registered via the OpenMP directive registry. |
| `depobj` | ✅ | Registered via the OpenMP directive registry. |
| `dispatch` | ✅ | Registered via the OpenMP directive registry. |
| `distribute` | ✅ | Registered via the OpenMP directive registry. |
| `distribute parallel for` | ✅ | Registered via the OpenMP directive registry. |
| `distribute parallel for simd` | ✅ | Registered via the OpenMP directive registry. |
| `distribute parallel loop` | ✅ | Registered via the OpenMP directive registry. |
| `distribute parallel loop simd` | ✅ | Registered via the OpenMP directive registry. |
| `distribute simd` | ✅ | Registered via the OpenMP directive registry. |
| `error` | ✅ | Registered via the OpenMP directive registry. |
| `flush` | ✅ | Registered via the OpenMP directive registry. |
| `fuse` | ✅ | Registered via the OpenMP directive registry. |
| `for` | ✅ | Recognized via the OpenMP directive registry. |
| `for simd` | ✅ | Recognized via the OpenMP directive registry. |
| `groupprivate` | ✅ | Registered via the OpenMP directive registry. |
| `interop` | ✅ | Registered via the OpenMP directive registry. |
| `interchange` | ✅ | Registered via the OpenMP directive registry. |
| `loop` | ✅ | Registered via the OpenMP directive registry. |
| `masked` | ✅ | Registered via the OpenMP directive registry. |
| `masked taskloop` | ✅ | Registered via the OpenMP directive registry. |
| `masked taskloop simd` | ✅ | Registered via the OpenMP directive registry. |
| `master` | ✅ | Registered via the OpenMP directive registry. |
| `metadirective` | ✅ | Registered via the OpenMP directive registry. |
| `nothing` | ✅ | Registered via the OpenMP directive registry. |
| `ordered` | ✅ | Registered via the OpenMP directive registry. |
| `parallel` | ✅ | Recognized via the OpenMP directive registry. |
| `parallel for` | ✅ | Recognized via the OpenMP directive registry. |
| `parallel for simd` | ✅ | Recognized via the OpenMP directive registry. |
| `parallel loop` | ✅ | Recognized via the OpenMP directive registry. |
| `parallel loop simd` | ✅ | Registered via the OpenMP directive registry. |
| `parallel masked` | ✅ | Recognized via the OpenMP directive registry. |
| `parallel masked taskloop` | ✅ | Recognized via the OpenMP directive registry. |
| `parallel masked taskloop simd` | ✅ | Recognized via the OpenMP directive registry. |
| `parallel master` | ✅ | Registered via the OpenMP directive registry. |
| `parallel master taskloop` | ✅ | Recognized via the OpenMP directive registry. |
| `parallel master taskloop simd` | ✅ | Recognized via the OpenMP directive registry. |
| `parallel sections` | ✅ | Registered via the OpenMP directive registry. |
| `requires` | ✅ | Registered via the OpenMP directive registry. |
| `reverse` | ✅ | Registered via the OpenMP directive registry. |
| `scan` | ✅ | Registered via the OpenMP directive registry. |
| `scope` | ✅ | Registered via the OpenMP directive registry. |
| `section` | ✅ | Registered via the OpenMP directive registry. |
| `sections` | ✅ | Registered via the OpenMP directive registry. |
| `simd` | ✅ | Registered via the OpenMP directive registry. |
| `single` | ✅ | Registered via the OpenMP directive registry. |
| `split` | ✅ | Registered via the OpenMP directive registry. |
| `stripe` | ✅ | Registered via the OpenMP directive registry. |
| `target` | ✅ | Recognized via the OpenMP directive registry. |
| `task` | ✅ | Recognized via the OpenMP directive registry. |
| `task iteration` | ✅ | Registered via the OpenMP directive registry. |
| `taskgroup` | ✅ | Registered via the OpenMP directive registry. |
| `taskgraph` | ✅ | Registered via the OpenMP directive registry. |
| `taskloop` | ✅ | Recognized via the OpenMP directive registry. |
| `taskloop simd` | ✅ | Recognized via the OpenMP directive registry. |
| `taskwait` | ✅ | Registered via the OpenMP directive registry. |
| `taskyield` | ✅ | Registered via the OpenMP directive registry. |
| `teams` | ✅ | Recognized via the OpenMP directive registry. |
| `teams distribute` | ✅ | Recognized via the OpenMP directive registry. |
| `teams distribute parallel for` | ✅ | Recognized via the OpenMP directive registry. |
| `teams distribute parallel for simd` | ✅ | Recognized via the OpenMP directive registry. |
| `teams distribute parallel loop` | ✅ | Recognized via the OpenMP directive registry. |
| `teams distribute parallel loop simd` | ✅ | Registered via the OpenMP directive registry. |
| `teams distribute simd` | ✅ | Recognized via the OpenMP directive registry. |
| `teams loop` | ✅ | Recognized via the OpenMP directive registry. |
| `teams loop simd` | ✅ | Recognized via the OpenMP directive registry. |
| `threadprivate` | ✅ | Registered via the OpenMP directive registry. |
| `tile` | ✅ | Registered via the OpenMP directive registry. |
| `unroll` | ✅ | Registered via the OpenMP directive registry. |
| `workdistribute` | ✅ | Registered via the OpenMP directive registry. |
| `workshare` | ✅ | Registered via the OpenMP directive registry. |

### Target and combined offloading directives
| Directive | Status | Notes |
| --- | --- | --- |
| `begin declare target` | ✅ | Registered via the OpenMP directive registry. |
| `begin declare variant` | ✅ | Registered via the OpenMP directive registry. |
| `begin metadirective` | ✅ | Registered via the OpenMP directive registry. |
| `declare mapper` | ✅ | Registered via the OpenMP directive registry. |
| `declare reduction` | ✅ | Registered via the OpenMP directive registry. |
| `declare simd` | ✅ | Registered via the OpenMP directive registry. |
| `declare target` | ✅ | Registered via the OpenMP directive registry. |
| `declare variant` | ✅ | Registered via the OpenMP directive registry. |
| `end declare target` | ✅ | Registered via the OpenMP directive registry. |
| `target` | ✅ | Recognized via the OpenMP directive registry. |
| `target data` | ✅ | Registered via the OpenMP directive registry. |
| `target enter data` | ✅ | Registered via the OpenMP directive registry. |
| `target exit data` | ✅ | Registered via the OpenMP directive registry. |
| `target loop` | ✅ | Recognized via the OpenMP directive registry. |
| `target loop simd` | ✅ | Registered via the OpenMP directive registry. |
| `target parallel` | ✅ | Recognized via the OpenMP directive registry. |
| `target parallel for` | ✅ | Recognized via the OpenMP directive registry. |
| `target parallel for simd` | ✅ | Recognized via the OpenMP directive registry. |
| `target parallel loop` | ✅ | Recognized via the OpenMP directive registry. |
| `target parallel loop simd` | ✅ | Registered via the OpenMP directive registry. |
| `target simd` | ✅ | Recognized via the OpenMP directive registry. |
| `target teams` | ✅ | Recognized via the OpenMP directive registry. |
| `target teams distribute` | ✅ | Recognized via the OpenMP directive registry. |
| `target teams distribute parallel for` | ✅ | Recognized via the OpenMP directive registry. |
| `target teams distribute parallel for simd` | ✅ | Recognized via the OpenMP directive registry. |
| `target teams distribute parallel loop` | ✅ | Recognized via the OpenMP directive registry. |
| `target teams distribute parallel loop simd` | ✅ | Registered via the OpenMP directive registry. |
| `target teams distribute simd` | ✅ | Registered via the OpenMP directive registry. |
| `target teams loop` | ✅ | Recognized via the OpenMP directive registry. |
| `target teams loop simd` | ✅ | Recognized via the OpenMP directive registry. |
| `target update` | ✅ | Registered via the OpenMP directive registry. |

## Clause Support (C/C++)

All 125 OpenMP 6.0 clause keywords are registered with the parser.  The clause registry now includes the new 6.0 additions such as `absent`, `adjust_args`, `align`, `append_args`, `device_safesync`, `graph_id`, `graph_reset`, `has_device_addr`, `looprange`, `memscope`, `no_openmp_constructs`, `reverse_offload`, `self_maps`, `threadset`, `transparent`, `uniform`, and many others.  The support matrix test exercises each clause in both bare and parenthesized form (when applicable) to guard against regressions.  For the clause-by-clause breakdown, including directive applicability and argument syntax, consult the [OpenMP 6.0 directive reference](../docs/book/src/openmp60-directives-clauses.md).

## Other specification items

| Construct | Status | Notes |
| --- | --- | --- |
| Memory-order modifiers (`acq_rel`, `acquire`, `release`, `relaxed`, `seq_cst`) | ✅ | See clause coverage above; all standard memory-order modifiers parse as bare clauses. |
| Directive modifiers such as `begin/end declare target` pairs | ✅ | All begin/end directive modifiers are registered as individual directives. |
| Trait selectors for `declare variant` (e.g., `match`, `when`) | ✅ | Clause keywords recorded above and available through the clause registry. |

