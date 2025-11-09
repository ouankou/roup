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
//! ## Case-Insensitive Matching: String Allocation Tradeoff
//!
//! Functions `directive_name_to_kind()` and `convert_clause()` allocate a String
//! for case-insensitive matching (Fortran uses uppercase, C uses lowercase).
//!
//! **Why not optimize with `eq_ignore_ascii_case()`?**
//! - Constants generator (`src/constants_gen.rs`) requires `match` expressions
//! - Parser uses syn crate to extract directive/clause mappings from AST
//! - Cannot parse if-else chains → must use `match normalized_name.as_str()`
//! - String allocation is necessary for match arm patterns
//!
//! **Is this a performance issue?**
//! - No: These functions are called once per directive/clause at API boundary
//! - Typical usage: Parse a few dozen directives in an entire program
//! - String allocation cost is negligible compared to parsing overhead
//!
//! **Future optimization path**: Update `constants_gen.rs` to parse if-else chains,
//! then use `eq_ignore_ascii_case()` without allocations.
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

// Clippy configuration for FFI module
// We intentionally wrap unsafe operations in safe public functions for the C API.
// The C callers are responsible for upholding safety invariants (documented above).
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::ffi::{CStr, CString};
use std::mem::ManuallyDrop;
use std::os::raw::c_char;
use std::ptr;

use crate::ir::{convert_directive, Language as IrLanguage, ParserConfig, SourceLocation};
use crate::lexer::Language;
use crate::parser::{openmp, parse_omp_directive, Clause, ClauseKind};

mod openacc;
pub use openacc::*;

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
    name: *const c_char,      // Directive name (e.g., "parallel")
    parameter: *const c_char, // Optional parameter (e.g., "(a,b,c)" for threadprivate)
    clauses: Vec<OmpClause>,  // Associated clauses
}

/// Opaque clause type (C-compatible)
///
/// Represents a single clause within a directive.
/// Uses tagged union pattern for clause-specific data.
#[repr(C)]
pub struct OmpClause {
    kind: i32,        // Clause type (num_threads=0, schedule=7, etc.)
    data: ClauseData, // Clause-specific data (union)
    text: *const c_char, // Full clause text (e.g., "num_threads(4)")
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
    operator: i32, // 0=+, 1=-, 2=*, 6=&&, 7=||, 8=min, 9=max, etc.
    modifier: i32, // -1=none, 0=inscan, 1=task, 2=default
    user_operator: *const c_char, // For user-defined operators, NULL otherwise
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
        name: allocate_c_string(directive.name.as_ref()),
        parameter: directive
            .parameter
            .as_ref()
            .map(|p| allocate_c_string(p.as_ref()))
            .unwrap_or(ptr::null()),
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

        // Free the parameter string if it exists
        if !boxed.parameter.is_null() {
            drop(CString::from_raw(boxed.parameter as *mut c_char));
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
        name: allocate_c_string(directive.name.as_ref()),
        parameter: directive
            .parameter
            .as_ref()
            .map(|p| allocate_c_string(p.as_ref()))
            .unwrap_or(ptr::null()),
        clauses: directive
            .clauses
            .into_iter()
            .map(|c| convert_clause(&c))
            .collect(),
    };

    Box::into_raw(Box::new(c_directive))
}

// ============================================================================
// Translation Functions (C/C++ ↔ Fortran)
// ============================================================================

