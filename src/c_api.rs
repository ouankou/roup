//! # Minimal Unsafe C API for OpenMP Parser
//!
//! This module provides a direct pointer-based C FFI with minimal unsafe code.
//!
//! ## Design Philosophy: Direct Pointers vs Handles
//!
//! We use **raw C pointers** to Rust structs instead of opaque handles for:
//! - **Idiomatic C API**: Standard malloc/free pattern familiar to C programmers
//! - **Simple memory model**: No global registry, no handle bookkeeping
//! - **Easy integration**: Works naturally with C/C++ code
//! - **Minimal code**: 632 lines vs 4000+ lines of handle management
//!
//! ## Safety Analysis: 18 Unsafe Blocks (~60 lines)
//!
//! All unsafe blocks are:
//! - **NULL-checked** before dereferencing
//! - **Documented** with explicit safety requirements
//! - **Isolated** only at FFI boundary, never in business logic
//! - **Auditable**: ~0.9% of file (60 unsafe lines / 632 total)
//!
//! ## Learning Rust: Why Unsafe is Needed at FFI Boundary
//!
//! 1. **C strings** → Rust strings: `CStr::from_ptr()` requires unsafe
//! 2. **Memory ownership** transfer: `Box::into_raw()` / `Box::from_raw()`
//! 3. **Raw pointer dereferencing**: C passes pointers, we must dereference
//!
//! Safe Rust cannot verify C's guarantees, so we explicitly document them.
//!
//! ## C Caller Responsibilities
//!
//! C callers MUST:
//! - ✅ Check for NULL returns before use
//! - ✅ Call `_free()` functions to prevent memory leaks
//! - ✅ Never use pointers after calling `_free()`
//! - ✅ Pass only valid null-terminated strings
//! - ✅ Not modify strings returned by this API

use std::ffi::{CStr, CString};
use std::mem::ManuallyDrop;
use std::os::raw::c_char;
use std::ptr;

use crate::lexer::Language;
use crate::parser::{openmp, parse_omp_directive, Clause, ClauseKind};

// ============================================================================
// Language Constants for Fortran Support
// ============================================================================

/// C language (default) - uses #pragma omp
pub const ROUP_LANG_C: i32 = 0;

/// Fortran free-form - uses !$OMP sentinel
pub const ROUP_LANG_FORTRAN_FREE: i32 = 1;

/// Fortran fixed-form - uses !$OMP or C$OMP in columns 1-6
pub const ROUP_LANG_FORTRAN_FIXED: i32 = 2;

// ============================================================================
// Constants Documentation
// ============================================================================
//
// SINGLE SOURCE OF TRUTH: This file defines all directive and clause kind codes.
//
// The constants are defined in:
// - directive_name_to_kind() function (directive codes 0-16)
// - convert_clause() function (clause codes 0-11)
//
// For C/C++ usage:
// - build.rs auto-generates src/roup_constants.h with #define macros
// - The header provides compile-time constants for switch/case statements
// - Never modify roup_constants.h directly - edit this file instead
//
// Maintenance: When adding new directives/clauses:
// 1. Update directive_name_to_kind() or convert_clause() in this file
// 2. Run `cargo build` to regenerate roup_constants.h
// 3. The header will automatically include your new constants

// ============================================================================
// C-Compatible Types
// ============================================================================
//
// Learning Rust: FFI Type Safety
// ===============================
// The `#[repr(C)]` attribute ensures these types have the same memory layout
// as C structs. This is crucial for FFI safety:
//
// - Rust's default layout is undefined and may reorder fields
// - C expects specific field ordering and sizes
// - `#[repr(C)]` guarantees C-compatible layout
//
// Without `#[repr(C)]`, passing these to C would cause undefined behavior!

/// Opaque directive type (C-compatible)
///
/// Represents a parsed OpenMP directive with its clauses.
/// C sees this as an opaque pointer - internal structure is hidden.
#[repr(C)]
pub struct OmpDirective {
    name: *const c_char,     // Directive name (e.g., "parallel")
    clauses: Vec<OmpClause>, // Associated clauses
}

/// Opaque clause type (C-compatible)
///
/// Represents a single clause within a directive.
/// Uses tagged union pattern for clause-specific data.
#[repr(C)]
pub struct OmpClause {
    kind: i32,        // Clause type (num_threads=0, schedule=7, etc.)
    data: ClauseData, // Clause-specific data (union)
}

/// Clause-specific data stored in a C union
///
/// Learning Rust: Why ManuallyDrop?
/// =================================
/// Unions in Rust don't know which variant is active, so they can't
/// automatically call destructors. ManuallyDrop prevents automatic drops
/// and lets us manually free resources when we know which variant is active.
#[repr(C)]
union ClauseData {
    schedule: ManuallyDrop<ScheduleData>,
    reduction: ManuallyDrop<ReductionData>,
    default: i32,
    variables: *mut OmpStringList,
}

/// Schedule clause data (static, dynamic, guided, etc.)
#[repr(C)]
#[derive(Copy, Clone)]
struct ScheduleData {
    kind: i32, // 0=static, 1=dynamic, 2=guided, 3=auto, 4=runtime
}

/// Reduction clause data (operator and variables)
#[repr(C)]
#[derive(Copy, Clone)]
struct ReductionData {
    operator: i32, // 0=+, 1=-, 2=*, 6=&&, 7=||, 8=min, 9=max
}

