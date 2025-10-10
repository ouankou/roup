# OpenMP 6.0 Support Matrix

This document catalogues the OpenMP 6.0 surface area for C and C++ and records what the ROUP parser currently understands.  The
lists below are derived from the [OpenMP Application Programming Interface Version 6.0 specification](https://www.openmp.org/wp-content/uploads/OpenMP-API-Specification-6-0.pdf).  The focus is on directive
keywords, the standard combined forms, and the clauses that may appear on those directives.

**Legend**: ✅ Supported in the parser & tests · ❌ Not yet implemented in the parser

All directive and clause keywords from the OpenMP 6.0 specification are registered with the parser and covered by automated tests.
Unsupported clause keywords are rejected by the OpenMP clause registry to avoid
silently accepting directives outside the documented coverage.

## Directive Support (C/C++)

### Core execution and worksharing directives
| Directive | Status | Notes |
| --- | --- | --- |
| `assume` | ✅ | Registered via the OpenMP directive registry. |
| `atomic` | ✅ | Registered via the OpenMP directive registry. |
| `atomic read` | ✅ | Registered via the OpenMP directive registry. |
| `atomic write` | ✅ | Registered via the OpenMP directive registry. |
| `atomic update` | ✅ | Registered via the OpenMP directive registry. |
| `atomic capture` | ✅ | Registered via the OpenMP directive registry. |
| `atomic compare capture` | ✅ | Registered via the OpenMP directive registry. |
| `barrier` | ✅ | Registered via the OpenMP directive registry. |
| `cancel` | ✅ | Registered via the OpenMP directive registry. |
| `cancellation point` | ✅ | Registered via the OpenMP directive registry. |
| `critical` | ✅ | Registered via the OpenMP directive registry. |
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
| `for` | ✅ | Recognized via the OpenMP directive registry. |
| `for simd` | ✅ | Recognized via the OpenMP directive registry. |
| `interop` | ✅ | Registered via the OpenMP directive registry. |
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
| `scope` | ✅ | Registered via the OpenMP directive registry. |
| `sections` | ✅ | Registered via the OpenMP directive registry. |
| `simd` | ✅ | Registered via the OpenMP directive registry. |
| `single` | ✅ | Registered via the OpenMP directive registry. |
| `target` | ✅ | Recognized via the OpenMP directive registry. |
| `task` | ✅ | Recognized via the OpenMP directive registry. |
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

### Target and combined offloading directives
| Directive | Status | Notes |
| --- | --- | --- |
| `begin declare target` | ✅ | Registered via the OpenMP directive registry. |
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

The table below enumerates every OpenMP 6.0 clause keyword for C and C++.  Clauses are marked supported when the parser accepts
them through the `OpenMpClause` enum.

| Clause | Status | Notes |
| --- | --- | --- |
| `acq_rel` | ✅ | Memory-order clause for `atomic`; registered as a bare clause. |
| `acquire` | ✅ | Memory-order clause for `atomic`; registered as a bare clause. |
| `affinity` | ✅ | Registered with the clause registry. |
| `aligned` | ✅ | Registered with the clause registry. |
| `allocate` | ✅ | Registered with the clause registry. |
| `allocator` | ✅ | Registered with the clause registry. |
| `atomic_default_mem_order` | ✅ | Registered with the clause registry. |
| `bind` | ✅ | Registered with the clause registry. |
| `capture` | ✅ | Registered with the clause registry. |
| `collapse` | ✅ | Registered with the clause registry. |
| `compare` | ✅ | Registered with the clause registry. |
| `copyin` | ✅ | Registered with the clause registry. |
| `copyprivate` | ✅ | Registered with the clause registry. |
| `default` | ✅ | Registered with the clause registry. |
| `defaultmap` | ✅ | Registered with the clause registry. |
| `depend` | ✅ | Registered with the clause registry. |
| `destroy` | ✅ | Registered with the clause registry. |
| `detach` | ✅ | Registered with the clause registry. |
| `device` | ✅ | Registered with the clause registry. |
| `device_resident` | ✅ | Registered with the clause registry. |
| `device_type` | ✅ | Registered with the clause registry. |
| `dist_schedule` | ✅ | Registered with the clause registry. |
| `doacross` | ✅ | Registered with the clause registry. |
| `dynamic_allocators` | ✅ | Registered with the clause registry. |
| `exclusive` | ✅ | Registered with the clause registry. |
| `fail` | ✅ | Registered with the clause registry. |
| `final` | ✅ | Registered with the clause registry. |
| `filter` | ✅ | Registered with the clause registry. |
| `firstprivate` | ✅ | Registered with the clause registry. |
| `from` | ✅ | Registered with the clause registry. |
| `grainsize` | ✅ | Registered with the clause registry. |
| `hint` | ✅ | Registered with the clause registry. |
| `holds` | ✅ | Registered with the clause registry. |
| `if` | ✅ | Registered with the clause registry. |
| `in_reduction` | ✅ | Registered with the clause registry. |
| `inbranch` | ✅ | Registered with the clause registry. |
| `inclusive` | ✅ | Registered with the clause registry. |
| `init` | ✅ | Registered with the clause registry. |
| `interop` | ✅ | Registered with the clause registry. |
| `is_device_ptr` | ✅ | Registered with the clause registry. |
| `label` | ✅ | Registered with the clause registry. |
| `lastprivate` | ✅ | Registered with the clause registry. |
| `linear` | ✅ | Registered with the clause registry. |
| `link` | ✅ | Registered with the clause registry. |
| `map` | ✅ | Registered with the clause registry. |
| `match` | ✅ | Registered with the clause registry. |
| `message` | ✅ | Registered with the clause registry. |
| `mergeable` | ✅ | Registered with the clause registry. |
| `nontemporal` | ✅ | Registered with the clause registry. |
| `no_openmp` | ✅ | Registered with the clause registry. |
| `no_openmp_routines` | ✅ | Registered with the clause registry. |
| `no_parallelism` | ✅ | Registered with the clause registry. |
| `nogroup` | ✅ | Registered with the clause registry. |
| `novariants` | ✅ | Registered with the clause registry. |
| `nowait` | ✅ | Registered with the clause registry. |
| `num_tasks` | ✅ | Registered with the clause registry. |
| `num_teams` | ✅ | Registered with the clause registry. |
| `num_threads` | ✅ | Registered with the clause registry. |
| `order` | ✅ | Registered with the clause registry. |
| `ordered` | ✅ | Registered with the clause registry. |
| `partial` | ✅ | Registered with the clause registry. |
| `priority` | ✅ | Registered with the clause registry. |
| `private` | ✅ | Registered with the clause registry. |
| `proc_bind` | ✅ | Registered with the clause registry. |
| `public` | ✅ | Registered with the clause registry. |
| `reduction` | ✅ | Registered with the clause registry. |
| `release` | ✅ | Memory-order clause for `atomic`; registered as a bare clause. |
| `relaxed` | ✅ | Memory-order clause for `atomic`; registered as a bare clause. |
| `reverse` | ✅ | Registered with the clause registry. |
| `reproducible` | ✅ | Registered with the clause registry. |
| `safelen` | ✅ | Registered with the clause registry. |
| `schedule` | ✅ | Registered with the clause registry. |
| `seq_cst` | ✅ | Memory-order clause for `atomic`; registered as a bare clause. |
| `shared` | ✅ | Registered with the clause registry. |
| `simdlen` | ✅ | Registered with the clause registry. |
| `sizes` | ✅ | Registered with the clause registry. |
| `task_reduction` | ✅ | Registered with the clause registry. |
| `thread_limit` | ✅ | Registered with the clause registry. |
| `tile` | ✅ | Registered with the clause registry. |
| `to` | ✅ | Registered with the clause registry. |
| `unified_address` | ✅ | Registered with the clause registry. |
| `unified_shared_memory` | ✅ | Registered with the clause registry. |
| `unroll` | ✅ | Registered with the clause registry. |
| `untied` | ✅ | Registered with the clause registry. |
| `update` | ✅ | Registered with the clause registry. |
| `use_device_addr` | ✅ | Registered with the clause registry. |
| `use_device_ptr` | ✅ | Registered with the clause registry. |
| `uses_allocators` | ✅ | Registered with the clause registry. |
| `weak` | ✅ | Registered with the clause registry. |
| `when` | ✅ | Registered with the clause registry. |

## Other specification items

| Construct | Status | Notes |
| --- | --- | --- |
| Memory-order modifiers (`acq_rel`, `acquire`, `release`, `relaxed`, `seq_cst`) | ✅ | See clause coverage above; all standard memory-order modifiers parse as bare clauses. |
| Directive modifiers such as `begin/end declare target` pairs | ✅ | All begin/end directive modifiers are registered as individual directives. |
| Trait selectors for `declare variant` (e.g., `match`, `when`) | ✅ | Clause keywords recorded above and available through the clause registry. |

