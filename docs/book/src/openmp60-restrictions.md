# OpenMP 6.0 Restrictions

This reference consolidates **all normative restrictions** for OpenMP 6.0 directives and clauses, essential for correct implementation and usage.

## Purpose

For developers implementing or using OpenMP 6.0, this document provides:
- **Complete restrictions** - All normative rules from the specification
- **Organized by construct** - Easy to find what rules apply to each directive/clause
- **Verbatim text** - Reproduced directly from the standard for accuracy
- **Implementation guidance** - What is forbidden, required, or conditional

## Organization

- Organized by directive and clause (alphabetically within categories)
- Each entry shows all restrictions for that construct
- Language-specific restrictions (C/C++ vs Fortran) are noted where applicable

## Usage

When implementing or using a directive/clause, consult this document to ensure compliance with all OpenMP 6.0 normative requirements. Violating these restrictions results in undefined behavior or non-conforming programs.

## Clause: `if` (Section 5.5; p. 210)

Restrictions to the if clause are as follows:
- At most one if clause can be specified that applies to the semantics of any construct or
constituent construct of a directive-specification.

## Clause: `init` (Section 5.6; pp. 211–212)

- init-var must not be constant.
- If the init clause appears on a depobj construct, init-var must refer to a variable of
dependOpenMP type that isuninitialized.
- If theinitclause appears on adepobjconstruct then thedepinfo-modifier has the
required property and otherwise it must not be present.
- If theinitclause appears on aninteropconstruct, init-var must refer to a variable of
interopOpenMP type.
- If theinitclause appears on aninteropconstruct, theinterop-typemodifier has the
required property and eachinterop-typekeyword has the unique property. Otherwise, the
interop-typemodifier must not be present.
- Theprefer-typemodifier must not be present unless theinitclause appears on an
interopconstruct.

## Clause: `destroy` (Section 5.7; pp. 213–235)

- destroy-varmust not be constant.
- If thedestroyclause appears on adepobjconstruct, destroy-varmust refer to a variable
ofdependOpenMP type that isinitialized.
- If thedestroyclause appears on aninteropconstruct, destroy-varmust refer to a
variable ofinteropOpenMP type that isinitialized.

## Clause: `collapse` (Section 6.4.5; p. 236)

- n must not evaluate to a value greater than the loop nest depth.

## Clause: `ordered` (Section 6.4.6; p. 237)

- None of the doacross-affected loops may be non-rectangular loops.
- n must not evaluate to a value greater than the depth of the associated loop nest.
- Ifn is explicitly specified and thecollapseclause is also specified for theordered
clause on the same construct,n must be greater than or equal to then specified for the
collapseclause.

## Clause: `looprange` (Section 6.4.7; pp. 238–245)

Restrictions to thelooprangeclause are as follows:
- first + count − 1 must not evaluate to a value greater than the loop sequence length of the
associated canonical loop sequence.

## Directive/Construct: `threadprivate` (Section 7.3; pp. 246–253)

Restrictions to thethreadprivatedirective are as follows:
- A thread must not reference a copy of a threadprivate variable that belongs to another thread.
- A threadprivate variable must not appear as the base variable of a list item in any clause
except for thecopyinand copyprivateclauses.
- An OpenMP program in which an untied task accesses threadprivate memory is
non-conforming.

## Clause: `default` (Section 7.5.1; p. 254)

Restrictions to thedefaultclause are as follows:
- Ifdata-sharing-attributeis none, each variable that is referenced in the construct and does
not have a predetermined data-sharing attribute must have an explicitly determined
data-sharing attribute.

## Clause: `shared` (Section 7.5.2; p. 255)

_No explicit restrictions are stated in the specification section._

## Clause: `private` (Section 7.5.3; pp. 256–257)

Restrictions to theprivateclause are as specified in Section 7.4.

## Clause: `firstprivate` (Section 7.5.4; pp. 258–259)

Restrictions to thefirstprivateclause are as follows:
- A list item that is private within aparallelregion must not appear in afirstprivate
clause on a worksharing construct if any of the worksharing regions that arise from the
worksharing construct ever bind to any of theparallelregions that arise from the
parallelconstruct.
- A list item that is private within ateamsregion must not appear in afirstprivate
clause on adistributeconstruct if any of thedistributeregions that arise from the
distributeconstruct ever bind to any of theteamsregions that arise from theteams
construct.
- A list item that appears in areductionclause on aparallelconstruct must not appear
in afirstprivateclause on ataskortaskloopconstruct if any of thetaskregions
thatarisefromthe taskortaskloopconstructeverbindtoanyofthe parallelregions
that arise from theparallelconstruct.

## Clause: `lastprivate` (Section 7.5.5; pp. 260–262)

Restrictions to thelastprivateclause are as follows:
- A list item must not appear in alastprivateclause on a work-distribution construct if
the corresponding region binds to the region of a parallelism-generating construct in which
the list item is private.
- A list item that appears in alastprivateclause with theconditionalmodifier must
be a scalar variable.

## Clause: `linear` (Section 7.5.6; pp. 263–265)

Restrictions to thelinearclause are as follows:
- If areductionclause with theinscanmodifier also appears on the construct, only
loop-iteration variables of affected loops may appear as list items in alinearclause.
- Alinear-modifier may be specified asrefor uvalonly forlinearclauses on
declare_simddirectives.
- For alinearclause that appears on a loop-nest-associated directive, the difference between
the value of a list item at the end of a collapsed iteration and its value at the beginning of the
collapsed iteration must be equal tolinear-step.
- If linear-modifier is uvalfor a list item in alinearclause that is specified on a
declare_simddirective and the list item is modified during a call to the SIMD version of
the procedure, the OpenMP program must not depend on the value of the list item upon
return from the procedure.
- If linear-modifier is uvalfor a list item in alinearclause that is specified on a
declare_simddirective, the OpenMP program must not depend on the storage of the
argument in the procedure being the same as the storage of the corresponding argument at the
callsite.

## Clause: `is_device_ptr` (Section 7.5.7; p. 266)

Restrictions to theis_device_ptrclause are as follows:
- Each list item must be a valid device pointer for the device data environment.

## Clause: `use_device_ptr` (Section 7.5.8; p. 267)

Restrictions to theuse_device_ptrclause are as follows:
- Each list item must be a C pointer for which the value is the address of an object that has
corresponding storage or is accessible on the target device.

## Clause: `has_device_addr` (Section 7.5.9; p. 268)

Restrictions to thehas_device_addrclause are as follows:
C / C++
- Each list item must have a valid device address for the device data environment.
C / C++
Fortran
- A list item must either have a valid device address for the device data environment, be an
unallocated allocatable variable, or be a disassociated data pointer.
- The association status of a list item that is a pointer must not be undefined unless it is a
structure component and it results from a predefined default mapper.
Fortran
## Clause: `use_device_addr` (Section 7.5.10; pp. 269–282)

