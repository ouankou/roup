# OpenACC 3.4 Directive and Clause Restrictions

This digest enumerates every rule, restriction, and mandatory condition attached to OpenACC 3.4 directives and clauses. Entries are grouped by the section that defines the constraint. Page references match the official specification pagination so each item can be cross-checked word-for-word.

## Compute constructs (§2.5.4, p.36)
- Programs must not branch into or out of a compute construct.
- Only the `async`, `wait`, `num_gangs`, `num_workers`, and `vector_length` clauses may follow a `device_type` clause on any compute construct.
- At most one `if` clause may appear on a compute construct.
- At most one `default` clause may appear and its value must be either `none` or `present`.
- A `reduction` clause must not appear on a `parallel` construct whose `num_gangs` clause has more than one argument.

## Compute construct errors (§2.5.5, p.37)
- Errors raised by violating compute construct semantics follow §2.5.5; implementations must signal `acc_error_host_only`, `acc_error_invalid_compute_region`, `acc_error_invalid_parallelism`, or `acc_error_invalid_matrix_shape` as described in the specification when these conditions are detected.

## Enter/exit data directives (§2.6.6, p.46)
- `enter data` directives must include at least one of: `copyin`, `create`, or `attach`.
- `exit data` directives must include at least one of: `copyout`, `delete`, or `detach`.
- Only one `if` clause may appear on either directive.
- `finalize` on `exit data` resets dynamic and attachment counters to zero for the listed variables; without it the counters are decremented normally.

## Data environment (§2.6, pp.43–47)
- Implicit data lifetime management must obey the reference counter semantics in §§2.6.3–2.6.8; structured and dynamic counters must never become negative.
- Pointer attachments must respect the attach/detach counter pairing in §§2.6.7–2.6.8.

## Host_data construct (§2.8, pp.62–63)
- `use_device` lists must reference variables that are present on the device; otherwise behavior is undefined.
- Host pointers aliased inside `host_data` regions must not be dereferenced on the host while mapped to device addresses.

## Loop construct (§2.9, pp.64–71)
- Only `collapse`, `gang`, `worker`, `vector`, `seq`, `independent`, `auto`, and `tile` clauses may follow a `device_type` clause.
- `worker` and `vector` clause arguments must be invariant within the surrounding `kernels` region.
- Loops without `seq` must satisfy: loop variable is integer/pointer/random-access iterator, iteration monotonicity, and constant-time trip count computation.
- Only one of `seq`, `independent`, or `auto` may appear.
- `gang`, `worker`, and `vector` clauses are mutually exclusive with an explicit `seq` clause.
- A loop with a gang/worker/vector clause must not lexically enclose another loop with an equal or higher parallelism level unless the parent compute scope differs.
- At most one `gang` clause may appear per loop construct.
- `tile` and `collapse` must not be combined on loops associated with `do concurrent`.
- Each associated loop in a `tile` construct (except the innermost) must contain exactly one loop or loop nest.
- `private` clauses on loops must honor Fortran optional argument rules (§2.17.1, p.100).

## Cache directive (§2.10, p.75)
- References within the loop iteration must stay inside the index ranges listed in the `cache` directive.
- Fortran optional arguments used in `cache` directives must follow §2.17.1.

## Combined constructs (§2.11, p.76)
- Combined constructs inherit all restrictions from their constituent `parallel`, `serial`, `kernels`, and `loop` components.

## Atomic construct (§2.12, pp.77–81)
- All atomic accesses to a given storage location must use the same type and type parameters.
- The storage location designated by `x` must not exceed the hardware’s maximum native atomic width.
- At most one `if` clause may appear on an atomic construct.

## Declare directive (§2.13, pp.81–84)
- `declare` must share scope with the declared variables (or enclosing function/module scope for Fortran).
- At least one clause is required.
- Clause arguments must be variable names or Fortran common block names; each variable may appear only once across `declare` clauses within a program unit.
- Fortran assumed-size dummy arrays cannot appear; pointer arrays lose association on the device.
- Fortran module declaration sections allow only `create`, `copyin`, `device_resident`, and `link`; C/C++ global scope allows only `create`, `copyin`, `deviceptr`, `device_resident`, and `link`.
- C/C++ extern variables are limited to `create`, `copyin`, `deviceptr`, `device_resident`, and `link`.
- `link` clauses must appear at global/module scope or reference extern/common-block entities.
- `declare` regions must not contain `longjmp`/`setjmp` mismatches or uncaught C++ exceptions.
- Fortran optional dummy arguments in data clauses must respect §2.17.1.

