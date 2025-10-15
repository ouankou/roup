# OpenMP 6.0 Directive–Clause Components

For each OpenMP 6.0 directive or construct, this section lists every clause permitted by the specification. Clause entries include their specification metadata as well as the argument and modifier tables transcribed verbatim from the standard to preserve exact semantics.

## `allocate` (Section 8.5; pp. 341–342; category: declarative; association: explicit; properties: pure)

### Clause `align` (Section 8.3; p. 340)

Permitted on directives: `allocate`.

**Arguments**

`````ignore
Name Type Properties
alignment expression of integer
type
constant, positive
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `allocator` (Section 8.4)

Permitted on directives: `allocate`.

**Arguments**

`````ignore
Name Type Properties
allocator expression of allocator_-
handle type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `allocators` (Section 8.7; category: executable; association: block : allocator; properties: default)

### Clause `allocate` (Section 8.6; pp. 343–345)

Permitted on directives: `allocators`, `distribute`, `do`, `for`, `parallel`, `scope`, `sections`, `single`, `target`, `target_data`, `task`, `taskgroup`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
allocator-simple-
modifier
list expression of OpenMP allo-
cator_handle type
exclusive, unique
allocator-complex-
modifier
list Complex, name:
allocator
`````


## `assume` (Section 10.6.3; category: informational; association: block; properties: pure)

### Clause `absent` (Section 10.6.1.1; p. 394)

Permitted on directives: `assume`, `assumes`, `begin assumes`.

**Arguments**

`````ignore
Name Type Properties
directive-name-list list of directive-name list
item type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `contains` (Section 10.6.1.2)

Permitted on directives: `assume`, `assumes`, `begin assumes`.

**Arguments**

`````ignore
Name Type Properties
directive-name-list list of directive-name list
item type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `holds` (Section 10.6.1.3; p. 395)

Permitted on directives: `assume`, `assumes`, `begin assumes`.

**Arguments**

`````ignore
Name Type Properties
hold-expr expression of OpenMP
logical type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `no_openmp` (Section 10.6.1.4; p. 396)

Permitted on directives: `assume`, `assumes`, `begin assumes`.

**Arguments**

`````ignore
Name Type Properties
can_assume expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `no_openmp_constructs` (Section 10.6.1.5)

Permitted on directives: `assume`, `assumes`, `begin assumes`.

**Arguments**

`````ignore
Name Type Properties
can_assume expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `no_openmp_routines` (Section 10.6.1.6; p. 397)

Permitted on directives: `assume`, `assumes`, `begin assumes`.

**Arguments**

`````ignore
Name Type Properties
can_assume expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `no_parallelism` (Section 10.6.1.7; p. 398)

Permitted on directives: `assume`, `assumes`, `begin assumes`.

**Arguments**

`````ignore
Name Type Properties
can_assume expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `assumes` (Section 10.6.2; p. 399; category: informational; association: unassociated; properties: pure)

### Clause `absent` (Section 10.6.1.1; p. 394)

Permitted on directives: `assume`, `assumes`, `begin assumes`.

**Arguments**

`````ignore
Name Type Properties
directive-name-list list of directive-name list
item type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `contains` (Section 10.6.1.2)

Permitted on directives: `assume`, `assumes`, `begin assumes`.

**Arguments**

`````ignore
Name Type Properties
directive-name-list list of directive-name list
item type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `holds` (Section 10.6.1.3; p. 395)

Permitted on directives: `assume`, `assumes`, `begin assumes`.

**Arguments**

`````ignore
Name Type Properties
hold-expr expression of OpenMP
logical type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `no_openmp` (Section 10.6.1.4; p. 396)

Permitted on directives: `assume`, `assumes`, `begin assumes`.

**Arguments**

`````ignore
Name Type Properties
can_assume expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `no_openmp_constructs` (Section 10.6.1.5)

Permitted on directives: `assume`, `assumes`, `begin assumes`.

**Arguments**

`````ignore
Name Type Properties
can_assume expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `no_openmp_routines` (Section 10.6.1.6; p. 397)

Permitted on directives: `assume`, `assumes`, `begin assumes`.

**Arguments**

`````ignore
Name Type Properties
can_assume expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `no_parallelism` (Section 10.6.1.7; p. 398)

Permitted on directives: `assume`, `assumes`, `begin assumes`.

**Arguments**

`````ignore
Name Type Properties
can_assume expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `atomic` (Section 17.8.5; pp. 525–528; category: executable; association: block : atomic; properties: mutual-exclusion, order-)

### Clause `acq_rel` (Section 17.8.1.1; p. 515)

Permitted on directives: `atomic`, `flush`.

**Arguments**

`````ignore
Name Type Properties
use-semantics expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `acquire` (Section 17.8.1.2; p. 516)

Permitted on directives: `atomic`, `flush`.

**Arguments**

`````ignore
Name Type Properties
use_semantics expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `capture` (Section 17.8.3.1; p. 521)

Permitted on directives: `atomic`.

**Arguments**

`````ignore
Name Type Properties
use_semantics expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `compare` (Section 17.8.3.2; p. 522)

Permitted on directives: `atomic`.

**Arguments**

`````ignore
Name Type Properties
use_semantics expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `fail` (Section 17.8.3.3)

Permitted on directives: `atomic`.

**Arguments**

`````ignore
Name Type Properties
memorder Keyword:acquire,
relaxed, seq_cst
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `hint` (Section 17.1; p. 503)

Permitted on directives: `atomic`, `critical`.

**Arguments**

`````ignore
Name Type Properties
hint-expr expression of sync_hint
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `memscope` (Section 17.8.4; p. 524)

Permitted on directives: `atomic`, `flush`.

**Arguments**

`````ignore
Name Type Properties
scope-specifier Keyword:all,
cgroup,device
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `read` (Section 17.8.2.1; p. 519)

Permitted on directives: `atomic`.

**Arguments**

`````ignore
Name Type Properties
use_semantics expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `relaxed` (Section 17.8.1.3)

Permitted on directives: `atomic`, `flush`.

**Arguments**

`````ignore
Name Type Properties
use_semantics expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `release` (Section 17.8.1.4; p. 517)

Permitted on directives: `atomic`, `flush`.

**Arguments**

`````ignore
Name Type Properties
use_semantics expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `seq_cst` (Section 17.8.1.5; p. 518; properties: exclusive, unique Members:)

Permitted on directives: `atomic`, `flush`.

**Arguments**

`````ignore
Name Type Properties
use_semantics expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `weak` (Section 17.8.3.4; p. 523)

Permitted on directives: `atomic`.

**Arguments**

`````ignore
Name Type Properties
use_semantics expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `write` (Section 17.8.2.3; p. 520; properties: unique Members:)

Permitted on directives: `atomic`.

**Arguments**

`````ignore
Name Type Properties
use_semantics expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `barrier` (Section 17.3.1; pp. 506–508; category: executable; association: unassociated; properties: team-executed)

_No clauses are defined for this directive in the specification._

## `begin assumes` (Section 10.6.4; category: informational; association: delimited; properties: default)

### Clause `absent` (Section 10.6.1.1; p. 394)

Permitted on directives: `assume`, `assumes`, `begin assumes`.

**Arguments**

`````ignore
Name Type Properties
directive-name-list list of directive-name list
item type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `contains` (Section 10.6.1.2)

Permitted on directives: `assume`, `assumes`, `begin assumes`.

**Arguments**

`````ignore
Name Type Properties
directive-name-list list of directive-name list
item type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `holds` (Section 10.6.1.3; p. 395)

Permitted on directives: `assume`, `assumes`, `begin assumes`.

**Arguments**

`````ignore
Name Type Properties
hold-expr expression of OpenMP
logical type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `no_openmp` (Section 10.6.1.4; p. 396)

Permitted on directives: `assume`, `assumes`, `begin assumes`.

**Arguments**

`````ignore
Name Type Properties
can_assume expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `no_openmp_constructs` (Section 10.6.1.5)

Permitted on directives: `assume`, `assumes`, `begin assumes`.

**Arguments**

`````ignore
Name Type Properties
can_assume expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `no_openmp_routines` (Section 10.6.1.6; p. 397)

Permitted on directives: `assume`, `assumes`, `begin assumes`.

**Arguments**

`````ignore
Name Type Properties
can_assume expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `no_parallelism` (Section 10.6.1.7; p. 398)

Permitted on directives: `assume`, `assumes`, `begin assumes`.

**Arguments**

`````ignore
Name Type Properties
can_assume expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `begin declare_target` (Section 9.9.2; p. 380; category: declarative; association: delimited; properties: declare-target, device,)

### Clause `device_type` (Section 15.1; p. 481)

Permitted on directives: `begin declare_target`, `declare_target`, `groupprivate`, `target`.

**Arguments**

`````ignore
Name Type Properties
device-type-description Keyword:any,host,
nohost
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `indirect` (Section 9.9.3; pp. 381–382)

Permitted on directives: `begin declare_target`, `declare_target`.

**Arguments**

`````ignore
Name Type Properties
invoked-by-fptr expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `begin declare_variant` (Section 9.6.5; p. 367; category: declarative; association: delimited; properties: default)

### Clause `match` (Section 9.6.1; p. 361)

Permitted on directives: `begin declare_variant`, `declare_variant`.

**Arguments**

`````ignore
Name Type Properties
context-selector An OpenMP context-
selector-specification
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `cancel` (Section 18.2; pp. 551–554; category: executable; association: unassociated; properties: default)

### Clause `if` (Section 5.5; p. 210)

Permitted on directives: `cancel`, `parallel`, `simd`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `task_iteration`, `taskgraph`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
if-expression expression of OpenMP
logical type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `cancellation_point` (Section 18.3; pp. 555–964; category: executable; association: unassociated; properties: default)

_No clauses are defined for this directive in the specification._

## `critical` (Section 17.2; pp. 504–505; category: executable; association: block; properties: mutual-exclusion, thread-)

### Clause `hint` (Section 17.1; p. 503)

