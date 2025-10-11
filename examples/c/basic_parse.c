/**
 * @file basic_parse.c
 * @brief Basic example of parsing OpenMP directives and querying them
 * 
 * This example demonstrates:
 * - Parsing OpenMP directives from a string using omp_parse_cstr()
 * - Querying directive properties (kind, location, clause count)
 * - Iterating through clauses with cursors
 * - Proper resource cleanup
 * 
 * Build:
 *   gcc -o basic_parse basic_parse.c -L../../target/debug -lroup -lpthread -ldl -lm
 * 
 * Run:
 *   LD_LIBRARY_PATH=../../target/debug ./basic_parse
 */

#include "../../include/roup.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/**
 * @brief Get directive kind as string
 */
const char* directive_kind_to_string(int32_t kind) {
    switch (kind) {
        case 0: return "parallel";
        case 1: return "for";
        case 2: return "parallel for";
        case 3: return "task";
        case 4: return "target";
        case 5: return "teams";
        case 6: return "simd";
        default: return "unknown";
    }
}

/**
 * @brief Print directive information
 */
void print_directive(Handle directive) {
    int32_t kind;
    uint32_t line, column;
    int32_t lang;
    uintptr_t clause_count;
    
    // Query directive properties using pointer-based API
    if (omp_directive_kind_ptr(directive, &kind) != OMP_SUCCESS) {
        printf("  Error: Failed to get directive kind\n");
        return;
    }
    
    omp_directive_line_ptr(directive, &line);
    omp_directive_column_ptr(directive, &column);
    omp_directive_language_ptr(directive, &lang);
    omp_directive_clause_count_ptr(directive, &clause_count);
    
    printf("Directive: %s\n", directive_kind_to_string(kind));
    printf("  Location: line %u, column %u\n", line, column);
    printf("  Language: %s\n", lang == OMP_LANG_C ? "C" : "Fortran");
    printf("  Clauses: %zu\n", clause_count);
    
    // Iterate through clauses using cursor
    if (clause_count > 0) {
        printf("  Iterating clauses with cursor:\n");
        Handle cursor;
        if (omp_directive_clauses_cursor_ptr(directive, &cursor) == OMP_SUCCESS) {
            uintptr_t total;
            omp_cursor_total_ptr(cursor, &total);
            printf("  Total clauses in cursor: %zu\n", total);
            
            uintptr_t pos = 0;
            int32_t has_next;
            while (omp_cursor_has_next_ptr(cursor, &has_next) == OMP_SUCCESS && has_next) {
                printf("    Position %zu\n", pos);
                omp_cursor_next(cursor);
                pos++;
            }
            omp_cursor_free(cursor);
        }
    }
}

int main() {
    printf("=== Roup OpenMP Parser - Basic Example ===\n\n");
    
    // Example 1: Simple parallel directive
    printf("Example 1: Simple parallel directive\n");
    printf("Input: \"#pragma omp parallel\"\n\n");
    
    Handle directive1;
    OmpStatus status = omp_parse_cstr("#pragma omp parallel", OMP_LANG_C, &directive1);
    
    if (status != OMP_SUCCESS) {
        printf("Error: Parse failed with status %d\n", status);
        return 1;
    }
    
    print_directive(directive1);
    omp_directive_free(directive1);
    
    // Example 2: Parallel directive with clauses
    printf("\n----------------------------------------\n");
    printf("Example 2: Parallel directive with clauses\n");
    printf("Input: \"#pragma omp parallel num_threads(4) private(x, y) shared(z)\"\n\n");
    
    Handle directive2;
    status = omp_parse_cstr("#pragma omp parallel num_threads(4) private(x, y) shared(z)", 
                            OMP_LANG_C, &directive2);
    
    if (status != OMP_SUCCESS) {
        printf("Error: Parse failed\n");
        return 1;
    }
    
    print_directive(directive2);
    omp_directive_free(directive2);
    
    // Example 3: Parallel for with schedule
    printf("\n----------------------------------------\n");
    printf("Example 3: Parallel for with schedule\n");
    printf("Input: \"#pragma omp parallel for schedule(static, 16)\"\n\n");
    
    Handle directive3;
    status = omp_parse_cstr("#pragma omp parallel for schedule(static, 16)", 
                           OMP_LANG_C, &directive3);
    
    if (status == OMP_SUCCESS) {
        print_directive(directive3);
        omp_directive_free(directive3);
    }
    
    printf("\n=== All examples completed successfully ===\n");
    return 0;
}
