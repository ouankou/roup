program tutorial_basic
    use, intrinsic :: iso_c_binding
    use roup_capi
    implicit none

    type(RoupParserOptions) :: options
    type(RoupParserResult) :: parser
    type(RoupDirectiveResult) :: directive
    type(RoupSizeResult) :: clauses
    type(RoupCallResult) :: released
    character(kind=c_char), allocatable :: source(:)

    options%abi_version = ROUP_ABI_VERSION
    options%struct_size = int(c_sizeof(options), c_int32_t)
    options%dialect = ROUP_DIALECT_OPENACC
    options%version_policy = ROUP_VERSION_ANY
    options%version = 0
    options%host_language = ROUP_HOST_FORTRAN
    options%host_standard = ROUP_FORTRAN_2023
    options%source_form = ROUP_SOURCE_FORTRAN_FREE
    options%flags = 0
    options%reserved = 0

    parser = roup_parser_create(options)
    call require_ok(parser%result)

    source = bytes("!$acc parallel async(queue) private(values)")
    directive = roup_parse(parser%value, source, size(source, kind=c_size_t))
    call require_ok(directive%result)

    clauses = roup_directive_clause_count(directive%value)
    call require_ok(clauses%result)
    if (clauses%value /= 2_c_size_t) error stop "unexpected OpenACC clause count"

    released = roup_directive_release(directive%value)
    call require_ok(released)
    released = roup_parser_release(parser%value)
    call require_ok(released)
end program
