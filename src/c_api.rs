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
use crate::parser::{openacc, openmp, parse_omp_directive, Clause, ClauseKind};

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
        name: allocate_c_string(directive.name.as_ref()),
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
        name: allocate_c_string(directive.name.as_ref()),
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
/// ## Clause Kind Mapping:
/// - 0 = num_threads    - 6 = reduction
/// - 1 = if             - 7 = schedule
/// - 2 = private        - 8 = collapse
/// - 3 = shared         - 9 = ordered
/// - 4 = firstprivate   - 10 = nowait
/// - 5 = lastprivate    - 11 = default
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
        "private" => (
            2,
            ClauseData {
                variables: ptr::null_mut(),
            },
        ),
        "shared" => (
            3,
            ClauseData {
                variables: ptr::null_mut(),
            },
        ),
        "firstprivate" => (
            4,
            ClauseData {
                variables: ptr::null_mut(),
            },
        ),
        "lastprivate" => (
            5,
            ClauseData {
                variables: ptr::null_mut(),
            },
        ),
        "reduction" => {
            let operator = parse_reduction_operator(clause);
            (
                6,
                ClauseData {
                    reduction: ManuallyDrop::new(ReductionData { operator }),
                },
            )
        }
        "schedule" => {
            let schedule_kind = parse_schedule_kind(clause);
            (
                7,
                ClauseData {
                    schedule: ManuallyDrop::new(ScheduleData {
                        kind: schedule_kind,
                    }),
                },
            )
        }
        "collapse" => (8, ClauseData { default: 0 }),
        "ordered" => (9, ClauseData { default: 0 }),
        "nowait" => (10, ClauseData { default: 0 }),
        "default" => {
            let default_kind = parse_default_kind(clause);
            (
                11,
                ClauseData {
                    default: default_kind,
                },
            )
        }
        _ => (999, ClauseData { default: 0 }), // Unknown
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
    if let ClauseKind::Parenthesized(ref args) = clause.kind {
        let args = args.as_ref();
        // Operators (+, -, *, etc.) are ASCII symbols - no case conversion needed
        if args.contains('+') && !args.contains("++") {
            return 0; // Plus
        } else if args.contains('-') && !args.contains("--") {
            return 1; // Minus
        } else if args.contains('*') {
            return 2; // Times
        } else if args.contains('&') && !args.contains("&&") {
            return 3; // BitwiseAnd
        } else if args.contains('|') && !args.contains("||") {
            return 4; // BitwiseOr
        } else if args.contains('^') {
            return 5; // BitwiseXor
        } else if args.contains("&&") {
            return 6; // LogicalAnd
        } else if args.contains("||") {
            return 7; // LogicalOr
        }

        // For text keywords (min, max), normalize once for case-insensitive comparison
        let args_lower = args.to_ascii_lowercase();
        if args_lower.contains("min") {
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

        // Case-insensitive directive name matching via to_lowercase()
        // Note: This allocates a String. While eq_ignore_ascii_case() would be more efficient,
        // the build system's constant parser requires a match expression with string literals.
        // The performance impact is negligible for the C API boundary.
        //
        // Fortran DO variants map to same codes as their C FOR equivalents:
        // - "do" -> 1 (same as "for")
        // - "parallel do" -> 0 (same as "parallel for")
        // - "distribute parallel do" -> 15 (same as "distribute parallel for")
        // - "target parallel do" -> 13 (same as "target parallel for")
        // etc.
        match name_str.to_lowercase().as_str() {
            // Parallel directives (kind 0)
            "parallel" => 0,
            "parallel for" => 0,
            "parallel do" => 0, // Fortran variant
            "parallel for simd" => 0,
            "parallel do simd" => 0, // Fortran variant
            "parallel sections" => 0,

            // For/Do directives (kind 1)
            "for" => 1,
            "do" => 1, // Fortran variant
            "for simd" => 1,
            "do simd" => 1, // Fortran variant

            // Other basic directives
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

            // Target directives (kind 13)
            "target" => 13,
            "target teams" => 13,
            "target parallel" => 13,
            "target parallel for" => 13,
            "target parallel do" => 13, // Fortran variant
            "target parallel for simd" => 13,
            "target parallel do simd" => 13, // Fortran variant
            "target teams distribute" => 13,
            "target teams distribute parallel for" => 13,
            "target teams distribute parallel do" => 13, // Fortran variant
            "target teams distribute parallel for simd" => 13,
            "target teams distribute parallel do simd" => 13, // Fortran variant

            // Teams directives (kind 14)
            "teams" => 14,
            "teams distribute" => 14,
            "teams distribute parallel for" => 14,
            "teams distribute parallel do" => 14, // Fortran variant
            "teams distribute parallel for simd" => 14,
            "teams distribute parallel do simd" => 14, // Fortran variant

            // Distribute directives (kind 15)
            "distribute" => 15,
            "distribute parallel for" => 15,
            "distribute parallel do" => 15, // Fortran variant
            "distribute parallel for simd" => 15,
            "distribute parallel do simd" => 15, // Fortran variant
            "distribute simd" => 15,

            // Metadirective (kind 16)
            "metadirective" => 16,

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
// OpenACC C API Implementation
// ============================================================================
//
// This section provides a C FFI for OpenACC parsing, mirroring the OpenMP API
// design pattern. It enables C/C++ code (specifically the accparser compatibility
// layer) to use ROUP's OpenACC parser.
//
// Architecture: C code → acc_parse() → AccDirective → iterate clauses → free
//
// The API follows the same safety model as the OpenMP API above.

/// Opaque OpenACC directive type (C-compatible)
#[repr(C)]
pub struct AccDirective {
    name: *const c_char,
    clauses: Vec<AccClause>,
}

/// Opaque OpenACC clause type (C-compatible)
#[repr(C)]
pub struct AccClause {
    kind: i32,
    expressions: *mut AccStringList, // For variable lists, arguments, etc.
}

/// Iterator over OpenACC clauses
#[repr(C)]
pub struct AccClauseIterator {
    clauses: Vec<*const AccClause>,
    index: usize,
}

/// List of strings for OpenACC clause arguments
#[repr(C)]
pub struct AccStringList {
    items: Vec<*const c_char>,
}

// ============================================================================
// OpenACC Parse Functions
// ============================================================================

/// Parse an OpenACC directive from a C string.
///
/// ## Parameters
/// - `input`: Null-terminated C string containing the directive
///
/// ## Returns
/// - Pointer to `AccDirective` on success
/// - NULL on parse failure or NULL input
///
/// ## Safety
/// Caller must:
/// - Pass valid null-terminated C string or NULL
/// - Call `acc_directive_free()` on the returned pointer
///
/// ## Example
/// ```c
/// AccDirective* dir = acc_parse("#pragma acc parallel loop");
/// if (dir) {
///     // use directive
///     acc_directive_free(dir);
/// }
/// ```
#[no_mangle]
pub extern "C" fn acc_parse(input: *const c_char) -> *mut AccDirective {
    if input.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let c_str = CStr::from_ptr(input);
        let rust_str = match c_str.to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null_mut(),
        };

        let parser = openacc::parser();
        let directive = match parser.parse(rust_str) {
            Ok((_, dir)) => dir,
            Err(_) => return ptr::null_mut(),
        };

        let c_directive = AccDirective {
            name: allocate_c_string(directive.name.as_ref()),
            clauses: directive
                .clauses
                .into_iter()
                .map(|c| convert_acc_clause(&c))
                .collect(),
        };

        Box::into_raw(Box::new(c_directive))
    }
}

/// Parse an OpenACC directive with explicit language specification.
///
/// ## Parameters
/// - `input`: Null-terminated string containing the directive
/// - `language`: Language format (ROUP_LANG_C, ROUP_LANG_FORTRAN_FREE, ROUP_LANG_FORTRAN_FIXED)
///
/// ## Returns
/// - Pointer to `AccDirective` on success
/// - NULL on error
#[no_mangle]
pub extern "C" fn acc_parse_with_language(
    input: *const c_char,
    language: i32,
) -> *mut AccDirective {
    if input.is_null() {
        return ptr::null_mut();
    }

    let lang = match language {
        ROUP_LANG_C => Language::C,
        ROUP_LANG_FORTRAN_FREE => Language::FortranFree,
        ROUP_LANG_FORTRAN_FIXED => Language::FortranFixed,
        _ => return ptr::null_mut(),
    };

    unsafe {
        let c_str = CStr::from_ptr(input);
        let rust_str = match c_str.to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null_mut(),
        };

        let parser = openacc::parser().with_language(lang);
        let directive = match parser.parse(rust_str) {
            Ok((_, dir)) => dir,
            Err(_) => return ptr::null_mut(),
        };

        let c_directive = AccDirective {
            name: allocate_c_string(directive.name.as_ref()),
            clauses: directive
                .clauses
                .into_iter()
                .map(|c| convert_acc_clause(&c))
                .collect(),
        };

        Box::into_raw(Box::new(c_directive))
    }
}