Permitted on directives: `atomic`, `critical`.

**Arguments**

`````ignore
Name Type Properties
hint-expr expression of sync_hint
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `declare_induction` (Section 7.6.17; pp. 294–295; category: declarative; association: unassociated; properties: pure)

### Clause `collector` (Section 7.6.19)

Permitted on directives: `declare_induction`.

**Arguments**

`````ignore
Name Type Properties
collector-expr expression of collector
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `inductor` (Section 7.6.18; p. 296)

Permitted on directives: `declare_induction`.

**Arguments**

`````ignore
Name Type Properties
inductor-expr expression of inductor
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `declare_mapper` (Section 7.9.10; pp. 324–327; category: declarative; association: unassociated; properties: pure)

### Clause `map` (Section 7.9.6; pp. 310–319)

Permitted on directives: `declare_mapper`, `target`, `target_data`, `target_enter_data`, `target_exit_data`.

**Arguments**

`````ignore
Name Type Properties
locator-list list of locator list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
always-modifier locator-list Keyword:always map-type-
modifying
close-modifier locator-list Keyword:close map-type-
modifying
present-modifier locator-list Keyword:present map-type-
modifying
self-modifier locator-list Keyword:self map-type-
modifying
ref-modifier all arguments Keyword:ref_ptee,
ref_ptr,ref_ptr_ptee
unique
delete-modifier locator-list Keyword:delete map-type-
modifying
mapper locator-list Complex, name:mapper
`````


## `declare_reduction` (Section 7.6.14; pp. 291–292; category: declarative; association: unassociated; properties: pure)

### Clause `combiner` (Section 7.6.15)

Permitted on directives: `declare_reduction`.

**Arguments**

`````ignore
Name Type Properties
combiner-expr expression of combiner
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `initializer` (Section 7.6.16; p. 293)

Permitted on directives: `declare_reduction`.

**Arguments**

`````ignore
Name Type Properties
initializer-expr expression of initializer
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `declare_simd` (Section 9.8; pp. 372–373; category: declarative; association: declaration; properties: pure, variant-generating)

### Clause `aligned` (Section 7.12; p. 331)

Permitted on directives: `declare_simd`, `simd`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
alignment list OpenMP integer expression positive, region
invariant, ultimate,
unique
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `inbranch` (Section 9.8.1.1; p. 374)

Permitted on directives: `declare_simd`.

**Arguments**

`````ignore
Name Type Properties
inbranch expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `linear` (Section 7.5.6; pp. 263–265)

Permitted on directives: `declare_simd`, `do`, `for`, `simd`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
step-simple-
modifier
list OpenMP integer expression exclusive, region-
invariant, unique
step-complex-
modifier
list Complex, name:step
`````

### Clause `notinbranch` (Section 9.8.1.2; pp. 375–376)

Permitted on directives: `declare_simd`.

**Arguments**

`````ignore
Name Type Properties
notinbranch expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `simdlen` (Section 12.4.3; p. 432)

Permitted on directives: `declare_simd`, `simd`.

**Arguments**

`````ignore
Name Type Properties
length expression of integer
type
positive, constant
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `uniform` (Section 7.11; p. 330)

Permitted on directives: `declare_simd`.

**Arguments**

`````ignore
Name Type Properties
parameter-list list of parameter list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `declare_target` (Section 9.9.1; pp. 377–379; category: declarative; association: explicit; properties: declare-target, device,)

### Clause `device_type` (Section 15.1; p. 481)

Permitted on directives: `begin declare_target`, `declare_target`, `groupprivate`, `target`.

**Arguments**

`````ignore
Name Type Properties
device-type-description Keyword:any,host,
nohost
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `enter` (Section 7.9.7; p. 320)

Permitted on directives: `declare_target`.

**Arguments**

`````ignore
Name Type Properties
list list of extended list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
automap-modifier list Keyword:automap default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `indirect` (Section 9.9.3; pp. 381–382)

Permitted on directives: `begin declare_target`, `declare_target`.

**Arguments**

`````ignore
Name Type Properties
invoked-by-fptr expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `link` (Section 7.9.8; p. 321)

Permitted on directives: `declare_target`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `local` (Section 7.14; pp. 334–339)

Permitted on directives: `declare_target`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `declare_variant` (Section 9.6.4; pp. 365–366; category: declarative; association: declaration; properties: pure)

### Clause `adjust_args` (Section 9.6.2; pp. 362–363)

Permitted on directives: `declare_variant`.

**Arguments**

`````ignore
Name Type Properties
parameter-list list of parameter list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
adjust-op parameter-list Keyword:
need_device_addr,
need_device_ptr,
nothing
required
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `append_args` (Section 9.6.3; p. 364)

Permitted on directives: `declare_variant`.

**Arguments**

`````ignore
Name Type Properties
append-op-list list of OpenMP opera-
tion list item type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `match` (Section 9.6.1; p. 361)

Permitted on directives: `begin declare_variant`, `declare_variant`.

**Arguments**

`````ignore
Name Type Properties
context-selector An OpenMP context-
selector-specification
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `depobj` (Section 17.9.3; p. 536; category: executable; association: unassociated; properties: default)

### Clause `destroy` (Section 5.7; pp. 213–235)

Permitted on directives: `depobj`, `interop`.

**Arguments**

`````ignore
Name Type Properties
destroy-var variable of OpenMP
variable type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `init` (Section 5.6; pp. 211–212)

Permitted on directives: `depobj`, `interop`.

**Arguments**

`````ignore
Name Type Properties
init-var variable of OpenMP
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
prefer-type init-var Complex, name:
prefer_type
`````

### Clause `update` (Section 17.9.4; p. 537)

Permitted on directives: `depobj`.

**Arguments**

`````ignore
Name Type Properties
update-var variable of OpenMP
depend type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
task-dependence-
type
all arguments Keyword:depobj, in,
inout,inoutset,
mutexinoutset, out
unique
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `dispatch` (Section 9.7; pp. 368–369; category: executable; association: block : function-dispatch; properties: context-matching)

### Clause `depend` (Section 17.9.5; pp. 538–540)

Permitted on directives: `dispatch`, `interop`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `task_iteration`, `taskwait`.

**Arguments**

`````ignore
Name Type Properties
locator-list list of locator list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
task-dependence-
type
all arguments Keyword:depobj, in,
inout,inoutset,
mutexinoutset, out
unique
iterator locator-list Complex, name:iterator
`````

### Clause `device` (Section 15.2; p. 482)

Permitted on directives: `dispatch`, `interop`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`.

**Arguments**

`````ignore
Name Type Properties
device-description expression of integer
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
device-modifier device-description Keyword:ancestor,
device_num
default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `has_device_addr` (Section 7.5.9; p. 268)

Permitted on directives: `dispatch`, `target`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `interop` (Section 9.7.1; p. 370)

Permitted on directives: `dispatch`.

**Arguments**

`````ignore
Name Type Properties
interop-var-list list of variable of interop
OpenMP type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `is_device_ptr` (Section 7.5.7; p. 266)

Permitted on directives: `dispatch`, `target`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `nocontext` (Section 9.7.3; p. 371)

Permitted on directives: `dispatch`.

**Arguments**

`````ignore
Name Type Properties
do-not-update-context expression of OpenMP
logical type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `novariants` (Section 9.7.2)

Permitted on directives: `dispatch`.

**Arguments**

`````ignore
Name Type Properties
do-not-use-variant expression of OpenMP
logical type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `nowait` (Section 17.6; pp. 512–513)

Permitted on directives: `dispatch`, `do`, `for`, `interop`, `scope`, `sections`, `single`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `taskwait`, `workshare`.

**Arguments**

`````ignore
Name Type Properties
do_not_synchronize expression of OpenMP
logical type
optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `distribute` (Section 13.7; pp. 451–452; category: executable; association: loop nest; properties: SIMD-partitionable,)

### Clause `allocate` (Section 8.6; pp. 343–345)

Permitted on directives: `allocators`, `distribute`, `do`, `for`, `parallel`, `scope`, `sections`, `single`, `target`, `target_data`, `task`, `taskgroup`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
allocator-simple-
modifier
list expression of OpenMP allo-
cator_handle type
exclusive, unique
allocator-complex-
modifier
list Complex, name:
allocator
`````

### Clause `collapse` (Section 6.4.5; p. 236)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `simd`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
n expression of integer
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `dist_schedule` (Section 13.7.1; p. 453)

Permitted on directives: `distribute`.

**Arguments**

`````ignore
Name Type Properties
kind Keyword:static default
chunk_size expression of integer
type
ultimate, optional, posi-
tive, region-invariant
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `firstprivate` (Section 7.5.4; pp. 258–259)

Permitted on directives: `distribute`, `do`, `for`, `parallel`, `scope`, `sections`, `single`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
saved list Keyword:saved default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `induction` (Section 7.6.13; pp. 288–290)

Permitted on directives: `distribute`, `do`, `for`, `simd`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
induction-
identifier
list OpenMP induction identifier required, ultimate
step-modifier list Complex, name:step
`````

### Clause `lastprivate` (Section 7.5.5; pp. 260–262)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `sections`, `simd`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
lastprivate-
modifier
list Keyword:conditional default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `order` (Section 12.3; pp. 428–429)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `simd`.

**Arguments**

`````ignore
Name Type Properties
ordering Keyword:
concurrent
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
order-modifier ordering Keyword:reproducible,
unconstrained
default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `private` (Section 7.5.3; pp. 256–257)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `parallel`, `scope`, `sections`, `simd`, `single`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `do` (Section 13.6.2; p. 448; category: executable; association: loop nest; properties: work-distribution,)

### Clause `allocate` (Section 8.6; pp. 343–345)

