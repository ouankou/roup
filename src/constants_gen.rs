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

use std::collections::{HashMap, HashSet};
use std::fs;

use syn::{
    Arm, Expr, ExprCast, ExprLit, ExprMatch, ExprPath, ExprTuple, Fields, File, Item, ItemEnum,
    ItemFn, Lit, Pat, PatLit, PatOr, Variant,
};

/// Special value for unknown directive/clause kinds
pub const UNKNOWN_KIND: i32 = 999;

/// FNV-1a hash algorithm constants
const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

/// Normalize constant name: uppercase and replace hyphens with underscores
fn normalize_constant_name(name: &str) -> String {
    name.to_uppercase().replace('-', "_")
}

/// Parse enum definition and extract variant name -> discriminant mapping
fn parse_enum_discriminants(ast: &File, enum_name: &str) -> HashMap<String, i32> {
    let mut map = HashMap::new();

    for item in &ast.items {
        if let Item::Enum(ItemEnum {
            ident, variants, ..
        }) = item
        {
            if ident == enum_name {
                for variant in variants {
                    if let Some(discriminant) = extract_discriminant(variant) {
                        map.insert(variant.ident.to_string(), discriminant);
                    }
                }
                break;
            }
        }
    }

    map
}

/// Extract discriminant value from an enum variant
fn extract_discriminant(variant: &Variant) -> Option<i32> {
    if let Fields::Unit = &variant.fields {
        if let Some((
            _,
            Expr::Lit(ExprLit {
                lit: Lit::Int(lit_int),
                ..
            }),
        )) = &variant.discriminant
        {
            return lit_int.base10_parse::<i32>().ok();
        }
    }
    None
}