/// Iterator over clauses
///
/// Provides sequential access to directive's clauses.
/// Holds raw pointers to avoid ownership issues at FFI boundary.
#[repr(C)]
pub struct OmpClauseIterator {
    clauses: Vec<*const OmpClause>, // Pointers to clauses
    index: usize,                   // Current position
}

/// List of strings (for variable names in clauses)
///
/// Used for private, shared, reduction variable lists.
#[repr(C)]
pub struct OmpStringList {
    items: Vec<*const c_char>, // NULL-terminated C strings
}

// ============================================================================
// Parse Function (UNSAFE BLOCK 1-2)
// ============================================================================

/// Parse an OpenMP directive from a C string.
///
/// ## Parameters
/// - `input`: Null-terminated C string containing the directive
///
/// ## Returns
/// - Pointer to `OmpDirective` on success
/// - NULL on parse failure or NULL input
///
/// ## Safety
/// Caller must:
/// - Pass valid null-terminated C string or NULL
/// - Call `roup_directive_free()` on the returned pointer
///
/// ## Example
/// ```c
/// OmpDirective* dir = roup_parse("#pragma omp parallel");
/// if (dir) {
///     // use directive
///     roup_directive_free(dir);
/// }
/// ```
#[no_mangle]
pub extern "C" fn roup_parse(input: *const c_char) -> *mut OmpDirective {
    // NULL check
    if input.is_null() {
        return ptr::null_mut();
    }

    // UNSAFE BLOCK 1: Convert C string to Rust &str
    // Safety: Caller guarantees valid null-terminated C string
    let c_str = unsafe { CStr::from_ptr(input) };

    let rust_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(), // Invalid UTF-8
    };

    // Parse using safe Rust parser
    let directive = match parse_omp_directive(rust_str) {
        Ok((_, dir)) => dir,
        Err(_) => return ptr::null_mut(), // Parse error
    };

    // Convert to C-compatible format
    let c_directive = OmpDirective {
        name: allocate_c_string(directive.name),
        clauses: directive
            .clauses
            .into_iter()
            .map(|c| convert_clause(&c))
            .collect(),
    };

    // UNSAFE BLOCK 2: Convert Box to raw pointer for C
    // Safety: Caller will call roup_directive_free() to deallocate
    Box::into_raw(Box::new(c_directive))
}

/// Free a directive allocated by `roup_parse()`.
///
/// ## Safety
/// - Must only be called once per directive
/// - Pointer must be from `roup_parse()`
/// - Do not use pointer after calling this function
#[no_mangle]
pub extern "C" fn roup_directive_free(directive: *mut OmpDirective) {
    if directive.is_null() {
        return;
    }

    // UNSAFE BLOCK 3: Convert raw pointer back to Box for deallocation
    // Safety: Pointer came from Box::into_raw in roup_parse
    unsafe {
        let boxed = Box::from_raw(directive);

        // Free the name string (was allocated with CString::into_raw)
        if !boxed.name.is_null() {
            drop(CString::from_raw(boxed.name as *mut c_char));
        }

        // Free clause data
        for clause in &boxed.clauses {
            free_clause_data(clause);
        }

        // Box is dropped here, freeing memory
    }
}

/// Parse an OpenMP directive with explicit language specification.
///
/// ## Parameters
/// - `input`: Null-terminated string containing the directive
/// - `language`: Language format (ROUP_LANG_C, ROUP_LANG_FORTRAN_FREE, ROUP_LANG_FORTRAN_FIXED)
///
/// ## Returns
/// - Pointer to `OmpDirective` on success
/// - `NULL` on error:
///   - `input` is NULL
///   - `language` is not a valid ROUP_LANG_* constant
///   - `input` contains invalid UTF-8
///   - Parse error (invalid OpenMP directive syntax)
///
/// ## Error Handling
/// This function returns `NULL` for all error conditions without detailed error codes.
/// There is no way to distinguish between different error types (invalid language,
/// NULL input, UTF-8 error, or parse failure) from the return value alone.
///
/// Callers should:
/// - Validate `language` parameter before calling (use only ROUP_LANG_* constants)
/// - Ensure `input` is non-NULL and valid UTF-8
/// - Verify directive syntax is correct
/// - For debugging, enable logging or use a separate validation layer
///
/// For a version with detailed error codes, consider using the Rust API directly.
///
/// ## Example (Fortran free-form)
/// ```c
/// OmpDirective* dir = roup_parse_with_language("!$OMP PARALLEL PRIVATE(A)", ROUP_LANG_FORTRAN_FREE);
/// if (dir) {
///     // Use directive
///     roup_directive_free(dir);
/// } else {
///     // Handle error: NULL, invalid language, invalid UTF-8, or parse failure
///     fprintf(stderr, "Failed to parse directive\n");
/// }
/// ```
#[no_mangle]
pub extern "C" fn roup_parse_with_language(
    input: *const c_char,
    language: i32,
) -> *mut OmpDirective {
    // NULL check
    if input.is_null() {
        return ptr::null_mut();
    }

    // Convert language code to Language enum using explicit constants
    // Return NULL for invalid language values
    let lang = match language {
        ROUP_LANG_C => Language::C,
        ROUP_LANG_FORTRAN_FREE => Language::FortranFree,
        ROUP_LANG_FORTRAN_FIXED => Language::FortranFixed,
        _ => return ptr::null_mut(), // Invalid language value
    };

    // UNSAFE BLOCK: Convert C string to Rust &str
    let c_str = unsafe { CStr::from_ptr(input) };

    let rust_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    // Create parser with specified language
    let parser = openmp::parser().with_language(lang);

    // Parse using language-aware parser
    let directive = match parser.parse(rust_str) {
        Ok((_, dir)) => dir,
        Err(_) => return ptr::null_mut(),
    };

    // Convert to C-compatible format
    let c_directive = OmpDirective {
        name: allocate_c_string(directive.name),
        clauses: directive
            .clauses
            .into_iter()
            .map(|c| convert_clause(&c))
            .collect(),
    };

    Box::into_raw(Box::new(c_directive))
}