Permitted on directives: `allocators`, `distribute`, `do`, `for`, `parallel`, `scope`, `sections`, `single`, `target`, `target_data`, `task`, `taskgroup`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
allocator-simple-
modifier
list expression of OpenMP allo-
cator_handle type
exclusive, unique
allocator-complex-
modifier
list Complex, name:
allocator
`````

### Clause `collapse` (Section 6.4.5; p. 236)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `simd`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
n expression of integer
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `firstprivate` (Section 7.5.4; pp. 258–259)

Permitted on directives: `distribute`, `do`, `for`, `parallel`, `scope`, `sections`, `single`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
saved list Keyword:saved default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `induction` (Section 7.6.13; pp. 288–290)

Permitted on directives: `distribute`, `do`, `for`, `simd`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
induction-
identifier
list OpenMP induction identifier required, ultimate
step-modifier list Complex, name:step
`````

### Clause `lastprivate` (Section 7.5.5; pp. 260–262)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `sections`, `simd`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
lastprivate-
modifier
list Keyword:conditional default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `linear` (Section 7.5.6; pp. 263–265)

Permitted on directives: `declare_simd`, `do`, `for`, `simd`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
step-simple-
modifier
list OpenMP integer expression exclusive, region-
invariant, unique
step-complex-
modifier
list Complex, name:step
`````

### Clause `nowait` (Section 17.6; pp. 512–513)

Permitted on directives: `dispatch`, `do`, `for`, `interop`, `scope`, `sections`, `single`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `taskwait`, `workshare`.

**Arguments**

`````ignore
Name Type Properties
do_not_synchronize expression of OpenMP
logical type
optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `order` (Section 12.3; pp. 428–429)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `simd`.

**Arguments**

`````ignore
Name Type Properties
ordering Keyword:
concurrent
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
order-modifier ordering Keyword:reproducible,
unconstrained
default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `ordered` (Section 6.4.6; p. 237)

Permitted on directives: `do`, `for`.

**Arguments**

`````ignore
Name Type Properties
n expression of integer
type
optional, constant, posi-
tive
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `private` (Section 7.5.3; pp. 256–257)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `parallel`, `scope`, `sections`, `simd`, `single`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `reduction` (Section 7.6.10; pp. 283–285)

Permitted on directives: `do`, `for`, `loop`, `parallel`, `scope`, `sections`, `simd`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
reduction-
identifier
all arguments An OpenMP reduction iden-
tifier
required, ultimate
reduction-modifier list Keyword:default,
inscan, task
default
original-sharing-
modifier
list Complex, name:original
`````

### Clause `schedule` (Section 13.6.3; pp. 449–450)

Permitted on directives: `do`, `for`.

**Arguments**

`````ignore
Name Type Properties
kind Keyword:auto,
dynamic, guided,
runtime, static
default
chunk_size expression of integer
type
ultimate, optional, posi-
tive, region-invariant
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
ordering-modifier kind Keyword:monotonic,
nonmonotonic
unique
chunk-modifier kind Keyword:simd unique
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `error` (Section 10.1; p. 383; category: utility; association: unassociated; properties: pure)

### Clause `at` (Section 10.2)

Permitted on directives: `error`.

**Arguments**

`````ignore
Name Type Properties
action-time Keyword:
compilation,
execution
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `message` (Section 10.3; p. 384)

Permitted on directives: `error`, `parallel`.

**Arguments**

`````ignore
Name Type Properties
msg-string expression of string type default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `severity` (Section 10.4; p. 385)

Permitted on directives: `error`, `parallel`.

**Arguments**

`````ignore
Name Type Properties
sev-level Keyword:fatal,
warning
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `flush` (Section 17.8.6; pp. 529–535; category: executable; association: unassociated; properties: default)

### Clause `acq_rel` (Section 17.8.1.1; p. 515)

Permitted on directives: `atomic`, `flush`.

**Arguments**

`````ignore
Name Type Properties
use-semantics expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `acquire` (Section 17.8.1.2; p. 516)

Permitted on directives: `atomic`, `flush`.

**Arguments**

`````ignore
Name Type Properties
use_semantics expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `memscope` (Section 17.8.4; p. 524)

Permitted on directives: `atomic`, `flush`.

**Arguments**

`````ignore
Name Type Properties
scope-specifier Keyword:all,
cgroup,device
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `relaxed` (Section 17.8.1.3)

Permitted on directives: `atomic`, `flush`.

**Arguments**

`````ignore
Name Type Properties
use_semantics expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `release` (Section 17.8.1.4; p. 517)

Permitted on directives: `atomic`, `flush`.

**Arguments**

`````ignore
Name Type Properties
use_semantics expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `seq_cst` (Section 17.8.1.5; p. 518; properties: exclusive, unique Members:)

Permitted on directives: `atomic`, `flush`.

**Arguments**

`````ignore
Name Type Properties
use_semantics expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `for` (Section 13.6.1; p. 447; category: executable; association: loop nest; properties: work-distribution,)

### Clause `allocate` (Section 8.6; pp. 343–345)

Permitted on directives: `allocators`, `distribute`, `do`, `for`, `parallel`, `scope`, `sections`, `single`, `target`, `target_data`, `task`, `taskgroup`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
allocator-simple-
modifier
list expression of OpenMP allo-
cator_handle type
exclusive, unique
allocator-complex-
modifier
list Complex, name:
allocator
`````

### Clause `collapse` (Section 6.4.5; p. 236)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `simd`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
n expression of integer
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `firstprivate` (Section 7.5.4; pp. 258–259)

Permitted on directives: `distribute`, `do`, `for`, `parallel`, `scope`, `sections`, `single`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
saved list Keyword:saved default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `induction` (Section 7.6.13; pp. 288–290)

Permitted on directives: `distribute`, `do`, `for`, `simd`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
induction-
identifier
list OpenMP induction identifier required, ultimate
step-modifier list Complex, name:step
`````

### Clause `lastprivate` (Section 7.5.5; pp. 260–262)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `sections`, `simd`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
lastprivate-
modifier
list Keyword:conditional default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `linear` (Section 7.5.6; pp. 263–265)

Permitted on directives: `declare_simd`, `do`, `for`, `simd`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
step-simple-
modifier
list OpenMP integer expression exclusive, region-
invariant, unique
step-complex-
modifier
list Complex, name:step
`````

### Clause `nowait` (Section 17.6; pp. 512–513)

Permitted on directives: `dispatch`, `do`, `for`, `interop`, `scope`, `sections`, `single`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `taskwait`, `workshare`.

**Arguments**

`````ignore
Name Type Properties
do_not_synchronize expression of OpenMP
logical type
optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `order` (Section 12.3; pp. 428–429)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `simd`.

**Arguments**

`````ignore
Name Type Properties
ordering Keyword:
concurrent
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
order-modifier ordering Keyword:reproducible,
unconstrained
default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `ordered` (Section 6.4.6; p. 237)

Permitted on directives: `do`, `for`.

**Arguments**

`````ignore
Name Type Properties
n expression of integer
type
optional, constant, posi-
tive
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `private` (Section 7.5.3; pp. 256–257)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `parallel`, `scope`, `sections`, `simd`, `single`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `reduction` (Section 7.6.10; pp. 283–285)

Permitted on directives: `do`, `for`, `loop`, `parallel`, `scope`, `sections`, `simd`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
reduction-
identifier
all arguments An OpenMP reduction iden-
tifier
required, ultimate
reduction-modifier list Keyword:default,
inscan, task
default
original-sharing-
modifier
list Complex, name:original
`````

### Clause `schedule` (Section 13.6.3; pp. 449–450)

Permitted on directives: `do`, `for`.

**Arguments**

`````ignore
Name Type Properties
kind Keyword:auto,
dynamic, guided,
runtime, static
default
chunk_size expression of integer
type
ultimate, optional, posi-
tive, region-invariant
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
ordering-modifier kind Keyword:monotonic,
nonmonotonic
unique
chunk-modifier kind Keyword:simd unique
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `fuse` (Section 11.3; p. 405; category: executable; association: loop sequence; properties: loop-transforming, order-)

### Clause `apply` (Section 11.1; pp. 403–404)

Permitted on directives: `fuse`, `interchange`, `nothing`, `reverse`, `split`, `stripe`, `tile`, `unroll`.

**Arguments**

`````ignore
Name Type Properties
applied-directives list of directive specifi-
cation list item type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
loop-modifier applied-directives Complex, Keyword:
fused,grid, identity,
interchanged,
intratile,offsets,
reversed, split,
unrolled
`````

### Clause `looprange` (Section 6.4.7; pp. 238–245)

Permitted on directives: `fuse`.

**Arguments**

`````ignore
Name Type Properties
first expression of OpenMP
integer type
constant, positive
count expression of OpenMP
integer type
constant, positive, ulti-
mate
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
saved list Keyword:saved default
`````


## `groupprivate` (Section 7.13; pp. 332–333; category: declarative; association: explicit; properties: pure)

### Clause `device_type` (Section 15.1; p. 481)

Permitted on directives: `begin declare_target`, `declare_target`, `groupprivate`, `target`.

**Arguments**

`````ignore
Name Type Properties
device-type-description Keyword:any,host,
nohost
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `interchange` (Section 11.4; p. 406; category: executable; association: loop nest; properties: loop-transforming,)

### Clause `apply` (Section 11.1; pp. 403–404)

Permitted on directives: `fuse`, `interchange`, `nothing`, `reverse`, `split`, `stripe`, `tile`, `unroll`.

**Arguments**

`````ignore
Name Type Properties
applied-directives list of directive specifi-
cation list item type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
loop-modifier applied-directives Complex, Keyword:
fused,grid, identity,
interchanged,
intratile,offsets,
reversed, split,
unrolled
`````

### Clause `permutation` (Section 11.4.1; p. 407)

Permitted on directives: `interchange`.

**Arguments**

`````ignore
Name Type Properties
permutation-list list of OpenMP integer
expression type
constant, positive
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `interop` (Section 16.1; p. 499; category: executable; association: unassociated; properties: device)

### Clause `depend` (Section 17.9.5; pp. 538–540)

Permitted on directives: `dispatch`, `interop`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `task_iteration`, `taskwait`.

**Arguments**

`````ignore
Name Type Properties
locator-list list of locator list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
task-dependence-
type
all arguments Keyword:depobj, in,
inout,inoutset,
mutexinoutset, out
unique
iterator locator-list Complex, name:iterator
`````

### Clause `destroy` (Section 5.7; pp. 213–235)

Permitted on directives: `depobj`, `interop`.

**Arguments**

`````ignore
Name Type Properties
destroy-var variable of OpenMP
variable type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `device` (Section 15.2; p. 482)

