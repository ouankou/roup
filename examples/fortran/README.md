# ROUP Fortran Examples

This directory contains Fortran example programs demonstrating the use of ROUP (Rust-based OpenMP/OpenACC Unified Parser) with Fortran code.

## 🚧 Experimental Status

**Fortran support in ROUP is experimental and under active development.**

## Examples

### basic_parse.f90

Simple Fortran program demonstrating OpenMP directive formats in Fortran:
- Free-form `!$OMP` sentinel
- Various directive types
- Standalone example (no C API calls)

**Build:**
```bash
make basic_parse
./basic_parse
```

### tutorial_basic.f90

Comprehensive tutorial showing:
- Fortran-C interoperability with ROUP C API
- Parsing Fortran free-form OpenMP directives
- Extracting directive names and clause information
- Memory management with C API

**Build:**
```bash
make tutorial_basic
./tutorial_basic
```

## Building

### Prerequisites

- GNU Fortran compiler (`gfortran`)
- ROUP library built (run `cargo build --release` in project root)

### Quick Start

```bash
# Build all examples
make

# Or build specific example
make tutorial_basic

# Run
./tutorial_basic
```

## Fortran OpenMP Directive Formats

ROUP supports both Fortran free-form and fixed-form formats:

### Free-Form (Modern Fortran)

```fortran
!$OMP PARALLEL PRIVATE(A, B) NUM_THREADS(4)
!$OMP FOR SCHEDULE(STATIC, 10)
!$OMP END PARALLEL
```

### Fixed-Form (Fortran 77)

```fortran
C$OMP PARALLEL PRIVATE(A, B)
!$OMP DO SCHEDULE(DYNAMIC)
C$OMP END PARALLEL
```

## C API Integration

The tutorial demonstrates using ROUP's C API from Fortran via `iso_c_binding`:

```fortran
use iso_c_binding
use roup_interface

! Parse Fortran directive
directive_ptr = roup_parse_with_language(c_input, ROUP_LANG_FORTRAN_FREE)

! Extract information
name_ptr = roup_directive_name(directive_ptr)
num_clauses = roup_directive_clause_count(directive_ptr)

! Clean up
call roup_directive_free(directive_ptr)
```

## Language Constants

From `src/roup_constants.h`:

```c
#define ROUP_LANG_C                0  // C/C++ (#pragma omp)
#define ROUP_LANG_FORTRAN_FREE     1  // Fortran free-form (!$OMP)
#define ROUP_LANG_FORTRAN_FIXED    2  // Fortran fixed-form (!$OMP or C$OMP)
```

## Known Limitations

⚠️ **Experimental features:**
- End directives (e.g., `!$OMP END PARALLEL`) are not yet fully supported
- Array section syntax may have limited support
- Common block specifications in THREADPRIVATE need testing

## Further Reading

- [Fortran Tutorial](../../docs/book/src/fortran-tutorial.md) - Comprehensive guide
- [C Tutorial](../c/README.md) - C API reference
- [Architecture](../../docs/book/src/architecture.md) - Parser design

## Support

For issues or questions about Fortran support, please file an issue on the project repository.