Restrictions to theuse_device_addrclause are as follows:
- Each list item must have a corresponding list item in the device data environment or be
accessible on the target device.
- If a list item is an array section, the array base must be a base language identifier.

## Clause: `reduction` (Section 7.6.10; pp. 283–285)

Restrictions to thereductionclause are as follows:
- All restrictions common to all reduction clauses, as listed in Section 7.6.5 and Section 7.6.6,
apply to this clause.
- For a given construct on which the clause appears, the lifetime of all original list items must
extend at least until after the synchronization point at which the completion of the
corresponding region by all participants in the reduction can be observed by all participants.
- If theinscanreduction-modifier is specified on areductionclause that appears on a
worksharing construct and an original list item is private in the enclosing context of the
construct, the private copies must all have identical values when the construct is encountered.
- If thereductionclause appears on a worksharing construct and the
original-sharing-modifier specifiesdefaultas itssharingargument, each original list item
must be shared in the enclosing context unless it is determined not to be shared according to
the rules specified in Section 7.1.

## Clause: `task_reduction` (Section 7.6.11; p. 286)

Restrictions to thetask_reductionclause are as follows:
- All restrictions common to all reduction clauses, as listed in Section 7.6.5 and Section 7.6.6,
apply to this clause.

## Clause: `in_reduction` (Section 7.6.12; p. 287)

Restrictions to thein_reductionclause are as follows:
- All restrictions common to all reduction clauses, as listed in Section 7.6.5 and Section 7.6.6,
apply to this clause.
- For each list item, a matching list item must exist that appears in atask_reduction
clause or areductionclause with thetaskreduction-modifier that is specified on a
construct that corresponds to a region in which the region of the participating task is closely
nested. The construct that corresponds to the innermost enclosing region that meets this
condition must specify the samereduction-identifier for the matching list item as the
in_reductionclause.

## Clause: `induction` (Section 7.6.13; pp. 288–290)

Restrictions to theinductionclause are as follows:
- All restrictions listed in Section 7.6.5 apply to this clause.
- Theinduction-stepmust not be an array or array section.
- If an array section or array element appears as a list item in aninductionclause on a
worksharing construct, all threads of the team must specify the same storage location.
- None of the affected loops of a loop-nest-associated construct that has aninduction
clause may be a non-rectangular loop.

## Directive/Construct: `declare_reduction` (Section 7.6.14; pp. 291–292)

Restrictions to thedeclare_reductiondirective are as follows:
- A reduction identifier must not be re-declared in the current scope for the same type or for a
type that is compatible according to the base language rules.
- The type-name list must not declare new types.

## Clause: `combiner` (Section 7.6.15)

_No explicit restrictions are stated in the specification section._

## Clause: `initializer` (Section 7.6.16; p. 293)

_No explicit restrictions are stated in the specification section._

## Directive/Construct: `declare_induction` (Section 7.6.17; pp. 294–295)

Restrictions to thedeclare_inductiondirective are as follows:
- An induction identifier must not be re-declared in the current scope for the same type or for a
type that is compatible according to the base language rules.
- A type-name list item in thetype-specifier-listmust not declare a new type.

## Clause: `inductor` (Section 7.6.18; p. 296)

_No explicit restrictions are stated in the specification section._

## Clause: `collector` (Section 7.6.19)

_No explicit restrictions are stated in the specification section._

## Directive/Construct: `scan` (Section 7.7; pp. 297–299)

Restrictions to thescandirective are as follows:
- The separated construct must have at most onescandirective with aninclusiveor
exclusiveclause as a separating directive.
- The separated construct must have at most onescandirective with aninit_complete
clause as a separating directive.
- If specified, ascandirective with aninit_completeclause must precede ascan
directive with anexclusiveclause that is a subsidiary directive of the same construct.
- The affected loops of the separated construct must all be perfectly nested loops.
- Each list item that appears in theinclusiveorexclusiveclause must appear in a
reductionclause with theinscanmodifier on the separated construct.
- Each list item that appears in areductionclause with theinscanmodifier on the
separated construct must appear in a clause on thescanseparating directive.
- Cross-iteration dependences across different collapsed iterations of the separated construct
must not exist, except for dependences for the list items specified in aninclusiveor
exclusiveclause.
- Intra-iteration dependences from a statement in the structured block sequence that
immediately precedes ascandirective with aninclusiveor exclusiveclause to a
statement in the structured block sequence that follows thatscandirective must not exist,
except for dependences for the list items specified in that clause.
- The private copy of a list item that appears in theinclusiveorexclusiveclause must
not be modified in the scan phase.
- Any list item that appears in anexclusiveclause must not be modified or used in the
initialization phase.
- Statements in the initialization phase must only modify private variables. Any private
variables modified in the initialization phase must not be used in the scan phase.

## Clause: `inclusive` (Section 7.7.1)

_No explicit restrictions are stated in the specification section._

## Clause: `exclusive` (Section 7.7.2; p. 300)

_No explicit restrictions are stated in the specification section._

## Clause: `init_complete` (Section 7.7.3; p. 301)

_No explicit restrictions are stated in the specification section._

## Clause: `copyin` (Section 7.8.1; p. 302)

Restrictions to thecopyinclause are as follows:
- A list item that appears in acopyinclause must be threadprivate.

## Clause: `copyprivate` (Section 7.8.2; pp. 303–309)

Restrictions to thecopyprivateclause are as follows:
- All list items that appear in acopyprivateclause must be either threadprivate or private
in the enclosing context.

## Clause: `map` (Section 7.9.6; pp. 310–319)

Restrictions to themapclause are as follows:
- Two list items of themapclauses on the same construct must not share original storage
unless one of the following is true: they are the same list item, one is the containing structure
of the other, at least one is an assumed-size array, or at least one is implicitly mapped due to
the list item also appearing in ause_device_addrclause.
- If the same list item appears more than once inmapclauses on the same construct, themap
clauses must specify the samemapper modifier.
- A variable that is a groupprivate variable or a device-local variable must not appear as a list
item in amapclause.
- If a list item is an array or an array section, it must specify contiguous storage.
- If an expression that is used to form a list item in amapclause contains an iterator identifier
that is defined by aniterator modifier, the list item instances that would result from different
values of the iterator must not have the same containing array and must not have base
pointers that share original storage.
- If multiple list items are explicitly mapped on the same construct and have the same
containing array or have base pointers that share original storage, and if any of the list items
do not have corresponding list items that are present in the device data environment prior to a
task encountering the construct, then the list items must refer to the same array elements of
either the containing array or the implicit array of the base pointers.
- If any part of the original storage of a list item that is explicitly mapped by amapclause has
corresponding storage in the device data environment prior to a task encountering the
construct associated with themapclause, all of the original storage must have corresponding
storage in the device data environment prior to the task encountering the construct.

