# Fortran Tutorial

This tutorial demonstrates how to use ROUP to parse OpenMP directives in Fortran code.

## 🚧 Experimental Status

**Fortran support in ROUP is currently experimental.** Features may change as development progresses.

## Introduction

ROUP supports parsing OpenMP directives from Fortran source code in both free-form and fixed-form formats. This enables Fortran developers to:

- Parse OpenMP directives from Fortran source
- Validate directive syntax
- Extract clause information
- Build tools for Fortran+OpenMP code analysis

## Fortran Directive Formats

### Free-Form (Modern Fortran)

Free-form Fortran uses the `!$OMP` sentinel (case-insensitive):

```fortran
!$OMP PARALLEL PRIVATE(A, B) NUM_THREADS(4)
!$OMP END PARALLEL

!$omp parallel do private(i) reduction(+:sum)
!$OMP END PARALLEL DO
```

### Fixed-Form (Fortran 77)

Fixed-form Fortran uses `!$OMP` or `C$OMP` sentinels in columns 1-6:

```fortran
C$OMP PARALLEL PRIVATE(X, Y)
!$OMP DO SCHEDULE(STATIC, 10)
C$OMP END DO
C$OMP END PARALLEL
```

## Language Constants

ROUP provides language format constants for Fortran parsing:

```c
#define ROUP_LANG_C                0  // C/C++ (#pragma omp)
#define ROUP_LANG_FORTRAN_FREE     1  // Fortran free-form (!$OMP)
#define ROUP_LANG_FORTRAN_FIXED    2  // Fortran fixed-form (!$OMP or C$OMP)
```

## Using the Rust API

### Parsing Fortran Directives

```rust
use roup::lexer::Language;
use roup::parser::openmp;

// Create parser with Fortran free-form support
let parser = openmp::parser().with_language(Language::FortranFree);

// Parse a Fortran OpenMP directive
let input = "!$OMP PARALLEL PRIVATE(A, B) NUM_THREADS(4)";
let (rest, directive) = parser.parse(input).expect("parsing should succeed");

println!("Directive: {}", directive.name);
println!("Clauses: {}", directive.clauses.len());
```

### Supported Language Formats

```rust
use roup::lexer::Language;

// C/C++ format (default)
let c_parser = openmp::parser().with_language(Language::C);

// Fortran free-form
let fortran_free = openmp::parser().with_language(Language::FortranFree);

// Fortran fixed-form
let fortran_fixed = openmp::parser().with_language(Language::FortranFixed);
```

## Using the C API from Fortran

### Setting Up Fortran-C Interoperability

Create an interface module for ROUP C API:

```fortran
module roup_interface
    use iso_c_binding
    implicit none
    
    ! Language constants
    integer(c_int), parameter :: ROUP_LANG_FORTRAN_FREE = 1
    integer(c_int), parameter :: ROUP_LANG_FORTRAN_FIXED = 2
    
    interface
        ! Parse with language specification
        function roup_parse_with_language(input, language) &
            bind(C, name="roup_parse_with_language")
            use iso_c_binding
            type(c_ptr), value :: input
            integer(c_int), value :: language
            type(c_ptr) :: roup_parse_with_language
        end function roup_parse_with_language
        
        ! Free directive
        subroutine roup_directive_free(directive) &
            bind(C, name="roup_directive_free")
            use iso_c_binding
            type(c_ptr), value :: directive
        end subroutine roup_directive_free
        
        ! Get directive name
        function roup_directive_name(directive) &
            bind(C, name="roup_directive_name")
            use iso_c_binding
            type(c_ptr), value :: directive
            type(c_ptr) :: roup_directive_name
        end function roup_directive_name
        
        ! Get clause count
        function roup_directive_clause_count(directive) &
            bind(C, name="roup_directive_clause_count")
            use iso_c_binding
            type(c_ptr), value :: directive
            integer(c_size_t) :: roup_directive_clause_count
        end function roup_directive_clause_count
    end interface
end module roup_interface
```

### Parsing Fortran Directives from Fortran

```fortran
program parse_example
    use iso_c_binding
    use roup_interface
    implicit none
    
    type(c_ptr) :: directive_ptr, name_ptr
    character(len=100) :: input = "!$OMP PARALLEL PRIVATE(X)"
    character(kind=c_char), dimension(:), allocatable :: c_input
    integer :: i, n
    
    ! Convert Fortran string to C string
    n = len_trim(input)
    allocate(c_input(n+1))
    do i = 1, n
        c_input(i) = input(i:i)
    end do
    c_input(n+1) = c_null_char
    
    ! Parse directive
    directive_ptr = roup_parse_with_language(c_loc(c_input), &
                                              ROUP_LANG_FORTRAN_FREE)
    
    if (c_associated(directive_ptr)) then
        ! Get directive information
        name_ptr = roup_directive_name(directive_ptr)
        ! ... process name_ptr ...
        
        ! Clean up
        call roup_directive_free(directive_ptr)
    else
        print *, "Parse error"
    end if
    
    deallocate(c_input)
end program parse_example
```