/// Free an OpenACC directive allocated by `acc_parse()`.
#[no_mangle]
pub extern "C" fn acc_directive_free(directive: *mut AccDirective) {
    if directive.is_null() {
        return;
    }

    unsafe {
        let boxed = Box::from_raw(directive);

        if !boxed.name.is_null() {
            drop(CString::from_raw(boxed.name as *mut c_char));
        }

        for clause in &boxed.clauses {
            free_acc_clause_data(clause);
        }
    }
}

// ============================================================================
// OpenACC Directive Query Functions
// ============================================================================

/// Get OpenACC directive kind.
///
/// Returns -1 if directive is NULL.
#[no_mangle]
pub extern "C" fn acc_directive_kind(directive: *const AccDirective) -> i32 {
    if directive.is_null() {
        return -1;
    }

    unsafe {
        let dir = &*directive;
        acc_directive_name_to_kind(dir.name)
    }
}

/// Get OpenACC directive name as a C string.
///
/// Returns NULL if directive is NULL.
#[no_mangle]
pub extern "C" fn acc_directive_name(directive: *const AccDirective) -> *const c_char {
    if directive.is_null() {
        return ptr::null();
    }

    unsafe {
        let dir = &*directive;
        dir.name
    }
}

/// Get number of clauses in an OpenACC directive.
///
/// Returns 0 if directive is NULL.
#[no_mangle]
pub extern "C" fn acc_directive_clause_count(directive: *const AccDirective) -> i32 {
    if directive.is_null() {
        return 0;
    }

    unsafe {
        let dir = &*directive;
        dir.clauses.len() as i32
    }
}

