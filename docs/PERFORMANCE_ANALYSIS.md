# IR ownership performance notes

Cloning directive strings while building the IR fixes dangling references from
continuation handling. The extra allocation happens only when a directive or
clause text actually needs to be normalised; the common fast path (no
continuations) remains borrow-only. Benchmarks show the difference is within the
noise floor, so the design favours correctness and predictable ownership over
micro-optimisations.
