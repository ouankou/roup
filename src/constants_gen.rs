//! Shared code for generating and validating roup_constants.h
//!
//! This module contains the core logic for parsing directive/clause mappings
//! from c_api.rs using syn-based AST parsing. It's used by both:
//! - build.rs (to generate the header during compilation)
//! - Standalone mode (to verify header is up-to-date in CI)
//!
//! # Current Limitations
//!
//! **Composite Directives**: Directives with spaces (e.g., "parallel for", "target teams")
//! are currently excluded from the constant mapping. In the ompparser compatibility layer,
//! these have dedicated enum values (e.g., OMPD_parallel_for, OMPD_target_teams in
//! OpenMPDirectiveKind), but mapping composite directives from ROUP to these ompparser
//! enums is not yet implemented. Only simple, single-word directive names are currently
//! supported in the generated constants.
//!
//! # Hash Function Choice
//!
//! FNV-1a is used for checksum generation because it provides:
//! - Fast computation for small input sets
//! - Good distribution and low collision rates
//! - Sufficient non-cryptographic integrity verification
//! - Wide adoption in compilers (e.g., Rust's rustc_span for symbol hashing)
//!
//! See: <https://github.com/rust-lang/rust/blob/master/compiler/rustc_span/src/symbol.rs>

use std::collections::HashSet;
use std::fs;

use syn::{Arm, Expr, ExprLit, ExprMatch, ExprTuple, File, Item, ItemFn, Lit, Pat, PatLit};

/// Special value for unknown directive/clause kinds
pub const UNKNOWN_KIND: i32 = 999;

/// FNV-1a hash algorithm constants
const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

/// Normalize constant name: convert CamelCase to UPPER_SNAKE_CASE
fn normalize_constant_name(name: &str) -> String {
    let mut result = String::new();
    let mut chars = name.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch.is_uppercase() && !result.is_empty() {
            // Add underscore before uppercase letters (except at start)
            result.push('_');
        }
        result.push(ch.to_ascii_uppercase());
    }

    result.replace('-', "_")
}

/// Parse directive mappings from c_api.rs directive_name_to_kind() using AST
pub fn parse_directive_mappings() -> Vec<(String, i32)> {
    let c_api = fs::read_to_string("src/c_api.rs").expect("Failed to read c_api.rs");
    let ast: File = syn::parse_file(&c_api).expect("Failed to parse c_api.rs");

    let mut mappings = Vec::new();
    let mut seen_numbers = HashSet::new();

    // Find the directive_name_to_kind function
    for item in &ast.items {
        if let Item::Fn(ItemFn { sig, block, .. }) = item {
            if sig.ident == "directive_name_to_kind" {
                // Recursively find match expressions in the function body
                find_matches_in_stmts(&block.stmts, &mut |arms| {
                    for arm in arms {
                        if let Some((name, num)) = parse_directive_arm(arm) {
                            // Filter out: (1) composite directives with spaces, (2) UNKNOWN_KIND, (3) duplicates
                            // Note: OpenMP spec defines composite directives as having spaces
                            // (e.g., "parallel for", "target teams"). Single-word directives
                            // like "parallel", "for", "target" never contain spaces by definition.
                            // Exclude composite directives with spaces - these require special mapping logic
                            // not yet implemented in the compatibility layer (see module docs).
                            let is_simple_directive = name.split_whitespace().count() == 1;
                            let is_known_kind = num != UNKNOWN_KIND; // Exclude unknown sentinel
                            let is_first_occurrence = seen_numbers.insert(num); // Deduplicate

                            if is_simple_directive && is_known_kind && is_first_occurrence {
                                mappings.push((normalize_constant_name(&name), num));
                            }
                        }
                    }
                });
            }
        }
    }

    mappings.sort_by_key(|(_, num)| *num);
    mappings
}

