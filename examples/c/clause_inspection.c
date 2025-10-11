/**
 * @file clause_inspection.c
 * @brief Demonstrates detailed clause inspection with typed accessors
 * 
 * This example shows how to:
 * - Query clause types
 * - Use typed accessors (num_threads, schedule, reduction, default)
 * - Handle list clauses (private, shared, etc.)
 * - Extract string values from clauses
 * - Proper error handling
 * 
 * Build:
 *   gcc -o clause_inspection clause_inspection.c -L../../target/debug -lroup -lpthread -ldl -lm
 * 
 * Run:
 *   LD_LIBRARY_PATH=../../target/debug ./clause_inspection
 */

#include "../../include/roup.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/**
 * @brief Helper: Copy string handle to C buffer and print
 */
void print_string_handle(const char* prefix, Handle str_handle) {
    if (OMP_IS_INVALID(str_handle)) {
        printf("%s(none)\n", prefix);
        return;
    }
    
    uintptr_t len;
    if (omp_str_len(str_handle, &len) != OMP_SUCCESS) {
        printf("%s(error getting length)\n", prefix);
        return;
    }
    
    char *buffer = malloc(len + 1);
    if (!buffer) {
        printf("%s(malloc failed)\n", prefix);
        return;
    }
    
    uintptr_t written;
    if (omp_str_copy_to_buffer(str_handle, buffer, len + 1, &written) == OMP_SUCCESS) {
        printf("%s%s\n", prefix, buffer);
    } else {
        printf("%s(error copying)\n", prefix);
    }
    
    free(buffer);
    omp_str_free(str_handle);
}

/**
 * @brief Inspect a num_threads clause
 */
void inspect_num_threads_clause(Handle clause) {
    Handle value;
    if (omp_clause_num_threads_value(clause, &value) == OMP_SUCCESS) {
        print_string_handle("    Value: ", value);
    }
}

/**
 * @brief Inspect a default clause
 */
void inspect_default_clause(Handle clause) {
    DefaultKind kind;
    if (omp_clause_default_kind(clause, &kind) == OMP_SUCCESS) {
        const char *kind_str;
        switch (kind) {
            case OMP_DEFAULT_SHARED: kind_str = "shared"; break;
            case OMP_DEFAULT_NONE: kind_str = "none"; break;
            case OMP_DEFAULT_PRIVATE: kind_str = "private"; break;
            case OMP_DEFAULT_FIRSTPRIVATE: kind_str = "firstprivate"; break;
            default: kind_str = "unknown"; break;
        }
        printf("    Kind: %s\n", kind_str);
    }
}

/**
 * @brief Inspect a schedule clause
 */
void inspect_schedule_clause(Handle clause) {
    ScheduleKind kind;
    if (omp_clause_schedule_kind(clause, &kind) == OMP_SUCCESS) {
        const char *kind_str;
        switch (kind) {
            case OMP_SCHEDULE_STATIC: kind_str = "static"; break;
            case OMP_SCHEDULE_DYNAMIC: kind_str = "dynamic"; break;
            case OMP_SCHEDULE_GUIDED: kind_str = "guided"; break;
            case OMP_SCHEDULE_AUTO: kind_str = "auto"; break;
            case OMP_SCHEDULE_RUNTIME: kind_str = "runtime"; break;
            default: kind_str = "unknown"; break;
        }
        printf("    Kind: %s\n", kind_str);
        
        Handle chunk_size;
        if (omp_clause_schedule_chunk_size(clause, &chunk_size) == OMP_SUCCESS) {
            print_string_handle("    Chunk size: ", chunk_size);
        }
    }
}

/**
 * @brief Inspect a reduction clause
 */