/// Create an iterator over OpenACC directive clauses.
///
/// Returns NULL if directive is NULL.
/// Caller must call `acc_clause_iterator_free()`.
#[no_mangle]
pub extern "C" fn acc_directive_clauses_iter(
    directive: *const AccDirective,
) -> *mut AccClauseIterator {
    if directive.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let dir = &*directive;
        let iter = AccClauseIterator {
            clauses: dir.clauses.iter().map(|c| c as *const AccClause).collect(),
            index: 0,
        };
        Box::into_raw(Box::new(iter))
    }
}

// ============================================================================
// OpenACC Clause Iterator Functions
// ============================================================================

/// Get next clause from OpenACC iterator.
///
/// ## Returns
/// - 1 if clause available (written to `out`)
/// - 0 if no more clauses or NULL inputs
#[no_mangle]
pub extern "C" fn acc_clause_iterator_next(
    iter: *mut AccClauseIterator,
    out: *mut *const AccClause,
) -> i32 {
    if iter.is_null() || out.is_null() {
        return 0;
    }

    unsafe {
        let iterator = &mut *iter;

        if iterator.index >= iterator.clauses.len() {
            return 0;
        }

        let clause_ptr = iterator.clauses[iterator.index];
        iterator.index += 1;

        *out = clause_ptr;
        1
    }
}