/// Parse clause mappings from c_api.rs convert_clause() using AST
pub fn parse_clause_mappings() -> Vec<(String, i32)> {
    let c_api = fs::read_to_string("src/c_api.rs").expect("Failed to read c_api.rs");
    let ast: File = syn::parse_file(&c_api).expect("Failed to parse c_api.rs");

    let mut mappings = Vec::new();
    let mut seen_numbers = HashSet::new();

    // Find the convert_clause function
    for item in &ast.items {
        if let Item::Fn(ItemFn { sig, block, .. }) = item {
            if sig.ident == "convert_clause" {
                // Recursively find match expressions in the function body
                find_matches_in_stmts(&block.stmts, &mut |arms| {
                    for arm in arms {
                        if let Some((name, num)) = parse_clause_arm(arm) {
                            // Skip unknown and duplicates
                            if num != UNKNOWN_KIND && seen_numbers.insert(num) {
                                mappings.push((normalize_constant_name(&name), num));
                            }
                        }
                    }
                });
            }
        }
    }

    mappings.sort_by_key(|(_, num)| *num);
    mappings
}

/// Parse OpenACC directive mappings from c_api.rs acc_directive_name_to_kind() using AST
pub fn parse_acc_directive_mappings() -> Vec<(String, i32)> {
    let c_api =
        fs::read_to_string("src/c_api/openacc.rs").expect("Failed to read src/c_api/openacc.rs");
    let ast: File = syn::parse_file(&c_api).expect("Failed to parse c_api.rs");

    let mut mappings = Vec::new();
    let mut seen_numbers = HashSet::new();

    // Find the acc_directive_name_to_kind function
    for item in &ast.items {
        if let Item::Fn(ItemFn { sig, block, .. }) = item {
            if sig.ident == "acc_directive_name_to_kind" {
                // Recursively find match expressions in the function body
                find_matches_in_stmts(&block.stmts, &mut |arms| {
                    for arm in arms {
                        if let Some((name, num)) = parse_directive_arm(arm) {
                            // Filter out: (1) composite directives with spaces, (2) UNKNOWN_KIND, (3) duplicates
                            let is_simple_directive = name.split_whitespace().count() == 1;
                            let is_known_kind = num != UNKNOWN_KIND;
                            let is_first_occurrence = seen_numbers.insert(num);

                            if is_simple_directive && is_known_kind && is_first_occurrence {
                                mappings.push((normalize_constant_name(&name), num));
                            }
                        }
                    }
                });
            }
        }
    }

    mappings.sort_by_key(|(_, num)| *num);
    mappings
}

