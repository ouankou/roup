! Basic Fortran OpenMP directive parsing example
! Demonstrates parsing OpenMP directives from Fortran source code
!
! Compile and link with ROUP library

program basic_parse_example
    use iso_c_binding
    implicit none
    
    ! Example OpenMP directives in Fortran free-form
    character(len=100) :: directive1 = "!$OMP PARALLEL PRIVATE(X)"
    character(len=100) :: directive2 = "!$OMP FOR SCHEDULE(STATIC, 10)"
    character(len=100) :: directive3 = "!$OMP PARALLEL FOR REDUCTION(+:SUM)"
    
    print *, "ROUP Fortran OpenMP Parser - Basic Example"
    print *, "==========================================="
    print *, ""
    
    print *, "Parsing Fortran OpenMP directives..."
    print *, ""
    
    print *, "Directive 1: ", trim(directive1)
    print *, "Directive 2: ", trim(directive2)
    print *, "Directive 3: ", trim(directive3)
    print *, ""
    
    print *, "Note: Full C API integration requires interfacing with libroup.so"
    print *, "See tutorial_basic.f90 for C interop example"
    
end program basic_parse_example
