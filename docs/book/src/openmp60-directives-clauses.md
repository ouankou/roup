# OpenMP 6.0 Directives and Clauses

This catalogue is derived directly from the ROUP parser's keyword registries and
shows exactly which OpenMP 6.0 directive and clause tokens are recognised.  The
source of truth is `OpenMpDirective::ALL` and `OpenMpClause::ALL` in
[`src/parser/openmp.rs`](../../../src/parser/openmp.rs).  Every entry listed
below is exercised by the automated support-matrix tests, so the tables always
reflect parser reality.

For the normative meaning of each keyword, consult the
[OpenMP Application Programming Interface Version 6.0 specification](https://www.openmp.org/wp-content/uploads/OpenMP-API-Specification-6-0.pdf).

## Directive keywords (127 total)

|  |  |  |  |
| --- | --- | --- | --- |
| `allocate` | `distribute parallel loop simd` | `parallel master taskloop` | `target teams distribute parallel for simd` |
| `allocators` | `distribute simd` | `parallel master taskloop simd` | `target teams distribute parallel loop` |
| `assume` | `do` | `parallel sections` | `target teams distribute parallel loop simd` |
| `assumes` | `do simd` | `requires` | `target teams distribute simd` |
| `atomic` | `end declare target` | `reverse` | `target teams loop` |
| `atomic capture` | `error` | `scan` | `target teams loop simd` |
| `atomic compare capture` | `flush` | `scope` | `target update` |
| `atomic read` | `for` | `section` | `task` |
| `atomic update` | `for simd` | `sections` | `task iteration` |
| `atomic write` | `fuse` | `simd` | `taskgraph` |
| `barrier` | `groupprivate` | `single` | `taskgroup` |
| `begin assumes` | `interchange` | `split` | `taskloop` |
| `begin declare target` | `interop` | `stripe` | `taskloop simd` |
| `begin declare variant` | `loop` | `target` | `taskwait` |
| `begin metadirective` | `masked` | `target data` | `taskyield` |
| `cancel` | `masked taskloop` | `target enter data` | `teams` |
| `cancellation point` | `masked taskloop simd` | `target exit data` | `teams distribute` |
| `critical` | `master` | `target loop` | `teams distribute parallel do` |
| `declare induction` | `metadirective` | `target loop simd` | `teams distribute parallel do simd` |
| `declare mapper` | `nothing` | `target parallel` | `teams distribute parallel for` |
| `declare reduction` | `ordered` | `target parallel do` | `teams distribute parallel for simd` |
| `declare simd` | `parallel` | `target parallel do simd` | `teams distribute parallel loop` |
| `declare target` | `parallel do` | `target parallel for` | `teams distribute parallel loop simd` |
| `declare variant` | `parallel do simd` | `target parallel for simd` | `teams distribute simd` |
| `depobj` | `parallel for` | `target parallel loop` | `teams loop` |
| `dispatch` | `parallel for simd` | `target parallel loop simd` | `teams loop simd` |
| `distribute` | `parallel loop` | `target simd` | `threadprivate` |
| `distribute parallel do` | `parallel loop simd` | `target teams` | `tile` |
| `distribute parallel do simd` | `parallel masked` | `target teams distribute` | `unroll` |
| `distribute parallel for` | `parallel masked taskloop` | `target teams distribute parallel do` | `workdistribute` |
| `distribute parallel for simd` | `parallel masked taskloop simd` | `target teams distribute parallel do simd` | `workshare` |
| `distribute parallel loop` | `parallel master` | `target teams distribute parallel for` |  |

## Clause keywords (132 total)

|  |  |  |  |
| --- | --- | --- | --- |
| `absent` | `doacross` | `looprange` | `reproducible` |
| `acq_rel` | `dynamic_allocators` | `map` | `reverse` |
| `acquire` | `enter` | `match` | `reverse_offload` |
| `adjust_args` | `exclusive` | `memscope` | `safelen` |
| `affinity` | `fail` | `mergeable` | `safesync` |
| `align` | `filter` | `message` | `schedule` |
| `aligned` | `final` | `no_openmp` | `self_maps` |
| `allocate` | `firstprivate` | `no_openmp_constructs` | `seq_cst` |
| `allocator` | `from` | `no_openmp_routines` | `severity` |
| `append_args` | `full` | `no_parallelism` | `shared` |
| `apply` | `grainsize` | `nocontext` | `simd` |
| `at` | `graph_id` | `nogroup` | `simdlen` |
| `atomic_default_mem_order` | `graph_reset` | `nontemporal` | `sizes` |
| `bind` | `has_device_addr` | `notinbranch` | `task_reduction` |
| `capture` | `hint` | `novariants` | `thread_limit` |
| `collapse` | `holds` | `nowait` | `threads` |
| `collector` | `if` | `num_tasks` | `threadset` |
| `combiner` | `in_reduction` | `num_teams` | `tile` |
| `compare` | `inbranch` | `num_threads` | `to` |
| `contains` | `inclusive` | `order` | `transparent` |
| `copyin` | `indirect` | `ordered` | `unified_address` |
| `copyprivate` | `induction` | `otherwise` | `unified_shared_memory` |
| `counts` | `inductor` | `partial` | `uniform` |
| `default` | `init` | `permutation` | `unroll` |
| `defaultmap` | `init_complete` | `priority` | `untied` |
| `depend` | `initializer` | `private` | `update` |
| `destroy` | `interop` | `proc_bind` | `use` |
| `detach` | `is_device_ptr` | `public` | `use_device_addr` |
| `device` | `label` | `read` | `use_device_ptr` |
| `device_resident` | `lastprivate` | `reduction` | `uses_allocators` |
| `device_safesync` | `linear` | `relaxed` | `weak` |
| `device_type` | `link` | `release` | `when` |
| `dist_schedule` | `local` | `replayable` | `write` |

## Keeping the list in sync

To regenerate the tables after changing `src/parser/openmp.rs`, run the helper
script below and replace the output in this document:

```bash
python - <<'PY'
import math, pathlib, re
text = pathlib.Path('src/parser/openmp.rs').read_text()
block = re.search(r"openmp_directives!\s*{(.*?)}\s*\n\npub fn clause_registry", text, re.S).group(1)
directives = sorted({re.search(r'"([^"]+)"', line).group(1)
                      for line in block.splitlines() if '"' in line})
clauses_block = re.search(r"openmp_clauses!\s*{(.*?)}\s*\n\nmacro_rules!", text, re.S).group(1)
clauses = sorted({re.search(r'name: "([^"]+)"', line).group(1)
                   for line in clauses_block.splitlines() if 'name:' in line})

def make_table(items, columns=4):
    rows = math.ceil(len(items) / columns)
    table = ['| ' + ' | '.join([''] * columns) + ' |', '| ' + ' | '.join(['---'] * columns) + ' |']
    for r in range(rows):
        row = []
        for c in range(columns):
            idx = c * rows + r
            row.append(f"`{items[idx]}`" if idx < len(items) else '')
        table.append('| ' + ' | '.join(row) + ' |')
    return '\n'.join(table)

print(make_table(directives))
print('\n')
print(make_table(clauses))
PY
```

Keeping this document machine-derived guarantees it matches the parser at all
times.