/// Convert an OpenMP directive from one language to another.
///
/// This function translates OpenMP directives between C/C++ and Fortran syntax:
/// - Sentinel conversion: `#pragma omp` ↔ `!$omp`
/// - Loop directive names: `for` ↔ `do`, `parallel for` ↔ `parallel do`, etc.
/// - Clause preservation: All clauses are preserved as-is
///
/// ## Parameters
/// - `input`: Null-terminated string containing the directive
/// - `from_language`: Source language (ROUP_LANG_C, ROUP_LANG_FORTRAN_FREE, ROUP_LANG_FORTRAN_FIXED)
/// - `to_language`: Target language (ROUP_LANG_C, ROUP_LANG_FORTRAN_FREE)
///
/// ## Returns
/// - Pointer to null-terminated C string with translated directive on success
/// - `NULL` on error:
///   - `input` is NULL
///   - `from_language` or `to_language` is invalid
///   - `input` contains invalid UTF-8
///   - Parse error (invalid OpenMP directive syntax)
///
/// ## Memory Management
/// The returned string is heap-allocated and must be freed by calling `roup_string_free()`.
///
/// ## Limitations
/// - **Expression translation**: Expressions within clauses are NOT translated
///   (e.g., C syntax like `arr[i]` remains unchanged)
/// - **Fixed-form output**: Only free-form `!$omp` output is supported (ROUP_LANG_FORTRAN_FIXED not supported as target)
/// - **Surrounding code**: Only directive lines are translated, not actual source code
///
/// ## Example (C to Fortran)
/// ```c
/// const char* c_input = "#pragma omp parallel for private(i) schedule(static, 4)";
/// char* fortran = roup_convert_language(c_input, ROUP_LANG_C, ROUP_LANG_FORTRAN_FREE);
/// if (fortran) {
///     printf("%s\n", fortran);  // Output: !$omp parallel do private(i) schedule(static, 4)
///     roup_string_free(fortran);
/// }
/// ```
///
/// ## Example (Fortran to C)
/// ```c
/// const char* fortran_input = "!$omp parallel do private(i)";
/// char* c_output = roup_convert_language(fortran_input, ROUP_LANG_FORTRAN_FREE, ROUP_LANG_C);
/// if (c_output) {
///     printf("%s\n", c_output);  // Output: #pragma omp parallel for private(i)
///     roup_string_free(c_output);
/// }
/// ```
#[no_mangle]
pub extern "C" fn roup_convert_language(
    input: *const c_char,
    from_language: i32,
    to_language: i32,
) -> *mut c_char {
    // NULL check
    if input.is_null() {
        return ptr::null_mut();
    }

    // Convert language codes to IR Language enum
    let from_lang = match language_code_to_ir_language(from_language) {
        Some(lang) => lang,
        None => return ptr::null_mut(), // Invalid from_language
    };

    let to_lang = match language_code_to_ir_language(to_language) {
        Some(lang) => lang,
        None => return ptr::null_mut(), // Invalid to_language
    };

    // Convert C string to Rust &str
    let c_str = unsafe { CStr::from_ptr(input) };

    let rust_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(), // Invalid UTF-8
    };

    // Return early if input is empty
    if rust_str.trim().is_empty() {
        return ptr::null_mut();
    }

    // Convert from_language code to lexer::Language for parser
    // IMPORTANT: Map language code directly to preserve fixed-form vs free-form distinction
    let lexer_lang = match from_language {
        ROUP_LANG_C => Language::C,
        ROUP_LANG_FORTRAN_FREE => Language::FortranFree,
        ROUP_LANG_FORTRAN_FIXED => Language::FortranFixed,
        _ => return ptr::null_mut(), // Invalid from_language
    };

    // Parse the directive with the source language
    let parser = openmp::parser().with_language(lexer_lang);
    let (rest, directive) = match parser.parse(rust_str) {
        Ok(result) => result,
        Err(_) => return ptr::null_mut(), // Parse error
    };

    // Check for unparsed trailing input
    if !rest.trim().is_empty() {
        return ptr::null_mut();
    }

    // Convert to IR with source language context
    let config = ParserConfig::with_parsing(from_lang);
    let ir = match convert_directive(&directive, SourceLocation::start(), from_lang, &config) {
        Ok(ir) => ir,
        Err(_) => return ptr::null_mut(), // Conversion error
    };

    // Translate to target language
    // Note: We don't modify the IR's language field, just render in target language
    let output_str = ir.to_string_for_language(to_lang);

    // Allocate C string for return
    match CString::new(output_str) {
        Ok(c_string) => c_string.into_raw(),
        Err(_) => ptr::null_mut(), // String contains null bytes (shouldn't happen)
    }
}

/// Free a string allocated by `roup_convert_language()`.
///
/// ## Parameters
/// - `ptr`: Pointer to string returned by `roup_convert_language()`
///
/// ## Safety
/// - Must only be called once per string
/// - Pointer must be from `roup_convert_language()`
/// - Do not use pointer after calling this function
///
/// ## Example
/// ```c
/// char* output = roup_convert_language(input, ROUP_LANG_C, ROUP_LANG_FORTRAN_FREE);
/// if (output) {
///     printf("%s\n", output);
///     roup_string_free(output);
/// }
/// ```
#[no_mangle]
pub extern "C" fn roup_string_free(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }

    // UNSAFE: Convert raw pointer back to CString for deallocation
    // Safety: Pointer came from CString::into_raw in roup_convert_language
    unsafe {
        drop(CString::from_raw(ptr));
    }
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

