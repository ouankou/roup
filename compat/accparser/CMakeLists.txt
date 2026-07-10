cmake_minimum_required(VERSION 3.20)
project(roup-accparser-compat VERSION 0.3.0 LANGUAGES CXX C)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# ============================================================================
# Submodule and ROUP C ABI checks
# ============================================================================

if(NOT EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/accparser/src/OpenACCIR.h")
    message(FATAL_ERROR
        "accparser submodule not initialized. Run: git submodule update --init --recursive")
endif()

# Configuration never invokes Cargo or mutates the Rust build tree. The caller
# explicitly builds the optional C ABI before configuring this adapter.
set(ROUP_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../..")
set(ROUP_CAPI_INCLUDE "${ROUP_ROOT}/crates/roup-capi/include")
set(ROUP_STATIC_LIB "${ROUP_ROOT}/target/release/libroup_capi.a"
    CACHE FILEPATH "Path to the prebuilt ROUP C ABI static library")

if(NOT EXISTS "${ROUP_STATIC_LIB}")
    message(FATAL_ERROR
        "Prebuilt roup-capi archive not found: ${ROUP_STATIC_LIB}\n"
        "Build it explicitly with: cargo build --locked --release -p roup-capi")
endif()
if(NOT EXISTS "${ROUP_CAPI_INCLUDE}/roup.h")
    message(FATAL_ERROR "ROUP C ABI header not found: ${ROUP_CAPI_INCLUDE}/roup.h")
endif()

# ============================================================================
# Build Compat Library (libaccparser.so)
# ============================================================================

# Include accparser IR sources (NO ANTLR!)
set(ACCPARSER_IR_SRC ${CMAKE_CURRENT_SOURCE_DIR}/accparser/src/OpenACCIR.cpp)
set(ACCPARSER_TOSTRING_SRC ${CMAKE_CURRENT_SOURCE_DIR}/accparser/src/OpenACCIRToString.cpp)

# Build shared library named "accparser" (drop-in replacement)
add_library(accparser SHARED
    src/compat_impl.cpp
    ${ACCPARSER_IR_SRC}
    ${ACCPARSER_TOSTRING_SRC}
)

target_include_directories(accparser PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/accparser/src  # Use headers directly from submodule
    ${ROUP_CAPI_INCLUDE}
)

# Link ROUP statically into libaccparser.so
target_link_libraries(accparser PRIVATE
    ${ROUP_STATIC_LIB}
)

# Platform-specific libraries
if(UNIX AND NOT APPLE)
    target_link_libraries(accparser PRIVATE pthread dl m)
elseif(APPLE)
    target_link_libraries(accparser PRIVATE pthread m)
elseif(WIN32)
    # Windows: no special libraries needed
endif()

set_target_properties(accparser PROPERTIES
    VERSION 1.0.0
    SOVERSION 1
    OUTPUT_NAME "accparser"
)

# ============================================================================
# Test infrastructure
# ============================================================================

enable_testing()

# Build only the runners needed by the explicit strict gate. Importing the
# upstream tests directory would register its permissive builtin and whole-VV
# corpora as if they were ROUP's acceptance contract.
add_executable(acc_tester.out
    src/acc_tester_roup.cpp)
target_link_libraries(acc_tester.out accparser)
set_target_properties(acc_tester.out PROPERTIES
    RUNTIME_OUTPUT_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/tests")

add_executable(strict_contract_test
    tests/strict_contract_test.cpp)
target_link_libraries(strict_contract_test accparser)
target_include_directories(strict_contract_test PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/src)

add_executable(lang_flag_test.out
    tests/lang_flag_strict_test.cpp)
target_link_libraries(lang_flag_test.out accparser)

add_executable(compat_caller.out
    tests/compat_caller_strict_test.cpp)
target_link_libraries(compat_caller.out accparser)

add_test(NAME strict_contract COMMAND strict_contract_test)
add_test(NAME lang_flag_test COMMAND lang_flag_test.out)
add_test(NAME compat_caller COMMAND compat_caller.out)
add_test(
    NAME no_string_enum_audit
    COMMAND ${CMAKE_COMMAND}
        -DADAPTER_SOURCE=${CMAKE_CURRENT_SOURCE_DIR}/src/compat_impl.cpp
        -P ${CMAKE_CURRENT_SOURCE_DIR}/tests/no_string_enum_audit.cmake)

# These pinned fixtures were audited line by line for strict host-language
# syntax and exact canonical output. No glob or configure-time discovery may
# expand this gate implicitly.
set(ACCPARSER_AUDITED_FIXTURES
    atomic
    atomic_fortran)

set(ACCPARSER_INPUT_HASH_atomic
    91ec9aea81612b7b8baf0f010b93f08a3814e9d45c2ce2dc08ff4f2507a7d74c)
set(ACCPARSER_REFERENCE_HASH_atomic
    82e1f743eec813b3d420bac41d0925143ba9cfefbc8844330497702af2d8cea4)
set(ACCPARSER_INPUT_HASH_atomic_fortran
    cc5c7b3b580b45bfccfa096f51239d9a964f11849a3b27f52ab4262649dac660)
set(ACCPARSER_REFERENCE_HASH_atomic_fortran
    951040ac50c2dfcaee9d4f355a33991d9a42edadcf2eb55c2a2db701ec463de0)

set(ACCPARSER_LOCAL_AUDITED_FIXTURES
    cache
    cache_fortran)
set(ACCPARSER_LOCAL_INPUT_HASH_cache
    9ff08ca1c79f2909b97f462cbdad02bd986ab62c94cb315a09756c69069b695c)
set(ACCPARSER_LOCAL_REFERENCE_HASH_cache
    7dd88ebe8130fb1b4b94f50130045210808c614f4190a61f5e27d93f8a0e452f)
set(ACCPARSER_LOCAL_INPUT_HASH_cache_fortran
    7ffc1c2238669bd40f6186b3f2929ab7ee6abf10e2388731d66078d2b43252eb)
set(ACCPARSER_LOCAL_REFERENCE_HASH_cache_fortran
    af999ca637171d150b6c25eb89433413c6951362cdd5dd311a5a59b57ad237bc)
set(ACCPARSER_GATE_TESTS
    strict_contract
    lang_flag_test
    compat_caller
    no_string_enum_audit)

foreach(FIXTURE_NAME IN LISTS ACCPARSER_AUDITED_FIXTURES)
    set(INPUT_FILE
        "${CMAKE_CURRENT_SOURCE_DIR}/accparser/tests/builtin/${FIXTURE_NAME}.txt")
    set(REFERENCE_FILE
        "${CMAKE_CURRENT_SOURCE_DIR}/accparser/tests/builtin/reference/ref_${FIXTURE_NAME}.txt")
    if(NOT EXISTS "${INPUT_FILE}" OR NOT EXISTS "${REFERENCE_FILE}")
        message(FATAL_ERROR
            "Audited accparser fixture pair is missing: ${FIXTURE_NAME}")
    endif()
    set(INPUT_HASH_VARIABLE "ACCPARSER_INPUT_HASH_${FIXTURE_NAME}")
    set(REFERENCE_HASH_VARIABLE "ACCPARSER_REFERENCE_HASH_${FIXTURE_NAME}")
    if(NOT DEFINED ${INPUT_HASH_VARIABLE} OR
       NOT DEFINED ${REFERENCE_HASH_VARIABLE})
        message(FATAL_ERROR
            "Audited accparser fixture pair has no pinned hashes: ${FIXTURE_NAME}")
    endif()
    file(SHA256 "${INPUT_FILE}" ACTUAL_INPUT_HASH)
    file(SHA256 "${REFERENCE_FILE}" ACTUAL_REFERENCE_HASH)
    if(NOT ACTUAL_INPUT_HASH STREQUAL "${${INPUT_HASH_VARIABLE}}" OR
       NOT ACTUAL_REFERENCE_HASH STREQUAL "${${REFERENCE_HASH_VARIABLE}}")
        message(FATAL_ERROR
            "Audited accparser fixture pair changed and requires re-audit: ${FIXTURE_NAME}")
    endif()

    set(TEST_NAME "fixture_${FIXTURE_NAME}")
    add_test(
        NAME "${TEST_NAME}"
        COMMAND "${CMAKE_CURRENT_SOURCE_DIR}/accparser/tests/test.sh"
                "${INPUT_FILE}" "${REFERENCE_FILE}"
        WORKING_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/tests")
    list(APPEND ACCPARSER_GATE_TESTS "${TEST_NAME}")
endforeach()

# The upstream cache fixtures use scalar names, which the OpenACC cache
# directive forbids. Keep equivalent strict C and Fortran fixture gates under
# repository ownership so the pinned submodule stays unmodified.
foreach(FIXTURE_NAME IN LISTS ACCPARSER_LOCAL_AUDITED_FIXTURES)
    set(INPUT_FILE
        "${CMAKE_CURRENT_SOURCE_DIR}/tests/fixtures/${FIXTURE_NAME}.txt")
    set(REFERENCE_FILE
        "${CMAKE_CURRENT_SOURCE_DIR}/tests/fixtures/ref_${FIXTURE_NAME}.txt")
    if(NOT EXISTS "${INPUT_FILE}" OR NOT EXISTS "${REFERENCE_FILE}")
        message(FATAL_ERROR
            "Local audited accparser fixture pair is missing: ${FIXTURE_NAME}")
    endif()
    set(INPUT_HASH_VARIABLE "ACCPARSER_LOCAL_INPUT_HASH_${FIXTURE_NAME}")
    set(REFERENCE_HASH_VARIABLE "ACCPARSER_LOCAL_REFERENCE_HASH_${FIXTURE_NAME}")
    file(SHA256 "${INPUT_FILE}" ACTUAL_INPUT_HASH)
    file(SHA256 "${REFERENCE_FILE}" ACTUAL_REFERENCE_HASH)
    if(NOT ACTUAL_INPUT_HASH STREQUAL "${${INPUT_HASH_VARIABLE}}" OR
       NOT ACTUAL_REFERENCE_HASH STREQUAL "${${REFERENCE_HASH_VARIABLE}}")
        message(FATAL_ERROR
            "Local audited accparser fixture pair changed and requires re-audit: ${FIXTURE_NAME}")
    endif()

    set(TEST_NAME "fixture_${FIXTURE_NAME}")
    add_test(
        NAME "${TEST_NAME}"
        COMMAND "${CMAKE_CURRENT_SOURCE_DIR}/accparser/tests/test.sh"
                "${INPUT_FILE}" "${REFERENCE_FILE}"
        WORKING_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/tests")
    list(APPEND ACCPARSER_GATE_TESTS "${TEST_NAME}")
endforeach()

# Timeouts fail the gate; they are not skips or ignored results.
set_tests_properties(${ACCPARSER_GATE_TESTS}
    PROPERTIES LABELS "strict-compat-gate" TIMEOUT 20)
