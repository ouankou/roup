# OpenMP Support

ROUP provides comprehensive support for **OpenMP 6.0** directives and clauses for C/C++ and Fortran.

---

## Quick Summary

| Feature | Support |
|---------|---------|
| **OpenMP Version** | 3.0 - 6.0 |
| **Directive keywords** | 128 keywords (core, combined, declarative, and meta) |
| **Clause keywords** | 132 keywords |
| **Languages** | C, C++, Fortran |
| **Test Coverage** | 581 automated tests |
| **Specification** | [OpenMP 6.0 PDF](https://www.openmp.org/wp-content/uploads/OpenMP-API-Specification-6-0.pdf) |

**ROUP supports the complete OpenMP 6.0 directive and clause keyword set.** ✅
For the machine-generated catalogues see
[`openmp60-directives-clauses.md`](./openmp60-directives-clauses.md) and
[`openmp60-directive-clause-components.md`](./openmp60-directive-clause-components.md).

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
| `atomic` | `#pragma omp atomic` | Atomic operation (includes read, write, update, capture, compare capture) |
| `flush` | `#pragma omp flush` | Memory fence |
| `ordered` | `#pragma omp ordered` | Ordered execution |
| `simd` | `#pragma omp simd` | SIMD vectorization |
| `loop` | `#pragma omp loop` | Generic loop (OpenMP 5.0+) |
| `scope` | `#pragma omp scope` | Scoped region (OpenMP 5.1+) |

### Tasking (10 directives)

| Directive | Example | Notes |
|-----------|---------|-------|
| `task` | `#pragma omp task` | Explicit task |
| `taskwait` | `#pragma omp taskwait` | Wait for child tasks |
| `taskyield` | `#pragma omp taskyield` | Yield to other tasks |
| `taskgroup` | `#pragma omp taskgroup` | Task group |
| `taskloop` | `#pragma omp taskloop` | Task-generating loop |
| `taskloop simd` | `#pragma omp taskloop simd` | SIMD taskloop |
| `taskgraph` | `#pragma omp taskgraph` | Task graph (OpenMP 6.0) |
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
- `parallel loop simd`
- `parallel sections`
- `parallel master`
- `parallel masked`
- `parallel master taskloop`
- `parallel master taskloop simd`
- `parallel masked taskloop`
- `parallel masked taskloop simd`

**Target + Parallel:**
- `target parallel`
- `target parallel for`
- `target parallel for simd`
- `target parallel loop`
- `target parallel loop simd`
- `target loop`
- `target loop simd`
- `target teams`
- `target teams distribute`
- `target teams distribute parallel for`
- `target teams distribute parallel for simd`
- `target teams distribute parallel loop`
- `target teams distribute parallel loop simd`
- `target teams loop simd`

**Teams + Distribute:**
- `teams distribute`
- `teams distribute simd`
- `teams distribute parallel for`
- `teams distribute parallel for simd`
- `teams distribute parallel loop`
- `teams distribute parallel loop simd`
- `teams loop`
- `teams loop simd`

**Distribute + Loop:**
- `distribute parallel loop`
- `distribute parallel loop simd`

**Masked + Taskloop:**
- `masked taskloop`
- `masked taskloop simd`

**And many more...** (all 128 directive keywords from OpenMP 6.0)

### Meta-directives & Variants (5 directives)

| Directive | Example | Notes |
|-----------|---------|-------|
| `metadirective` | `#pragma omp metadirective when(...)` | Conditional directive selection |
| `declare variant` | `#pragma omp declare variant(...)` | Function variants |
| `declare simd` | `#pragma omp declare simd` | SIMD function |
| `declare reduction` | `#pragma omp declare reduction` | Custom reduction |
| `declare mapper` | `#pragma omp declare mapper` | Custom mapper |

### Utility Directives (6 directives)

| Directive | Example | Notes |
|-----------|---------|-------|
| `threadprivate` | `#pragma omp threadprivate(var)` | Thread-private data |
| `assume` | `#pragma omp assume` | Compiler hints (OpenMP 5.1+) |
| `nothing` | `#pragma omp nothing` | No-op directive |
| `error` | `#pragma omp error` | Compilation error |
| `requires` | `#pragma omp requires` | Implementation requirements |
| `allocate` | `#pragma omp allocate` | Memory allocation |

---

## Clause Support (92 clauses)

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

✅ **Supported:** Free-form (`!$omp`), Fixed-form (`!$omp`, `c$omp`, `*$omp`), Short forms (`!$`, `c$`, `*$`)
✅ **Case insensitive:** `!$OMP`, `!$Omp`, `!$omp` all work

---

## Test Coverage

ROUP includes comprehensive automated testing:

| Test Suite | Count | Coverage |
|------------|-------|----------|
| **Unit tests** | 285 | Core parsing logic |
| **Integration tests** | 234 | Directive parsing, clause combinations |
| **Doc tests** | 62 | API examples, edge cases |
| **Total** | **581** | All directives + clauses tested |

**Every directive and clause in OpenMP 6.0 has a passing test.** ✅

See `tests/` directory for the full test suite.

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

**ROUP provides OpenMP 3.0-6.0 parsing support:**

- ✅ **128 directive keywords** (core, combined, declarative, and meta-directives)
- ✅ **132 clause keywords** (data-sharing, control, device, SIMD, sync, etc.)
- ✅ **C/C++/Fortran** syntax support
- ✅ **581 automated tests** (comprehensive directive/clause coverage)
- ✅ **Type-safe Rust API** + **C FFI**
- ✅ **Latest spec** (OpenMP 6.0, 2024)

**⚠️ Experimental - for research, education, and prototype tooling. Not yet production-ready.** 🧪
