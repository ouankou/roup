# ROUP C Examples

This directory contains a small tutorial (`tutorial_basic.c`) that exercises the C API. The tutorial walks through parsing a
handful of directives, inspecting clauses, and handling memory management.

## Building

```bash
cargo build --release
cd examples/c
gcc -std=c11 -Wall -Wextra \
    -I../../target/release \
    tutorial_basic.c \
    -L../../target/release \
    -lroup \
    -o tutorial
```

Use `clang` if preferred. Set `LD_LIBRARY_PATH=../../target/release` (or your platform equivalent) before running the binary so
the dynamic loader can find `libroup`.

The tutorial output shows each step of the API walkthrough. Refer to the documentation site for a detailed explanation of the
functions involved.