/// Parse IR enum values and generate Rust const modules + C header defines
///
/// This extracts enum values from IR layer enums and generates:
/// 1. Rust modules (clause_kind, reduction_op, schedule_kind, default_kind, etc.)
/// 2. C header #defines (ROUP_CLAUSE_KIND_*, ROUP_REDUCTION_OP_*, etc.)
pub fn generate_constants_from_ir() -> (
    String,                    // rust_modules
    Vec<(String, i32)>,       // clause_kinds
    Vec<(String, i32)>,       // reduction_ops
    Vec<(String, i32)>,       // schedule_kinds
    Vec<(String, i32)>,       // default_kinds
    Vec<(String, i32)>,       // proc_bind_kinds
    Vec<(String, i32)>,       // reduction_modifiers
    Vec<(String, i32)>,       // if_modifiers
    Vec<(String, i32)>,       // order_modifiers
    Vec<(String, i32)>,       // grainsize_modifiers
    Vec<(String, i32)>,       // num_tasks_modifiers
    Vec<(String, i32)>,       // bind_kinds
    Vec<(String, i32)>,       // at_kinds
    Vec<(String, i32)>,       // severity_kinds
) {
    // Parse IR enums
    let mut reduction_ops = parse_ir_enum("src/ir/clause.rs", "ReductionOperator");
    let schedule_kinds = parse_ir_enum("src/ir/clause.rs", "ScheduleKind");
    let default_kinds = parse_ir_enum("src/ir/clause.rs", "DefaultKind");
    let proc_bind_kinds = parse_ir_enum("src/ir/clause.rs", "ProcBind");

    // Parse new enums added to match OpenMPKinds.h
    let reduction_modifiers = parse_ir_enum("src/ir/clause.rs", "ReductionModifier");
    let if_modifiers = parse_ir_enum("src/ir/clause.rs", "IfModifier");
    let order_modifiers = parse_ir_enum("src/ir/clause.rs", "OrderModifier");
    let grainsize_modifiers = parse_ir_enum("src/ir/clause.rs", "GrainsizeModifier");
    let num_tasks_modifiers = parse_ir_enum("src/ir/clause.rs", "NumTasksModifier");
    let bind_kinds = parse_ir_enum("src/ir/clause.rs", "BindKind");
    let at_kinds = parse_ir_enum("src/ir/clause.rs", "AtKind");
    let severity_kinds = parse_ir_enum("src/ir/clause.rs", "SeverityKind");

    // Add UNKNOWN sentinel for unknown/unsupported reduction operators
    reduction_ops.push(("UNKNOWN".to_string(), 999));

    // Clause kinds are mapped from ClauseData variants (not an enum)
    // Keep existing numbers for backward compatibility
    let clause_kinds = vec![
        ("NUM_THREADS".to_string(), 0),
        ("IF".to_string(), 1),
        ("PRIVATE".to_string(), 2),
        ("SHARED".to_string(), 3),
        ("FIRSTPRIVATE".to_string(), 4),
        ("LASTPRIVATE".to_string(), 5),
        ("REDUCTION".to_string(), 6),
        ("SCHEDULE".to_string(), 7),
        ("COLLAPSE".to_string(), 8),
        ("ORDERED".to_string(), 9),
        ("NOWAIT".to_string(), 10),
        ("DEFAULT".to_string(), 11),
        ("COPYIN".to_string(), 12),
        ("PROC_BIND".to_string(), 13),
        // New clause kinds (14+)
        ("LINEAR".to_string(), 14),
        ("ALIGNED".to_string(), 15),
        ("SAFELEN".to_string(), 16),
        ("SIMDLEN".to_string(), 17),
        ("NONTEMPORAL".to_string(), 18),
        ("DIST_SCHEDULE".to_string(), 19),
        ("NUM_TEAMS".to_string(), 20),
        ("THREAD_LIMIT".to_string(), 21),
        ("GRAINSIZE".to_string(), 22),
        ("NUM_TASKS".to_string(), 23),
        ("COPYPRIVATE".to_string(), 24),
        ("FILTER".to_string(), 25),
        ("PRIORITY".to_string(), 26),
        ("DEVICE".to_string(), 27),
        ("MAP".to_string(), 28),
        ("DEPEND".to_string(), 29),
        ("USE_DEVICE_PTR".to_string(), 30),
        ("USE_DEVICE_ADDR".to_string(), 31),
        ("IS_DEVICE_PTR".to_string(), 32),
        ("HAS_DEVICE_ADDR".to_string(), 33),
        ("AFFINITY".to_string(), 34),
        ("ALLOCATE".to_string(), 35),
        ("ALLOCATOR".to_string(), 36),
        ("ATOMIC_OPERATION".to_string(), 37),
        ("ORDER".to_string(), 38),
        ("BIND".to_string(), 39),
        ("HINT".to_string(), 40),
        ("ALIGN".to_string(), 41),
        ("SEQ_CST".to_string(), 42),
        ("ACQ_REL".to_string(), 43),
        ("RELEASE".to_string(), 44),
        ("ACQUIRE".to_string(), 45),
        ("RELAXED".to_string(), 46),
        ("READ".to_string(), 47),
        ("WRITE".to_string(), 48),
        ("UPDATE".to_string(), 49),
        ("CAPTURE".to_string(), 50),
        ("COMPARE".to_string(), 51),
        ("GENERIC".to_string(), 900),
        ("UNKNOWN".to_string(), 999),
    ];

    // Generate Rust module code
    let rust_modules = format!(
        r#"// Auto-generated constant modules - DO NOT EDIT
// Generated from IR enums by build.rs

pub mod clause_kind {{
{}
}}

pub mod reduction_op {{
{}
}}

pub mod schedule_kind {{
{}
}}

pub mod default_kind {{
{}
}}

pub mod proc_bind_kind {{
{}
}}

pub mod reduction_modifier {{
{}
}}

pub mod if_modifier {{
{}
}}

pub mod order_modifier {{
{}
}}

pub mod grainsize_modifier {{
{}
}}

pub mod num_tasks_modifier {{
{}
}}

pub mod bind_kind {{
{}
}}

pub mod at_kind {{
{}
}}

pub mod severity_kind {{
{}
}}
"#,
        clause_kinds
            .iter()
            .map(|(name, val)| format!("    pub const {}: i32 = {};", name, val))
            .collect::<Vec<_>>()
            .join("\n"),
        reduction_ops
            .iter()
            .map(|(name, val)| format!("    pub const {}: i32 = {};", name, val))
            .collect::<Vec<_>>()
            .join("\n"),
        schedule_kinds
            .iter()
            .map(|(name, val)| format!("    pub const {}: i32 = {};", name, val))
            .collect::<Vec<_>>()
            .join("\n"),
        default_kinds
            .iter()
            .map(|(name, val)| format!("    pub const {}: i32 = {};", name, val))
            .collect::<Vec<_>>()
            .join("\n"),
        proc_bind_kinds
            .iter()
            .map(|(name, val)| format!("    pub const {}: i32 = {};", name, val))
            .collect::<Vec<_>>()
            .join("\n"),
        reduction_modifiers
            .iter()
            .map(|(name, val)| format!("    pub const {}: i32 = {};", name, val))
            .collect::<Vec<_>>()
            .join("\n"),
        if_modifiers
            .iter()
            .map(|(name, val)| format!("    pub const {}: i32 = {};", name, val))
            .collect::<Vec<_>>()
            .join("\n"),
        order_modifiers
            .iter()
            .map(|(name, val)| format!("    pub const {}: i32 = {};", name, val))
            .collect::<Vec<_>>()
            .join("\n"),
        grainsize_modifiers
            .iter()
            .map(|(name, val)| format!("    pub const {}: i32 = {};", name, val))
            .collect::<Vec<_>>()
            .join("\n"),
        num_tasks_modifiers
            .iter()
            .map(|(name, val)| format!("    pub const {}: i32 = {};", name, val))
            .collect::<Vec<_>>()
            .join("\n"),
        bind_kinds
            .iter()
            .map(|(name, val)| format!("    pub const {}: i32 = {};", name, val))
            .collect::<Vec<_>>()
            .join("\n"),
        at_kinds
            .iter()
            .map(|(name, val)| format!("    pub const {}: i32 = {};", name, val))
            .collect::<Vec<_>>()
            .join("\n"),
        severity_kinds
            .iter()
            .map(|(name, val)| format!("    pub const {}: i32 = {};", name, val))
            .collect::<Vec<_>>()
            .join("\n")
    );

    (
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
    )
}

