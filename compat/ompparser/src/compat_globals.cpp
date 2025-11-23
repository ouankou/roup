/* Compatibility shim: provide symbols expected by ompparser headers
 * without invoking the original bison/flex-generated parser.
 *
 * We intentionally provide a minimal definition for the global
 * `normalize_clauses_global` and its setter so the compatibility
 * build can link successfully while keeping the project free of
 * additional bison/flex steps.
 */

#include "OpenMPIR.h"
#include <cstdlib>

// Define the global used by the ompparser sources
bool normalize_clauses_global = true;
// Separator flag set by the original parser when clauses are comma-delimited.
// The compatibility build does not run the parser, so default to false.
bool clause_separator_comma = false;

// Provide the setter (matches signature declared in OpenMPIR.h)
void setNormalizeClauses(bool normalize) {
    normalize_clauses_global = normalize;
    // Propagate to the Rust parser via environment variable understood by the C API.
    const char* value = normalize ? "parser_parity" : "disabled";
    setenv("ROUP_NORMALIZE_CLAUSES", value, /*overwrite=*/1);
}