/// Free OpenACC clause iterator.
#[no_mangle]
pub extern "C" fn acc_clause_iterator_free(iter: *mut AccClauseIterator) {
    if iter.is_null() {
        return;
    }

    unsafe {
        drop(Box::from_raw(iter));
    }
}

// ============================================================================
// OpenACC Clause Query Functions
// ============================================================================

/// Get OpenACC clause kind.
///
/// Returns -1 if clause is NULL.
#[no_mangle]
pub extern "C" fn acc_clause_kind(clause: *const AccClause) -> i32 {
    if clause.is_null() {
        return -1;
    }

    unsafe {
        let c = &*clause;
        c.kind
    }
}

/// Get number of expressions in an OpenACC clause.
///
/// Returns 0 if clause is NULL or has no expressions.
#[no_mangle]
pub extern "C" fn acc_clause_expressions_count(clause: *const AccClause) -> i32 {
    if clause.is_null() {
        return 0;
    }

    unsafe {
        let c = &*clause;
        if c.expressions.is_null() {
            return 0;
        }
        let list = &*c.expressions;
        list.items.len() as i32
    }
}

/// Get expression at index from OpenACC clause.
///
/// Returns NULL if clause is NULL, has no expressions, or index out of bounds.
#[no_mangle]
pub extern "C" fn acc_clause_expression_at(clause: *const AccClause, index: i32) -> *const c_char {
    if clause.is_null() || index < 0 {
        return ptr::null();
    }

    unsafe {
        let c = &*clause;
        if c.expressions.is_null() {
            return ptr::null();
        }

        let list = &*c.expressions;
        let idx = index as usize;

        if idx >= list.items.len() {
            return ptr::null();
        }

        list.items[idx]
    }
}

// ============================================================================
// OpenACC String List Functions
// ============================================================================

/// Free OpenACC string list.
#[no_mangle]
pub extern "C" fn acc_string_list_free(list: *mut AccStringList) {
    if list.is_null() {
        return;
    }

    unsafe {
        let boxed = Box::from_raw(list);

        for item_ptr in &boxed.items {
            if !item_ptr.is_null() {
                drop(CString::from_raw(*item_ptr as *mut c_char));
            }
        }
    }
}

// ============================================================================
// OpenACC Helper Functions (Internal)
// ============================================================================

/// Convert OpenACC directive name to kind enum code.
///
/// Maps directive names to integer codes for C switch statements.
///
/// ## Directive Codes (27 total, including variants):
/// - 0 = parallel           - 14 = kernels loop
/// - 1 = loop               - 15 = parallel loop
/// - 2 = kernels            - 16 = serial loop
/// - 3 = data               - 17 = serial
/// - 4 = enter data         - 18 = routine
/// - 5 = exit data          - 19 = set
/// - 6 = host_data          - 20 = init
/// - 7 = atomic             - 21 = shutdown
/// - 8 = declare            - 22 = update
/// - 9 = wait               - 23 = cache
/// - 10 = end               - 24 = enter_data (underscore variant)
/// - 11 = host data (space) - 25 = exit_data (underscore variant)
/// - 12 = update directive  - 26 = wait directive
/// - 13 = cache directive
/// - -1 = NULL/unknown
fn acc_directive_name_to_kind(name: *const c_char) -> i32 {
    if name.is_null() {
        return -1;
    }

    unsafe {
        let c_str = CStr::from_ptr(name);
        let name_str = c_str.to_str().unwrap_or("");

        // Case-insensitive matching
        match name_str.to_lowercase().as_str() {
            "parallel" => 0,
            "loop" => 1,
            "kernels" => 2,
            "data" => 3,
            "enter data" => 4,
            "exit data" => 5,
            "host_data" => 6,
            "atomic" => 7,
            "declare" => 8,
            "wait" => 9,
            "end" => 10,
            "host data" => 11,
            "update" => 12,
            "kernels loop" => 14,
            "parallel loop" => 15,
            "serial loop" => 16,
            "serial" => 17,
            "routine" => 18,
            "set" => 19,
            "init" => 20,
            "shutdown" => 21,
            "enter_data" => 24,
            "exit_data" => 25,
            // Special directives that may have embedded content
            name if name.starts_with("cache(") => 23,
            name if name.starts_with("wait(") => 26,
            name if name.starts_with("end ") => 10,
            _ => 999, // Unknown
        }
    }
}

