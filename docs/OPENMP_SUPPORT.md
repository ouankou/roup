# OpenMP 6.0 Support Matrix

This document catalogues the OpenMP 6.0 surface area for C and C++ and records what the ROUP parser currently understands.  The
lists below are derived from the OpenMP Application Programming Interface Version 6.0 specification.  The focus is on directive
keywords, the standard combined forms, and the clauses that may appear on those directives.

**Legend**: ✅ Supported in the parser & tests · ❌ Not yet implemented in the parser

Unsupported clause keywords are rejected by the OpenMP clause registry to avoid
silently accepting directives outside the documented coverage.

## Directive Support (C/C++)

### Core execution and worksharing directives
| Directive | Status | Notes |
| --- | --- | --- |
| `assume` | ❌ | Not yet implemented in the directive registry. |
| `atomic` | ❌ | Not yet implemented in the directive registry. |
| `atomic read` | ❌ | Not yet implemented in the directive registry. |
| `atomic write` | ❌ | Not yet implemented in the directive registry. |
| `atomic update` | ❌ | Not yet implemented in the directive registry. |
| `atomic capture` | ❌ | Not yet implemented in the directive registry. |
| `atomic compare capture` | ❌ | Not yet implemented in the directive registry. |
| `barrier` | ❌ | Not yet implemented in the directive registry. |
| `cancel` | ❌ | Not yet implemented in the directive registry. |
| `cancellation point` | ❌ | Not yet implemented in the directive registry. |
| `critical` | ❌ | Not yet implemented in the directive registry. |
| `depobj` | ❌ | Not yet implemented in the directive registry. |
| `dispatch` | ❌ | Not yet implemented in the directive registry. |
| `distribute` | ❌ | Not yet implemented in the directive registry. |
| `distribute parallel for` | ❌ | Not yet implemented in the directive registry. |
| `distribute parallel for simd` | ❌ | Not yet implemented in the directive registry. |
| `distribute parallel loop` | ❌ | Not yet implemented in the directive registry. |
| `distribute parallel loop simd` | ❌ | Not yet implemented in the directive registry. |
| `distribute simd` | ❌ | Not yet implemented in the directive registry. |
| `error` | ❌ | Not yet implemented in the directive registry. |
| `flush` | ❌ | Not yet implemented in the directive registry. |
| `for` | ✅ | Recognized via the OpenMP directive registry. |
| `for simd` | ✅ | Recognized via the OpenMP directive registry. |
| `interop` | ❌ | Not yet implemented in the directive registry. |
| `loop` | ❌ | Not yet implemented in the directive registry. |
| `masked` | ❌ | Not yet implemented in the directive registry. |
| `masked taskloop` | ❌ | Not yet implemented in the directive registry. |
| `masked taskloop simd` | ❌ | Not yet implemented in the directive registry. |
| `master` | ❌ | Not yet implemented in the directive registry. |
| `metadirective` | ❌ | Not yet implemented in the directive registry. |
| `nothing` | ❌ | Not yet implemented in the directive registry. |
| `ordered` | ❌ | Not yet implemented in the directive registry. |
| `parallel` | ✅ | Recognized via the OpenMP directive registry. |
| `parallel for` | ✅ | Recognized via the OpenMP directive registry. |
| `parallel for simd` | ✅ | Recognized via the OpenMP directive registry. |
| `parallel loop` | ✅ | Recognized via the OpenMP directive registry. |
| `parallel loop simd` | ❌ | Not yet implemented in the directive registry. |
| `parallel masked` | ✅ | Recognized via the OpenMP directive registry. |
| `parallel masked taskloop` | ✅ | Recognized via the OpenMP directive registry. |
| `parallel masked taskloop simd` | ✅ | Recognized via the OpenMP directive registry. |
| `parallel master` | ❌ | Not yet implemented in the directive registry. |
| `parallel master taskloop` | ✅ | Recognized via the OpenMP directive registry. |
| `parallel master taskloop simd` | ✅ | Recognized via the OpenMP directive registry. |
| `parallel sections` | ❌ | Not yet implemented in the directive registry. |
| `requires` | ❌ | Not yet implemented in the directive registry. |
| `scope` | ❌ | Not yet implemented in the directive registry. |
| `sections` | ❌ | Not yet implemented in the directive registry. |
| `simd` | ❌ | Not yet implemented in the directive registry. |
| `single` | ❌ | Not yet implemented in the directive registry. |
| `target` | ✅ | Recognized via the OpenMP directive registry. |
| `task` | ✅ | Recognized via the OpenMP directive registry. |
| `taskgroup` | ❌ | Not yet implemented in the directive registry. |
| `taskgraph` | ❌ | Not yet implemented in the directive registry. |
| `taskloop` | ✅ | Recognized via the OpenMP directive registry. |
| `taskloop simd` | ✅ | Recognized via the OpenMP directive registry. |
| `taskwait` | ❌ | Not yet implemented in the directive registry. |
| `taskyield` | ❌ | Not yet implemented in the directive registry. |
| `teams` | ✅ | Recognized via the OpenMP directive registry. |
| `teams distribute` | ✅ | Recognized via the OpenMP directive registry. |
| `teams distribute parallel for` | ✅ | Recognized via the OpenMP directive registry. |
| `teams distribute parallel for simd` | ✅ | Recognized via the OpenMP directive registry. |
| `teams distribute parallel loop` | ✅ | Recognized via the OpenMP directive registry. |
| `teams distribute parallel loop simd` | ❌ | Not yet implemented in the directive registry. |
| `teams distribute simd` | ✅ | Recognized via the OpenMP directive registry. |
| `teams loop` | ✅ | Recognized via the OpenMP directive registry. |
| `teams loop simd` | ✅ | Recognized via the OpenMP directive registry. |
| `threadprivate` | ❌ | Not yet implemented in the directive registry. |

