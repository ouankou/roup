# OpenMP 6.0 Directive–Clause Components

This reference summarises how the ROUP parser tokenises OpenMP 6.0 clause
keywords.  Rather than attempting to restate the entire specification, the
information below mirrors the parser's [`ClauseRule`](../../../src/parser/openmp.rs)
data so you can quickly see which textual forms are accepted.  Combined
constructs (for example `parallel for` or `target teams distribute parallel for
simd`) are already part of the directive keyword registry listed in the
[directive catalogue](./openmp60-directives-clauses.md).

> **Note:** The OpenMP specification defines which directives may use a given
> clause and any semantic restrictions.  ROUP currently enforces keyword syntax
> and delegates the normative rules to higher layers.  Consult the
> [OpenMP 6.0 specification](https://www.openmp.org/wp-content/uploads/OpenMP-API-Specification-6-0.pdf)
> for the full directive–clause compatibility matrix.

## Clause syntax summary

| Clause | Parser rule | Accepted forms |
| --- | --- | --- |
| `absent` | Parenthesized | `absent(...)` |
| `acq_rel` | Bare | `#pragma omp parallel acq_rel` |
| `acquire` | Bare | `#pragma omp parallel acquire` |
| `adjust_args` | Parenthesized | `adjust_args(...)` |
| `affinity` | Parenthesized | `affinity(...)` |
| `align` | Parenthesized | `align(...)` |
| `aligned` | Parenthesized | `aligned(...)` |
| `allocate` | Parenthesized | `allocate(...)` |
| `allocator` | Parenthesized | `allocator(...)` |
| `append_args` | Parenthesized | `append_args(...)` |
| `apply` | Parenthesized | `apply(...)` |
| `at` | Parenthesized | `at(...)` |
| `atomic_default_mem_order` | Parenthesized | `atomic_default_mem_order(...)` |
| `bind` | Parenthesized | `bind(...)` |
| `capture` | Flexible | `capture` or `capture(...)` |
| `collapse` | Parenthesized | `collapse(...)` |
| `collector` | Parenthesized | `collector(...)` |
| `combiner` | Parenthesized | `combiner(...)` |
| `compare` | Flexible | `compare` or `compare(...)` |
| `contains` | Parenthesized | `contains(...)` |
| `copyin` | Parenthesized | `copyin(...)` |
| `copyprivate` | Parenthesized | `copyprivate(...)` |
| `counts` | Parenthesized | `counts(...)` |
| `default` | Parenthesized | `default(...)` |
| `defaultmap` | Parenthesized | `defaultmap(...)` |
| `depend` | Parenthesized | `depend(...)` |
| `destroy` | Flexible | `destroy` or `destroy(...)` |
| `detach` | Parenthesized | `detach(...)` |
| `device` | Parenthesized | `device(...)` |
| `device_resident` | Parenthesized | `device_resident(...)` |
| `device_safesync` | Flexible | `device_safesync` or `device_safesync(...)` |
| `device_type` | Parenthesized | `device_type(...)` |
| `dist_schedule` | Parenthesized | `dist_schedule(...)` |
| `doacross` | Parenthesized | `doacross(...)` |
| `dynamic_allocators` | Bare | `#pragma omp parallel dynamic_allocators` |
| `enter` | Parenthesized | `enter(...)` |
| `exclusive` | Bare | `#pragma omp parallel exclusive` |
| `fail` | Flexible | `fail` or `fail(...)` |
| `filter` | Parenthesized | `filter(...)` |
| `final` | Parenthesized | `final(...)` |
| `firstprivate` | Parenthesized | `firstprivate(...)` |
| `from` | Parenthesized | `from(...)` |
| `full` | Flexible | `full` or `full(...)` |
| `grainsize` | Parenthesized | `grainsize(...)` |
| `graph_id` | Parenthesized | `graph_id(...)` |
| `graph_reset` | Parenthesized | `graph_reset(...)` |
| `has_device_addr` | Parenthesized | `has_device_addr(...)` |
| `hint` | Parenthesized | `hint(...)` |
| `holds` | Parenthesized | `holds(...)` |
| `if` | Parenthesized | `if(...)` |
| `in_reduction` | Parenthesized | `in_reduction(...)` |
| `inbranch` | Bare | `#pragma omp parallel inbranch` |
| `inclusive` | Bare | `#pragma omp parallel inclusive` |
| `indirect` | Flexible | `indirect` or `indirect(...)` |
| `induction` | Parenthesized | `induction(...)` |
| `inductor` | Parenthesized | `inductor(...)` |
| `init` | Parenthesized | `init(...)` |
| `init_complete` | Flexible | `init_complete` or `init_complete(...)` |
| `initializer` | Parenthesized | `initializer(...)` |
| `interop` | Parenthesized | `interop(...)` |
| `is_device_ptr` | Parenthesized | `is_device_ptr(...)` |
| `label` | Parenthesized | `label(...)` |
| `lastprivate` | Parenthesized | `lastprivate(...)` |
| `linear` | Parenthesized | `linear(...)` |
| `link` | Parenthesized | `link(...)` |
| `local` | Parenthesized | `local(...)` |
| `looprange` | Parenthesized | `looprange(...)` |
| `map` | Parenthesized | `map(...)` |
| `match` | Parenthesized | `match(...)` |
| `memscope` | Parenthesized | `memscope(...)` |
| `mergeable` | Bare | `#pragma omp parallel mergeable` |
| `message` | Parenthesized | `message(...)` |
| `no_openmp` | Flexible | `no_openmp` or `no_openmp(...)` |
| `no_openmp_constructs` | Flexible | `no_openmp_constructs` or `no_openmp_constructs(...)` |
| `no_openmp_routines` | Flexible | `no_openmp_routines` or `no_openmp_routines(...)` |
| `no_parallelism` | Flexible | `no_parallelism` or `no_parallelism(...)` |
| `nocontext` | Parenthesized | `nocontext(...)` |
| `nogroup` | Bare | `#pragma omp parallel nogroup` |
| `nontemporal` | Parenthesized | `nontemporal(...)` |
| `notinbranch` | Bare | `#pragma omp parallel notinbranch` |
| `novariants` | Flexible | `novariants` or `novariants(...)` |
| `nowait` | Bare | `#pragma omp parallel nowait` |
| `num_tasks` | Parenthesized | `num_tasks(...)` |
| `num_teams` | Parenthesized | `num_teams(...)` |
| `num_threads` | Parenthesized | `num_threads(...)` |
| `order` | Parenthesized | `order(...)` |
| `ordered` | Flexible | `ordered` or `ordered(...)` |
| `otherwise` | Parenthesized | `otherwise(...)` |
| `partial` | Flexible | `partial` or `partial(...)` |
| `permutation` | Parenthesized | `permutation(...)` |
| `priority` | Parenthesized | `priority(...)` |
| `private` | Parenthesized | `private(...)` |
| `proc_bind` | Parenthesized | `proc_bind(...)` |
| `public` | Flexible | `public` or `public(...)` |
| `read` | Flexible | `read` or `read(...)` |
| `reduction` | Parenthesized | `reduction(...)` |
| `relaxed` | Bare | `#pragma omp parallel relaxed` |
| `release` | Bare | `#pragma omp parallel release` |
| `replayable` | Flexible | `replayable` or `replayable(...)` |
| `reproducible` | Bare | `#pragma omp parallel reproducible` |
| `reverse` | Flexible | `reverse` or `reverse(...)` |
| `reverse_offload` | Bare | `#pragma omp parallel reverse_offload` |
| `safelen` | Parenthesized | `safelen(...)` |
| `safesync` | Bare | `#pragma omp parallel safesync` |
| `schedule` | Parenthesized | `schedule(...)` |
| `self_maps` | Bare | `#pragma omp parallel self_maps` |
| `seq_cst` | Bare | `#pragma omp parallel seq_cst` |
| `severity` | Parenthesized | `severity(...)` |
| `shared` | Parenthesized | `shared(...)` |
| `simd` | Bare | `#pragma omp parallel simd` |
| `simdlen` | Parenthesized | `simdlen(...)` |
| `sizes` | Parenthesized | `sizes(...)` |
| `task_reduction` | Parenthesized | `task_reduction(...)` |
| `thread_limit` | Parenthesized | `thread_limit(...)` |
| `threads` | Bare | `#pragma omp parallel threads` |
| `threadset` | Parenthesized | `threadset(...)` |
| `tile` | Parenthesized | `tile(...)` |
| `to` | Parenthesized | `to(...)` |
| `transparent` | Flexible | `transparent` or `transparent(...)` |
| `unified_address` | Flexible | `unified_address` or `unified_address(...)` |
| `unified_shared_memory` | Flexible | `unified_shared_memory` or `unified_shared_memory(...)` |
| `uniform` | Parenthesized | `uniform(...)` |
| `unroll` | Flexible | `unroll` or `unroll(...)` |
| `untied` | Bare | `#pragma omp parallel untied` |
| `update` | Flexible | `update` or `update(...)` |
| `use` | Parenthesized | `use(...)` |
| `use_device_addr` | Parenthesized | `use_device_addr(...)` |
| `use_device_ptr` | Parenthesized | `use_device_ptr(...)` |
| `uses_allocators` | Parenthesized | `uses_allocators(...)` |
| `weak` | Flexible | `weak` or `weak(...)` |
| `when` | Parenthesized | `when(...)` |
| `write` | Flexible | `write` or `write(...)` |

## Updating this index

The table can be regenerated with the following helper if new clauses are added
or the parser changes a clause rule:

```bash
python - <<'PY'
import pathlib, re
text = pathlib.Path('src/parser/openmp.rs').read_text()
block = re.search(r"openmp_clauses!\s*{(.*?)}\s*\n\nmacro_rules!", text, re.S).group(1)
clauses = []
for entry in block.split('},'):
    name_match = re.search(r'name: "([^"]+)"', entry)
    rule_match = re.search(r'rule: ClauseRule::([A-Za-z_]+)', entry)
    if name_match and rule_match:
        clauses.append((name_match.group(1), rule_match.group(1)))
for name, rule in sorted(clauses):
    if rule == 'Bare':
        forms = f"`#pragma omp parallel {name}`"
    elif rule == 'Parenthesized':
        forms = f"`{name}(...)`"
    else:
        forms = f"`{name}` or `{name}(...)`"
    print(f"| `{name}` | {rule} | {forms} |")
PY
```

Copy the generated rows into the table above to keep this document aligned with
`OpenMpClause::ALL`.