/// Parse an enum from a Rust source file
fn parse_ir_enum(file_path: &str, enum_name: &str) -> Vec<(String, i32)> {
    let source = fs::read_to_string(file_path)
        .unwrap_or_else(|_| panic!("Failed to read {}", file_path));
    let ast: File = syn::parse_file(&source)
        .unwrap_or_else(|_| panic!("Failed to parse {}", file_path));

    let mut mappings = Vec::new();

    for item in &ast.items {
        if let Item::Enum(item_enum) = item {
            if item_enum.ident == enum_name {
                for variant in &item_enum.variants {
                    let name = normalize_constant_name(&variant.ident.to_string());

                    if let Some((_, expr)) = &variant.discriminant {
                        if let Expr::Lit(ExprLit { lit: Lit::Int(lit_int), .. }) = expr {
                            if let Ok(value) = lit_int.base10_parse::<i32>() {
                                mappings.push((name, value));
                            }
                        }
                    }
                }
            }
        }
    }

    mappings.sort_by_key(|(_, num)| *num);
    mappings
}

/// Parse DirectiveKind enum values from src/ir/directive.rs
pub fn parse_directive_kind_enum() -> Vec<(String, i32)> {
    let source = fs::read_to_string("src/ir/directive.rs")
        .expect("Failed to read src/ir/directive.rs");
    let ast: File = syn::parse_file(&source).expect("Failed to parse directive.rs");

    let mut mappings = Vec::new();

    // Find the DirectiveKind enum
    for item in &ast.items {
        if let Item::Enum(item_enum) = item {
            if item_enum.ident == "DirectiveKind" {
                // Parse each enum variant
                for variant in &item_enum.variants {
                    let name = variant.ident.to_string();

                    // Extract the discriminant value (the = N part)
                    if let Some((_, expr)) = &variant.discriminant {
                        if let Expr::Lit(ExprLit { lit: Lit::Int(lit_int), .. }) = expr {
                            if let Ok(value) = lit_int.base10_parse::<i32>() {
                                mappings.push((normalize_constant_name(&name), value));
                            }
                        }
                    }
                }
            }
        }
    }

    mappings.sort_by_key(|(_, num)| *num);
    mappings
}

