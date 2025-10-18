# ROUP Fortran Examples

Two examples demonstrate the current Fortran support:

- `basic_parse.f90` – standalone examples of OpenMP directive syntax in Fortran
- `tutorial_basic.f90` – shows how to call the ROUP C API from Fortran via `iso_c_binding`

> Fortran support is experimental and still evolving.

## Building

```bash
cargo build --release
cd examples/fortran
make            # builds both programs
make tutorial_basic
./tutorial_basic
```

`make basic_parse` builds the syntax-only sample. Adjust compiler flags in the Makefile if you need a different Fortran
compiler.

## Notes

- Supports both free-form (`!$OMP`) and fixed-form (`C$OMP`) sentinels.
- Callers are responsible for releasing resources through the C API wrappers.
- Some Fortran-specific constructs (e.g., complex array sections) are still being validated.

Refer to the Fortran tutorial in the documentation site for a deeper walkthrough.