### Target and combined offloading directives
| Directive | Status | Notes |
| --- | --- | --- |
| `begin declare target` | ❌ | Not yet implemented in the directive registry. |
| `declare mapper` | ❌ | Not yet implemented in the directive registry. |
| `declare reduction` | ❌ | Not yet implemented in the directive registry. |
| `declare simd` | ❌ | Not yet implemented in the directive registry. |
| `declare target` | ❌ | Not yet implemented in the directive registry. |
| `declare variant` | ❌ | Not yet implemented in the directive registry. |
| `end declare target` | ❌ | Not yet implemented in the directive registry. |
| `target` | ✅ | Recognized via the OpenMP directive registry. |
| `target data` | ❌ | Not yet implemented in the directive registry. |
| `target enter data` | ❌ | Not yet implemented in the directive registry. |
| `target exit data` | ❌ | Not yet implemented in the directive registry. |
| `target loop` | ✅ | Recognized via the OpenMP directive registry. |
| `target loop simd` | ❌ | Not yet implemented in the directive registry. |
| `target parallel` | ✅ | Recognized via the OpenMP directive registry. |
| `target parallel for` | ✅ | Recognized via the OpenMP directive registry. |
| `target parallel for simd` | ✅ | Recognized via the OpenMP directive registry. |
| `target parallel loop` | ✅ | Recognized via the OpenMP directive registry. |
| `target parallel loop simd` | ❌ | Not yet implemented in the directive registry. |
| `target simd` | ✅ | Recognized via the OpenMP directive registry. |
| `target teams` | ✅ | Recognized via the OpenMP directive registry. |
| `target teams distribute` | ✅ | Recognized via the OpenMP directive registry. |
| `target teams distribute parallel for` | ✅ | Recognized via the OpenMP directive registry. |
| `target teams distribute parallel for simd` | ✅ | Recognized via the OpenMP directive registry. |
| `target teams distribute parallel loop` | ✅ | Recognized via the OpenMP directive registry. |
| `target teams distribute parallel loop simd` | ❌ | Not yet implemented in the directive registry. |
| `target teams distribute simd` | ❌ | Not yet implemented in the directive registry. |
| `target teams loop` | ✅ | Recognized via the OpenMP directive registry. |
| `target teams loop simd` | ✅ | Recognized via the OpenMP directive registry. |
| `target update` | ❌ | Not yet implemented in the directive registry. |

## Clause Support (C/C++)

The table below enumerates every OpenMP 6.0 clause keyword for C and C++.  Clauses are marked supported when the parser accepts
them through the `OpenMpClause` enum.

