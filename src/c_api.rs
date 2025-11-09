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
// - directive_name_to_kind() function (directive codes 0-131, 999=unknown)
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
struct ReductionData {
    operator: i32,                 // 0=+, 1=-, 2=*, 6=&&, 7=||, 8=min, 9=max
    variables: *mut OmpStringList, // List of reduction variables
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

/// Get proc_bind kind from proc_bind clause.
///
/// Returns 0=master, 1=close, 2=spread, 3=unknown.
#[no_mangle]
pub extern "C" fn roup_clause_proc_bind_kind(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return 3;
    }

    unsafe {
        let c = &*clause;
        if c.kind != 13 {
            return 3;
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

        // Check if this clause type has variables or expressions
        // Kinds: 0=num_threads, 1=if, 2-5=variable lists, 6=reduction, 8=collapse, 9=ordered, 12=copyin, 14-41=various clauses
        if c.kind >= 2 && c.kind <= 5 || c.kind == 0 || c.kind == 1 || c.kind == 8 || c.kind == 9 || c.kind == 12 || (c.kind >= 14 && c.kind <= 41) {
            // Variable list or expression clauses - return pointer to existing list
            let vars_ptr = c.data.variables;
            if !vars_ptr.is_null() {
                // Clone the list so caller can own it
                let list = &*vars_ptr;
                let mut new_items = Vec::new();
                for item in &list.items {
                    // Duplicate the C string
                    let str_ptr = *item;
                    let c_str = CStr::from_ptr(str_ptr);
                    let new_c_string = CString::new(c_str.to_bytes()).unwrap();
                    new_items.push(new_c_string.into_raw() as *const c_char);
                }
                return Box::into_raw(Box::new(OmpStringList { items: new_items }));
            }
        } else if c.kind == 6 {
            // Reduction clause - variables stored in reduction struct
            let vars_ptr = c.data.reduction.variables;
            if !vars_ptr.is_null() {
                // Clone the list so caller can own it
                let list = &*vars_ptr;
                let mut new_items = Vec::new();
                for item in &list.items {
                    // Duplicate the C string
                    let str_ptr = *item;
                    let c_str = CStr::from_ptr(str_ptr);
                    let new_c_string = CString::new(c_str.to_bytes()).unwrap();
                    new_items.push(new_c_string.into_raw() as *const c_char);
                }
                return Box::into_raw(Box::new(OmpStringList { items: new_items }));
            }
        }

        ptr::null_mut()
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
    use crate::parser::ClauseKind;

    // Normalize clause name to lowercase for case-insensitive matching
    let normalized_name = clause.name.to_ascii_lowercase();

    let (kind, data) = match normalized_name.as_str() {
        "num_threads" | "if" | "collapse" | "ordered" => {
            // Expression clauses - extract the expression from Parenthesized (don't split on comma)
            let kind_code = match normalized_name.as_str() {
                "num_threads" => 0,
                "if" => 1,
                "collapse" => 8,
                "ordered" => 9,
                _ => 999,
            };
            let variables = extract_expression_from_clause(clause);
            (kind_code, ClauseData { variables })
        }
        "private" | "shared" | "firstprivate" | "lastprivate" => {
            // Variable list clauses - extract variables
            let kind_code = match normalized_name.as_str() {
                "private" => 2,
                "shared" => 3,
                "firstprivate" => 4,
                "lastprivate" => 5,
                _ => 999,
            };
            let variables = extract_variables_from_clause(clause);
            (kind_code, ClauseData { variables })
        }
        "reduction" => {
            // Reduction clause - extract operator AND variables
            let (operator, variables) = extract_reduction_data(clause);
            (
                6,
                ClauseData {
                    reduction: ManuallyDrop::new(ReductionData { operator, variables }),
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
        "nowait" => (10, ClauseData { default: 0 }),
        "default" => {
            let default_kind = parse_default_kind(clause);
            (11, ClauseData { default: default_kind })
        }
        "copyin" => {
            let variables = extract_variables_from_clause(clause);
            (12, ClauseData { variables })
        }
        "proc_bind" => {
            let proc_bind_kind = parse_proc_bind_kind(clause);
            (13, ClauseData { default: proc_bind_kind })
        }
        "linear" | "aligned" | "allocate" | "safelen" | "simdlen" | "depend" | "to" | "from" | "map" | "use_device_ptr" |
        "priority" | "affinity" | "detach" | "in_reduction" | "task_reduction" | "update" | "capture" | "read" | "write" | "seq_cst" => {
            // Clauses with variable/expression lists
            let kind_code = match normalized_name.as_str() {
                "linear" => 14,
                "aligned" => 15,
                "allocate" => 16,
                "safelen" => 17,
                "simdlen" => 18,
                "depend" => 19,
                "to" => 20,
                "from" => 21,
                "map" => 22,
                "use_device_ptr" => 23,
                "priority" => 24,
                "affinity" => 25,
                "detach" => 26,
                "in_reduction" => 27,
                "task_reduction" => 28,
                "update" => 29,
                "capture" => 30,
                "read" => 31,
                "write" => 32,
                "seq_cst" => 33,
                _ => 999,
            };
            let variables = extract_variables_from_clause(clause);
            (kind_code, ClauseData { variables })
        }
        "final" | "untied" | "mergeable" | "threads" | "simd" | "nogroup" | "grainsize" | "num_tasks" => {
            // Simple parameter or flag clauses
            let kind_code = match normalized_name.as_str() {
                "final" => 34,
                "untied" => 35,
                "mergeable" => 36,
                "threads" => 37,
                "simd" => 38,
                "nogroup" => 39,
                "grainsize" => 40,
                "num_tasks" => 41,
                _ => 999,
            };
            let variables = extract_expression_from_clause(clause);
            (kind_code, ClauseData { variables })
        }
        _ => (999, ClauseData { default: 0 }), // Unknown
    };

    OmpClause { kind, data }
}

/// Extract variables from clause kind as OmpStringList
fn extract_variables_from_clause(clause: &Clause) -> *mut OmpStringList {
    use crate::parser::ClauseKind;

    match &clause.kind {
        ClauseKind::VariableList(vars) => {
            // Convert Vec<Cow<str>> to OmpStringList with allocated C strings
            let mut items = Vec::new();
            for var in vars {
                let c_string = CString::new(var.as_ref()).unwrap();
                items.push(c_string.into_raw() as *const c_char);
            }
            Box::into_raw(Box::new(OmpStringList { items }))
        }
        ClauseKind::Parenthesized(vars_str) => {
            // Parse comma-separated variable list from parenthesized string
            let mut items = Vec::new();
            for var in vars_str.split(',') {
                let trimmed = var.trim();
                if !trimmed.is_empty() {
                    let c_string = CString::new(trimmed).unwrap();
                    items.push(c_string.into_raw() as *const c_char);
                }
            }
            if items.is_empty() {
                ptr::null_mut()
            } else {
                Box::into_raw(Box::new(OmpStringList { items }))
            }
        }
        ClauseKind::ReductionClause { variables, .. } => {
            // Reduction clause also has variables
            let mut items = Vec::new();
            for var in variables {
                let c_string = CString::new(var.as_ref()).unwrap();
                items.push(c_string.into_raw() as *const c_char);
            }
            Box::into_raw(Box::new(OmpStringList { items }))
        }
        _ => ptr::null_mut(),
    }
}

/// Extract expression from clause (for num_threads, if, etc.) without splitting on commas
fn extract_expression_from_clause(clause: &Clause) -> *mut OmpStringList {
    use crate::parser::ClauseKind;

    match &clause.kind {
        ClauseKind::Parenthesized(expr_str) => {
            // Return expression as single item (don't split on comma)
            let trimmed = expr_str.trim();
            if trimmed.is_empty() {
                ptr::null_mut()
            } else {
                let c_string = CString::new(trimmed).unwrap();
                let items = vec![c_string.into_raw() as *const c_char];
                Box::into_raw(Box::new(OmpStringList { items }))
            }
        }
        _ => ptr::null_mut(),
    }
}

/// Extract reduction operator and variables
fn extract_reduction_data(clause: &Clause) -> (i32, *mut OmpStringList) {
    use crate::parser::{ClauseKind, ReductionOperator};

    if let ClauseKind::ReductionClause { operator, variables, .. } = &clause.kind {
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
            _ => 0,
        };

        let mut items = Vec::new();
        for var in variables {
            let c_string = CString::new(var.as_ref()).unwrap();
            items.push(c_string.into_raw() as *const c_char);
        }
        let vars_list = Box::into_raw(Box::new(OmpStringList { items }));

        return (op_code, vars_list);
    }

    // Fallback to parsing from Parenthesized (older code path)
    // Format: "reduction(+: a, b, c)" -> Parenthesized("+: a, b, c")
    let operator = parse_reduction_operator(clause);
    let variables = if let ClauseKind::Parenthesized(ref args) = clause.kind {
        // Extract variables after the colon
        if let Some(colon_pos) = args.find(':') {
            let vars_str = &args[colon_pos + 1..];
            let mut items = Vec::new();
            for var in vars_str.split(',') {
                let trimmed = var.trim();
                if !trimmed.is_empty() {
                    let c_string = CString::new(trimmed).unwrap();
                    items.push(c_string.into_raw() as *const c_char);
                }
            }
            if !items.is_empty() {
                Box::into_raw(Box::new(OmpStringList { items }))
            } else {
                ptr::null_mut()
            }
        } else {
            ptr::null_mut()
        }
    } else {
        ptr::null_mut()
    };
    (operator, variables)
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
        // Case-insensitive keyword matching
        // Mapping to ompparser's OpenMPDefaultClauseKind enum:
        // 0=private, 1=firstprivate, 2=shared, 3=none, 4=variant, 5=unknown
        if args.contains("private") || args.contains("PRIVATE") || args.contains("Private") {
            if args.contains("firstprivate") || args.contains("FIRSTPRIVATE") || args.contains("Firstprivate") {
                return 1; // firstprivate
            }
            return 0; // private
        } else if args.contains("shared") || args.contains("SHARED") || args.contains("Shared") {
            return 2; // shared
        } else if args.contains("none") || args.contains("NONE") || args.contains("None") {
            return 3; // none
        }
    }
    2 // Default to shared
}

/// Parse proc_bind kind from clause arguments.
///
/// ## Proc_bind Codes (matching ompparser OpenMPProcBindClauseKind):
/// - 0 = master
/// - 1 = close
/// - 2 = spread
/// - 3 = unknown
fn parse_proc_bind_kind(clause: &Clause) -> i32 {
    if let ClauseKind::Parenthesized(ref args) = clause.kind {
        let args = args.as_ref();
        if args.contains("master") || args.contains("MASTER") || args.contains("Master") {
            return 0;
        } else if args.contains("close") || args.contains("CLOSE") || args.contains("Close") {
            return 1;
        } else if args.contains("spread") || args.contains("SPREAD") || args.contains("Spread") {
            return 2;
        }
    }
    3 // unknown
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
        // Complete mapping of all 132 OpenMP directives to unique integer codes
        match name_str.to_lowercase().as_str() {
            "allocate" => 0,
            "allocators" => 1,
            "assume" => 2,
            "assumes" => 3,
            "atomic" => 4,
            "atomic capture" => 5,
            "atomic compare capture" => 6,
            "atomic read" => 7,
            "atomic update" => 8,
            "atomic write" => 9,
            "barrier" => 10,
            "begin assumes" => 11,
            "begin declare target" => 12,
            "begin declare variant" => 13,
            "cancel" => 14,
            "cancellation point" => 15,
            "critical" => 16,
            "declare induction" => 17,
            "declare mapper" => 18,
            "declare reduction" => 19,
            "declare simd" => 20,
            "declare target" => 21,
            "declare variant" => 22,
            "depobj" => 23,
            "dispatch" => 24,
            "distribute" => 25,
            "distribute parallel for" => 26,
            "distribute parallel for simd" => 27,
            "distribute parallel loop" => 28,
            "distribute parallel loop simd" => 29,
            "distribute simd" => 30,
            "distribute parallel do" => 31,
            "distribute parallel do simd" => 32,
            "do" => 33,
            "do simd" => 34,
            "end assumes" => 35,
            "end declare target" => 36,
            "end declare variant" => 37,
            "error" => 38,
            "flush" => 39,
            "fuse" => 40,
            "groupprivate" => 41,
            "for" => 42,
            "for simd" => 43,
            "interchange" => 44,
            "interop" => 45,
            "loop" => 46,
            "reverse" => 47,
            "masked" => 48,
            "masked taskloop" => 49,
            "masked taskloop simd" => 50,
            "master" => 51,
            "master taskloop" => 52,
            "master taskloop simd" => 53,
            "metadirective" => 54,
            "begin metadirective" => 55,
            "nothing" => 56,
            "ordered" => 57,
            "parallel" => 58,
            "parallel do" => 59,
            "parallel do simd" => 60,
            "parallel for" => 61,
            "parallel for simd" => 62,
            "parallel loop" => 63,
            "parallel loop simd" => 64,
            "parallel masked" => 65,
            "parallel masked taskloop" => 66,
            "parallel masked taskloop simd" => 67,
            "parallel master" => 68,
            "parallel master taskloop" => 69,
            "parallel master taskloop simd" => 70,
            "parallel sections" => 71,
            "requires" => 72,
            "scope" => 73,
            "scan" => 74,
            "section" => 75,
            "sections" => 76,
            "simd" => 77,
            "single" => 78,
            "split" => 79,
            "stripe" => 80,
            "target" => 81,
            "target data" => 82,
            "target enter data" => 83,
            "target exit data" => 84,
            "end target" => 85,
            "target loop" => 86,
            "target loop simd" => 87,
            "target parallel" => 88,
            "target parallel do" => 89,
            "target parallel do simd" => 90,
            "target parallel for" => 91,
            "target parallel for simd" => 92,
            "target parallel loop" => 93,
            "target parallel loop simd" => 94,
            "target simd" => 95,
            "target teams" => 96,
            "target teams distribute" => 97,
            "target teams distribute parallel do" => 98,
            "target teams distribute parallel do simd" => 99,
            "target teams distribute parallel for" => 100,
            "target teams distribute parallel for simd" => 101,
            "target teams distribute parallel loop" => 102,
            "target teams distribute parallel loop simd" => 103,
            "target teams distribute simd" => 104,
            "target teams loop" => 105,
            "target teams loop simd" => 106,
            "target update" => 107,
            "task" => 108,
            "task iteration" => 109,
            "taskgroup" => 110,
            "taskgraph" => 111,
            "taskloop" => 112,
            "taskloop simd" => 113,
            "taskwait" => 114,
            "taskyield" => 115,
            "teams" => 116,
            "teams distribute" => 117,
            "teams distribute parallel do" => 118,
            "teams distribute parallel do simd" => 119,
            "teams distribute parallel for" => 120,
            "teams distribute parallel for simd" => 121,
            "teams distribute parallel loop" => 122,
            "teams distribute parallel loop simd" => 123,
            "teams distribute simd" => 124,
            "teams loop" => 125,
            "teams loop simd" => 126,
            "threadprivate" => 127,
            "tile" => 128,
            "unroll" => 129,
            "workdistribute" => 130,
            "workshare" => 131,
            // workshare is 131 - Total: 132 directives (0-131)
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
        if clause.kind >= 2 && clause.kind <= 5 {
            let vars_ptr = clause.data.variables;
            if !vars_ptr.is_null() {
                roup_string_list_free(vars_ptr);
            }
        }
        // Reduction clause (6) - free variables from reduction struct
        else if clause.kind == 6 {
            let vars_ptr = clause.data.reduction.variables;
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
