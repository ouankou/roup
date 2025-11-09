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
    std::map<OpenMPClauseKind, OpenMPClause*> created_clauses;

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
            auto it = created_clauses.find(clause_kind);
            if (it != created_clauses.end()) {
                // Reuse existing clause (merging)
                omp_clause = it->second;
            } else {
                // Create new clause
                omp_clause = dir->addOpenMPClause(static_cast<int>(clause_kind));
                created_clauses[clause_kind] = omp_clause;
            }

            // If clause has content, parse and add variables individually
            // This enables ompparser's duplicate detection to work correctly
            if (omp_clause && content && content[0] != '\0') {
                std::string content_str(content);

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
        roup_clause_iterator_free(iter);
    }

    // Free ROUP directive (we've extracted what we need)
    roup_directive_free(roup_dir);

    return dir;
}

} // extern "C"