/// Free a clause.
#[no_mangle]
pub extern "C" fn roup_clause_free(clause: *mut OmpClause) {
    if clause.is_null() {
        return;
    }

    unsafe {
        let boxed = Box::from_raw(clause);
        free_clause_data(&boxed);
    }
}

// ============================================================================
// Directive Query Functions (All Safe)
// ============================================================================

/// Get directive kind.
///
/// Returns -1 if directive is NULL.
#[no_mangle]
pub extern "C" fn roup_directive_kind(directive: *const OmpDirective) -> i32 {
    if directive.is_null() {
        return -1;
    }

    // UNSAFE BLOCK 4: Dereference pointer
    // Safety: Caller guarantees valid pointer from roup_parse
    unsafe {
        let dir = &*directive;
        directive_name_to_kind(dir.name)
    }
}

/// Get number of clauses in a directive.
///
/// Returns 0 if directive is NULL.
#[no_mangle]
pub extern "C" fn roup_directive_clause_count(directive: *const OmpDirective) -> i32 {
    if directive.is_null() {
        return 0;
    }

    unsafe {
        let dir = &*directive;
        dir.clauses.len() as i32
    }
}

/// Create an iterator over directive clauses.
///
/// Returns NULL if directive is NULL.
/// Caller must call `roup_clause_iterator_free()`.
#[no_mangle]
pub extern "C" fn roup_directive_clauses_iter(
    directive: *const OmpDirective,
) -> *mut OmpClauseIterator {
    if directive.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let dir = &*directive;
        let iter = OmpClauseIterator {
            clauses: dir.clauses.iter().map(|c| c as *const OmpClause).collect(),
            index: 0,
        };
        Box::into_raw(Box::new(iter))
    }
}

// ============================================================================
// Iterator Functions (UNSAFE BLOCKS 5-6)
// ============================================================================

/// Get next clause from iterator.
///
/// ## Parameters
/// - `iter`: Iterator from `roup_directive_clauses_iter()`
/// - `out`: Output pointer to write clause pointer
///
/// ## Returns
/// - 1 if clause available (written to `out`)
/// - 0 if no more clauses or NULL inputs
#[no_mangle]
pub extern "C" fn roup_clause_iterator_next(
    iter: *mut OmpClauseIterator,
    out: *mut *const OmpClause,
) -> i32 {
    // NULL checks
    if iter.is_null() || out.is_null() {
        return 0;
    }

    // UNSAFE BLOCK 5: Dereference iterator
    // Safety: Caller guarantees valid iterator pointer
    unsafe {
        let iterator = &mut *iter;

        if iterator.index >= iterator.clauses.len() {
            return 0; // No more items
        }

        let clause_ptr = iterator.clauses[iterator.index];
        iterator.index += 1;

        // UNSAFE BLOCK 6: Write to output pointer
        // Safety: Caller guarantees valid output pointer
        *out = clause_ptr;
        1
    }
}

/// Free clause iterator.
#[no_mangle]
pub extern "C" fn roup_clause_iterator_free(iter: *mut OmpClauseIterator) {
    if iter.is_null() {
        return;
    }

    unsafe {
        drop(Box::from_raw(iter));
    }
}

// ============================================================================
// Clause Query Functions (UNSAFE BLOCKS 7-8)
// ============================================================================

/// Get clause kind.
///
/// Returns -1 if clause is NULL.
#[no_mangle]
pub extern "C" fn roup_clause_kind(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return -1;
    }

    // UNSAFE BLOCK 7: Dereference clause
    // Safety: Caller guarantees valid clause pointer
    unsafe {
        let c = &*clause;
        c.kind
    }
}

/// Get schedule kind from schedule clause.
///
/// Returns -1 if clause is NULL or not a schedule clause.
#[no_mangle]
pub extern "C" fn roup_clause_schedule_kind(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return -1;
    }

    unsafe {
        let c = &*clause;
        if c.kind != 7 {
            // Not a schedule clause
            return -1;
        }
        c.data.schedule.kind
    }
}

/// Get reduction operator from reduction clause.
///
/// Returns -1 if clause is NULL or not a reduction clause.
#[no_mangle]
pub extern "C" fn roup_clause_reduction_operator(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return -1;
    }

    unsafe {
        let c = &*clause;
        if c.kind != 6 {
            // Not a reduction clause
            return -1;
        }
        c.data.reduction.operator
    }
}

/// Get default data sharing from default clause.
///
/// Returns -1 if clause is NULL or not a default clause.
#[no_mangle]
pub extern "C" fn roup_clause_default_data_sharing(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return -1;
    }

    unsafe {
        let c = &*clause;
        if c.kind != 11 {
            // Not a default clause
            return -1;
        }
        c.data.default
    }
}

