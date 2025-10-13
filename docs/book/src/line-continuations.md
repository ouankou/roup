# OpenMP Line Continuations

⚠️ **Experimental**: Continuation support is part of ROUP's evolving parser pipeline. The behaviour described here applies to the current working prototype and may change as the project matures.

OpenMP directives frequently span multiple source lines, either for readability or to respect column limits in Fortran. ROUP normalises these layouts before parsing so that callers can feed directives exactly as they appear in their source files. The normalisation happens inside the lexer and therefore works uniformly for the Rust API, the C API, and the ompparser compatibility layer.

## Supported Patterns

ROUP recognises three families of continuation syntax:

1. **C/C++ backslash-newline** – the standard preprocessor line continuation.
2. **Fortran free-form ampersand** – trailing `&` on a line, with an optional leading `&` on the continuation line.
3. **Fortran fixed-form sentinels** – repeated `!$OMP`, `C$OMP`, or `*$OMP` with an optional column-6 `&`.

Whitespace, comments, and Windows-style newlines (`\r\n`) are preserved for readability but ignored by the parser when they appear in continuation positions.

## C / C++ Example

```c
#pragma omp target \
    // Comments and indentation are fine
    map(to: a[0:N]) \
    map(from: b[0:N])
```

Key points:

- Every backslash must be the last non-whitespace character on the line.
- Line-local comments (`//` or `/* ... */`) between continued lines are ignored.
- After normalisation, the parser sees a single logical directive equivalent to `#pragma omp target map(to: ...) map(from: ...)`.

## Fortran Free-Form Examples

```fortran
!$omp parallel do &
  private(i, j) &
& schedule(static)
```

```fortran
!$omp target teams distribute &
!$omp& map(to: a(:)) &
!$omp& map(from: b(:))
```

Highlights:

- Trailing `&` characters, even when separated by spaces from the sentinel, mark a continuation.
- A leading `&` on the next line is optional; if present it is discarded automatically.
- Sentinel repetition (`!$omp`, `!$OMP`, etc.) on continuation lines is supported but not required.
- Indentation and blank lines between continued sections are tolerated.

## Fortran Fixed-Form Example

```fortran
      C$OMP PARALLEL DO&
      C$OMP& PRIVATE(I) &
      C$OMP  SCHEDULE(STATIC)
```

Rules of thumb:

- Continuation lines may use any of the standard sentinels (`C$OMP`, `!$OMP`, or `*$OMP`).
- Column 6 `&` markers are optional; when present they are removed by the lexer.
- Mixed-case sentinels are accepted, matching the behaviour of common Fortran compilers.

## Diagnostics and Edge Cases

- Continuations must follow standard OpenMP syntax. Non-standard tokens (e.g., a trailing comma without `&`) will surface as parse errors.
- Clauses that contain literal `&` characters (for example, bitwise expressions in C) continue to parse correctly because the lexer only treats an ampersand as a continuation when it appears at a line boundary.
- Unterminated continuations (a trailing backslash with no subsequent line, or a Fortran line ending with `&` at end-of-file) are reported as parser errors.

## Interoperability

- The ompparser compatibility layer uses the same normalisation pipeline. Tests in `compat/ompparser/tests/comprehensive_test.cpp` cover multi-line directives to guard against regressions.
- The C API (`roup_parse`) automatically enables continuation handling; callers do not need extra flags.

## Further Reading

- [Fortran Tutorial](./fortran-tutorial.md) – background on language modes and sentinel selection.
- [C Tutorial](./c-tutorial.md) – directive structure and clause registry overview.
