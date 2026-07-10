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

# The caller builds the optional ABI explicitly. Configuration never invokes
# Cargo or mutates the Rust build tree.
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

# Fail configuration if the adapter regresses to lexical reclassification or
# the upstream-only declare-induction passthrough. Open lexical leaves still
# cross as strings, but closed semantics must use U32 tags or semantic nodes.
file(READ "${CMAKE_CURRENT_SOURCE_DIR}/src/compat_impl.cpp"
     OMPPARSER_ADAPTER_SOURCE)
foreach(FORBIDDEN_ADAPTER_FRAGMENT IN ITEMS
        "addPassthroughItem"
        "enum_key"
        "directive_name("
        "clause_name("
        "== \""
        "!= \"")
    string(FIND "${OMPPARSER_ADAPTER_SOURCE}"
           "${FORBIDDEN_ADAPTER_FRAGMENT}" FORBIDDEN_FRAGMENT_OFFSET)
    if(NOT FORBIDDEN_FRAGMENT_OFFSET EQUAL -1)
        message(FATAL_ERROR
            "Strict adapter contains forbidden fallback/reclassification: "
            "${FORBIDDEN_ADAPTER_FRAGMENT}")
    endif()
endforeach()
string(REGEX MATCH
    "(required|optional)_string\\([ \\t\\r\\n]*ROUP_FIELD_(KIND|MODIFIER|MODIFIERS|BEHAVIOR|CATEGORY|DIRECTIVE|DEPEND_TYPE|MEMORY_ORDER|ALLOCATOR)[ \\t\\r\\n]*\\)"
    CLOSED_SEMANTIC_STRING_ACCESS "${OMPPARSER_ADAPTER_SOURCE}")
if(CLOSED_SEMANTIC_STRING_ACCESS)
    message(FATAL_ERROR
        "Strict adapter reads a closed semantic field through a string getter: "
        "${CLOSED_SEMANTIC_STRING_ACCESS}")
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
    ${ROUP_CAPI_INCLUDE}
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
    ${ROUP_CAPI_INCLUDE}
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

add_executable(strict_contract_test
    tests/strict_contract_test.cpp
)

target_link_libraries(strict_contract_test
    ompparser
)

# Enable testing
enable_testing()
add_test(NAME compat_basic COMMAND compat_example)
add_test(NAME ompparser_drop_in COMMAND ompparser_example)
add_test(NAME comprehensive COMMAND comprehensive_test)
add_test(NAME strict_contract COMMAND strict_contract_test)

# The pinned runner assumes parse failures return null. ROUP's public contract
# uses hard errors, so use a local runner which asserts exceptions for
# `EXPECT: invalid` while reading the unchanged upstream fixture corpus.
add_executable(strict_fixture_tester
    tests/strict_fixture_tester.cpp)
target_link_libraries(strict_fixture_tester ompparser)

set_target_properties(strict_fixture_tester PROPERTIES BUILD_RPATH "$ORIGIN")

set(OMPPARSER_TEST_LD_LIBRARY_PATH
    "LD_LIBRARY_PATH=$<TARGET_FILE_DIR:ompparser>:$ENV{LD_LIBRARY_PATH}")

# Apply only individually audited fixture corrections. Every replacement must
# match the pinned input exactly; a submodule update that changes the evidence
# is a configuration error, not a reason to skip more coverage.
function(audited_fixture_replace CONTENT_VARIABLE OLD_TEXT NEW_TEXT DESCRIPTION)
    string(FIND "${${CONTENT_VARIABLE}}" "${OLD_TEXT}" MATCH_OFFSET)
    if(MATCH_OFFSET EQUAL -1)
        message(FATAL_ERROR
            "Pinned fixture no longer contains audited case: ${DESCRIPTION}")
    endif()
    string(REPLACE "${OLD_TEXT}" "${NEW_TEXT}" UPDATED_CONTENT
           "${${CONTENT_VARIABLE}}")
    set(${CONTENT_VARIABLE} "${UPDATED_CONTENT}" PARENT_SCOPE)
endfunction()

file(READ "${CMAKE_CURRENT_SOURCE_DIR}/ompparser/tests/builtin/parallel.txt"
     PARALLEL_STRICT_CONTENT)

# Malformed host expressions that the legacy parser accepted are replaced by
# valid locator coverage; strict_contract_test asserts the hard-negative forms.
audited_fixture_replace(PARALLEL_STRICT_CONTENT
    [=[#pragma omp parallel private (c/)
PASS: #pragma omp parallel private(c/)
]=]
    ""
    "malformed private(c/) accepted as valid")