## Clause: `enter` (Section 7.9.7; p. 320)

Restrictions to theenterclause are as follows:
- Each list item must have a mappable type.
- Each list item must have static storage duration.

## Clause: `link` (Section 7.9.8; p. 321)

Restrictions to thelinkclause are as follows:
- Each list item must have a mappable type.
- Each list item must have static storage duration.

## Clause: `defaultmap` (Section 7.9.9; pp. 322–323)

Restrictions to thedefaultmapclause are as follows:
- A givenvariable-category may be specified in at most onedefaultmapclause on a
construct.
- If adefaultmapclause specifies theallvariable-category, no otherdefaultmap
clause may appear on the construct.
- Ifimplicit-behavior is none, each variable that is specified byvariable-category and is
referenced in the construct but does not have a predetermined data-sharing attribute and does
not appear in anenteror linkclause on adeclare_targetdirective must be
explicitly listed in a data-environment attribute clause on the construct.

## Directive/Construct: `declare_mapper` (Section 7.9.10; pp. 324–327)

Restrictions to thedeclare_mapperdirective are as follows:
- No instance oftypecan be mapped as part of the mapper, either directly or indirectly through
another base language type, except the instancevar that is passed as the list item. If a set of
declare_mapperdirectives results in a cyclic definition then the behavior is unspecified.
- The typemust not declare a new base language type.
- At least onemapclause that mapsvar or at least one element ofvar is required.
- Listitemsin mapclausesonthe declare_mapperdirectivemayonlyrefertothedeclared
variablevar and entities that could be referenced by a procedure defined at the same location.
- If amapper modifier is specified for amapclause, its parameter must bedefault.
- Multipledeclare_mapperdirectives that specify the samemapper-identifierfor the same
base language type or for compatible base language types, according to the base language
rules, must not appear in the same scope.

## Clause: `to` (Section 7.10.1; p. 328)

_No explicit restrictions are stated in the specification section._

## Clause: `from` (Section 7.10.2; p. 329)

_No explicit restrictions are stated in the specification section._

## Clause: `uniform` (Section 7.11; p. 330)

Restrictions to theuniformclause are as follows:
- Only named parameter list items can be specified in theparameter-list.

## Clause: `aligned` (Section 7.12; p. 331)

Restrictions to thealignedclause are as follows:
- If the clause appears on adeclare_simddirective, each list item must be a named
parameter list item of the associated procedure.

## Directive/Construct: `groupprivate` (Section 7.13; pp. 332–333)

Restrictions to thegroupprivatedirective are as follows:
- A task that executes in a particular contention group must not access the storage of a
groupprivate copy of the list item that is created for a different contention group.
- Avariablethatisdeclaredwithaninitializermustnotappearina groupprivatedirective.

## Clause: `local` (Section 7.14; pp. 334–339)

Restrictions to OpenMP memory spaces are as follows:
- Variables in theomp_const_mem_spacememory space may not be written.

## Clause: `align` (Section 8.3; p. 340)

Restrictions to thealignclause are as follows:
- alignment must evaluate to a power of two.

## Clause: `allocator` (Section 8.4)

_No explicit restrictions are stated in the specification section._

## Directive/Construct: `allocate` (Section 8.5; pp. 341–342)

Restrictions to theallocatedirective are as follows:
- An allocatedirective must appear in the same scope as the declarations of each of its list
items and must follow all such declarations.
- A declared variable may appear as a list item in at most oneallocatedirective in a given
compilation unit.
- allocatedirectives that appear in atargetregion must specify anallocatorclause
unless arequiresdirective with thedynamic_allocatorsclause is present in the
same compilation unit.

## Clause: `allocate` (Section 8.6; pp. 343–345)

Restrictions to theallocateclause are as follows:
- For any list item that is specified in theallocateclause on a directive other than the
allocatorsdirective, a data-sharing attribute clause that may create a private copy of that
list item must be specified on the same directive.
- Fortask,taskloopor targetdirectives, allocation requests to memory allocators with
the accesstrait set tothreadresult in unspecified behavior.
- allocateclauses that appear on atargetconstruct or on constructs in atargetregion
must specify anallocator-simple-modifieror allocator-complex-modifierunless a
requiresdirective with thedynamic_allocatorsclause is present in the same
compilation unit.

## Directive/Construct: `allocators` (Section 8.7)

Restrictions to theallocatorsconstruct are as follows:
- A list item that appears in anallocateclause must appear as one of the variables that is
allocated by theallocate-stmt in the associated allocator structured block.
- A list item must not be a coarray or have a coarray as an ultimate component.

## Clause: `uses_allocators` (Section 8.8; pp. 346–355)

- Theallocator expression must be a base language identifier.
- Ifallocator is an identifier that matches the name of a predefined allocator, no modifiers may
be specified.
- Ifallocator is not the name of a predefined allocator and is notomp_null_allocator, it
must be a variable.
- Theallocator argument must not appear in other data-sharing attribute clauses or
data-mapping attribute clauses on the same construct.

## Clause: `when` (Section 9.4.1; p. 356)

Restrictions to thewhenclause are as follows:
- directive-variantmust not specify a metadirective.
- context-selector must not specify any properties for thesimdtrait selector.

## Clause: `otherwise` (Section 9.4.2; pp. 357–360)

Restrictions to theotherwiseclause are as follows:
- directive-variantmust not specify a metadirective.

## Clause: `match` (Section 9.6.1; p. 361)

Restrictions to thematchclause are as follows:
- All variables that are referenced in an expression that appears in the context selector of a
matchclause must be accessible at each call site to the base function according to the base
language rules.

## Clause: `adjust_args` (Section 9.6.2; pp. 362–363)

- If theneed_device_addradjust-opmodifier is present and thehas-device-addr element
does not exist for a specified argument in the semantic requirement set of the current task, all
restrictions that apply to a list item in ause_device_addrclause also apply to the
corresponding argument that is passed by the call.

## Clause: `append_args` (Section 9.6.3; p. 364)

_No explicit restrictions are stated in the specification section._

## Directive/Construct: `declare_variant` (Section 9.6.4; pp. 365–366)

The restrictions to thedeclare_variantdirective are as follows:

## Directive/Construct: `begin declare_variant` (Section 9.6.5; p. 367)

The restrictions tobegin declare_variantdirective are as follows:
- matchclause must not contain asimdtrait selector.
- Twobegin declare_variantdirectives and their paired end directives must either
encompass disjoint source ranges or be perfectly nested.

