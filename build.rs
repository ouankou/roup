//! Build script to generate roup_constants.h from source.
//!
//! **Usage:** Runs automatically during `cargo build`
//!
//! **Source of truth:** `src/c_api.rs` (directive_name_to_kind, convert_clause)
//!
//! For design rationale and maintenance instructions:
//! See [`docs/BUILD_SCRIPT_RATIONALE.md`](../docs/BUILD_SCRIPT_RATIONALE.md)

use std::env;
use std::fs;
use std::path::Path;

// Use #[path] to share code between build.rs and main crate
// This pattern is documented in docs/BUILD_SCRIPT_RATIONALE.md (see section "Dual-purpose #[path] pattern" for rationale and ecosystem examples).
// Rationale: build.rs runs in a separate environment (can't access crate modules), so we use
// #[path] to include constants_gen for both build-time generation and runtime verification.
// Alternative (separate build-utils crate) rejected: adds workspace complexity for several hundred lines.
#[path = "src/constants_gen.rs"]
mod constants_gen;

use constants_gen::*;

/// Generate the header file content
///
/// This function is specific to build.rs and generates the C header with all constants.
/// The gen binary doesn't need this - it only verifies using checksum comparison.
fn generate_header(
    directives: &[(String, i32)],
    clauses: &[(String, i32)],
    acc_directives: &[(String, i32)],
    acc_clauses: &[(String, i32)],
    directive_kinds: &[(String, i32)],
    clause_kinds: &[(String, i32)],
    reduction_ops: &[(String, i32)],
    schedule_kinds: &[(String, i32)],
    default_kinds: &[(String, i32)],
    proc_bind_kinds: &[(String, i32)],
    reduction_modifiers: &[(String, i32)],
    if_modifiers: &[(String, i32)],
    order_modifiers: &[(String, i32)],
    grainsize_modifiers: &[(String, i32)],
    num_tasks_modifiers: &[(String, i32)],
    bind_kinds: &[(String, i32)],
    at_kinds: &[(String, i32)],
    severity_kinds: &[(String, i32)],
) -> String {
    // Generate DirectiveKind enum constants from IR layer (source of truth)
    let mut directive_kind_defs = String::new();
    for (name, num) in directive_kinds {
        directive_kind_defs.push_str(&format!("#define ROUP_DIRECTIVE_KIND_{name:<35} {num}\n"));
    }

    // Generate ClauseKind constants from C API (source of truth)
    let mut clause_kind_defs = String::new();
    for (name, num) in clause_kinds {
        clause_kind_defs.push_str(&format!("#define ROUP_CLAUSE_KIND_{name:<15} {num}\n"));
    }

    // Generate default_kind constants from IR enum
    let mut default_kind_defs = String::new();
    for (name, num) in default_kinds {
        default_kind_defs.push_str(&format!("#define ROUP_DEFAULT_KIND_{name:<15} {num}\n"));
    }

    // Generate proc_bind_kind constants from IR enum
    let mut proc_bind_kind_defs = String::new();
    for (name, num) in proc_bind_kinds {
        proc_bind_kind_defs.push_str(&format!("#define ROUP_PROC_BIND_KIND_{name:<15} {num}\n"));
    }

    // Generate schedule_kind constants from IR enum
    let mut schedule_kind_defs = String::new();
    for (name, num) in schedule_kinds {
        schedule_kind_defs.push_str(&format!("#define ROUP_SCHEDULE_KIND_{name:<15} {num}\n"));
    }

    // Generate reduction_op constants from IR enum
    let mut reduction_op_defs = String::new();
    for (name, num) in reduction_ops {
        reduction_op_defs.push_str(&format!("#define ROUP_REDUCTION_OP_{name:<15} {num}\n"));
    }

    // Generate reduction_modifier constants from IR enum
    let mut reduction_modifier_defs = String::new();
    for (name, num) in reduction_modifiers {
        reduction_modifier_defs.push_str(&format!("#define ROUP_REDUCTION_MODIFIER_{name:<15} {num}\n"));
    }

    // Generate if_modifier constants from IR enum
    let mut if_modifier_defs = String::new();
    for (name, num) in if_modifiers {
        if_modifier_defs.push_str(&format!("#define ROUP_IF_MODIFIER_{name:<20} {num}\n"));
    }

    // Generate order_modifier constants from IR enum
    let mut order_modifier_defs = String::new();
    for (name, num) in order_modifiers {
        order_modifier_defs.push_str(&format!("#define ROUP_ORDER_MODIFIER_{name:<15} {num}\n"));
    }

    // Generate grainsize_modifier constants from IR enum
    let mut grainsize_modifier_defs = String::new();
    for (name, num) in grainsize_modifiers {
        grainsize_modifier_defs.push_str(&format!("#define ROUP_GRAINSIZE_MODIFIER_{name:<15} {num}\n"));
    }

    // Generate num_tasks_modifier constants from IR enum
    let mut num_tasks_modifier_defs = String::new();
    for (name, num) in num_tasks_modifiers {
        num_tasks_modifier_defs.push_str(&format!("#define ROUP_NUM_TASKS_MODIFIER_{name:<15} {num}\n"));
    }

    // Generate bind_kind constants from IR enum
    let mut bind_kind_defs = String::new();
    for (name, num) in bind_kinds {
        bind_kind_defs.push_str(&format!("#define ROUP_BIND_KIND_{name:<15} {num}\n"));
    }

    // Generate at_kind constants from IR enum
    let mut at_kind_defs = String::new();
    for (name, num) in at_kinds {
        at_kind_defs.push_str(&format!("#define ROUP_AT_KIND_{name:<15} {num}\n"));
    }

    // Generate severity_kind constants from IR enum
    let mut severity_kind_defs = String::new();
    for (name, num) in severity_kinds {
        severity_kind_defs.push_str(&format!("#define ROUP_SEVERITY_KIND_{name:<15} {num}\n"));
    }

    // Generate OpenMP directive constants
    let mut directive_defs = String::new();
    for (name, num) in directives {
        directive_defs.push_str(&format!("#define ROUP_DIRECTIVE_{name:<20} {num}\n"));
    }
    directive_defs.push_str(&format!(
        "#define ROUP_DIRECTIVE_UNKNOWN       {UNKNOWN_KIND}\n"
    ));

    // Generate OpenMP clause constants
    let mut clause_defs = String::new();
    for (name, num) in clauses {
        clause_defs.push_str(&format!("#define ROUP_CLAUSE_{name:<15} {num}\n"));
    }
    clause_defs.push_str(&format!(
        "#define ROUP_CLAUSE_UNKNOWN      {UNKNOWN_KIND}\n"
    ));

    // Generate OpenACC directive constants
    let mut acc_directive_defs = String::new();
    for (name, num) in acc_directives {
        acc_directive_defs.push_str(&format!("#define ACC_DIRECTIVE_{name:<20} {num}\n"));
    }
    acc_directive_defs.push_str(&format!(
        "#define ACC_DIRECTIVE_UNKNOWN        {UNKNOWN_KIND}\n"
    ));

    // Generate OpenACC clause constants
    let mut acc_clause_defs = String::new();
    for (name, num) in acc_clauses {
        acc_clause_defs.push_str(&format!("#define ACC_CLAUSE_{name:<15} {num}\n"));
    }
    acc_clause_defs.push_str(&format!(
        "#define ACC_CLAUSE_UNKNOWN       {UNKNOWN_KIND}\n"
    ));

    // Generate checksum for validation (includes both OpenMP and OpenACC)
    let checksum = calculate_combined_checksum(directives, clauses, acc_directives, acc_clauses);
    let dir_count = directives.len();
    let clause_count = clauses.len();
    let acc_dir_count = acc_directives.len();
    let acc_clause_count = acc_clauses.len();

    format!(
        r#"/*
 * ROUP C API Constants (Auto-generated)
 *
 * DO NOT EDIT THIS FILE MANUALLY!
 * Generated by build.rs from src/c_api.rs
 *
 * Single source of truth: src/c_api.rs
 * - directive_name_to_kind() for directives
 * - convert_clause() for clauses
 *
 * Copyright (c) 2025 ROUP Project
 * SPDX-License-Identifier: BSD-3-Clause
 */

#ifndef ROUP_CONSTANTS_H
#define ROUP_CONSTANTS_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {{
#endif

// ============================================================================
// Synchronization Check
// ============================================================================
// Auto-generated checksum: FNV-1a hash of OpenMP ({dir_count} directives + {clause_count} clauses) + OpenACC ({acc_dir_count} directives + {acc_clause_count} clauses) = 0x{checksum:016X}
// If this doesn't match c_api.rs, rebuild with `cargo clean && cargo build`
#define ROUP_CONSTANTS_CHECKSUM 0x{checksum:016X}

// ============================================================================
// Language Format Constants
// ============================================================================
// Language format for roup_parse_with_language() and acc_parse_with_language()
#define ROUP_LANG_C                         0  // C/C++ (#pragma omp/#pragma acc)
#define ROUP_LANG_FORTRAN_FREE              1  // Fortran free-form (!$OMP/!$ACC)
#define ROUP_LANG_FORTRAN_FIXED             2  // Fortran fixed-form (!$OMP/!$ACC or C$OMP/C$ACC)

// ============================================================================
// DirectiveKind Enum Constants (ROUP IR Layer)
// ============================================================================
// Auto-generated from src/ir/directive.rs:DirectiveKind enum
// These are the authoritative directive kind values from ROUP's IR layer.
// Use these for mapping ROUP DirectiveKind to other parser enums.

{directive_kind_defs}
// ============================================================================
// ClauseKind Constants (C API Layer)
// ============================================================================
// Auto-generated from src/c_api.rs:clause_kind module
// These are the clause kind codes used by the C API.

{clause_kind_defs}

// ============================================================================
// Default Kind Constants (IR Enum)
// ============================================================================
// Auto-generated from src/ir/clause.rs:DefaultKind enum

{default_kind_defs}

// ============================================================================
// Proc Bind Kind Constants (IR Enum)
// ============================================================================
// Auto-generated from src/ir/clause.rs:ProcBind enum

{proc_bind_kind_defs}

// ============================================================================
// Schedule Kind Constants (IR Enum)
// ============================================================================
// Auto-generated from src/ir/clause.rs:ScheduleKind enum

{schedule_kind_defs}

// ============================================================================
// Reduction Operator Constants (IR Enum)
// ============================================================================
// Auto-generated from src/ir/clause.rs:ReductionOperator enum

{reduction_op_defs}

// ============================================================================
// Reduction Modifier Constants (IR Enum)
// ============================================================================
// Auto-generated from src/ir/clause.rs:ReductionModifier enum

{reduction_modifier_defs}

// ============================================================================
// If Modifier Constants (IR Enum)
// ============================================================================
// Auto-generated from src/ir/clause.rs:IfModifier enum

{if_modifier_defs}

// ============================================================================
// Order Modifier Constants (IR Enum)
// ============================================================================
// Auto-generated from src/ir/clause.rs:OrderModifier enum

{order_modifier_defs}

// ============================================================================
// Grainsize Modifier Constants (IR Enum)
// ============================================================================
// Auto-generated from src/ir/clause.rs:GrainsizeModifier enum

{grainsize_modifier_defs}

// ============================================================================
// NumTasks Modifier Constants (IR Enum)
// ============================================================================
// Auto-generated from src/ir/clause.rs:NumTasksModifier enum

{num_tasks_modifier_defs}

// ============================================================================
// Bind Kind Constants (IR Enum)
// ============================================================================
// Auto-generated from src/ir/clause.rs:BindKind enum

{bind_kind_defs}

// ============================================================================
// At Kind Constants (IR Enum)
// ============================================================================
// Auto-generated from src/ir/clause.rs:AtKind enum

{at_kind_defs}

// ============================================================================
// Severity Kind Constants (IR Enum)
// ============================================================================
// Auto-generated from src/ir/clause.rs:SeverityKind enum

{severity_kind_defs}

// ============================================================================
// OpenMP Directive Kind Constants
// ============================================================================
// Auto-generated from src/c_api.rs:directive_name_to_kind()

{directive_defs}

// ============================================================================
// OpenMP Clause Kind Constants
// ============================================================================
// Auto-generated from src/c_api.rs:convert_clause()

{clause_defs}

// ============================================================================
// OpenACC Directive Kind Constants
// ============================================================================
// Auto-generated from src/c_api.rs:acc_directive_name_to_kind()

{acc_directive_defs}

// ============================================================================
// OpenACC Clause Kind Constants
// ============================================================================
// Auto-generated from src/c_api.rs:convert_acc_clause()

{acc_clause_defs}

// ============================================================================
// Validation Constants
// ============================================================================
#define ROUP_MAX_PRAGMA_LENGTH 65536  // 64KB

#ifdef __cplusplus
}}
#endif