/// Parse OpenACC clause mappings from c_api.rs convert_acc_clause() using AST
pub fn parse_acc_clause_mappings() -> Vec<(String, i32)> {
    let source =
        fs::read_to_string("src/c_api/openacc.rs").expect("Failed to read src/c_api/openacc.rs");

    // Extract the body of clause_name_to_kind()
    let fn_pos = source
        .find("fn clause_name_to_kind")
        .expect("clause_name_to_kind() not found in openacc module");
    let body_start = source[fn_pos..]
        .find('{')
        .map(|idx| fn_pos + idx + 1)
        .expect("Failed to locate clause_name_to_kind body");

    let mut depth = 1usize;
    let mut idx = body_start;
    let bytes = source.as_bytes();
    while idx < bytes.len() && depth > 0 {
        match bytes[idx] as char {
            '{' => depth += 1,
            '}' => depth -= 1,
            _ => {}
        }
        idx += 1;
    }
    let body = &source[body_start..idx.saturating_sub(1)];

    let mut mappings = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('"') {
            continue;
        }

        if let Some((name_part, rest)) = trimmed[1..].split_once('"') {
            if let Some((_, value_part)) = rest.split_once("=>") {
                if let Some(num_str) = value_part.split(',').next() {
                    if let Ok(value) = num_str.trim().parse::<i32>() {
                        if value != UNKNOWN_KIND {
                            mappings.push((normalize_constant_name(name_part), value));
                        }
                    }
                }
            }
        }
    }

    mappings.sort_by_key(|(_, num)| *num);
    mappings
}

