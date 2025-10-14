# Line Continuations

⚠️ Experimental: ROUP now understands multi-line OpenMP directives across C, C++, and Fortran. This guide shows how to format
continuations so the parser and ompparser compatibility layer recognize them reliably.

## When to use continuations

OpenMP pragmas often grow long once multiple clauses are attached. Rather than forcing everything onto a single line, you can
split directives while keeping source files readable. ROUP stitches the continued lines together during lexing, so downstream
APIs still observe canonical single-line directive strings.

Continuations are supported in two situations:

- **C / C++ pragmas** that end a line with a trailing backslash (`\`).
- **Fortran sentinels** (`!$OMP`, `C$OMP`, `*$OMP`) that use the standard ampersand (`&`) continuation syntax.

ROUP preserves clause order and trims whitespace that was introduced only to align indentation.

## C / C++ example

```c
#pragma omp parallel for \
    schedule(dynamic, 4) \
    private(i, \
            j)
```

ROUP merges this directive into `#pragma omp parallel for schedule(dynamic, 4) private(i, j)`. Clause arguments keep their
original spacing. Comments (`/* */` or `//`) may appear between continued lines and are ignored during merging.

### Tips for C / C++

- The backslash must be the final character on the line (aside from trailing spaces or tabs).
- Windows line endings (`\r\n`) are handled automatically.
- Keep at least one space between the directive name and the first clause on subsequent lines.

## Fortran free-form example

```fortran
!$omp target teams distribute &
!$omp parallel do &
!$omp& private(i, j)
```

The parser removes the continuation markers and produces `!$omp target teams distribute parallel do private(i, j)`.

### Fortran fixed-form example

```fortran
      C$OMP   DO &
      !$OMP& SCHEDULE(DYNAMIC) &
      !$OMP PRIVATE(I) SHARED(A)
```

Column prefixes (`!`, `C`, or `*`) are respected. ROUP normalizes the directive to `do schedule(DYNAMIC) private(I) shared(A)`.

### Tips for Fortran continuations

- Terminate every continued line with `&`. ROUP tolerates trailing comments (e.g., `& ! explanation`) and skips them automatically.
- You may repeat the sentinel on continuation lines (`!$OMP&`), or start the next line with only `&`. Both forms are accepted.
- Blank continuation lines are ignored as long as they contain only whitespace.
- Clause bodies can span multiple lines; nested continuations inside parentheses are collapsed to a single line in the parsed
  clause value.

## Troubleshooting

- **Missing continuation marker**: If a line break appears without `&` (Fortran) or `\` (C/C++), the parser treats the next line
  as a separate statement and reports an error or unexpected directive name.
- **Custom formatting macros**: Preprocessors that insert trailing spaces after `\` may break continuations. Ensure the backslash
  is the final printable character.
- **Compatibility layer**: The ompparser shim mirrors the same behavior. The comprehensive tests in
  `compat/ompparser/tests/comprehensive_test.cpp` include multi-line inputs for both languages.

For more examples, refer to the automated tests in `tests/openmp_line_continuations.rs` and the parser unit tests in
`src/parser/mod.rs`.