## Init directive (§2.14.1, p.85)
- May appear only in host code.
- Re-initializing with different device types without shutting down is implementation-defined.
- Initializing a device type not used by compiled accelerator regions yields undefined behavior.

## Shutdown directive (§2.14.2, p.85)
- May appear only in host code.

## Set directive (§2.14.3, p.87)
- Host-only directive.
- `default_async` accepts only valid async identifiers; `acc_async_noval` has no effect, `acc_async_sync` forces synchronous execution on the default queue, and `acc_async_default` restores the initial queue.
- Must include at least one of `default_async`, `device_num`, or `device_type`.
- Duplicate clause kinds are forbidden on the same directive.

## Update directive (§2.14.4, pp.88–90)
- Requires at least one of `self`, `host`, or `device` clauses.
- If `if_present` is absent, all listed variables must already be present on the device.
- Only `async` and `wait` clauses may follow `device_type`.
- At most one `if` clause may appear; it must evaluate to a scalar logical/integer value (Fortran vs C/C++ rules).
- Noncontiguous subarrays are permitted; implementations may choose between multiple transfers or pack/unpack strategies but must not transfer outside the minimal containing contiguous region.
- Struct/class and derived-type member restrictions follow §2.14.4; parent objects cannot simultaneously use subarray notation with member subarrays.
- Fortran optional arguments in `self`, `host`, and `device` follow §2.17.1.
- Directive must occupy a statement position (cannot replace the body after conditional headers or labels).

## Wait directive (§2.16.3, p.100)
- `devnum` values in wait-arguments must identify valid devices; invalid values trigger runtime errors (§2.16.3).
- Queue identifiers must be valid async arguments or errors result (§2.16.3).

## Routine directive (§2.15.1, pp.91–97)
- Implicit routine directives derive from usage; implementations must propagate relevant clauses to dependent procedures and avoid infinite recursion when determining implicit attributes.
- `gang` dimension argument must be an integer constant expression in {1,2,3}.
- `worker` routines cannot be parents of gang routines; `vector` routines cannot be parents of worker or gang routines; `seq` routines cannot be parents of parallel routines.
- Procedures compiled with `nohost` must not be called from host-only regions; enclosing procedures must also carry `nohost` when they call such routines.

## Do concurrent integration (§2.17.2, pp.102–103)
- When mapping Fortran `do concurrent` locality specs to OpenACC clauses, users must ensure host/device sharing matches the specified locality (e.g., `local` to `private`, `local_init` to `firstprivate`).

## Data clauses (§§2.7–2.7.14, pp.48–60)
- Clause arguments must reference contiguous array sections or pointer references as defined in §2.7.1.
- Overlapping array sections in data clauses yield unspecified behavior (§2.7.1).
- Modifier keywords (`always`, `alwaysin`, `alwaysout`, `capture`, `readonly`, `zero`) enforce the transfer behaviors in §2.7.4 and must not contradict device pointer semantics.
- `deviceptr` variables cannot appear in other data clauses in the same region (§2.7.5).
- `present` clauses require existing device data; absence triggers runtime errors (§2.7.6).
- `no_create` forbids allocation and therefore requires prior presence (§2.7.11).
- `attach`/`detach` must pair with pointer lifetimes and respect attachment counters (§§2.7.13–2.7.14).

## Cache clause (§2.10, p.75)
- Array references must remain within the listed cache ranges per iteration; violations are undefined.

## Clause argument rules (§2.16, p.98)
- `async-argument` values are limited to nonnegative integers or the special constants `acc_async_default`, `acc_async_noval`, `acc_async_sync`.
- `wait-argument` syntax `[devnum:int-expr:][queues:]async-argument-list` requires valid device numbers and async identifiers.

