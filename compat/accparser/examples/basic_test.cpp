/*
 * Basic test for ROUP accparser compatibility layer
 *
 * Tests:
 * 1. Parse OpenACC pragmas using parseOpenACC()
 * 2. Query directive kind
 * 3. Query clauses
 * 4. Generate pragma string
 */

#include <OpenACCIR.h>
#include "../src/roup_acc_compat.h"
#include <iostream>
#include <cassert>

void test_simple_parallel() {
    std::cout << "Test 1: #pragma acc parallel" << std::endl;

    OpenACCDirective* dir = parseOpenACC("acc parallel", nullptr);
    assert(dir != nullptr);
    assert(dir->getKind() == ACCD_parallel);

    std::string str = dir->toString();
    std::cout << "  Generated: " << str << std::endl;
    assert(str.find("parallel") != std::string::npos);

    delete dir;
    std::cout << "  ✓ PASS\n" << std::endl;
}

void test_parallel_num_gangs() {
    std::cout << "Test 2: #pragma acc parallel num_gangs(4)" << std::endl;

    OpenACCDirective* dir = parseOpenACC("acc parallel num_gangs(4)", nullptr);
    assert(dir != nullptr);
    assert(dir->getKind() == ACCD_parallel);

    auto* all_clauses = dir->getAllClauses();
    assert(all_clauses != nullptr);
    std::cout << "  Clauses: " << all_clauses->size() << " types" << std::endl;

    std::string str = dir->toString();
    std::cout << "  Generated: " << str << std::endl;

    delete dir;
    std::cout << "  ✓ PASS\n" << std::endl;
}

void test_loop_directive() {
    std::cout << "Test 3: #pragma acc loop" << std::endl;

    OpenACCDirective* dir = parseOpenACC("acc loop", nullptr);
    assert(dir != nullptr);
    assert(dir->getKind() == ACCD_loop);

    std::string str = dir->toString();
    std::cout << "  Generated: " << str << std::endl;

    delete dir;
    std::cout << "  ✓ PASS\n" << std::endl;
}

void test_kernels_directive() {
    std::cout << "Test 4: #pragma acc kernels" << std::endl;

    OpenACCDirective* dir = parseOpenACC("acc kernels", nullptr);
    assert(dir != nullptr);
    assert(dir->getKind() == ACCD_kernels);

    std::string str = dir->toString();
    std::cout << "  Generated: " << str << std::endl;

    delete dir;
    std::cout << "  ✓ PASS\n" << std::endl;
}

void test_data_clauses() {
    std::cout << "Test 5: #pragma acc data copy(x) copyin(y)" << std::endl;

    OpenACCDirective* dir = parseOpenACC("acc data copy(x) copyin(y)", nullptr);
    assert(dir != nullptr);
    assert(dir->getKind() == ACCD_data);

    auto* all_clauses = dir->getAllClauses();
    std::cout << "  Clauses: " << all_clauses->size() << " types" << std::endl;

    std::string str = dir->toString();
    std::cout << "  Generated: " << str << std::endl;

    delete dir;
    std::cout << "  ✓ PASS\n" << std::endl;
}

void test_invalid_input() {
    std::cout << "Test 6: Invalid input" << std::endl;

    OpenACCDirective* dir = parseOpenACC("not a pragma", nullptr);
    assert(dir == nullptr);

    std::cout << "  ✓ PASS (correctly rejected)\n" << std::endl;
}

int main() {
    std::cout << "======================================" << std::endl;
    std::cout << "  ROUP accparser Compatibility Tests" << std::endl;
    std::cout << "======================================\n" << std::endl;

    setLang(ACC_Lang_C);

    test_simple_parallel();
    test_parallel_num_gangs();
    test_loop_directive();
    test_kernels_directive();
    test_data_clauses();
    test_invalid_input();

    std::cout << "======================================" << std::endl;
    std::cout << "  All 6 tests passed! ✓" << std::endl;
    std::cout << "======================================" << std::endl;

    return 0;
}
