#include "src/c_api.h"
#include <stdio.h>

int main() {
    OmpDirective* dir = roup_parse("!$omp barrier");
    if (dir) {
        int32_t kind = roup_directive_kind(dir);
        printf("Parsed! Kind: %d\n", kind);
        roup_directive_free(dir);
        return 0;
    } else {
        printf("Failed to parse Fortran directive\n");
        return 1;
    }
}