| Clause | Status | Notes |
| --- | --- | --- |
| `acq_rel` | ❌ | Memory-order clause for `atomic`; not yet parsed. |
| `acquire` | ❌ | Memory-order clause for `atomic`; not yet parsed. |
| `affinity` | ✅ | Registered with the clause registry. |
| `aligned` | ✅ | Registered with the clause registry. |
| `allocate` | ✅ | Registered with the clause registry. |
| `allocator` | ❌ | Not yet parsed. |
| `atomic_default_mem_order` | ✅ | Registered with the clause registry. |
| `bind` | ✅ | Registered with the clause registry. |
| `capture` | ❌ | Not yet parsed. |
| `collapse` | ✅ | Registered with the clause registry. |
| `compare` | ❌ | Not yet parsed. |
| `copyin` | ✅ | Registered with the clause registry. |
| `copyprivate` | ✅ | Registered with the clause registry. |
| `default` | ✅ | Registered with the clause registry. |
| `defaultmap` | ✅ | Registered with the clause registry. |
| `depend` | ✅ | Registered with the clause registry. |
| `destroy` | ❌ | Not yet parsed. |
| `detach` | ✅ | Registered with the clause registry. |
| `device` | ✅ | Registered with the clause registry. |
| `device_resident` | ❌ | Not yet parsed. |
| `device_type` | ✅ | Registered with the clause registry. |
| `dist_schedule` | ✅ | Registered with the clause registry. |
| `doacross` | ❌ | Not yet parsed. |
| `dynamic_allocators` | ✅ | Registered with the clause registry. |
| `exclusive` | ✅ | Registered with the clause registry. |
| `fail` | ❌ | Not yet parsed. |
| `final` | ✅ | Registered with the clause registry. |
| `filter` | ❌ | Not yet parsed. |
| `firstprivate` | ✅ | Registered with the clause registry. |
| `from` | ❌ | Not yet parsed. |
| `grainsize` | ✅ | Registered with the clause registry. |
| `hint` | ✅ | Registered with the clause registry. |
| `holds` | ❌ | Not yet parsed. |
| `if` | ✅ | Registered with the clause registry. |
| `in_reduction` | ✅ | Registered with the clause registry. |
| `inbranch` | ✅ | Registered with the clause registry. |
| `inclusive` | ✅ | Registered with the clause registry. |
| `init` | ❌ | Not yet parsed. |
| `interop` | ❌ | Not yet parsed. |
| `is_device_ptr` | ✅ | Registered with the clause registry. |
| `label` | ✅ | Registered with the clause registry. |
| `lastprivate` | ✅ | Registered with the clause registry. |
| `linear` | ✅ | Registered with the clause registry. |
| `link` | ❌ | Not yet parsed. |
| `map` | ✅ | Registered with the clause registry. |
| `match` | ❌ | Not yet parsed. |
| `message` | ❌ | Not yet parsed. |
| `mergeable` | ✅ | Registered with the clause registry. |
| `nontemporal` | ✅ | Registered with the clause registry. |
| `no_openmp` | ❌ | Not yet parsed. |
| `no_openmp_routines` | ❌ | Not yet parsed. |
| `no_parallelism` | ❌ | Not yet parsed. |
| `nogroup` | ✅ | Registered with the clause registry. |
| `novariants` | ❌ | Not yet parsed. |
| `nowait` | ✅ | Registered with the clause registry. |
| `num_tasks` | ✅ | Registered with the clause registry. |
| `num_teams` | ✅ | Registered with the clause registry. |
| `num_threads` | ✅ | Registered with the clause registry. |
| `order` | ✅ | Registered with the clause registry. |
| `ordered` | ✅ | Registered with the clause registry. |
| `partial` | ❌ | Not yet parsed. |
| `priority` | ✅ | Registered with the clause registry. |
| `private` | ✅ | Registered with the clause registry. |
| `proc_bind` | ✅ | Registered with the clause registry. |
| `public` | ❌ | Not yet parsed. |
| `reduction` | ✅ | Registered with the clause registry. |
| `release` | ❌ | Memory-order clause for `atomic`; not yet parsed. |
| `relaxed` | ❌ | Memory-order clause for `atomic`; not yet parsed. |
| `reverse` | ❌ | Not yet parsed. |
| `reproducible` | ✅ | Registered with the clause registry. |
| `safelen` | ✅ | Registered with the clause registry. |
| `schedule` | ✅ | Registered with the clause registry. |
| `seq_cst` | ❌ | Memory-order clause for `atomic`; not yet parsed. |
| `shared` | ✅ | Registered with the clause registry. |
| `simdlen` | ✅ | Registered with the clause registry. |
| `sizes` | ❌ | Not yet parsed. |
| `task_reduction` | ❌ | Not yet parsed. |
| `thread_limit` | ✅ | Registered with the clause registry. |
| `tile` | ✅ | Registered with the clause registry. |
| `to` | ❌ | Not yet parsed. |
| `unified_address` | ❌ | Not yet parsed. |
| `unified_shared_memory` | ❌ | Not yet parsed. |
| `unknown` | ❌ | Not yet parsed. |
| `unroll` | ✅ | Registered with the clause registry. |
| `untied` | ✅ | Registered with the clause registry. |
| `update` | ❌ | Not yet parsed. |
| `use_device_addr` | ✅ | Registered with the clause registry. |
| `use_device_ptr` | ✅ | Registered with the clause registry. |
| `uses_allocators` | ✅ | Registered with the clause registry. |
| `weak` | ❌ | Not yet parsed. |
| `when` | ❌ | Not yet parsed. |

## Other specification items

| Construct | Status | Notes |
| --- | --- | --- |
| Memory-order modifiers (`acq_rel`, `acquire`, `release`, `relaxed`, `seq_cst`) | ❌ | Listed above; parser does not yet model them. |
| Directive modifiers such as `begin/end declare target` pairs | ❌ | Not yet represented in the registry layer. |
| Trait selectors for `declare variant` (e.g., `match`, `when`) | ❌ | Clause keywords recorded above but not parsed. |

