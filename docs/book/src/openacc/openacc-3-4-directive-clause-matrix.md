# OpenACC 3.4 Directive–Clause Matrix

This matrix cross-references every OpenACC 3.4 directive with its allowed clauses and enumerates the clause-level modifiers and arguments. Section numbers and page references point back to the canonical OpenACC 3.4 PDF so that every entry can be validated directly against the specification. Use this document together with the directive/clauses index and the restrictions digest to obtain a complete, single-source view of the standard.

## Directive coverage

### Parallel construct (§2.5.1, p.33)
- `async [(async-argument)]` — asynchronous queue selection; semantics defined in §2.16.1 (p.99) with async-argument rules in §2.16 (p.98).
- `wait [(wait-argument)]` — queue synchronization; wait-argument syntax in §2.16 (p.99) and clause behavior in §2.16.2 (p.100).
- `num_gangs(int-expr-list)` — up to three gang dimensions (missing entries default to 1) for parallel regions; details in §2.5.10 (p.37).
- `num_workers(int-expr)` — worker count per gang (§2.5.11, p.38).
- `vector_length(int-expr)` — vector lane count per worker (§2.5.12, p.38).
- `device_type(device-type-list)` — device-specific clause selection (§2.4, p.31).
- `if(condition)` — host vs device execution control (§2.5.6, p.37).
- `self[(condition)]` — execute region on host without moving data (§2.5.7, p.37).
- `reduction(operator:var-list)` — reduction variables imply copy semantics (§2.5.15, p.39).
- Data clauses `copy`, `copyin`, `copyout`, `create`, `no_create`, `present`, `present_or_copy`, `present_or_copyin`, `present_or_copyout`, `present_or_create`, `deviceptr`, and `attach` each accept optional modifier lists from §2.7.4 (p.52) and actions defined in §§2.7.1–2.7.14 (pp.48–60).
- `private(var-list)` — private instances per gang (§2.5.13, p.38).
- `firstprivate(var-list)` — initialize privates from host values (§2.5.14, p.38).
- `default(none|present)` — default data scoping (§2.5.16, p.40).

### Serial construct (§2.5.2, p.34)
- Permits the same clauses as the parallel construct except that `num_gangs`, `num_workers`, and `vector_length` are forbidden (§2.5.2, p.34). Other clause semantics match the sections cited above.

### Kernels construct (§2.5.3, p.35)
- `async[(async-argument)]` and `wait[(wait-argument)]` per §§2.16.1–2.16.2 (pp.99–100).
- `num_gangs(int-expr)` — single argument specifying gangs per kernel (§2.5.10, p.37).
- `num_workers(int-expr)` and `vector_length(int-expr)` as in §§2.5.11–2.5.12 (p.38).
- `device_type`, `if`, `self`, and all data clauses (`copy`, `copyin`, `copyout`, `create`, `no_create`, `present`, `present_or_copy`, `present_or_copyin`, `present_or_copyout`, `present_or_create`, `deviceptr`, `attach`) with modifiers per §§2.4 and 2.7.
- `default(none|present)` per §2.5.16 (p.40).

### Data construct (§2.6.5, p.43)
- `if(condition)` for conditional region creation (§2.6.5, p.43).
- `async[(async-argument)]` and `wait[(wait-argument)]` per §§2.16.1–2.16.2 (pp.99–100).
- `device_type(device-type-list)` per §2.4 (p.31).
- Data movement clauses `copy`, `copyin`, `copyout`, `create`, `no_create`, `present`, `present_or_copy`, `present_or_copyin`, `present_or_copyout`, `present_or_create`, `deviceptr`, `attach` with modifier lists from §2.7.4 (p.52) and semantics in §§2.7.1–2.7.14 (pp.48–60).
- `default(none|present)` (treated as in §2.5.16, p.40).

### Enter data directive (§2.6.6, p.45)
- `if(condition)` optional guard (§2.6.6, p.45).
- `async[(async-argument)]` and `wait[(wait-argument)]` per §§2.16.1–2.16.2 (pp.99–100).
- `copyin([modifier-list:]var-list)`, `present_or_copyin([modifier-list:]var-list)`, `present_or_create([modifier-list:]var-list)`, `create([modifier-list:]var-list)`, and `attach(var-list)` with data clause modifiers from §2.7.4 (p.52).