Permitted on directives: `dispatch`, `interop`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`.

**Arguments**

`````ignore
Name Type Properties
device-description expression of integer
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
device-modifier device-description Keyword:ancestor,
device_num
default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `init` (Section 5.6; pp. 211–212)

Permitted on directives: `depobj`, `interop`.

**Arguments**

`````ignore
Name Type Properties
init-var variable of OpenMP
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
prefer-type init-var Complex, name:
prefer_type
`````

### Clause `nowait` (Section 17.6; pp. 512–513)

Permitted on directives: `dispatch`, `do`, `for`, `interop`, `scope`, `sections`, `single`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `taskwait`, `workshare`.

**Arguments**

`````ignore
Name Type Properties
do_not_synchronize expression of OpenMP
logical type
optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `use` (Section 16.1.2; pp. 500–502)

Permitted on directives: `interop`.

**Arguments**

`````ignore
Name Type Properties
interop-var variable of interop
OpenMP type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `loop` (Section 13.8; p. 454; category: executable; association: loop nest; properties: order-concurrent-nestable,)

### Clause `bind` (Section 13.8.1; pp. 455–456)

Permitted on directives: `loop`.

**Arguments**

`````ignore
Name Type Properties
binding Keyword:parallel,
teams,thread
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `collapse` (Section 6.4.5; p. 236)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `simd`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
n expression of integer
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `lastprivate` (Section 7.5.5; pp. 260–262)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `sections`, `simd`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
lastprivate-
modifier
list Keyword:conditional default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `order` (Section 12.3; pp. 428–429)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `simd`.

**Arguments**

`````ignore
Name Type Properties
ordering Keyword:
concurrent
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
order-modifier ordering Keyword:reproducible,
unconstrained
default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `private` (Section 7.5.3; pp. 256–257)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `parallel`, `scope`, `sections`, `simd`, `single`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `reduction` (Section 7.6.10; pp. 283–285)

Permitted on directives: `do`, `for`, `loop`, `parallel`, `scope`, `sections`, `simd`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
reduction-
identifier
all arguments An OpenMP reduction iden-
tifier
required, ultimate
reduction-modifier list Keyword:default,
inscan, task
default
original-sharing-
modifier
list Complex, name:original
`````


## `masked` (Section 12.5; p. 433; category: executable; association: block; properties: thread-limiting, thread-)

### Clause `filter` (Section 12.5.1; pp. 434–435)

Permitted on directives: `masked`.

**Arguments**

`````ignore
Name Type Properties
thread_num expression of integer
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `nothing` (Section 10.7; pp. 400–402; category: utility; association: unassociated; properties: pure, loop-transforming)

### Clause `apply` (Section 11.1; pp. 403–404)

Permitted on directives: `fuse`, `interchange`, `nothing`, `reverse`, `split`, `stripe`, `tile`, `unroll`.

**Arguments**

`````ignore
Name Type Properties
applied-directives list of directive specifi-
cation list item type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
loop-modifier applied-directives Complex, Keyword:
fused,grid, identity,
interchanged,
intratile,offsets,
reversed, split,
unrolled
`````


## `ordered` (Section 17.10.2; pp. 546–547; category: executable; association: block; properties: mutual-exclusion, simdiz-)

### Clause `doacross` (Section 17.9.7; pp. 542–544)

Permitted on directives: `ordered`.

**Arguments**

`````ignore
Name Type Properties
iteration-specifier OpenMP iteration speci-
fier
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
dependence-type iteration-specifier Keyword:sink, source required
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `simd` (Section 17.10.3.2; pp. 549–550; properties: exclusive, required, unique Members:)

Permitted on directives: `ordered`.

**Arguments**

`````ignore
Name Type Properties
apply-to-simd expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `threads` (Section 17.10.3.1; p. 548)

Permitted on directives: `ordered`.

**Arguments**

`````ignore
Name Type Properties
apply-to-threads expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `parallel` (Section 12.1; pp. 415–418; category: executable; association: block; properties: cancellable, context-)

### Clause `allocate` (Section 8.6; pp. 343–345)

Permitted on directives: `allocators`, `distribute`, `do`, `for`, `parallel`, `scope`, `sections`, `single`, `target`, `target_data`, `task`, `taskgroup`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
allocator-simple-
modifier
list expression of OpenMP allo-
cator_handle type
exclusive, unique
allocator-complex-
modifier
list Complex, name:
allocator
`````

### Clause `copyin` (Section 7.8.1; p. 302)

Permitted on directives: `parallel`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `default` (Section 7.5.1; p. 254)

Permitted on directives: `parallel`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
data-sharing-attribute Keyword:
firstprivate,
none, private,
shared
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
variable-category implicit-behavior Keyword:aggregate,
all, allocatable,
pointer,scalar
default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `firstprivate` (Section 7.5.4; pp. 258–259)

Permitted on directives: `distribute`, `do`, `for`, `parallel`, `scope`, `sections`, `single`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
saved list Keyword:saved default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `if` (Section 5.5; p. 210)

Permitted on directives: `cancel`, `parallel`, `simd`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `task_iteration`, `taskgraph`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
if-expression expression of OpenMP
logical type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `message` (Section 10.3; p. 384)

Permitted on directives: `error`, `parallel`.

**Arguments**

`````ignore
Name Type Properties
msg-string expression of string type default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `num_threads` (Section 12.1.2; pp. 419–422)

Permitted on directives: `parallel`.

**Arguments**

`````ignore
Name Type Properties
nthreads list of OpenMP integer
expression type
positive
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
prescriptiveness nthreads Keyword:strict default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `private` (Section 7.5.3; pp. 256–257)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `parallel`, `scope`, `sections`, `simd`, `single`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `proc_bind` (Section 12.1.4; p. 423)

Permitted on directives: `parallel`.

**Arguments**

`````ignore
Name Type Properties
affinity-policy Keyword:close,
primary, spread
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `reduction` (Section 7.6.10; pp. 283–285)

Permitted on directives: `do`, `for`, `loop`, `parallel`, `scope`, `sections`, `simd`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
reduction-
identifier
all arguments An OpenMP reduction iden-
tifier
required, ultimate
reduction-modifier list Keyword:default,
inscan, task
default
original-sharing-
modifier
list Complex, name:original
`````

### Clause `safesync` (Section 12.1.5; p. 424)

Permitted on directives: `parallel`.

**Arguments**

`````ignore
Name Type Properties
width expression of integer
type
positive, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `severity` (Section 10.4; p. 385)

Permitted on directives: `error`, `parallel`.

**Arguments**

`````ignore
Name Type Properties
sev-level Keyword:fatal,
warning
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `shared` (Section 7.5.2; p. 255)

Permitted on directives: `parallel`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `requires` (Section 10.5; p. 386; category: informational; association: unassociated; properties: default)

### Clause `atomic_default_mem_order` (Section 10.5.1.1; p. 387)

Permitted on directives: `requires`.

**Arguments**

`````ignore
Name Type Properties
memory-order Keyword:acq_rel,
acquire,relaxed,
release,seq_cst
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `device_safesync` (Section 10.5.1.7; p. 393; properties: required, unique Members:)

Permitted on directives: `requires`.

**Arguments**

`````ignore
Name Type Properties
required expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `dynamic_allocators` (Section 10.5.1.2; p. 388)

Permitted on directives: `requires`.

**Arguments**

`````ignore
Name Type Properties
required expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `reverse_offload` (Section 10.5.1.3; p. 389)

Permitted on directives: `requires`.

**Arguments**

`````ignore
Name Type Properties
required expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `self_maps` (Section 10.5.1.6; p. 392)

Permitted on directives: `requires`.

**Arguments**

`````ignore
Name Type Properties
required expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `unified_address` (Section 10.5.1.4; p. 390)

Permitted on directives: `requires`.

**Arguments**

`````ignore
Name Type Properties
required expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `unified_shared_memory` (Section 10.5.1.5; p. 391)

Permitted on directives: `requires`.

**Arguments**

`````ignore
Name Type Properties
required expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `reverse` (Section 11.5; category: executable; association: loop nest; properties: generally-composable,)

### Clause `apply` (Section 11.1; pp. 403–404)

Permitted on directives: `fuse`, `interchange`, `nothing`, `reverse`, `split`, `stripe`, `tile`, `unroll`.

**Arguments**

`````ignore
Name Type Properties
applied-directives list of directive specifi-
cation list item type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
loop-modifier applied-directives Complex, Keyword:
fused,grid, identity,
interchanged,
intratile,offsets,
reversed, split,
unrolled
`````


## `scan` (Section 7.7; pp. 297–299; category: subsidiary; association: separating; properties: pure)

### Clause `exclusive` (Section 7.7.2; p. 300)

Permitted on directives: `scan`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `inclusive` (Section 7.7.1)

Permitted on directives: `scan`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `init_complete` (Section 7.7.3; p. 301)

Permitted on directives: `scan`.

**Arguments**

`````ignore
Name Type Properties
create_init_phase expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `scope` (Section 13.2; p. 437; category: executable; association: block; properties: work-distribution, team-)

### Clause `allocate` (Section 8.6; pp. 343–345)

Permitted on directives: `allocators`, `distribute`, `do`, `for`, `parallel`, `scope`, `sections`, `single`, `target`, `target_data`, `task`, `taskgroup`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
allocator-simple-
modifier
list expression of OpenMP allo-
cator_handle type
exclusive, unique
allocator-complex-
modifier
list Complex, name:
allocator
`````

### Clause `firstprivate` (Section 7.5.4; pp. 258–259)

Permitted on directives: `distribute`, `do`, `for`, `parallel`, `scope`, `sections`, `single`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
saved list Keyword:saved default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `nowait` (Section 17.6; pp. 512–513)

Permitted on directives: `dispatch`, `do`, `for`, `interop`, `scope`, `sections`, `single`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `taskwait`, `workshare`.

**Arguments**

`````ignore
Name Type Properties
do_not_synchronize expression of OpenMP
logical type
optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `private` (Section 7.5.3; pp. 256–257)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `parallel`, `scope`, `sections`, `simd`, `single`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `reduction` (Section 7.6.10; pp. 283–285)

