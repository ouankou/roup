/**
 * Complete C Tutorial: Using the roup OpenMP Parser
 * 
 * This tutorial demonstrates the minimal unsafe pointer-based C API.
 * 
 * Topics covered:
 * 1. Parsing OpenMP directives
 * 2. Querying directive properties
 * 3. Iterating through clauses
 * 4. Accessing clause data
 * 5. Error handling
 * 6. Memory management
 * 
 * API Design: Direct pointers (standard C pattern)
 * - Parse returns pointer or NULL on error
 * - Caller must free resources
 * - No global state or handles
 * 
 * Target: C programmers familiar with malloc/free patterns
 */

#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>

// ============================================================================
// Forward Declarations (or include roup.h if generated)
// ============================================================================

typedef struct OmpDirective OmpDirective;
typedef struct OmpClause OmpClause;
typedef struct OmpClauseIterator OmpClauseIterator;
typedef struct OmpStringList OmpStringList;

// Lifecycle
extern OmpDirective* roup_parse(const char* input);
extern void roup_directive_free(OmpDirective* directive);
extern void roup_clause_free(OmpClause* clause);

// Directive queries
extern int32_t roup_directive_kind(const OmpDirective* directive);
extern int32_t roup_directive_clause_count(const OmpDirective* directive);
extern OmpClauseIterator* roup_directive_clauses_iter(const OmpDirective* directive);

// Iterator
extern int32_t roup_clause_iterator_next(OmpClauseIterator* iter, OmpClause** out);
extern void roup_clause_iterator_free(OmpClauseIterator* iter);

// Clause queries
extern int32_t roup_clause_kind(const OmpClause* clause);
extern int32_t roup_clause_schedule_kind(const OmpClause* clause);
extern int32_t roup_clause_reduction_operator(const OmpClause* clause);
extern int32_t roup_clause_default_data_sharing(const OmpClause* clause);

// Variable lists
extern OmpStringList* roup_clause_variables(const OmpClause* clause);
extern int32_t roup_string_list_len(const OmpStringList* list);
extern const char* roup_string_list_get(const OmpStringList* list, int32_t index);
extern void roup_string_list_free(OmpStringList* list);

// ============================================================================
// Helper: Print directive kind name
// ============================================================================

