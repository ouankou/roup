module roup_capi
    use, intrinsic :: iso_c_binding
    implicit none
    private

    integer(c_int32_t), parameter, public :: ROUP_ABI_VERSION = 1
    integer(c_int32_t), parameter, public :: ROUP_STATUS_OK = 0
    integer(c_int32_t), parameter, public :: ROUP_DIALECT_OPENMP = 1
    integer(c_int32_t), parameter, public :: ROUP_DIALECT_OPENACC = 2
    integer(c_int32_t), parameter, public :: ROUP_VERSION_ANY = 0
    integer(c_int32_t), parameter, public :: ROUP_HOST_C = 1
    integer(c_int32_t), parameter, public :: ROUP_HOST_FORTRAN = 3
    integer(c_int32_t), parameter, public :: ROUP_C_23 = 23
    integer(c_int32_t), parameter, public :: ROUP_FORTRAN_2023 = 2023
    integer(c_int32_t), parameter, public :: ROUP_SOURCE_PRAGMA = 1
    integer(c_int32_t), parameter, public :: ROUP_SOURCE_FORTRAN_FREE = 2

    type, bind(C), public :: RoupParserHandle
        integer(c_int64_t) :: generation
        integer(c_int64_t) :: index
    end type

    type, bind(C), public :: RoupDirectiveHandle
        integer(c_int64_t) :: generation
        integer(c_int64_t) :: index
    end type

    type, bind(C), public :: RoupErrorHandle
        integer(c_int64_t) :: generation
        integer(c_int64_t) :: index
    end type

    type, bind(C), public :: RoupCallResult
        integer(c_int32_t) :: status
        integer(c_int32_t) :: reserved
        type(RoupErrorHandle) :: error
    end type

    type, bind(C), public :: RoupParserOptions
        integer(c_int32_t) :: abi_version
        integer(c_int32_t) :: struct_size
        integer(c_int32_t) :: dialect
        integer(c_int32_t) :: version_policy
        integer(c_int32_t) :: version
        integer(c_int32_t) :: host_language
        integer(c_int32_t) :: host_standard
        integer(c_int32_t) :: source_form
        integer(c_int32_t) :: flags
        integer(c_int32_t) :: reserved(3)
    end type

    type, bind(C), public :: RoupParserResult
        type(RoupCallResult) :: result
        type(RoupParserHandle) :: value
    end type

    type, bind(C), public :: RoupDirectiveResult
        type(RoupCallResult) :: result
        type(RoupDirectiveHandle) :: value
    end type

    type, bind(C), public :: RoupSizeResult
        type(RoupCallResult) :: result
        integer(c_size_t) :: value
    end type

    public :: bytes, require_ok
    public :: roup_parser_create, roup_parser_release, roup_parse
    public :: roup_directive_clause_count, roup_directive_release
    public :: roup_error_release

    interface
        function roup_parser_create(options) bind(C) result(output)
            import :: RoupParserOptions, RoupParserResult
            type(RoupParserOptions), value :: options
            type(RoupParserResult) :: output
        end function

        function roup_parser_release(parser) bind(C) result(output)
            import :: RoupCallResult, RoupParserHandle
            type(RoupParserHandle), value :: parser
            type(RoupCallResult) :: output
        end function

        function roup_parse(parser, input, length) bind(C) result(output)
            import :: c_char, c_size_t, RoupDirectiveResult, RoupParserHandle
            type(RoupParserHandle), value :: parser
            character(kind=c_char), intent(in) :: input(*)
            integer(c_size_t), value :: length
            type(RoupDirectiveResult) :: output
        end function

        function roup_directive_clause_count(directive) bind(C) result(output)
            import :: RoupDirectiveHandle, RoupSizeResult
            type(RoupDirectiveHandle), value :: directive
            type(RoupSizeResult) :: output
        end function

        function roup_directive_release(directive) bind(C) result(output)
            import :: RoupCallResult, RoupDirectiveHandle
            type(RoupDirectiveHandle), value :: directive
            type(RoupCallResult) :: output
        end function

        function roup_error_release(error) bind(C) result(output)
            import :: RoupCallResult, RoupErrorHandle
            type(RoupErrorHandle), value :: error
            type(RoupCallResult) :: output
        end function
    end interface

contains

    function bytes(text) result(output)
        character(len=*), intent(in) :: text
        character(kind=c_char), allocatable :: output(:)
        integer :: index

        allocate(output(len(text)))
        do index = 1, len(text)
            output(index) = text(index:index)
        end do
    end function

    subroutine require_ok(result)
        type(RoupCallResult), intent(in) :: result
        type(RoupCallResult) :: released

        if (result%status == ROUP_STATUS_OK) return
        if (result%error%generation == 0_c_int64_t) then
            error stop "ROUP failure did not provide an error handle"
        end if
        released = roup_error_release(result%error)
        if (released%status /= ROUP_STATUS_OK) then
            error stop "failed to release ROUP error handle"
        end if
        error stop "ROUP operation failed"
    end subroutine

end module