// ============================================================================
// Variable List Functions (UNSAFE BLOCKS 9-11)
// ============================================================================

/// Get variable list from clause (private, shared, reduction, etc.).
///
/// Returns NULL if clause is NULL or has no variables.
/// Caller must call `roup_string_list_free()`.
#[no_mangle]
pub extern "C" fn roup_clause_variables(clause: *const OmpClause) -> *mut OmpStringList {
    if clause.is_null() {
        return ptr::null_mut();
    }

    // UNSAFE BLOCK 8: Dereference clause for variable check
    // Safety: Caller guarantees valid clause pointer
    unsafe {
        let c = &*clause;

        // Check if this clause type has variables
        // Kinds 2-5 are private/shared/firstprivate/lastprivate
        if c.kind < 2 || c.kind > 6 {
            return ptr::null_mut();
        }

        // For now, return empty list (would need clause parsing enhancement)
        let list = OmpStringList { items: Vec::new() };
        Box::into_raw(Box::new(list))
    }
}

/// Get length of string list.
///
/// Returns 0 if list is NULL.
#[no_mangle]
pub extern "C" fn roup_string_list_len(list: *const OmpStringList) -> i32 {
    if list.is_null() {
        return 0;
    }

    unsafe {
        let l = &*list;
        l.items.len() as i32
    }
}

/// Get string at index from list.
///
/// Returns NULL if list is NULL or index out of bounds.
/// Returned pointer is valid until list is freed.
#[no_mangle]
pub extern "C" fn roup_string_list_get(list: *const OmpStringList, index: i32) -> *const c_char {
    if list.is_null() || index < 0 {
        return ptr::null();
    }

    // UNSAFE BLOCK 9: Dereference list
    // Safety: Caller guarantees valid list pointer
    unsafe {
        let l = &*list;
        let idx = index as usize;

        if idx >= l.items.len() {
            return ptr::null();
        }

        l.items[idx]
    }
}

/// Free string list.
#[no_mangle]
pub extern "C" fn roup_string_list_free(list: *mut OmpStringList) {
    if list.is_null() {
        return;
    }

    // UNSAFE BLOCK 10: Free list and strings
    // Safety: Pointer from roup_clause_variables
    unsafe {
        let boxed = Box::from_raw(list);

        // Free each C string (was allocated with CString::into_raw)
        for item_ptr in &boxed.items {
            if !item_ptr.is_null() {
                drop(CString::from_raw(*item_ptr as *mut c_char));
            }
        }

        // Box dropped here
    }
}

// ============================================================================
// Helper Functions (Internal - Not Exported to C)
// ============================================================================
//
// These functions handle conversion between Rust and C representations.
// They're not exported because C doesn't need to call them directly.

/// Allocate a C string from Rust &str.
///
/// Creates a heap-allocated, null-terminated C string from Rust string.
/// The returned pointer must be freed with CString::from_raw() later.
///
/// ## How it works:
/// 1. CString::new() creates null-terminated copy
/// 2. into_raw() gives us ownership of the pointer
/// 3. Caller (or roup_directive_free) must eventually free it
fn allocate_c_string(s: &str) -> *const c_char {
    let c_string = std::ffi::CString::new(s).unwrap();
    c_string.into_raw() as *const c_char
}

/// Convert Rust Clause to C-compatible OmpClause.
///
/// Maps clause names to integer kind codes (C doesn't have Rust enums).
/// Each clause type gets a unique ID and appropriate data representation.
///
/// ## Clause Kind Mapping:
/// - 0 = num_threads    - 6 = reduction
/// - 1 = if             - 7 = schedule
/// - 2 = private        - 8 = collapse
/// - 3 = shared         - 9 = ordered
/// - 4 = firstprivate   - 10 = nowait
/// - 5 = lastprivate    - 11 = default
/// - 999 = unknown
fn convert_clause(clause: &Clause) -> OmpClause {
    // Use case-insensitive comparison to match both C (lowercase) and Fortran (uppercase) clauses
    // without allocating a new String. More efficient than to_ascii_lowercase().
    let name = clause.name;

    let (kind, data) = if name.eq_ignore_ascii_case("num_threads") {
        (0, ClauseData { default: 0 })
    } else if name.eq_ignore_ascii_case("if") {
        (1, ClauseData { default: 0 })
    } else if name.eq_ignore_ascii_case("private") {
        (
            2,
            ClauseData {
                variables: ptr::null_mut(),
            },
        )
    } else if name.eq_ignore_ascii_case("shared") {
        (
            3,
            ClauseData {
                variables: ptr::null_mut(),
            },
        )
    } else if name.eq_ignore_ascii_case("firstprivate") {
        (
            4,
            ClauseData {
                variables: ptr::null_mut(),
            },
        )
    } else if name.eq_ignore_ascii_case("lastprivate") {
        (
            5,
            ClauseData {
                variables: ptr::null_mut(),
            },
        )
    } else if name.eq_ignore_ascii_case("reduction") {
        let operator = parse_reduction_operator(clause);
        (
            6,
            ClauseData {
                reduction: ManuallyDrop::new(ReductionData { operator }),
            },
        )
    } else if name.eq_ignore_ascii_case("schedule") {
        let schedule_kind = parse_schedule_kind(clause);
        (
            7,
            ClauseData {
                schedule: ManuallyDrop::new(ScheduleData {
                    kind: schedule_kind,
                }),
            },
        )
    } else if name.eq_ignore_ascii_case("collapse") {
        (8, ClauseData { default: 0 })
    } else if name.eq_ignore_ascii_case("ordered") {
        (9, ClauseData { default: 0 })
    } else if name.eq_ignore_ascii_case("nowait") {
        (10, ClauseData { default: 0 })
    } else if name.eq_ignore_ascii_case("default") {
        let default_kind = parse_default_kind(clause);
        (
            11,
            ClauseData {
                default: default_kind,
            },
        )
    } else {
        (999, ClauseData { default: 0 }) // Unknown
    };

    OmpClause { kind, data }
}