## Directive/Construct: `dispatch` (Section 9.7; pp. 368–369)

Restrictions to thedispatchconstruct are as follows:
- If theinteropclause is present and has more than oneinterop-varthen thedevice
clause must also be present.

## Clause: `interop` (Section 9.7.1; p. 370)

Restrictions to theinteropclause are as follows:
- If theinteropclause is specified on adispatchconstruct, the matching
declare_variantdirective for thetarget-callmust have anappend_argsclause with
a number of list items that equals or exceeds the number of list items in theinteropclause.

## Clause: `novariants` (Section 9.7.2)

_No explicit restrictions are stated in the specification section._

## Clause: `nocontext` (Section 9.7.3; p. 371)

_No explicit restrictions are stated in the specification section._

## Directive/Construct: `declare_simd` (Section 9.8; pp. 372–373)

Restrictions to thedeclare_simddirective are as follows:
- The procedure body must be a structured block.
- The execution of the procedure, when called from a SIMD loop, must not result in the
execution of any constructs except foratomicconstructs andorderedconstructs on
which thesimdclause is specified.
- The execution of the procedure must not have any side effects that would alter its execution
for concurrent iterations of a SIMD chunk.

## Clause: `inbranch` (Section 9.8.1.1; p. 374)

_No explicit restrictions are stated in the specification section._

## Clause: `notinbranch` (Section 9.8.1.2; pp. 375–376)

Restrictions to any declare target directive are as follows:
- The same list item must not explicitly appear in both anenterclause on one declare target
directive and alinkorlocalclause on another declare target directive.
- The same list item must not explicitly appear in both alinkclause on one declare target
directive and alocalclause on another declare target directive.
- If a variable appears in aenterclause on a declare target directive, its initializer must not
refer to a variable that appears in alinkclause on a declare target directive.

## Directive/Construct: `declare_target` (Section 9.9.1; pp. 377–379)

Restrictions to thedeclare_targetdirective are as follows:
- If theextended-list argument is specified, no clauses may be specified.
- If the directive is not a declaration-associated directive and anextended-list argument is not
specified, a data-environment attribute clause must be present.
- A variable for whichnohostis specified must not appear in alinkclause.
- A groupprivate variable must not appear in anyenterclauses orlinkclauses.

## Directive/Construct: `begin declare_target` (Section 9.9.2; p. 380)

Restrictions to thebegin declare_targetdirective are as follows:

## Clause: `indirect` (Section 9.9.3; pp. 381–382)

Restrictions to theindirectclause are as follows:
- If invoked-by-fptr evaluates totrue, adevice_typeclause must not appear on the same
directive unless it specifiesanyfor itsdevice-type-description.

## Directive/Construct: `error` (Section 10.1; p. 383)

Restrictions to theerrordirective are as follows:
- The directive is pure only ifaction-time is compilation.

## Clause: `at` (Section 10.2)

_No explicit restrictions are stated in the specification section._

## Clause: `message` (Section 10.3; p. 384)

- If theaction-time is compilation,msg-stringmust be a constant expression.

## Clause: `severity` (Section 10.4; p. 385)

_No explicit restrictions are stated in the specification section._

## Directive/Construct: `requires` (Section 10.5; p. 386)

Restrictions to therequiresdirective are as follows:
- Arequiresdirective must appear lexically after the specification of a context selector in
which any clause of thatrequiresdirective is used, nor may the directive appear lexically
after any code that depends on such a context selector.

## Clause: `atomic_default_mem_order` (Section 10.5.1.1; p. 387)

Restrictions to theatomic_default_mem_orderclause are as follows:
- All requiresdirectives in the same compilation unit that specify the
atomic_default_mem_orderrequirement must specify the same argument.
- Any directive that specifies theatomic_default_mem_orderclause must not appear
lexically after anyatomicconstruct on which amemory-order clause is not specified.

## Clause: `dynamic_allocators` (Section 10.5.1.2; p. 388)

_No explicit restrictions are stated in the specification section._

## Clause: `reverse_offload` (Section 10.5.1.3; p. 389)

_No explicit restrictions are stated in the specification section._

## Clause: `unified_address` (Section 10.5.1.4; p. 390)

_No explicit restrictions are stated in the specification section._

## Clause: `unified_shared_memory` (Section 10.5.1.5; p. 391)

_No explicit restrictions are stated in the specification section._

## Clause: `self_maps` (Section 10.5.1.6; p. 392)

_No explicit restrictions are stated in the specification section._

## Clause: `device_safesync` (Section 10.5.1.7; p. 393)

The restrictions toassumptionclauses are as follows:
- Adirective-namelist item must not specify a directive that is a declarative directive, an
informational directive, or a metadirective.

## Clause: `absent` (Section 10.6.1.1; p. 394)

_No explicit restrictions are stated in the specification section._

## Clause: `contains` (Section 10.6.1.2)

_No explicit restrictions are stated in the specification section._

## Clause: `holds` (Section 10.6.1.3; p. 395)

_No explicit restrictions are stated in the specification section._

## Clause: `no_openmp` (Section 10.6.1.4; p. 396)

_No explicit restrictions are stated in the specification section._

## Clause: `no_openmp_constructs` (Section 10.6.1.5)

_No explicit restrictions are stated in the specification section._

## Clause: `no_openmp_routines` (Section 10.6.1.6; p. 397)

_No explicit restrictions are stated in the specification section._

## Clause: `no_parallelism` (Section 10.6.1.7; p. 398)

_No explicit restrictions are stated in the specification section._

## Directive/Construct: `assumes` (Section 10.6.2; p. 399)

The restrictions to theassumesdirective are as follows:

## Directive/Construct: `assume` (Section 10.6.3)

_No explicit restrictions are stated in the specification section._

## Directive/Construct: `begin assumes` (Section 10.6.4)

_No explicit restrictions are stated in the specification section._

## Directive/Construct: `nothing` (Section 10.7; pp. 400–402)

- Theapplyclause can be specified if and only if thenothingdirective forms a
loop-transforming construct.

## Clause: `apply` (Section 11.1; pp. 403–404)

Restrictions to theapplyclause are as follows:
- Each list item in theapplied-directiveslist of anyapplyclause must benothingor the
directive-specificationof a loop-nest-associated construct.
- The loop-transforming construct on which theapplyclause is specified must either have the
generally-composable property or every list item in theapplied-directiveslist of anyapply
clause must be thedirective-specificationof a loop-transforming directive.
- Every list item in theapplied-directiveslist of anyapplyclause that is specified on a
loop-transforming construct that is itself specified as a list item in theapplied-directiveslist
of anotherapplyclause must be thedirective-specificationof a loop-transforming directive.
- For a givenloop-modifier keyword, everyindices list item may appear at most once in any
applyclause on the directive.
- Everyindices list item must be a positive constant less than or equal tom, the number of
generated loops according to the specification of theloop-modifier keyword.
- The list items inindices must be in ascending order.
- If a directive does not define a defaultloop-modifier keyword, aloop-modifier is required.

