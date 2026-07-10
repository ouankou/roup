# Fortran examples

`roup_capi.f90` declares the subset of the opaque-handle ABI used by these
examples with `ISO_C_BINDING`; the checked-in C header remains the complete ABI
definition. The two programs parse OpenMP and OpenACC free-form directives and
hard-error on every unexpected status or typed result.

```bash
make -C examples/fortran clean all run-all
```
