cmake_minimum_required(VERSION 3.20)
project(roup-ompparser-compat VERSION 0.3.0 LANGUAGES CXX C)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# ============================================================================
# Submodule Check
# ============================================================================

if(NOT EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/ompparser/src/OpenMPIR.h")
    message(FATAL_ERROR 
        "ompparser submodule not initialized!\n"
        "Run: git submodule update --init --recursive\n"
        "Or see: compat/ompparser/README.md")
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

# Reuse upstream implementation sources (excluding parseOpenMP symbol clash)
set(OMPPARSER_IR_SRC ${CMAKE_CURRENT_SOURCE_DIR}/ompparser/src/OpenMPIR.cpp)
set(OMPPARSER_TOSTRING_SRC ${CMAKE_CURRENT_SOURCE_DIR}/ompparser/src/OpenMPIRToString.cpp)
set(OMPPARSER_TODOT_SRC ${CMAKE_CURRENT_SOURCE_DIR}/ompparser/src/OpenMPIRToDOT.cpp)

# Provide our own small compatibility source that defines globals expected by
# the ompparser headers (e.g., normalize_clauses_global) so we don't need to
# invoke the original project's bison/flex pipeline.
set(OMP_COMPATIAL_SRC ${CMAKE_CURRENT_SOURCE_DIR}/src/compat_globals.cpp)

# Rename upstream parseOpenMP symbol to avoid multiple definition when linking
set_source_files_properties(${OMPPARSER_IR_SRC}
    PROPERTIES COMPILE_DEFINITIONS "parseOpenMP=ompparser_legacy_parseOpenMP")

# Option 1: Static library (current)
add_library(roup-ompparser-compat STATIC
    src/compat_impl.cpp
    ${OMP_COMPATIAL_SRC}
    # Include ompparser's own implementation for toString, etc.
    ${OMPPARSER_IR_SRC}
    ${OMPPARSER_TOSTRING_SRC}
    ${OMPPARSER_TODOT_SRC}
)

target_include_directories(roup-ompparser-compat PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/ompparser/src  # ompparser headers from submodule
    ${ROUP_ROOT}/src                            # ROUP C API
)

# Link ROUP statically and required system libraries
# Link pthread and dl for portability across different systems
# Modern glibc (2.34+) includes these in libc, but explicit linking is harmless
if(UNIX AND NOT APPLE)
    set(EXTRA_LIBS pthread dl)
endif()

target_link_libraries(roup-ompparser-compat PUBLIC
    ${ROUP_STATIC_LIB}
    ${EXTRA_LIBS}
)

# Option 2: Shared library with ompparser's name (drop-in replacement!)
add_library(ompparser SHARED
    src/compat_impl.cpp
    ${OMPPARSER_IR_SRC}
    ${OMP_COMPATIAL_SRC}
    ${OMPPARSER_TOSTRING_SRC}
    ${OMPPARSER_TODOT_SRC}
)

target_include_directories(ompparser PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/ompparser/src
    ${ROUP_ROOT}/src
)

# Statically embed ROUP parser into libompparser.so
# This creates a self-contained library with no external ROUP dependency
target_link_libraries(ompparser PRIVATE
    ${ROUP_STATIC_LIB}
    ${EXTRA_LIBS}  # Use same glibc-aware linking as static library
)

# Set version matching ompparser for compatibility
set_target_properties(ompparser PROPERTIES
    VERSION 1.0.0
    SOVERSION 1
    OUTPUT_NAME "ompparser"  # Creates libompparser.so
)

# ============================================================================
# Examples and Tests
# ============================================================================

# Example using our compat library (old way)
add_executable(compat_example
    examples/basic_test.cpp
)

target_link_libraries(compat_example
    roup-ompparser-compat
)

# Example using drop-in replacement (new way - same as original ompparser!)
add_executable(ompparser_example
    examples/basic_test.cpp
)

target_link_libraries(ompparser_example
    ompparser  # Just link against libompparser.so - that's it!
)

# Comprehensive test suite
add_executable(comprehensive_test
    tests/comprehensive_test.cpp
)

target_link_libraries(comprehensive_test
    ompparser
)

# Enable testing
enable_testing()
add_test(NAME compat_basic COMMAND compat_example)
add_test(NAME ompparser_drop_in COMMAND ompparser_example)
add_test(NAME comprehensive COMMAND comprehensive_test)

# Include ompparser submodule tests (1524 tests from openmp_examples and openmp_vv)
add_subdirectory(ompparser/tests)

# ============================================================================
# Installation
# ============================================================================

# Install both versions for flexibility
install(TARGETS roup-ompparser-compat ompparser
    ARCHIVE DESTINATION lib
    LIBRARY DESTINATION lib
)

install(FILES 
    ${CMAKE_CURRENT_SOURCE_DIR}/ompparser/src/OpenMPIR.h
    ${CMAKE_CURRENT_SOURCE_DIR}/ompparser/src/OpenMPKinds.h
    DESTINATION include
)

# Create pkg-config file for easy discovery
configure_file(
    ${CMAKE_CURRENT_SOURCE_DIR}/ompparser.pc.in
    ${CMAKE_CURRENT_BINARY_DIR}/ompparser.pc
    @ONLY
)
install(FILES ${CMAKE_CURRENT_BINARY_DIR}/ompparser.pc
    DESTINATION lib/pkgconfig
)