/// Parse directive mappings from c_api.rs directive_name_to_kind() using AST
pub fn parse_directive_mappings() -> Vec<(String, i32)> {
    let c_api = fs::read_to_string("src/c_api.rs").expect("Failed to read c_api.rs");
    let ast: File = syn::parse_file(&c_api).expect("Failed to parse c_api.rs");

    // Parse DirectiveKindC enum to get variant -> discriminant mapping
    let enum_map = parse_enum_discriminants(&ast, "DirectiveKindC");

    let mut mappings = Vec::new();
    let mut seen_numbers = HashSet::new();

    // Find the directive_name_to_kind function
    for item in &ast.items {
        if let Item::Fn(ItemFn { sig, block, .. }) = item {
            if sig.ident == "directive_name_to_kind" {
                // Recursively find match expressions in the function body
                find_matches_in_stmts(&block.stmts, &mut |arms| {
                    for arm in arms {
                        if let Some((name, num)) = parse_directive_arm(arm, &enum_map) {
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

    // Parse ClauseKindC enum to get variant -> discriminant mapping
    let enum_map = parse_enum_discriminants(&ast, "ClauseKindC");

    let mut mappings = Vec::new();
    let mut seen_numbers = HashSet::new();

    // Find the convert_clause function
    for item in &ast.items {
        if let Item::Fn(ItemFn { sig, block, .. }) = item {
            if sig.ident == "convert_clause" {
                // Recursively find match expressions in the function body
                find_matches_in_stmts(&block.stmts, &mut |arms| {
                    for arm in arms {
                        if let Some((name, num)) = parse_clause_arm(arm, &enum_map) {
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

    // Parse AccDirectiveKindC enum to get variant -> discriminant mapping
    let enum_map = parse_enum_discriminants(&ast, "AccDirectiveKindC");

    let mut mappings = Vec::new();
    let mut seen_numbers = HashSet::new();

    // Find the acc_directive_name_to_kind function
    for item in &ast.items {
        if let Item::Fn(ItemFn { sig, block, .. }) = item {
            if sig.ident == "acc_directive_name_to_kind" {
                // Recursively find match expressions in the function body
                find_matches_in_stmts(&block.stmts, &mut |arms| {
                    for arm in arms {
                        if let Some((name, num)) = parse_directive_arm(arm, &enum_map) {
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

/// Parse OpenACC clause mappings from c_api.rs convert_acc_clause() using AST
pub fn parse_acc_clause_mappings() -> Vec<(String, i32)> {
    let c_api =
        fs::read_to_string("src/c_api/openacc.rs").expect("Failed to read src/c_api/openacc.rs");
    let ast: File = syn::parse_file(&c_api).expect("Failed to parse openacc.rs");

    // Parse AccClauseKindC enum to get variant -> discriminant mapping
    let enum_map = parse_enum_discriminants(&ast, "AccClauseKindC");

    let mut mappings = Vec::new();
    let mut seen_numbers = HashSet::new();

    // Find the clause_name_to_kind function
    for item in &ast.items {
        if let Item::Fn(ItemFn { sig, block, .. }) = item {
            if sig.ident == "clause_name_to_kind" {
                // Recursively find match expressions in the function body
                find_matches_in_stmts(&block.stmts, &mut |arms| {
                    for arm in arms {
                        if let Some((name, num)) = parse_directive_arm(arm, &enum_map) {
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

/// Extract (name, number) from a match arm like: "directive-name" => DirectiveKindC::Parallel,
/// Also handles multi-pattern matches like: "copy" | "pcopy" => AccClauseKindC::Copy
fn parse_directive_arm(arm: &Arm, enum_map: &HashMap<String, i32>) -> Option<(String, i32)> {
    // Extract pattern (the "directive-name" part)
    // Handle both single patterns and multi-pattern OR matches
    let name = match &arm.pat {
        // Single string literal: "directive-name"
        Pat::Lit(PatLit {
            lit: Lit::Str(lit_str),
            ..
        }) => lit_str.value(),

        // Multi-pattern OR: "copy" | "pcopy" | "present_or_copy"
        // We take the first pattern as the canonical name
        Pat::Or(PatOr { cases, .. }) => {
            if let Some(Pat::Lit(PatLit {
                lit: Lit::Str(lit_str),
                ..
            })) = cases.first()
            {
                lit_str.value()
            } else {
                return None;
            }
        }

        _ => return None,
    };

    // Extract number from body - now handles enum paths like DirectiveKindC::Parallel
    let num = match &*arm.body {
        // Direct integer literal: => 0 (old style, for backwards compatibility)
        Expr::Lit(ExprLit {
            lit: Lit::Int(lit_int),
            ..
        }) => lit_int.base10_parse::<i32>().ok()?,

        // Enum path: => DirectiveKindC::Parallel
        Expr::Path(ExprPath { path, .. }) => {
            // Get the last segment (variant name) from the path
            if let Some(segment) = path.segments.last() {
                let variant_name = segment.ident.to_string();
                *enum_map.get(&variant_name)?
            } else {
                return None;
            }
        }

        _ => return None,
    };

    Some((name, num))
}

/// Extract (name, number) from a match arm like: "clause-name" => (ClauseKindC::NumThreads, ClauseData)
/// Also handles multi-pattern matches like: "copy" | "pcopy" => (AccClauseKindC::Copy, ...)
fn parse_clause_arm(arm: &Arm, enum_map: &HashMap<String, i32>) -> Option<(String, i32)> {
    // Extract pattern (the "clause-name" part)
    // Handle both single patterns and multi-pattern OR matches
    let name = match &arm.pat {
        // Single string literal: "clause-name"
        Pat::Lit(PatLit {
            lit: Lit::Str(lit_str),
            ..
        }) => lit_str.value(),

        // Multi-pattern OR: "copy" | "pcopy" | "present_or_copy"
        // We take the first pattern as the canonical name
        Pat::Or(PatOr { cases, .. }) => {
            if let Some(Pat::Lit(PatLit {
                lit: Lit::Str(lit_str),
                ..
            })) = cases.first()
            {
                lit_str.value()
            } else {
                return None;
            }
        }

        _ => return None,
    };

    // Extract number from tuple body: (ClauseKindC::NumThreads, ClauseData) or (6, ClauseData)
    // The body could be a tuple directly, or a block containing a tuple
    let num = match &*arm.body {
        // Direct tuple: => (ClauseKindC::NumThreads, ClauseData::...)
        Expr::Tuple(ExprTuple { elems, .. }) => {
            if let Some(first_elem) = elems.first() {
                extract_number_from_expr(first_elem, enum_map)?
            } else {
                return None;
            }
        }
        // Block containing tuple: => { ... (ClauseKindC::NumThreads, ClauseData::...) }
        Expr::Block(block) => {
            for stmt in &block.block.stmts {
                if let syn::Stmt::Expr(Expr::Tuple(ExprTuple { elems, .. }), _) = stmt {
                    if let Some(first_elem) = elems.first() {
                        return Some((name, extract_number_from_expr(first_elem, enum_map)?));
                    }
                }
            }
            return None;
        }
        _ => return None,
    };

    Some((name, num))
}

/// Helper to extract integer from either a literal or an enum path
fn extract_number_from_expr(expr: &Expr, enum_map: &HashMap<String, i32>) -> Option<i32> {
    match expr {
        // Direct integer literal: 6
        Expr::Lit(ExprLit {
            lit: Lit::Int(lit_int),
            ..
        }) => lit_int.base10_parse::<i32>().ok(),

        // Enum path: ClauseKindC::NumThreads
        Expr::Path(ExprPath { path, .. }) => {
            if let Some(segment) = path.segments.last() {
                let variant_name = segment.ident.to_string();
                enum_map.get(&variant_name).copied()
            } else {
                None
            }
        }

        // Cast expression: ClauseKindC::NumThreads as i32
        Expr::Cast(ExprCast { expr, .. }) => extract_number_from_expr(expr, enum_map),

        _ => None,
    }
}
