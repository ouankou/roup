/**
 * @file error_handling.c
 * @brief Demonstrates proper error handling and resource cleanup
 * 
 * This example shows:
 * - Checking return status codes
 * - Validating handles
 * - Handling parse errors
 * - Proper cleanup on error paths
 * - Using helper macros
 * 
 * Build:
 *   gcc -o error_handling error_handling.c -L../../target/debug -lroup -lpthread -ldl -lm
 * 
 * Run:
 *   LD_LIBRARY_PATH=../../target/debug ./error_handling
 */

#include "../../include/roup.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/**
 * @brief Convert status to string
 */
const char* status_to_string(OmpStatus status) {
    switch (status) {
        case OMP_SUCCESS: return "SUCCESS";
        case OMP_INVALID_HANDLE: return "INVALID_HANDLE";
        case OMP_INVALID_UTF8: return "INVALID_UTF8";
        case OMP_NULL_POINTER: return "NULL_POINTER";
        case OMP_OUT_OF_BOUNDS: return "OUT_OF_BOUNDS";
        case OMP_PARSE_ERROR: return "PARSE_ERROR";
        case OMP_TYPE_MISMATCH: return "TYPE_MISMATCH";
        case OMP_EMPTY_RESULT: return "EMPTY_RESULT";
        default: return "UNKNOWN";
    }
}

/**
 * @brief Example 1: Handling invalid handles
 */
void example_invalid_handle() {
    printf("Example 1: Invalid handle errors\n\n");
    
    Handle invalid = INVALID_HANDLE;
    DirectiveKind kind;
    
    printf("Attempting to use INVALID_HANDLE...\n");
    OmpStatus status = omp_directive_kind(invalid, &kind);
    printf("Result: %s\n", status_to_string(status));
    
    if (status != OMP_SUCCESS) {
        printf("✓ Correctly detected invalid handle\n");
    }
    
    // Try with a bogus handle value
    printf("\nAttempting to use arbitrary handle (12345)...\n");
    Handle bogus = 12345;
    status = omp_directive_kind(bogus, &kind);
    printf("Result: %s\n", status_to_string(status));
    
    if (status == OMP_INVALID_HANDLE) {
        printf("✓ Correctly rejected unknown handle\n");
    }
}

/**
 * @brief Example 2: Handling NULL pointers
 */
void example_null_pointer() {
    printf("\n----------------------------------------\n");
    printf("Example 2: NULL pointer errors\n\n");
    
    Handle str;
    
    printf("Attempting omp_str_new with NULL output pointer...\n");
    OmpStatus status = omp_str_new(NULL);
    printf("Result: %s\n", status_to_string(status));
    
    if (status == OMP_NULL_POINTER) {
        printf("✓ Correctly detected NULL pointer\n");
    }
    
    // Create a valid string
    if (omp_str_new(&str) == OMP_SUCCESS) {
        printf("\nAttempting omp_str_len with NULL output pointer...\n");
        status = omp_str_len(str, NULL);
        printf("Result: %s\n", status_to_string(status));
        
        if (status == OMP_NULL_POINTER) {
            printf("✓ Correctly detected NULL pointer\n");
        }
        
        omp_str_free(str);
    }
}

/**
 * @brief Example 3: Handling out of bounds access
 */
void example_out_of_bounds() {
    printf("\n----------------------------------------\n");
    printf("Example 3: Out of bounds errors\n\n");
    
    Handle result;
    if (omp_parse("#pragma omp parallel num_threads(4)", OMP_LANG_C, &result) == OMP_SUCCESS) {
        Handle *dirs;
        uintptr_t count;
        omp_take_last_parse_result(&dirs, &count);
        
        if (count > 0) {
            uintptr_t clause_count;
            omp_directive_clause_count(dirs[0], &clause_count);
            printf("Directive has %zu clause(s)\n", clause_count);
            
            // Try to access beyond bounds
            printf("Attempting to access clause at index %zu...\n", clause_count + 5);
            Handle clause;
            OmpStatus status = omp_clause_at(dirs[0], clause_count + 5, &clause);
            printf("Result: %s\n", status_to_string(status));
            
            if (status == OMP_OUT_OF_BOUNDS) {
                printf("✓ Correctly detected out of bounds access\n");
            }
        }
        
        free(dirs);
        omp_parse_result_free(result);
    }
}

/**
 * @brief Example 4: Handling parse errors
 */
void example_parse_error() {
    printf("\n----------------------------------------\n");
    printf("Example 4: Parse errors\n\n");
    
    const char *invalid_inputs[] = {
        "not an openmp directive",
        "#pragma omp unknown_directive",
        "",
    };
    
    for (int i = 0; i < 3; i++) {
        printf("Attempting to parse: \"%s\"\n", invalid_inputs[i]);
        Handle result;
        OmpStatus status = omp_parse(invalid_inputs[i], OMP_LANG_C, &result);
        printf("Result: %s\n", status_to_string(status));
        
        if (status == OMP_SUCCESS) {
            printf("  (Parsed successfully, checking if empty...)\n");
            Handle *dirs;
            uintptr_t count;
            if (omp_take_last_parse_result(&dirs, &count) == OMP_SUCCESS) {
                printf("  Found %zu directive(s)\n", count);
                free(dirs);
            }
            omp_parse_result_free(result);
        } else {
            printf("  ✓ Parse failed as expected\n");
        }
        printf("\n");
    }
}