Permitted on directives: `do`, `for`, `loop`, `parallel`, `scope`, `sections`, `simd`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
reduction-
identifier
all arguments An OpenMP reduction iden-
tifier
required, ultimate
reduction-modifier list Keyword:default,
inscan, task
default
original-sharing-
modifier
list Complex, name:original
`````


## `section` (Section 13.3.1; p. 439; category: subsidiary; association: separating; properties: default)

_No clauses are defined for this directive in the specification._

## `sections` (Section 13.3; p. 438; category: executable; association: block; properties: work-distribution, team-)

### Clause `allocate` (Section 8.6; pp. 343–345)

Permitted on directives: `allocators`, `distribute`, `do`, `for`, `parallel`, `scope`, `sections`, `single`, `target`, `target_data`, `task`, `taskgroup`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
allocator-simple-
modifier
list expression of OpenMP allo-
cator_handle type
exclusive, unique
allocator-complex-
modifier
list Complex, name:
allocator
`````

### Clause `firstprivate` (Section 7.5.4; pp. 258–259)

Permitted on directives: `distribute`, `do`, `for`, `parallel`, `scope`, `sections`, `single`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
saved list Keyword:saved default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `lastprivate` (Section 7.5.5; pp. 260–262)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `sections`, `simd`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
lastprivate-
modifier
list Keyword:conditional default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `nowait` (Section 17.6; pp. 512–513)

Permitted on directives: `dispatch`, `do`, `for`, `interop`, `scope`, `sections`, `single`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `taskwait`, `workshare`.

**Arguments**

`````ignore
Name Type Properties
do_not_synchronize expression of OpenMP
logical type
optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `private` (Section 7.5.3; pp. 256–257)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `parallel`, `scope`, `sections`, `simd`, `single`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `reduction` (Section 7.6.10; pp. 283–285)

Permitted on directives: `do`, `for`, `loop`, `parallel`, `scope`, `sections`, `simd`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
reduction-
identifier
all arguments An OpenMP reduction iden-
tifier
required, ultimate
reduction-modifier list Keyword:default,
inscan, task
default
original-sharing-
modifier
list Complex, name:original
`````


## `simd` (Section 12.4; p. 430; category: executable; association: loop nest; properties: context-matching, order-)

### Clause `aligned` (Section 7.12; p. 331)

Permitted on directives: `declare_simd`, `simd`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
alignment list OpenMP integer expression positive, region
invariant, ultimate,
unique
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `collapse` (Section 6.4.5; p. 236)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `simd`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
n expression of integer
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `if` (Section 5.5; p. 210)

Permitted on directives: `cancel`, `parallel`, `simd`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `task_iteration`, `taskgraph`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
if-expression expression of OpenMP
logical type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `induction` (Section 7.6.13; pp. 288–290)

Permitted on directives: `distribute`, `do`, `for`, `simd`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
induction-
identifier
list OpenMP induction identifier required, ultimate
step-modifier list Complex, name:step
`````

### Clause `lastprivate` (Section 7.5.5; pp. 260–262)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `sections`, `simd`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
lastprivate-
modifier
list Keyword:conditional default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `linear` (Section 7.5.6; pp. 263–265)

Permitted on directives: `declare_simd`, `do`, `for`, `simd`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
step-simple-
modifier
list OpenMP integer expression exclusive, region-
invariant, unique
step-complex-
modifier
list Complex, name:step
`````

### Clause `nontemporal` (Section 12.4.1; p. 431)

Permitted on directives: `simd`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `order` (Section 12.3; pp. 428–429)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `simd`.

**Arguments**

`````ignore
Name Type Properties
ordering Keyword:
concurrent
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
order-modifier ordering Keyword:reproducible,
unconstrained
default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `private` (Section 7.5.3; pp. 256–257)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `parallel`, `scope`, `sections`, `simd`, `single`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `reduction` (Section 7.6.10; pp. 283–285)

Permitted on directives: `do`, `for`, `loop`, `parallel`, `scope`, `sections`, `simd`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
reduction-
identifier
all arguments An OpenMP reduction iden-
tifier
required, ultimate
reduction-modifier list Keyword:default,
inscan, task
default
original-sharing-
modifier
list Complex, name:original
`````

### Clause `safelen` (Section 12.4.2)

Permitted on directives: `simd`.

**Arguments**

`````ignore
Name Type Properties
length expression of integer
type
positive, constant
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `simdlen` (Section 12.4.3; p. 432)

Permitted on directives: `declare_simd`, `simd`.

**Arguments**

`````ignore
Name Type Properties
length expression of integer
type
positive, constant
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `single` (Section 13.1; p. 436; category: executable; association: block; properties: work-distribution, team-)

### Clause `allocate` (Section 8.6; pp. 343–345)

Permitted on directives: `allocators`, `distribute`, `do`, `for`, `parallel`, `scope`, `sections`, `single`, `target`, `target_data`, `task`, `taskgroup`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
allocator-simple-
modifier
list expression of OpenMP allo-
cator_handle type
exclusive, unique
allocator-complex-
modifier
list Complex, name:
allocator
`````

### Clause `copyprivate` (Section 7.8.2; pp. 303–309)

Permitted on directives: `single`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `firstprivate` (Section 7.5.4; pp. 258–259)

Permitted on directives: `distribute`, `do`, `for`, `parallel`, `scope`, `sections`, `single`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
saved list Keyword:saved default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `nowait` (Section 17.6; pp. 512–513)

Permitted on directives: `dispatch`, `do`, `for`, `interop`, `scope`, `sections`, `single`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `taskwait`, `workshare`.

**Arguments**

`````ignore
Name Type Properties
do_not_synchronize expression of OpenMP
logical type
optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `private` (Section 7.5.3; pp. 256–257)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `parallel`, `scope`, `sections`, `simd`, `single`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `split` (Section 11.6; p. 408; category: executable; association: loop nest; properties: generally-composable,)

### Clause `apply` (Section 11.1; pp. 403–404)

Permitted on directives: `fuse`, `interchange`, `nothing`, `reverse`, `split`, `stripe`, `tile`, `unroll`.

**Arguments**

`````ignore
Name Type Properties
applied-directives list of directive specifi-
cation list item type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
loop-modifier applied-directives Complex, Keyword:
fused,grid, identity,
interchanged,
intratile,offsets,
reversed, split,
unrolled
`````

### Clause `counts` (Section 11.6.1; p. 409)

Permitted on directives: `split`.

**Arguments**

`````ignore
Name Type Properties
count-list list of OpenMP integer
expression type
non-negative
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `stripe` (Section 11.7; p. 410; category: executable; association: loop nest; properties: loop-transforming, order-)

### Clause `apply` (Section 11.1; pp. 403–404)

Permitted on directives: `fuse`, `interchange`, `nothing`, `reverse`, `split`, `stripe`, `tile`, `unroll`.

**Arguments**

`````ignore
Name Type Properties
applied-directives list of directive specifi-
cation list item type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
loop-modifier applied-directives Complex, Keyword:
fused,grid, identity,
interchanged,
intratile,offsets,
reversed, split,
unrolled
`````

### Clause `sizes` (Section 11.2)

Permitted on directives: `stripe`, `tile`.

**Arguments**

`````ignore
Name Type Properties
size-list list of OpenMP integer
expression type
positive
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `target` (Section 15.8; pp. 491–495; category: executable; association: block; properties: parallelism-generating,)

### Clause `allocate` (Section 8.6; pp. 343–345)

Permitted on directives: `allocators`, `distribute`, `do`, `for`, `parallel`, `scope`, `sections`, `single`, `target`, `target_data`, `task`, `taskgroup`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
allocator-simple-
modifier
list expression of OpenMP allo-
cator_handle type
exclusive, unique
allocator-complex-
modifier
list Complex, name:
allocator
`````

### Clause `default` (Section 7.5.1; p. 254)

Permitted on directives: `parallel`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
data-sharing-attribute Keyword:
firstprivate,
none, private,
shared
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
variable-category implicit-behavior Keyword:aggregate,
all, allocatable,
pointer,scalar
default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `defaultmap` (Section 7.9.9; pp. 322–323)

Permitted on directives: `target`.

**Arguments**

`````ignore
Name Type Properties
implicit-behavior Keyword:default,
firstprivate,
from, none,
present,private,
self, storage,to,
tofrom
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
variable-category implicit-behavior Keyword:aggregate,
all, allocatable,
pointer,scalar
default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `depend` (Section 17.9.5; pp. 538–540)

Permitted on directives: `dispatch`, `interop`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `task_iteration`, `taskwait`.

**Arguments**

`````ignore
Name Type Properties
locator-list list of locator list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
task-dependence-
type
all arguments Keyword:depobj, in,
inout,inoutset,
mutexinoutset, out
unique
iterator locator-list Complex, name:iterator
`````

### Clause `device` (Section 15.2; p. 482)

Permitted on directives: `dispatch`, `interop`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`.

**Arguments**

`````ignore
Name Type Properties
device-description expression of integer
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
device-modifier device-description Keyword:ancestor,
device_num
default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `device_type` (Section 15.1; p. 481)

Permitted on directives: `begin declare_target`, `declare_target`, `groupprivate`, `target`.

**Arguments**