## Clause: `sizes` (Section 11.2)

Restrictions to thesizesclause are as follows:
- The loop nest depth of the associated loop nest of the loop-transforming construct on which
the clause is specified must be greater than or equal tom.

## Directive/Construct: `fuse` (Section 11.3; p. 405)

_No explicit restrictions are stated in the specification section._

## Directive/Construct: `interchange` (Section 11.4; p. 406)

Restrictions to theinterchangeclause are as follows:
- No transformation-affected loops may be a non-rectangular loop.
- The transformation-affected loops must be perfectly nested loops.

## Clause: `permutation` (Section 11.4.1; p. 407)

Restrictions to thepermutationclause are as follows:
- Every integer from 1 ton must appear exactly once inpermutation-list.
- n must be at least 2.

## Directive/Construct: `reverse` (Section 11.5)

_No explicit restrictions are stated in the specification section._

## Directive/Construct: `split` (Section 11.6; p. 408)

The following restrictions apply to thesplitconstruct:
- Exactly one list item in thecountsclause must be the predefined identifieromp_fill.

## Clause: `counts` (Section 11.6.1; p. 409)

Restrictions to thecountsclause are as follows:
- A list item incount-list must be constant oromp_fill.

## Directive/Construct: `stripe` (Section 11.7; p. 410)

Restrictions to thestripeconstruct are as follows:
- The transformation-affected loops must be perfectly nested loops.
- No transformation-affected loops may be a non-rectangular loop.

## Directive/Construct: `tile` (Section 11.8; p. 411)

Restrictions to thetileconstruct are as follows:
- The transformation-affected loops must be perfectly nested loops.
- No transformation-affected loops may be a non-rectangular loop.

## Directive/Construct: `unroll` (Section 11.9; p. 412)

Restrictions to theunrolldirective are as follows:
- Theapplyclause can only be specified if thepartialclause is specified.

## Clause: `full` (Section 11.9.1; p. 413)

Restrictions to thefullclause are as follows:
- The iteration count of the transformation-affected loop must be constant.

## Clause: `partial` (Section 11.9.2; p. 414)

_No explicit restrictions are stated in the specification section._

## Directive/Construct: `parallel` (Section 12.1; pp. 415–418)

_No explicit restrictions are stated in the specification section._

## Clause: `num_threads` (Section 12.1.2; pp. 419–422)

_No explicit restrictions are stated in the specification section._

## Clause: `proc_bind` (Section 12.1.4; p. 423)

_No explicit restrictions are stated in the specification section._

## Clause: `safesync` (Section 12.1.5; p. 424)

Restrictions to thesafesyncclause are as follows:
- The widthargument must be asafesync-compatible expression.

## Directive/Construct: `teams` (Section 12.2; pp. 425–427)

Restrictions to theteamsconstruct are as follows:
- Ifa reduction-modifier isspecifiedina reductionclausethatappearsonthedirectivethen
the reduction-modifier must bedefault.
- Ateamsregion must be a strictly nested region of the implicit parallel region that surrounds
the whole OpenMP program or atargetregion. If ateamsregion is nested inside a
targetregion, the correspondingtargetconstruct must not contain any statements,
declarations or directives outside of the correspondingteamsconstruct.
- For ateamsconstruct that is an immediately nested construct of atargetconstruct, the
bounds expressions of any array sections and the index expressions of any array elements
used in any clause on the construct, as well as all expressions of any target-consistent
clauses on the construct, must be target-consistent expressions.

## Clause: `num_teams` (Section 12.2.1)

- lower-boundmust be less than or equal toupper-bound.

## Clause: `order` (Section 12.3; pp. 428–429)

Restrictions to theorderclause are as follows:
- The only routines for which a call may be nested inside a region that corresponds to a
construct on which theorderclause is specified withconcurrentas theordering
argument areorder-concurrent-nestable routines.
- Only regions that correspond toorder-concurrent-nestable constructs or
order-concurrent-nestable routines may be strictly nested regions of regions that
correspond to constructs on which theorderclause is specified withconcurrentas the
orderingargument.
- If a threadprivate variable is referenced inside a region that corresponds to a construct with
anorderclause that specifiesconcurrent, the behavior is unspecified.

## Directive/Construct: `simd` (Section 12.4; p. 430)

Restrictions to thesimdconstruct are as follows:
- If bothsimdlenand safelenclauses are specified, the value of thesimdlenlength
must be less than or equal to the value of thesafelenlength.
- Only SIMDizable constructs may be encountered during execution of asimdregion.
- If anorderclause that specifiesconcurrentappears on asimddirective, thesafelen
clause must not also appear.

## Clause: `nontemporal` (Section 12.4.1; p. 431)

_No explicit restrictions are stated in the specification section._

## Clause: `safelen` (Section 12.4.2)

_No explicit restrictions are stated in the specification section._

## Clause: `simdlen` (Section 12.4.3; p. 432)

_No explicit restrictions are stated in the specification section._

## Directive/Construct: `masked` (Section 12.5; p. 433)

_No explicit restrictions are stated in the specification section._

## Clause: `filter` (Section 12.5.1; pp. 434–435)

The following restrictions apply to work-distribution constructs:
- Each work-distribution region must be encountered by all threads in the binding thread set or
by none at all unless cancellation has been requested for the innermost enclosing parallel
region.
- The sequence of encountered work-distribution regions that have the same binding thread set
must be the same for every thread in the binding thread set.
- The sequence of encountered worksharing regions andbarrierregions that bind to the
same team must be the same for every thread in the team.

## Directive/Construct: `single` (Section 13.1; p. 436)

_No explicit restrictions are stated in the specification section._

## Directive/Construct: `scope` (Section 13.2; p. 437)

_No explicit restrictions are stated in the specification section._

## Directive/Construct: `sections` (Section 13.3; p. 438)

_No explicit restrictions are stated in the specification section._

## Directive/Construct: `section` (Section 13.3.1; p. 439)

_No explicit restrictions are stated in the specification section._

## Directive/Construct: `workshare` (Section 13.4; pp. 440–442)

