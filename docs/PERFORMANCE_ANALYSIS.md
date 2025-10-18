# Owning Strings in the IR

To avoid lifetime bugs the IR owns the directive and clause strings produced during parsing. The extra allocation only happens
when a directive spans multiple physical lines—continuations already require copying during normalisation. Directives without
continuations still travel through borrowed slices until the conversion step, where they are cloned once into the IR.

The end result matches Flang's safety guarantees while preserving Clang's fast path for the common case. Profiling confirmed that
the additional `String` allocations only affect the small subset of directives that needed normalisation in the first place.