/**
 * @brief Example 5: Type mismatch errors
 */
void example_type_mismatch() {
    printf("----------------------------------------\n");
    printf("Example 5: Type mismatch errors\n\n");
    
    Handle result;
    if (omp_parse("#pragma omp parallel private(x)", OMP_LANG_C, &result) == OMP_SUCCESS) {
        Handle *dirs;
        uintptr_t count;
        omp_take_last_parse_result(&dirs, &count);
        
        if (count > 0) {
            Handle clause;
            if (omp_clause_at(dirs[0], 0, &clause) == OMP_SUCCESS) {
                ClauseType type;
                omp_clause_type(clause, &type);
                printf("Clause type: %d (should be PRIVATE)\n", type);
                
                // Try to get num_threads value from a private clause
                printf("Attempting to get num_threads value from private clause...\n");
                Handle value;
                OmpStatus status = omp_clause_num_threads_value(clause, &value);
                printf("Result: %s\n", status_to_string(status));
                
                if (status == OMP_TYPE_MISMATCH) {
                    printf("✓ Correctly detected type mismatch\n");
                }
            }
        }
        
        free(dirs);
        omp_parse_result_free(result);
    }
}

/**
 * @brief Example 6: Proper cleanup on error
 */
void example_proper_cleanup() {
    printf("\n----------------------------------------\n");
    printf("Example 6: Proper cleanup patterns\n\n");
    
    Handle result = INVALID_HANDLE;
    Handle *dirs = NULL;
    Handle cursor = INVALID_HANDLE;
    OmpStatus status;
    
    printf("Parsing directive...\n");
    status = omp_parse("#pragma omp parallel for private(i)", OMP_LANG_C, &result);
    if (status != OMP_SUCCESS) {
        printf("Parse failed: %s\n", status_to_string(status));
        goto cleanup;
    }
    printf("✓ Parse succeeded\n");
    
    printf("Getting parse result...\n");
    uintptr_t count;
    status = omp_take_last_parse_result(&dirs, &count);
    if (status != OMP_SUCCESS) {
        printf("Failed to get result: %s\n", status_to_string(status));
        goto cleanup;
    }
    printf("✓ Got %zu directive(s)\n", count);
    
    if (count > 0) {
        printf("Creating cursor...\n");
        status = omp_directive_clauses_cursor(dirs[0], &cursor);
        if (status != OMP_SUCCESS) {
            printf("Failed to create cursor: %s\n", status_to_string(status));
            goto cleanup;
        }
        printf("✓ Cursor created\n");
        
        // Simulate an error by trying invalid operation
        printf("Simulating error condition...\n");
        Handle invalid_clause;
        status = omp_clause_at(dirs[0], 999, &invalid_clause);
        if (status != OMP_SUCCESS) {
            printf("Error occurred: %s\n", status_to_string(status));
            printf("Jumping to cleanup...\n");
            goto cleanup;
        }
    }
    
cleanup:
    printf("\nCleaning up resources...\n");
    if (OMP_IS_VALID(cursor)) {
        omp_cursor_free(cursor);
        printf("  ✓ Freed cursor\n");
    }
    if (dirs != NULL) {
        free(dirs);
        printf("  ✓ Freed directives array\n");
    }
    if (OMP_IS_VALID(result)) {
        omp_parse_result_free(result);
        printf("  ✓ Freed parse result\n");
    }
    printf("Cleanup complete\n");
}

/**
 * @brief Example 7: Using helper macros
 */
void example_helper_macros() {
    printf("\n----------------------------------------\n");
    printf("Example 7: Using OMP_CHECK macro\n\n");
    
    Handle str;
    
    printf("Using OMP_IS_VALID macro...\n");
    Handle test_handle = INVALID_HANDLE;
    if (OMP_IS_INVALID(test_handle)) {
        printf("✓ OMP_IS_INVALID correctly identified INVALID_HANDLE\n");
    }
    
    if (omp_str_new(&str) == OMP_SUCCESS) {
        if (OMP_IS_VALID(str)) {
            printf("✓ OMP_IS_VALID correctly identified valid handle\n");
        }
        omp_str_free(str);
    }
}

int main() {
    printf("=== Error Handling Example ===\n\n");
    
    example_invalid_handle();
    example_null_pointer();
    example_out_of_bounds();
    example_parse_error();
    example_type_mismatch();
    example_proper_cleanup();
    example_helper_macros();
    
    printf("\n=== All examples completed successfully ===\n");
    return 0;
}