Restrictions to theworkshareconstruct are as follows:
- The only OpenMP constructs that may be closely nested constructs of aworkshare
construct are theatomic,critical, andparallelconstructs.
- Base language statements that are encountered inside aworkshareconstruct but that are
not enclosed within aparallelor atomicconstruct that is nested inside the
workshareconstruct must consist of only the following:
– array assignments;
– scalar assignments;
– FORALLstatements;
– FORALLconstructs;
– WHEREstatements;
– WHEREconstructs; and
– BLOCKconstructs that are strictly structured blocks associated with directives.
- All array assignments, scalar assignments, and masked array assignments that are
encounteredinsidea workshareconstructbutarenotnestedinsidea parallelconstruct
that is nested inside theworkshareconstruct must be intrinsic assignments.
- The construct must not contain any user-defined function calls unless either the function is
pure and elemental or the function call is contained inside aparallelconstruct that is
nested inside theworkshareconstruct.

## Directive/Construct: `workdistribute` (Section 13.5; pp. 443–446)

Restrictions to theworkdistributeconstruct are as follows:
- Theworkdistributeconstruct must be a closely nested construct inside ateams
construct.
- No explicit region may be nested inside aworkdistributeregion.
- Base language statements that are encountered inside aworkdistributemust consist of
only the following:
– array assignments;
– scalar assignments; and
– calls to pure and elemental procedures.
- All array assignments and scalar assignments that are encountered inside a
workdistributeconstruct must be intrinsic assignments.
- The construct must not contain any calls to procedures that are not pure and elemental.
- If a threadprivate variable or groupprivate variable is referenced inside a
workdistributeregion, the behavior is unspecified.

## Directive/Construct: `for` (Section 13.6.1; p. 447)

_No explicit restrictions are stated in the specification section._

## Directive/Construct: `do` (Section 13.6.2; p. 448)

_No explicit restrictions are stated in the specification section._

## Clause: `schedule` (Section 13.6.3; pp. 449–450)

Restrictions to thescheduleclause are as follows:
- The scheduleclause cannot be specified if any of the collapsed loops is a non-rectangular
loop.
- The value of thechunk_sizeexpression must be the same for all threads in the team.
- If runtimeor autois specified forkind, chunk_sizemust not be specified.
- The nonmonotonicordering-modifier cannot be specified if anorderedclause is
specified on the same construct.

## Directive/Construct: `distribute` (Section 13.7; pp. 451–452)

Restrictions to thedistributeconstruct are as follows:
- The collapsed iteration space must the same for all teams in the league.
- The region that corresponds to thedistributeconstruct must be a strictly nested region
of ateamsregion.
- A list item may appear in afirstprivateor lastprivateclause, but not in both.
- The conditionallastprivate-modifier must not be specified.
- All list items that appear in aninductionclause must be private variables in the enclosing
context.

## Clause: `dist_schedule` (Section 13.7.1; p. 453)

Restrictions to thedist_scheduleclause are as follows:
- The value of thechunk_sizeexpression must be the same for all teams in the league.
- The dist_scheduleclause cannot be specified if any of the collapsed loops is a
non-rectangular loop.

## Directive/Construct: `loop` (Section 13.8; p. 454)

Restrictions to theloopconstruct are as follows:
- A list item must not appear in alastprivateclause unless it is the loop-iteration variable
of an affected loop.
- Ifa reduction-modifier isspecifiedina reductionclausethatappearsonthedirectivethen
the reduction-modifier must bedefault.
- If aloopconstruct is not nested inside another construct then thebindclause must be
present.
- If aloopregion binds to ateamsregion or parallel region, it must be encountered by all
threads in the binding thread set or by none of them.

## Clause: `bind` (Section 13.8.1; pp. 455–456)

Restrictions to thebindclause are as follows:
- If teamsis specified asbinding then the correspondingloopregion must be a strictly
nested region of ateamsregion.
- If teamsis specified asbinding and the correspondingloopregion executes on a non-host
device then the behavior of areductionclause that appears on the correspondingloop
construct is unspecified if the construct is not nested inside ateamsconstruct.
- If parallelis specified asbinding, the behavior is unspecified if the correspondingloop
region is a closely nested region of asimdregion.

## Directive/Construct: `task` (Section 14.1; pp. 457–459)

_No explicit restrictions are stated in the specification section._

## Directive/Construct: `taskloop` (Section 14.2; pp. 460–462)

Restrictions to thetaskloopconstruct are as follows:
- Thereduction-modifier must bedefault.
- The conditionallastprivate-modifier must not be specified.
- If thetaskloopconstruct is associated with atask_iterationdirective, none of the
taskloop-affected loops may be the generated loop of a loop-transforming construct.

## Clause: `grainsize` (Section 14.2.1; p. 463)

Restrictions to thegrainsizeclause are as follows:
- None of the collapsed loops may be non-rectangular loops.

## Clause: `num_tasks` (Section 14.2.2; p. 464)

Restrictions to thenum_tasksclause are as follows:
- None of the collapsed loops may be non-rectangular loops.

## Directive/Construct: `task_iteration` (Section 14.2.3; p. 465)

The restrictions to thetask_iterationdirective are as follows:
- Eachtask_iterationdirective must appear in the loop body of one of the
taskloop-affected loops and must precede all statements and directives (except other
task_iterationdirectives) in that loop body.
- If atask_iterationdirective appears in the loop body of one of the
taskloop-affected loops, no intervening code may occur between any two collapsed loops
of thetaskloop-affected loops.

## Directive/Construct: `taskgraph` (Section 14.3; pp. 466–468)

Restrictions to thetaskgraphconstruct are as follows:
- Task-generating constructs are the only constructs that may be encountered as part of the
taskgraphregion.
- Ataskgraphconstruct must not be encountered in a final task region.
- A replayable construct that generates an importing or exporting transparent task, a detachable
task, or an undeferred task must not be encountered in ataskgraphregion.
- Any variable referenced in a replayable construct that does not have static storage duration
and that does not exist in the enclosing data environment of thetaskgraphconstruct must
be a private-only or firstprivate variable in the replayable construct.
- A list item of a clause on a replayable construct that accepts a locator list and is not a
taskgraph-altering clause must have a base variable or base pointer.
- Any variable that appears in an expression of a variable list item or locator list item for a
clause on a replayable construct and does not designate the base variable or base pointer of
that list item must be listed in a data-environment attribute clause with thesaved modifier on
that construct.
- If a construct that permits thenogroupclause is encountered in ataskgraphregion then
the nogroupclause must be specified with thedo_not_synchronizeargument evaluating to
true.

## Clause: `graph_id` (Section 14.3.1)

_No explicit restrictions are stated in the specification section._

## Clause: `graph_reset` (Section 14.3.2; p. 469)

_No explicit restrictions are stated in the specification section._

## Clause: `untied` (Section 14.4; p. 470)

_No explicit restrictions are stated in the specification section._

## Clause: `mergeable` (Section 14.5)

_No explicit restrictions are stated in the specification section._

## Clause: `replayable` (Section 14.6; p. 471)

_No explicit restrictions are stated in the specification section._

## Clause: `final` (Section 14.7; p. 472)

