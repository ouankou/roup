# Strict accparser fixture audit

This audit distinguishes adapter failures from intentionally rejected fixture
inputs. The upstream fixture corpus is retained unchanged in the pinned
submodule. It was written for a permissive parser and normalization-oriented
unparser, so its aggregate result is not the strict adapter's acceptance
contract.

## Validated contract

The local `strict_contract` test proves:

- exhaustive directive and clause ordinal conversion without display-name
  classification;
- numeric closed-enum conversion and hard errors for unknown numeric tags;
- exact directive and clause start line/column propagation;
- ordered copy modifiers and variable lists;
- typed `collapse`, `gang`, `tile`, `worker`, `vector`, and directive-form
  `wait` data, including tagged automatic `*` sizes;
- tagged device types and reduction operators, without string reparsing;
- typed cache array elements and contiguous subarrays, with scalar and strided
  items rejected;
- the closed standardized Fortran `end`-kind set and required named-routine
  payloads;
- `update host(...)` conversion to the same canonical `self` clause and payload
  as `update self(...)`;
- historical `pcopy` acceptance with canonical `copy` output;
- mandatory explicit C, C++, and Fortran host profiles, with pre-selection and
  source/profile mismatches rejected; and
- exceptions for empty, prefix-free, malformed, and unknown input.

Repository-owned strict equivalents retain the logical coverage of upstream
`lang_flag_test` and `compat_caller` while calling `setLang` explicitly.
Focused upstream `atomic` and `atomic_fortran` fixtures pass without correction. The
upstream cache fixtures contain only scalar names, which OpenACC does not
permit as cache items, so equivalent repository-owned cache and cache-Fortran
fixtures exercise array elements and contiguous subarrays instead.

These eight tests are the complete registered CTest gate: the three contract
executables, the no-string-enum source audit, two upstream atomic fixture
pairs, and two repository-owned cache fixture pairs. CMake does not import the
upstream test directory or glob OpenACC-VV sources. Every registered test is
mandatory and has a hard timeout; a missing fixture/reference pair fails
configuration. Input and reference hashes also make fixture drift a hard
configure-time re-audit requirement.

## Intentionally rejected fixture categories

The following exact upstream patterns are not valid inputs for the selected
host language or directive context and now fail immediately:

- C fixtures use C++ qualified expressions such as `readonly::m`,
  `zero::12`, and `max::x`.
- Fortran fixtures use C array-section spelling such as `x[0:N]` and `x[5]`.
- `routine` fixtures repeat the singleton `bind` clause.
- early `update` fixture lines contain only `async`, `device_type`, `wait`, or
  `if` and omit the required `host`, `device`, or `self` action.
- several locator lists use numeric literals (`12`, `23`, `34`) where a
  variable designator is required.
- the builtin cache fixtures list scalar names (`a`, `b`, and `c`) instead of
  the required array elements or contiguous subarrays.

These are hard errors. The adapter does not drop the offending item, switch
host languages, return null, or continue with a partial AST.

## Canonical output differences

The strict AST stores wait device and queue semantics, not whether the optional
`queues:` keyword was written. A wait parameter with `devnum:` therefore emits
the canonical explicit `queues:` form. The old reference files preserve that
surface distinction.

The adapter does not add an output rewrite to reproduce the old spelling. Any
remaining divergence in accparser's own `toString()` implementation is kept
visible instead of being repaired after conversion.

## Deliberate unsupported extension

The pinned accparser accepts an `indirect` clause, but it is not part of the
standardized OpenACC 1.0 through 3.4 surface implemented by ROUP. It remains a
hard parse error rather than entering the typed AST as an opaque extension.

## Upstream representation limits

The pinned accparser IR cannot preserve every typed ROUP value. The adapter
therefore hard-rejects these shapes instead of changing or dropping data:

- user-defined OpenACC reduction operators, because accparser exposes only a
  closed reduction enum;
- UTF-8, UTF-16, UTF-32, and wide prefixes on `bind` string literals, because
  accparser retains only a string-literal boolean (ordinary C/C++ and Fortran
  encodings are consumed and checked explicitly);
- a builtin `device_type` after a named type, because accparser stores builtin
  and named types in separate vectors and would reorder them; and
- duplicate device types, because accparser would silently deduplicate them.