/// Calculate FNV-1a hash checksum of directive and clause mappings (single-API version).
///
/// **Status**: No longer used by build.rs (superseded by `calculate_combined_checksum`).
/// Retained for:
/// - API stability (external tools may depend on this function)
/// - Potential single-API verification use cases (OpenMP-only or OpenACC-only)
///
/// Returns a 64-bit FNV-1a hash of directive and clause mappings for verification.
/// See module documentation for algorithm rationale.
#[allow(dead_code)] // Intentionally unused - kept for API stability
pub fn calculate_checksum(directives: &[(String, i32)], clauses: &[(String, i32)]) -> u64 {
    let mut hash: u64 = FNV_OFFSET_BASIS;

    // Hash directive names and numbers
    for (name, num) in directives {
        for byte in name.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash ^= *num as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    // Hash clause names and numbers
    for (name, num) in clauses {
        for byte in name.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash ^= *num as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    hash
}

/// Calculate combined FNV-1a hash checksum of OpenMP and OpenACC mappings.
///
/// Used to verify the generated header matches c_api.rs for both APIs.
pub fn calculate_combined_checksum(
    omp_directives: &[(String, i32)],
    omp_clauses: &[(String, i32)],
    acc_directives: &[(String, i32)],
    acc_clauses: &[(String, i32)],
) -> u64 {
    let mut hash: u64 = FNV_OFFSET_BASIS;

    // Hash OpenMP directive names and numbers
    for (name, num) in omp_directives {
        for byte in name.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash ^= *num as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    // Hash OpenMP clause names and numbers
    for (name, num) in omp_clauses {
        for byte in name.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash ^= *num as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    // Hash OpenACC directive names and numbers
    for (name, num) in acc_directives {
        for byte in name.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash ^= *num as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    // Hash OpenACC clause names and numbers
    for (name, num) in acc_clauses {
        for byte in name.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash ^= *num as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    hash
}

/// Extract checksum from a generated header file
pub fn extract_checksum_from_header(header_content: &str) -> Option<u64> {
    // Look for hex format: #define ROUP_CONSTANTS_CHECKSUM 0xNNNNNNNNNNNNNNNN
    if let Some(line) = header_content
        .lines()
        .find(|l| l.contains("ROUP_CONSTANTS_CHECKSUM"))
    {
        // Extract hex value after "0x"
        if let Some(hex_start) = line.find("0x") {
            let hex_str = &line[hex_start + 2..].trim();
            // Take only the hex digits
            let hex_digits: String = hex_str
                .chars()
                .take_while(|c| c.is_ascii_hexdigit())
                .collect();
            return u64::from_str_radix(&hex_digits, 16).ok();
        }
    }
    None
}

// ============================================================================
// Internal helper functions for AST parsing
// ============================================================================

/// Recursively search for match expressions in statements
fn find_matches_in_stmts<F>(stmts: &[syn::Stmt], callback: &mut F)
where
    F: FnMut(&[Arm]),
{
    for stmt in stmts {
        match stmt {
            syn::Stmt::Expr(expr, _) => {
                find_matches_in_expr(expr, callback);
            }
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    find_matches_in_expr(&init.expr, callback);
                }
            }
            _ => {}
        }
    }
}

/// Recursively search for match expressions in an expression
fn find_matches_in_expr<F>(expr: &Expr, callback: &mut F)
where
    F: FnMut(&[Arm]),
{
    match expr {
        Expr::Match(ExprMatch { arms, .. }) => {
            callback(arms);
        }
        Expr::Unsafe(unsafe_block) => {
            find_matches_in_stmts(&unsafe_block.block.stmts, callback);
        }
        Expr::Block(block) => {
            find_matches_in_stmts(&block.block.stmts, callback);
        }
        _ => {}
    }
}

/// Extract (name, number) from a match arm like: "directive-name" => number,
fn parse_directive_arm(arm: &Arm) -> Option<(String, i32)> {
    // Extract pattern (the "directive-name" part)
    let name = if let Pat::Lit(PatLit {
        lit: Lit::Str(lit_str),
        ..
    }) = &arm.pat
    {
        lit_str.value()
    } else {
        return None;
    };

    // Extract number from body (the number part after =>)
    let num = if let Expr::Lit(ExprLit {
        lit: Lit::Int(lit_int),
        ..
    }) = &*arm.body
    {
        lit_int.base10_parse::<i32>().ok()?
    } else {
        return None;
    };

    Some((name, num))
}

/// Extract (name, number) from a match arm like: "clause-name" => (number, ClauseData)
fn parse_clause_arm(arm: &Arm) -> Option<(String, i32)> {
    // Extract pattern (the "clause-name" part)
    let name = if let Pat::Lit(PatLit {
        lit: Lit::Str(lit_str),
        ..
    }) = &arm.pat
    {
        lit_str.value()
    } else {
        return None;
    };

    // Extract number from tuple body: (number, ClauseData)
    // The body could be a tuple directly, or a block containing a tuple
    let num = match &*arm.body {
        // Direct tuple: => (6, ClauseData::...)
        Expr::Tuple(ExprTuple { elems, .. }) => {
            if let Some(Expr::Lit(ExprLit {
                lit: Lit::Int(lit_int),
                ..
            })) = elems.first()
            {
                lit_int.base10_parse::<i32>().ok()?
            } else {
                return None;
            }
        }
        // Block containing tuple: => { ... (6, ClauseData::...) }
        Expr::Block(block) => {
            for stmt in &block.block.stmts {
                if let syn::Stmt::Expr(Expr::Tuple(ExprTuple { elems, .. }), _) = stmt {
                    if let Some(Expr::Lit(ExprLit {
                        lit: Lit::Int(lit_int),
                        ..
                    })) = elems.first()
                    {
                        return Some((name, lit_int.base10_parse::<i32>().ok()?));
                    }
                }
            }
            return None;
        }
        _ => return None,
    };

    Some((name, num))
}
