# OpenACC 3.4 Directives and Clauses

This comprehensive reference catalogue documents **all** OpenACC 3.4 keywords from the [OpenACC Application Programming Interface Version 3.4](https://www.openacc.org/sites/default/files/inline-files/OpenACC_3.4.pdf) specification.

## Purpose

This document serves as a complete keyword inventory for development and reference. Each entry includes:
- Specification section and page numbers
- Categorization and properties
- No duplication - each keyword appears once

## Coverage

- **23 Directives/Constructs** - All compute, data, loop, synchronization, declaration, and runtime directives
- **49 Clauses** - All clause keywords, including `present_or_*` data variants and runtime controls
- **Modifiers** - Data clause modifiers, gang/worker/vector modifiers, collapse modifiers
- **Special Values** - Async values, device types, default values
- **Reduction Operators** - All supported reduction operations
- **Parallelism Levels** - Gang, worker, vector, seq

## Directives and Constructs

### Compute Constructs

- `parallel` (§2.5.1; p.33; category: compute; association: block; properties: creates gang-worker-vector parallelism)
- `serial` (§2.5.2; p.34; category: compute; association: block; properties: serialized execution on device)
- `kernels` (§2.5.3; p.35; category: compute; association: block; properties: compiler-optimized kernel launch)

### Data Constructs

- `data` (§2.6.5; p.43; category: data; association: block; properties: structured data lifetime)
- `enter data` (§2.6.6; p.45; category: data; association: executable; properties: dynamic data region entry)
- `exit data` (§2.6.6; p.45; category: data; association: executable; properties: dynamic data region exit)
- `host_data` (§2.8; p.62; category: data; association: block; properties: host pointer mapping)

### Loop Constructs

- `loop` (§2.9; p.64; category: loop; association: loop nest; properties: loop parallelization)
- `parallel loop` (§2.11; p.75; category: combined; association: loop nest; properties: parallel + loop combined)
- `serial loop` (§2.11; p.75; category: combined; association: loop nest; properties: serial + loop combined)
- `kernels loop` (§2.11; p.75; category: combined; association: loop nest; properties: kernels + loop combined)

### Synchronization Constructs

- `atomic` (§2.12; p.77; category: synchronization; association: statement; properties: atomic memory operations)
- `cache` (§2.10; p.75; category: synchronization; association: loop; properties: cache hint)
- `wait` (§2.16.3; p.100; category: synchronization; association: executable; properties: async queue synchronization)

### Declaration Directives

- `declare` (§2.13; p.81; category: declarative; association: scope; properties: device data declaration)
- `routine` (§2.15.1; p.91; category: declarative; association: function; properties: device routine declaration)

### Runtime Directives

- `init` (§2.14.1; p.84; category: runtime; association: executable; properties: device initialization)
- `shutdown` (§2.14.2; p.85; category: runtime; association: executable; properties: device shutdown)
- `set` (§2.14.3; p.87; category: runtime; association: executable; properties: runtime configuration)
- `update` (§2.14.4; p.88; category: runtime; association: executable; properties: explicit data transfer)

### Special Constructs

- `do concurrent` (§2.17.2; p.102; category: integration; association: Fortran; properties: Fortran do concurrent mapping)

## Clauses

### Compute Clauses

- `if` (§2.5.6; p.37; category: conditional; applicable to: parallel, serial, kernels, host_data, atomic, init, set, update)
- `self[(condition)]` (§2.5.7; p.37; category: conditional; applicable to: parallel, serial, kernels; properties: execute on host without data movement)
- `async` (§2.5.8, §2.16.1; pp.37, 99; category: synchronization; applicable to: parallel, serial, kernels, data, enter data, exit data, update, wait)
- `wait` (§2.5.9, §2.16.2; pp.37, 100; category: synchronization; applicable to: parallel, serial, kernels, data, enter data, exit data, update)
- `num_gangs` (§2.5.10; p.37; category: parallelism; applicable to: parallel, kernels; properties: specifies number of gangs)
- `num_workers` (§2.5.11; p.38; category: parallelism; applicable to: parallel, kernels; properties: specifies workers per gang)
- `vector_length` (§2.5.12; p.38; category: parallelism; applicable to: parallel, kernels; properties: specifies vector length per worker)
- `private` (§2.5.13, §2.9.10; pp.38, 70; category: data sharing; applicable to: parallel, serial, kernels, loop)
- `firstprivate` (§2.5.14; p.38; category: data sharing; applicable to: parallel, serial, kernels; properties: initialized private variables)
- `reduction` (§2.5.15, §2.9.11; pp.39, 71; category: data sharing; applicable to: parallel, kernels, loop; properties: reduction operations)
- `default` (§2.5.16; p.40; category: data sharing; applicable to: parallel, serial, kernels, data; properties: values are none or present)

### Data Clauses

- `copy` (§2.7.7; p.54; category: data movement; properties: copy in and copy out)
- `copyin` (§2.7.8; p.55; category: data movement; properties: copy to device)
- `copyout` (§2.7.9; p.56; category: data movement; properties: copy from device)
- `create` (§2.7.10, §2.13.2; pp.57, 83; category: data allocation; properties: allocate on device)
- `no_create` (§2.7.11; p.57; category: data allocation; properties: use if present, don't create)
- `delete` (§2.7.12; p.58; category: data allocation; properties: deallocate from device)
- `present` (§2.7.6; p.53; category: data presence; properties: data must be present on device)
- `present_or_copy` (§2.7.6; p.53; category: data movement; properties: copy if not present, otherwise reuse existing device data)
- `present_or_copyin` (§2.7.6; p.53; category: data movement; properties: copy to device only when data is absent)
- `present_or_copyout` (§2.7.6; p.53; category: data movement; properties: copy from device only when data is present)
- `present_or_create` (§2.7.6; p.53; category: data allocation; properties: create device data if absent, otherwise reuse present data)
- `deviceptr` (§2.7.5; p.53; category: data presence; properties: device pointer)
- `attach` (§2.7.13; p.59; category: pointer; properties: attach pointer to device address)
- `detach` (§2.7.14; p.59; category: pointer; properties: detach pointer from device address)

### Host-Device Interaction Clauses

- `use_device` (§2.8.1; p.63; category: host access; applicable to: host_data; properties: map device pointers to host)
- `if_present` (§2.8.3; p.63; category: conditional; applicable to: host_data, update; properties: conditional on presence)

### Loop Clauses

- `collapse` (§2.9.1; p.65; category: loop transformation; applicable to: loop; properties: collapse nested loops)
- `gang` (§2.9.2; p.66; category: parallelism; applicable to: loop; properties: gang-level parallelism)
- `worker` (§2.9.3; p.68; category: parallelism; applicable to: loop; properties: worker-level parallelism)
- `vector` (§2.9.4; p.68; category: parallelism; applicable to: loop; properties: vector-level parallelism)
- `seq` (§2.9.5; p.68; category: parallelism; applicable to: loop; properties: sequential execution)
- `independent` (§2.9.6; p.69; category: parallelism; applicable to: loop; properties: loop iterations are independent)
- `auto` (§2.9.7; p.69; category: parallelism; applicable to: loop; properties: compiler decides parallelism)
- `tile` (§2.9.8; p.69; category: loop transformation; applicable to: loop; properties: tile nested loops)
- `device_type` (§2.9.9; p.70; category: device-specific; applicable to: loop, compute constructs; properties: device-specific clauses)

### Declaration Clauses

- `device_resident` (§2.13.1; p.82; category: data declaration; applicable to: declare; properties: data resides on device)
- `link` (§2.13.3; p.84; category: data declaration; applicable to: declare; properties: static device linkage)

### Special Clauses

- `finalize` (§2.6.6; p.46; category: data management; applicable to: exit data; properties: force deallocation)
- `bind` (§2.15.1; p.92; category: routine; applicable to: routine; properties: specify device routine name)
- `nohost` (§2.15.1; p.93; category: routine; applicable to: routine; properties: routine only on device)

## Modifiers

Modifiers are keywords that modify the behavior of clauses. They appear as part of clause syntax to refine clause semantics.

### Data Clause Modifiers

- `always` (§2.7.4; p.52; data clause modifier; forces data transfer even if present)
- `zero` (§2.7.4; p.52; data clause modifier; zero memory on allocation)
- `readonly` (§2.7.4; p.52; data clause modifier; read-only access)

### Gang Clause Modifiers

- `num` (§2.9.2; p.66; gang modifier; specifies number of gangs)
- `dim` (§2.9.2; p.67; gang modifier; specifies gang dimension)
- `static` (§2.9.2; p.67; gang modifier; static gang distribution)

### Worker Clause Modifiers

- `num` (§2.9.3; p.68; worker modifier; specifies number of workers)

### Vector Clause Modifiers

- `length` (§2.9.4; p.68; vector modifier; specifies vector length)

### Collapse Clause Modifiers

- `force` (§2.9.1; p.65; collapse modifier; force collapse even with dependencies)

### Cache Clause Modifiers

- `readonly` (§2.10; p.75; cache modifier; read-only cache hint)

## Special Values and Constants

### Async Values

- `acc_async_default` (§2.16; p.98; async value; default async queue)
- `acc_async_noval` (§2.16; p.98; async value; no async queue specified)
- `acc_async_sync` (§2.16; p.98; async value; synchronous execution)

### Device Types

- `*` (§2.4; p.31; device type; all device types)
- `host` (§2.4; p.31; device type; host device)
- `nvidia` (§2.4; p.31; device type; NVIDIA devices)
- `radeon` (§2.4; p.31; device type; AMD Radeon devices)
- `default` (§2.4; p.31; device type; implementation default)

### Default Clause Values

- `none` (§2.5.16; p.40; default value; no implicit data sharing)
- `present` (§2.5.16; p.40; default value; assume all data present)

## Reduction Operators

Operators used with the `reduction` clause for parallel reduction operations.

### Arithmetic Operators

- `+` (§2.5.15, §2.9.11; pp.39, 71; addition)
- `*` (§2.5.15, §2.9.11; pp.39, 71; multiplication)
- `max` (§2.5.15, §2.9.11; pp.39, 71; maximum value)
- `min` (§2.5.15, §2.9.11; pp.39, 71; minimum value)

### Bitwise Operators

- `&` (§2.5.15, §2.9.11; pp.39, 71; bitwise AND)
- `|` (§2.5.15, §2.9.11; pp.39, 71; bitwise OR)
- `^` (§2.5.15, §2.9.11; pp.39, 71; bitwise XOR)

### Logical Operators

- `&&` (§2.5.15, §2.9.11; pp.39, 71; logical AND)
- `||` (§2.5.15, §2.9.11; pp.39, 71; logical OR)

### Fortran-Specific Operators

- `.and.` (§2.5.15, §2.9.11; pp.39, 71; Fortran logical AND)
- `.or.` (§2.5.15, §2.9.11; pp.39, 71; Fortran logical OR)
- `.eqv.` (§2.5.15, §2.9.11; pp.39, 71; Fortran logical equivalence)
- `.neqv.` (§2.5.15, §2.9.11; pp.39, 71; Fortran logical non-equivalence)
- `iand` (§2.5.15, §2.9.11; pp.39, 71; Fortran bitwise AND)
- `ior` (§2.5.15, §2.9.11; pp.39, 71; Fortran bitwise OR)
- `ieor` (§2.5.15, §2.9.11; pp.39, 71; Fortran bitwise XOR)

## Parallelism Levels

OpenACC defines a three-level parallelism hierarchy:

- `gang` (§2.2.3; p.23; parallelism level; coarse-grain parallelism, analogous to thread blocks)
- `worker` (§2.2.3; p.23; parallelism level; medium-grain parallelism, analogous to threads)
- `vector` (§2.2.3; p.23; parallelism level; fine-grain parallelism, analogous to SIMD lanes)
- `seq` (§2.9.5; p.68; parallelism level; sequential execution, no parallelism)

## Atomic Operation Keywords

- `read` (§2.12; p.77; atomic operation; atomic read)
- `write` (§2.12; p.78; atomic operation; atomic write)
- `update` (§2.12; p.78; atomic operation; atomic update)
- `capture` (§2.12; p.79; atomic operation; atomic capture)

## Runtime Clause Keywords

### Set Directive Clauses

- `device_type` (§2.14.3; p.87; applicable to: set; specifies device type)
- `device_num` (§2.14.3; p.87; applicable to: set; specifies device number)
- `default_async` (§2.14.3; p.87; applicable to: set; sets default async queue)

### Update Directive Clauses

- `self` (§2.14.4; p.88; applicable to: update; copy to host)
- `host` (§2.14.4; p.88; applicable to: update; alias for self)
- `device` (§2.14.4; p.88; applicable to: update; copy to device)

### Routine Directive Clauses

- `gang` (§2.15.1; p.92; applicable to: routine; routine contains gang-level parallelism)
- `worker` (§2.15.1; p.92; applicable to: routine; routine contains worker-level parallelism)
- `vector` (§2.15.1; p.92; applicable to: routine; routine contains vector-level parallelism)
- `seq` (§2.15.1; p.92; applicable to: routine; routine is sequential)
