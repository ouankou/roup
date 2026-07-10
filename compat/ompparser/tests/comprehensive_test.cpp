/*
 * Comprehensive Test Suite for ROUP ompparser Compatibility Layer
 * 
 * Tests:
 * - Basic directive parsing
 * - Clause handling
 * - Error cases
 * - Memory management
 * - String generation
 * - Language modes
 * 
 * Memory Management Strategy:
 * - ALL tests now use DirectivePtr (std::unique_ptr with custom deleter) for RAII
 * - This ensures cleanup even if assertions throw exceptions
 * - Prevents memory leaks during test failures
 * - No manual delete calls needed
 * 
 * Copyright (c) 2025 ROUP Project
 * SPDX-License-Identifier: BSD-3-Clause
 */

#include <OpenMPIR.h>
#include "../src/roup_compat.h"
#include <iostream>
#include <cassert>
#include <string>
#include <cstring>
#include <memory>

// Test counter
static int tests_passed = 0;
static int tests_failed = 0;

// ============================================================================
// RAII Wrapper for OpenMPDirective to prevent leaks on test failure
// ============================================================================

struct DirectiveDeleter {
    void operator()(OpenMPDirective* dir) const {
        delete dir;
    }
};

// Unique pointer type for automatic cleanup
using DirectivePtr = std::unique_ptr<OpenMPDirective, DirectiveDeleter>;

// Macros for testing
#define TEST(name) \
    void test_##name(); \
    void run_##name() { \
        std::cout << "Testing: " << #name << "..." << std::flush; \
        try { \
            test_##name(); \
            std::cout << " ✓ PASS" << std::endl; \
            tests_passed++; \
        } catch (const std::exception& e) { \
            std::cout << " ✗ FAIL: " << e.what() << std::endl; \
            tests_failed++; \
        } catch (...) { \
            std::cout << " ✗ FAIL: Unknown exception" << std::endl; \
            tests_failed++; \
        } \
    } \
    void test_##name()

