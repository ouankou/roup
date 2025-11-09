/*
 * OpenMPIR.cpp - Minimal ompparser compatibility implementation using ROUP
 *
 * This provides ONLY the implementation (.cpp), using ompparser's headers
 * from the git submodule at compat/ompparser/ompparser/src/
 *
 * Copyright (c) 2025 ROUP Project
 * SPDX-License-Identifier: BSD-3-Clause
 */

#include <OpenMPIR.h>
#include "roup_compat.h"
#include <cstring>
#include <map>
#include <sstream>
#include <string>
#include <vector>

// ============================================================================
// Global State
// ============================================================================

static OpenMPBaseLang current_lang = Lang_C;

// Language prefix constants - defined once to avoid manual synchronization
static constexpr const char FORTRAN_PREFIX[] = "!$omp";       // Fortran prefix (lowercase)
static constexpr const char FORTRAN_PREFIX_UPPER[] = "!$OMP"; // Fortran prefix (uppercase)
static constexpr const char C_PRAGMA_PREFIX[] = "#pragma";    // C/C++ pragma prefix
// Compile-time string lengths: sizeof() includes null terminator, subtract 1 for actual length
static constexpr size_t FORTRAN_PREFIX_LEN = sizeof(FORTRAN_PREFIX) - 1;
static constexpr size_t C_PRAGMA_PREFIX_LEN = sizeof(C_PRAGMA_PREFIX) - 1;

void setLang(OpenMPBaseLang lang) {
    current_lang = lang;
}

// ============================================================================
// Helper Functions
// ============================================================================

static OpenMPDirectiveKind mapRoupToOmpparserDirective(int32_t roup_kind) {
    // ROUP directive kind mapping to ompparser OpenMPDirectiveKind enum
    // Based on enum order in compat/ompparser/ompparser/src/OpenMPKinds.h
    // Each ROUP kind (0-86) maps directly to corresponding OMPD_* enum value
    switch (roup_kind) {
        case 0:  return OMPD_parallel;
        case 1:  return OMPD_for;
        case 2:  return OMPD_do;
        case 3:  return OMPD_simd;
        case 4:  return OMPD_for_simd;
        case 5:  return OMPD_do_simd;
        case 6:  return OMPD_parallel_for_simd;
        case 7:  return OMPD_parallel_do_simd;
        case 8:  return OMPD_declare_simd;
        case 9:  return OMPD_distribute;
        case 10: return OMPD_distribute_simd;
        case 11: return OMPD_distribute_parallel_for;
        case 12: return OMPD_distribute_parallel_do;
        case 13: return OMPD_distribute_parallel_for_simd;
        case 14: return OMPD_distribute_parallel_do_simd;
        case 15: return OMPD_loop;
        case 16: return OMPD_scan;
        case 17: return OMPD_sections;
        case 18: return OMPD_section;
        case 19: return OMPD_single;
        case 20: return OMPD_workshare;
        case 21: return OMPD_cancel;
        case 22: return OMPD_cancellation_point;
        case 23: return OMPD_allocate;
        case 24: return OMPD_threadprivate;
        case 25: return OMPD_declare_reduction;
        case 26: return OMPD_declare_mapper;
        case 27: return OMPD_parallel_for;
        case 28: return OMPD_parallel_do;
        case 29: return OMPD_parallel_loop;
        case 30: return OMPD_parallel_sections;
        case 31: return OMPD_parallel_workshare;
        case 32: return OMPD_parallel_master;
        case 33: return OMPD_master_taskloop;
        case 34: return OMPD_master_taskloop_simd;
        case 35: return OMPD_parallel_master_taskloop;
        case 36: return OMPD_parallel_master_taskloop_simd;
        case 37: return OMPD_teams;
        case 38: return OMPD_metadirective;
        case 39: return OMPD_declare_variant;
        case 40: return OMPD_task;
        case 41: return OMPD_taskloop;
        case 42: return OMPD_taskloop_simd;
        case 43: return OMPD_taskyield;
        case 44: return OMPD_requires;
        case 45: return OMPD_target_data;
        case 46: return OMPD_target_enter_data;
        case 47: return OMPD_target_update;
        case 48: return OMPD_target_exit_data;
        case 49: return OMPD_target;
        case 50: return OMPD_declare_target;
        case 51: return OMPD_end_declare_target;
        case 52: return OMPD_master;
        case 53: return OMPD_end;
        case 54: return OMPD_barrier;
        case 55: return OMPD_taskwait;
        case 56: return OMPD_unroll;
        case 57: return OMPD_tile;
        case 58: return OMPD_taskgroup;
        case 59: return OMPD_flush;
        case 60: return OMPD_atomic;
        case 61: return OMPD_critical;
        case 62: return OMPD_depobj;
        case 63: return OMPD_ordered;
        case 64: return OMPD_teams_distribute;
        case 65: return OMPD_teams_distribute_simd;
        case 66: return OMPD_teams_distribute_parallel_for;
        case 67: return OMPD_teams_distribute_parallel_for_simd;
        case 68: return OMPD_teams_loop;
        case 69: return OMPD_target_parallel;
        case 70: return OMPD_target_parallel_for;
        case 71: return OMPD_target_parallel_for_simd;
        case 72: return OMPD_target_parallel_loop;
        case 73: return OMPD_target_simd;
        case 74: return OMPD_target_teams;
        case 75: return OMPD_target_teams_distribute;
        case 76: return OMPD_target_teams_distribute_simd;
        case 77: return OMPD_target_teams_loop;
        case 78: return OMPD_target_teams_distribute_parallel_for;
        case 79: return OMPD_target_teams_distribute_parallel_for_simd;
        case 80: return OMPD_teams_distribute_parallel_do;
        case 81: return OMPD_teams_distribute_parallel_do_simd;
        case 82: return OMPD_target_parallel_do;
        case 83: return OMPD_target_parallel_do_simd;
        case 84: return OMPD_target_teams_distribute_parallel_do;
        case 85: return OMPD_target_teams_distribute_parallel_do_simd;
        case 86: return OMPD_unknown;
        default: return OMPD_unknown;
    }
}