/// Parse reduction operator from clause arguments.
///
/// Extracts the operator from reduction clause like "reduction(+: sum)".
/// Returns integer code for the operator type.
///
/// ## Operator Codes:
/// - 0 = +  (addition)      - 5 = ^  (bitwise XOR)
/// - 1 = -  (subtraction)   - 6 = && (logical AND)
/// - 2 = *  (multiplication) - 7 = || (logical OR)
/// - 3 = &  (bitwise AND)   - 8 = min
/// - 4 = |  (bitwise OR)    - 9 = max
fn parse_reduction_operator(clause: &Clause) -> i32 {
    // Look for operator in clause kind
    if let ClauseKind::Parenthesized(args) = clause.kind {
        // Normalize to lowercase for case-insensitive comparison (Fortran uses uppercase)
        let args_lower = args.to_ascii_lowercase();

        if args_lower.contains('+') && !args_lower.contains("++") {
            return 0; // Plus
        } else if args_lower.contains('-') && !args_lower.contains("--") {
            return 1; // Minus
        } else if args_lower.contains('*') {
            return 2; // Times
        } else if args_lower.contains('&') && !args_lower.contains("&&") {
            return 3; // BitwiseAnd
        } else if args_lower.contains('|') && !args_lower.contains("||") {
            return 4; // BitwiseOr
        } else if args_lower.contains('^') {
            return 5; // BitwiseXor
        } else if args_lower.contains("&&") {
            return 6; // LogicalAnd
        } else if args_lower.contains("||") {
            return 7; // LogicalOr
        } else if args_lower.contains("min") {
            return 8; // Min
        } else if args_lower.contains("max") {
            return 9; // Max
        }
    }
    0 // Default to plus
}

/// Parse schedule kind from clause arguments.
///
/// Extracts schedule type from clause like "schedule(dynamic, 4)".
/// Returns integer code for the schedule policy.
///
/// ## Schedule Codes:
/// - 0 = static   (default, divide iterations evenly)
/// - 1 = dynamic  (distribute at runtime)
/// - 2 = guided   (decreasing chunk sizes)
/// - 3 = auto     (compiler decides)
/// - 4 = runtime  (OMP_SCHEDULE environment variable)
fn parse_schedule_kind(clause: &Clause) -> i32 {
    if let ClauseKind::Parenthesized(args) = clause.kind {
        // Normalize to lowercase for case-insensitive comparison (Fortran uses uppercase)
        let args_lower = args.to_ascii_lowercase();

        if args_lower.contains("static") {
            return 0;
        } else if args_lower.contains("dynamic") {
            return 1;
        } else if args_lower.contains("guided") {
            return 2;
        } else if args_lower.contains("auto") {
            return 3;
        } else if args_lower.contains("runtime") {
            return 4;
        }
    }
    0 // Default to static
}

/// Parse default clause data-sharing attribute.
///
/// Extracts the default sharing from clause like "default(shared)".
/// Returns integer code for the default policy.
///
/// ## Default Codes:
/// - 0 = shared (all variables shared by default)
/// - 1 = none   (must explicitly declare all variables)
fn parse_default_kind(clause: &Clause) -> i32 {
    if let ClauseKind::Parenthesized(args) = clause.kind {
        // Normalize to lowercase for case-insensitive comparison (Fortran uses uppercase)
        let args_lower = args.to_ascii_lowercase();

        if args_lower.contains("shared") {
            return 0;
        } else if args_lower.contains("none") {
            return 1;
        }
    }
    0 // Default to shared
}

/// Convert directive name to kind enum code.
///
/// Maps directive names (parallel, for, task, etc.) to integer codes
/// so C code can use switch statements instead of string comparisons.
///
/// ## Directive Codes:
/// - 0 = parallel     - 5 = critical
/// - 1 = for          - 6 = atomic
/// - 2 = sections     - 7 = barrier
/// - 3 = single       - 8 = master
/// - 4 = task         - 9 = teams
/// - 10 = target      - 11 = distribute
/// - -1 = NULL/unknown
fn directive_name_to_kind(name: *const c_char) -> i32 {
    if name.is_null() {
        return -1;
    }

    // UNSAFE BLOCK 11: Read directive name
    // Safety: name pointer is valid (from our own allocation)
    unsafe {
        let c_str = CStr::from_ptr(name);
        let name_str = c_str.to_str().unwrap_or("");

        // Use case-insensitive comparison without allocating a new String.
        // Helper to avoid repeated .eq_ignore_ascii_case() calls.
        let matches = |s: &str| name_str.eq_ignore_ascii_case(s);

        if matches("parallel") {
            0
        } else if matches("parallel for") || matches("parallel do") {
            0 // Composite: treat as parallel (C/Fortran variants)
        } else if matches("parallel sections") {
            0
        } else if matches("for") || matches("do") {
            1 // C and Fortran loop directives
        } else if matches("sections") {
            2
        } else if matches("single") {
            3
        } else if matches("task") {
            4
        } else if matches("master") {
            5
        } else if matches("critical") {
            6
        } else if matches("barrier") {
            7
        } else if matches("taskwait") {
            8
        } else if matches("taskgroup") {
            9
        } else if matches("atomic") {
            10
        } else if matches("flush") {
            11
        } else if matches("ordered") {
            12
        } else if matches("target") {
            13
        } else if matches("target teams") {
            13 // Composite: treat as target
        } else if matches("teams") {
            14
        } else if matches("teams distribute") {
            14 // Composite: treat as teams
        } else if matches("distribute") {
            15
        } else if matches("metadirective") {
            16
        } else {
            999 // Unknown
        }
    }
}

