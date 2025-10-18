# Clang vs. Flang continuation handling

A quick comparison of how the LLVM front-ends normalise continuation lines helps
explain the current ROUP design.

## Clang (C/C++)

- Continuations are consumed by the lexer (`clang/lib/Lex/Lexer.cpp`).
- Tokens reference the original source buffer; the OpenMP parser never sees the
  `\
` sequence.
- No OpenMP-specific logic is required—tokens are already clean.

## Flang (Fortran)

- Continuations are resolved in the prescanner
  (`flang/lib/Parser/prescan.cpp`).
- The prescanner builds a fresh `TokenSequence`, so directive handling owns the
  merged text.

## Impact on ROUP

Earlier experiments used `Cow<'a, str>` to splice continuation lines, which left
IR structures borrowing from temporary buffers. The fix is to clone only when a
continuation is encountered so that the IR owns stable strings while the hot
path stays zero-copy. This hybrid model mirrors Flang’s safety with Clang’s
performance for the common case.