#endif /* ROUP_CONSTANTS_H */
"#
    )
}

fn main() {
    // Parse DirectiveKind enum from IR layer (source of truth)
    let directive_kinds = parse_directive_kind_enum();

    // Generate all constant modules from IR enums
    let (
        rust_modules,
        clause_kinds,
        reduction_ops,
        schedule_kinds,
        default_kinds,
        proc_bind_kinds,
        reduction_modifiers,
        if_modifiers,
        order_modifiers,
        grainsize_modifiers,
        num_tasks_modifiers,
        bind_kinds,
        at_kinds,
        severity_kinds,
    ) = generate_constants_from_ir();

    // Parse OpenMP constants from source
    let directives = parse_directive_mappings();
    let clauses = parse_clause_mappings();

    // Parse OpenACC constants from source
    let acc_directives = parse_acc_directive_mappings();
    let acc_clauses = parse_acc_clause_mappings();

    // Get the output directory
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("roup_constants.h");

    // Generate the header with DirectiveKind, ClauseKind, OpenMP, and OpenACC constants
    let header = generate_header(
        &directives,
        &clauses,
        &acc_directives,
        &acc_clauses,
        &directive_kinds,
        &clause_kinds,
        &reduction_ops,
        &schedule_kinds,
        &default_kinds,
        &proc_bind_kinds,
        &reduction_modifiers,
        &if_modifiers,
        &order_modifiers,
        &grainsize_modifiers,
        &num_tasks_modifiers,
        &bind_kinds,
        &at_kinds,
        &severity_kinds,
    );

    // Write to OUT_DIR
    fs::write(&dest_path, &header).expect("Failed to write to OUT_DIR");

    // Also copy to src/ for easier access during development
    let src_dest = Path::new("src").join("roup_constants.h");
    fs::write(&src_dest, &header).expect("Failed to write to src/");

    // Write Rust constant modules to OUT_DIR
    let modules_dest = Path::new(&out_dir).join("constants_modules.rs");
    fs::write(&modules_dest, &rust_modules).expect("Failed to write constants_modules.rs");

    // Validate checksum by reading back the generated file (Case 1 validation)
    let generated_content =
        fs::read_to_string(&src_dest).expect("Failed to read generated header for validation");

    let extracted_checksum = extract_checksum_from_header(&generated_content);
    let expected_checksum =
        calculate_combined_checksum(&directives, &clauses, &acc_directives, &acc_clauses);

    let dir_count = directives.len();
    let clause_count = clauses.len();
    let acc_dir_count = acc_directives.len();
    let acc_clause_count = acc_clauses.len();

    match extracted_checksum {
        Some(checksum) => {
            assert_eq!(
                checksum,
                expected_checksum,
                "FATAL: Constants checksum mismatch!\n\
                 Expected: 0x{expected_checksum:08X} (FNV-1a hash of OpenMP: {dir_count} directives + {clause_count} clauses, OpenACC: {acc_dir_count} directives + {acc_clause_count} clauses)\n\
                 Found in header: 0x{checksum:08X}\n\
                 The generated header is out of sync with c_api.rs.\n\
                 This should never happen - build.rs generates both values.\n\
                 Please file a bug report."
            );
        }
        None => {
            panic!("FATAL: Could not find ROUP_CONSTANTS_CHECKSUM in generated header");
        }
    }

    println!("cargo:rerun-if-changed=src/c_api.rs");
    println!("cargo:rerun-if-changed=src/c_api/openacc.rs");
    println!("cargo:rerun-if-changed=src/ir/directive.rs");
    println!("cargo:rerun-if-changed=src/constants_gen.rs");
    println!("cargo:rerun-if-changed=build.rs");
}