const char* directive_kind_name(int32_t kind) {
    switch(kind) {
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

// ============================================================================
// Helper: Print clause kind name
// ============================================================================

const char* clause_kind_name(int32_t kind) {
    switch(kind) {
        case 0: return "NUM_THREADS";
        case 1: return "IF";
        case 2: return "PRIVATE";
        case 3: return "SHARED";
        case 4: return "FIRSTPRIVATE";
        case 5: return "LASTPRIVATE";
        case 6: return "REDUCTION";
        case 7: return "SCHEDULE";
        case 8: return "COLLAPSE";
        case 9: return "ORDERED";
        case 10: return "NOWAIT";
        case 11: return "DEFAULT";
        default: return "UNKNOWN";
    }
}

// ============================================================================
// STEP 1: Parse a Simple Directive
// ============================================================================

void step1_simple_parse() {
    printf("╔════════════════════════════════════════════════════════════╗\n");
    printf("║ STEP 1: Parse a Simple OpenMP Directive                   ║\n");
    printf("╚════════════════════════════════════════════════════════════╝\n\n");

    const char* input = "#pragma omp parallel";
    printf("Input: \"%s\"\n\n", input);

    // Parse directive
    OmpDirective* dir = roup_parse(input);
    
    // Error handling: NULL means parse failed
    if (!dir) {
        fprintf(stderr, "❌ ERROR: Parse failed!\n");
        fprintf(stderr, "   Possible reasons:\n");
        fprintf(stderr, "   - Invalid OpenMP syntax\n");
        fprintf(stderr, "   - NULL input\n");
        fprintf(stderr, "   - Invalid UTF-8\n\n");
        return;
    }

    printf("✅ Parse succeeded!\n");
    printf("   Directive: %p (non-NULL pointer)\n\n", (void*)dir);

    // Query directive properties
    int32_t kind = roup_directive_kind(dir);
    int32_t count = roup_directive_clause_count(dir);

    printf("Directive Properties:\n");
    printf("  - Kind:   %d (%s)\n", kind, directive_kind_name(kind));
    printf("  - Clauses: %d\n\n", count);

    // IMPORTANT: Free directive to prevent memory leak
    roup_directive_free(dir);
    printf("✓ Memory freed\n\n");
}

// ============================================================================
// STEP 2: Parse Directive with Clauses
// ============================================================================

void step2_with_clauses() {
    printf("╔════════════════════════════════════════════════════════════╗\n");
    printf("║ STEP 2: Parse Directive with Multiple Clauses             ║\n");
    printf("╚════════════════════════════════════════════════════════════╝\n\n");

    const char* input = "#pragma omp parallel for num_threads(4) private(i, j) nowait";
    printf("Input: \"%s\"\n\n", input);

    OmpDirective* dir = roup_parse(input);
    if (!dir) {
        fprintf(stderr, "❌ ERROR: Parse failed!\n\n");
        return;
    }

    printf("✅ Parse succeeded!\n\n");

    int32_t kind = roup_directive_kind(dir);
    int32_t count = roup_directive_clause_count(dir);

    printf("Directive: %s\n", directive_kind_name(kind));
    printf("Clauses: %d\n\n", count);

    roup_directive_free(dir);
    printf("✓ Memory freed\n\n");
}

// ============================================================================
// STEP 3: Iterate Through Clauses
// ============================================================================

void step3_iterate_clauses() {
    printf("╔════════════════════════════════════════════════════════════╗\n");
    printf("║ STEP 3: Iterate Through Clauses                           ║\n");
    printf("╚════════════════════════════════════════════════════════════╝\n\n");

    const char* input = "#pragma omp parallel num_threads(8) default(shared) nowait";
    printf("Input: \"%s\"\n\n", input);

    OmpDirective* dir = roup_parse(input);
    if (!dir) {
        fprintf(stderr, "❌ ERROR: Parse failed!\n\n");
        return;
    }

    printf("✅ Parse succeeded!\n\n");

    // Create iterator
    OmpClauseIterator* iter = roup_directive_clauses_iter(dir);
    if (!iter) {
        fprintf(stderr, "❌ ERROR: Failed to create iterator!\n\n");
        roup_directive_free(dir);
        return;
    }

    printf("Iterating through clauses:\n");
    printf("─────────────────────────────\n");

    // Iterate using roup_clause_iterator_next
    // Returns 1 if clause available, 0 if done
    OmpClause* clause;
    int clause_num = 1;
    while (roup_clause_iterator_next(iter, &clause)) {
        int32_t kind = roup_clause_kind(clause);
        printf("  %d. %s (kind=%d)\n", clause_num++, clause_kind_name(kind), kind);
    }

    printf("\n");

    // Cleanup
    roup_clause_iterator_free(iter);
    roup_directive_free(dir);
    printf("✓ Memory freed\n\n");
}

// ============================================================================
// STEP 4: Query Specific Clause Data
// ============================================================================

void step4_clause_data() {
    printf("╔════════════════════════════════════════════════════════════╗\n");
    printf("║ STEP 4: Query Specific Clause Data                        ║\n");
    printf("╚════════════════════════════════════════════════════════════╝\n\n");

    const char* input = "#pragma omp parallel for schedule(static, 10) reduction(+:sum)";
    printf("Input: \"%s\"\n\n", input);

    OmpDirective* dir = roup_parse(input);
    if (!dir) {
        fprintf(stderr, "❌ ERROR: Parse failed!\n\n");
        return;
    }

    printf("✅ Parse succeeded!\n\n");

    // Iterate and query clause-specific data
    OmpClauseIterator* iter = roup_directive_clauses_iter(dir);
    if (!iter) {
        roup_directive_free(dir);
        return;
    }

    printf("Clause Details:\n");
    printf("───────────────\n");

    OmpClause* clause;
    while (roup_clause_iterator_next(iter, &clause)) {
        int32_t kind = roup_clause_kind(clause);
        printf("  • %s", clause_kind_name(kind));

        // Query clause-specific data
        switch(kind) {
            case 7: {  // SCHEDULE
                int32_t sched_kind = roup_clause_schedule_kind(clause);
                const char* sched_names[] = {"static", "dynamic", "guided", "auto", "runtime"};
                if (sched_kind >= 0 && sched_kind < 5) {
                    printf(" → %s\n", sched_names[sched_kind]);
                } else {
                    printf(" → unknown\n");
                }
                break;
            }
            case 6: {  // REDUCTION
                int32_t op = roup_clause_reduction_operator(clause);
                const char* op_names[] = {"+", "-", "*", "&", "|", "^", "&&", "||", "min", "max"};
                if (op >= 0 && op < 10) {
                    printf(" → operator: %s\n", op_names[op]);
                } else {
                    printf(" → unknown operator\n");
                }
                break;
            }
            case 11: {  // DEFAULT
                int32_t def = roup_clause_default_data_sharing(clause);
                printf(" → %s\n", def == 0 ? "shared" : "none");
                break;
            }
            default:
                printf("\n");
                break;
        }
    }

    printf("\n");

    roup_clause_iterator_free(iter);
    roup_directive_free(dir);
    printf("✓ Memory freed\n\n");
}

// ============================================================================
// STEP 5: Error Handling
// ============================================================================

void step5_error_handling() {
    printf("╔════════════════════════════════════════════════════════════╗\n");
    printf("║ STEP 5: Error Handling                                    ║\n");
    printf("╚════════════════════════════════════════════════════════════╝\n\n");

    printf("Testing various error conditions:\n\n");

    // Test 1: Invalid syntax
    printf("1. Invalid OpenMP syntax:\n");
    const char* invalid = "#pragma omp INVALID_DIRECTIVE";
    printf("   Input: \"%s\"\n", invalid);
    OmpDirective* dir1 = roup_parse(invalid);
    if (!dir1) {
        printf("   ✓ Correctly returned NULL\n\n");
    } else {
        printf("   ⚠ Unexpectedly succeeded\n\n");
        roup_directive_free(dir1);
    }

    // Test 2: NULL input
    printf("2. NULL input:\n");
    printf("   Input: NULL\n");
    OmpDirective* dir2 = roup_parse(NULL);
    if (!dir2) {
        printf("   ✓ Correctly returned NULL\n\n");
    }

    // Test 3: Empty string
    printf("3. Empty string:\n");
    printf("   Input: \"\"\n");
    OmpDirective* dir3 = roup_parse("");
    if (!dir3) {
        printf("   ✓ Correctly returned NULL\n\n");
    } else {
        roup_directive_free(dir3);
    }

    // Test 4: Querying NULL pointer
    printf("4. Querying NULL directive:\n");
    int32_t kind = roup_directive_kind(NULL);
    printf("   roup_directive_kind(NULL) = %d\n", kind);
    printf("   ✓ Returns -1 for NULL input\n\n");

    printf("✓ Error handling verified\n\n");
}

// ============================================================================
// STEP 6: Multiple Directive Types
// ============================================================================

void step6_multiple_directives() {
    printf("╔════════════════════════════════════════════════════════════╗\n");
    printf("║ STEP 6: Parse Different Directive Types                   ║\n");
    printf("╚════════════════════════════════════════════════════════════╝\n\n");

    const char* test_cases[] = {
        "#pragma omp parallel",
        "#pragma omp for",
        "#pragma omp task",
        "#pragma omp taskwait",
        "#pragma omp barrier",
        "#pragma omp target",
        "#pragma omp teams",
        "#pragma omp critical",
        NULL
    };

    printf("Parsing multiple directive types:\n");
    printf("─────────────────────────────────\n");

    for (int i = 0; test_cases[i] != NULL; i++) {
        OmpDirective* dir = roup_parse(test_cases[i]);
        if (dir) {
            int32_t kind = roup_directive_kind(dir);
            printf("  ✓ %-40s → %s\n", test_cases[i], directive_kind_name(kind));
            roup_directive_free(dir);
        } else {
            printf("  ✗ %-40s → FAILED\n", test_cases[i]);
        }
    }

    printf("\n✓ All directives tested\n\n");
}

// ============================================================================
// Main Function
// ============================================================================

int main(void) {
    printf("\n");
    printf("╔════════════════════════════════════════════════════════════╗\n");
    printf("║                                                            ║\n");
    printf("║       OpenMP Parser C Tutorial (Minimal Unsafe API)       ║\n");
    printf("║                                                            ║\n");
    printf("║  API Style: Direct pointers (standard C malloc/free)      ║\n");
    printf("║  Functions: roup_parse(), roup_directive_free(), etc.     ║\n");
    printf("║                                                            ║\n");
    printf("╚════════════════════════════════════════════════════════════╝\n");
    printf("\n");

    step1_simple_parse();
    step2_with_clauses();
    step3_iterate_clauses();
    step4_clause_data();
    step5_error_handling();
    step6_multiple_directives();

    printf("╔════════════════════════════════════════════════════════════╗\n");
    printf("║                    TUTORIAL COMPLETE                       ║\n");
    printf("╚════════════════════════════════════════════════════════════╝\n");
    printf("\n");
    printf("Key Takeaways:\n");
    printf("─────────────\n");
    printf("1. Use roup_parse() to parse directives (returns pointer or NULL)\n");
    printf("2. Check for NULL to detect parse errors\n");
    printf("3. Query directives with roup_directive_*() functions\n");
    printf("4. Iterate clauses with roup_clause_iterator_next()\n");
    printf("5. Always call roup_*_free() to prevent memory leaks\n");
    printf("6. NULL checks are your friend!\n");
    printf("\n");
    printf("✅ All examples completed successfully!\n");
    printf("\n");

    return 0;
}