`````ignore
Name Type Properties
device-type-description Keyword:any,host,
nohost
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `firstprivate` (Section 7.5.4; pp. 258–259)

Permitted on directives: `distribute`, `do`, `for`, `parallel`, `scope`, `sections`, `single`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
saved list Keyword:saved default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `has_device_addr` (Section 7.5.9; p. 268)

Permitted on directives: `dispatch`, `target`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `if` (Section 5.5; p. 210)

Permitted on directives: `cancel`, `parallel`, `simd`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `task_iteration`, `taskgraph`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
if-expression expression of OpenMP
logical type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `in_reduction` (Section 7.6.12; p. 287)

Permitted on directives: `target`, `target_data`, `task`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
reduction-
identifier
all arguments An OpenMP reduction iden-
tifier
required, ultimate
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `is_device_ptr` (Section 7.5.7; p. 266)

Permitted on directives: `dispatch`, `target`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `map` (Section 7.9.6; pp. 310–319)

Permitted on directives: `declare_mapper`, `target`, `target_data`, `target_enter_data`, `target_exit_data`.

**Arguments**

`````ignore
Name Type Properties
locator-list list of locator list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
always-modifier locator-list Keyword:always map-type-
modifying
close-modifier locator-list Keyword:close map-type-
modifying
present-modifier locator-list Keyword:present map-type-
modifying
self-modifier locator-list Keyword:self map-type-
modifying
ref-modifier all arguments Keyword:ref_ptee,
ref_ptr,ref_ptr_ptee
unique
delete-modifier locator-list Keyword:delete map-type-
modifying
mapper locator-list Complex, name:mapper
`````

### Clause `nowait` (Section 17.6; pp. 512–513)

Permitted on directives: `dispatch`, `do`, `for`, `interop`, `scope`, `sections`, `single`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `taskwait`, `workshare`.

**Arguments**

`````ignore
Name Type Properties
do_not_synchronize expression of OpenMP
logical type
optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `priority` (Section 14.9; p. 474)

Permitted on directives: `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `taskgraph`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
priority-value expression of integer
type
constant, non-negative
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `private` (Section 7.5.3; pp. 256–257)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `parallel`, `scope`, `sections`, `simd`, `single`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `replayable` (Section 14.6; p. 471)

Permitted on directives: `target`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `taskloop`, `taskwait`.

**Arguments**

`````ignore
Name Type Properties
replayable-expression expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `thread_limit` (Section 15.3; pp. 483–484)

Permitted on directives: `target`, `teams`.

**Arguments**

`````ignore
Name Type Properties
threadlim expression of integer
type
positive
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `uses_allocators` (Section 8.8; pp. 346–355)

Permitted on directives: `target`.

**Arguments**

`````ignore
Name Type Properties
allocator expression of allocator_-
handle type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
mem-space allocator Complex, name:memspace
`````


## `target_data` (Section 15.7; pp. 489–490; category: executable; association: block; properties: device, device-affecting,)

### Clause `affinity` (Section 14.10; p. 475)

Permitted on directives: `target_data`, `task`, `task_iteration`.

**Arguments**

`````ignore
Name Type Properties
locator-list list of locator list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
iterator locator-list Complex, name:iterator
`````

### Clause `allocate` (Section 8.6; pp. 343–345)

Permitted on directives: `allocators`, `distribute`, `do`, `for`, `parallel`, `scope`, `sections`, `single`, `target`, `target_data`, `task`, `taskgroup`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
allocator-simple-
modifier
list expression of OpenMP allo-
cator_handle type
exclusive, unique
allocator-complex-
modifier
list Complex, name:
allocator
`````

### Clause `default` (Section 7.5.1; p. 254)

Permitted on directives: `parallel`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
data-sharing-attribute Keyword:
firstprivate,
none, private,
shared
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
variable-category implicit-behavior Keyword:aggregate,
all, allocatable,
pointer,scalar
default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `depend` (Section 17.9.5; pp. 538–540)

Permitted on directives: `dispatch`, `interop`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `task_iteration`, `taskwait`.

**Arguments**

`````ignore
Name Type Properties
locator-list list of locator list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
task-dependence-
type
all arguments Keyword:depobj, in,
inout,inoutset,
mutexinoutset, out
unique
iterator locator-list Complex, name:iterator
`````

### Clause `detach` (Section 14.11; p. 476)

Permitted on directives: `target_data`, `task`.

**Arguments**

`````ignore
Name Type Properties
event-handle variable of event_handle
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `device` (Section 15.2; p. 482)

Permitted on directives: `dispatch`, `interop`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`.

**Arguments**

`````ignore
Name Type Properties
device-description expression of integer
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
device-modifier device-description Keyword:ancestor,
device_num
default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `firstprivate` (Section 7.5.4; pp. 258–259)

Permitted on directives: `distribute`, `do`, `for`, `parallel`, `scope`, `sections`, `single`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
saved list Keyword:saved default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `if` (Section 5.5; p. 210)

Permitted on directives: `cancel`, `parallel`, `simd`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `task_iteration`, `taskgraph`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
if-expression expression of OpenMP
logical type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `in_reduction` (Section 7.6.12; p. 287)

Permitted on directives: `target`, `target_data`, `task`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
reduction-
identifier
all arguments An OpenMP reduction iden-
tifier
required, ultimate
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `map` (Section 7.9.6; pp. 310–319)

Permitted on directives: `declare_mapper`, `target`, `target_data`, `target_enter_data`, `target_exit_data`.

**Arguments**

`````ignore
Name Type Properties
locator-list list of locator list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
always-modifier locator-list Keyword:always map-type-
modifying
close-modifier locator-list Keyword:close map-type-
modifying
present-modifier locator-list Keyword:present map-type-
modifying
self-modifier locator-list Keyword:self map-type-
modifying
ref-modifier all arguments Keyword:ref_ptee,
ref_ptr,ref_ptr_ptee
unique
delete-modifier locator-list Keyword:delete map-type-
modifying
mapper locator-list Complex, name:mapper
`````

### Clause `mergeable` (Section 14.5)

Permitted on directives: `target_data`, `task`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
can_merge expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `nogroup` (Section 17.7; p. 514; properties: exclusive, unique Members:)

Permitted on directives: `target_data`, `taskgraph`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
do_not_synchronize expression of OpenMP
logical type
optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `nowait` (Section 17.6; pp. 512–513)

Permitted on directives: `dispatch`, `do`, `for`, `interop`, `scope`, `sections`, `single`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `taskwait`, `workshare`.

**Arguments**

`````ignore
Name Type Properties
do_not_synchronize expression of OpenMP
logical type
optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `priority` (Section 14.9; p. 474)

Permitted on directives: `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `taskgraph`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
priority-value expression of integer
type
constant, non-negative
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `private` (Section 7.5.3; pp. 256–257)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `parallel`, `scope`, `sections`, `simd`, `single`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `shared` (Section 7.5.2; p. 255)

Permitted on directives: `parallel`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `transparent` (Section 17.9.6; p. 541)

Permitted on directives: `target_data`, `task`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
impex-type expression of impex
OpenMP type
optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `use_device_addr` (Section 7.5.10; pp. 269–282)

Permitted on directives: `target_data`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `use_device_ptr` (Section 7.5.8; p. 267)

Permitted on directives: `target_data`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `target_enter_data` (Section 15.5; pp. 485–486; category: executable; association: unassociated; properties: parallelism-generating,)

### Clause `depend` (Section 17.9.5; pp. 538–540)

Permitted on directives: `dispatch`, `interop`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `task_iteration`, `taskwait`.

**Arguments**

`````ignore
Name Type Properties
locator-list list of locator list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
task-dependence-
type
all arguments Keyword:depobj, in,
inout,inoutset,
mutexinoutset, out
unique
iterator locator-list Complex, name:iterator
`````

### Clause `device` (Section 15.2; p. 482)

Permitted on directives: `dispatch`, `interop`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`.

**Arguments**

`````ignore
Name Type Properties
device-description expression of integer
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
device-modifier device-description Keyword:ancestor,
device_num
default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `if` (Section 5.5; p. 210)

Permitted on directives: `cancel`, `parallel`, `simd`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `task_iteration`, `taskgraph`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
if-expression expression of OpenMP
logical type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `map` (Section 7.9.6; pp. 310–319)

Permitted on directives: `declare_mapper`, `target`, `target_data`, `target_enter_data`, `target_exit_data`.

**Arguments**

`````ignore
Name Type Properties
locator-list list of locator list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
always-modifier locator-list Keyword:always map-type-
modifying
close-modifier locator-list Keyword:close map-type-
modifying
present-modifier locator-list Keyword:present map-type-
modifying
self-modifier locator-list Keyword:self map-type-
modifying
ref-modifier all arguments Keyword:ref_ptee,
ref_ptr,ref_ptr_ptee
unique
delete-modifier locator-list Keyword:delete map-type-
modifying
mapper locator-list Complex, name:mapper
`````

### Clause `nowait` (Section 17.6; pp. 512–513)

Permitted on directives: `dispatch`, `do`, `for`, `interop`, `scope`, `sections`, `single`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `taskwait`, `workshare`.

**Arguments**

`````ignore
Name Type Properties
do_not_synchronize expression of OpenMP
logical type
optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `priority` (Section 14.9; p. 474)

Permitted on directives: `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `taskgraph`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
priority-value expression of integer
type
constant, non-negative
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `replayable` (Section 14.6; p. 471)

Permitted on directives: `target`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `taskloop`, `taskwait`.

**Arguments**

`````ignore
Name Type Properties
replayable-expression expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `target_exit_data` (Section 15.6; pp. 487–488; category: executable; association: unassociated; properties: parallelism-generating,)

### Clause `depend` (Section 17.9.5; pp. 538–540)

Permitted on directives: `dispatch`, `interop`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `task_iteration`, `taskwait`.

**Arguments**

`````ignore
Name Type Properties
locator-list list of locator list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
task-dependence-
type
all arguments Keyword:depobj, in,
inout,inoutset,
mutexinoutset, out
unique
iterator locator-list Complex, name:iterator
`````

### Clause `device` (Section 15.2; p. 482)

Permitted on directives: `dispatch`, `interop`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`.

**Arguments**

`````ignore
Name Type Properties
device-description expression of integer
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
device-modifier device-description Keyword:ancestor,
device_num
default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `if` (Section 5.5; p. 210)