/// Convert Rust Clause to C-compatible AccClause.
///
/// ## Clause Kind Mapping (45 clauses):
/// - 0  = async             - 15 = default          - 30 = finalize
/// - 1  = wait              - 16 = firstprivate     - 31 = if_present
/// - 2  = num_gangs         - 17 = default_async    - 32 = capture
/// - 3  = num_workers       - 18 = link             - 33 = write
/// - 4  = vector_length     - 19 = no_create        - 34 = update (clause)
/// - 5  = gang              - 20 = nohost           - 35 = copy / pcopy / present_or_copy
/// - 6  = worker            - 21 = present          - 36 = copyin / pcopyin / present_or_copyin
/// - 7  = vector            - 22 = private          - 37 = copyout / pcopyout / present_or_copyout
/// - 8  = seq               - 23 = reduction        - 38 = create / pcreate / present_or_create
/// - 9  = independent       - 24 = read             - 39 = delete
/// - 10 = auto              - 25 = self             - 40 = device
/// - 11 = collapse          - 26 = tile             - 41 = deviceptr
/// - 12 = device_type / dtype - 27 = use_device      - 42 = device_num
/// - 13 = bind              - 28 = attach           - 43 = device_resident
/// - 14 = if                - 29 = detach           - 44 = host
/// - 999 = unknown
fn convert_acc_clause(clause: &Clause) -> AccClause {
    let normalized_name = clause.name.to_ascii_lowercase();

    // Use tuple pattern for AST parser compatibility (constants_gen.rs)
    // The second tuple element is a dummy unit type that gets optimized away
    let (kind, _) = match normalized_name.as_str() {
        "async" => (0, ()),
        "wait" => (1, ()),
        "num_gangs" => (2, ()),
        "num_workers" => (3, ()),
        "vector_length" => (4, ()),
        "gang" => (5, ()),
        "worker" => (6, ()),
        "vector" => (7, ()),
        "seq" => (8, ()),
        "independent" => (9, ()),
        "auto" => (10, ()),
        "collapse" => (11, ()),
        "device_type" => (12, ()),
        "dtype" => (12, ()),
        "bind" => (13, ()),
        "if" => (14, ()),
        "default" => (15, ()),
        "firstprivate" => (16, ()),
        "default_async" => (17, ()),
        "link" => (18, ()),
        "no_create" => (19, ()),
        "nohost" => (20, ()),
        "present" => (21, ()),
        "private" => (22, ()),
        "reduction" => (23, ()),
        "read" => (24, ()),
        "self" => (25, ()),
        "tile" => (26, ()),
        "use_device" => (27, ()),
        "attach" => (28, ()),
        "detach" => (29, ()),
        "finalize" => (30, ()),
        "if_present" => (31, ()),
        "capture" => (32, ()),
        "write" => (33, ()),
        "update" => (34, ()),
        // Data clauses
        "copy" => (35, ()),
        "pcopy" => (35, ()),
        "present_or_copy" => (35, ()),
        "copyin" => (36, ()),
        "pcopyin" => (36, ()),
        "present_or_copyin" => (36, ()),
        "copyout" => (37, ()),
        "pcopyout" => (37, ()),
        "present_or_copyout" => (37, ()),
        "create" => (38, ()),
        "pcreate" => (38, ()),
        "present_or_create" => (38, ()),
        "delete" => (39, ()),
        "device" => (40, ()),
        "deviceptr" => (41, ()),
        "device_num" => (42, ()),
        "device_resident" => (43, ()),
        "host" => (44, ()),
        _ => (999, ()),
    };

    // Extract expressions from clause content
    let expressions = match &clause.kind {
        ClauseKind::Parenthesized(content) => {
            let content_str = content.as_ref();
            if !content_str.is_empty() {
                // For now, store the entire content as a single expression
                // More sophisticated parsing could split by commas, etc.
                let items = vec![allocate_c_string(content_str)];

                Box::into_raw(Box::new(AccStringList { items }))
            } else {
                ptr::null_mut()
            }
        }
        ClauseKind::Bare => ptr::null_mut(),
    };

    AccClause { kind, expressions }
}

