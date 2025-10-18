# OpenMP Support

ROUP tracks the OpenMP 6.0 specification.  Keyword registration and parsing
behaviour are validated by the integration tests in `tests/` (notably
`openmp_keyword_coverage.rs`, `openmp_support_matrix.rs`, and the dedicated
loop-transform suites).

## Directive coverage

- Declarative constructs such as `declare variant`, `declare mapper`, and
  `declare induction`.
- Worksharing and tasking constructs (`parallel`, `for`/`do`, `taskloop`,
  `taskgraph`, `sections`, `single`, etc.).
- Device and teams directives including the various `target` forms, `teams`, and
  `distribute` combinations.
- Meta directives (`metadirective`, `begin metadirective`) and control flows
  such as `assumes`.
- Loop transformation directives (`tile`, `unroll`, `interchange`, `split`, and
  related constructs).

Each directive maps to the internal IR (see `src/ir/`) and is exercised by at
least one integration test.  Unsupported constructs are rejected explicitly so
they do not silently parse as unknown directives.

## Clause coverage

Clause parsing is language-aware: the parser retains enough information about
C/C++ and Fortran expressions to keep mapper expressions, array sections, and
reduction operators intact.  Examples include:

- Data sharing clauses (`private`, `firstprivate`, `lastprivate`, `shared`,
  `reduction`, `map`, `declare mapper`).
- Control clauses (`if`, `num_threads`, `schedule`, `ordered`, `nowait`,
  `collapse`).
- Device and memory clauses (`uses_allocators`, `detach`, `depend`,
  `has_device_addr`, `link`, `indirect`).
- Loop transformation clauses (`tile sizes(...)`, `unroll full`,
  `interchange permutation(...)`, etc.).

The clause registry is synchronised with the integration tests so new keywords
must be added together with coverage tests.

## Language awareness

The lexer accepts C/C++, fixed-form Fortran, and free-form Fortran input.  It
normalises line continuations, sentinel comments, and case differences before
handing the stream to the parser.  Language-specific behaviour is tested via the
`openmp_fortran.rs`, `openmp_fortran_sentinels.rs`, and `openmp_line_continuations.rs`
integration suites.

## Where to look next

- `src/parser/` – contains the grammar implementation used for both the Rust API
  and the C bindings.
- `src/ir/` – defines the intermediate representation that downstream tools can
  inspect.
- `tests/` – provides concrete examples for most directives and clauses.
