/*
 * Comprehensive Test Suite for ROUP accparser Compatibility Layer
 *
 * Tests: Basic directives, clauses, error cases, memory management
 *
 * Copyright (c) 2025 ROUP Project
 * SPDX-License-Identifier: BSD-3-Clause
 */

#include <OpenACCIR.h>
#include "../src/roup_acc_compat.h"
#include <iostream>
#include <cassert>
#include <memory>
#include <stdexcept>

// Test counters
static int tests_passed = 0;
static int tests_failed = 0;

// RAII wrapper for automatic cleanup
struct DirectiveDeleter {
    void operator()(OpenACCDirective* dir) const { delete dir; }
};
using DirectivePtr = std::unique_ptr<OpenACCDirective, DirectiveDeleter>;

// Test macros
#define TEST(name) \
    void test_##name(); \
    void run_##name() { \
        std::cout << "  " << #name << "..." << std::flush; \
        try { \
            test_##name(); \
            std::cout << " ✓" << std::endl; \
            tests_passed++; \
        } catch (const std::exception& e) { \
            std::cout << " ✗ FAIL: " << e.what() << std::endl; \
            tests_failed++; \
        } \
    } \
    void test_##name()

#define ASSERT(cond) \
    if (!(cond)) throw std::runtime_error("Assertion failed: " #cond)

#define ASSERT_EQ(a, b) \
    if ((a) != (b)) throw std::runtime_error("Assertion failed: " #a " != " #b)

// =============================================================================
// Basic Directive Tests
// =============================================================================

TEST(parallel) {
    DirectivePtr dir(parseOpenACC("acc parallel", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_parallel);
}

TEST(loop) {
    DirectivePtr dir(parseOpenACC("acc loop", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_loop);
}

TEST(kernels) {
    DirectivePtr dir(parseOpenACC("acc kernels", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_kernels);
}

TEST(data) {
    DirectivePtr dir(parseOpenACC("acc data", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_data);
}

TEST(enter_data) {
    DirectivePtr dir(parseOpenACC("acc enter data", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_enter_data);
}

TEST(exit_data) {
    DirectivePtr dir(parseOpenACC("acc exit data", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_exit_data);
}

// =============================================================================
// Clause Tests
// =============================================================================

TEST(num_gangs_clause) {
    DirectivePtr dir(parseOpenACC("acc parallel num_gangs(4)", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_parallel);
    auto* clauses = dir->getAllClauses();
    ASSERT(clauses != nullptr);
    ASSERT(clauses->size() > 0);
}

TEST(num_workers_clause) {
    DirectivePtr dir(parseOpenACC("acc parallel num_workers(8)", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_parallel);
}

TEST(vector_length_clause) {
    DirectivePtr dir(parseOpenACC("acc parallel vector_length(32)", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_parallel);
}

TEST(async_clause) {
    DirectivePtr dir(parseOpenACC("acc parallel async", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_parallel);
}

TEST(wait_clause) {
    DirectivePtr dir(parseOpenACC("acc parallel wait", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_parallel);
}

TEST(private_clause) {
    DirectivePtr dir(parseOpenACC("acc parallel private(x,y)", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_parallel);
}

TEST(firstprivate_clause) {
    DirectivePtr dir(parseOpenACC("acc parallel firstprivate(a)", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_parallel);
}

TEST(reduction_clause) {
    DirectivePtr dir(parseOpenACC("acc parallel reduction(+:sum)", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_parallel);
}

// =============================================================================
// Data Clause Tests
// =============================================================================

TEST(copy_clause) {
    DirectivePtr dir(parseOpenACC("acc data copy(x)", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_data);
}

TEST(copyin_clause) {
    DirectivePtr dir(parseOpenACC("acc data copyin(x)", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_data);
}

TEST(copyout_clause) {
    DirectivePtr dir(parseOpenACC("acc data copyout(x)", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_data);
}

TEST(create_clause) {
    DirectivePtr dir(parseOpenACC("acc data create(x)", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_data);
}

TEST(present_clause) {
    DirectivePtr dir(parseOpenACC("acc data present(x)", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_data);
}

TEST(present_or_copy_clause) {
    DirectivePtr dir(parseOpenACC("acc data present_or_copy(x)", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_data);
}

TEST(present_or_copyin_clause) {
    DirectivePtr dir(parseOpenACC("acc data present_or_copyin(x)", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_data);
}

TEST(present_or_copyout_clause) {
    DirectivePtr dir(parseOpenACC("acc data present_or_copyout(x)", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_data);
}

TEST(present_or_create_clause) {
    DirectivePtr dir(parseOpenACC("acc data present_or_create(x)", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_data);
}

TEST(pcopy_clause) {
    DirectivePtr dir(parseOpenACC("acc data pcopy(x)", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_data);
}

TEST(pcopyin_clause) {
    DirectivePtr dir(parseOpenACC("acc data pcopyin(x)", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_data);
}

TEST(pcopyout_clause) {
    DirectivePtr dir(parseOpenACC("acc data pcopyout(x)", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_data);
}

TEST(pcreate_clause) {
    DirectivePtr dir(parseOpenACC("acc data pcreate(x)", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_data);
}

// =============================================================================
// Loop Clause Tests
// =============================================================================

TEST(gang_clause) {
    DirectivePtr dir(parseOpenACC("acc loop gang", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_loop);
}

TEST(worker_clause) {
    DirectivePtr dir(parseOpenACC("acc loop worker", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_loop);
}

TEST(vector_clause) {
    DirectivePtr dir(parseOpenACC("acc loop vector", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_loop);
}

TEST(seq_clause) {
    DirectivePtr dir(parseOpenACC("acc loop seq", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_loop);
}

TEST(independent_clause) {
    DirectivePtr dir(parseOpenACC("acc loop independent", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_loop);
}

TEST(collapse_clause) {
    DirectivePtr dir(parseOpenACC("acc loop collapse(2)", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_loop);
}

TEST(tile_clause) {
    DirectivePtr dir(parseOpenACC("acc loop tile(8,8)", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_loop);
}

// =============================================================================
// Runtime Clause Tests
// =============================================================================

TEST(update_self_clause) {
    DirectivePtr dir(parseOpenACC("acc update self(buf)", nullptr));
    ASSERT(dir != nullptr);
    ASSERT_EQ(dir->getKind(), ACCD_update);
}

// =============================================================================
// Error Cases
// =============================================================================

TEST(null_input) {
    DirectivePtr dir(parseOpenACC(nullptr, nullptr));
    ASSERT(dir == nullptr);
}

TEST(empty_input) {
    DirectivePtr dir(parseOpenACC("", nullptr));
    ASSERT(dir == nullptr);
}

TEST(invalid_pragma) {
    DirectivePtr dir(parseOpenACC("not a pragma", nullptr));
    ASSERT(dir == nullptr);
}

TEST(wrong_prefix) {
    DirectivePtr dir(parseOpenACC("omp parallel", nullptr));
    ASSERT(dir == nullptr);
}

// =============================================================================
// String Generation Tests
// =============================================================================

TEST(toString_basic) {
    DirectivePtr dir(parseOpenACC("acc parallel", nullptr));
    ASSERT(dir != nullptr);
    std::string str = dir->toString();
    ASSERT(str.find("parallel") != std::string::npos);
}

TEST(toString_with_clause) {
    DirectivePtr dir(parseOpenACC("acc parallel num_gangs(4)", nullptr));
    ASSERT(dir != nullptr);
    std::string str = dir->toString();
    ASSERT(str.find("parallel") != std::string::npos);
}

// =============================================================================
// Main Test Runner
// =============================================================================

int main() {
    std::cout << "======================================" << std::endl;
    std::cout << "  accparser Comprehensive Tests" << std::endl;
    std::cout << "======================================\n" << std::endl;

    setLang(ACC_Lang_C);

    std::cout << "Basic Directives:" << std::endl;
    run_parallel();
    run_loop();
    run_kernels();
    run_data();
    run_enter_data();
    run_exit_data();

    std::cout << "\nCompute Clauses:" << std::endl;
    run_num_gangs_clause();
    run_num_workers_clause();
    run_vector_length_clause();
    run_async_clause();
    run_wait_clause();
    run_private_clause();
    run_firstprivate_clause();
    run_reduction_clause();

    std::cout << "\nData Clauses:" << std::endl;
    run_copy_clause();
    run_copyin_clause();
    run_copyout_clause();
    run_create_clause();
    run_present_clause();
    run_present_or_copy_clause();
    run_present_or_copyin_clause();
    run_present_or_copyout_clause();
    run_present_or_create_clause();
    run_pcopy_clause();
    run_pcopyin_clause();
    run_pcopyout_clause();
    run_pcreate_clause();

    std::cout << "\nLoop Clauses:" << std::endl;
    run_gang_clause();
    run_worker_clause();
    run_vector_clause();
    run_seq_clause();
    run_independent_clause();
    run_collapse_clause();
    run_tile_clause();

    std::cout << "\nRuntime Clauses:" << std::endl;
    run_update_self_clause();

    std::cout << "\nError Handling:" << std::endl;
    run_null_input();
    run_empty_input();
    run_invalid_pragma();
    run_wrong_prefix();

    std::cout << "\nString Generation:" << std::endl;
    run_toString_basic();
    run_toString_with_clause();

    std::cout << "\n======================================" << std::endl;
    std::cout << "  Results: " << tests_passed << " passed";
    if (tests_failed > 0) {
        std::cout << ", " << tests_failed << " failed";
    }
    std::cout << "\n======================================" << std::endl;

    return (tests_failed == 0) ? 0 : 1;
}