/// Free OpenACC clause-specific data.
fn free_acc_clause_data(clause: &AccClause) {
    if !clause.expressions.is_null() {
        acc_string_list_free(clause.expressions);
    }
}

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
        assert_eq!(kind, 0, "PARALLEL directive should have kind 0, got {kind}");

        roup_directive_free(directive);

        // Test Fortran DO directive (equivalent to C FOR)
        let fortran_do = CString::new("!$OMP DO").unwrap();
        let directive = roup_parse_with_language(fortran_do.as_ptr(), ROUP_LANG_FORTRAN_FREE);
        assert!(!directive.is_null(), "Failed to parse Fortran DO directive");

        let kind = roup_directive_kind(directive);
        assert_eq!(
            kind, 1,
            "DO directive should have kind 1 (same as FOR), got {kind}"
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
            "PARALLEL DO directive should have kind 0 (composite), got {kind}"
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
        assert_eq!(clause_count, 1, "Should have 1 clause, got {clause_count}");

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
            "PRIVATE clause should have kind 2, got {clause_kind}"
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
        assert_eq!(clause_count, 1, "Should have 1 clause, got {clause_count}");

        let iter = roup_directive_clauses_iter(directive);
        let mut clause: *const OmpClause = ptr::null();
        let has_clause = roup_clause_iterator_next(iter, &mut clause);
        assert_eq!(has_clause, 1, "Should have a clause");

        let clause_kind = roup_clause_kind(clause);
        assert_eq!(
            clause_kind, 6,
            "REDUCTION clause should have kind 6, got {clause_kind}"
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
        assert_eq!(clause_count, 1, "Should have 1 clause, got {clause_count}");

        let iter = roup_directive_clauses_iter(directive);
        let mut clause: *const OmpClause = ptr::null();
        let has_clause = roup_clause_iterator_next(iter, &mut clause);
        assert_eq!(has_clause, 1, "Should have a clause");

        let clause_kind = roup_clause_kind(clause);
        assert_eq!(
            clause_kind, 7,
            "SCHEDULE clause should have kind 7, got {clause_kind}"
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
            "SCHEDULE(DYNAMIC) should have kind 1 (dynamic), got {schedule_kind}"
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
            "SCHEDULE(GUIDED) should have kind 2, got {schedule_kind}"
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
            "DEFAULT(NONE) should have kind 1 (none), got {default_kind}"
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
            "DEFAULT(SHARED) should have kind 0, got {default_kind}"
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
            "REDUCTION(MIN:X) should have operator 8 (min), got {reduction_op}"
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
            "REDUCTION(MAX:RESULT) should have operator 9 (max), got {reduction_op}"
        );

        roup_clause_iterator_free(iter);
        roup_directive_free(directive);
    }

    #[test]
    fn test_convert_c_to_fortran() {
        // Test basic C to Fortran translation
        let c_input = CString::new("#pragma omp parallel for").unwrap();
        let fortran = roup_convert_language(c_input.as_ptr(), ROUP_LANG_C, ROUP_LANG_FORTRAN_FREE);
        assert!(!fortran.is_null(), "Translation should succeed");

        let result = unsafe { CStr::from_ptr(fortran).to_str().unwrap() };
        assert_eq!(result, "!$omp parallel do");

        roup_string_free(fortran);
    }

    #[test]
    fn test_convert_fortran_to_c() {
        // Test basic Fortran to C translation
        let fortran_input = CString::new("!$omp parallel do").unwrap();
        let c_output =
            roup_convert_language(fortran_input.as_ptr(), ROUP_LANG_FORTRAN_FREE, ROUP_LANG_C);
        assert!(!c_output.is_null(), "Translation should succeed");

        let result = unsafe { CStr::from_ptr(c_output).to_str().unwrap() };
        assert_eq!(result, "#pragma omp parallel for");

        roup_string_free(c_output);
    }

    #[test]
    fn test_convert_with_clauses() {
        // Test C to Fortran with clauses
        let c_input =
            CString::new("#pragma omp parallel for private(i) schedule(static, 4)").unwrap();
        let fortran = roup_convert_language(c_input.as_ptr(), ROUP_LANG_C, ROUP_LANG_FORTRAN_FREE);
        assert!(!fortran.is_null(), "Translation should succeed");

        let result = unsafe { CStr::from_ptr(fortran).to_str().unwrap() };
        assert_eq!(result, "!$omp parallel do private(i) schedule(static, 4)");

        roup_string_free(fortran);
    }

    #[test]
    fn test_convert_complex_directive() {
        // Test complex directive translation
        let c_input =
            CString::new("#pragma omp target teams distribute parallel for simd collapse(2)")
                .unwrap();
        let fortran = roup_convert_language(c_input.as_ptr(), ROUP_LANG_C, ROUP_LANG_FORTRAN_FREE);
        assert!(!fortran.is_null(), "Translation should succeed");

        let result = unsafe { CStr::from_ptr(fortran).to_str().unwrap() };
        assert_eq!(
            result,
            "!$omp target teams distribute parallel do simd collapse(2)"
        );

        roup_string_free(fortran);
    }

    #[test]
    fn test_convert_for_only() {
        // Test standalone for/do directive
        let c_input = CString::new("#pragma omp for nowait").unwrap();
        let fortran = roup_convert_language(c_input.as_ptr(), ROUP_LANG_C, ROUP_LANG_FORTRAN_FREE);
        assert!(!fortran.is_null(), "Translation should succeed");

        let result = unsafe { CStr::from_ptr(fortran).to_str().unwrap() };
        assert_eq!(result, "!$omp do nowait");

        roup_string_free(fortran);
    }

    #[test]
    fn test_convert_non_loop_directive() {
        // Test non-loop directives (should remain unchanged)
        let c_input = CString::new("#pragma omp parallel").unwrap();
        let fortran = roup_convert_language(c_input.as_ptr(), ROUP_LANG_C, ROUP_LANG_FORTRAN_FREE);
        assert!(!fortran.is_null(), "Translation should succeed");

        let result = unsafe { CStr::from_ptr(fortran).to_str().unwrap() };
        assert_eq!(result, "!$omp parallel");

        roup_string_free(fortran);
    }

    #[test]
    fn test_convert_null_input() {
        // Test NULL input handling
        let result = roup_convert_language(ptr::null(), ROUP_LANG_C, ROUP_LANG_FORTRAN_FREE);
        assert!(result.is_null(), "NULL input should return NULL");
    }

    #[test]
    fn test_convert_invalid_language() {
        // Test invalid language codes
        let c_input = CString::new("#pragma omp parallel").unwrap();

        // Invalid from_language
        let result = roup_convert_language(c_input.as_ptr(), 999, ROUP_LANG_FORTRAN_FREE);
        assert!(result.is_null(), "Invalid from_language should return NULL");

        // Invalid to_language
        let result = roup_convert_language(c_input.as_ptr(), ROUP_LANG_C, 999);
        assert!(result.is_null(), "Invalid to_language should return NULL");
    }

    #[test]
    fn test_convert_empty_input() {
        // Test empty string handling
        let empty = CString::new("").unwrap();
        let result = roup_convert_language(empty.as_ptr(), ROUP_LANG_C, ROUP_LANG_FORTRAN_FREE);
        assert!(result.is_null(), "Empty input should return NULL");

        // Test whitespace-only input
        let whitespace = CString::new("   ").unwrap();
        let result =
            roup_convert_language(whitespace.as_ptr(), ROUP_LANG_C, ROUP_LANG_FORTRAN_FREE);
        assert!(result.is_null(), "Whitespace-only input should return NULL");
    }

    #[test]
    fn test_convert_parse_error() {
        // Test invalid directive syntax
        let invalid = CString::new("not a pragma").unwrap();
        let result = roup_convert_language(invalid.as_ptr(), ROUP_LANG_C, ROUP_LANG_FORTRAN_FREE);
        assert!(
            result.is_null(),
            "Invalid directive syntax should return NULL"
        );
    }

    #[test]
    fn test_string_free_null() {
        // Test that roup_string_free handles NULL gracefully
        roup_string_free(ptr::null_mut());
        // Should not crash
    }

    #[test]
    fn test_convert_fortran_fixed_form_to_c() {
        // Test uppercase fixed-form sentinel (C$OMP)
        let fortran_input = CString::new("C$OMP PARALLEL DO").unwrap();
        let c_output =
            roup_convert_language(fortran_input.as_ptr(), ROUP_LANG_FORTRAN_FIXED, ROUP_LANG_C);
        assert!(!c_output.is_null(), "Fixed-form translation should succeed");

        let result = unsafe { CStr::from_ptr(c_output).to_str().unwrap() };
        assert_eq!(result, "#pragma omp parallel for");

        roup_string_free(c_output);

        // Test lowercase fixed-form sentinel (c$omp)
        let fortran_input = CString::new("c$omp do schedule(dynamic)").unwrap();
        let c_output =
            roup_convert_language(fortran_input.as_ptr(), ROUP_LANG_FORTRAN_FIXED, ROUP_LANG_C);
        assert!(!c_output.is_null(), "Fixed-form translation should succeed");

        let result = unsafe { CStr::from_ptr(c_output).to_str().unwrap() };
        assert_eq!(result, "#pragma omp for schedule(dynamic)");

        roup_string_free(c_output);

        // Test asterisk fixed-form sentinel (*$omp)
        let fortran_input = CString::new("*$omp parallel").unwrap();
        let c_output =
            roup_convert_language(fortran_input.as_ptr(), ROUP_LANG_FORTRAN_FIXED, ROUP_LANG_C);
        assert!(!c_output.is_null(), "Fixed-form translation should succeed");

        let result = unsafe { CStr::from_ptr(c_output).to_str().unwrap() };
        assert_eq!(result, "#pragma omp parallel");

        roup_string_free(c_output);
    }

    #[test]
    fn test_convert_c_to_fortran_free_form() {
        // Verify C to Fortran always outputs free-form (not fixed-form)
        let c_input = CString::new("#pragma omp parallel for").unwrap();
        let fortran_free =
            roup_convert_language(c_input.as_ptr(), ROUP_LANG_C, ROUP_LANG_FORTRAN_FREE);
        assert!(!fortran_free.is_null());

        let result = unsafe { CStr::from_ptr(fortran_free).to_str().unwrap() };
        assert_eq!(result, "!$omp parallel do", "Output should be free-form");

        roup_string_free(fortran_free);
    }
}