## Case Insensitivity

Fortran is case-insensitive, and ROUP respects this:

```fortran
! All of these are equivalent
!$OMP PARALLEL PRIVATE(X)
!$omp parallel private(x)
!$Omp Parallel Private(X)
```

ROUP normalizes Fortran identifiers to lowercase internally while preserving the original case in the parsed output.

## Common Fortran OpenMP Constructs

### Parallel Regions

```fortran
!$OMP PARALLEL PRIVATE(TID) SHARED(N)
    ! Parallel code
!$OMP END PARALLEL
```

### Work-Sharing Constructs

In Fortran, use `DO` instead of C's `for`:

```fortran
!$OMP DO PRIVATE(I) SCHEDULE(STATIC, 10)
    do i = 1, n
        ! Loop body
    end do
!$OMP END DO
```

Or combined:

```fortran
!$OMP PARALLEL DO PRIVATE(I,J) REDUCTION(+:SUM)
    do i = 1, n
        sum = sum + array(i)
    end do
!$OMP END PARALLEL DO
```

### Array Sections

Fortran array sections use different syntax than C:

```fortran
!$OMP PARALLEL PRIVATE(A(1:N), B(:,1:M))
    ! Work with array sections
!$OMP END PARALLEL
```

### Common Blocks

Fortran's `THREADPRIVATE` can apply to common blocks:

```fortran
      COMMON /MYDATA/ X, Y, Z
!$OMP THREADPRIVATE(/MYDATA/)
```

## Examples

See the [`examples/fortran/`](../../examples/fortran/) directory for complete working examples:

- **basic_parse.f90**: Simple Fortran directive examples
- **tutorial_basic.f90**: Full C API integration tutorial

## Building Fortran Programs with ROUP

### Using gfortran

```bash
# Compile Fortran code
gfortran -c my_program.f90

# Link with ROUP library
gcc my_program.o -L/path/to/roup/target/release -lroup -o my_program

# Run (set LD_LIBRARY_PATH)
LD_LIBRARY_PATH=/path/to/roup/target/release ./my_program
```

### Makefile Example

```makefile
FC = gfortran
ROUP_LIB = -L../../target/release -lroup -Wl,-rpath,../../target/release

my_program: my_program.f90
	$(FC) -o $@ $< $(ROUP_LIB)
```

## Known Limitations

⚠️ **Current limitations in experimental Fortran support:**

1. **End Directives**: `!$OMP END PARALLEL` and similar end directives may not parse correctly
2. **Continuation Lines**: Line continuation with `&` is not fully implemented
3. **Array Sections**: Complex array section syntax may have issues
4. **Fixed-Form Column Rules**: Strict column 1-6 sentinel placement not enforced
5. **Fortran-Specific Directives**: Some Fortran-only directives (e.g., `WORKSHARE`) may not be registered

## Troubleshooting

### Parse Errors

If parsing fails:

1. **Check sentinel format**: Use `!$OMP` for free-form or `!$OMP`/`C$OMP` for fixed-form
2. **Verify case**: While case-insensitive, ensure proper formatting
3. **Check whitespace**: Ensure proper spacing after sentinel
4. **Use correct language mode**: Specify `ROUP_LANG_FORTRAN_FREE` or `ROUP_LANG_FORTRAN_FIXED`

### Directive Not Found

Some directives may not be in the registry. Check:

- Is the directive name correct? (Use `FOR` not `DO` for work-sharing)
- Is it a composite directive? (Use `PARALLEL FOR` not `PARALLEL` + `FOR`)

## API Reference

See:
- [C Tutorial](./c-tutorial.md) - C API documentation
- [API Reference](./api-reference.md) - Complete API listing
- [Architecture](./architecture.md) - Parser internals

## Contributing

Fortran support is under active development. Contributions welcome:

- Test with real Fortran+OpenMP code
- Report parsing issues
- Add more Fortran-specific test cases
- Improve documentation

## Further Reading

- [OpenMP 5.2 Specification](https://www.openmp.org/specifications/) - Official OpenMP standard
- [Fortran OpenMP Documentation](https://gcc.gnu.org/onlinedocs/gfortran/OpenMP.html) - GCC Fortran OpenMP guide
- [ISO C Binding](https://gcc.gnu.org/onlinedocs/gfortran/Interoperability-with-C.html) - Fortran-C interop guide