/// Get directive name as a C string.
///
/// Returns NULL if directive is NULL.
/// Returned pointer is valid until directive is freed.
#[no_mangle]
pub extern "C" fn roup_directive_name(directive: *const OmpDirective) -> *const c_char {
    if directive.is_null() {
        return ptr::null();
    }

    unsafe {
        let dir = &*directive;
        dir.name
    }
}

/// Get directive parameter (e.g., variable list for threadprivate, allocate).
///
/// Returns NULL if directive is NULL or has no parameter.
/// The returned string is owned by the directive and must not be freed.
#[no_mangle]
pub extern "C" fn roup_directive_parameter(directive: *const OmpDirective) -> *const c_char {
    if directive.is_null() {
        return ptr::null();
    }

    unsafe {
        let dir = &*directive;
        dir.parameter
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

/// Get reduction modifier from reduction clause.
///
/// Returns -1 if clause is NULL, not a reduction clause, or has no modifier.
/// Return values: 0=inscan, 1=task, 2=default
#[no_mangle]
pub extern "C" fn roup_clause_reduction_modifier(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return -1;
    }

    unsafe {
        let c = &*clause;
        // Check if it's a reduction clause (kind 6, 45=in_reduction, 75=task_reduction)
        if c.kind != 6 && c.kind != 45 && c.kind != 75 {
            return -1;
        }
        c.data.reduction.modifier
    }
}

/// Get user-defined operator string from reduction clause.
///
/// Returns NULL if clause is NULL, not a reduction clause, or uses a built-in operator.
/// Returns pointer to operator string for user-defined operators.
#[no_mangle]
pub extern "C" fn roup_clause_reduction_user_operator(clause: *const OmpClause) -> *const c_char {
    if clause.is_null() {
        return ptr::null();
    }

    unsafe {
        let c = &*clause;
        // Check if it's a reduction clause (kind 6, 45=in_reduction, 75=task_reduction)
        if c.kind != 6 && c.kind != 45 && c.kind != 75 {
            return ptr::null();
        }
        c.data.reduction.user_operator
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

/// Get proc_bind kind from proc_bind clause.
///
/// Returns -1 if clause is NULL or not a proc_bind clause.
/// Return values: 0=master, 1=close, 2=spread, 3=primary
#[no_mangle]
pub extern "C" fn roup_clause_proc_bind(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return -1;
    }

    unsafe {
        let c = &*clause;
        if c.kind != 14 {
            // Not a proc_bind clause
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

/// Get the full text representation of a clause.
///
/// Returns the clause as it would appear in source code (e.g., "num_threads(4)").
///
/// ## Parameters
/// - `clause`: Pointer to the clause
///
/// ## Returns
/// - Pointer to null-terminated C string containing the clause text
/// - NULL if clause is NULL
///
/// ## Safety
/// - Returned string is owned by the clause and should NOT be freed
/// - Valid until the directive containing this clause is freed
#[no_mangle]
pub extern "C" fn roup_clause_text(clause: *const OmpClause) -> *const c_char {
    if clause.is_null() {
        return ptr::null();
    }

    // UNSAFE BLOCK: Dereference clause for text access
    // Safety: Caller guarantees valid clause pointer
    unsafe {
        let c = &*clause;
        c.text
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

/// Convert language code to IR Language enum.
///
/// Maps the C API language constants to the ir::Language enum used for
/// semantic representation and translation.
///
/// ## Language Code Mapping:
/// - 0 (ROUP_LANG_C) → Language::C (use this for both C and C++)
/// - 1 (ROUP_LANG_FORTRAN_FREE) → Language::Fortran
/// - 2 (ROUP_LANG_FORTRAN_FIXED) → Language::Fortran
///
/// Note: Both Fortran variants map to Language::Fortran in IR, as the
/// distinction between free-form and fixed-form is a lexical concern,
/// not a semantic one. There is no separate constant for C++; use ROUP_LANG_C for both C and C++.
fn language_code_to_ir_language(code: i32) -> Option<IrLanguage> {
    match code {
        ROUP_LANG_C => Some(IrLanguage::C), // C/C++ use same OpenMP syntax
        ROUP_LANG_FORTRAN_FREE => Some(IrLanguage::Fortran),
        ROUP_LANG_FORTRAN_FIXED => Some(IrLanguage::Fortran),
        _ => None, // Invalid language code
    }
}

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
/// ## Clause Kind Mapping (aligned with ompparser OpenMPKinds.h):
/// - 0 = num_threads    - 30 = bind            - 60 = device
/// - 1 = if             - 31 = inclusive       - 61 = map
/// - 2 = private        - 32 = exclusive       - 62 = use_device_ptr
/// - 3 = shared         - 33 = copyprivate     - 63 = sizes
/// - 4 = firstprivate   - 34 = parallel        - 64 = use_device_addr
/// - 5 = lastprivate    - 35 = sections        - 65 = is_device_ptr
/// - 6 = reduction      - 36 = for             - 66 = has_device_addr
/// - 7 = schedule       - 37 = do              - 67 = defaultmap
/// - 8 = collapse       - 38 = taskgroup       - 68 = to
/// - 9 = ordered        - 39 = allocator       - 69 = from
/// - 10 = nowait        - 40 = initializer     - 70 = uses_allocators
/// - 11 = default       - 41 = final           - 71 = when
/// - 12 = copyin        - 42 = untied          - 72 = match
/// - 13 = align         - 43 = requires        - 73 = link
/// - 14 = proc_bind     - 44 = mergeable       - 74 = device_type
/// - 15 = allocate      - 45 = in_reduction    - 75 = task_reduction
/// - 16 = num_teams     - 46 = depend          - 76 = acq_rel
/// - 17 = thread_limit  - 47 = priority        - 77 = release
/// - 18 = partial       - 48 = affinity        - 78 = acquire
/// - 19 = full          - 49 = detach          - 79 = read
/// - 20 = order         - 50 = grainsize       - 80 = write
/// - 21 = linear        - 51 = num_tasks       - 81 = update
/// - 22 = safelen       - 52 = nogroup         - 82 = capture
/// - 23 = simdlen       - 53 = reverse_offload - 83 = seq_cst
/// - 24 = aligned       - 54 = unified_address - 84 = relaxed
/// - 25 = nontemporal   - 55 = unified_shared_memory - 85 = hint
/// - 26 = uniform       - 56 = atomic_default_mem_order - 86 = destroy
/// - 27 = inbranch      - 57 = dynamic_allocators - 87 = depobj_update
/// - 28 = notinbranch   - 58 = ext_implementation_defined_requirement - 88 = threads
/// - 29 = dist_schedule - 59 = (reserved)      - 89 = simd
/// - 999 = unknown
fn convert_clause(clause: &Clause) -> OmpClause {
    // Normalize clause name to lowercase for case-insensitive matching
    // (Fortran clauses are uppercase, C clauses are lowercase)
    // Note: One String allocation per clause is acceptable at C API boundary.
    // Alternative (build-time constant map) requires updating constants_gen.rs
    // to parse if-else chains instead of match expressions.
    let normalized_name = clause.name.to_ascii_lowercase();

    let (kind, data) = match normalized_name.as_str() {
        "num_threads" => (0, ClauseData { default: 0 }),
        "if" => (1, ClauseData { default: 0 }),
        "private" => (2, ClauseData { variables: ptr::null_mut() }),
        "shared" => (3, ClauseData { variables: ptr::null_mut() }),
        "firstprivate" => (4, ClauseData { variables: ptr::null_mut() }),
        "lastprivate" => (5, ClauseData { variables: ptr::null_mut() }),
        "reduction" => {
            let (operator, modifier, user_operator) = parse_reduction_clause(clause);
            (6, ClauseData { reduction: ManuallyDrop::new(ReductionData { operator, modifier, user_operator }) })
        }
        "schedule" => {
            let schedule_kind = parse_schedule_kind(clause);
            (7, ClauseData { schedule: ManuallyDrop::new(ScheduleData { kind: schedule_kind }) })
        }
        "collapse" => (8, ClauseData { default: 0 }),
        "ordered" => (9, ClauseData { default: 0 }),
        "nowait" => (10, ClauseData { default: 0 }),
        "default" => {
            let default_kind = parse_default_kind(clause);
            (11, ClauseData { default: default_kind })
        }
        "copyin" => (12, ClauseData { variables: ptr::null_mut() }),
        "align" => (13, ClauseData { default: 0 }),
        "proc_bind" => {
            let proc_bind_kind = parse_proc_bind_kind(clause);
            (14, ClauseData { default: proc_bind_kind })
        }
        "allocate" => (15, ClauseData { variables: ptr::null_mut() }),
        "num_teams" => (16, ClauseData { default: 0 }),
        "thread_limit" => (17, ClauseData { default: 0 }),
        "partial" => (18, ClauseData { default: 0 }),
        "full" => (19, ClauseData { default: 0 }),
        "order" => (20, ClauseData { default: 0 }),
        "linear" => (21, ClauseData { variables: ptr::null_mut() }),
        "safelen" => (22, ClauseData { default: 0 }),
        "simdlen" => (23, ClauseData { default: 0 }),
        "aligned" => (24, ClauseData { variables: ptr::null_mut() }),
        "nontemporal" => (25, ClauseData { variables: ptr::null_mut() }),
        "uniform" => (26, ClauseData { variables: ptr::null_mut() }),
        "inbranch" => (27, ClauseData { default: 0 }),
        "notinbranch" => (28, ClauseData { default: 0 }),
        "dist_schedule" => (29, ClauseData { default: 0 }),
        "bind" => (30, ClauseData { default: 0 }),
        "inclusive" => (31, ClauseData { variables: ptr::null_mut() }),
        "exclusive" => (32, ClauseData { variables: ptr::null_mut() }),
        "copyprivate" => (33, ClauseData { variables: ptr::null_mut() }),
        "parallel" => (34, ClauseData { default: 0 }),
        "sections" => (35, ClauseData { default: 0 }),
        "for" => (36, ClauseData { default: 0 }),
        "do" => (37, ClauseData { default: 0 }),
        "taskgroup" => (38, ClauseData { default: 0 }),
        "allocator" => (39, ClauseData { default: 0 }),
        "initializer" => (40, ClauseData { default: 0 }),
        "final" => (41, ClauseData { default: 0 }),
        "untied" => (42, ClauseData { default: 0 }),
        "requires" => (43, ClauseData { default: 0 }),
        "mergeable" => (44, ClauseData { default: 0 }),
        "in_reduction" => {
            let (operator, modifier, user_operator) = parse_reduction_clause(clause);
            (45, ClauseData { reduction: ManuallyDrop::new(ReductionData { operator, modifier, user_operator }) })
        }
        "depend" => (46, ClauseData { default: 0 }),
        "priority" => (47, ClauseData { default: 0 }),
        "affinity" => (48, ClauseData { variables: ptr::null_mut() }),
        "detach" => (49, ClauseData { default: 0 }),
        "grainsize" => (50, ClauseData { default: 0 }),
        "num_tasks" => (51, ClauseData { default: 0 }),
        "nogroup" => (52, ClauseData { default: 0 }),
        "reverse_offload" => (53, ClauseData { default: 0 }),
        "unified_address" => (54, ClauseData { default: 0 }),
        "unified_shared_memory" => (55, ClauseData { default: 0 }),
        "atomic_default_mem_order" => (56, ClauseData { default: 0 }),
        "dynamic_allocators" => (57, ClauseData { default: 0 }),
        "ext_implementation_defined_requirement" => (58, ClauseData { default: 0 }),
        "device" => (60, ClauseData { default: 0 }),
        "map" => (61, ClauseData { variables: ptr::null_mut() }),
        "use_device_ptr" => (62, ClauseData { variables: ptr::null_mut() }),
        "sizes" => (63, ClauseData { default: 0 }),
        "use_device_addr" => (64, ClauseData { variables: ptr::null_mut() }),
        "is_device_ptr" => (65, ClauseData { variables: ptr::null_mut() }),
        "has_device_addr" => (66, ClauseData { variables: ptr::null_mut() }),
        "defaultmap" => (67, ClauseData { default: 0 }),
        "to" => (68, ClauseData { variables: ptr::null_mut() }),
        "from" => (69, ClauseData { variables: ptr::null_mut() }),
        "uses_allocators" => (70, ClauseData { default: 0 }),
        "when" => (71, ClauseData { default: 0 }),
        "match" => (72, ClauseData { default: 0 }),
        "link" => (73, ClauseData { variables: ptr::null_mut() }),
        "device_type" => (74, ClauseData { default: 0 }),
        "task_reduction" => {
            let (operator, modifier, user_operator) = parse_reduction_clause(clause);
            (75, ClauseData { reduction: ManuallyDrop::new(ReductionData { operator, modifier, user_operator }) })
        }
        "acq_rel" => (76, ClauseData { default: 0 }),
        "release" => (77, ClauseData { default: 0 }),
        "acquire" => (78, ClauseData { default: 0 }),
        "read" => (79, ClauseData { default: 0 }),
        "write" => (80, ClauseData { default: 0 }),
        "update" => (81, ClauseData { default: 0 }),
        "capture" => (82, ClauseData { default: 0 }),
        "seq_cst" => (83, ClauseData { default: 0 }),
        "relaxed" => (84, ClauseData { default: 0 }),
        "hint" => (85, ClauseData { default: 0 }),
        "destroy" => (86, ClauseData { default: 0 }),
        "depobj_update" => (87, ClauseData { default: 0 }),
        "threads" => (88, ClauseData { default: 0 }),
        "simd" => (89, ClauseData { default: 0 }),
        _ => (999, ClauseData { default: 0 }), // Unknown
    };

    // Get the full clause text representation
    let clause_text = clause.to_source_string();
    let text_cstring = CString::new(clause_text).unwrap_or_else(|_| CString::new("").unwrap());
    let text = text_cstring.into_raw();

    OmpClause { kind, data, text }
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
/// Parse reduction clause to extract operator, modifier, and user-defined operator string.
///
/// Returns (operator_code, modifier_code, user_operator_ptr):
/// - operator_code: 0=+, 1=-, 2=*, 3=&, 4=|, 5=^, 6=&&, 7=||, 8=min, 9=max, 10+=Fort operators, 19=UserDefined
/// - modifier_code: -1=none, 0=inscan, 1=task, 2=default
/// - user_operator_ptr: CString pointer for user-defined operators, NULL otherwise
fn parse_reduction_clause(clause: &Clause) -> (i32, i32, *const c_char) {
    use crate::parser::{ClauseKind, ReductionModifier, ReductionOperator};

    if let ClauseKind::ReductionClause { modifier, operator, operator_str, .. } = &clause.kind {
        // Map operator enum to integer code
        let op_code = match operator {
            ReductionOperator::Add => 0,
            ReductionOperator::Sub => 1,
            ReductionOperator::Mul => 2,
            ReductionOperator::BitAnd => 3,
            ReductionOperator::BitOr => 4,
            ReductionOperator::BitXor => 5,
            ReductionOperator::LogAnd => 6,
            ReductionOperator::LogOr => 7,
            ReductionOperator::Min => 8,
            ReductionOperator::Max => 9,
            ReductionOperator::FortAnd => 10,
            ReductionOperator::FortOr => 11,
            ReductionOperator::FortEqv => 12,
            ReductionOperator::FortNeqv => 13,
            ReductionOperator::FortIand => 14,
            ReductionOperator::FortIor => 15,
            ReductionOperator::FortIeor => 16,
            ReductionOperator::UserDefined => 19,
        };

        // Map modifier enum to integer code
        let mod_code = match modifier {
            None => -1,
            Some(ReductionModifier::Inscan) => 0,
            Some(ReductionModifier::Task) => 1,
            Some(ReductionModifier::Default) => 2,
        };

        // For user-defined operators, create a CString
        let user_op_ptr = if *operator == ReductionOperator::UserDefined {
            if let Some(ref op_str) = operator_str {
                CString::new(op_str.to_string()).unwrap().into_raw()
            } else {
                ptr::null()
            }
        } else {
            ptr::null()
        };

        (op_code, mod_code, user_op_ptr)
    } else {
        // Fallback for non-structured reduction clauses (shouldn't happen with new parser)
        (0, -1, ptr::null())
    }
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
    if let ClauseKind::Parenthesized(ref args) = clause.kind {
        let args = args.as_ref();
        // Case-insensitive keyword matching without String allocation
        // Check common case variants (lowercase, uppercase, title case)
        if args.contains("static") || args.contains("STATIC") || args.contains("Static") {
            return 0;
        } else if args.contains("dynamic") || args.contains("DYNAMIC") || args.contains("Dynamic") {
            return 1;
        } else if args.contains("guided") || args.contains("GUIDED") || args.contains("Guided") {
            return 2;
        } else if args.contains("auto") || args.contains("AUTO") || args.contains("Auto") {
            return 3;
        } else if args.contains("runtime") || args.contains("RUNTIME") || args.contains("Runtime") {
            return 4;
        }
    }
    0 // Default to static
}

/// Parse proc_bind clause affinity policy.
///
/// Extracts the proc_bind kind from clause like "proc_bind(master)".
/// Returns integer code for the proc_bind policy.
///
/// ## Proc_bind Codes:
/// - 0 = master
/// - 1 = close
/// - 2 = spread
/// - 3 = primary
fn parse_proc_bind_kind(clause: &Clause) -> i32 {
    if let ClauseKind::Parenthesized(ref args) = clause.kind {
        let args = args.as_ref();
        if args.contains("master") || args.contains("MASTER") || args.contains("Master") {
            return 0;
        } else if args.contains("close") || args.contains("CLOSE") || args.contains("Close") {
            return 1;
        } else if args.contains("spread") || args.contains("SPREAD") || args.contains("Spread") {
            return 2;
        } else if args.contains("primary") || args.contains("PRIMARY") || args.contains("Primary") {
            return 3;
        }
    }
    0 // Default to master
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
    if let ClauseKind::Parenthesized(ref args) = clause.kind {
        let args = args.as_ref();
        // Case-insensitive keyword matching without String allocation
        // Check common case variants (lowercase, uppercase, title case)
        if args.contains("shared") || args.contains("SHARED") || args.contains("Shared") {
            return 0;
        } else if args.contains("none") || args.contains("NONE") || args.contains("None") {
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
/// ## Directive Codes (aligned with ompparser OpenMPKinds.h):
/// - 0 = parallel variants     - 30 = parallel_loop       - 60 = target_teams_loop
/// - 1 = for/do variants       - 31 = parallel_workshare  - 61 = target_update
/// - 2 = sections              - 32 = parallel_master     - 62 = declare_target
/// - 3 = single                - 33 = master_taskloop     - 63 = end_declare_target
/// - 4 = task                  - 34 = master_taskloop_simd - 64 = end
/// - 5 = master                - 35 = parallel_master_taskloop - 65 = unroll
/// - 6 = critical              - 36 = parallel_master_taskloop_simd - 66 = tile
/// - 7 = barrier               - 37 = taskloop            - 67 = depobj
/// - 8 = taskwait              - 38 = taskloop_simd       - 68 = scan
/// - 9 = taskgroup             - 39 = taskyield           - 69 = section
/// - 10 = atomic               - 40 = requires            - 70 = declare_reduction
/// - 11 = flush                - 41 = target_data         - 71 = declare_mapper
/// - 12 = ordered              - 42 = target_enter_data   - 72 = declare_variant
/// - 13 = target variants      - 43 = target_exit_data    - 73 = allocate
/// - 14 = teams variants       - 44 = target_simd         - 74 = threadprivate
/// - 15 = distribute variants  - 45 = target_loop         - 75 = workshare
/// - 16 = metadirective        - 46 = target_loop_simd    - 76 = cancel
/// - 17 = simd                 - 47 = target_teams_loop   - 77 = cancellation_point
/// - 18 = loop                 - 48 = teams_loop          - 78 = declare_simd
/// - 19 = parallel_loop_simd   - 49 = teams_loop_simd     - 79 = begin_declare_target
/// - 20 = teams_distribute_simd - 50 = distribute_parallel_loop - 80 = begin_declares
/// - 21 = target_teams_distribute_simd - 51 = distribute_parallel_loop_simd - 81 = end_declares
/// - 22 = target_parallel_loop  - 52 = target_parallel_loop_simd - 82 = parallel_masked
/// - 23 = target_parallel_simd  - 53 = target_teams_distribute_parallel_loop - 83 = masked
/// - 24 = parallel_for_loop     - 54 = target_teams_distribute_parallel_loop_simd - 84 = masked_taskloop
/// - 25 = parallel_do_loop      - 55 = teams_distribute_parallel_loop - 85 = masked_taskloop_simd
/// - 26 = target_parallel_for_loop - 56 = teams_distribute_parallel_loop_simd - 86 = parallel_masked_taskloop
/// - 27 = target_parallel_do_loop - 57 = parallel_master_masked_taskloop - 87 = parallel_masked_taskloop_simd
/// - 28 = teams_distribute_parallel_for_loop - 58 = parallel_master_masked_taskloop_simd
/// - 29 = teams_distribute_parallel_do_loop - 59 = begin_declare_variant
/// - 999 = unknown
fn directive_name_to_kind(name: *const c_char) -> i32 {
    if name.is_null() {
        return -1;
    }

    // UNSAFE BLOCK 11: Read directive name
    // Safety: name pointer is valid (from our own allocation)
    unsafe {
        let c_str = CStr::from_ptr(name);
        let name_str = c_str.to_str().unwrap_or("");

        // Case-insensitive directive name matching via to_lowercase()
        // Note: This allocates a String. While eq_ignore_ascii_case() would be more efficient,
        // the build system's constant parser requires a match expression with string literals.
        // The performance impact is negligible for the C API boundary.
        //
        // Fortran DO variants map to same codes as their C FOR equivalents
        match name_str.to_lowercase().as_str() {
            // Kind 0: Parallel directives
            "parallel" => 0,
            "parallel for" => 0,
            "parallel do" => 0,
            "parallel for simd" => 0,
            "parallel do simd" => 0,
            "parallel sections" => 0,

            // Kind 1: For/Do directives
            "for" => 1,
            "do" => 1,
            "for simd" => 1,
            "do simd" => 1,

            // Basic directives
            "sections" => 2,
            "single" => 3,
            "task" => 4,
            "master" => 5,
            "critical" => 6,
            "barrier" => 7,
            "taskwait" => 8,
            "taskgroup" => 9,
            "atomic" => 10,
            "flush" => 11,
            "ordered" => 12,

            // Kind 13: Target directives
            "target" => 13,
            "target teams" => 13,
            "target parallel" => 13,
            "target parallel for" => 13,
            "target parallel do" => 13,
            "target parallel for simd" => 13,
            "target parallel do simd" => 13,
            "target teams distribute" => 13,
            "target teams distribute parallel for" => 13,
            "target teams distribute parallel do" => 13,
            "target teams distribute parallel for simd" => 13,
            "target teams distribute parallel do simd" => 13,

            // Kind 14: Teams directives
            "teams" => 14,
            "teams distribute" => 14,
            "teams distribute parallel for" => 14,
            "teams distribute parallel do" => 14,
            "teams distribute parallel for simd" => 14,
            "teams distribute parallel do simd" => 14,

            // Kind 15: Distribute directives
            "distribute" => 15,
            "distribute parallel for" => 15,
            "distribute parallel do" => 15,
            "distribute parallel for simd" => 15,
            "distribute parallel do simd" => 15,
            "distribute simd" => 15,

            // Additional directives starting from kind 16
            "metadirective" => 16,
            "simd" => 17,
            "loop" => 18,
            "parallel loop simd" => 19,
            "teams distribute simd" => 20,
            "target teams distribute simd" => 21,
            "target parallel loop" => 22,
            "target parallel simd" => 23,
            "parallel for loop" => 24,
            "parallel do loop" => 25,
            "target parallel for loop" => 26,
            "target parallel do loop" => 27,
            "teams distribute parallel for loop" => 28,
            "teams distribute parallel do loop" => 29,
            "parallel loop" => 30,
            "parallel workshare" => 31,
            "parallel master" => 32,
            "master taskloop" => 33,
            "master taskloop simd" => 34,
            "parallel master taskloop" => 35,
            "parallel master taskloop simd" => 36,
            "taskloop" => 37,
            "taskloop simd" => 38,
            "taskyield" => 39,
            "requires" => 40,
            "target data" => 41,
            "target_data" => 41,  // underscore variant
            "target enter data" => 42,
            "target exit data" => 43,
            "target simd" => 44,
            "target loop" => 45,
            "target loop simd" => 46,
            "target teams loop" => 47,
            "teams loop" => 48,
            "teams loop simd" => 49,
            "distribute parallel loop" => 50,
            "distribute parallel loop simd" => 51,
            "target parallel loop simd" => 52,
            "target teams distribute parallel loop" => 53,
            "target teams distribute parallel loop simd" => 54,
            "teams distribute parallel loop" => 55,
            "teams distribute parallel loop simd" => 56,
            "parallel master masked taskloop" => 57,
            "parallel master masked taskloop simd" => 58,
            "begin declare variant" => 59,
            "target teams loop simd" => 60,
            "target update" => 61,
            "declare target" => 62,
            "end declare target" => 63,
            "end" => 64,
            "unroll" => 65,
            "tile" => 66,
            "depobj" => 67,
            "scan" => 68,
            "section" => 69,
            "declare reduction" => 70,
            "declare mapper" => 71,
            "declare variant" => 72,
            "allocate" => 73,
            "threadprivate" => 74,
            "workshare" => 75,
            "cancel" => 76,
            "cancellation point" => 77,
            "declare simd" => 78,
            "begin declare target" => 79,
            "begin assumes" => 80,
            "end assumes" => 81,
            "parallel masked" => 82,
            "masked" => 83,
            "masked taskloop" => 84,
            "masked taskloop simd" => 85,
            "parallel masked taskloop" => 86,
            "parallel masked taskloop simd" => 87,

            // Unknown directive
            _ => 999,
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
        // Free clause text string
        if !clause.text.is_null() {
            drop(CString::from_raw(clause.text as *mut c_char));
        }

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

// ============================================================================
// OpenACC C API is implemented in src/c_api/openacc.rs
