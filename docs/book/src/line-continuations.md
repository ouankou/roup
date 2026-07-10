# Line continuations and physical spans

ROUP accepts standard C/C++ line splices and Fortran directive continuations.
It validates the complete logical directive before parsing and maps directive,
clause, and nested-directive spans back to the original physical source.

## C and C++

A C/C++ continuation is exactly a backslash immediately followed by LF or
CRLF. No whitespace may occur between the backslash and the line ending.
Translation removes exactly those characters and never invents a separator:

```c
#pragma omp parallel for \
    schedule(dynamic, 4) \
    private(i, \
            j)
```

Source whitespace on either side of the splice remains significant. Therefore
`parallel\` followed immediately by `for` forms the single token `parallelfor`
and is rejected; at least one actual source-space character is required to form
`parallel for`.

A backslash followed by spaces, a bare CR, or an uncontinued physical newline
with more directive text is a hard error.

## Fortran free and fixed forms

Fortran continuations use a trailing `&`. A continuation line may repeat an
OpenMP or OpenACC sentinel and may place `&` immediately after that sentinel.
Only continuation syntax is removed; any source whitespace needed to separate
tokens must be present in the input.

```fortran
!$omp target teams distribute &
!$omp& parallel do &
!$omp& private(i, j)
```

Fixed-form input accepts the configured standard sentinels, including `!$OMP`,
`C$OMP`, and `*$OMP`, subject to fixed-form placement rules. A comment may
follow a valid trailing `&`. Missing markers, text after a purported marker, or
another directive line reached through ordinary whitespace are errors.

## Spans

Logical parsing does not discard the physical location map. For a token that
crosses a C splice, its `Span` covers the complete physical slice, including the
backslash and newline. Clause-name spans identify the exact source alias even
when the semantic kind is canonicalized. Nested metadirective and construct
selector directives use the same outer-source coordinate system.

See `tests/openmp_line_continuations.rs` and
`tests/source_span_regressions.rs` for executable examples.