_No explicit restrictions are stated in the specification section._

## Clause: `threadset` (Section 14.8; p. 473)

_No explicit restrictions are stated in the specification section._

## Clause: `priority` (Section 14.9; p. 474)

_No explicit restrictions are stated in the specification section._

## Clause: `affinity` (Section 14.10; p. 475)

_No explicit restrictions are stated in the specification section._

## Clause: `detach` (Section 14.11; p. 476)

Restrictions to thedetachclause are as follows:
- Ifa detachclauseappearsonadirective,thentheencounteringtaskmustnotbeafinaltask.
- A variable that appears in adetachclause cannot appear as a list item on any
data-environment attribute clause on the same construct.
- A variable that is part of an aggregate variable cannot appear in adetachclause.

## Directive/Construct: `taskyield` (Section 14.12; pp. 477–480)

_No explicit restrictions are stated in the specification section._

## Clause: `device_type` (Section 15.1; p. 481)

_No explicit restrictions are stated in the specification section._

## Clause: `device` (Section 15.2; p. 482)

- The ancestordevice-modifier must not appear on thedeviceclause on any directive
other than thetargetconstruct.
- If theancestordevice-modifier is specified, thedevice-descriptionmust evaluate to 1 and
a requiresdirective with thereverse_offloadclause must be specified;
- If thedevice_numdevice-modifier is specified andtarget-offload-varis notmandatory,
device-descriptionmust evaluate to a conforming device number.

## Clause: `thread_limit` (Section 15.3; pp. 483–484)

Restrictions to OpenMP device initialization are as follows:
- No thread may offload execution of a construct to a device until a dispatched
device_initializecallback completes.
- No thread may offload execution of a construct to a device after a dispatched
device_finalizecallback occurs.

## Directive/Construct: `target_enter_data` (Section 15.5; pp. 485–486)

Restrictions to thetarget_enter_dataconstruct are as follows:
- At least onemapclause must appear on the directive.
- All mapclauses must be map-entering clauses.

## Directive/Construct: `target_exit_data` (Section 15.6; pp. 487–488)

Restrictions to thetarget_exit_dataconstruct are as follows:
- At least onemapclause must appear on the directive.
- All mapclauses must be map-exiting clauses.

## Directive/Construct: `target_data` (Section 15.7; pp. 489–490)

_No explicit restrictions are stated in the specification section._

## Directive/Construct: `target` (Section 15.8; pp. 491–495)

Restrictions to thetargetconstruct are as follows:
- Device-affecting constructs, other thantargetconstructs for which theancestor
device-modifier is specified, must not be encountered during execution of atargetregion.
- The result of anomp_set_default_device,omp_get_default_device, or
omp_get_num_devicesroutine called within atargetregion is unspecified.
- The effect of an access to a threadprivate variable in atargetregion is unspecified.
- If a list item in amapclause is a structure element, any other element of that structure that is
referenced in thetargetconstruct must also appear as a list item in amapclause.
- A list item in amapclause that is specified on atargetconstruct must have a base variable
or base pointer.

## Directive/Construct: `target_update` (Section 15.9; pp. 496–498)

_No explicit restrictions are stated in the specification section._

## Directive/Construct: `interop` (Section 16.1; p. 499)

Restrictions to theinteropconstruct are as follows:
- Adependclause must only appear on the directive if theinterop-typeincludes
targetsync.
- An interoperability object must not be specified in more than oneaction-clause that appears
on theinteropconstruct.

## Clause: `use` (Section 16.1.2; pp. 500–502)

- The state ofinterop-varmust beinitialized.

## Clause: `hint` (Section 17.1; p. 503)

- hint-expr must evaluate to a valid synchronization hint.

## Directive/Construct: `critical` (Section 17.2; pp. 504–505)

Restrictions to thecriticalconstruct are as follows:
- Unlessomp_sync_hint_noneis specified in ahintclause, thecriticalconstruct
must specify a name.
- Thehint-expr that is specified in thehintclause on eachcriticalconstruct with the
same namemust evaluate to the same value.
- Acriticalregion must not be nested (closely or otherwise) inside acriticalregion
with the samename. This restriction is not sufficient to prevent deadlock.

## Directive/Construct: `barrier` (Section 17.3.1; pp. 506–508)

Restrictions to thebarrierconstruct are as follows:
- Eachbarrierregion must be encountered by all threads in a team or by none at all, unless
cancellation has been requested for the innermost enclosing parallel region.
- The sequence of worksharing regions andbarrierregions encountered must be the same
for every thread in a team.

## Directive/Construct: `taskgroup` (Section 17.4; p. 509)

_No explicit restrictions are stated in the specification section._

## Directive/Construct: `taskwait` (Section 17.5; pp. 510–511)

Restrictions to thetaskwaitconstruct are as follows:
- The mutexinoutsettask-dependence-typemay not appear in adependclause on a
taskwaitconstruct.

## Clause: `nowait` (Section 17.6; pp. 512–513)

Restrictions to thenowaitclause are as follows:
- Thedo_not_synchronizeargument must evaluate to the same value for all threads in the
binding thread set, if defined for the construct on which thenowaitclause appears.
- Thedo_not_synchronizeargument must evaluate to the same value for all tasks in the binding
task set, if defined for the construct on which thenowaitclause appears.

## Clause: `nogroup` (Section 17.7; p. 514)

_No explicit restrictions are stated in the specification section._

## Clause: `acq_rel` (Section 17.8.1.1; p. 515)

_No explicit restrictions are stated in the specification section._

## Clause: `acquire` (Section 17.8.1.2; p. 516)

_No explicit restrictions are stated in the specification section._

## Clause: `relaxed` (Section 17.8.1.3)

_No explicit restrictions are stated in the specification section._

## Clause: `release` (Section 17.8.1.4; p. 517)

_No explicit restrictions are stated in the specification section._

## Clause: `seq_cst` (Section 17.8.1.5; p. 518)

_No explicit restrictions are stated in the specification section._

## Clause: `read` (Section 17.8.2.1; p. 519)

_No explicit restrictions are stated in the specification section._

## Clause: `write` (Section 17.8.2.3; p. 520)

Restrictions to theextended-atomicclause group are as follows:
- The compareclause may not be specified such thatuse_semantics evaluates tofalseif the
weakclause is specified such thatuse_semantics evaluates totrue.

## Clause: `capture` (Section 17.8.3.1; p. 521)

_No explicit restrictions are stated in the specification section._

## Clause: `compare` (Section 17.8.3.2; p. 522)

_No explicit restrictions are stated in the specification section._

## Clause: `fail` (Section 17.8.3.3)

Restrictions to thefailclause are as follows:
- memorder may not beacq_relor release.

