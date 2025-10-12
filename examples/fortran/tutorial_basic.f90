! Comprehensive Fortran tutorial for ROUP OpenMP parser
! Demonstrates Fortran-C interoperability with ROUP C API
!
! Build instructions:
!   gfortran -c tutorial_basic.f90
!   gcc tutorial_basic.o -L../../target/release -lroup -o tutorial_basic
!   LD_LIBRARY_PATH=../../target/release ./tutorial_basic

module roup_interface
    use iso_c_binding
    implicit none
    
    ! Language constants from roup_constants.h
    integer(c_int), parameter :: ROUP_LANG_C = 0
    integer(c_int), parameter :: ROUP_LANG_FORTRAN_FREE = 1
    integer(c_int), parameter :: ROUP_LANG_FORTRAN_FIXED = 2
    
    ! Opaque types
    type, bind(C) :: OmpDirective
        type(c_ptr) :: ptr
    end type OmpDirective
    
    interface
        ! C standard library strlen for dynamic string sizing
        function strlen(s) bind(C, name="strlen")
            use iso_c_binding
            type(c_ptr), value :: s
            integer(c_size_t) :: strlen
        end function strlen
        
        ! Parse with language specification
        function roup_parse_with_language(input, language) bind(C, name="roup_parse_with_language")
            use iso_c_binding
            type(c_ptr), value :: input
            integer(c_int), value :: language
            type(c_ptr) :: roup_parse_with_language
        end function roup_parse_with_language
        
        ! Free directive
        subroutine roup_directive_free(directive) bind(C, name="roup_directive_free")
            use iso_c_binding
            type(c_ptr), value :: directive
        end subroutine roup_directive_free
        
        ! Get directive name
        function roup_directive_name(directive) bind(C, name="roup_directive_name")
            use iso_c_binding
            type(c_ptr), value :: directive
            type(c_ptr) :: roup_directive_name
        end function roup_directive_name
        
        ! Get clause count
        function roup_directive_clause_count(directive) bind(C, name="roup_directive_clause_count")
            use iso_c_binding
            type(c_ptr), value :: directive
            integer(c_size_t) :: roup_directive_clause_count
        end function roup_directive_clause_count
    end interface
    
contains
    
    ! Helper to convert Fortran string to C string
    function f_to_c_string(f_string) result(c_string)
        character(len=*), intent(in) :: f_string
        character(kind=c_char), dimension(:), allocatable :: c_string
        integer :: i, n
        
        n = len_trim(f_string)
        allocate(c_string(n+1))
        
        do i = 1, n
            c_string(i) = f_string(i:i)
        end do
        c_string(n+1) = c_null_char
    end function f_to_c_string
    
    ! Helper to convert C string to Fortran string with dynamic sizing
    function c_to_f_string(c_string_ptr) result(f_string)
        type(c_ptr), intent(in) :: c_string_ptr
        character(len=:), allocatable :: f_string
        character(kind=c_char), pointer :: c_string_array(:)
        integer(c_size_t) :: length
        integer :: i
        
        if (.not. c_associated(c_string_ptr)) then
            f_string = ""
            return
        end if
        
        ! Dynamically determine string length using C strlen
        ! No hardcoded buffer limit - handles arbitrarily long strings
        length = strlen(c_string_ptr)
        
        ! Create properly-sized pointer to C string array
        call c_f_pointer(c_string_ptr, c_string_array, [length])
        
        ! Allocate Fortran string with exact length needed
        allocate(character(len=int(length)) :: f_string)
        
        ! Copy characters from C string to Fortran string
        do i = 1, int(length)
            f_string(i:i) = c_string_array(i)
        end do
    end function c_to_f_string
    
end module roup_interface

program tutorial_basic
    use iso_c_binding
    use roup_interface
    implicit none
    
    type(c_ptr) :: directive_ptr, name_ptr
    character(len=:), allocatable :: directive_name
    integer(c_size_t) :: num_clauses
    
    ! Example Fortran free-form directives
    character(len=200) :: example1 = "!$OMP PARALLEL PRIVATE(A,B) NUM_THREADS(4)"
    character(len=200) :: example2 = "!$OMP FOR SCHEDULE(DYNAMIC) REDUCTION(+:SUM)"
    character(len=200) :: example3 = "!$OMP PARALLEL FOR PRIVATE(I,J)"
    
    print *, "==============================================="
    print *, "ROUP Fortran Tutorial - Basic Parsing"
    print *, "==============================================="
    print *, ""
    
    ! Example 1: Parse PARALLEL directive
    print *, "Example 1: Parsing Fortran PARALLEL directive"
    print *, "Input: ", trim(example1)
    
    directive_ptr = parse_fortran_directive(trim(example1))
    if (c_associated(directive_ptr)) then
        name_ptr = roup_directive_name(directive_ptr)
        directive_name = c_to_f_string(name_ptr)
        num_clauses = roup_directive_clause_count(directive_ptr)
        
        print *, "  Directive name: ", directive_name
        print *, "  Number of clauses: ", num_clauses
        
        call roup_directive_free(directive_ptr)
    else
        print *, "  ERROR: Failed to parse directive"
    end if
    print *, ""
    
    ! Example 2: Parse FOR directive
    print *, "Example 2: Parsing Fortran FOR directive"
    print *, "Input: ", trim(example2)
    
    directive_ptr = parse_fortran_directive(trim(example2))
    if (c_associated(directive_ptr)) then
        name_ptr = roup_directive_name(directive_ptr)
        directive_name = c_to_f_string(name_ptr)
        num_clauses = roup_directive_clause_count(directive_ptr)
        
        print *, "  Directive name: ", directive_name
        print *, "  Number of clauses: ", num_clauses
        
        call roup_directive_free(directive_ptr)
    else
        print *, "  ERROR: Failed to parse directive"
    end if
    print *, ""
    
    ! Example 3: Parse compound PARALLEL FOR directive
    print *, "Example 3: Parsing compound PARALLEL FOR directive"
    print *, "Input: ", trim(example3)
    
    directive_ptr = parse_fortran_directive(trim(example3))
    if (c_associated(directive_ptr)) then
        name_ptr = roup_directive_name(directive_ptr)
        directive_name = c_to_f_string(name_ptr)
        num_clauses = roup_directive_clause_count(directive_ptr)
        
        print *, "  Directive name: ", directive_name
        print *, "  Number of clauses: ", num_clauses
        
        call roup_directive_free(directive_ptr)
    else
        print *, "  ERROR: Failed to parse directive"
    end if
    print *, ""
    
    print *, "Tutorial complete!"
    
contains
    
    function parse_fortran_directive(input_str) result(dir_ptr)
        character(len=*), intent(in) :: input_str
        type(c_ptr) :: dir_ptr
        character(kind=c_char), dimension(:), allocatable, target :: c_str
        
        c_str = f_to_c_string(input_str)
        dir_ptr = roup_parse_with_language(c_loc(c_str), ROUP_LANG_FORTRAN_FREE)
    end function parse_fortran_directive
    
end program tutorial_basic