#define ASSERT(cond) \
    if (!(cond)) { \
        throw std::runtime_error("Assertion failed: " #cond); \
    }

#define ASSERT_EQ(a, b) \
    if ((a) != (b)) { \
        throw std::runtime_error(std::string("Assertion failed: ") + #a + " != " + #b); \
    }

#define ASSERT_NE(a, b) \
    if ((a) == (b)) { \
        throw std::runtime_error(std::string("Assertion failed: ") + #a + " == " + #b); \
    }

static void assert_parse_hard_error(const char* input) {
    bool threw = false;
    try {
        DirectivePtr directive(parseOpenMP(input, nullptr));
    } catch (const std::exception&) {
        threw = true;
    }
    ASSERT(threw);
}

#define ASSERT_NULL(ptr) ASSERT((ptr) == nullptr)
#define ASSERT_NOT_NULL(ptr) ASSERT((ptr) != nullptr)

// ============================================================================
// Basic Directive Tests
// ============================================================================

TEST(parallel_directive) {
    DirectivePtr dir(parseOpenMP("#pragma omp parallel", nullptr));
    ASSERT_NOT_NULL(dir.get());
    ASSERT_EQ(dir->getKind(), OMPD_parallel);
}

TEST(parallel_with_pragma) {
    DirectivePtr dir(parseOpenMP("#pragma omp parallel", nullptr));
    ASSERT_NOT_NULL(dir.get());
    ASSERT_EQ(dir->getKind(), OMPD_parallel);
}

TEST(for_directive) {
    DirectivePtr dir(parseOpenMP("#pragma omp for", nullptr));
    ASSERT_NOT_NULL(dir.get());
    ASSERT_EQ(dir->getKind(), OMPD_for);
}

TEST(sections_directive) {
    DirectivePtr dir(parseOpenMP("#pragma omp sections", nullptr));
    ASSERT_NOT_NULL(dir.get());
    ASSERT_EQ(dir->getKind(), OMPD_sections);
}

TEST(single_directive) {
    DirectivePtr dir(parseOpenMP("#pragma omp single", nullptr));
    ASSERT_NOT_NULL(dir.get());
    ASSERT_EQ(dir->getKind(), OMPD_single);
}

TEST(task_directive) {
    DirectivePtr dir(parseOpenMP("#pragma omp task", nullptr));
    ASSERT_NOT_NULL(dir.get());
    ASSERT_EQ(dir->getKind(), OMPD_task);
}

TEST(barrier_directive) {
    DirectivePtr dir(parseOpenMP("#pragma omp barrier", nullptr));
    ASSERT_NOT_NULL(dir.get());
    ASSERT_EQ(dir->getKind(), OMPD_barrier);
}

TEST(taskwait_directive) {
    DirectivePtr dir(parseOpenMP("#pragma omp taskwait", nullptr));
    ASSERT_NOT_NULL(dir.get());
    ASSERT_EQ(dir->getKind(), OMPD_taskwait);
}

TEST(critical_directive) {
    DirectivePtr dir(parseOpenMP("#pragma omp critical", nullptr));
    ASSERT_NOT_NULL(dir.get());
    ASSERT_EQ(dir->getKind(), OMPD_critical);
}

TEST(master_directive) {
    DirectivePtr dir(parseOpenMP("#pragma omp master", nullptr));
    ASSERT_NOT_NULL(dir.get());
    ASSERT_EQ(dir->getKind(), OMPD_master);
}

// ============================================================================
// Clause Tests
// ============================================================================

TEST(num_threads_clause) {
    DirectivePtr dir(parseOpenMP("#pragma omp parallel num_threads(4)", nullptr));
    ASSERT_NOT_NULL(dir.get());
    ASSERT_EQ(dir->getKind(), OMPD_parallel);
    
    auto* clauses = dir->getAllClauses();
    ASSERT_NOT_NULL(clauses);
    ASSERT(clauses->size() > 0);
}

TEST(private_clause) {
    DirectivePtr dir(parseOpenMP("#pragma omp parallel private(x)", nullptr));
    ASSERT_NOT_NULL(dir.get());
    
    auto* clauses = dir->getAllClauses();
    ASSERT_NOT_NULL(clauses);
    ASSERT(clauses->size() > 0);
}

TEST(shared_clause) {
    DirectivePtr dir(parseOpenMP("#pragma omp parallel shared(y)", nullptr));
    ASSERT_NOT_NULL(dir.get());
    
    auto* clauses = dir->getAllClauses();
    ASSERT_NOT_NULL(clauses);
}

TEST(firstprivate_clause) {
    DirectivePtr dir(parseOpenMP("#pragma omp parallel firstprivate(z)", nullptr));
    ASSERT_NOT_NULL(dir.get());
    
    auto* clauses = dir->getAllClauses();
    ASSERT_NOT_NULL(clauses);
}

TEST(firstprivate_modifier_preserves_state_and_cpp_qualified_name) {
    setLang(Lang_Cplusplus);
    DirectivePtr dir(parseOpenMP(
        "#pragma omp target firstprivate(target, saved: ns::value)", nullptr
    ));
    ASSERT_NOT_NULL(dir.get());
    auto *clauses = dir->getClauses(OMPC_firstprivate);
    ASSERT(clauses != nullptr && clauses->size() == 1);
    auto *firstprivate =
        dynamic_cast<OpenMPFirstprivateClause *>(clauses->front());
    ASSERT(firstprivate != nullptr);
    ASSERT(firstprivate->hasDirectiveNameModifier());
    ASSERT(firstprivate->getDirectiveNameModifier() == OMPD_target);
    ASSERT(firstprivate->isSaved());
    ASSERT(firstprivate->getExpressions()->size() == 1);
    ASSERT(std::string(firstprivate->getExpressions()->front()) == "ns::value");
    setLang(Lang_C);
}

TEST(induction_requires_a_structured_upstream_api) {
    assert_parse_hard_error(
        "#pragma omp parallel for induction(step(i - -1), *: index)");
}

TEST(apply_requires_structured_applied_directives_upstream) {
    assert_parse_hard_error(
        "#pragma omp tile sizes(4) apply(grid: reverse)");
}

TEST(prefer_type_requires_structured_preferences_upstream) {
    assert_parse_hard_error(
        "#pragma omp interop init(prefer_type({fr(\"cuda\")}), target: object)");
}

TEST(declare_induction_requires_a_structured_upstream_api) {
    assert_parse_hard_error(
        "#pragma omp declare induction (+ : int, (long, short)) "
        "collector(omp_out + omp_in) inductor(omp_priv + omp_step)");
}

TEST(declare_reduction_preserves_historical_and_current_semantics) {
    DirectivePtr historical(parseOpenMP(
        "#pragma omp declare reduction(sum : int : omp_out += omp_in) "
        "initializer(omp_priv = 0)",
        nullptr
    ));
    ASSERT_NOT_NULL(historical.get());
    ASSERT_EQ(historical->getKind(), OMPD_declare_reduction);
    auto* historical_reduction =
        dynamic_cast<OpenMPDeclareReductionDirective*>(historical.get());
    ASSERT_NOT_NULL(historical_reduction);
    ASSERT_EQ(historical_reduction->getIdentifier(), std::string("sum"));
    ASSERT_EQ(historical_reduction->getCombiner(),
              std::string("omp_out += omp_in"));
    auto* historical_types = historical_reduction->getTypenameList();
    ASSERT_NOT_NULL(historical_types);
    ASSERT_EQ(historical_types->size(), static_cast<std::size_t>(1));
    ASSERT_EQ(std::string(historical_types->at(0)), std::string("int"));
    const std::string historical_text = historical->generatePragmaString();
    ASSERT(historical_text.find(
               "declare reduction(sum : int : omp_out += omp_in)") !=
           std::string::npos);
    ASSERT(historical_text.find("initializer(omp_priv = 0)") !=
           std::string::npos);

    setLang(Lang_Cplusplus);
    DirectivePtr current(parseOpenMP(
        "#pragma omp declare_reduction(ns::merge<int> : std::vector<int>) "
        "combiner(omp_out += omp_in) initializer(omp_priv(omp_orig))",
        nullptr
    ));
    setLang(Lang_C);
    ASSERT_NOT_NULL(current.get());
    ASSERT_EQ(current->getKind(), OMPD_declare_reduction);
    auto* current_reduction =
        dynamic_cast<OpenMPDeclareReductionDirective*>(current.get());
    ASSERT_NOT_NULL(current_reduction);
    ASSERT_EQ(current_reduction->getIdentifier(),
              std::string("ns::merge<int>"));
    ASSERT_EQ(current_reduction->getCombiner(),
              std::string("omp_out += omp_in"));
    auto* current_types = current_reduction->getTypenameList();
    ASSERT_NOT_NULL(current_types);
    ASSERT_EQ(current_types->size(), static_cast<std::size_t>(1));
    ASSERT_EQ(std::string(current_types->at(0)),
              std::string("std::vector < int >"));
    const std::string current_text = current->generatePragmaString();
    ASSERT(current_text.find("ns::merge<int>") != std::string::npos);
    ASSERT(current_text.find("std::vector < int >") != std::string::npos);
    ASSERT(current_text.find("initializer(omp_priv(omp_orig))") !=
           std::string::npos);
}

TEST(declare_reduction_preserves_cpp_operator_function_id) {
    setLang(Lang_Cplusplus);
    DirectivePtr dir(parseOpenMP(
        "#pragma omp declare_reduction(ns::operator+ : widget) "
        "combiner(omp_out += omp_in) initializer(omp_priv = omp_orig)",
        nullptr
    ));
    setLang(Lang_C);
    ASSERT_NOT_NULL(dir.get());
    auto* reduction = dynamic_cast<OpenMPDeclareReductionDirective*>(dir.get());
    ASSERT_NOT_NULL(reduction);
    ASSERT_EQ(reduction->getIdentifier(), std::string("ns::operator+"));
    ASSERT_EQ(reduction->getCombiner(), std::string("omp_out += omp_in"));
    const std::string str = dir->generatePragmaString();
    ASSERT(str.find("ns::operator+") != std::string::npos);
    ASSERT(str.find("initializer(omp_priv = omp_orig)") != std::string::npos);
}

TEST(declare_reduction_preserves_fortran_ids_and_assignments) {
    setLang(Lang_Fortran);
    DirectivePtr historical(parseOpenMP(
        "!$omp declare reduction(IAND : integer : "
        "omp_out = iand(omp_in, omp_out)) initializer(omp_priv = 0)",
        nullptr
    ));
    setLang(Lang_C);
    ASSERT_NOT_NULL(historical.get());
    auto* intrinsic =
        dynamic_cast<OpenMPDeclareReductionDirective*>(historical.get());
    ASSERT_NOT_NULL(intrinsic);
    ASSERT_EQ(intrinsic->getIdentifier(), std::string("iand"));
    ASSERT_EQ(intrinsic->getCombiner(),
              std::string("omp_out = iand(omp_in, omp_out)"));
    const std::string historical_text = historical->generatePragmaString();
    ASSERT(historical_text.find(
               "declare reduction(iand : integer : "
               "omp_out = iand(omp_in, omp_out))") != std::string::npos);
    ASSERT(historical_text.find("initializer(omp_priv = 0)") !=
           std::string::npos);

    setLang(Lang_Fortran);
    DirectivePtr current(parseOpenMP(
        "!$omp declare_reduction(.COMBINE. : integer) "
        "combiner(omp_out = combine_values(omp_out, omp_in)) "
        "initializer(omp_priv = omp_orig)",
        nullptr
    ));
    setLang(Lang_C);
    ASSERT_NOT_NULL(current.get());
    auto* defined = dynamic_cast<OpenMPDeclareReductionDirective*>(current.get());
    ASSERT_NOT_NULL(defined);
    ASSERT_EQ(defined->getIdentifier(), std::string(".combine."));
    ASSERT_EQ(defined->getCombiner(),
              std::string("omp_out = combine_values(omp_out, omp_in)"));
    const std::string current_text = current->generatePragmaString();
    ASSERT(current_text.find(".combine.") != std::string::npos);
    ASSERT(current_text.find("omp_out = combine_values(omp_out, omp_in)") !=
           std::string::npos);
    ASSERT(current_text.find("initializer(omp_priv = omp_orig)") !=
           std::string::npos);
}

TEST(declare_induction_cpp_ids_hard_error_without_passthrough) {
    setLang(Lang_Cplusplus);
    assert_parse_hard_error(
        "#pragma omp declare_induction(ns::step<int> : (state_t, step_t)) "
        "inductor(omp_var += omp_step) collector(omp_step * omp_idx)");

    assert_parse_hard_error(
        "#pragma omp declare_induction(ns::operator+ : (state_t, step_t)) "
        "inductor(omp_var += omp_step) collector(omp_step * omp_idx)");
    setLang(Lang_C);
}

TEST(expression_lists_preserve_order_and_nested_commas) {
    DirectivePtr dir(parseOpenMP(
        "#pragma omp tile sizes(f(1, 2), n + 1)", nullptr
    ));
    ASSERT_NOT_NULL(dir.get());

    const std::string str = dir->generatePragmaString();
    ASSERT(str.find("sizes(f(1, 2), n + 1)") != std::string::npos);
}

TEST(detach_preserves_one_typed_event_locator) {
    DirectivePtr dir(parseOpenMP(
        "#pragma omp task detach(event_handle)", nullptr
    ));
    ASSERT_NOT_NULL(dir.get());

    const std::string str = dir->generatePragmaString();
    ASSERT(str.find("detach(event_handle)") != std::string::npos);
}

TEST(multiple_clauses) {
    DirectivePtr dir(parseOpenMP(
        "#pragma omp parallel num_threads(4) private(x) shared(y)", nullptr
    ));
    ASSERT_NOT_NULL(dir.get());
    
    auto* clauses = dir->getAllClauses();
    ASSERT_NOT_NULL(clauses);
    ASSERT(clauses->size() >= 2);  // At least num_threads and private
}

TEST(reduction_clause) {
    DirectivePtr dir(parseOpenMP("#pragma omp parallel reduction(+:sum)", nullptr));
    ASSERT_NOT_NULL(dir.get());

    auto* clauses = dir->getAllClauses();
    ASSERT_NOT_NULL(clauses);
}

TEST(multiline_c_directive) {
    const char* input =
        "#pragma omp parallel for \\\n"
        "    schedule(dynamic, 4) \\\n"
        "    private(i, \\\n"
        "            j)";

    DirectivePtr dir(parseOpenMP(input, nullptr));
    ASSERT_NOT_NULL(dir.get());
    ASSERT_EQ(dir->getKind(), OMPD_parallel_for);

    auto* clauses = dir->getAllClauses();
    ASSERT_NOT_NULL(clauses);
    ASSERT_EQ(clauses->size(), 2);
}

TEST(multiline_fortran_directive) {
    setLang(Lang_Fortran);
    const char* input =
        "!$omp target teams distribute &\n"
        "!$omp parallel do &\n"
        "!$omp& private(i, j)";

    DirectivePtr dir(parseOpenMP(input, nullptr));
    const auto kind = dir->getKind();
    const auto clause_count = dir->getAllClauses()->size();
    setLang(Lang_C);
    ASSERT_NOT_NULL(dir.get());
    ASSERT_EQ(kind, OMPD_target_teams_distribute_parallel_do);
    ASSERT_EQ(clause_count, 1);
}

TEST(schedule_clause) {
    DirectivePtr dir(parseOpenMP("#pragma omp for schedule(static, 64)", nullptr));
    ASSERT_NOT_NULL(dir.get());
    
    auto* clauses = dir->getAllClauses();
    ASSERT_NOT_NULL(clauses);
}

TEST(if_clause) {
    DirectivePtr dir(parseOpenMP("#pragma omp parallel if(n > 1000)", nullptr));
    ASSERT_NOT_NULL(dir.get());
    
    auto* clauses = dir->getAllClauses();
    ASSERT_NOT_NULL(clauses);
}

TEST(nowait_clause) {
    DirectivePtr dir(parseOpenMP("#pragma omp for nowait", nullptr));
    ASSERT_NOT_NULL(dir.get());
    
    auto* clauses = dir->getAllClauses();
    ASSERT_NOT_NULL(clauses);
}

// ============================================================================
// String Generation Tests
// ============================================================================

TEST(toString_basic) {
    DirectivePtr dir(parseOpenMP("#pragma omp parallel", nullptr));
    ASSERT_NOT_NULL(dir.get());
    
    std::string str = dir->toString();
    ASSERT(str.find("parallel") != std::string::npos);
}

TEST(toString_with_clause) {
    DirectivePtr dir(parseOpenMP("#pragma omp parallel num_threads(4)", nullptr));
    ASSERT_NOT_NULL(dir.get());
    
    std::string str = dir->toString();
    ASSERT(str.find("parallel") != std::string::npos);
}

TEST(generatePragmaString_default) {
    DirectivePtr dir(parseOpenMP("#pragma omp parallel", nullptr));
    ASSERT_NOT_NULL(dir.get());
    
    std::string str = dir->generatePragmaString();
    ASSERT(str.find("#pragma omp") != std::string::npos);
    ASSERT(str.find("parallel") != std::string::npos);
}

TEST(generatePragmaString_custom_prefix) {
    DirectivePtr dir(parseOpenMP("#pragma omp parallel", nullptr));
    ASSERT_NOT_NULL(dir.get());
    
    std::string str = dir->generatePragmaString("!$omp ", "", "");
    ASSERT(str.find("!$omp") != std::string::npos);
}

// ============================================================================
// Error Handling Tests
// ============================================================================

TEST(null_input) {
    assert_parse_hard_error(nullptr);
}

TEST(empty_string) {
    assert_parse_hard_error("");
}

TEST(invalid_directive) {
    assert_parse_hard_error("#pragma omp invalidstuff");
}

TEST(malformed_pragma) {
    assert_parse_hard_error("pragma omp parallel");
}

TEST(garbage_input) {
    assert_parse_hard_error("asdfjkl;");
}

// ============================================================================
// Memory Management Tests
// ============================================================================

TEST(multiple_allocations) {
    for (int i = 0; i < 100; i++) {
        DirectivePtr dir(parseOpenMP("#pragma omp parallel", nullptr));
        ASSERT_NOT_NULL(dir.get());
        // DirectivePtr automatically cleans up
    }
}

TEST(delete_null_safe) {
    DirectivePtr dir(nullptr);
    // DirectivePtr handles null safely
}

TEST(reuse_same_input) {
    const char* input = "#pragma omp parallel num_threads(4)";
    
    DirectivePtr dir1(parseOpenMP(input, nullptr));
    ASSERT_NOT_NULL(dir1.get());
    // dir1 cleaned up automatically
    
    DirectivePtr dir2(parseOpenMP(input, nullptr));
    ASSERT_NOT_NULL(dir2.get());
}

// ============================================================================
// Language Mode Tests
// ============================================================================

TEST(lang_c) {
    setLang(Lang_C);
    DirectivePtr dir(parseOpenMP("#pragma omp parallel", nullptr));
    ASSERT_NOT_NULL(dir.get());
    ASSERT_EQ(dir->getBaseLang(), Lang_C);
}

TEST(lang_cpp) {
    setLang(Lang_Cplusplus);
    DirectivePtr dir(parseOpenMP("#pragma omp parallel", nullptr));
    ASSERT_NOT_NULL(dir.get());
    ASSERT_EQ(dir->getBaseLang(), Lang_Cplusplus);
}

TEST(lang_fortran) {
    setLang(Lang_Fortran);
    DirectivePtr dir(parseOpenMP("!$omp parallel private(i)", nullptr));
    const auto language = dir->getBaseLang();
    const auto kind = dir->getKind();
    setLang(Lang_C);
    ASSERT_NOT_NULL(dir.get());
    ASSERT_EQ(language, Lang_Fortran);
    ASSERT_EQ(kind, OMPD_parallel);
}

// ============================================================================
// Complex Directive Tests
// ============================================================================

TEST(complex_parallel_for) {
    DirectivePtr dir(parseOpenMP(
        "#pragma omp parallel for num_threads(4) schedule(static, 64) private(i) reduction(+:sum)",
        nullptr
    ));
    ASSERT_NOT_NULL(dir.get());
    
    auto* clauses = dir->getAllClauses();
    ASSERT_NOT_NULL(clauses);
    ASSERT(clauses->size() >= 3);
    
    std::string str = dir->toString();
    ASSERT(str.length() > 0);
}

TEST(nested_clause_parsing) {
    DirectivePtr dir(parseOpenMP(
        "#pragma omp parallel if(parallel: n > 100) num_threads(omp_get_max_threads())",
        nullptr
    ));
    ASSERT_NOT_NULL(dir.get());
}

// ============================================================================
// Main Test Runner
// ============================================================================

int main() {
    std::cout << "========================================" << std::endl;
    std::cout << "  ROUP ompparser Compatibility Tests" << std::endl;
    std::cout << "========================================" << std::endl;
    std::cout << std::endl;
    
    assert_parse_hard_error("#pragma omp parallel");
    setLang(Lang_C);

    // Run all tests
    std::cout << "--- Basic Directive Tests ---" << std::endl;
    run_parallel_directive();
    run_parallel_with_pragma();
    run_for_directive();
    run_sections_directive();
    run_single_directive();
    run_task_directive();
    run_barrier_directive();
    run_taskwait_directive();
    run_critical_directive();
    run_master_directive();
    std::cout << std::endl;
    
    std::cout << "--- Clause Tests ---" << std::endl;
    run_num_threads_clause();
    run_private_clause();
    run_shared_clause();
    run_firstprivate_clause();
    run_firstprivate_modifier_preserves_state_and_cpp_qualified_name();
    run_induction_requires_a_structured_upstream_api();
    run_apply_requires_structured_applied_directives_upstream();
    run_prefer_type_requires_structured_preferences_upstream();
    run_declare_induction_requires_a_structured_upstream_api();
    run_declare_reduction_preserves_historical_and_current_semantics();
    run_declare_reduction_preserves_cpp_operator_function_id();
    run_declare_reduction_preserves_fortran_ids_and_assignments();
    run_declare_induction_cpp_ids_hard_error_without_passthrough();
    run_expression_lists_preserve_order_and_nested_commas();
    run_detach_preserves_one_typed_event_locator();
    run_multiple_clauses();
    run_reduction_clause();
    run_multiline_c_directive();
    run_multiline_fortran_directive();
    run_schedule_clause();
    run_if_clause();
    run_nowait_clause();
    std::cout << std::endl;
    
    std::cout << "--- String Generation Tests ---" << std::endl;
    run_toString_basic();
    run_toString_with_clause();
    run_generatePragmaString_default();
    run_generatePragmaString_custom_prefix();
    std::cout << std::endl;
    
    std::cout << "--- Error Handling Tests ---" << std::endl;
    run_null_input();
    run_empty_string();
    run_invalid_directive();
    run_malformed_pragma();
    run_garbage_input();
    std::cout << std::endl;
    
    std::cout << "--- Memory Management Tests ---" << std::endl;
    run_multiple_allocations();
    run_delete_null_safe();
    run_reuse_same_input();
    std::cout << std::endl;
    
    std::cout << "--- Language Mode Tests ---" << std::endl;
    run_lang_c();
    run_lang_cpp();
    run_lang_fortran();
    std::cout << std::endl;
    
    std::cout << "--- Complex Directive Tests ---" << std::endl;
    run_complex_parallel_for();
    run_nested_clause_parsing();
    std::cout << std::endl;
    
    // Summary
    std::cout << "========================================" << std::endl;
    std::cout << "  Test Results" << std::endl;
    std::cout << "========================================" << std::endl;
    std::cout << "Passed: " << tests_passed << std::endl;
    std::cout << "Failed: " << tests_failed << std::endl;
    std::cout << "Total:  " << (tests_passed + tests_failed) << std::endl;
    std::cout << std::endl;
    
    if (tests_failed == 0) {
        std::cout << "🎉 All tests passed!" << std::endl;
        return 0;
    } else {
        std::cout << "❌ Some tests failed!" << std::endl;
        return 1;
    }
}
