/**
 * @file basic_parse.c
 * @brief Basic example of parsing OpenMP directives and querying them
 * 
 * This example demonstrates:
 * - Parsing OpenMP directives from a string using roup_parse()
 * - Querying directive properties (kind, clause count)
 * - Iterating through clauses with iterators
 * - Proper resource cleanup
 * 
 * Build:
 *   clang -o basic_parse basic_parse.c -L../../target/debug -lroup -lpthread -ldl -lm
 * 
 * Run:
 *   LD_LIBRARY_PATH=../../target/debug ./basic_parse
 */

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>

// Forward declarations for roup C API
typedef struct OmpDirective OmpDirective;
typedef struct OmpClause OmpClause;
typedef struct OmpClauseIterator OmpClauseIterator;

// Lifecycle functions
extern OmpDirective* roup_parse(const char* input);
extern void roup_directive_free(OmpDirective* directive);

// Directive queries
extern int32_t roup_directive_kind(const OmpDirective* directive);
extern int32_t roup_directive_clause_count(const OmpDirective* directive);
extern OmpClauseIterator* roup_directive_clauses_iter(const OmpDirective* directive);

// Iterator functions
extern int32_t roup_clause_iterator_next(OmpClauseIterator* iter, OmpClause** out);
extern void roup_clause_iterator_free(OmpClauseIterator* iter);

// Clause queries
extern int32_t roup_clause_kind(const OmpClause* clause);


/**
 * @brief Get directive kind as string
 */
const char* directive_kind_to_string(int32_t kind) {
    switch (kind) {
        case 0: return "PARALLEL";
        case 1: return "FOR";
        case 2: return "SECTIONS";
        case 3: return "SINGLE";
        case 4: return "TASK";
        case 5: return "MASTER";
        case 6: return "CRITICAL";
        case 7: return "BARRIER";
        case 8: return "TASKWAIT";
        case 9: return "TASKGROUP";
        case 10: return "ATOMIC";
        case 11: return "FLUSH";
        case 12: return "ORDERED";
        case 13: return "TARGET";
        case 14: return "TEAMS";
        case 15: return "DISTRIBUTE";
        case 16: return "METADIRECTIVE";
        default: return "UNKNOWN";
    }
}

/**
 * @brief Print directive information
 */
void print_directive(OmpDirective* directive) {
    if (!directive) {
        printf("  Error: NULL directive\n");
        return;
    }
    
    // Query directive properties
    int32_t kind = roup_directive_kind(directive);
    int32_t clause_count = roup_directive_clause_count(directive);
    
    printf("Directive: %s\n", directive_kind_to_string(kind));
    printf("  Clauses: %d\n", clause_count);
    
    // Iterate through clauses
    if (clause_count > 0) {
        printf("  Clause details:\n");
        OmpClauseIterator* iter = roup_directive_clauses_iter(directive);
        if (iter) {
            OmpClause* clause = NULL;
            int32_t pos = 0;
            
            // Note: clauses are owned by directive, don't free them individually
            while (roup_clause_iterator_next(iter, &clause) == 1) {
                int32_t clause_kind = roup_clause_kind(clause);
                printf("    [%d] Clause kind: %d\n", pos, clause_kind);
                pos++;
            }
            
            roup_clause_iterator_free(iter);
        }
    }
}


int main() {
    printf("=== Roup OpenMP Parser - Basic Example ===\n\n");
    
    // Example 1: Simple parallel directive
    printf("Example 1: Simple parallel directive\n");
    printf("Input: \"#pragma omp parallel\"\n\n");
    
    OmpDirective* directive1 = roup_parse("#pragma omp parallel");
    
    if (!directive1) {
        printf("Error: Parse failed\n");
        return 1;
    }
    
    print_directive(directive1);
    roup_directive_free(directive1);
    
    // Example 2: Parallel directive with clauses
    printf("\n----------------------------------------\n");
    printf("Example 2: Parallel directive with clauses\n");
    printf("Input: \"#pragma omp parallel num_threads(4) private(x, y) shared(z)\"\n\n");
    
    OmpDirective* directive2 = roup_parse("#pragma omp parallel num_threads(4) private(x, y) shared(z)");
    
    if (!directive2) {
        printf("Error: Parse failed\n");
        return 1;
    }
    
    print_directive(directive2);
    roup_directive_free(directive2);
    
    // Example 3: For directive with schedule
    printf("\n----------------------------------------\n");
    printf("Example 3: For directive with schedule\n");
    printf("Input: \"#pragma omp for schedule(static, 16)\"\n\n");
    
    OmpDirective* directive3 = roup_parse("#pragma omp for schedule(static, 16)");
    
    if (directive3) {
        print_directive(directive3);
        roup_directive_free(directive3);
    } else {
        printf("Error: Parse failed\n");
    }
    
    printf("\n=== All examples completed successfully ===\n");
    return 0;
}