void inspect_reduction_clause(Handle clause) {
    ReductionOperator op;
    if (omp_clause_reduction_operator(clause, &op) == OMP_SUCCESS) {
        const char *op_str;
        switch (op) {
            case OMP_REDUCTION_ADD: op_str = "+"; break;
            case OMP_REDUCTION_MULTIPLY: op_str = "*"; break;
            case OMP_REDUCTION_SUBTRACT: op_str = "-"; break;
            case OMP_REDUCTION_AND: op_str = "&"; break;
            case OMP_REDUCTION_OR: op_str = "|"; break;
            case OMP_REDUCTION_XOR: op_str = "^"; break;
            case OMP_REDUCTION_LAND: op_str = "&&"; break;
            case OMP_REDUCTION_LOR: op_str = "||"; break;
            case OMP_REDUCTION_MIN: op_str = "min"; break;
            case OMP_REDUCTION_MAX: op_str = "max"; break;
            case OMP_REDUCTION_CUSTOM: op_str = "custom"; break;
            default: op_str = "unknown"; break;
        }
        printf("    Operator: %s\n", op_str);
        
        if (op == OMP_REDUCTION_CUSTOM) {
            Handle identifier;
            if (omp_clause_reduction_identifier(clause, &identifier) == OMP_SUCCESS) {
                print_string_handle("    Identifier: ", identifier);
            }
        }
    }
    
    // Print variables
    uintptr_t count;
    if (omp_clause_item_count(clause, &count) == OMP_SUCCESS && count > 0) {
        printf("    Variables (%zu):\n", count);
        for (uintptr_t i = 0; i < count; i++) {
            Handle item;
            if (omp_clause_item_at(clause, i, &item) == OMP_SUCCESS) {
                char prefix[64];
                snprintf(prefix, sizeof(prefix), "      [%zu] ", i);
                print_string_handle(prefix, item);
            }
        }
    }
}

/**
 * @brief Inspect a list clause (private, shared, etc.)
 */
void inspect_list_clause(Handle clause, const char *name) {
    uintptr_t count;
    if (omp_clause_item_count(clause, &count) == OMP_SUCCESS) {
        printf("    %s variables (%zu):\n", name, count);
        for (uintptr_t i = 0; i < count; i++) {
            Handle item;
            if (omp_clause_item_at(clause, i, &item) == OMP_SUCCESS) {
                char prefix[64];
                snprintf(prefix, sizeof(prefix), "      [%zu] ", i);
                print_string_handle(prefix, item);
            }
        }
    }
}

/**
 * @brief Inspect a clause based on its type
 */
void inspect_clause(Handle clause, ClauseType type) {
    switch (type) {
        case OMP_CLAUSE_NUM_THREADS:
            printf("  Clause: num_threads\n");
            inspect_num_threads_clause(clause);
            break;
        case OMP_CLAUSE_DEFAULT:
            printf("  Clause: default\n");
            inspect_default_clause(clause);
            break;
        case OMP_CLAUSE_SCHEDULE:
            printf("  Clause: schedule\n");
            inspect_schedule_clause(clause);
            break;
        case OMP_CLAUSE_REDUCTION:
            printf("  Clause: reduction\n");
            inspect_reduction_clause(clause);
            break;
        case OMP_CLAUSE_PRIVATE:
            printf("  Clause: private\n");
            inspect_list_clause(clause, "Private");
            break;
        case OMP_CLAUSE_SHARED:
            printf("  Clause: shared\n");
            inspect_list_clause(clause, "Shared");
            break;
        case OMP_CLAUSE_FIRSTPRIVATE:
            printf("  Clause: firstprivate\n");
            inspect_list_clause(clause, "Firstprivate");
            break;
        case OMP_CLAUSE_NOWAIT:
            printf("  Clause: nowait (bare)\n");
            break;
        default:
            printf("  Clause: (other type %d)\n", type);
            break;
    }
}