### Exit data directive (§2.6.6, p.45)
- `if(condition)`, `async[(async-argument)]`, `wait[(wait-argument)]` as above.
- `copyout([modifier-list:]var-list)`, `present_or_copyout([modifier-list:]var-list)`, `delete(var-list)`, `detach(var-list)` with modifiers from §2.7.4 (p.52) and clause semantics in §§2.7.9–2.7.14 (pp.56–60).
- `finalize` — forces dynamic reference counters to zero (§2.6.6, p.46).

### Host_data construct (§2.8, p.62)
- `use_device(var-list)` — maps host pointers to device addresses (§2.8.1, p.63).
- `if(condition)` and `if_present` clauses (§§2.8.2–2.8.3, p.63).

### Loop construct (§2.9, p.64)
- `collapse([force:]n)` — loop nest collapsing with optional `force` qualifier (§2.9.1, p.65).
- `gang[(gang-arg-list)]` — optional `num:`, `dim:`, and `static:` modifiers per §2.9.2 (pp.66–67).
- `worker[( [num:]int-expr )]` (§2.9.3, p.68).
- `vector[( [length:]int-expr )]` (§2.9.4, p.68).
- `seq`, `independent`, and `auto` exclusivity rules in §§2.9.5–2.9.7 (pp.68–69).
- `tile(size-expr-list)` with optional `*` entries (§2.9.8, p.69).
- `device_type(device-type-list)` per §2.9.9 (p.70).
- `private(var-list)` (§2.9.10, p.70) and `reduction(operator:var-list)` (§2.9.11, p.71).

### Cache directive (§2.10, p.75)
- `cache([readonly:]var-list)` — optional `readonly` modifier constrains writes (§2.10, p.75).

### Combined constructs (§2.11, p.75)
- `parallel loop`, `serial loop`, and `kernels loop` accept any clause allowed on both the outer construct and the loop construct; reductions imply `copy` semantics (§2.11, pp.75–76).

### Atomic construct (§2.12, pp.77–80)
- Optional `atomic-clause` of `read`, `write`, `update`, or `capture`; Fortran syntax variants follow §2.12 (pp.77–80).
- Optional `if(condition)` clause (§2.12, p.77).

### Declare directive (§2.13, pp.81–84)
- Data clauses `copy`, `copyin`, `copyout`, `create`, `present`, `present_or_copy`, `present_or_copyin`, `present_or_copyout`, `present_or_create`, `deviceptr` as in §2.13 (pp.82–83).
- `device_resident(var-list)` (§2.13.1, p.82).
- `link(var-list)` for static linkage of device allocations (§2.13.3, p.84).

### Init directive (§2.14.1, p.84)
- `device_type(device-type-list)` and `device_num(int-expr)` to select targets (§2.14.1, p.84).
- Optional `if(condition)` guard (§2.14.1, p.84).

### Shutdown directive (§2.14.2, p.85)
- Same clause set as `init`: `device_type`, `device_num`, and optional `if(condition)` (§2.14.2, p.85).

### Set directive (§2.14.3, p.87)
- `default_async(async-argument)` — sets the default queue (§2.14.3, p.87).
- `device_num(int-expr)` and `device_type(device-type-list)` adjust internal control variables (§2.14.3, p.87).
- Optional `if(condition)` (§2.14.3, p.87).

### Update directive (§2.14.4, p.88)
- `async[(async-argument)]`, `wait[(wait-argument)]`, `device_type(device-type-list)`, and `if(condition)` as above.
- `if_present` skip modifier (§2.14.4, p.89).
- Data movement clauses `self(var-list)`, `host(var-list)`, `device(var-list)` with semantics in §2.14.4 (pp.88–89).

### Wait directive (§2.16.3, p.100; see also §2.14.5)
- Optional `wait-argument` tuple `[devnum:int-expr:][queues:]async-argument-list` per §2.16 (p.99).
- Optional `async[(async-argument)]` to queue the wait (§2.16.3, p.100).
- Optional `if(condition)` (§2.16.3, p.100).