audited_fixture_replace(PARALLEL_STRICT_CONTENT
    [=[#pragma omp parallel reduction (inscan, + : a, foo(x)) reduction (abc : x, y, z) reduction (task, user_defined_value : x, y, z) reduction (inscan, max : a, foo(x))
PASS: #pragma omp parallel reduction(inscan, + : a, foo(x)) reduction(abc : x, y, z) reduction(task, user_defined_value : x, y, z) reduction(inscan, max : a, foo(x))
]=]
    [=[#pragma omp parallel reduction (inscan, + : a, foo) reduction (abc : x, y, z) reduction (task, user_defined_value : x, y, z) reduction (inscan, max : a, foo)
PASS: #pragma omp parallel reduction(inscan, + : a, foo) reduction(abc : x, y, z) reduction(task, user_defined_value : x, y, z) reduction(inscan, max : a, foo)
]=]
    "function calls used as reduction locators")
audited_fixture_replace(PARALLEL_STRICT_CONTENT
    [=[#pragma omp parallel allocate (omp_high_bw_mem_alloc : m, n[1:5]) allocate (no, allo, cator) allocate (user_defined_test : m, n[1:5])
PASS: #pragma omp parallel allocate (omp_high_bw_mem_alloc: m, n[1 : 5]) allocate (no, allo, cator) allocate (user_defined_test: m, n[1 : 5])
]=]
    [=[EXPECT: invalid
#pragma omp parallel allocate (omp_high_bw_mem_alloc : m, n[1:5]) allocate (no, allo, cator) allocate (user_defined_test : m, n[1:5])
]=]
    "allocate items missing a privatizing data-sharing clause")

# Canonical typed rendering intentionally normalizes expression whitespace and
# the specification-defined proc_bind(master) alias.
audited_fixture_replace(PARALLEL_STRICT_CONTENT
    "PASS: #pragma omp parallel num_threads(3*5+4/(7+10))"
    "PASS: #pragma omp parallel num_threads(3 * 5 + 4 / (7 + 10))"
    "canonical num_threads expression spacing")
audited_fixture_replace(PARALLEL_STRICT_CONTENT
    "PASS: #pragma omp parallel proc_bind (master)"
    "PASS: #pragma omp parallel proc_bind (primary)"
    "proc_bind master alias canonicalization")

# An unmodified if clause applies to every eligible constituent. It therefore
# overlaps `if(parallel: ...)` on a standalone parallel directive and must be
# a hard error under the historical uniqueness restriction.
audited_fixture_replace(PARALLEL_STRICT_CONTENT
    [=[#pragma omp parallel if (a) if (parallel : b) default (none)
PASS: #pragma omp parallel if (a) if (parallel: b) default (none)
]=]
    [=[EXPECT: invalid
#pragma omp parallel if (a) if (parallel : b) default (none)
]=]
    "overlapping unmodified and parallel-targeted if clauses")

# The remaining unpaired scratch inputs are genuine hard-negative cases.
foreach(MALFORMED_INPUT IN ITEMS
    "#pragma omp parallel  private (a[foo(x, goo(x, y)):100], b[1:30], c) firstprivate (foo(x), y), shared (a, b, c[1:10]) num_threads (4)"
    "#pragma omp parallel reduction (tasktest : x11, y, z) allocate (user_defined_test : m, n[1:5]) allocate (omp_high_bw_mem_alloc : m, n[1:5]) reduction (inscan, max : a, foo(x))"
    "#pragma omp parallel if (a) if (parallel : b) default (private) if(b)"
    "#pragma omp parallel if (a) if (parallel : b) default (copyprivate)")
    audited_fixture_replace(PARALLEL_STRICT_CONTENT
        "\n${MALFORMED_INPUT}\n"
        "\nEXPECT: invalid\n${MALFORMED_INPUT}\n"
        "unpaired malformed parallel input")
endforeach()

set(PARALLEL_STRICT_FILE "${CMAKE_CURRENT_BINARY_DIR}/parallel_strict.txt")
file(WRITE "${PARALLEL_STRICT_FILE}" "${PARALLEL_STRICT_CONTENT}")

file(READ "${CMAKE_CURRENT_SOURCE_DIR}/ompparser/tests/builtin/atomic.txt"
     ATOMIC_STRICT_CONTENT)

# The legacy fixture accepts a trailing comma. Preserve the source as an
# explicit hard-negative case instead of weakening the strict parser or
# deleting the evidence.
audited_fixture_replace(ATOMIC_STRICT_CONTENT
    [=[#pragma omp atomic seq_cst, hint(abc), read,
PASS: #pragma omp atomic seq_cst hint(abc) read
]=]
    [=[EXPECT: invalid
#pragma omp atomic seq_cst, hint(abc), read,
]=]
    "trailing comma after atomic clause sequence")

set(ATOMIC_STRICT_FILE "${CMAKE_CURRENT_BINARY_DIR}/atomic_strict.txt")
file(WRITE "${ATOMIC_STRICT_FILE}" "${ATOMIC_STRICT_CONTENT}")