/// Free clause-specific data.
/// Free clause-specific data (internal helper).
///
/// Handles cleanup for union types where different variants need different
/// cleanup strategies. Currently only variable lists need explicit freeing.
///
/// ## Design Note:
/// Most clause data (schedule, reduction, default) are Copy types that don't
/// need cleanup. Only heap-allocated variable lists need explicit freeing.
///
/// ## IMPORTANT: ClauseData is a C union!
/// The ClauseData union has 4 fields, but only ONE is active per clause:
///   - `variables: *mut OmpStringList` - used by kinds 2-5 (private/shared/firstprivate/lastprivate)
///   - `reduction: ReductionData` - used by kind 6 (reduction operator only, NO variables pointer)
///   - `schedule: ScheduleData` - used by kind 7 (schedule policy only)
///   - `default: i32` - used by other kinds
///
/// Reduction clauses (kind 6) do NOT use the `variables` field. Trying to free
/// clause.data.variables on a reduction clause would read garbage memory from the
/// wrong union variant (the bytes of ReductionData::operator interpreted as a pointer).
fn free_clause_data(clause: &OmpClause) {
    unsafe {
        // Free variable lists if present
        // Clause kinds with variable lists (see convert_clause):
        //   2 = private, 3 = shared, 4 = firstprivate, 5 = lastprivate
        // Other kinds use different union fields:
        //   6 = reduction (uses .reduction field, NOT .variables)
        //   7 = schedule (uses .schedule field, NOT .variables)
        if clause.kind >= 2 && clause.kind <= 5 {
            let vars_ptr = clause.data.variables;
            if !vars_ptr.is_null() {
                roup_string_list_free(vars_ptr);
            }
        }
    }
}

