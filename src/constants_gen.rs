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

use syn::{Arm, Expr, ExprLit, ExprMatch, ExprTuple, File, Item, ItemFn, Lit, Pat};

/// Special value for unknown directive/clause kinds
///
/// Historically this module used 999 as the unknown sentinel. The project
/// now standardizes on -1 for unknown directive/clause sentinel values so
/// that runtime mappings and generated headers are consistent.
pub const UNKNOWN_KIND: i32 = -1;

/// FNV-1a hash algorithm constants
const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

// Normalize constant name: uppercase and replace hyphens with underscores
// normalize_constant_name removed; generator uses enum-variant -> constant naming
/// Convert CamelCase variant name to UPPER_SNAKE constant name
fn variant_to_constant(variant: &str) -> String {
    let mut out = String::new();
    for (i, ch) in variant.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if i != 0 {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out.to_uppercase()
}

/// Parse directive mappings from c_api.rs directive_name_to_kind() using AST
pub fn parse_directive_mappings() -> Vec<(String, i32)> {
    // Reuse the raw extractor but keep dedup behavior for the canonical
    // OpenMP mapping. The raw extractor returns all enum variants -> numbers
    // (including duplicates where multiple variants map to the same code).
    let raw = parse_directive_enum_raw_mappings();

    if raw.is_empty() {
        panic!("No enum-based directive mapping found: directive_name_enum_to_kind not present in src/c_api.rs");
    }

    let mut mappings = Vec::new();
    let mut seen = HashSet::new();
    for (variant, num) in raw {
        if num == UNKNOWN_KIND || !seen.insert(num) {
            continue;
        }
        mappings.push((variant_to_constant(&variant), num));
    }

    mappings.sort_by_key(|(_, num)| *num);
    mappings
}

/// Raw extractor for directive enum arms: returns all (variant, num) pairs
/// without deduping numeric codes. Useful for compatibility layers that
/// expect alternate variant names (e.g., Loop vs For) to be defined as macros.
fn parse_directive_enum_raw_mappings() -> Vec<(String, i32)> {
    let c_api = fs::read_to_string("src/c_api.rs").expect("Failed to read c_api.rs");
    let ast: File = syn::parse_file(&c_api).expect("Failed to parse c_api.rs");

    let mut enum_mappings: Vec<(String, i32)> = Vec::new();
    for item in &ast.items {
        if let Item::Fn(ItemFn { sig, block, .. }) = item {
            if sig.ident == "directive_name_enum_to_kind" {
                find_matches_in_stmts(&block.stmts, &mut |arms| {
                    for arm in arms {
                        for (variant_name, num) in parse_enum_directive_arm(arm) {
                            enum_mappings.push((variant_name, num));
                        }
                    }
                });
            }
        }
    }

    enum_mappings
}

/// Parse clause mappings from c_api.rs convert_clause() using AST
pub fn parse_clause_mappings() -> Vec<(String, i32)> {
    let c_api = fs::read_to_string("src/c_api.rs").expect("Failed to read c_api.rs");
    let ast: File = syn::parse_file(&c_api).expect("Failed to parse c_api.rs");

    let mut mappings = Vec::new();
    let mut seen_numbers: HashSet<i32> = HashSet::new();

    // Find the convert_clause function (AST-only enum-patterns)
    for item in &ast.items {
        if let Item::Fn(ItemFn { sig, block, .. }) = item {
            if sig.ident == "convert_clause" {
                // Recursively find match expressions in the function body
                find_matches_in_stmts(&block.stmts, &mut |arms| {
                    for arm in arms {
                        // AST-only: handle enum-pattern arms mapping ClauseName::Variant => num
                        for (variant, num) in parse_enum_clause_arm(arm) {
                            if num != UNKNOWN_KIND && seen_numbers.insert(num) {
                                let const_name = variant_to_constant(&variant);
                                mappings.push((const_name, num));
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
    // Find an enum-based mapping in acc_directive_name_to_kind.
    // This generator is enum-only; legacy string-based mappings are not supported.
    let mut enum_mappings: Vec<(String, i32)> = Vec::new();
    for item in &ast.items {
        if let Item::Fn(ItemFn { sig, block, .. }) = item {
            if sig.ident == "acc_directive_name_to_kind" {
                find_matches_in_stmts(&block.stmts, &mut |arms| {
                    for arm in arms {
                        for (variant, num) in parse_enum_directive_arm(arm) {
                            enum_mappings.push((variant, num));
                        }
                    }
                });
            }
        }
    }

    if enum_mappings.is_empty() {
        // OpenACC may reuse the canonical DirectiveName -> kind mapping from
        // the parent module (src/c_api.rs). In that case, there's no
        // `acc_directive_name_to_kind` in `openacc.rs`.
        // Use the raw directive enum extractor to emit all variant names
        // (AST-only) so compatibility layers that expect alternate macro
        // names (like ACC_DIRECTIVE_LOOP) get defined.
        let raw = parse_directive_enum_raw_mappings();
        for (variant, num) in raw {
            if num == UNKNOWN_KIND {
                continue;
            }
            mappings.push((variant_to_constant(&variant), num));
        }
        mappings.sort_by_key(|(_, num)| *num);
        return mappings;
    }

    // Build a map from generated name -> num for quick lookup
    let mut gen_map = std::collections::HashMap::new();
    for (variant, num) in &enum_mappings {
        gen_map.insert(variant_to_constant(variant), *num);
    }

    // List of ACC_DIRECTIVE_* identifiers expected by compat/accparser
    // This list focuses on the directive names used by the compatibility layer
    // and mirrors the identifiers referenced in compat/accparser/src/compat_impl.cpp
    let expected = vec![
        "PARALLEL",
        "LOOP",
        "KERNELS",
        "DATA",
        "ENTER_DATA",
        "EXIT_DATA",
        "HOST_DATA",
        "ATOMIC",
        "DECLARE",
        "WAIT",
        "END",
        "UPDATE",
        "KERNELS_LOOP",
        "PARALLEL_LOOP",
        "SERIAL_LOOP",
        "SERIAL",
        "ROUTINE",
        "SET",
        "INIT",
        "SHUTDOWN",
        "CACHE",
        "TARGET",
        "TARGET_TEAMS",
        "TEAMS",
        "DISTRIBUTE",
        "METADIRECTIVE",
    ];

    // Known alias mapping from expected compat names -> generated variant names
    // (in case the AST variant uses a slightly different token)
    let alias_map = vec![
        ("ENTER_DATA", "ENTER_DATA"),
        ("EXIT_DATA", "EXIT_DATA"),
        ("HOST_DATA", "HOST_DATA"),
        ("KERNELS_LOOP", "KernelsLoop"),
    ]
    .into_iter()
    .collect::<std::collections::HashMap<_, _>>();

    let mut final_mappings: Vec<(String, i32)> = Vec::new();
    // First emit the expected compat list in fixed order using available AST values
    for key in &expected {
        if let Some(&v) = gen_map.get(*key) {
            final_mappings.push((key.to_string(), v));
            continue;
        }
        if let Some(canon) = alias_map.get(*key) {
            // alias_map stores raw variant names; convert to constant form
            let canon_const = variant_to_constant(canon);
            if let Some(&v) = gen_map.get(&canon_const) {
                final_mappings.push((key.to_string(), v));
                continue;
            }
        }
        // Not present in AST-derived map; emit as UNKNOWN_KIND to keep compat builds compiling
        final_mappings.push((key.to_string(), UNKNOWN_KIND));
    }

    // Also append any generator-discovered directive names that weren't on the expected list
    for (name, num) in enum_mappings {
        let const_name = variant_to_constant(&name);
        if !final_mappings.iter().any(|(n, _)| n == &const_name) {
            final_mappings.push((const_name, num));
        }
    }

    // Preserve expected order for first entries; otherwise sort remaining by numeric value
    final_mappings.sort_by(|(a_name, a_num), (b_name, b_num)| {
        let a_expected = expected.contains(&a_name.as_str());
        let b_expected = expected.contains(&b_name.as_str());
        match (a_expected, b_expected) {
            (true, true) | (false, false) => a_num.cmp(b_num),
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
        }
    });

    // Fail fast if any expected compat identifiers are missing in the AST-derived map.
    // The user requested there be no post-parse string heuristics and that any
    // unknown keyword should produce a fatal error during header generation so
    // the issue is fixed at compile time instead of silently assigned placeholders.
    check_no_unknowns(&final_mappings, "OpenACC directive (ACC_DIRECTIVE_*)");

    final_mappings
}

/// Parse OpenACC clause mappings from c_api.rs convert_acc_clause() using AST
pub fn parse_acc_clause_mappings() -> Vec<(String, i32)> {
    let c_api = fs::read_to_string("src/c_api/openacc.rs").expect("Failed to read openacc c_api");
    let ast: File = syn::parse_file(&c_api).expect("Failed to parse src/c_api/openacc.rs");

    let mut mappings = Vec::new();

    // AST-only: find clause_name_to_kind and extract enum arms
    for item in &ast.items {
        if let Item::Fn(ItemFn { sig, block, .. }) = item {
            if sig.ident == "clause_name_to_kind" {
                find_matches_in_stmts(&block.stmts, &mut |arms| {
                    for arm in arms {
                        for (variant, num) in parse_enum_clause_arm(arm) {
                            if num != UNKNOWN_KIND {
                                mappings.push((variant_to_constant(&variant), num));
                            }
                        }
                    }
                });
            }
        }
    }

    // Build a map from generated name -> num for quick lookup
    let mut gen_map = std::collections::HashMap::new();
    for (name, num) in &mappings {
        gen_map.insert(name.clone(), *num);
    }

    // List of ACC_CLAUSE_* identifiers expected by compat/accparser
    // This expected list is authoritative for the compatibility header
    // generation and is validated by a unit test (tests/acc_clause_macros.rs)
    // which asserts that `src/roup_constants.h` contains each
    // `ACC_CLAUSE_{NAME}` macro and that none are left as UNKNOWN_KIND.
    let expected = vec![
        "ASYNC",
        "WAIT",
        "NUM_GANGS",
        "NUM_WORKERS",
        "VECTOR_LENGTH",
        "GANG",
        "WORKER",
        "VECTOR",
        "SEQ",
        "INDEPENDENT",
        "AUTO",
        "COLLAPSE",
        "DEVICE_TYPE",
        "BIND",
        "IF",
        "DEFAULT",
        "FIRSTPRIVATE",
        "DEFAULT_ASYNC",
        "LINK",
        "NO_CREATE",
        "NOHOST",
        "PRESENT",
        "PRIVATE",
        "REDUCTION",
        "READ",
        "SELF",
        "TILE",
        "USE_DEVICE",
        "ATTACH",
        "DETACH",
        "FINALIZE",
        "IF_PRESENT",
        "CAPTURE",
        "WRITE",
        "UPDATE",
        "COPY",
        "COPYIN",
        "COPYOUT",
        "CREATE",
        "DELETE",
        "DEVICE",
        "DEVICEPTR",
        "DEVICE_NUM",
        "DEVICE_RESIDENT",
        "HOST",
        // include some canonical OMP names that OpenACC also uses
        "NUM_THREADS",
    ];

    // Known alias mapping from expected compat names -> generated variant names
    // Note: do NOT alias PRESENT to SHARED — they are distinct in OpenACC.
    let alias_map = vec![
        ("COPYIN", "COPY_IN"),
        ("COPYOUT", "COPY_OUT"),
        ("DEVICE_NUM", "DEVICE_NUM"),
        // Map compat names without underscores to enum variant names used in parser
        ("DEVICEPTR", "DEVICE_PTR"),
        ("NOHOST", "NO_HOST"),
        ("SELF", "SELF_CLAUSE"),
    ]
    .into_iter()
    .collect::<std::collections::HashMap<_, _>>();

    let mut final_mappings: Vec<(String, i32)> = Vec::new();
    // First emit the expected compat list in fixed order using available AST values
    for key in &expected {
        if let Some(&v) = gen_map.get(*key) {
            final_mappings.push((key.to_string(), v));
            continue;
        }
        if let Some(canon) = alias_map.get(*key) {
            if let Some(&v) = gen_map.get(*canon) {
                final_mappings.push((key.to_string(), v));
                continue;
            }
        }
        // Not present in AST-derived map; emit as UNKNOWN_KIND to keep compat builds compiling
        final_mappings.push((key.to_string(), UNKNOWN_KIND));
    }

    // Also append any generator-discovered clause names that weren't on the expected list
    for (name, num) in mappings {
        if !final_mappings.iter().any(|(n, _)| n == &name) {
            final_mappings.push((name, num));
        }
    }

    // Sort by numeric value for deterministic output where possible, but keep
    // expected list order for the ones we explicitly added above by assigning
    // a stable sort key: expected entries have priority; others sorted by num.
    final_mappings.sort_by(|(a_name, a_num), (b_name, b_num)| {
        let a_expected = expected.contains(&a_name.as_str());
        let b_expected = expected.contains(&b_name.as_str());
        match (a_expected, b_expected) {
            (true, true) | (false, false) => a_num.cmp(b_num),
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
        }
    });

    // Fail fast if any expected compat clause identifiers are missing in the AST-derived map.
    check_no_unknowns(&final_mappings, "OpenACC clause (ACC_CLAUSE_*)");

    final_mappings
}

/// Assign unique numeric placeholders for entries that were emitted as UNKNOWN_KIND.
/// This prevents multiple ACC_* macros from expanding to the identical value (e.g. 999)
/// which would cause duplicate case labels when used in C/C++ switch statements.
/// Fail the build if any mapping entries are still UNKNOWN_KIND.
///
/// This enforces the invariant that the AST must exhaustively declare all
/// ACC macros expected by the compatibility layer. When missing entries are
/// detected the generator panics with a clear message listing the missing
/// identifiers to make remediation straightforward for the developer.
fn check_no_unknowns(mappings: &[(String, i32)], context: &str) {
    let mut unknowns: Vec<&String> = mappings
        .iter()
        .filter_map(|(name, num)| {
            if *num == UNKNOWN_KIND {
                Some(name)
            } else {
                None
            }
        })
        .collect();

    if !unknowns.is_empty() {
        unknowns.sort();
        let mut msg = format!(
            "Found {} unknown mappings when generating constants for {}:\n",
            unknowns.len(),
            context
        );
        for name in unknowns {
            msg.push_str(&format!("  - {}\n", name));
        }
        msg.push_str("\nPlease add the missing enum arms to the corresponding parser/c_api code so the mapping can be generated from the AST.\n");
        panic!("{}", msg);
    }
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
            // Expression statement (with optional semicolon)
            syn::Stmt::Expr(expr, _) => {
                find_matches_in_expr(expr, callback);
            }
            // Local let-binding with optional initializer: let x = expr;
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

// Note: older versions supported string-based directive arms. That helper
// was removed during the enum-first migration; keep the codebase lean.

// String-based clause-arm parsing removed: generator is AST-only.

/// Parse an arm that maps an enum variant to an integer, e.g.
/// `DirectiveName::Parallel => 0,` returning ("Parallel", 0)
fn extract_variant_from_pat(pat: &Pat) -> Option<String> {
    match pat {
        Pat::Path(pat_path) => pat_path.path.segments.last().map(|s| s.ident.to_string()),
        Pat::TupleStruct(ts) => ts.path.segments.last().map(|s| s.ident.to_string()),
        Pat::Ident(ident) => Some(ident.ident.to_string()),
        _ => None,
    }
}

/// Parse an arm that maps an enum variant (or multiple variants separated by `|`) to an integer
/// e.g. `Parallel | ParallelFor => 0,` returning vec![("Parallel", 0), ("ParallelFor", 0)]
fn parse_enum_directive_arm(arm: &Arm) -> Vec<(String, i32)> {
    let mut results = Vec::new();

    // Extract number from body
    let num = if let Expr::Lit(ExprLit {
        lit: Lit::Int(lit_int),
        ..
    }) = &*arm.body
    {
        if let Ok(n) = lit_int.base10_parse::<i32>() {
            n
        } else {
            return results;
        }
    } else {
        return results;
    };

    match &arm.pat {
        // Multiple patterns separated by `|`
        Pat::Or(or_pat) => {
            for case in &or_pat.cases {
                if let Some(var) = extract_variant_from_pat(case) {
                    results.push((var, num));
                }
            }
        }
        // Single pattern
        _ => {
            if let Some(var) = extract_variant_from_pat(&arm.pat) {
                results.push((var, num));
            }
        }
    }

    results
}

/// Parse an arm that maps a ClauseName enum variant to an integer
/// e.g. `ClauseName::NumThreads => 0,` returning Some(("NumThreads", 0))
fn parse_enum_clause_arm(arm: &Arm) -> Vec<(String, i32)> {
    let mut results = Vec::new();
    // Helper: recursively extract an integer tag from expression bodies (AST-only)
    fn extract_num_from_expr(expr: &Expr) -> Option<i32> {
        match expr {
            Expr::Lit(ExprLit {
                lit: Lit::Int(li), ..
            }) => li.base10_parse::<i32>().ok(),
            Expr::Tuple(ExprTuple { elems, .. }) => {
                if let Some(Expr::Lit(ExprLit {
                    lit: Lit::Int(li), ..
                })) = elems.first()
                {
                    li.base10_parse::<i32>().ok()
                } else {
                    None
                }
            }
            Expr::Return(ret) => {
                if let Some(e) = &ret.expr {
                    extract_num_from_expr(e)
                } else {
                    None
                }
            }
            Expr::Block(block) => {
                // Scan statements for an expression or semi containing a tuple/literal
                for stmt in &block.block.stmts {
                    if let syn::Stmt::Expr(e, _) = stmt {
                        if let Some(n) = extract_num_from_expr(e) {
                            return Some(n);
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    let Some(num) = extract_num_from_expr(&arm.body) else {
        return results;
    };

    match &arm.pat {
        Pat::Or(or_pat) => {
            for case in &or_pat.cases {
                if let Some(var) = extract_variant_from_pat(case) {
                    results.push((var, num));
                }
            }
        }
        _ => {
            if let Some(var) = extract_variant_from_pat(&arm.pat) {
                results.push((var, num));
            }
        }
    }

    results
}

// String-based helpers removed: generator is AST-only