file(READ "${CMAKE_CURRENT_SOURCE_DIR}/ompparser/tests/builtin/flush.txt"
     FLUSH_STRICT_CONTENT)
audited_fixture_replace(FLUSH_STRICT_CONTENT
    [=[#pragma omp flush acq_rel(x,y,z)
PASS: #pragma omp flush (x,y,z) acq_rel
]=]
    [=[#pragma omp flush acq_rel(x,y,z)
PASS: #pragma omp flush acq_rel(x , y , z)
]=]
    "C comma expression misclassified as a historical flush list")
set(FLUSH_STRICT_FILE "${CMAKE_CURRENT_BINARY_DIR}/flush_strict.txt")
file(WRITE "${FLUSH_STRICT_FILE}" "${FLUSH_STRICT_CONTENT}")

file(READ "${CMAKE_CURRENT_SOURCE_DIR}/ompparser/tests/builtin/flush_fortran.txt"
     FLUSH_FORTRAN_STRICT_CONTENT)
foreach(INVALID_ORDER IN ITEMS acq_rel release acquire)
    audited_fixture_replace(FLUSH_FORTRAN_STRICT_CONTENT
        "!$omp flush ${INVALID_ORDER}(x,y,z)\nPASS: !$omp flush (x,y,z) ${INVALID_ORDER}\n"
        "EXPECT: invalid\n!$omp flush ${INVALID_ORDER}(x,y,z)\n"
        "Fortran memory-order plus flush-list restriction for ${INVALID_ORDER}")
endforeach()
set(FLUSH_FORTRAN_STRICT_FILE
    "${CMAKE_CURRENT_BINARY_DIR}/flush_fortran_strict.txt")
file(WRITE "${FLUSH_FORTRAN_STRICT_FILE}" "${FLUSH_FORTRAN_STRICT_CONTENT}")

# The strict compatibility gate deliberately registers a fixed manifest. These
# pinned fixtures were individually run against the strict adapter and contain
# either exact canonical output or explicit hard-error expectations. The
# permissive aggregate builtin/VV/example corpora are audit inputs, not an
# executable contract, and are never registered implicitly.
set(OMPPARSER_AUDITED_FIXTURES
    atomic
    atomic_fortran
    barrier
    barrier_fortran
    critical
    critical_fortran
    declare_mapper_cplusplus
    end_declare_target
    flush
    flush_fortran
    parallel
    scan
    scan_fortran
    taskgroup
    taskgroup_fortran
    taskwait
    taskwait_fortran
    taskyield
    taskyield_fortran
    tile
    tile_fortran)

set(OMPPARSER_FIXTURE_HASH_atomic
    cfc0a4d86b8fcc7516b38ae379bb37043c62fb878c208f96dfd888980a6e178c)
set(OMPPARSER_FIXTURE_HASH_atomic_fortran
    f410ccb27c8bee03e5b460a71c805b7915fa27888c11a1962bfff9da44eb9144)
set(OMPPARSER_FIXTURE_HASH_barrier
    2c5268a1bb33757ed27ae8b5b79a53d5e2be3fe5e697ec584aa75f2fd960dc4c)
set(OMPPARSER_FIXTURE_HASH_barrier_fortran
    fb1ee7c5e30a6c48243d051bc1c855c3e36df9af80f9e16ebd56f3a24dd650e9)
set(OMPPARSER_FIXTURE_HASH_critical
    41e1c79af0cf4948acbba9335925777d73f3c15383d6c4953ddf684d5d084ccd)
set(OMPPARSER_FIXTURE_HASH_critical_fortran
    3b1ed47e85c2f2be7efb64ec4383f7db52b3c25cdb9b5fd4e27940b5ebd8d058)
set(OMPPARSER_FIXTURE_HASH_declare_mapper_cplusplus
    410659f6bfe1cfe43e1352b260cb9fdb3e92a6e3944daf0b84ea1b27d18384b9)
set(OMPPARSER_FIXTURE_HASH_end_declare_target
    2e6f171b3b91b71df041a7429b09d110235b6089ba05e54f285531f395c387a7)
set(OMPPARSER_FIXTURE_HASH_flush
    284bbaa10affccd5f9d810f081276e34038c317fa444ec0cddc94534de8f371f)
set(OMPPARSER_FIXTURE_HASH_flush_fortran
    660368d7272a23db253b829658ba340f30ab96de70035b8eca846f9dfb20fc9e)
set(OMPPARSER_FIXTURE_HASH_parallel
    fbb0d907dcf7492d52569ceaacdf0bd9b719d5bc205d983fedc93645458025d6)