### Routine directive (§2.15.1, pp.91–97)
- Parallelism clauses `gang[(dim:int-expr)]`, `worker`, `vector`, and `seq` define callable levels (§2.15.1, pp.91–93).
- `bind(name|string)` for device linkage (§2.15.1, pp.93–94).
- `device_type(device-type-list)` for specialization (§2.15.1, pp.94–95).
- `nohost` to omit host compilation (§2.15.1, pp.94–95).

### Do concurrent integration (§2.17.2, p.102)
- When combined with loop constructs, `local`, `local_init`, `shared`, and `default(none)` locality specs map to `private`, `firstprivate`, `copy`, and `default(none)` clauses on the enclosing compute construct (§2.17.2, p.102).

## Clause reference

### Device-specific clause (§2.4, pp.31–33)
- `device_type(device-type-list)` partitions clause lists by architecture name or `*`; default clauses apply when no device-specific override exists (§2.4, pp.31–33).
- Abbreviation `dtype` is permitted (§2.4, p.31).
- Device-specific clauses are limited per directive as documented in each directive section.

### if clause (§§2.5.6 & 2.8.2, p.37 & p.63)
- Compute constructs: true runs on the device; false reverts to host execution (§2.5.6, p.37).
- Host_data: governs creation of device pointer aliases (§2.8.2, p.63).
- Enter/exit/update data: conditional data movement (§2.6.6, p.45; §2.14.4, p.88).

### self clause (§§2.5.7 & 2.14.4, pp.37 & 88)
- On compute constructs, `self[(condition)]` forces host execution when true (§2.5.7, p.37).
- On update, `self(var-list)` copies from device to host for uncaptured data (§2.14.4, p.88).

### async clause (§2.16.1, p.99)
- Allowed on parallel, serial, kernels, data constructs, enter/exit data, update, and wait directives (§2.16.1, p.99).
- `async-argument` values: nonnegative integers or `acc_async_default`, `acc_async_noval`, `acc_async_sync` (§2.16, p.98).
- Missing clause implies synchronous execution; empty argument implies `acc_async_noval` (§2.16.1, p.99).

### wait clause (§2.16.2, p.100)
- Accepts the `wait-argument` tuple defined in §2.16 (p.99).
- Without arguments waits on all queues of the current device; with arguments delays launch until specified queues drain (§2.16.2, p.100).

### num_gangs clause (§2.5.10, p.37)
- Parallel construct: up to three integers for gang dimensions; defaults to 1 when omitted (§2.5.10, p.37).
- Kernels construct: single argument per generated kernel (§2.5.10, p.37).
- Implementations may cap values based on device limits (§2.5.10, p.37).

### num_workers clause (§2.5.11, p.38)
- Sets workers per gang; unspecified defaults are implementation-defined (§2.5.11, p.38).

### vector_length clause (§2.5.12, p.38)
- Sets vector lanes per worker; unspecified defaults are implementation-defined (§2.5.12, p.38).

### private clause (§§2.5.13 & 2.9.10, pp.38 & 70)
- Compute constructs: allocate private copies for gang members (§2.5.13, p.38).
- Loop constructs: each iteration gets a private copy; allowed only where clause lists permit (§2.9.10, p.70).

### firstprivate clause (§2.5.14, p.38)
- Initializes private variables from original values at region entry (§2.5.14, p.38).

### reduction clause (§§2.5.15 & 2.9.11, pp.39 & 71)
- Supports operators `+`, `*`, `max`, `min`, bitwise ops, logical ops, and Fortran `iand/ior/ieor` with initialization table specified in §2.5.15 (pp.39–40).
- Applies element-wise to arrays/subarrays; implies appropriate data clauses (§2.5.15, p.39).
- Loop reductions follow §2.9.11 (pp.71–72).

### default clause (§2.5.16, p.40)
- `default(none)` requires explicit data clauses; `default(present)` asserts device presence (§2.5.16, p.40).

### Data clause framework (§§2.7–2.7.4, pp.48–53)
- Data specification syntax in §2.7.1 (pp.48–49).
- Data actions (`copy`, `create`, `delete`, etc.) in §2.7.2 (pp.50–52).
- Error conditions in §2.7.3 (p.52).
- Modifier list tokens: `always`, `alwaysin`, `alwaysout`, `capture`, `readonly`, `zero` (§2.7.4, p.52).