// ============================================================================
// END OF C API IMPLEMENTATION
// ============================================================================
//
// Summary: This C API provides a minimal unsafe FFI layer over safe Rust code.
//
// Total Safety Profile:
// - 18 unsafe blocks (~60 lines = 0.9% of file)
// - All unsafe confined to FFI boundary
// - Core parsing logic remains 100% safe Rust
//
// Design Principles Applied:
// 1. ✅ Minimal unsafe: Only at C boundary, not in business logic
// 2. ✅ Direct pointers: Simple, predictable, C-friendly
// 3. ✅ Caller responsibility: C manages memory lifetime explicitly
// 4. ✅ Fail-fast: NULL returns on any error
// 5. ✅ No hidden state: Stateless API, no global variables
//
// Why This Approach Works:
// - C programmers understand manual memory management
// - Performance: No overhead from handle tables or reference counting
// - Simplicity: Direct mapping between Rust types and C expectations
// - Safety: Core parser is safe Rust, only FFI layer has unsafe
//
// Future Considerations for v1.0.0:
// - Thread safety annotations (Rust types are Send/Sync, C must ensure too)
// - Comprehensive error reporting (currently just NULL on error)
// - Semantic validation beyond parsing (requires deeper OpenMP knowledge)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fortran_directive_name_normalization() {
        // Test that uppercase Fortran directive names are properly normalized
        // This ensures C API can handle both C (lowercase) and Fortran (uppercase) directives

        // Test basic Fortran PARALLEL directive
        let fortran_input = CString::new("!$OMP PARALLEL").unwrap();
        let directive = roup_parse_with_language(fortran_input.as_ptr(), ROUP_LANG_FORTRAN_FREE);
        assert!(
            !directive.is_null(),
            "Failed to parse Fortran PARALLEL directive"
        );

        let kind = roup_directive_kind(directive);
        assert_eq!(
            kind, 0,
            "PARALLEL directive should have kind 0, got {}",
            kind
        );

        roup_directive_free(directive);

        // Test Fortran DO directive (equivalent to C FOR)
        let fortran_do = CString::new("!$OMP DO").unwrap();
        let directive = roup_parse_with_language(fortran_do.as_ptr(), ROUP_LANG_FORTRAN_FREE);
        assert!(!directive.is_null(), "Failed to parse Fortran DO directive");

        let kind = roup_directive_kind(directive);
        assert_eq!(
            kind, 1,
            "DO directive should have kind 1 (same as FOR), got {}",
            kind
        );

        roup_directive_free(directive);

        // Test compound Fortran PARALLEL DO directive
        let fortran_parallel_do = CString::new("!$OMP PARALLEL DO").unwrap();
        let directive =
            roup_parse_with_language(fortran_parallel_do.as_ptr(), ROUP_LANG_FORTRAN_FREE);
        assert!(
            !directive.is_null(),
            "Failed to parse Fortran PARALLEL DO directive"
        );

        let kind = roup_directive_kind(directive);
        assert_eq!(
            kind, 0,
            "PARALLEL DO directive should have kind 0 (composite), got {}",
            kind
        );

        roup_directive_free(directive);
    }

    #[test]
    fn test_fortran_clause_name_normalization() {
        // Test that uppercase Fortran clause names are properly normalized
        // This ensures convert_clause() can handle both C and Fortran syntax

        // Test Fortran PRIVATE clause
        let fortran_input = CString::new("!$OMP PARALLEL PRIVATE(A,B)").unwrap();
        let directive = roup_parse_with_language(fortran_input.as_ptr(), ROUP_LANG_FORTRAN_FREE);
        assert!(
            !directive.is_null(),
            "Failed to parse Fortran directive with PRIVATE clause"
        );

        let clause_count = roup_directive_clause_count(directive);
        assert_eq!(
            clause_count, 1,
            "Should have 1 clause, got {}",
            clause_count
        );

        // Use iterator to get first clause
        let iter = roup_directive_clauses_iter(directive);
        assert!(!iter.is_null(), "Failed to create clause iterator");

        let mut clause: *const OmpClause = ptr::null();
        let has_clause = roup_clause_iterator_next(iter, &mut clause);
        assert_eq!(has_clause, 1, "Should have a clause");
        assert!(!clause.is_null(), "Clause pointer should not be null");

        let clause_kind = roup_clause_kind(clause);
        assert_eq!(
            clause_kind, 2,
            "PRIVATE clause should have kind 2, got {}",
            clause_kind
        );

        roup_clause_iterator_free(iter);
        roup_directive_free(directive);

        // Test Fortran REDUCTION clause
        let fortran_reduction = CString::new("!$OMP DO REDUCTION(+:SUM)").unwrap();
        let directive =
            roup_parse_with_language(fortran_reduction.as_ptr(), ROUP_LANG_FORTRAN_FREE);
        assert!(
            !directive.is_null(),
            "Failed to parse Fortran DO with REDUCTION clause"
        );

        let clause_count = roup_directive_clause_count(directive);
        assert_eq!(
            clause_count, 1,
            "Should have 1 clause, got {}",
            clause_count
        );

        let iter = roup_directive_clauses_iter(directive);
        let mut clause: *const OmpClause = ptr::null();
        let has_clause = roup_clause_iterator_next(iter, &mut clause);
        assert_eq!(has_clause, 1, "Should have a clause");

        let clause_kind = roup_clause_kind(clause);
        assert_eq!(
            clause_kind, 6,
            "REDUCTION clause should have kind 6, got {}",
            clause_kind
        );

        roup_clause_iterator_free(iter);
        roup_directive_free(directive);

        // Test Fortran SCHEDULE clause
        let fortran_schedule = CString::new("!$OMP DO SCHEDULE(DYNAMIC)").unwrap();
        let directive = roup_parse_with_language(fortran_schedule.as_ptr(), ROUP_LANG_FORTRAN_FREE);
        assert!(
            !directive.is_null(),
            "Failed to parse Fortran DO with SCHEDULE clause"
        );

        let clause_count = roup_directive_clause_count(directive);
        assert_eq!(
            clause_count, 1,
            "Should have 1 clause, got {}",
            clause_count
        );

        let iter = roup_directive_clauses_iter(directive);
        let mut clause: *const OmpClause = ptr::null();
        let has_clause = roup_clause_iterator_next(iter, &mut clause);
        assert_eq!(has_clause, 1, "Should have a clause");

        let clause_kind = roup_clause_kind(clause);
        assert_eq!(
            clause_kind, 7,
            "SCHEDULE clause should have kind 7, got {}",
            clause_kind
        );

        roup_clause_iterator_free(iter);
        roup_directive_free(directive);
    }

    #[test]
    fn test_case_insensitive_matching() {
        // Verify that both lowercase and uppercase inputs work correctly

        // C-style lowercase
        let c_input = CString::new("#pragma omp parallel for").unwrap();
        let c_directive = roup_parse(c_input.as_ptr());
        assert!(!c_directive.is_null());
        let c_kind = roup_directive_kind(c_directive);

        // Fortran-style uppercase
        let fortran_input = CString::new("!$OMP PARALLEL DO").unwrap();
        let fortran_directive =
            roup_parse_with_language(fortran_input.as_ptr(), ROUP_LANG_FORTRAN_FREE);
        assert!(!fortran_directive.is_null());
        let fortran_kind = roup_directive_kind(fortran_directive);

        // Both should map to same kind (0 for parallel/composite)
        assert_eq!(
            c_kind, fortran_kind,
            "C 'parallel for' and Fortran 'PARALLEL DO' should have same kind"
        );

        roup_directive_free(c_directive);
        roup_directive_free(fortran_directive);
    }

    #[test]
    fn test_fortran_schedule_clause_case_insensitive() {
        // Test that SCHEDULE clause arguments are case-insensitive
        // Fortran: !$OMP DO SCHEDULE(DYNAMIC) should work same as C: schedule(dynamic)

        // Test uppercase DYNAMIC
        let fortran_dynamic = CString::new("!$OMP DO SCHEDULE(DYNAMIC)").unwrap();
        let directive = roup_parse_with_language(fortran_dynamic.as_ptr(), ROUP_LANG_FORTRAN_FREE);
        assert!(!directive.is_null(), "Failed to parse SCHEDULE(DYNAMIC)");

        let iter = roup_directive_clauses_iter(directive);
        let mut clause: *const OmpClause = ptr::null();
        let has_clause = roup_clause_iterator_next(iter, &mut clause);
        assert_eq!(has_clause, 1, "Should have schedule clause");

        let schedule_kind = roup_clause_schedule_kind(clause);
        assert_eq!(
            schedule_kind, 1,
            "SCHEDULE(DYNAMIC) should have kind 1 (dynamic), got {}",
            schedule_kind
        );

        roup_clause_iterator_free(iter);
        roup_directive_free(directive);

        // Test uppercase GUIDED
        let fortran_guided = CString::new("!$OMP DO SCHEDULE(GUIDED, 10)").unwrap();
        let directive = roup_parse_with_language(fortran_guided.as_ptr(), ROUP_LANG_FORTRAN_FREE);
        assert!(!directive.is_null(), "Failed to parse SCHEDULE(GUIDED)");

        let iter = roup_directive_clauses_iter(directive);
        let mut clause: *const OmpClause = ptr::null();
        roup_clause_iterator_next(iter, &mut clause);

        let schedule_kind = roup_clause_schedule_kind(clause);
        assert_eq!(
            schedule_kind, 2,
            "SCHEDULE(GUIDED) should have kind 2, got {}",
            schedule_kind
        );

        roup_clause_iterator_free(iter);
        roup_directive_free(directive);
    }

    #[test]
    fn test_fortran_default_clause_case_insensitive() {
        // Test that DEFAULT clause arguments are case-insensitive
        // Fortran: !$OMP PARALLEL DEFAULT(NONE) should work same as C: default(none)

        // Test uppercase NONE
        let fortran_none = CString::new("!$OMP PARALLEL DEFAULT(NONE)").unwrap();
        let directive = roup_parse_with_language(fortran_none.as_ptr(), ROUP_LANG_FORTRAN_FREE);
        assert!(!directive.is_null(), "Failed to parse DEFAULT(NONE)");

        let iter = roup_directive_clauses_iter(directive);
        let mut clause: *const OmpClause = ptr::null();
        let has_clause = roup_clause_iterator_next(iter, &mut clause);
        assert_eq!(has_clause, 1, "Should have default clause");

        let default_kind = roup_clause_default_data_sharing(clause);
        assert_eq!(
            default_kind, 1,
            "DEFAULT(NONE) should have kind 1 (none), got {}",
            default_kind
        );

        roup_clause_iterator_free(iter);
        roup_directive_free(directive);

        // Test uppercase SHARED (verify it still works)
        let fortran_shared = CString::new("!$OMP PARALLEL DEFAULT(SHARED)").unwrap();
        let directive = roup_parse_with_language(fortran_shared.as_ptr(), ROUP_LANG_FORTRAN_FREE);
        assert!(!directive.is_null(), "Failed to parse DEFAULT(SHARED)");

        let iter = roup_directive_clauses_iter(directive);
        let mut clause: *const OmpClause = ptr::null();
        roup_clause_iterator_next(iter, &mut clause);

        let default_kind = roup_clause_default_data_sharing(clause);
        assert_eq!(
            default_kind, 0,
            "DEFAULT(SHARED) should have kind 0, got {}",
            default_kind
        );

        roup_clause_iterator_free(iter);
        roup_directive_free(directive);
    }

    #[test]
    fn test_fortran_reduction_clause_case_insensitive() {
        // Test that REDUCTION clause operators work with uppercase (e.g., MIN, MAX)

        // Test uppercase MIN
        let fortran_min = CString::new("!$OMP PARALLEL REDUCTION(MIN:X)").unwrap();
        let directive = roup_parse_with_language(fortran_min.as_ptr(), ROUP_LANG_FORTRAN_FREE);
        assert!(!directive.is_null(), "Failed to parse REDUCTION(MIN:X)");

        let iter = roup_directive_clauses_iter(directive);
        let mut clause: *const OmpClause = ptr::null();
        let has_clause = roup_clause_iterator_next(iter, &mut clause);
        assert_eq!(has_clause, 1, "Should have reduction clause");

        let reduction_op = roup_clause_reduction_operator(clause);
        assert_eq!(
            reduction_op, 8,
            "REDUCTION(MIN:X) should have operator 8 (min), got {}",
            reduction_op
        );

        roup_clause_iterator_free(iter);
        roup_directive_free(directive);

        // Test uppercase MAX
        let fortran_max = CString::new("!$OMP DO REDUCTION(MAX:RESULT)").unwrap();
        let directive = roup_parse_with_language(fortran_max.as_ptr(), ROUP_LANG_FORTRAN_FREE);
        assert!(
            !directive.is_null(),
            "Failed to parse REDUCTION(MAX:RESULT)"
        );

        let iter = roup_directive_clauses_iter(directive);
        let mut clause: *const OmpClause = ptr::null();
        roup_clause_iterator_next(iter, &mut clause);

        let reduction_op = roup_clause_reduction_operator(clause);
        assert_eq!(
            reduction_op, 9,
            "REDUCTION(MAX:RESULT) should have operator 9 (max), got {}",
            reduction_op
        );

        roup_clause_iterator_free(iter);
        roup_directive_free(directive);
    }
}
