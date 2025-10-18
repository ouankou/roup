cmake_minimum_required(VERSION 3.10)
project(roup-accparser-compat VERSION 0.1.0 LANGUAGES CXX C)

set(CMAKE_CXX_STANDARD 11)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# ============================================================================
# Submodule Check
# ============================================================================

if(NOT EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/accparser/src/OpenACCIR.h")
    message(FATAL_ERROR
        "accparser submodule not initialized!\n"
        "Run: git submodule update --init --recursive\n"
        "Or see: compat/accparser/README.md")
endif()

# ============================================================================
# ROUP C Library
# ============================================================================

# Find or build ROUP library
set(ROUP_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../..")
set(ROUP_STATIC_LIB "${ROUP_ROOT}/target/release/libroup.a")

# Check if ROUP library exists
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
# Compatibility Library
# ============================================================================

# Reuse upstream implementation sources (IR only, no ANTLR!)
# We cherry-pick only the IR files, completely avoiding ANTLR grammar/parser
set(ACCPARSER_IR_SRC ${CMAKE_CURRENT_SOURCE_DIR}/accparser/src/OpenACCIR.cpp)
set(ACCPARSER_TOSTRING_SRC ${CMAKE_CURRENT_SOURCE_DIR}/accparser/src/OpenACCIRToString.cpp)

# Note: We do NOT include OpenACCASTConstructor.cpp because it depends on ANTLR-generated code.
# ROUP's parser replaces accparser's ANTLR-based parsing completely.

# Option 1: Static library (current)
add_library(roup-accparser-compat STATIC
    src/compat_impl.cpp
    # Include accparser's own IR implementation for toString, etc.
    ${ACCPARSER_IR_SRC}
    ${ACCPARSER_TOSTRING_SRC}
)

target_include_directories(roup-accparser-compat PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/accparser/src  # accparser headers from submodule
    ${ROUP_ROOT}/src                            # ROUP C API
)

# Link ROUP statically and required system libraries
# Link pthread and dl for portability across different systems
# Modern glibc (2.34+) includes these in libc, but explicit linking is harmless
if(UNIX AND NOT APPLE)
    set(EXTRA_LIBS pthread dl)
endif()

target_link_libraries(roup-accparser-compat PUBLIC
    ${ROUP_STATIC_LIB}
    ${EXTRA_LIBS}
)

# Option 2: Shared library with accparser's name (drop-in replacement!)
add_library(accparser SHARED
    src/compat_impl.cpp
    ${ACCPARSER_IR_SRC}
    ${ACCPARSER_TOSTRING_SRC}
)

target_include_directories(accparser PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/accparser/src
    ${ROUP_ROOT}/src
)

# Statically embed ROUP parser into libaccparser.so
# This creates a self-contained library with no external ROUP or ANTLR dependency
target_link_libraries(accparser PRIVATE
    ${ROUP_STATIC_LIB}
    ${EXTRA_LIBS}  # Use same glibc-aware linking as static library
)

# Set version matching accparser for compatibility
set_target_properties(accparser PROPERTIES
    VERSION 1.0.0
    SOVERSION 1
    OUTPUT_NAME "accparser"  # Creates libaccparser.so
)

# ============================================================================
# Examples and Tests
# ============================================================================

# Example using our compat library (old way)
add_executable(compat_example
    examples/basic_test.cpp
)

target_link_libraries(compat_example
    roup-accparser-compat
)

# Example using drop-in replacement (new way - same as original accparser!)
add_executable(accparser_example
    examples/basic_test.cpp
)

target_link_libraries(accparser_example
    accparser  # Just link against libaccparser.so - that's it!
)

# Comprehensive test suite
add_executable(comprehensive_test
    tests/comprehensive_test.cpp
)

target_link_libraries(comprehensive_test
    accparser
)

# Enable testing
enable_testing()
add_test(NAME compat_basic COMMAND compat_example)
add_test(NAME accparser_drop_in COMMAND accparser_example)
add_test(NAME comprehensive COMMAND comprehensive_test)

# ============================================================================
# Installation
# ============================================================================

# Install both versions for flexibility
install(TARGETS roup-accparser-compat accparser
    ARCHIVE DESTINATION lib
    LIBRARY DESTINATION lib
)

install(FILES
    ${CMAKE_CURRENT_SOURCE_DIR}/accparser/src/OpenACCIR.h
    ${CMAKE_CURRENT_SOURCE_DIR}/accparser/src/OpenACCKinds.h
    DESTINATION include
)

# Create pkg-config file for easy discovery
configure_file(
    ${CMAKE_CURRENT_SOURCE_DIR}/accparser.pc.in
    ${CMAKE_CURRENT_BINARY_DIR}/accparser.pc
    @ONLY
)
install(FILES ${CMAKE_CURRENT_BINARY_DIR}/accparser.pc
    DESTINATION lib/pkgconfig
)
