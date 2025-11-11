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

// Provide the setter (matches signature declared in OpenMPIR.h)
void setNormalizeClauses(bool normalize) { normalize_clauses_global = normalize; }