## Clause: `weak` (Section 17.8.3.4; p. 523)

_No explicit restrictions are stated in the specification section._

## Clause: `memscope` (Section 17.8.4; p. 524)

The restrictions for thememscopeclause are as follows:
- The binding thread set defined by thescope-specifier of thememscopeclause on an
atomicconstruct must be a subset of the atomic scope of the atomically accessed memory.
- The binding thread set defined by thescope-specifier of thememscopeclause on an
atomicconstruct must be a subset of all threads that are executing tasks in the contention
group if the size of the atomically accessed storage location is not 8, 16, 32, or 64 bits.

## Directive/Construct: `atomic` (Section 17.8.5; pp. 525–528)

Restrictions to theatomicconstruct are as follows:
- Constructs may not be encountered during execution of anatomicregion.
- If acaptureor compareclause is specified, theatomicclause must beupdate.
- If acaptureclause is specified but thecompareclause is not specified, an update-capture
structured block must be associated with the construct.
- If bothcaptureand compareclauses are specified, a conditional-update-capture
structured block must be associated with the construct.
- If acompareclause is specified but thecaptureclause is not specified, a
conditional-update structured block must be associated with the construct.
- Ifa writeclauseisspecified, awritestructuredblockmustbeassociatedwiththeconstruct.
- If areadclause is specified, a read structured block must be associated with the construct.
- If theatomicclause isreadthen thememory-order clause must not berelease.
- If theatomicclause iswritethen thememory-order clause must not beacquire.
- Theweakclause may only appear if the resulting atomic operation is an atomic conditional
update for which the comparison tests for equality.

## Directive/Construct: `flush` (Section 17.8.6; pp. 529–535)

Restrictions to theflushconstruct are as follows:
- If amemory-order clause is specified, thelist argument must not be specified.
- Thememory-order clause must not berelaxed.

## Directive/Construct: `depobj` (Section 17.9.3; p. 536)

_No explicit restrictions are stated in the specification section._

## Clause: `update` (Section 17.9.4; p. 537)

Restrictions to theupdateclause are as follows:
- task-dependence-typemust not bedepobj.
- The state ofupdate-var must beinitialized.
- If the locator list item represented byupdate-var is theomp_all_memoryreserved locator,
task-dependence-typemust be eitheroutor inout.

## Clause: `depend` (Section 17.9.5; pp. 538–540)

Restrictions to thedependclause are as follows:
- List items, other than reserved locators, used independclauses of the same task or
dependence-compatible tasks must indicate identical storage locations or disjoint storage
locations.
- List items used independclauses cannot be zero-length array sections.
- Theomp_all_memoryreserved locator can only be used in adependclause with anout
or inouttask-dependence-type.
- Array sections cannot be specified independclauses with thedepobj
task-dependence-type.
- List items used independclauses with thedepobjtask-dependence-typemust be
expressions of thedependOpenMP type that correspond to depend objects in theinitialized
state.
- List items that are expressions of thedependOpenMP type can only be used independ
clauses with thedepobjtask-dependence-type.

## Clause: `transparent` (Section 17.9.6; p. 541)

_No explicit restrictions are stated in the specification section._

## Clause: `doacross` (Section 17.9.7; pp. 542–544)

Restrictions to thedoacrossclause are as follows:
- If iteration-specifier is a loop-iteration vector that hasn elements, the innermost
loop-nest-associated construct that encloses the construct on which the clause appears must
specify anorderedclause for which the parameter value equalsn.
- If iteration-specifier is specified with theomp_cur_iterationkeyword and withsink
as thedependence-type then it must beomp_cur_iteration- 1.
- If iteration-specifier is specified withsourceas thedependence-typethen it must be
omp_cur_iteration.
- If iteration-specifier is a loop-iteration vector and thesinkdependence-typeis specified
then for each element, if the loop-iteration variablevari has an integral or pointer type, theith
expression of vector must be computable without overflow in that type for any value of vari
that can encounter the construct on which the doacross clause appears.
C++
- If iteration-specifier is a loop-iteration vector and thesinkdependence-typeis specified
then for each element, if the loop-iteration variablevari is of a random access iterator type
other than pointer type, theith expression of vector must be computable without overflow in
the type that would be used by std::distance applied to variables of the type of vari for
any value of vari that can encounter the construct on which the doacross clause appears.
C++

## Directive/Construct: `ordered` (Section 17.10.2; pp. 546–547)

Additional restrictions to the block-associatedorderedconstruct are as follows:
- The construct is SIMDizable only if thesimdparallelization-levelclause is specified.
- If thesimdparallelization-levelclause is specified, the binding region must correspond to a
construct for which thesimdconstruct is a leaf construct.
- If thethreadsparallelization-levelclause is specified, the binding region must correspond
to a construct for which a worksharing-loop construct is a leaf construct.
- If thethreadsparallelization-levelclause is specified and the binding region corresponds
to a compound construct then thesimdconstruct must not be a leaf construct unless the
simdparallelization-levelclause is also specified.
- During execution of the collapsed iteration associated with a loop-nest-associated directive, a
thread must not execute more than one block-associatedorderedregion that binds to the
corresponding region of the loop-nest-associated directive.
- An orderedclause with an argument value equal to the number of collapsed loops must
appear on the construct that corresponds to the binding region, if the binding region is not a
simdregion.

## Clause: `threads` (Section 17.10.3.1; p. 548)

_No explicit restrictions are stated in the specification section._

## Clause: `simd` (Section 17.10.3.2; pp. 549–550)

Restrictions to any clauses in thecancel-directive-nameclause group are as follows:
- Ifapply_to_directiveevaluates tofalseand anifclause is specified for the same constituent
construct, if-expressionmust evaluate tofalse.

## Directive/Construct: `cancel` (Section 18.2; pp. 551–554)

Restrictions to thecancelconstruct are as follows:
- The behavior for concurrent cancellation of a region and a region nested within it is
unspecified.
- Ifcancel-directive-nameis taskgroup, thecancelconstruct must be a closely nested
construct of ataskor ataskloopconstruct and thecancelregion must be a closely
nested region of ataskgroupregion.
- Ifcancel-directive-nameis nottaskgroup, thecancelconstruct must be a closely nested
construct of a construct that matchescancel-directive-name.

## Directive/Construct: `cancellation_point` (Section 18.3; pp. 555–556)

Restrictions to thecancellation pointconstruct are as follows:
- Acancellation_pointconstruct for whichcancel-directive-nameis taskgroup
must be a closely nested construct of ataskor taskloopconstruct, and the
cancellation_pointregion must be a closely nested region of ataskgroupregion.
- Acancellation_pointconstruct for whichcancel-directive-nameis nottaskgroup
must be a closely nested construct inside a construct that matchescancel-directive-name.

