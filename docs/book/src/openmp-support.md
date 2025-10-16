# OpenMP Support

ROUP provides comprehensive support for **OpenMP 6.0** directives and clauses for C/C++ and Fortran.

---

## Quick Summary

| Feature | Support |
|---------|---------|
| **OpenMP Version** | 3.0 - 6.0 |
| **Directives** | 127 directive spellings (64 base + 63 combined forms) |
| **Clauses** | 132 clauses (125 from spec + 7 extras) |
| **Languages** | C, C++, Fortran |
| **Test Coverage** | 384 automated tests |
| **Specification** | [OpenMP 6.0 PDF](https://www.openmp.org/wp-content/uploads/OpenMP-API-Specification-6-0.pdf) |

**ROUP provides complete OpenMP 6.0 directive and clause coverage.** ✅

---

## Directive Support

### Core Parallelism (15 directives)

| Directive | Example | Notes |
|-----------|---------|-------|
| `parallel` | `#pragma omp parallel` | Basic parallel region |
| `for` | `#pragma omp for` | Work-sharing loop |
| `sections` | `#pragma omp sections` | Work-sharing sections |
| `section` | `#pragma omp section` | Individual section |
| `single` | `#pragma omp single` | Single-thread execution |
| `master` | `#pragma omp master` | Master thread only |
| `masked` | `#pragma omp masked` | Masked execution (OpenMP 5.1+) |
| `barrier` | `#pragma omp barrier` | Synchronization barrier |
| `critical` | `#pragma omp critical` | Critical section |
| `atomic` | `#pragma omp atomic` | Atomic operation |
| `flush` | `#pragma omp flush` | Memory fence |
| `ordered` | `#pragma omp ordered` | Ordered execution |
| `simd` | `#pragma omp simd` | SIMD vectorization |
| `loop` | `#pragma omp loop` | Generic loop (OpenMP 5.0+) |
| `scope` | `#pragma omp scope` | Scoped region (OpenMP 5.1+) |

### Tasking (11 directives)

| Directive | Example | Notes |
|-----------|---------|-------|
| `task` | `#pragma omp task` | Explicit task |
| `taskwait` | `#pragma omp taskwait` | Wait for child tasks |
| `taskyield` | `#pragma omp taskyield` | Yield to other tasks |
| `taskgroup` | `#pragma omp taskgroup` | Task group |
| `taskloop` | `#pragma omp taskloop` | Task-generating loop |
| `taskloop simd` | `#pragma omp taskloop simd` | SIMD taskloop |
| `taskgraph` | `#pragma omp taskgraph` | Task graph (OpenMP 6.0) |
| `task_iteration` | `#pragma omp task_iteration` | Task iteration (OpenMP 6.0) |
| `cancel` | `#pragma omp cancel` | Cancel construct |
| `cancellation point` | `#pragma omp cancellation point` | Cancellation check |
| `depobj` | `#pragma omp depobj` | Dependency object |

### Device Offloading (12 directives)

| Directive | Example | Notes |
|-----------|---------|-------|
| `target` | `#pragma omp target` | Offload to device |
| `target data` | `#pragma omp target data` | Data environment |
| `target enter data` | `#pragma omp target enter data` | Map to device |
| `target exit data` | `#pragma omp target exit data` | Unmap from device |
| `target update` | `#pragma omp target update` | Update data |
| `teams` | `#pragma omp teams` | Team of threads |
| `distribute` | `#pragma omp distribute` | Distribute iterations |
| `declare target` | `#pragma omp declare target` | Device function |
| `begin declare target` | `#pragma omp begin declare target` | Begin target block |
| `end declare target` | `#pragma omp end declare target` | End target block |
| `interop` | `#pragma omp interop` | Interoperability (OpenMP 5.1+) |
| `dispatch` | `#pragma omp dispatch` | Dynamic dispatch (OpenMP 5.1+) |

### Combined Directives (60+ forms)

ROUP supports all standard combined directives:

**Parallel + Worksharing:**
- `parallel for`
- `parallel for simd`
- `parallel loop`
- `parallel sections`
- `parallel master`
- `parallel masked`

**Target + Parallel:**
- `target parallel`
- `target parallel for`
- `target parallel for simd`
- `target parallel loop`
- `target teams`
- `target teams distribute`
- `target teams distribute parallel for`
- `target teams distribute parallel for simd`

**Teams + Distribute:**
- `teams distribute`
- `teams distribute simd`
- `teams distribute parallel for`
- `teams distribute parallel for simd`
- `teams loop`

**And many more...** (all 120+ combinations from OpenMP 6.0)

### Meta-directives & Variants (9 directives)

| Directive | Example | Notes |
|-----------|---------|-------|
| `metadirective` | `#pragma omp metadirective when(...)` | Conditional directive selection |
| `begin metadirective` | `#pragma omp begin metadirective` | Begin metadirective block |
| `declare variant` | `#pragma omp declare variant(...)` | Function variants |
| `begin declare_variant` | `#pragma omp begin declare_variant` | Begin variant block (OpenMP 6.0) |
| `declare simd` | `#pragma omp declare simd` | SIMD function |
| `declare reduction` | `#pragma omp declare reduction` | Custom reduction |
| `declare induction` | `#pragma omp declare_induction` | Custom induction (OpenMP 6.0) |
| `declare mapper` | `#pragma omp declare mapper` | Custom mapper |
| `declare target` | `#pragma omp declare target` | Device function declaration |

### Loop Transformation Directives (8 directives - OpenMP 6.0)

| Directive | Example | Notes |
|-----------|---------|-------|
| `tile` | `#pragma omp tile sizes(8, 8)` | Loop tiling |
| `unroll` | `#pragma omp unroll full` | Loop unrolling |
| `fuse` | `#pragma omp fuse` | Loop fusion |
| `split` | `#pragma omp split counts(10, omp_fill)` | Loop splitting |
| `interchange` | `#pragma omp interchange permutation(2,1,3)` | Loop interchange |
| `reverse` | `#pragma omp reverse` | Loop reversal |
| `stripe` | `#pragma omp stripe sizes(16)` | Loop striping |
| `workdistribute` | `#pragma omp workdistribute` | Work distribution (Fortran) |

### Utility Directives (11 directives)

| Directive | Example | Notes |
|-----------|---------|-------|
| `threadprivate` | `#pragma omp threadprivate(var)` | Thread-private data |
| `groupprivate` | `#pragma omp groupprivate(var)` | Group-private data (OpenMP 6.0) |
| `assume` | `#pragma omp assume` | Compiler hints (OpenMP 5.1+) |
| `assumes` | `#pragma omp assumes` | Assumption directives (OpenMP 5.1+) |
| `begin assumes` | `#pragma omp begin assumes` | Begin assumption block |
| `nothing` | `#pragma omp nothing` | No-op directive |
| `error` | `#pragma omp error` | Compilation error |
| `requires` | `#pragma omp requires` | Implementation requirements |
| `allocate` | `#pragma omp allocate` | Memory allocation (declarative) |
| `allocators` | `#pragma omp allocators` | Memory allocators (executable) |
| `scan` | `#pragma omp scan` | Scan directive (OpenMP 5.0+) |

---

## Clause Support (132 clauses)

### Data-Sharing Clauses (8)

| Clause | Example | Description |
|--------|---------|-------------|
| `private` | `private(x, y)` | Private variables |
| `shared` | `shared(a, b)` | Shared variables |
| `firstprivate` | `firstprivate(z)` | Private with initialization |
| `lastprivate` | `lastprivate(result)` | Private with final value |
| `reduction` | `reduction(+:sum)` | Reduction operation |
| `in_reduction` | `in_reduction(+:total)` | Participating reduction |
| `task_reduction` | `task_reduction(*:product)` | Task reduction |
| `copyin` | `copyin(global_var)` | Copy to private |

### Control Clauses (15)

| Clause | Example | Description |
|--------|---------|-------------|
| `if` | `if(condition)` | Conditional execution |
| `num_threads` | `num_threads(8)` | Thread count |
| `default` | `default(shared)` | Default data-sharing |
| `schedule` | `schedule(static, 100)` | Loop scheduling |
| `collapse` | `collapse(2)` | Nest loop collapsing |
| `ordered` | `ordered` | Ordered execution |
| `nowait` | `nowait` | Remove implicit barrier |
| `final` | `final(expr)` | Final task |
| `untied` | `untied` | Untied task |
| `mergeable` | `mergeable` | Mergeable task |
| `priority` | `priority(10)` | Task priority |
| `grainsize` | `grainsize(1000)` | Taskloop grainsize |
| `num_tasks` | `num_tasks(100)` | Taskloop task count |
| `nogroup` | `nogroup` | No taskgroup |
| `filter` | `filter(thread_num)` | Masked filter |

### Device Clauses (15)

| Clause | Example | Description |
|--------|---------|-------------|
| `device` | `device(gpu_id)` | Target device |
| `map` | `map(to: input)` | Data mapping |
| `to` | `to(data)` | Map to device |
| `from` | `from(results)` | Map from device |
| `defaultmap` | `defaultmap(tofrom:scalar)` | Default mapping |
| `is_device_ptr` | `is_device_ptr(ptr)` | Device pointer |
| `use_device_ptr` | `use_device_ptr(ptr)` | Use device pointer |
| `use_device_addr` | `use_device_addr(var)` | Use device address |
| `device_resident` | `device_resident(var)` | Device-resident data |
| `num_teams` | `num_teams(16)` | Number of teams |
| `thread_limit` | `thread_limit(256)` | Threads per team |
| `dist_schedule` | `dist_schedule(static)` | Distribution schedule |
| `interop` | `interop(...)` | Interoperability |
| `device_type` | `device_type(gpu)` | Device type selector |
| `init` | `init(...)` | Initialize interop |

### SIMD Clauses (8)

| Clause | Example | Description |
|--------|---------|-------------|
| `simdlen` | `simdlen(8)` | SIMD lane count |
| `safelen` | `safelen(16)` | Safe iteration count |
| `aligned` | `aligned(ptr:32)` | Alignment |
| `linear` | `linear(i:1)` | Linear variable |
| `uniform` | `uniform(step)` | Uniform across lanes |
| `nontemporal` | `nontemporal(a, b)` | Non-temporal access |
| `inbranch` | `inbranch` | In-branch SIMD |
| `notinbranch` | `notinbranch` | Not-in-branch SIMD |

### Synchronization & Memory (12)

| Clause | Example | Description |
|--------|---------|-------------|
| `depend` | `depend(in: x)` | Task dependencies |
| `doacross` | `doacross(source:)` | Cross-iteration dependencies |
| `detach` | `detach(event)` | Detachable task |
| `atomic_default_mem_order` | `atomic_default_mem_order(seq_cst)` | Default memory order |
| `seq_cst` | `seq_cst` | Sequential consistency |
| `acq_rel` | `acq_rel` | Acquire-release |
| `acquire` | `acquire` | Acquire |
| `release` | `release` | Release |
| `relaxed` | `relaxed` | Relaxed |
| `compare` | `compare` | Compare atomic |
| `fail` | `fail(relaxed)` | Failure memory order |
| `weak` | `weak` | Weak compare |

### Metadirective & Variant Clauses (5)

| Clause | Example | Description |
|--------|---------|-------------|
| `when` | `when(device:{kind(gpu)})` | Condition selector |
| `match` | `match(construct={...})` | Trait matching |
| `novariants` | `novariants` | No variants |
| `holds` | `holds(...)` | Assumption holds |
| `bind` | `bind(thread)` | Binding |

### Miscellaneous Clauses (29)

| Clause | Example | Description |
|--------|---------|-------------|
| `allocate` | `allocate(allocator:ptr)` | Memory allocator |
| `allocator` | `allocator(omp_default_mem_alloc)` | Allocator |
| `uses_allocators` | `uses_allocators(...)` | Allocator list |
| `affinity` | `affinity(...)` | Thread affinity |
| `proc_bind` | `proc_bind(close)` | Processor binding |
| `order` | `order(concurrent)` | Loop iteration order |
| `partial` | `partial(4)` | Partial unroll |
| `sizes` | `sizes(8, 16)` | Tile sizes |
| `tile` | `tile(...)` | Loop tiling |
| `unroll` | `unroll(4)` | Loop unrolling |
| `label` | `label(...)` | Dispatch label |
| `message` | `message("error text")` | Error message |
| `copyprivate` | `copyprivate(x)` | Copy-private |
| `link` | `link(...)` | Declare target link |
| `capture` | `capture` | Atomic capture |
| `update` | `update` | Atomic update |
| `hint` | `hint(...)` | Performance hint |
| `destroy` | `destroy` | Destroy clause |
| `reverse` | `reverse` | Reverse dependencies |
| `inclusive` | `inclusive(...)` | Inclusive scan |
| `exclusive` | `exclusive(...)` | Exclusive scan |
| `unified_address` | `unified_address` | Requires clause |
| `unified_shared_memory` | `unified_shared_memory` | Requires clause |
| `dynamic_allocators` | `dynamic_allocators` | Requires clause |
| `reproducible` | `reproducible` | Order modifier |
| `no_openmp` | `no_openmp` | Variant selector |
| `no_openmp_routines` | `no_openmp_routines` | Variant selector |
| `no_parallelism` | `no_parallelism` | Variant selector |
| `public` | `public` | Declare mapper |

### OpenMP 6.0 New Clauses (41 clauses)

| Clause | Example | Description |
|--------|---------|-------------|
| `absent` | `absent(x)` | Context selector - feature absent |
| `adjust_args` | `adjust_args(need_device_addr: x)` | Adjust function arguments |
| `align` | `align(64)` | Memory alignment |
| `append_args` | `append_args(x, y)` | Append function arguments |
| `apply` | `apply(tile(8,8))` | Apply transformation |
| `at` | `at(compilation)` | Error timing |
| `collector` | `collector(+)` | Induction collector operator |
| `combiner` | `combiner(+)` | Reduction combiner operator |
| `contains` | `contains(target)` | Assumption contains |
| `counts` | `counts(10, omp_fill)` | Split iteration counts |
| `device_safesync` | `device_safesync` | Device safe synchronization |
| `enter` | `enter(link: x)` | Declare target enter |
| `full` | `full` | Full unroll |
| `graph_id` | `graph_id(1)` | Task graph identifier |
| `graph_reset` | `graph_reset` | Reset task graph |
| `has_device_addr` | `has_device_addr(x)` | Has device address |
| `indirect` | `indirect` | Indirect call support |
| `induction` | `induction(i = 0 : N : 1)` | Induction variable |
| `inductor` | `inductor(+: x)` | Induction inductor |
| `init_complete` | `init_complete` | Initialization complete |
| `initializer` | `initializer(omp_priv = 0)` | Reduction initializer |
| `local` | `local(x)` | Local allocation |
| `looprange` | `looprange(1:10)` | Loop iteration range |
| `memscope` | `memscope(device)` | Memory scope |
| `no_openmp_constructs` | `no_openmp_constructs` | No OpenMP constructs |
| `nocontext` | `nocontext` | No context variant |
| `otherwise` | `otherwise(parallel)` | Metadirective fallback |
| `permutation` | `permutation(2,1)` | Loop interchange order |
| `read` | `read` | Atomic read operation |
| `replayable` | `replayable` | Replayable taskloop |
| `reverse_offload` | `reverse_offload` | Reverse offload support |
| `safesync` | `safesync` | Safe synchronization |
| `self_maps` | `self_maps` | Self maps support |
| `severity` | `severity(warning)` | Error severity level |
| `simd` | `simd` | SIMD execution |
| `threads` | `threads` | Thread execution mode |
| `threadset` | `threadset(1)` | Thread set specification |
| `transparent` | `transparent` | Transparent dependency object |
| `uniform` | `uniform(x)` | SIMD uniform variable |
| `use` | `use(obj)` | Interop use clause |
| `write` | `write` | Atomic write operation |

---

## Version Compatibility

| OpenMP Version | ROUP Support | Key Features |
|----------------|--------------|--------------|
| **6.0** (2024) | ✅ Full | `taskgraph`, `dispatch`, new loop constructs |
| **5.2** (2021) | ✅ Full | `scope`, `masked`, device extensions |
| **5.1** (2020) | ✅ Full | `assume`, `nothing`, `metadirective` enhancements |
| **5.0** (2018) | ✅ Full | `loop`, `requires`, memory allocators |
| **4.5** (2015) | ✅ Full | Task dependencies, device constructs |
| **4.0** (2013) | ✅ Full | Tasking, SIMD, device offloading |

**ROUP tracks the latest OpenMP specification** and is updated with each new release.

---

## Language Support

### C/C++ Syntax

```c
#pragma omp parallel for num_threads(4) schedule(static)
for (int i = 0; i < n; i++) {
    process(i);
}
```

✅ **Supported:** All C/C++ `#pragma omp` directives

### Fortran Syntax

```fortran
!$omp parallel do schedule(dynamic)
do i = 1, n
    call process(i)
end do
!$omp end parallel do
```

✅ **Supported:** Free-form (`!$omp`), Fixed-form (`c$omp`, `*$omp`)  
✅ **Case insensitive:** `!$OMP`, `!$Omp`, `!$omp` all work

---

## Test Coverage

ROUP includes comprehensive automated testing:

| Test Suite | Count | Coverage |
|------------|-------|----------|
| **Integration tests** | 145 | Directive parsing, clause combinations, OpenMP 6.0 coverage |
| **Doc tests** | 239 | API examples, edge cases |
| **Total** | **384** | All directives + clauses tested |

**Every directive and clause in OpenMP 6.0 has a passing test.** ✅

Key test files:
- `tests/openmp_keyword_coverage.rs` - Validates all 64 base directives and 41 new clauses
- `tests/language_parsing_integration.rs` - Cross-language parsing tests
- See full test suite in `tests/` directory

---

## Unsupported Features

ROUP focuses on **directive parsing**, not runtime semantics. The following are **out of scope**:

❌ **Runtime execution** - ROUP parses directives, doesn't execute them  
❌ **Code transformation** - No AST rewriting or code generation  
❌ **Semantic validation** - Doesn't check if directives make logical sense  
❌ **Context analysis** - Doesn't validate directive placement in code  

**Use ROUP for:**
- ✅ Static analysis tools
- ✅ Code documentation generators
- ✅ IDE syntax highlighting
- ✅ Linting and code quality checks
- ✅ Migration tools
- ✅ Research prototypes

**Don't use ROUP for:**
- ❌ Compiling OpenMP to executables (use GCC/Clang)
- ❌ Runtime task scheduling (use OpenMP runtime)
- ❌ Performance profiling (use performance tools)

---

## Implementation Status

| Component | Status | Notes |
|-----------|--------|-------|
| **Lexer** | ✅ Complete | Handles C/C++/Fortran syntax |
| **Parser** | ✅ Complete | nom-based combinator parser |
| **IR (Intermediate Representation)** | ✅ Complete | Type-safe Rust AST |
| **C FFI** | ✅ Complete | Pointer-based API (16 functions) |
| **Fortran support** | ✅ Complete | All comment styles |
| **Error messages** | ✅ Complete | Descriptive parse errors |
| **Round-trip** | ✅ Complete | IR → String preservation |

---

## Future Additions

Tracking future OpenMP specifications:

| Version | Status | Expected Features |
|---------|--------|-------------------|
| **OpenMP 6.1** | 📋 Planned | TBD by OpenMP ARB |
| **OpenMP 7.0** | 📋 Future | TBD |

ROUP will be updated as new OpenMP versions are released.

---

## References

- **[OpenMP 6.0 Specification](https://www.openmp.org/wp-content/uploads/OpenMP-API-Specification-6-0.pdf)** - Official spec
- **[OpenMP.org](https://www.openmp.org/)** - OpenMP Architecture Review Board
- **[ROUP GitHub](https://github.com/ouankou/roup)** - Source code and tests

---

## Summary

**ROUP provides complete OpenMP 3.0-6.0 parsing support:**

- ✅ **127 directive spellings** (64 base directives + 63 combined forms)
- ✅ **132 clause keywords** (125 from OpenMP 6.0 spec + 7 extras)
- ✅ **C/C++/Fortran** syntax support (all comment styles)
- ✅ **384 automated tests** (100% directive/clause coverage)
- ✅ **Type-safe Rust API** + **C FFI**
- ✅ **Latest spec** (OpenMP 6.0, November 2024)

**All OpenMP 6.0 directives and clauses are fully supported and tested.** ✅

**⚠️ Experimental - for research, education, and prototype tooling. Not yet production-ready.** 🧪
