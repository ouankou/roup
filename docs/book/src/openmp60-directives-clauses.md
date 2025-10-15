# OpenMP 6.0 Directives and Clauses

This comprehensive reference catalogue documents **all** OpenMP 6.0 keywords from the [OpenMP Application Programming Interface Version 6.0](https://www.openmp.org/wp-content/uploads/OpenMP-API-Specification-6-0.pdf) specification.

## Purpose

This document serves as a complete keyword inventory for development and reference. Each entry includes:
- Specification section and page numbers
- Categorization and properties
- No duplication - each keyword appears once

## Coverage

- **66 Directives/Constructs** - All executable, declarative, and meta directives
- **125 Clauses** - All clause keywords
- **Modifiers** - Map-type, dependence-type, schedule, and other modifiers
- **Keywords & Values** - Memory orders, atomic operations, schedule types, allocators, and more
- **Reduction Operators** - All supported reduction operations
- **Special Identifiers** - Reserved locators, device identifiers, and constants

## Directives and Constructs

- `allocate` (Section 8.5; pp. 341–342; category: declarative; association: explicit; properties: pure)
- `allocators` (Section 8.7; category: executable; association: block : allocator; properties: default)
- `assume` (Section 10.6.3; category: informational; association: block; properties: pure)
- `assumes` (Section 10.6.2; p. 399; category: informational; association: unassociated; properties: pure)
- `atomic` (Section 17.8.5; pp. 525–528; category: executable; association: block : atomic; properties: mutual-exclusion, order-concurrent-nestable, simdizable)
- `barrier` (Section 17.3.1; pp. 506–508; category: executable; association: unassociated; properties: team-executed)
- `begin assumes` (Section 10.6.4; p. 399; category: informational; association: delimited; properties: default)
- `begin declare_target` (Section 9.9.2; p. 380; category: declarative; association: delimited; properties: declare-target, device, variant-generating)
- `begin declare_variant` (Section 9.6.5; p. 367; category: declarative; association: delimited; properties: default)
- `cancel` (Section 18.2; pp. 551–554; category: executable; association: unassociated; properties: default)
- `cancellation_point` (Section 18.3; pp. 555–556; category: executable; association: unassociated; properties: default)
- `critical` (Section 17.2; pp. 504–505; category: executable; association: block; properties: mutual-exclusion, thread-limiting, thread-exclusive)
- `declare_induction` (Section 7.6.17; pp. 294–295; category: declarative; association: unassociated; properties: pure)
- `declare_mapper` (Section 7.9.10; pp. 324–327; category: declarative; association: unassociated; properties: pure)
- `declare_reduction` (Section 7.6.14; pp. 291–292; category: declarative; association: unassociated; properties: pure)
- `declare_simd` (Section 9.8; pp. 372–373; category: declarative; association: declaration; properties: pure, variant-generating)
- `declare_target` (Section 9.9.1; pp. 377–379; category: declarative; association: explicit; properties: declare-target, device, pure, variant-generating)
- `declare_variant` (Section 9.6.4; pp. 365–366; category: declarative; association: declaration; properties: pure)
- `depobj` (Section 17.9.3; p. 536; category: executable; association: unassociated; properties: default)
- `dispatch` (Section 9.7; pp. 368–369; category: executable; association: block : function-dispatch; properties: context-matching)
- `distribute` (Section 13.7; pp. 451–452; category: executable; association: loop nest; properties: SIMD-partitionable, teams-nestable, work-distribution, partitioned)
- `do` (Section 13.6.2; p. 448; category: executable; association: loop nest; properties: work-distribution, team-executed, partitioned, SIMD-partitionable, worksharing, worksharing-loop, cancellable, context-matching)
- `error` (Section 10.1; p. 383; category: utility; association: unassociated; properties: pure)
- `flush` (Section 17.8.6; pp. 529–535; category: executable; association: unassociated; properties: default)
- `for` (Section 13.6.1; p. 447; category: executable; association: loop nest; properties: work-distribution, team-executed, partitioned, SIMD-partitionable, worksharing, worksharing-loop, cancellable, context-matching)
- `fuse` (Section 11.3; p. 405; category: executable; association: loop sequence; properties: loop-transforming, order-concurrent-nestable, pure, simdizable, teams-nestable)
- `groupprivate` (Section 7.13; pp. 332–333; category: declarative; association: explicit; properties: pure)
- `interchange` (Section 11.4; p. 406; category: executable; association: loop nest; properties: loop-transforming, nonrectangular-compatible, order-concurrent-nestable, pure, simdizable, teams-nestable)
- `interop` (Section 16.1; p. 499; category: executable; association: unassociated; properties: device)
- `loop` (Section 13.8; p. 454; category: executable; association: loop nest; properties: order-concurrent-nestable, partitioned, simdizable, team-executed, teams-nestable, work-distribution, worksharing)
- `masked` (Section 12.5; p. 433; category: executable; association: block; properties: thread-limiting, thread-selecting)
- `begin metadirective` (Section 9.4.4; p. 327; category: meta; association: delimited; properties: pure)
- `metadirective` (Section 9.4.3; p. 327; category: meta; association: unassociated; properties: pure)
- `nothing` (Section 10.7; pp. 400–402; category: utility; association: unassociated; properties: pure, loop-transforming)
- `ordered` (Section 17.10.2; pp. 546–547; category: executable; association: block; properties: mutual-exclusion, simdizable, thread-limiting, thread-exclusive)
- `parallel` (Section 12.1; pp. 415–418; category: executable; association: block; properties: cancellable, context-matching, order-concurrent-nestable, parallelism-generating, team-generating, teams-nestable, thread-limiting)
- `requires` (Section 10.5; p. 386; category: informational; association: unassociated; properties: default)
- `reverse` (Section 11.5; p. 407; category: executable; association: loop nest; properties: generally-composable, loop-transforming, order-concurrent-nestable, pure, simdizable, teams-nestable)
- `scan` (Section 7.7; pp. 297–299; category: subsidiary; association: separating; properties: pure)
- `scope` (Section 13.2; p. 437; category: executable; association: block; properties: work-distribution, team-executed, worksharing, thread-limiting)
- `section` (Section 13.3.1; p. 439; category: subsidiary; association: separating; properties: default)
- `sections` (Section 13.3; p. 438; category: executable; association: block; properties: work-distribution, team-executed, partitioned, worksharing, thread-limiting, cancellable)
- `simd` (Section 12.4; p. 430; category: executable; association: loop nest; properties: context-matching, order-concurrent-nestable, parallelism-generating, pure, simdizable)
- `single` (Section 13.1; p. 436; category: executable; association: block; properties: work-distribution, team-executed, partitioned, worksharing, thread-limiting, thread-selecting)
- `split` (Section 11.6; p. 408; category: executable; association: loop nest; properties: generally-composable, loop-transforming, order-concurrent-nestable, pure, simdizable, teams-nestable)
- `stripe` (Section 11.7; p. 410; category: executable; association: loop nest; properties: loop-transforming, order-concurrent-nestable, pure, simdizable, teams-nestable)
- `target` (Section 15.8; pp. 491–495; category: executable; association: block; properties: parallelism-generating, team-generating, thread-limiting, exception-aborting, task-generating, device, device-affecting, data-mapping, map-entering, map-exiting, context-matching)
- `target_data` (Section 15.7; pp. 489–490; category: executable; association: block; properties: device, device-affecting, data-mapping, map-entering, map-exiting, parallelism-generating, sharing-task, task-generating)
- `target_enter_data` (Section 15.5; pp. 485–486; category: executable; association: unassociated; properties: parallelism-generating, task-generating, device, device-affecting, data-mapping, map-entering)
- `target_exit_data` (Section 15.6; pp. 487–488; category: executable; association: unassociated; properties: parallelism-generating, task-generating, device, device-affecting, data-mapping, map-exiting)
- `target_update` (Section 15.9; pp. 496–498; category: executable; association: unassociated; properties: parallelism-generating, task-generating, device, device-affecting)
- `task` (Section 14.1; pp. 457–459; category: executable; association: block; properties: parallelism-generating, thread-limiting, task-generating)
- `task_iteration` (Section 14.2.3; p. 465; category: subsidiary; association: unassociated; properties: default)
- `taskgraph` (Section 14.3; pp. 466–468; category: executable; association: block; properties: default)
- `taskgroup` (Section 17.4; p. 509; category: executable; association: block; properties: cancellable)
- `taskloop` (Section 14.2; pp. 460–462; category: executable; association: loop nest; properties: parallelism-generating, SIMD-partitionable, task-generating)
- `taskwait` (Section 17.5; pp. 510–511; category: executable; association: unassociated; properties: default)
- `taskyield` (Section 14.12; pp. 477–480; category: executable; association: unassociated; properties: default)
- `teams` (Section 12.2; pp. 425–427; category: executable; association: block; properties: parallelism-generating, team-generating, thread-limiting, context-matching)
- `threadprivate` (Section 7.3; pp. 246–253; category: declarative; association: explicit; properties: pure)
- `tile` (Section 11.8; p. 411; category: executable; association: loop nest; properties: loop-transforming, order-concurrent-nestable, pure, simdizable, teams-nestable)
- `unroll` (Section 11.9; p. 412; category: executable; association: loop nest; properties: generally-composable, loop-transforming, order-concurrent-nestable, pure, simdizable, teams-nestable)
- `workdistribute` (Section 13.5; pp. 443–446; category: executable; association: block; properties: work-distribution, partitioned)
- `workshare` (Section 13.4; pp. 440–442; category: executable; association: block; properties: work-distribution, team-executed, partitioned, worksharing)

## Clauses

- `absent` (Section 10.6.1.1; p. 394)
- `acq_rel` (Section 17.8.1.1; p. 515)
- `acquire` (Section 17.8.1.2; p. 516)
- `adjust_args` (Section 9.6.2; pp. 362–363)
- `affinity` (Section 14.10; p. 475)
- `align` (Section 8.3; p. 340)
- `aligned` (Section 7.12; p. 331)
- `allocate` (Section 8.6; pp. 343–345)
- `allocator` (Section 8.4; p. 340)
- `append_args` (Section 9.6.3; p. 364)
- `apply` (Section 11.1; pp. 403–404)
- `at` (Section 10.2; p. 383)
- `atomic_default_mem_order` (Section 10.5.1.1; p. 387)
- `bind` (Section 13.8.1; pp. 455–456)
- `capture` (Section 17.8.3.1; p. 521)
- `collapse` (Section 6.4.5; p. 236)
- `collector` (Section 7.6.19; p. 296)
- `combiner` (Section 7.6.15; p. 292)
- `compare` (Section 17.8.3.2; p. 522)
- `contains` (Section 10.6.1.2; p. 394)
- `copyin` (Section 7.8.1; p. 302)
- `copyprivate` (Section 7.8.2; pp. 303–309)
- `counts` (Section 11.6.1; p. 409)
- `default` (Section 7.5.1; p. 254)
- `defaultmap` (Section 7.9.9; pp. 322–323)
- `depend` (Section 17.9.5; pp. 538–540)
- `destroy` (Section 5.7; pp. 213–235)
- `detach` (Section 14.11; p. 476)
- `device` (Section 15.2; p. 482)
- `device_safesync` (Section 10.5.1.7; p. 393; properties: required, unique Members:)
- `device_type` (Section 15.1; p. 481)
- `dist_schedule` (Section 13.7.1; p. 453)
- `doacross` (Section 17.9.7; pp. 542–544)
- `dynamic_allocators` (Section 10.5.1.2; p. 388)
- `enter` (Section 7.9.7; p. 320)
- `exclusive` (Section 7.7.2; p. 300)
- `fail` (Section 17.8.3.3; p. 522)
- `filter` (Section 12.5.1; pp. 434–435)
- `final` (Section 14.7; p. 472)
- `firstprivate` (Section 7.5.4; pp. 258–259)
- `from` (Section 7.10.2; p. 329)
- `full` (Section 11.9.1; p. 413)
- `grainsize` (Section 14.2.1; p. 463)
- `graph_id` (Section 14.3.1; p. 468)
- `graph_reset` (Section 14.3.2; p. 469)
- `has_device_addr` (Section 7.5.9; p. 268)
- `hint` (Section 17.1; p. 503)
- `holds` (Section 10.6.1.3; p. 395)
- `if` (Section 5.5; p. 210)
- `in_reduction` (Section 7.6.12; p. 287)
- `inbranch` (Section 9.8.1.1; p. 374)
- `inclusive` (Section 7.7.1; p. 299)
- `indirect` (Section 9.9.3; pp. 381–382)
- `induction` (Section 7.6.13; pp. 288–290)
- `inductor` (Section 7.6.18; p. 296)
- `init` (Section 5.6; pp. 211–212)
- `init_complete` (Section 7.7.3; p. 301)
- `initializer` (Section 7.6.16; p. 293)
- `interop` (Section 9.7.1; p. 370)
- `is_device_ptr` (Section 7.5.7; p. 266)
- `lastprivate` (Section 7.5.5; pp. 260–262)
- `linear` (Section 7.5.6; pp. 263–265)
- `link` (Section 7.9.8; p. 321)
- `local` (Section 7.14; pp. 334–339)
- `looprange` (Section 6.4.7; pp. 238–245)
- `map` (Section 7.9.6; pp. 310–319)
- `match` (Section 9.6.1; p. 361)
- `memscope` (Section 17.8.4; p. 524)
- `mergeable` (Section 14.5; p. 470)
- `message` (Section 10.3; p. 384)
- `no_openmp` (Section 10.6.1.4; p. 396)
- `no_openmp_constructs` (Section 10.6.1.5; p. 396)
- `no_openmp_routines` (Section 10.6.1.6; p. 397)
- `no_parallelism` (Section 10.6.1.7; p. 398)
- `nocontext` (Section 9.7.3; p. 371)
- `nogroup` (Section 17.7; p. 514; properties: exclusive, unique Members:)
- `nontemporal` (Section 12.4.1; p. 431)
- `notinbranch` (Section 9.8.1.2; pp. 375–376)
- `novariants` (Section 9.7.2; p. 370)
- `nowait` (Section 17.6; pp. 512–513)
- `num_tasks` (Section 14.2.2; p. 464)
- `num_teams` (Section 12.2.1; p. 427)
- `num_threads` (Section 12.1.2; pp. 419–422)
- `order` (Section 12.3; pp. 428–429)
- `ordered` (Section 6.4.6; p. 237)
- `otherwise` (Section 9.4.2; pp. 357–360; properties: pure)
- `partial` (Section 11.9.2; p. 414)
- `permutation` (Section 11.4.1; p. 407)
- `priority` (Section 14.9; p. 474)
- `private` (Section 7.5.3; pp. 256–257)
- `proc_bind` (Section 12.1.4; p. 423)
- `read` (Section 17.8.2.1; p. 519)
- `reduction` (Section 7.6.10; pp. 283–285)
- `relaxed` (Section 17.8.1.3; p. 516)
- `release` (Section 17.8.1.4; p. 517)
- `replayable` (Section 14.6; p. 471)
- `reverse_offload` (Section 10.5.1.3; p. 389)
- `safelen` (Section 12.4.2; p. 431)
- `safesync` (Section 12.1.5; p. 424)
- `schedule` (Section 13.6.3; pp. 449–450)
- `self_maps` (Section 10.5.1.6; p. 392)
- `seq_cst` (Section 17.8.1.5; p. 518; properties: exclusive, unique Members:)
- `severity` (Section 10.4; p. 385)
- `shared` (Section 7.5.2; p. 255)
- `simd` (Section 17.10.3.2; pp. 549–550; properties: exclusive, required, unique Members:)
- `simdlen` (Section 12.4.3; p. 432)
- `sizes` (Section 11.2; p. 404)
- `task_reduction` (Section 7.6.11; p. 286)
- `thread_limit` (Section 15.3; pp. 483–484)
- `threads` (Section 17.10.3.1; p. 548)
- `threadset` (Section 14.8; p. 473)
- `to` (Section 7.10.1; p. 328)
- `transparent` (Section 17.9.6; p. 541)
- `unified_address` (Section 10.5.1.4; p. 390)
- `unified_shared_memory` (Section 10.5.1.5; p. 391)
- `uniform` (Section 7.11; p. 330)
- `untied` (Section 14.4; p. 470)
- `update` (Section 17.9.4; p. 537)
- `use` (Section 16.1.2; pp. 500–502)
- `use_device_addr` (Section 7.5.10; pp. 269–282)
- `use_device_ptr` (Section 7.5.8; p. 267)
- `uses_allocators` (Section 8.8; pp. 346–355)
- `weak` (Section 17.8.3.4; p. 523)
- `when` (Section 9.4.1; p. 356)
- `write` (Section 17.8.2.3; p. 520; properties: unique Members:)

## Modifiers

Modifiers are keywords that modify the behavior of clauses. They appear as part of clause syntax to refine clause semantics.

### Map-Type Modifiers

- `storage` (Section 7.9.1; p. 305; map-type modifier; default value)
- `to` (Section 7.9.1; p. 305; map-type modifier; map-entering, assigning)
- `from` (Section 7.9.1; p. 305; map-type modifier; map-exiting, assigning)
- `tofrom` (Section 7.9.1; p. 305; map-type modifier; map-entering, map-exiting, assigning)
- `alloc` (Section 7.9.1; p. 305; map-type modifier; alias for storage on map-entering constructs)
- `release` (Section 7.9.1; p. 305; map-type modifier; alias for storage on map-exiting constructs)
- `delete` (Section 7.9.1; p. 305; map-type modifier; used with delete-modifier)

### Task-Dependence-Type Modifiers

- `in` (Section 17.9.1; p. 535; task-dependence-type modifier; input dependence)
- `out` (Section 17.9.1; p. 535; task-dependence-type modifier; output dependence)
- `inout` (Section 17.9.1; p. 535; task-dependence-type modifier; input-output dependence)
- `inoutset` (Section 17.9.1; p. 535; task-dependence-type modifier; inout with set semantics)
- `mutexinoutset` (Section 17.9.1; p. 535; task-dependence-type modifier; mutual exclusion inout)
- `depobj` (Section 17.9.1; p. 535; task-dependence-type modifier; depend object)

### Schedule Modifiers

- `monotonic` (Section 13.6.3; p. 449; ordering-modifier for schedule clause)
- `nonmonotonic` (Section 13.6.3; p. 449; ordering-modifier for schedule clause)
- `simd` (Section 13.6.3; p. 449; chunk-modifier for schedule clause)

### Reduction and Induction Modifiers

- `reduction-identifier` (Section 7.6.9; p. 282; modifier specifying reduction operation)
- `iterator` (Section 5.2.6; p. 200; modifier for creating iterator expressions)

### Other Clause Modifiers

- `ref` (Section 7.9.5; ref-modifier for map clause; indicates referencing variable)
- `mapper` (Section 7.9.4; mapper-modifier for map clause; specifies custom mapper)
- `allocator-simple-modifier` (Section 8.6; allocator modifier; simple allocator specification)
- `allocator-complex-modifier` (Section 8.6; allocator modifier; complex allocator specification)
- `prefer-type` (Section 16.1.3; p. 501; prefer-type modifier for interop clause)
- `directive-name-modifier` (Section 5.4; p. 204; conditional modifier using directive name)

### Map Clause Modifiers

- `always` (Section 7.9.6; always-modifier; forces data transfer)
- `close` (Section 7.9.6; close-modifier; allocates in fastest memory)
- `present` (Section 7.9.6; present-modifier; data must be present)
- `self` (Section 7.9.6; self-modifier; for device-host data mapping)

### Lastprivate Modifiers

- `conditional` (Section 7.5.5; p. 261; modifier for lastprivate clause; conditional assignment)

### Original-Sharing Modifiers

- `original` (Section 7.6.10; original-sharing-modifier; preserves original data-sharing)

## Keywords and Values

Keywords and values used as arguments to clauses and directives.

### Memory Order Keywords

- `seq_cst` (Section 17.8.1.5; p. 518; sequentially consistent memory ordering)
- `acq_rel` (Section 17.8.1.1; p. 515; acquire-release memory ordering)
- `acquire` (Section 17.8.1.2; p. 516; acquire memory ordering)
- `release` (Section 17.8.1.4; p. 517; release memory ordering)
- `relaxed` (Section 17.8.1.3; p. 516; relaxed memory ordering)

### Atomic Operation Keywords

- `read` (Section 17.8.2.1; p. 519; atomic read operation)
- `write` (Section 17.8.2.3; p. 520; atomic write operation)
- `update` (Section 17.8.2.2; atomic update operation)
- `capture` (Section 17.8.3.1; p. 521; atomic capture operation)
- `compare` (Section 17.8.3.2; p. 522; atomic compare operation)
- `weak` (Section 17.8.3.4; p. 523; weak compare semantics)
- `fail` (Section 17.8.3.3; fail memory order for atomic compare)

### Schedule Types

- `static` (Section 13.6.3; p. 449; static loop schedule)
- `dynamic` (Section 13.6.3; p. 449; dynamic loop schedule)
- `guided` (Section 13.6.3; p. 449; guided loop schedule)
- `auto` (Section 13.6.3; p. 449; implementation-defined loop schedule)
- `runtime` (Section 13.6.3; p. 449; runtime-determined loop schedule)

### Proc_bind Values

- `primary` (Section 12.1.4; p. 423; bind to primary thread's place)
- `close` (Section 12.1.4; p. 423; bind close to parent thread)
- `spread` (Section 12.1.4; p. 423; spread across places)
- `master` (deprecated; use `primary` instead)

### Order Values

- `concurrent` (Section 12.3; p. 428; concurrent execution ordering)
- `reproducible` (Section 12.3; p. 428; reproducible ordering)
- `unconstrained` (Section 12.3; p. 428; unconstrained ordering)

### Device-Type Values

- `host` (Section 15.1; p. 481; host device type)
- `nohost` (Section 15.1; p. 481; non-host device type)
- `any` (Section 15.1; p. 481; any device type)

### Bind Values

- `thread` (Section 13.8.1; p. 455; bind to thread)
- `parallel` (Section 13.8.1; p. 455; bind to parallel region)
- `teams` (Section 13.8.1; p. 455; bind to teams region)

### Default Values

- `shared` (Section 7.5.1; p. 254; default shared data-sharing)
- `private` (Section 7.5.1; p. 254; default private data-sharing)
- `firstprivate` (Section 7.5.1; p. 254; default firstprivate data-sharing)
- `none` (Section 7.5.1; p. 254; no default data-sharing)

### Error Directive Values

- `compilation` (Section 10.2; at clause value; error at compilation time)
- `execution` (Section 10.2; at clause value; error at execution time)
- `fatal` (Section 10.4; p. 385; severity clause value; fatal error)
- `warning` (Section 10.4; p. 385; severity clause value; warning)

### Defaultmap Values

- `alloc` (Section 7.9.9; p. 322; defaultmap behavior)
- `to` (Section 7.9.9; p. 322; defaultmap behavior)
- `from` (Section 7.9.9; p. 322; defaultmap behavior)
- `tofrom` (Section 7.9.9; p. 322; defaultmap behavior)
- `firstprivate` (Section 7.9.9; p. 322; defaultmap behavior)
- `none` (Section 7.9.9; p. 322; defaultmap behavior)
- `default` (Section 7.9.9; p. 322; defaultmap behavior)
- `present` (Section 7.9.9; p. 322; defaultmap behavior)

### Variable Categories (for defaultmap)

- `scalar` (Section 7.9.9; p. 322; scalar variable category)
- `aggregate` (Section 7.9.9; p. 322; aggregate variable category)
- `allocatable` (Section 7.9.9; p. 322; allocatable variable category)
- `pointer` (Section 7.9.9; p. 322; pointer variable category)

## Predefined Allocators

OpenMP defines several predefined memory allocators for different memory spaces.

### Standard Allocators

- `omp_default_mem_alloc` (Section 8.2; p. 336; default memory allocator)
- `omp_large_cap_mem_alloc` (Section 8.2; p. 336; large capacity memory allocator)
- `omp_const_mem_alloc` (Section 8.2; p. 336; constant memory allocator)
- `omp_high_bw_mem_alloc` (Section 8.2; p. 336; high bandwidth memory allocator)
- `omp_low_lat_mem_alloc` (Section 8.2; p. 336; low latency memory allocator)
- `omp_cgroup_mem_alloc` (Section 8.2; p. 336; contention group memory allocator)
- `omp_pteam_mem_alloc` (Section 8.2; p. 336; parallel team memory allocator)
- `omp_thread_mem_alloc` (Section 8.2; p. 336; thread-private memory allocator)

### Special Allocator Values

- `omp_null_allocator` (Section 8.2; p. 336; null allocator value)

### Allocator Traits

- `sync_hint` (Section 8.2; p. 336; allocator trait; contended, uncontended, serialized, private)
- `alignment` (Section 8.2; p. 336; allocator trait; byte alignment)
- `access` (Section 8.2; p. 336; allocator trait; all, memspace, device, cgroup, pteam, thread)
- `pool_size` (Section 8.2; p. 336; allocator trait; pool size limit)
- `fallback` (Section 8.2; p. 336; allocator trait; default_mem_fb, null_fb, abort_fb, allocator_fb)
- `fb_data` (Section 8.2; p. 336; allocator trait; fallback allocator handle)
- `pinned` (Section 8.2; p. 336; allocator trait; true, false)
- `partition` (Section 8.2; p. 336; allocator trait; environment, nearest, blocked, interleaved, partitioner)

## Reserved Locators

Reserved locators are special OpenMP identifiers representing system storage.

- `omp_all_memory` (Section 5.2.2; p. 195; reserved locator representing all memory)

## Reduction Operators

Reduction operators used with reduction clauses.

### Arithmetic Operators

- `+` (Section 7.6.3; addition reduction)
- `-` (Section 7.6.3; subtraction reduction)
- `*` (Section 7.6.3; multiplication reduction)

### Bitwise Operators

- `&` (Section 7.6.3; bitwise AND reduction)
- `|` (Section 7.6.3; bitwise OR reduction)
- `^` (Section 7.6.3; bitwise XOR reduction)

### Logical Operators

- `&&` (Section 7.6.3; logical AND reduction)
- `||` (Section 7.6.3; logical OR reduction)

### Min/Max Operators

- `min` (Section 7.6.3; minimum reduction)
- `max` (Section 7.6.3; maximum reduction)

### Language-Specific Operators

- `.eqv.` (Fortran; Section 7.6.3; logical equivalence reduction)
- `.neqv.` (Fortran; Section 7.6.3; logical non-equivalence reduction)
- `.and.` (Fortran; Section 7.6.3; logical AND reduction)
- `.or.` (Fortran; Section 7.6.3; logical OR reduction)
- `iand` (Fortran; Section 7.6.3; bitwise AND reduction)
- `ior` (Fortran; Section 7.6.3; bitwise OR reduction)
- `ieor` (Fortran; Section 7.6.3; bitwise XOR reduction)

## Special Constants and Identifiers

### Device Identifiers

- `omp_invalid_device` (Section 15.2; invalid device number)
- `omp_initial_device` (Section 15.2; initial device identifier)

### Interop Type Identifiers

- `targetsync` (Section 16.1.2; p. 500; target synchronization interop type)
- `target` (Section 16.1.2; p. 500; target interop type)

### Doacross Keywords

- `source` (Section 17.9.7; p. 543; doacross source)
- `sink` (Section 17.9.7; p. 543; doacross sink)
- `source_omp_cur_iteration` (Section 17.9.7; p. 543; current iteration source)
