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

    // Create ompparser-compatible directive
    // Use ompparser's actual constructor: OpenMPDirective(kind, lang, line, col)
    OpenMPDirective* dir = new OpenMPDirective(kind, current_lang, 0, 0);

    // Convert clauses with their parameters from ROUP to ompparser
    OmpClauseIterator* iter = roup_directive_clauses_iter(roup_dir);
    if (iter) {
        const OmpClause* roup_clause;
        while (roup_clause_iterator_next(iter, &roup_clause) == 1) {
            int32_t roup_kind_clause = roup_clause_kind(roup_clause);
            OpenMPClauseKind clause_kind = mapRoupToOmpparserClause(roup_kind_clause);

            // Get the raw clause content (e.g., "a, b, c" from private(a, b, c))
            const char* content = roup_clause_content(roup_clause);

            // Create clause
            OpenMPClause* omp_clause = dir->addOpenMPClause(static_cast<int>(clause_kind));

            // If clause has content, add it as a single expression
            // ompparser will parse it (variables, expressions, etc.)
            if (omp_clause && content && content[0] != '\0') {
                omp_clause->addLangExpr(content);
            }
        }
        roup_clause_iterator_free(iter);
    }

    // Free ROUP directive (we've extracted what we need)
    roup_directive_free(roup_dir);

    return dir;
}

} // extern "C"
