cmake_minimum_required(VERSION 3.20)
project(roup-ompparser-compat VERSION 0.3.0 LANGUAGES CXX C)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

if(NOT EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/ompparser/src/OpenMPIR.h")
    message(FATAL_ERROR
        "ompparser submodule not initialized. Run: git submodule update --init --recursive")
endif()

# The pure Rust parser remains independently buildable. This optional adapter
# consumes only the stable C ABI archive explicitly built by its caller.
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

# Closed semantics must cross the adapter as numeric tags or semantic nodes;
# only typed lexical leaves may use string fields.
file(READ "${CMAKE_CURRENT_SOURCE_DIR}/src/compat_impl.cpp"
     OMPPARSER_ADAPTER_SOURCE)
foreach(FORBIDDEN_ADAPTER_FRAGMENT IN ITEMS
        "enum_key"
        "directive_name("
        "clause_name(")
    string(FIND "${OMPPARSER_ADAPTER_SOURCE}"
           "${FORBIDDEN_ADAPTER_FRAGMENT}" FORBIDDEN_FRAGMENT_OFFSET)
    if(NOT FORBIDDEN_FRAGMENT_OFFSET EQUAL -1)
        message(FATAL_ERROR
            "Strict adapter contains forbidden fallback/reclassification: "
            "${FORBIDDEN_ADAPTER_FRAGMENT}")
    endif()
endforeach()
string(REGEX MATCH
    "(required|optional)_string\\([ \t\r\n]*ROUP_FIELD_(KIND|MODIFIER|MODIFIERS|BEHAVIOR|CATEGORY|DIRECTIVE|DEPEND_TYPE|MEMORY_ORDER|ALLOCATOR)[ \t\r\n]*\\)"
    CLOSED_SEMANTIC_STRING_ACCESS "${OMPPARSER_ADAPTER_SOURCE}")
if(CLOSED_SEMANTIC_STRING_ACCESS)
    message(FATAL_ERROR
        "Strict adapter reads a closed semantic field through a string getter: "
        "${CLOSED_SEMANTIC_STRING_ACCESS}")
endif()

set(OMPPARSER_IR_SRC ${CMAKE_CURRENT_SOURCE_DIR}/ompparser/src/OpenMPIR.cpp)
set(OMPPARSER_TOSTRING_SRC ${CMAKE_CURRENT_SOURCE_DIR}/ompparser/src/OpenMPIRToString.cpp)
set(OMPPARSER_TODOT_SRC ${CMAKE_CURRENT_SOURCE_DIR}/ompparser/src/OpenMPIRToDOT.cpp)
set(OMP_COMPATIAL_SRC ${CMAKE_CURRENT_SOURCE_DIR}/src/compat_globals.cpp)

# The adapter supplies parseOpenMP; upstream IR and rendering code remain the
# actual public implementation surface used by downstream clients and tests.
set_source_files_properties(${OMPPARSER_IR_SRC}
    PROPERTIES COMPILE_DEFINITIONS "parseOpenMP=ompparser_legacy_parseOpenMP")

add_library(roup-ompparser-compat STATIC
    src/compat_impl.cpp
    ${OMP_COMPATIAL_SRC}
    ${OMPPARSER_IR_SRC}
    ${OMPPARSER_TOSTRING_SRC}
    ${OMPPARSER_TODOT_SRC}
)
target_include_directories(roup-ompparser-compat PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/ompparser/src
    ${ROUP_CAPI_INCLUDE}
)

if(UNIX AND NOT APPLE)
    set(EXTRA_LIBS pthread dl)
endif()
target_link_libraries(roup-ompparser-compat PUBLIC
    ${ROUP_STATIC_LIB}
    ${EXTRA_LIBS}
)

add_library(ompparser SHARED
    src/compat_impl.cpp
    ${OMPPARSER_IR_SRC}
    ${OMP_COMPATIAL_SRC}
    ${OMPPARSER_TOSTRING_SRC}
    ${OMPPARSER_TODOT_SRC}
)
target_include_directories(ompparser PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/ompparser/src
    ${ROUP_CAPI_INCLUDE}
)
target_link_libraries(ompparser PRIVATE
    ${ROUP_STATIC_LIB}
    ${EXTRA_LIBS}
)
set_target_properties(ompparser PROPERTIES
    VERSION 1.0.0
    SOVERSION 1
    OUTPUT_NAME "ompparser"
)

add_executable(compat_example examples/basic_test.cpp)
target_link_libraries(compat_example roup-ompparser-compat)

add_executable(ompparser_example examples/basic_test.cpp)
target_link_libraries(ompparser_example ompparser)

add_executable(strict_contract_test tests/strict_contract_test.cpp)
target_link_libraries(strict_contract_test ompparser)

enable_testing()
add_test(NAME compat_basic COMMAND compat_example)
add_test(NAME ompparser_drop_in COMMAND ompparser_example)
add_test(NAME strict_contract COMMAND strict_contract_test)

# This is the compatibility criterion: register the pinned upstream suite
# unchanged and link every upstream runner against the replacement library.
add_subdirectory(ompparser/tests)

install(TARGETS roup-ompparser-compat ompparser
    ARCHIVE DESTINATION lib
    LIBRARY DESTINATION lib
)
install(FILES
    ${CMAKE_CURRENT_SOURCE_DIR}/ompparser/src/OpenMPIR.h
    ${CMAKE_CURRENT_SOURCE_DIR}/ompparser/src/OpenMPKinds.h
    DESTINATION include
)

configure_file(
    ${CMAKE_CURRENT_SOURCE_DIR}/ompparser.pc.in
    ${CMAKE_CURRENT_BINARY_DIR}/ompparser.pc
    @ONLY
)
install(FILES ${CMAKE_CURRENT_BINARY_DIR}/ompparser.pc
    DESTINATION lib/pkgconfig
)
