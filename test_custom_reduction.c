#include <stdio.h>
#include <stdlib.h>

// Forward declarations
struct OmpDirective;
struct OmpClause;
struct OmpClauseIterator;

extern struct OmpDirective* roup_parse(const char* input);
extern void roup_directive_free(struct OmpDirective* directive);
extern int roup_directive_clause_count(const struct OmpDirective* directive);
extern struct OmpClauseIterator* roup_directive_clauses_iter(const struct OmpDirective* directive);
extern int roup_clause_iterator_next(struct OmpClauseIterator* iter, const struct OmpClause** out);
extern void roup_clause_iterator_free(struct OmpClauseIterator* iter);
extern int roup_clause_kind(const struct OmpClause* clause);
extern int roup_clause_reduction_operator(const struct OmpClause* clause);
extern const char* roup_clause_reduction_custom_operator(const struct OmpClause* clause);
extern int roup_clause_items_count(const struct OmpClause* clause);
extern const char* roup_clause_item_to_string(const struct OmpClause* clause, int index);
extern void roup_string_free(char* s);

int main() {
    const char* input = "#pragma omp parallel reduction(abc:x)";
    printf("Testing: %s\n", input);

    struct OmpDirective* dir = roup_parse(input);
    if (!dir) {
        printf("ERROR: Failed to parse\n");
        return 1;
    }

    int clause_count = roup_directive_clause_count(dir);
    printf("Clause count: %d\n", clause_count);

    struct OmpClauseIterator* iter = roup_directive_clauses_iter(dir);
    const struct OmpClause* clause;
    while (roup_clause_iterator_next(iter, &clause) == 1) {
        int kind = roup_clause_kind(clause);
        printf("Clause kind: %d\n", kind);

        int op = roup_clause_reduction_operator(clause);
        printf("Reduction operator: %d\n", op);

        const char* custom_op = roup_clause_reduction_custom_operator(clause);
        if (custom_op) {
            printf("Custom operator: %s\n", custom_op);
        } else {
            printf("Custom operator: NULL\n");
        }

        int items = roup_clause_items_count(clause);
        printf("Items count: %d\n", items);
        for (int i = 0; i < items; i++) {
            const char* item = roup_clause_item_to_string(clause, i);
            if (item) {
                printf("  Item %d: %s\n", i, item);
                roup_string_free((char*)item);
            }
        }
    }
    roup_clause_iterator_free(iter);
    roup_directive_free(dir);

    return 0;
}
