# Continuation Handling Reference

LLVM's front-ends treat OpenMP continuation lines differently:

- **Clang** normalises `\`-newline pairs inside the lexer, so downstream parsers work with a contiguous token stream that still
  borrows from the original buffer.
- **Flang** assembles continuations explicitly when prescanning Fortran directives, producing owned token sequences.

ROUP follows a hybrid model that keeps the fast path for directives without continuations while cloning the normalised text when
building the IR. This avoids dangling references while limiting allocations to the cases that actually cross physical lines. The
implementation lives in `src/lexer/`, `src/parser/` and `src/ir/convert.rs`; see the commit history around PR #27 for the design
trade-offs.
