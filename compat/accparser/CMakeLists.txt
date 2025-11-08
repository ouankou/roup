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
    ${CMAKE_CURRENT_SOURCE_DIR}/accparser/src  # Use headers directly from submodule
    ${ROUP_ROOT}/src  # roup_constants.h
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

# Create tests/ directory with symlink to acc_tester.out (builtin tests expect this)
file(MAKE_DIRECTORY ${CMAKE_BINARY_DIR}/tests)
add_custom_command(
    TARGET accparser POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E create_symlink
        ${CMAKE_BINARY_DIR}/accparser/tests/acc_tester.out
        ${CMAKE_BINARY_DIR}/tests/acc_tester.out
)

add_subdirectory(accparser/tests)
