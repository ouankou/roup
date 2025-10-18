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

/// Normalize constant name: uppercase and replace hyphens with underscores
fn normalize_constant_name(name: &str) -> String {
    name.to_uppercase().replace('-', "_")
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
    let c_api = fs::read_to_string("src/c_api.rs").expect("Failed to read c_api.rs");
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

/// Parse OpenACC clause mappings from c_api.rs convert_acc_clause() using AST
pub fn parse_acc_clause_mappings() -> Vec<(String, i32)> {
    let c_api = fs::read_to_string("src/c_api.rs").expect("Failed to read c_api.rs");
    let ast: File = syn::parse_file(&c_api).expect("Failed to parse c_api.rs");

    let mut mappings = Vec::new();
    let mut seen_numbers = HashSet::new();

    // Find the convert_acc_clause function
    for item in &ast.items {
        if let Item::Fn(ItemFn { sig, block, .. }) = item {
            if sig.ident == "convert_acc_clause" {
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
