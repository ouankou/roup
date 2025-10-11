// Quick test of new C API
#include <stdio.h>
#include <stdint.h>

// Forward declarations
typedef struct OmpDirective OmpDirective;
typedef struct OmpClause OmpClause;
typedef struct OmpClauseIterator OmpClauseIterator;

extern OmpDirective* roup_parse(const char* input);
extern void roup_directive_free(OmpDirective* directive);
extern int32_t roup_directive_kind(const OmpDirective* directive);
extern int32_t roup_directive_clause_count(const OmpDirective* directive);
extern OmpClauseIterator* roup_directive_clauses_iter(const OmpDirective* directive);
extern int32_t roup_clause_iterator_next(OmpClauseIterator* iter, OmpClause** out);
extern void roup_clause_iterator_free(OmpClauseIterator* iter);
extern int32_t roup_clause_kind(const OmpClause* clause);

int main() {
    printf("Testing new minimal unsafe C API...\n\n");

    // Test 1: Parse simple directive
    const char* input = "#pragma omp parallel for num_threads(4)";
    printf("Input: %s\n", input);

    OmpDirective* dir = roup_parse(input);
    if (!dir) {
        fprintf(stderr, "ERROR: Parse failed\n");
        return 1;
    }
    printf("✓ Parse succeeded\n");

    // Test 2: Get directive kind
    int32_t kind = roup_directive_kind(dir);
    printf("✓ Directive kind: %d\n", kind);

    // Test 3: Get clause count
    int32_t count = roup_directive_clause_count(dir);
    printf("✓ Clause count: %d\n", count);

    // Test 4: Iterate clauses
    OmpClauseIterator* iter = roup_directive_clauses_iter(dir);
    if (!iter) {
        fprintf(stderr, "ERROR: Failed to create iterator\n");
        roup_directive_free(dir);
        return 1;
    }

    printf("✓ Clauses:\n");
    OmpClause* clause;
    while (roup_clause_iterator_next(iter, &clause)) {
        int32_t clause_kind = roup_clause_kind(clause);
        printf("  - Clause kind: %d\n", clause_kind);
    }

    // Cleanup
    roup_clause_iterator_free(iter);
    roup_directive_free(dir);

    printf("\n✅ All tests passed!\n");
    return 0;
}
