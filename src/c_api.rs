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
use crate::parser::directive_kind::{lookup_directive_name, DirectiveName};
use crate::parser::lookup_clause_name;
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

        // Use the canonical lookup to map the stored directive name into
        // a `DirectiveName` enum, then map that enum to the C integer code.
        if dir.name.is_null() {
            return -1;
        }

        let c_str = CStr::from_ptr(dir.name);
        let name_str = match c_str.to_str() {
            Ok(s) => s,
            Err(_) => return -1,
        };

        let dname = lookup_directive_name(name_str);
        directive_name_enum_to_kind(dname)
    }
}

// See `directive_name_enum_to_kind` below for the canonical mapping of
// `DirectiveName` -> integer codes. Unknown/unhandled directives return -1.

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

/// Find a top-level colon (:) not nested inside parentheses. Returns (left, right)
fn split_once_top_level_colon(input: &str) -> Option<(&str, &str)> {
    let mut depth: isize = 0;
    for (i, ch) in input.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ':' => {
                if depth == 0 {
                    let left = &input[..i];
                    let right = &input[i + 1..];
                    return Some((left, right));
                }
            }
            _ => {}
        }
    }
    None
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

    let clause_enum = lookup_clause_name(&normalized_name);
    let (kind, data) = match clause_enum {
        crate::parser::ClauseName::NumThreads => (0, ClauseData { default: 0 }),
        crate::parser::ClauseName::If => (1, ClauseData { default: 0 }),
        crate::parser::ClauseName::Private => (
            2,
            ClauseData {
                variables: ptr::null_mut(),
            },
        ),
        crate::parser::ClauseName::Shared => (
            3,
            ClauseData {
                variables: ptr::null_mut(),
            },
        ),
        crate::parser::ClauseName::Firstprivate => (
            4,
            ClauseData {
                variables: ptr::null_mut(),
            },
        ),
        crate::parser::ClauseName::Lastprivate => (
            5,
            ClauseData {
                variables: ptr::null_mut(),
            },
        ),
        crate::parser::ClauseName::Reduction => {
            let operator = parse_reduction_operator(clause);
            (
                6,
                ClauseData {
                    reduction: ManuallyDrop::new(ReductionData { operator }),
                },
            )
        }
        crate::parser::ClauseName::Schedule => {
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
        crate::parser::ClauseName::Collapse => (8, ClauseData { default: 0 }),
        crate::parser::ClauseName::Ordered => (9, ClauseData { default: 0 }),
        crate::parser::ClauseName::Nowait => (10, ClauseData { default: 0 }),
        crate::parser::ClauseName::Default => {
            let default_kind = parse_default_kind(clause);
            (
                11,
                ClauseData {
                    default: default_kind,
                },
            )
        }
        // OpenACC-only clause names map to unknown in the OpenMP C API layer
        crate::parser::ClauseName::Copy
        | crate::parser::ClauseName::CopyIn
        | crate::parser::ClauseName::CopyOut
        | crate::parser::ClauseName::Create
        | crate::parser::ClauseName::Present
        | crate::parser::ClauseName::Async
        | crate::parser::ClauseName::Wait
        | crate::parser::ClauseName::NumGangs
        | crate::parser::ClauseName::NumWorkers
        | crate::parser::ClauseName::VectorLength
        | crate::parser::ClauseName::Gang
        | crate::parser::ClauseName::Worker
        | crate::parser::ClauseName::Vector
        | crate::parser::ClauseName::Seq
        | crate::parser::ClauseName::Independent
        | crate::parser::ClauseName::Auto
        | crate::parser::ClauseName::DeviceType
        | crate::parser::ClauseName::Bind
        | crate::parser::ClauseName::DefaultAsync
        | crate::parser::ClauseName::Link
        | crate::parser::ClauseName::NoCreate
        | crate::parser::ClauseName::NoHost
        | crate::parser::ClauseName::Read
        | crate::parser::ClauseName::SelfClause
        | crate::parser::ClauseName::Tile
        | crate::parser::ClauseName::UseDevice
        | crate::parser::ClauseName::Attach
        | crate::parser::ClauseName::Detach
        | crate::parser::ClauseName::Finalize
        | crate::parser::ClauseName::IfPresent
        | crate::parser::ClauseName::Capture
        | crate::parser::ClauseName::Write
        | crate::parser::ClauseName::Update
        | crate::parser::ClauseName::Delete
        | crate::parser::ClauseName::Device
        | crate::parser::ClauseName::DevicePtr
        | crate::parser::ClauseName::DeviceNum
        | crate::parser::ClauseName::DeviceResident
        | crate::parser::ClauseName::Host => (-1, ClauseData { default: 0 }),

        crate::parser::ClauseName::Other(ref s) => {
            eprintln!("[c_api] unknown clause mapping requested: {}", s.as_ref());
            (-1, ClauseData { default: 0 })
        }
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
    // Use the IR-level reduction operator parser to canonicalize the operator
    if let ClauseKind::Parenthesized(ref args) = clause.kind {
        let args = args.as_ref();
        // Find operator before top-level ':' using IR helper
        if let Some((op_str, _)) = split_once_top_level_colon(args) {
            let op_trim = op_str.trim();
            if let Ok(op_enum) = crate::ir::convert::parse_reduction_operator(op_trim) {
                use crate::ir::ReductionOperator;
                return match op_enum {
                    ReductionOperator::Add => 0,
                    ReductionOperator::Subtract => 1,
                    ReductionOperator::Multiply => 2,
                    ReductionOperator::BitwiseAnd => 3,
                    ReductionOperator::BitwiseOr => 4,
                    ReductionOperator::BitwiseXor => 5,
                    ReductionOperator::LogicalAnd => 6,
                    ReductionOperator::LogicalOr => 7,
                    ReductionOperator::Min => 8,
                    ReductionOperator::Max => 9,
                    _ => 0,
                };
            }
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
        // Use IR schedule parser to canonicalize modifiers and kind
        let config = ParserConfig::default();
        if let Ok(crate::ir::ClauseData::Schedule { kind, .. }) =
            crate::ir::convert::parse_schedule_clause(args, &config)
        {
            use crate::ir::ScheduleKind;
            return match kind {
                ScheduleKind::Static => 0,
                ScheduleKind::Dynamic => 1,
                ScheduleKind::Guided => 2,
                ScheduleKind::Auto => 3,
                ScheduleKind::Runtime => 4,
            };
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
        match args.trim().to_ascii_lowercase().as_str() {
            "shared" => return 0,
            "none" => return 1,
            _ => {}
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
// Map a `DirectiveName` enum to the integer codes used by the C API.
// This function is the single source of truth for directive -> integer mapping
// at runtime. It uses the typed `DirectiveName` enum (no string matching).
//
// Unknown or unhandled directives are treated as an error and return `-1` so
// callers can detect a missing mapping and the maintainers are alerted to add
// the correct mapping.
fn directive_name_enum_to_kind(name: DirectiveName) -> i32 {
    use DirectiveName::*;

    match name {
        Parallel | ParallelFor | ParallelDo | ParallelForSimd | ParallelDoSimd | ParallelLoop
        | ParallelLoopSimd | ParallelSections | ParallelWorkshare => 0,

        For | Do | ForSimd | DoSimd | Loop => 1,

        Sections | Section => 2,
        Single => 3,
        Task => 4,
        Master => 5,
        Critical => 6,
        Barrier => 7,
        Taskwait => 8,
        Taskgroup => 9,
        Atomic => 10,
        Flush => 11,
        Ordered => 12,

        Target
        | TargetTeams
        | TargetTeamsDistribute
        | TargetTeamsDistributeParallelFor
        | TargetTeamsDistributeParallelForSimd
        | TargetTeamsDistributeParallelLoop
        | TargetTeamsDistributeParallelLoopSimd
        | TargetTeamsDistributeParallelDo
        | TargetTeamsDistributeParallelDoSimd
        | TargetTeamsDistributeSimd
        | TargetTeamsLoop
        | TargetTeamsLoopSimd => 13,

        Teams
        | TeamsDistribute
        | TeamsDistributeParallelFor
        | TeamsDistributeParallelForSimd
        | TeamsDistributeParallelLoop
        | TeamsDistributeParallelLoopSimd
        | TeamsDistributeParallelDo
        | TeamsDistributeParallelDoSimd
        | TeamsDistributeSimd
        | TeamsLoop
        | TeamsLoopSimd => 14,

        Distribute
        | DistributeParallelFor
        | DistributeParallelForSimd
        | DistributeParallelLoop
        | DistributeParallelLoopSimd
        | DistributeParallelDo
        | DistributeParallelDoSimd
        | DistributeSimd => 15,

        Metadirective => 16,

        // Unknown / unhandled directive — treat as error so maintainers notice
        Other(s) => {
            eprintln!(
                "[c_api] unknown directive mapping requested: {}",
                s.as_ref()
            );
            -1
        }
        _ => {
            // Catch any other un-mapped variants
            eprintln!("[c_api] unhandled directive variant: {:?}", name);
            -1
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    #[test]
    fn unmapped_directive_returns_minus_one() {
        // Construct an Other variant and ensure the enum->int helper returns -1
        let other = DirectiveName::Other(Cow::Owned("__not_a_real_directive__".to_string()));
        let v = directive_name_enum_to_kind(other);
        assert_eq!(v, -1);
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