int main() {
    printf("=== Clause Inspection Example ===\n\n");
    
    // Example 1: num_threads clause
    printf("Example 1: num_threads clause\n");
    printf("Input: \"#pragma omp parallel num_threads(omp_get_max_threads())\" \n\n");
    
    Handle result1;
    if (omp_parse("#pragma omp parallel num_threads(omp_get_max_threads())", 
                  OMP_LANG_C, &result1) == OMP_SUCCESS) {
        Handle *dirs;
        uintptr_t count;
        omp_take_last_parse_result(&dirs, &count);
        
        if (count > 0) {
            Handle clause;
            if (omp_clause_at(dirs[0], 0, &clause) == OMP_SUCCESS) {
                ClauseType type;
                omp_clause_type(clause, &type);
                inspect_clause(clause, type);
            }
        }
        
        free(dirs);
        omp_parse_result_free(result1);
    }
    
    // Example 2: schedule clause
    printf("\n----------------------------------------\n");
    printf("Example 2: schedule clause\n");
    printf("Input: \"#pragma omp for schedule(dynamic, 10)\"\n\n");
    
    Handle result2;
    if (omp_parse("#pragma omp for schedule(dynamic, 10)", 
                  OMP_LANG_C, &result2) == OMP_SUCCESS) {
        Handle *dirs;
        uintptr_t count;
        omp_take_last_parse_result(&dirs, &count);
        
        if (count > 0) {
            Handle clause;
            if (omp_clause_at(dirs[0], 0, &clause) == OMP_SUCCESS) {
                ClauseType type;
                omp_clause_type(clause, &type);
                inspect_clause(clause, type);
            }
        }
        
        free(dirs);
        omp_parse_result_free(result2);
    }
    
    // Example 3: reduction clause
    printf("\n----------------------------------------\n");
    printf("Example 3: reduction clause\n");
    printf("Input: \"#pragma omp parallel for reduction(+: sum, total)\"\n\n");
    
    Handle result3;
    if (omp_parse("#pragma omp parallel for reduction(+: sum, total)", 
                  OMP_LANG_C, &result3) == OMP_SUCCESS) {
        Handle *dirs;
        uintptr_t count;
        omp_take_last_parse_result(&dirs, &count);
        
        if (count > 0) {
            uintptr_t clause_count;
            omp_directive_clause_count(dirs[0], &clause_count);
            printf("Total clauses: %zu\n\n", clause_count);
            
            for (uintptr_t i = 0; i < clause_count; i++) {
                Handle clause;
                if (omp_clause_at(dirs[0], i, &clause) == OMP_SUCCESS) {
                    ClauseType type;
                    omp_clause_type(clause, &type);
                    inspect_clause(clause, type);
                    printf("\n");
                }
            }
        }
        
        free(dirs);
        omp_parse_result_free(result3);
    }
    
    // Example 4: Multiple list clauses
    printf("----------------------------------------\n");
    printf("Example 4: Multiple list clauses\n");
    printf("Input: \"#pragma omp parallel private(i, j, k) shared(array) firstprivate(n)\"\n\n");
    
    Handle result4;
    if (omp_parse("#pragma omp parallel private(i, j, k) shared(array) firstprivate(n)", 
                  OMP_LANG_C, &result4) == OMP_SUCCESS) {
        Handle *dirs;
        uintptr_t count;
        omp_take_last_parse_result(&dirs, &count);
        
        if (count > 0) {
            uintptr_t clause_count;
            omp_directive_clause_count(dirs[0], &clause_count);
            printf("Total clauses: %zu\n\n", clause_count);
            
            for (uintptr_t i = 0; i < clause_count; i++) {
                Handle clause;
                if (omp_clause_at(dirs[0], i, &clause) == OMP_SUCCESS) {
                    ClauseType type;
                    omp_clause_type(clause, &type);
                    inspect_clause(clause, type);
                    printf("\n");
                }
            }
        }
        
        free(dirs);
        omp_parse_result_free(result4);
    }
    
    printf("=== All examples completed successfully ===\n");
    return 0;
}