### deviceptr clause (§2.7.5, p.53)
- Treats variables as preallocated device pointers; disallows conflicting data actions (§2.7.5, p.53).

### present clause (§2.7.6, p.53)
- Requires data to exist on the device; raises errors otherwise (§2.7.6, p.53).

### copy/copyin/copyout clauses (§§2.7.7–2.7.9, pp.54–56)
- `copy` performs in/out transfers; `copyin` is host→device; `copyout` is device→host (§§2.7.7–2.7.9, pp.54–56).
- Respect modifier semantics from §2.7.4.

### create clause (§§2.7.10 & 2.13.2, pp.57 & 83)
- Allocates device storage without transfer (§2.7.10, p.57); declare directive variant described in §2.13.2 (p.83).

### no_create clause (§2.7.11, p.57)
- Asserts that data already exists on device; no allocation occurs (§2.7.11, p.57).

### delete clause (§2.7.12, p.58)
- Deallocates device storage at region exit (§2.7.12, p.58).

### attach/detach clauses (§§2.7.13–2.7.14, pp.59–60)
- Manage pointer attachments to device memory (§§2.7.13–2.7.14, pp.59–60).

### use_device clause (§2.8.1, p.63)
- Temporarily remaps host pointers to device addresses within host_data regions (§2.8.1, p.63).

### if_present clause (§§2.8.3 & 2.14.4, pp.63 & 89)
- Skips operations when data is absent on the device (§2.8.3, p.63; §2.14.4, p.89).

### collapse clause (§2.9.1, p.65)
- Optional `force` keyword overrides dependency analysis; requires positive iteration counts (§2.9.1, p.65).

### gang clause (§2.9.2, pp.66–67)
- `gang-arg-list` allows one each of `num:`, `dim:`, `static:` modifiers; `dim` is limited to 1–3 (§2.9.2, pp.66–67).

### worker clause (§2.9.3, p.68)
- Optional `num:` argument; interacts with compute scopes as described in §2.9.3 (p.68).

### vector clause (§2.9.4, p.68)
- Optional `length:` argument; selects vector mode (§2.9.4, p.68).

### seq clause (§2.9.5, p.68)
- Forces sequential execution of the associated loop (§2.9.5, p.68).

### independent clause (§2.9.6, p.69)
- Asserts absence of cross-iteration dependencies (§2.9.6, p.69).

### auto clause (§2.9.7, p.69)
- Delegates loop scheduling to implementation; interacts with routine clause inference (§2.9.7, p.69).

### tile clause (§2.9.8, p.69)
- Breaks iteration space into tile sizes; `*` uses runtime-determined tile length (§2.9.8, p.69).

### device_type clause on loops (§2.9.9, p.70)
- Restricts subsequent clauses to specified device types (§2.9.9, p.70).

### device_resident clause (§2.13.1, pp.82–83)
- Forces static device allocation with reference counting rules (§2.13.1, pp.82–83).

### link clause (§2.13.3, p.84)
- Creates persistent device linkages for large host data (§2.13.3, pp.83–84).

### bind clause (§2.15.1, pp.93–94)
- Sets alternate device symbol name (identifier or string) (§2.15.1, pp.93–94).

### device_num and default_async clauses (§2.14.3, p.87)
- Modify internal control variables `acc-current-device-num-var` and `acc-default-async-var` (§2.14.3, p.87).

### nohost clause (§2.15.1, pp.94–95)
- Suppresses host code generation for routines; cascades to dependent procedures (§2.15.1, pp.94–95).

### finalize clause (§2.6.6, p.46)
- Available on `exit data`; zeroes dynamic and attachment counters (§2.6.6, p.46).

### wait-argument modifiers (§2.16, p.99)
- `devnum:int-expr:` selects device; optional `queues:` prefix clarifies async argument list (§2.16, p.99).

### async-value semantics (§2.16, p.98)
- Maps async arguments to queue identifiers; `acc_async_sync` enforces synchronous completion, `acc_async_noval` uses default queue (§2.16, p.98).