static OpenMPClauseKind mapRoupToOmpparserClause(int32_t roup_kind) {
    // ROUP clause kinds (0-89) map directly to ompparser OpenMPClauseKind enum
    // Both follow the same order from OpenMPKinds.h
    // See src/c_api.rs:convert_clause() for the mapping
    if (roup_kind >= 0 && roup_kind <= 89) {
        return static_cast<OpenMPClauseKind>(roup_kind);
    }
    return OMPC_unknown;
}

// ============================================================================
// Main Entry Point
// ============================================================================

extern "C" {

OpenMPDirective* parseOpenMP(const char* input, void* exprParse(const char* expr)) {
    if (!input || input[0] == '\0') {
        return nullptr;
    }

    // Validate input length using constant from ROUP C API
    // Use strnlen to safely handle potentially untrusted/non-null-terminated input
    const size_t input_len = strnlen(input, ROUP_MAX_PRAGMA_LENGTH);
    if (input_len == ROUP_MAX_PRAGMA_LENGTH) {
        return nullptr;  // Input too long or not null-terminated within limit
    }

    // Determine input format based on current language mode
    std::string input_str(input, input_len);

    // Map ompparser language to ROUP language constant
    int32_t roup_lang = ROUP_LANG_C;  // Default to C

    // Handle different language pragmas
    if (current_lang == Lang_Fortran) {
        // Fortran uses !$omp prefix - add if missing (case-insensitive check)
        const bool has_prefix =
            input_str.length() >= FORTRAN_PREFIX_LEN &&
            (input_str.compare(0, FORTRAN_PREFIX_LEN, FORTRAN_PREFIX) == 0 ||
             input_str.compare(0, FORTRAN_PREFIX_LEN, FORTRAN_PREFIX_UPPER) == 0);
        if (!has_prefix) {
            input_str = std::string(FORTRAN_PREFIX) + " " + input_str;
        }
        // Use Fortran free-form (ompparser doesn't distinguish free/fixed at this level)
        roup_lang = ROUP_LANG_FORTRAN_FREE;
    } else {
        // C/C++ use #pragma omp prefix - add if missing
        // compare() returns 0 if strings match, non-zero otherwise
        if (input_str.compare(0, C_PRAGMA_PREFIX_LEN, C_PRAGMA_PREFIX) != 0) {
            input_str = std::string(C_PRAGMA_PREFIX) + " " + input_str;
        }
        roup_lang = ROUP_LANG_C;
    }

    // Call ROUP parser with language information
    OmpDirective* roup_dir = roup_parse_with_language(input_str.c_str(), roup_lang);
    if (!roup_dir) {
        return nullptr;
    }

    // Get directive kind from ROUP
    int32_t roup_kind = roup_directive_kind(roup_dir);
    OpenMPDirectiveKind kind = mapRoupToOmpparserDirective(roup_kind);

    // Get the original directive name for special handling
    const char* directive_name = roup_directive_name(roup_dir);
    std::string dir_name_str(directive_name ? directive_name : "");

    // DEBUG: Print what we're parsing
    if (std::getenv("DEBUG_OMPPARSER")) {
        fprintf(stderr, "DEBUG: input='%s' roup_kind=%d omp_kind=%d dir_name='%s'\n",
                input, roup_kind, (int)kind, dir_name_str.c_str());
    }

    // Create ompparser-compatible directive based on kind
    // Some directives require special subclasses (allocate, threadprivate, atomic, etc.)
    OpenMPDirective* dir = nullptr;

    switch (kind) {
        case OMPD_allocate:
            dir = new OpenMPAllocateDirective();
            break;
        case OMPD_threadprivate:
            dir = new OpenMPThreadprivateDirective();
            break;
        case OMPD_atomic:
            dir = new OpenMPAtomicDirective();
            break;
        case OMPD_critical:
            dir = new OpenMPCriticalDirective();
            break;
        case OMPD_flush:
            dir = new OpenMPFlushDirective();
            break;
        case OMPD_ordered:
            dir = new OpenMPOrderedDirective();
            break;
        case OMPD_depobj:
            dir = new OpenMPDepobjDirective();
            break;
        case OMPD_declare_simd:
            dir = new OpenMPDeclareSimdDirective();
            break;
        case OMPD_declare_reduction:
            dir = new OpenMPDeclareReductionDirective();
            break;
        case OMPD_declare_mapper:
            // Requires parameter, use generic directive
            dir = new OpenMPDirective(kind, current_lang, 0, 0);
            break;
        case OMPD_declare_target:
            dir = new OpenMPDeclareTargetDirective();
            break;
        case OMPD_declare_variant:
            // Requires parameter, use generic directive
            dir = new OpenMPDirective(kind, current_lang, 0, 0);
            break;
        case OMPD_requires:
            dir = new OpenMPRequiresDirective();
            break;
        case OMPD_end:
            dir = new OpenMPEndDirective();
            break;
        default:
            // Generic directive
            dir = new OpenMPDirective(kind, current_lang, 0, 0);
            break;
    }

    // Handle atomic variants: "atomic read", "atomic write", etc.
    // ROUP parses these as a single directive name (e.g., "atomic read")
    // But ompparser expects OMPD_atomic with the modifier as a separate token
    // We add the modifier as a bare clause (no content) so it appears in output
    if (kind == OMPD_atomic && dir_name_str.length() > 7) {  // "atomic " = 7 chars
        std::string modifier = dir_name_str.substr(7);  // Extract "read", "write", etc.
        if (!modifier.empty()) {
            // Map atomic modifiers to clause kinds
            // These should appear as bare tokens in the output
            OpenMPClauseKind modifier_kind = OMPC_unknown;
            if (modifier == "read") {
                modifier_kind = static_cast<OpenMPClauseKind>(78);  // OMPC_read
            } else if (modifier == "write") {
                modifier_kind = static_cast<OpenMPClauseKind>(79);  // OMPC_write
            } else if (modifier == "update") {
                modifier_kind = static_cast<OpenMPClauseKind>(80);  // OMPC_update
            } else if (modifier == "capture") {
                modifier_kind = static_cast<OpenMPClauseKind>(81);  // OMPC_capture
            }

            if (modifier_kind != OMPC_unknown) {
                dir->addOpenMPClause(static_cast<int>(modifier_kind));
            }
        }
    }

    // Handle directives with parameters
    const char* parameter = roup_directive_parameter(roup_dir);
    if (parameter && parameter[0] != '\0') {
        std::string param_str(parameter);

        // Remove parentheses if present
        if (param_str.length() >= 2 && param_str[0] == '(' && param_str.back() == ')') {
            param_str = param_str.substr(1, param_str.length() - 2);
        }

        if (kind == OMPD_allocate) {
            // allocate(a,b,c) - variable list
            OpenMPAllocateDirective* alloc_dir = static_cast<OpenMPAllocateDirective*>(dir);
            alloc_dir->addAllocateList(strdup(param_str.c_str()));
        } else if (kind == OMPD_threadprivate) {
            // threadprivate(a,b,c) - variable list
            OpenMPThreadprivateDirective* tp_dir = static_cast<OpenMPThreadprivateDirective*>(dir);
            tp_dir->addThreadprivateList(strdup(param_str.c_str()));
        } else if (kind == OMPD_critical) {
            // critical(name) - critical section name
            OpenMPCriticalDirective* crit_dir = static_cast<OpenMPCriticalDirective*>(dir);
            crit_dir->setCriticalName(param_str.c_str());
        } else if (kind == OMPD_cancel || kind == OMPD_cancellation_point) {
            // cancel parallel - construct type as bare clause
            // Map construct type string to clause kind
            OpenMPClauseKind construct_clause = OMPC_unknown;
            if (param_str == "parallel") {
                construct_clause = static_cast<OpenMPClauseKind>(34);  // OMPC_parallel
            } else if (param_str == "sections") {
                construct_clause = static_cast<OpenMPClauseKind>(35);  // OMPC_sections
            } else if (param_str == "for") {
                construct_clause = static_cast<OpenMPClauseKind>(36);  // OMPC_for
            } else if (param_str == "do") {
                construct_clause = static_cast<OpenMPClauseKind>(37);  // OMPC_do
            } else if (param_str == "taskgroup") {
                construct_clause = static_cast<OpenMPClauseKind>(38);  // OMPC_taskgroup
            }

            if (construct_clause != OMPC_unknown) {
                // Add construct type as first clause (bare, no parameters)
                dir->addOpenMPClause(static_cast<int>(construct_clause));
            }
        }
    }

    // Convert clauses with their parameters from ROUP to ompparser
    // Track which clause kinds we've already created to enable merging
    // For if clauses, we need separate tracking per modifier
    std::map<OpenMPClauseKind, OpenMPClause*> created_clauses;
    std::map<int, OpenMPClause*> created_if_clauses; // key is modifier

    OmpClauseIterator* iter = roup_directive_clauses_iter(roup_dir);
    if (iter) {
        const OmpClause* roup_clause;
        while (roup_clause_iterator_next(iter, &roup_clause) == 1) {
            int32_t roup_kind_clause = roup_clause_kind(roup_clause);
            OpenMPClauseKind clause_kind = mapRoupToOmpparserClause(roup_kind_clause);

            // Get the raw clause content (e.g., "a, b, c" from private(a, b, c))
            const char* content = roup_clause_content(roup_clause);

            // Check if we've already created this clause kind (for merging)
            OpenMPClause* omp_clause = nullptr;

            // Special handling for linear clause - needs modifier and step parsing
            if (clause_kind == OMPC_linear) {
                // linear clause: linear(modifier(variables): step)
                // Format: linear(val(a,b,c):2) or linear(a,b,c:2)
                int modifier = 4; // OMPC_LINEAR_MODIFIER_unspecified
                std::string variables = "";
                std::string step = "";

                if (content && content[0] != '\0') {
                    std::string content_str(content);

                    // Check if content starts with a modifier (val/ref/uval followed by '(')
                    std::string modifier_str = "";
                    size_t paren_pos = content_str.find('(');

                    if (paren_pos != std::string::npos && paren_pos < 10) {
                        // Might be a modifier
                        std::string potential_mod = content_str.substr(0, paren_pos);
                        // Trim
                        size_t start = potential_mod.find_first_not_of(" \t");
                        if (start != std::string::npos) {
                            potential_mod = potential_mod.substr(start);
                        }
                        size_t end = potential_mod.find_last_not_of(" \t");
                        if (end != std::string::npos) {
                            potential_mod = potential_mod.substr(0, end + 1);
                        }

                        if (potential_mod == "val" || potential_mod == "ref" || potential_mod == "uval") {
                            modifier_str = potential_mod;

                            // Extract content inside modifier parentheses
                            // Find matching closing paren
                            int depth = 1;
                            size_t i = paren_pos + 1;
                            while (i < content_str.length() && depth > 0) {
                                if (content_str[i] == '(') depth++;
                                else if (content_str[i] == ')') depth--;
                                i++;
                            }

                            if (depth == 0) {
                                // Found matching paren
                                std::string inside_mod = content_str.substr(paren_pos + 1, i - paren_pos - 2);

                                // Check for step after modifier
                                size_t colon_pos = content_str.find(':', i);
                                if (colon_pos != std::string::npos) {
                                    variables = inside_mod;
                                    step = content_str.substr(colon_pos + 1);
                                } else {
                                    variables = inside_mod;
                                }
                            }
                        }
                    }

                    // If no modifier found, parse as simple format
                    if (modifier_str.empty()) {
                        size_t colon_pos = content_str.find(':');
                        if (colon_pos != std::string::npos) {
                            variables = content_str.substr(0, colon_pos);
                            step = content_str.substr(colon_pos + 1);
                        } else {
                            variables = content_str;
                        }
                    }

                    // Map modifier
                    if (modifier_str == "val") modifier = 0;
                    else if (modifier_str == "ref") modifier = 1;
                    else if (modifier_str == "uval") modifier = 2;

                    // Trim step
                    if (!step.empty()) {
                        size_t start = step.find_first_not_of(" \t");
                        if (start != std::string::npos) {
                            step = step.substr(start);
                        }
                        size_t end = step.find_last_not_of(" \t");
                        if (end != std::string::npos) {
                            step = step.substr(0, end + 1);
                        }
                    }
                }

                // Create linear clause
                omp_clause = dir->addOpenMPClause(static_cast<int>(clause_kind), modifier);

                // Set step if present
                if (!step.empty() && omp_clause) {
                    dynamic_cast<OpenMPLinearClause*>(omp_clause)->setUserDefinedStep(step.c_str());
                }

                // Add variables
                if (omp_clause && !variables.empty()) {
                    // Split by comma and add each variable separately
                    size_t pos = 0;
                    while (pos < variables.length()) {
                        // Find next comma
                        size_t comma_pos = variables.find(',', pos);
                        if (comma_pos == std::string::npos) {
                            comma_pos = variables.length();
                        }

                        // Extract variable
                        std::string var = variables.substr(pos, comma_pos - pos);

                        // Trim whitespace
                        size_t start = var.find_first_not_of(" \t\r\n");
                        if (start != std::string::npos) {
                            var = var.substr(start);
                        }
                        size_t end = var.find_last_not_of(" \t\r\n");
                        if (end != std::string::npos) {
                            var = var.substr(0, end + 1);
                        }

                        // Add non-empty variable
                        if (!var.empty()) {
                            omp_clause->addLangExpr(var.c_str());
                        }

                        pos = comma_pos + 1;
                    }
                }
            } else if (clause_kind == OMPC_reduction) {
                // Special handling for reduction clause - don't merge, handle separately
                // reduction clause: reduction(modifier, operator : variables)
                // Format: reduction(inscan, + : a, foo(x))
                //         reduction(abc : x, y, z)  [user-defined operator]
                //         reduction(task, user_defined_value : x, y, z)

                int modifier = 4; // OMPC_REDUCTION_MODIFIER_unspecified
                int identifier = 13; // OMPC_REDUCTION_IDENTIFIER_unknown
                std::string user_defined_identifier = "";
                std::string variables = "";

                if (content && content[0] != '\0') {
                    std::string content_str(content);

                    // Find colon (separates operator from variables)
                    size_t colon_pos = content_str.find(':');

                    if (colon_pos != std::string::npos) {
                        std::string before_colon = content_str.substr(0, colon_pos);
                        variables = content_str.substr(colon_pos + 1);

                        // Trim variables
                        size_t start = variables.find_first_not_of(" \t");
                        if (start != std::string::npos) {
                            variables = variables.substr(start);
                        }

                        // Parse before_colon for modifier and operator
                        // Check if there's a comma (indicates modifier present)
                        size_t comma_pos = before_colon.find(',');

                        std::string modifier_str, operator_str;

                        if (comma_pos != std::string::npos) {
                            // Has modifier
                            modifier_str = before_colon.substr(0, comma_pos);
                            operator_str = before_colon.substr(comma_pos + 1);
                        } else {
                            // No modifier, just operator
                            operator_str = before_colon;
                        }

                        // Trim and map modifier
                        if (!modifier_str.empty()) {
                            size_t mod_start = modifier_str.find_first_not_of(" \t");
                            size_t mod_end = modifier_str.find_last_not_of(" \t");
                            if (mod_start != std::string::npos && mod_end != std::string::npos) {
                                modifier_str = modifier_str.substr(mod_start, mod_end - mod_start + 1);
                            }

                            if (modifier_str == "inscan") modifier = 0;
                            else if (modifier_str == "task") modifier = 1;
                            else if (modifier_str == "default") modifier = 2;
                        }

                        // Trim and map operator
                        size_t op_start = operator_str.find_first_not_of(" \t");
                        size_t op_end = operator_str.find_last_not_of(" \t");
                        if (op_start != std::string::npos && op_end != std::string::npos) {
                            operator_str = operator_str.substr(op_start, op_end - op_start + 1);
                        }

                        // Map operator to identifier
                        if (operator_str == "+") identifier = 0; // plus
                        else if (operator_str == "-") identifier = 1; // minus
                        else if (operator_str == "*") identifier = 2; // mul
                        else if (operator_str == "&") identifier = 3; // bitand
                        else if (operator_str == "|") identifier = 4; // bitor
                        else if (operator_str == "^") identifier = 5; // bitxor
                        else if (operator_str == "&&") identifier = 6; // logand
                        else if (operator_str == "||") identifier = 7; // logor
                        else if (operator_str == ".eqv.") identifier = 8; // eqv
                        else if (operator_str == ".neqv.") identifier = 9; // neqv
                        else if (operator_str == "max") identifier = 10;
                        else if (operator_str == "min") identifier = 11;
                        else {
                            // User-defined operator
                            identifier = 12; // user
                            user_defined_identifier = operator_str;
                        }
                    }
                }

                // Create reduction clause (don't track in created_clauses - it handles its own merging)
                omp_clause = dir->addOpenMPClause(static_cast<int>(clause_kind),
                                                  modifier, identifier,
                                                  user_defined_identifier.empty() ? nullptr : const_cast<char*>(user_defined_identifier.c_str()));

                // Add variables
                if (omp_clause && !variables.empty()) {
                    // Split by comma and add each variable separately
                    size_t pos = 0;
                    while (pos < variables.length()) {
                        // Find next comma (but respect parentheses for function calls like foo(x))
                        int paren_depth = 0;
                        size_t comma_pos = pos;
                        while (comma_pos < variables.length()) {
                            if (variables[comma_pos] == '(') paren_depth++;
                            else if (variables[comma_pos] == ')') paren_depth--;
                            else if (variables[comma_pos] == ',' && paren_depth == 0) break;
                            comma_pos++;
                        }

                        // Extract variable
                        std::string var = variables.substr(pos, comma_pos - pos);

                        // Trim whitespace
                        size_t start = var.find_first_not_of(" \t\r\n");
                        if (start != std::string::npos) {
                            var = var.substr(start);
                        }
                        size_t end = var.find_last_not_of(" \t\r\n");
                        if (end != std::string::npos) {
                            var = var.substr(0, end + 1);
                        }

                        // Add non-empty variable
                        if (!var.empty()) {
                            omp_clause->addLangExpr(var.c_str());
                        }

                        pos = comma_pos + 1;
                    }
                }
            } else if (clause_kind == OMPC_if) {
                // Special handling for if clause - each modifier needs separate clause
                // Parse if clause: "if(modifier: condition)" or "if(condition)"
                // Default modifier is unspecified (10) if not provided
                int modifier = 10; // OMPC_IF_MODIFIER_unspecified

                if (content && content[0] != '\0') {
                    std::string content_str(content);
                    size_t colon_pos = content_str.find(':');

                    if (colon_pos != std::string::npos) {
                        // Extract modifier
                        std::string modifier_str = content_str.substr(0, colon_pos);
                        // Trim whitespace
                        size_t start = modifier_str.find_first_not_of(" \t");
                        size_t end = modifier_str.find_last_not_of(" \t");
                        if (start != std::string::npos && end != std::string::npos) {
                            modifier_str = modifier_str.substr(start, end - start + 1);
                        }

                        // Map modifier string to enum
                        if (modifier_str == "parallel") modifier = 0;
                        else if (modifier_str == "simd") modifier = 1;
                        else if (modifier_str == "task") modifier = 2;
                        else if (modifier_str == "cancel") modifier = 3;
                        else if (modifier_str == "target_data") modifier = 4;
                        else if (modifier_str == "target_enter_data") modifier = 5;
                        else if (modifier_str == "target_exit_data") modifier = 6;
                        else if (modifier_str == "target") modifier = 7;
                        else if (modifier_str == "target_update") modifier = 8;
                        else if (modifier_str == "taskloop") modifier = 9;
                    }
                }

                // Check if we already created an if clause with this modifier
                auto if_it = created_if_clauses.find(modifier);
                if (if_it != created_if_clauses.end()) {
                    omp_clause = if_it->second;
                } else {
                    omp_clause = dir->addOpenMPClause(static_cast<int>(clause_kind), modifier, nullptr);
                    created_if_clauses[modifier] = omp_clause;
                }
            } else {
                // Check for existing clause of this kind (for merging)
                auto it = created_clauses.find(clause_kind);
                if (it != created_clauses.end()) {
                    // Reuse existing clause (merging)
                    omp_clause = it->second;
                } else if (clause_kind == OMPC_default) {
                    // default clause needs parameter: shared/none/private/firstprivate
                    int default_kind = 2; // OMPC_DEFAULT_shared (default)

                    if (content && content[0] != '\0') {
                        std::string content_str(content);
                        // Trim whitespace
                        size_t start = content_str.find_first_not_of(" \t");
                        size_t end = content_str.find_last_not_of(" \t");
                        if (start != std::string::npos && end != std::string::npos) {
                            content_str = content_str.substr(start, end - start + 1);
                        }

                        // Map default kind string to enum
                        if (content_str == "private") default_kind = 0;
                        else if (content_str == "firstprivate") default_kind = 1;
                        else if (content_str == "shared") default_kind = 2;
                        else if (content_str == "none") default_kind = 3;
                        else if (content_str == "variant") default_kind = 4;
                    }

                    omp_clause = dir->addOpenMPClause(static_cast<int>(clause_kind), default_kind);
                    created_clauses[clause_kind] = omp_clause;
                } else if (clause_kind == OMPC_schedule) {
                    // schedule clause: schedule(mod1,mod2:kind,chunk)
                    // Possible formats:
                    // - schedule(monotonic,simd:runtime,2)
                    // - schedule(monotonic:runtime,2)
                    // - schedule(runtime,2)
                    // - schedule(runtime)

                    int modifier1 = 4; // OMPC_SCHEDULE_MODIFIER_unspecified
                    int modifier2 = 4; // OMPC_SCHEDULE_MODIFIER_unspecified
                    int schedule_kind = 0; // OMPC_SCHEDULE_KIND_static (default)
                    std::string chunk_size = "";

                    if (content && content[0] != '\0') {
                        std::string content_str(content);

                        // Find colon (separates modifiers from kind)
                        size_t colon_pos = content_str.find(':');

                        std::string before_colon;
                        std::string after_colon;

                        if (colon_pos != std::string::npos) {
                            before_colon = content_str.substr(0, colon_pos);
                            after_colon = content_str.substr(colon_pos + 1);
                        } else {
                            after_colon = content_str;
                        }

                        // Parse modifiers from before_colon (if present)
                        if (!before_colon.empty()) {
                            // Split by comma
                            size_t comma_pos = before_colon.find(',');
                            std::string mod1_str, mod2_str;

                            if (comma_pos != std::string::npos) {
                                mod1_str = before_colon.substr(0, comma_pos);
                                mod2_str = before_colon.substr(comma_pos + 1);
                            } else {
                                mod1_str = before_colon;
                            }

                            // Trim and map modifier1
                            size_t start = mod1_str.find_first_not_of(" \t");
                            size_t end = mod1_str.find_last_not_of(" \t");
                            if (start != std::string::npos && end != std::string::npos) {
                                mod1_str = mod1_str.substr(start, end - start + 1);
                            }

                            if (mod1_str == "monotonic") modifier1 = 0;
                            else if (mod1_str == "nonmonotonic") modifier1 = 1;
                            else if (mod1_str == "simd") modifier1 = 2;
                            else if (mod1_str == "user") modifier1 = 3;

                            // Trim and map modifier2
                            if (!mod2_str.empty()) {
                                start = mod2_str.find_first_not_of(" \t");
                                end = mod2_str.find_last_not_of(" \t");
                                if (start != std::string::npos && end != std::string::npos) {
                                    mod2_str = mod2_str.substr(start, end - start + 1);
                                }

                                if (mod2_str == "monotonic") modifier2 = 0;
                                else if (mod2_str == "nonmonotonic") modifier2 = 1;
                                else if (mod2_str == "simd") modifier2 = 2;
                                else if (mod2_str == "user") modifier2 = 3;
                            }
                        }

                        // Parse kind and chunk_size from after_colon
                        if (!after_colon.empty()) {
                            // Find last comma (separates kind from chunk_size)
                            size_t last_comma = after_colon.rfind(',');
                            std::string kind_str;

                            if (last_comma != std::string::npos) {
                                kind_str = after_colon.substr(0, last_comma);
                                chunk_size = after_colon.substr(last_comma + 1);

                                // Trim chunk_size
                                size_t start = chunk_size.find_first_not_of(" \t");
                                if (start != std::string::npos) {
                                    chunk_size = chunk_size.substr(start);
                                }
                                size_t end = chunk_size.find_last_not_of(" \t");
                                if (end != std::string::npos) {
                                    chunk_size = chunk_size.substr(0, end + 1);
                                }
                            } else {
                                kind_str = after_colon;
                            }

                            // Trim and map schedule kind
                            size_t start = kind_str.find_first_not_of(" \t");
                            size_t end = kind_str.find_last_not_of(" \t");
                            if (start != std::string::npos && end != std::string::npos) {
                                kind_str = kind_str.substr(start, end - start + 1);
                            }

                            if (kind_str == "static") schedule_kind = 0;
                            else if (kind_str == "dynamic") schedule_kind = 1;
                            else if (kind_str == "guided") schedule_kind = 2;
                            else if (kind_str == "auto") schedule_kind = 3;
                            else if (kind_str == "runtime") schedule_kind = 4;
                            else if (kind_str == "user") schedule_kind = 5;
                        }
                    }

                    // Create schedule clause with all parameters
                    omp_clause = dir->addOpenMPClause(static_cast<int>(clause_kind),
                                                      modifier1, modifier2, schedule_kind, nullptr);

                    // Set chunk size if present
                    if (!chunk_size.empty() && omp_clause) {
                        dynamic_cast<OpenMPScheduleClause*>(omp_clause)->setChunkSize(chunk_size.c_str());
                    }

                    created_clauses[clause_kind] = omp_clause;
                } else if (clause_kind == OMPC_proc_bind) {
                    // proc_bind clause needs parameter: master/close/spread
                    int proc_bind_kind = 3; // OMPC_PROC_BIND_unknown (default)

                    if (content && content[0] != '\0') {
                        std::string content_str(content);
                        // Trim whitespace
                        size_t start = content_str.find_first_not_of(" \t");
                        size_t end = content_str.find_last_not_of(" \t");
                        if (start != std::string::npos && end != std::string::npos) {
                            content_str = content_str.substr(start, end - start + 1);
                        }

                        // Map proc_bind kind string to enum
                        if (content_str == "master") proc_bind_kind = 0;
                        else if (content_str == "close") proc_bind_kind = 1;
                        else if (content_str == "spread") proc_bind_kind = 2;
                    }

                    omp_clause = dir->addOpenMPClause(static_cast<int>(clause_kind), proc_bind_kind);
                    created_clauses[clause_kind] = omp_clause;
                } else if (clause_kind == OMPC_lastprivate) {
                    // lastprivate clause needs modifier: unspecified/conditional
                    // Format: lastprivate(conditional: a, b, c) or lastprivate(a, b, c)
                    int modifier = 0; // OMPC_LASTPRIVATE_MODIFIER_unspecified (default)

                    if (content && content[0] != '\0') {
                        std::string content_str(content);

                        // Check if content starts with "conditional:"
                        if (content_str.find("conditional") == 0 ||
                            content_str.find("conditional:") == 0 ||
                            content_str.find("conditional :") == 0) {
                            modifier = 1; // OMPC_LASTPRIVATE_MODIFIER_conditional
                        }
                    }

                    omp_clause = dir->addOpenMPClause(static_cast<int>(clause_kind), modifier);
                    created_clauses[clause_kind] = omp_clause;
                } else if (clause_kind == OMPC_order) {
                    // order clause needs parameter: concurrent/unspecified
                    int order_kind = 1; // OMPC_ORDER_unspecified (default)

                    if (content && content[0] != '\0') {
                        std::string content_str(content);
                        // Trim whitespace
                        size_t start = content_str.find_first_not_of(" \t");
                        size_t end = content_str.find_last_not_of(" \t");
                        if (start != std::string::npos && end != std::string::npos) {
                            content_str = content_str.substr(start, end - start + 1);
                        }

                        // Map order kind string to enum
                        if (content_str == "concurrent") order_kind = 0;
                    }

                    omp_clause = dir->addOpenMPClause(static_cast<int>(clause_kind), order_kind);
                    created_clauses[clause_kind] = omp_clause;
                } else {
                    // Generic clause creation
                    omp_clause = dir->addOpenMPClause(static_cast<int>(clause_kind));
                    created_clauses[clause_kind] = omp_clause;
                }
            }

            // If clause has content, parse and add variables individually
            // This enables ompparser's duplicate detection to work correctly
            // Skip for clauses that handle their own parameters/variables
            if (omp_clause && content && content[0] != '\0' &&
                clause_kind != OMPC_reduction && clause_kind != OMPC_schedule && clause_kind != OMPC_linear &&
                clause_kind != OMPC_default && clause_kind != OMPC_proc_bind && clause_kind != OMPC_order) {
                std::string content_str(content);

                // Special handling for lastprivate clause: strip conditional modifier prefix
                if (clause_kind == OMPC_lastprivate) {
                    size_t colon_pos = content_str.find(':');
                    if (colon_pos != std::string::npos) {
                        // Check if it's "conditional:" prefix
                        std::string before_colon = content_str.substr(0, colon_pos);
                        // Trim
                        size_t start = before_colon.find_first_not_of(" \t");
                        if (start != std::string::npos) {
                            before_colon = before_colon.substr(start);
                        }
                        size_t end = before_colon.find_last_not_of(" \t");
                        if (end != std::string::npos) {
                            before_colon = before_colon.substr(0, end + 1);
                        }

                        if (before_colon == "conditional") {
                            // Strip "conditional:" prefix, keep only the variables
                            content_str = content_str.substr(colon_pos + 1);
                            // Trim leading whitespace
                            start = content_str.find_first_not_of(" \t");
                            if (start != std::string::npos) {
                                content_str = content_str.substr(start);
                            }
                        }
                    }

                    // Split by comma and add each variable separately
                    size_t pos = 0;
                    while (pos < content_str.length()) {
                        // Find next comma
                        size_t comma_pos = content_str.find(',', pos);
                        if (comma_pos == std::string::npos) {
                            comma_pos = content_str.length();
                        }

                        // Extract variable (trimming whitespace)
                        std::string var = content_str.substr(pos, comma_pos - pos);

                        // Trim leading whitespace
                        size_t start = var.find_first_not_of(" \t\r\n");
                        if (start != std::string::npos) {
                            var = var.substr(start);
                        }

                        // Trim trailing whitespace
                        size_t end = var.find_last_not_of(" \t\r\n");
                        if (end != std::string::npos) {
                            var = var.substr(0, end + 1);
                        }

                        // Add non-empty variable
                        if (!var.empty()) {
                            omp_clause->addLangExpr(var.c_str());
                        }

                        pos = comma_pos + 1;
                    }
                } else if (clause_kind == OMPC_if) {
                    // Special handling for if clause: strip modifier prefix
                    size_t colon_pos = content_str.find(':');
                    if (colon_pos != std::string::npos) {
                        // Strip "modifier:" prefix, keep only the condition
                        content_str = content_str.substr(colon_pos + 1);
                        // Trim leading whitespace
                        size_t start = content_str.find_first_not_of(" \t");
                        if (start != std::string::npos) {
                            content_str = content_str.substr(start);
                        }
                    }
                    // Add the condition expression
                    if (!content_str.empty()) {
                        omp_clause->addLangExpr(content_str.c_str());
                    }
                } else {
                    // Split by comma and add each variable separately
                    // This allows ompparser's addLangExpr to deduplicate
                    size_t pos = 0;
                    while (pos < content_str.length()) {
                        // Find next comma
                        size_t comma_pos = content_str.find(',', pos);
                        if (comma_pos == std::string::npos) {
                            comma_pos = content_str.length();
                        }

                        // Extract variable (trimming whitespace)
                        std::string var = content_str.substr(pos, comma_pos - pos);

                        // Trim leading whitespace
                        size_t start = var.find_first_not_of(" \t\r\n");
                        if (start != std::string::npos) {
                            var = var.substr(start);
                        }

                        // Trim trailing whitespace
                        size_t end = var.find_last_not_of(" \t\r\n");
                        if (end != std::string::npos) {
                            var = var.substr(0, end + 1);
                        }

                        // Add non-empty variable
                        if (!var.empty()) {
                            omp_clause->addLangExpr(var.c_str());
                        }

                        pos = comma_pos + 1;
                    }
                }
            }
        }
        roup_clause_iterator_free(iter);
    }

    // Free ROUP directive (we've extracted what we need)
    roup_directive_free(roup_dir);

    return dir;
}

} // extern "C"
