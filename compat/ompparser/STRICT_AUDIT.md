# Strict ompparser compatibility audit

This file records standardized inputs rejected by the strict ROUP frontend or
lost by the typed adapter. It deliberately excludes unpaired scratch lines,
malformed legacy inputs, and spelling-only differences from the pinned
ompparser fixture corpus.

The C/C++ repros use ROUP's C23 host profile and the Fortran repros use its
Fortran 2023 free-form profile. The compatibility parser uses the `ANY` OpenMP
version policy so syntax from every supported historical specification remains
eligible.

## Executable gate manifest

The CMake gate is intentionally finite and deterministic. It registers the two
linkage examples, the comprehensive and strict contract executables, and the
explicit audited fixture list in `CMakeLists.txt`. That list covers atomic,
barrier, critical, C++ declare-mapper, end-declare-target, flush, parallel, scan,
taskgroup, taskwait, taskyield, and tile families, with the audited Fortran
variants. The legacy trailing atomic comma is rewritten only into an explicit
hard-error expectation; it is never accepted or removed from the evidence.

No fixture or source file is discovered by glob. Missing pinned files, changed
fixture hashes, and changed exact correction evidence fail configuration. A
parse exception, output mismatch, missing expectation, nonzero runner result,
or timeout fails CTest. The larger builtin, OpenMP-VV, and example corpora
remain evidence for future audit expansion; they are not silently skipped tests
and are not the strict adapter contract because they mix invalid legacy
acceptance, historical surface rendering, and whole-source harvesting.

## Frontend and legality issues fixed during this audit

The initial pinned-fixture failures exposed frontend bugs rather than adapter
workarounds. They are fixed in the typed parser and retained as public-API
regressions, primarily in `tests/openmp_historical_legality.rs`:

- `declare mapper` now accepts its required `map` clauses and stores the mapper
  type/declarator split structurally, including adjacent and spaced pointer
  stars. Missing variables, array declarators, and missing map clauses remain
  hard errors.
- Historical C/C++ and Fortran `declare target` extended lists remain accepted
  in later exact-version modes and use a dedicated typed parameter variant.
- Named Fortran `end critical` parameters and the standardized `nowait` and
  `copyprivate` end-directive clauses are retained instead of left unconsumed
  or rejected by the wrong clause table.
- `loop` and `simd` expose their standardized data-environment clauses, while
  `parallel loop` and `teams loop` retain the `bind` semantics inherited from
  their loop constituent.
- Historical `master taskloop` and `master taskloop simd` spellings remain
  cumulatively accepted and inherit their taskloop and SIMD clause sets.
- A `requires` directive retains each standardized requirement as an ordered
  typed clause. Unknown bare implementation spellings are hard errors rather
  than partially consumed extension payloads.

## Adapter issues fixed during this audit

- Directive and clause kinds are selected only from the C ABI's numeric
  OpenMP ordinals. The adapter has an explicit exhaustive mapping for all 194
  directive ordinals and all 132 clause ordinals at this audit revision; every
  end ordinal maps to a concrete paired construct, including `end allocators`
  and `end dispatch`. Names are never queried and unknown ordinals are hard
  errors.
- Closed clause semantics use scalar/list U32 fields, booleans use the
  dedicated boolean getter, and structured semantics use node families.
  Strings are restricted to identifiers, type names, expressions, and literal
  spellings required by ompparser's public IR API. CMake rejects reintroduced
  string comparisons, name reclassification helpers, closed-semantic string
  getters, legacy U64 boolean getters, and adapter passthrough calls.
- Combined atomic directive kinds (`atomic read`, `atomic write`, `atomic
  update`, `atomic capture`, and `atomic compare capture`) now create the
  corresponding typed ompparser operation clauses. Previously the operation
  disappeared from generated output.
- `partial(expression)` now consumes its expression field instead of passing
  through the bare-clause path. Bare `partial` is separately accepted by the
  frontend invariant.
- Directive and clause source locations now come from `RoupSpan`; the adapter
  no longer scans the source text to reconstruct line/column information.
- Detach event locators and ordered `sizes`/`counts`/`permutation` expression
  lists cross the ABI as typed fields rather than reconstructed source
  fragments.
- The compatibility process starts in `Lang_unknown`; `parseOpenMP` hard-errors
  before creating a parser or installing callbacks until the caller invokes
  `setLang` with C, C++, or Fortran.
- ROUP exposes `declare induction` as structured identifier/type nodes, but
  pinned ompparser can store its directive argument only through
  `OpenMPInductionClause::addPassthroughItem`. The adapter no longer rebuilds a
  raw signature or calls that API: structured `declare induction` conversion
  is a deliberate hard error until ompparser gains a typed representation.
- ROUP exposes `apply` as a generated-loop modifier plus complete nested
  directive nodes, `induction` as one identifier/step/variable-list payload,
  and `prefer_type` as typed preference-selector nodes. Pinned ompparser has
  only obsolete transform/binding bags and a raw preference string, so these
  conversions hard-error instead of unparsing semantic nodes into those APIs.
- Device-kind and implementation-vendor selector values are open identifiers
  in ROUP but closed enums in ompparser. Those two unrepresentable selector
  shapes hard-error instead of classifying identifier strings or substituting
  an `unknown` enum.

## Individually evidenced strict divergences

- `private(c/)` is malformed and is replaced by a local hard-negative test.
- Function calls such as `firstprivate(foo(x))` and
  `reduction(+: foo(x))` are not variable designators; valid locator fixtures
  replace them and local tests require hard errors for the malformed forms.
- `schedule(runtime, 2)` and `schedule(auto, 2)` illegally provide a chunk
  expression.
- `aligned(*a, &b, c:2)` uses expressions rather than aligned variables.
- Predefined allocators with traits, for example
  `uses_allocators(omp_default_mem_alloc(1234))`, are malformed.
- `proc_bind(master)` remains accepted as a historical spelling and is
  canonicalized to `proc_bind(primary)`.
- Host-expression whitespace differences are canonical-rendering differences,
  not adapter fallbacks; exact legacy expected strings are corrected only in
  generated outer fixtures.
