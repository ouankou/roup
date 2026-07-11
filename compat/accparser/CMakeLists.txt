cmake_minimum_required(VERSION 3.20)
project(roup-accparser-compat VERSION 0.3.0 LANGUAGES CXX C)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

if(NOT EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/accparser/src/OpenACCIR.h")
    message(FATAL_ERROR
        "accparser submodule not initialized. Run: git submodule update --init --recursive")
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

set(ACCPARSER_IR_SRC ${CMAKE_CURRENT_SOURCE_DIR}/accparser/src/OpenACCIR.cpp)
set(ACCPARSER_TOSTRING_SRC ${CMAKE_CURRENT_SOURCE_DIR}/accparser/src/OpenACCIRToString.cpp)

add_library(accparser SHARED
    src/compat_impl.cpp
    ${ACCPARSER_IR_SRC}
    ${ACCPARSER_TOSTRING_SRC}
)
target_include_directories(accparser PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/accparser/src
    ${ROUP_CAPI_INCLUDE}
)
target_link_libraries(accparser PRIVATE ${ROUP_STATIC_LIB})

if(UNIX AND NOT APPLE)
    target_link_libraries(accparser PRIVATE pthread dl m)
elseif(APPLE)
    target_link_libraries(accparser PRIVATE pthread m)
endif()

set_target_properties(accparser PROPERTIES
    VERSION 1.0.0
    SOVERSION 1
    OUTPUT_NAME "accparser"
)

enable_testing()

# This is the compatibility criterion: register the pinned upstream suite
# unchanged and link every upstream runner against the replacement library.
add_subdirectory(accparser/tests)

# Upstream builtin scripts run from <build>/tests and invoke
# ./acc_tester.out, so keep their expected executable layout when embedded.
# Ordinary CMake build targets are globally visible after add_subdirectory;
# fail explicitly if the pinned upstream suite ever stops defining this one.
if(NOT TARGET acc_tester.out)
    message(FATAL_ERROR "Pinned accparser tests did not define acc_tester.out")
endif()
set_target_properties(acc_tester.out PROPERTIES
    RUNTIME_OUTPUT_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/tests")

add_executable(strict_contract_test tests/strict_contract_test.cpp)
target_link_libraries(strict_contract_test accparser)
target_include_directories(strict_contract_test PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/src)

add_test(NAME strict_contract COMMAND strict_contract_test)
add_test(
    NAME no_string_enum_audit
    COMMAND ${CMAKE_COMMAND}
        -DADAPTER_SOURCE=${CMAKE_CURRENT_SOURCE_DIR}/src/compat_impl.cpp
        -P ${CMAKE_CURRENT_SOURCE_DIR}/tests/no_string_enum_audit.cmake)
