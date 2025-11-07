cmake_minimum_required(VERSION 3.10)
project(roup-accparser-compat)

set(CMAKE_CXX_STANDARD 11)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# ============================================================================
# Build ROUP Library
# ============================================================================

set(ROUP_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../..")
set(ROUP_STATIC_LIB "${ROUP_ROOT}/target/release/libroup.a")

# Build ROUP if needed
if(NOT EXISTS "${ROUP_STATIC_LIB}")
    message(STATUS "Building ROUP library...")
    execute_process(
        COMMAND cargo build --release
        WORKING_DIRECTORY "${ROUP_ROOT}"
        RESULT_VARIABLE CARGO_RESULT
    )
    if(NOT CARGO_RESULT EQUAL 0)
        message(FATAL_ERROR "Failed to build ROUP library")
    endif()
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
    ${CMAKE_CURRENT_SOURCE_DIR}/accparser/src
    ${ROUP_ROOT}/src
)

# Link ROUP statically into libaccparser.so
target_link_libraries(accparser PRIVATE
    ${ROUP_STATIC_LIB}
    pthread
    dl
    m
)

set_target_properties(accparser PROPERTIES
    VERSION 1.0.0
    SOVERSION 1
    OUTPUT_NAME "accparser"
)

# ============================================================================
# Use accparser's builtin test infrastructure
# ============================================================================

enable_testing()

# Build acc_tester (ROUP test harness using parseOpenACC)
add_executable(acc_tester.out
    src/acc_tester_roup.cpp
)

target_include_directories(acc_tester.out PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/accparser/src
)

target_link_libraries(acc_tester.out accparser)

# Import all builtin tests from accparser
file(GLOB TEST_FILES "${CMAKE_CURRENT_SOURCE_DIR}/accparser/tests/builtin/*.txt")
file(GLOB REF_FILES "${CMAKE_CURRENT_SOURCE_DIR}/accparser/tests/builtin/reference/*.txt")

foreach(TEST_FILE ${TEST_FILES})
    get_filename_component(TEST_NAME ${TEST_FILE} NAME)
    string(REPLACE ".txt" "" TEST_BASE ${TEST_NAME})

    set(REF_FILE "${CMAKE_CURRENT_SOURCE_DIR}/accparser/tests/builtin/reference/ref_${TEST_NAME}")

    if(EXISTS ${REF_FILE})
        add_test(
            NAME ${TEST_NAME}
            COMMAND ${CMAKE_CURRENT_SOURCE_DIR}/accparser/tests/test.sh ${TEST_FILE} ${REF_FILE}
            WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}
        )
    endif()
endforeach()