Permitted on directives: `cancel`, `parallel`, `simd`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `task_iteration`, `taskgraph`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
if-expression expression of OpenMP
logical type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `map` (Section 7.9.6; pp. 310–319)

Permitted on directives: `declare_mapper`, `target`, `target_data`, `target_enter_data`, `target_exit_data`.

**Arguments**

`````ignore
Name Type Properties
locator-list list of locator list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
always-modifier locator-list Keyword:always map-type-
modifying
close-modifier locator-list Keyword:close map-type-
modifying
present-modifier locator-list Keyword:present map-type-
modifying
self-modifier locator-list Keyword:self map-type-
modifying
ref-modifier all arguments Keyword:ref_ptee,
ref_ptr,ref_ptr_ptee
unique
delete-modifier locator-list Keyword:delete map-type-
modifying
mapper locator-list Complex, name:mapper
`````

### Clause `nowait` (Section 17.6; pp. 512–513)

Permitted on directives: `dispatch`, `do`, `for`, `interop`, `scope`, `sections`, `single`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `taskwait`, `workshare`.

**Arguments**

`````ignore
Name Type Properties
do_not_synchronize expression of OpenMP
logical type
optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `priority` (Section 14.9; p. 474)

Permitted on directives: `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `taskgraph`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
priority-value expression of integer
type
constant, non-negative
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `replayable` (Section 14.6; p. 471)

Permitted on directives: `target`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `taskloop`, `taskwait`.

**Arguments**

`````ignore
Name Type Properties
replayable-expression expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `target_update` (Section 15.9; pp. 496–498; category: executable; association: unassociated; properties: parallelism-generating,)

### Clause `depend` (Section 17.9.5; pp. 538–540)

Permitted on directives: `dispatch`, `interop`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `task_iteration`, `taskwait`.

**Arguments**

`````ignore
Name Type Properties
locator-list list of locator list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
task-dependence-
type
all arguments Keyword:depobj, in,
inout,inoutset,
mutexinoutset, out
unique
iterator locator-list Complex, name:iterator
`````

### Clause `device` (Section 15.2; p. 482)

Permitted on directives: `dispatch`, `interop`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`.

**Arguments**

`````ignore
Name Type Properties
device-description expression of integer
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
device-modifier device-description Keyword:ancestor,
device_num
default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `from` (Section 7.10.2; p. 329)

Permitted on directives: `target_update`.

**Arguments**

`````ignore
Name Type Properties
locator-list list of locator list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
present-modifier locator-list Keyword:present default
mapper locator-list Complex, name:mapper
`````

### Clause `if` (Section 5.5; p. 210)

Permitted on directives: `cancel`, `parallel`, `simd`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `task_iteration`, `taskgraph`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
if-expression expression of OpenMP
logical type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `nowait` (Section 17.6; pp. 512–513)

Permitted on directives: `dispatch`, `do`, `for`, `interop`, `scope`, `sections`, `single`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `taskwait`, `workshare`.

**Arguments**

`````ignore
Name Type Properties
do_not_synchronize expression of OpenMP
logical type
optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `priority` (Section 14.9; p. 474)

Permitted on directives: `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `taskgraph`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
priority-value expression of integer
type
constant, non-negative
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `replayable` (Section 14.6; p. 471)

Permitted on directives: `target`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `taskloop`, `taskwait`.

**Arguments**

`````ignore
Name Type Properties
replayable-expression expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `to` (Section 7.10.1; p. 328)

Permitted on directives: `target_update`.

**Arguments**

`````ignore
Name Type Properties
locator-list list of locator list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
present-modifier locator-list Keyword:present default
mapper locator-list Complex, name:mapper
`````


## `task` (Section 14.1; pp. 457–459; category: executable; association: block; properties: parallelism-generating,)

### Clause `affinity` (Section 14.10; p. 475)

Permitted on directives: `target_data`, `task`, `task_iteration`.

**Arguments**

`````ignore
Name Type Properties
locator-list list of locator list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
iterator locator-list Complex, name:iterator
`````

### Clause `allocate` (Section 8.6; pp. 343–345)

Permitted on directives: `allocators`, `distribute`, `do`, `for`, `parallel`, `scope`, `sections`, `single`, `target`, `target_data`, `task`, `taskgroup`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
allocator-simple-
modifier
list expression of OpenMP allo-
cator_handle type
exclusive, unique
allocator-complex-
modifier
list Complex, name:
allocator
`````

### Clause `default` (Section 7.5.1; p. 254)

Permitted on directives: `parallel`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
data-sharing-attribute Keyword:
firstprivate,
none, private,
shared
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
variable-category implicit-behavior Keyword:aggregate,
all, allocatable,
pointer,scalar
default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `depend` (Section 17.9.5; pp. 538–540)

Permitted on directives: `dispatch`, `interop`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `task_iteration`, `taskwait`.

**Arguments**

`````ignore
Name Type Properties
locator-list list of locator list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
task-dependence-
type
all arguments Keyword:depobj, in,
inout,inoutset,
mutexinoutset, out
unique
iterator locator-list Complex, name:iterator
`````

### Clause `detach` (Section 14.11; p. 476)

Permitted on directives: `target_data`, `task`.

**Arguments**

`````ignore
Name Type Properties
event-handle variable of event_handle
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `final` (Section 14.7; p. 472)

Permitted on directives: `task`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
finalize expression of OpenMP
logical type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `firstprivate` (Section 7.5.4; pp. 258–259)

Permitted on directives: `distribute`, `do`, `for`, `parallel`, `scope`, `sections`, `single`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
saved list Keyword:saved default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `if` (Section 5.5; p. 210)

Permitted on directives: `cancel`, `parallel`, `simd`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `task_iteration`, `taskgraph`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
if-expression expression of OpenMP
logical type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `in_reduction` (Section 7.6.12; p. 287)

Permitted on directives: `target`, `target_data`, `task`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
reduction-
identifier
all arguments An OpenMP reduction iden-
tifier
required, ultimate
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `mergeable` (Section 14.5)

Permitted on directives: `target_data`, `task`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
can_merge expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `priority` (Section 14.9; p. 474)

Permitted on directives: `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `taskgraph`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
priority-value expression of integer
type
constant, non-negative
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `private` (Section 7.5.3; pp. 256–257)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `parallel`, `scope`, `sections`, `simd`, `single`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `replayable` (Section 14.6; p. 471)

Permitted on directives: `target`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `taskloop`, `taskwait`.

**Arguments**

`````ignore
Name Type Properties
replayable-expression expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `shared` (Section 7.5.2; p. 255)

Permitted on directives: `parallel`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `threadset` (Section 14.8; p. 473)

Permitted on directives: `task`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
set Keyword:omp_pool,
omp_team
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `transparent` (Section 17.9.6; p. 541)

Permitted on directives: `target_data`, `task`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
impex-type expression of impex
OpenMP type
optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `untied` (Section 14.4; p. 470)

Permitted on directives: `task`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
can_change_threads expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `task_iteration` (Section 14.2.3; p. 465; category: subsidiary; association: unassociated; properties: default)

### Clause `affinity` (Section 14.10; p. 475)

Permitted on directives: `target_data`, `task`, `task_iteration`.

**Arguments**

`````ignore
Name Type Properties
locator-list list of locator list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
iterator locator-list Complex, name:iterator
`````

### Clause `depend` (Section 17.9.5; pp. 538–540)

Permitted on directives: `dispatch`, `interop`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `task_iteration`, `taskwait`.

**Arguments**

`````ignore
Name Type Properties
locator-list list of locator list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
task-dependence-
type
all arguments Keyword:depobj, in,
inout,inoutset,
mutexinoutset, out
unique
iterator locator-list Complex, name:iterator
`````

### Clause `if` (Section 5.5; p. 210)

Permitted on directives: `cancel`, `parallel`, `simd`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `task_iteration`, `taskgraph`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
if-expression expression of OpenMP
logical type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `taskgraph` (Section 14.3; pp. 466–468; category: executable; association: block; properties: default)

### Clause `graph_id` (Section 14.3.1)

Permitted on directives: `taskgraph`.

**Arguments**

`````ignore
Name Type Properties
graph-id-value expression of OpenMP
integer type
default
`````

_No modifiers specified._

### Clause `graph_reset` (Section 14.3.2; p. 469)

Permitted on directives: `taskgraph`.

**Arguments**

`````ignore
Name Type Properties
graph-reset-expression expression of OpenMP
logical type
default
`````

_No modifiers specified._

### Clause `if` (Section 5.5; p. 210)

Permitted on directives: `cancel`, `parallel`, `simd`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `task_iteration`, `taskgraph`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
if-expression expression of OpenMP
logical type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `nogroup` (Section 17.7; p. 514; properties: exclusive, unique Members:)

Permitted on directives: `target_data`, `taskgraph`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
do_not_synchronize expression of OpenMP
logical type
optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `priority` (Section 14.9; p. 474)

Permitted on directives: `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `taskgraph`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
priority-value expression of integer
type
constant, non-negative
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `taskgroup` (Section 17.4; p. 509; category: executable; association: block; properties: cancellable)

### Clause `allocate` (Section 8.6; pp. 343–345)

Permitted on directives: `allocators`, `distribute`, `do`, `for`, `parallel`, `scope`, `sections`, `single`, `target`, `target_data`, `task`, `taskgroup`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
allocator-simple-
modifier
list expression of OpenMP allo-
cator_handle type
exclusive, unique
allocator-complex-
modifier
list Complex, name:
allocator
`````

### Clause `task_reduction` (Section 7.6.11; p. 286)

Permitted on directives: `taskgroup`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
reduction-
identifier
all arguments An OpenMP reduction iden-
tifier
required, ultimate
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `taskloop` (Section 14.2; pp. 460–462; category: executable; association: loop nest; properties: parallelism-generating,)

### Clause `allocate` (Section 8.6; pp. 343–345)

