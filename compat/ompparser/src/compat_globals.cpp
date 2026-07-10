/* Compatibility shim: provide symbols expected by ompparser headers
 * without invoking the original bison/flex-generated parser.
 *
 * We intentionally provide a minimal definition for the global
 * `normalize_clauses_global` and its setter so the compatibility
 * build can link successfully while keeping the project free of
 * additional bison/flex steps.
 */

#include "OpenMPIR.h"

// Define the global used by the ompparser sources
bool normalize_clauses_global = true;
// Separator flag set by the original parser when clauses are comma-delimited.
// The compatibility build does not run the parser, so default to false.
bool clause_separator_comma = false;

// Provide the setter (matches signature declared in OpenMPIR.h)
void setNormalizeClauses(bool normalize) {
    normalize_clauses_global = normalize;
}

// The compatibility build bypasses ompparser's lexer/parser, so there is no
// live upstream token stream to query. The adapter never infers locations by
// rescanning source text; these stubs only satisfy SourceLocation's default
// constructor until an explicit typed source-range query is available.
int openmpGetCurrentTokenLine() {
    return 0;
}

int openmpGetCurrentTokenColumn() {
    return 0;
}
