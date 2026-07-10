# C examples

The examples include `crates/roup-capi/include/roup.h` and link the optional
`roup-capi` shared library. They exercise opaque handle ownership, structured
results, and recursive typed child nodes.

```bash
make -C examples/c clean all run-all BUILD_TYPE=release
```

Every unexpected status terminates with failure; no example substitutes a
default value or continues after a failed query.