Permitted on directives: `allocators`, `distribute`, `do`, `for`, `parallel`, `scope`, `sections`, `single`, `target`, `target_data`, `task`, `taskgroup`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
allocator-simple-
modifier
list expression of OpenMP allo-
cator_handle type
exclusive, unique
allocator-complex-
modifier
list Complex, name:
allocator
`````

### Clause `collapse` (Section 6.4.5; p. 236)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `simd`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
n expression of integer
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `default` (Section 7.5.1; p. 254)

Permitted on directives: `parallel`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
data-sharing-attribute Keyword:
firstprivate,
none, private,
shared
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
variable-category implicit-behavior Keyword:aggregate,
all, allocatable,
pointer,scalar
default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `final` (Section 14.7; p. 472)

Permitted on directives: `task`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
finalize expression of OpenMP
logical type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `firstprivate` (Section 7.5.4; pp. 258–259)

Permitted on directives: `distribute`, `do`, `for`, `parallel`, `scope`, `sections`, `single`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
saved list Keyword:saved default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `grainsize` (Section 14.2.1; p. 463)

Permitted on directives: `taskloop`.

**Arguments**

`````ignore
Name Type Properties
grain-size expression of integer
type
positive
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
prescriptiveness grain-size Keyword:strict unique
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `if` (Section 5.5; p. 210)

Permitted on directives: `cancel`, `parallel`, `simd`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `task_iteration`, `taskgraph`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
if-expression expression of OpenMP
logical type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `in_reduction` (Section 7.6.12; p. 287)

Permitted on directives: `target`, `target_data`, `task`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
reduction-
identifier
all arguments An OpenMP reduction iden-
tifier
required, ultimate
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `induction` (Section 7.6.13; pp. 288–290)

Permitted on directives: `distribute`, `do`, `for`, `simd`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
induction-
identifier
list OpenMP induction identifier required, ultimate
step-modifier list Complex, name:step
`````

### Clause `lastprivate` (Section 7.5.5; pp. 260–262)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `sections`, `simd`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
lastprivate-
modifier
list Keyword:conditional default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `mergeable` (Section 14.5)

Permitted on directives: `target_data`, `task`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
can_merge expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `nogroup` (Section 17.7; p. 514; properties: exclusive, unique Members:)

Permitted on directives: `target_data`, `taskgraph`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
do_not_synchronize expression of OpenMP
logical type
optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `num_tasks` (Section 14.2.2; p. 464)

Permitted on directives: `taskloop`.

**Arguments**

`````ignore
Name Type Properties
num-tasks expression of integer
type
positive
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
prescriptiveness num-tasks Keyword:strict unique
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `priority` (Section 14.9; p. 474)

Permitted on directives: `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `taskgraph`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
priority-value expression of integer
type
constant, non-negative
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `private` (Section 7.5.3; pp. 256–257)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `parallel`, `scope`, `sections`, `simd`, `single`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `reduction` (Section 7.6.10; pp. 283–285)

Permitted on directives: `do`, `for`, `loop`, `parallel`, `scope`, `sections`, `simd`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
reduction-
identifier
all arguments An OpenMP reduction iden-
tifier
required, ultimate
reduction-modifier list Keyword:default,
inscan, task
default
original-sharing-
modifier
list Complex, name:original
`````

### Clause `replayable` (Section 14.6; p. 471)

Permitted on directives: `target`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `taskloop`, `taskwait`.

**Arguments**

`````ignore
Name Type Properties
replayable-expression expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `shared` (Section 7.5.2; p. 255)

Permitted on directives: `parallel`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `threadset` (Section 14.8; p. 473)

Permitted on directives: `task`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
set Keyword:omp_pool,
omp_team
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `transparent` (Section 17.9.6; p. 541)

Permitted on directives: `target_data`, `task`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
impex-type expression of impex
OpenMP type
optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `untied` (Section 14.4; p. 470)

Permitted on directives: `task`, `taskloop`.

**Arguments**

`````ignore
Name Type Properties
can_change_threads expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `taskwait` (Section 17.5; pp. 510–511; category: executable; association: unassociated; properties: default)

### Clause `depend` (Section 17.9.5; pp. 538–540)

Permitted on directives: `dispatch`, `interop`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `task_iteration`, `taskwait`.

**Arguments**

`````ignore
Name Type Properties
locator-list list of locator list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
task-dependence-
type
all arguments Keyword:depobj, in,
inout,inoutset,
mutexinoutset, out
unique
iterator locator-list Complex, name:iterator
`````

### Clause `nowait` (Section 17.6; pp. 512–513)

Permitted on directives: `dispatch`, `do`, `for`, `interop`, `scope`, `sections`, `single`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `taskwait`, `workshare`.

**Arguments**

`````ignore
Name Type Properties
do_not_synchronize expression of OpenMP
logical type
optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `replayable` (Section 14.6; p. 471)

Permitted on directives: `target`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `taskloop`, `taskwait`.

**Arguments**

`````ignore
Name Type Properties
replayable-expression expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `taskyield` (Section 14.12; pp. 477–480; category: executable; association: unassociated; properties: default)

_No clauses are defined for this directive in the specification._

## `teams` (Section 12.2; pp. 425–427; category: executable; association: block; properties: parallelism-generating,)

### Clause `allocate` (Section 8.6; pp. 343–345)

Permitted on directives: `allocators`, `distribute`, `do`, `for`, `parallel`, `scope`, `sections`, `single`, `target`, `target_data`, `task`, `taskgroup`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
allocator-simple-
modifier
list expression of OpenMP allo-
cator_handle type
exclusive, unique
allocator-complex-
modifier
list Complex, name:
allocator
`````

### Clause `default` (Section 7.5.1; p. 254)

Permitted on directives: `parallel`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
data-sharing-attribute Keyword:
firstprivate,
none, private,
shared
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
variable-category implicit-behavior Keyword:aggregate,
all, allocatable,
pointer,scalar
default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `firstprivate` (Section 7.5.4; pp. 258–259)

Permitted on directives: `distribute`, `do`, `for`, `parallel`, `scope`, `sections`, `single`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
saved list Keyword:saved default
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `if` (Section 5.5; p. 210)

Permitted on directives: `cancel`, `parallel`, `simd`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `task`, `task_iteration`, `taskgraph`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
if-expression expression of OpenMP
logical type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `num_teams` (Section 12.2.1)

Permitted on directives: `teams`.

**Arguments**

`````ignore
Name Type Properties
upper-bound expression of integer
type
positive
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
lower-bound upper-bound OpenMP integer expression positive, ultimate,
unique
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `private` (Section 7.5.3; pp. 256–257)

Permitted on directives: `distribute`, `do`, `for`, `loop`, `parallel`, `scope`, `sections`, `simd`, `single`, `target`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `reduction` (Section 7.6.10; pp. 283–285)

Permitted on directives: `do`, `for`, `loop`, `parallel`, `scope`, `sections`, `simd`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
reduction-
identifier
all arguments An OpenMP reduction iden-
tifier
required, ultimate
reduction-modifier list Keyword:default,
inscan, task
default
original-sharing-
modifier
list Complex, name:original
`````

### Clause `shared` (Section 7.5.2; p. 255)

Permitted on directives: `parallel`, `target_data`, `task`, `taskloop`, `teams`.

**Arguments**

`````ignore
Name Type Properties
list list of variable list item
type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `thread_limit` (Section 15.3; pp. 483–484)

Permitted on directives: `target`, `teams`.

**Arguments**

`````ignore
Name Type Properties
threadlim expression of integer
type
positive
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `threadprivate` (Section 7.3; pp. 246–253; category: declarative; association: explicit; properties: pure)

_No clauses are defined for this directive in the specification._

## `tile` (Section 11.8; p. 411; category: executable; association: loop nest; properties: loop-transforming, order-)

### Clause `apply` (Section 11.1; pp. 403–404)

Permitted on directives: `fuse`, `interchange`, `nothing`, `reverse`, `split`, `stripe`, `tile`, `unroll`.

**Arguments**

`````ignore
Name Type Properties
applied-directives list of directive specifi-
cation list item type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
loop-modifier applied-directives Complex, Keyword:
fused,grid, identity,
interchanged,
intratile,offsets,
reversed, split,
unrolled
`````

### Clause `sizes` (Section 11.2)

Permitted on directives: `stripe`, `tile`.

**Arguments**

`````ignore
Name Type Properties
size-list list of OpenMP integer
expression type
positive
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `unroll` (Section 11.9; p. 412; category: executable; association: loop nest; properties: generally-composable,)

### Clause `apply` (Section 11.1; pp. 403–404)

Permitted on directives: `fuse`, `interchange`, `nothing`, `reverse`, `split`, `stripe`, `tile`, `unroll`.

**Arguments**

`````ignore
Name Type Properties
applied-directives list of directive specifi-
cation list item type
default
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
loop-modifier applied-directives Complex, Keyword:
fused,grid, identity,
interchanged,
intratile,offsets,
reversed, split,
unrolled
`````

### Clause `full` (Section 11.9.1; p. 413)

Permitted on directives: `unroll`.

**Arguments**

`````ignore
Name Type Properties
fully_unroll expression of OpenMP
logical type
constant, optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````

### Clause `partial` (Section 11.9.2; p. 414)

Permitted on directives: `unroll`.

**Arguments**

`````ignore
Name Type Properties
unroll-factor expression of integer
type
optional, constant, posi-
tive
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


## `workdistribute` (Section 13.5; pp. 443–446; category: executable; association: block; properties: work-distribution, parti-)

_No clauses are defined for this directive in the specification._

## `workshare` (Section 13.4; pp. 440–442; category: executable; association: block; properties: work-distribution, team-)

### Clause `nowait` (Section 17.6; pp. 512–513)

Permitted on directives: `dispatch`, `do`, `for`, `interop`, `scope`, `sections`, `single`, `target`, `target_data`, `target_enter_data`, `target_exit_data`, `target_update`, `taskwait`, `workshare`.

**Arguments**

`````ignore
Name Type Properties
do_not_synchronize expression of OpenMP
logical type
optional
`````

**Modifiers**

`````ignore
Name Modifies Type Properties
directive-name-
modifier
all arguments Keyword:directive-name(a
directive name)
unique
`````


