/**
 * @file basic_parse.c
 * @brief Basic example of parsing OpenMP directives and querying them
 * 
 * This example demonstrates:
 * - Parsing OpenMP directives from a string
 * - Querying directive properties (kind, location, clause count)
 * - Iterating through clauses
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
const char* directive_kind_to_string(DirectiveKind kind) {
    switch (kind) {
        case OMP_DIR_PARALLEL: return "parallel";
        case OMP_DIR_FOR: return "for";
        case OMP_DIR_PARALLEL_FOR: return "parallel for";
        case OMP_DIR_TASK: return "task";
        case OMP_DIR_TARGET: return "target";
        case OMP_DIR_TEAMS: return "teams";
        case OMP_DIR_SIMD: return "simd";
        default: return "unknown";
    }
}

/**
 * @brief Get clause type as string
 */
const char* clause_type_to_string(ClauseType type) {
    switch (type) {
        case OMP_CLAUSE_NUM_THREADS: return "num_threads";
        case OMP_CLAUSE_PRIVATE: return "private";
        case OMP_CLAUSE_SHARED: return "shared";
        case OMP_CLAUSE_REDUCTION: return "reduction";
        case OMP_CLAUSE_SCHEDULE: return "schedule";
        case OMP_CLAUSE_DEFAULT: return "default";
        case OMP_CLAUSE_NOWAIT: return "nowait";
        case OMP_CLAUSE_COLLAPSE: return "collapse";
        default: return "other";
    }
}

/**
 * @brief Print directive information
 */
void print_directive(Handle directive) {
    DirectiveKind kind;
    uintptr_t line, column, clause_count;
    Language lang;
    
    // Query directive properties
    if (omp_directive_kind(directive, &kind) != OMP_SUCCESS) {
        printf("  Error: Failed to get directive kind\n");
        return;
    }
    
    omp_directive_line(directive, &line);
    omp_directive_column(directive, &column);
    omp_directive_language(directive, &lang);
    omp_directive_clause_count(directive, &clause_count);
    
    printf("Directive: %s\n", directive_kind_to_string(kind));
    printf("  Location: line %zu, column %zu\n", line, column);
    printf("  Language: %s\n", lang == OMP_LANG_C ? "C" : "Fortran");
    printf("  Clauses: %zu\n", clause_count);
    
    // Print clause types
    if (clause_count > 0) {
        printf("  Clause types:\n");
        for (uintptr_t i = 0; i < clause_count; i++) {
            Handle clause;
            ClauseType type;
            
            if (omp_clause_at(directive, i, &clause) == OMP_SUCCESS) {
                if (omp_clause_type(clause, &type) == OMP_SUCCESS) {
                    printf("    [%zu] %s\n", i, clause_type_to_string(type));
                }
                // Don't free clause - it's managed by the directive
            }
        }
    }
}

int main() {
    printf("=== Roup OpenMP Parser - Basic Example ===\n\n");
    
    // Example 1: Simple parallel directive
    printf("Example 1: Simple parallel directive\n");
    printf("Input: \"#pragma omp parallel\"\n\n");
    
    Handle result1;
    OmpStatus status = omp_parse("#pragma omp parallel", OMP_LANG_C, &result1);
    
    if (status != OMP_SUCCESS) {
        printf("Error: Parse failed with status %d\n", status);
        return 1;
    }
    
    Handle *directives1;
    uintptr_t count1;
    status = omp_take_last_parse_result(&directives1, &count1);
    
    if (status != OMP_SUCCESS) {
        printf("Error: Failed to get parse result\n");
        omp_parse_result_free(result1);
        return 1;
    }
    
    printf("Parsed %zu directive(s)\n\n", count1);
    for (uintptr_t i = 0; i < count1; i++) {
        print_directive(directives1[i]);
    }
    
    free(directives1);
    omp_parse_result_free(result1);
    
    // Example 2: Parallel directive with clauses
    printf("\n----------------------------------------\n");
    printf("Example 2: Parallel directive with clauses\n");
    printf("Input: \"#pragma omp parallel num_threads(4) private(x, y) shared(z)\"\n\n");
    
    Handle result2;
    status = omp_parse("#pragma omp parallel num_threads(4) private(x, y) shared(z)", 
                       OMP_LANG_C, &result2);
    
    if (status != OMP_SUCCESS) {
        printf("Error: Parse failed\n");
        return 1;
    }
    
    Handle *directives2;
    uintptr_t count2;
    omp_take_last_parse_result(&directives2, &count2);
    
    printf("Parsed %zu directive(s)\n\n", count2);
    for (uintptr_t i = 0; i < count2; i++) {
        print_directive(directives2[i]);
        
        // Demonstrate cursor iteration
        printf("\n  Iterating clauses with cursor:\n");
        Handle cursor;
        if (omp_directive_clauses_cursor(directives2[i], &cursor) == OMP_SUCCESS) {
            uintptr_t total;
            omp_cursor_total(cursor, &total);
            printf("  Total clauses in cursor: %zu\n", total);
            
            bool is_done;
            uintptr_t pos = 0;
            while (omp_cursor_is_done(cursor, &is_done) == OMP_SUCCESS && !is_done) {
                Handle clause;
                if (omp_cursor_current(cursor, &clause) == OMP_SUCCESS && 
                    OMP_IS_VALID(clause)) {
                    ClauseType type;
                    omp_clause_type(clause, &type);
                    printf("    Position %zu: %s\n", pos, clause_type_to_string(type));
                }
                omp_cursor_next(cursor);
                pos++;
            }
            omp_cursor_free(cursor);
        }
    }
    
    free(directives2);
    omp_parse_result_free(result2);
    
    // Example 3: Parallel for with schedule
    printf("\n----------------------------------------\n");
    printf("Example 3: Parallel for with schedule\n");
    printf("Input: \"#pragma omp parallel for schedule(static, 16)\"\n\n");
    
    Handle result3;
    status = omp_parse("#pragma omp parallel for schedule(static, 16)", 
                       OMP_LANG_C, &result3);
    
    if (status == OMP_SUCCESS) {
        Handle *directives3;
        uintptr_t count3;
        omp_take_last_parse_result(&directives3, &count3);
        
        printf("Parsed %zu directive(s)\n\n", count3);
        for (uintptr_t i = 0; i < count3; i++) {
            print_directive(directives3[i]);
        }
        
        free(directives3);
        omp_parse_result_free(result3);
    }
    
    printf("\n=== All examples completed successfully ===\n");
    return 0;
}