set(OMPPARSER_FIXTURE_HASH_scan
    1a2eef1b46deeb81d1bffc61f25fe2cf6932f1928d258461f6c778682f1a53fe)
set(OMPPARSER_FIXTURE_HASH_scan_fortran
    991407c7ea313eae048cc4869fbf9d10ef83b943839edd4b72e566e52cb4725c)
set(OMPPARSER_FIXTURE_HASH_taskgroup
    4a25b34a4415bbd483656902397d97466c01c772279512215e8563d76c68e310)
set(OMPPARSER_FIXTURE_HASH_taskgroup_fortran
    f08769cb3a39fb37aaf5a676a448a35b768420a3036fdd15d37dbe7c6cbe5a17)
set(OMPPARSER_FIXTURE_HASH_taskwait
    3a0b960a5234aadfe175dc365ce152597d7d891c6112b97f1fc5b565ea33947e)
set(OMPPARSER_FIXTURE_HASH_taskwait_fortran
    7af550843d1cac8b9b396dbe2705250aaafc5d1281a2ce95e975383a0767dfd3)
set(OMPPARSER_FIXTURE_HASH_taskyield
    e7719b53e2f92fe18b8fa7909215f42ebc4d800bc88bd41c3f3642dc6f87eb67)
set(OMPPARSER_FIXTURE_HASH_taskyield_fortran
    749777bc4c52e4a085163f4a2cfe131813fb08348379aa4f174d5fdbf759c39f)
set(OMPPARSER_FIXTURE_HASH_tile
    049b60ade0ec07ae08df4e214931ca54b9d84758596437005fb9247c2d5fb1d5)
set(OMPPARSER_FIXTURE_HASH_tile_fortran
    ff152eeb744d4321a964b2b126322d6890baf1be9eedfa2ecac71e3ccf6a8d31)

set(OMPPARSER_GATE_TESTS
    compat_basic
    ompparser_drop_in
    comprehensive
    strict_contract)

foreach(FIXTURE_NAME IN LISTS OMPPARSER_AUDITED_FIXTURES)
    set(PINNED_TEST_FILE
        "${CMAKE_CURRENT_SOURCE_DIR}/ompparser/tests/builtin/${FIXTURE_NAME}.txt")
    if(NOT EXISTS "${PINNED_TEST_FILE}")
        message(FATAL_ERROR
            "Audited ompparser fixture is missing: ${PINNED_TEST_FILE}")
    endif()
    set(EXPECTED_HASH_VARIABLE "OMPPARSER_FIXTURE_HASH_${FIXTURE_NAME}")
    if(NOT DEFINED ${EXPECTED_HASH_VARIABLE})
        message(FATAL_ERROR
            "Audited ompparser fixture has no pinned hash: ${FIXTURE_NAME}")
    endif()
    file(SHA256 "${PINNED_TEST_FILE}" ACTUAL_FIXTURE_HASH)
    if(NOT ACTUAL_FIXTURE_HASH STREQUAL "${${EXPECTED_HASH_VARIABLE}}")
        message(FATAL_ERROR
            "Audited ompparser fixture changed and requires re-audit: ${FIXTURE_NAME}")
    endif()

    set(TEST_FILE "${PINNED_TEST_FILE}")
    if(FIXTURE_NAME STREQUAL "atomic")
        set(TEST_FILE "${ATOMIC_STRICT_FILE}")
    elseif(FIXTURE_NAME STREQUAL "flush")
        set(TEST_FILE "${FLUSH_STRICT_FILE}")
    elseif(FIXTURE_NAME STREQUAL "flush_fortran")
        set(TEST_FILE "${FLUSH_FORTRAN_STRICT_FILE}")
    elseif(FIXTURE_NAME STREQUAL "parallel")
        set(TEST_FILE "${PARALLEL_STRICT_FILE}")
    endif()

    set(TEST_LANG_ARGS)
    if(FIXTURE_NAME MATCHES "_cplusplus$")
        set(TEST_LANG_ARGS "--lang=c++")
    endif()

    set(TEST_NAME "fixture_${FIXTURE_NAME}")
    add_test(NAME "${TEST_NAME}"
             COMMAND ${CMAKE_COMMAND} -E env
                     "${OMPPARSER_TEST_LD_LIBRARY_PATH}"
                     $<TARGET_FILE:strict_fixture_tester>
                     ${TEST_LANG_ARGS} "${TEST_FILE}"
             WORKING_DIRECTORY "${CMAKE_BINARY_DIR}")
    list(APPEND OMPPARSER_GATE_TESTS "${TEST_NAME}")
endforeach()

# A timeout is a hard failure. It prevents a future fixture from silently
# turning the gate back into a whole-source stress run.
set_tests_properties(${OMPPARSER_GATE_TESTS}
    PROPERTIES LABELS "strict-compat-gate" TIMEOUT 20)
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
