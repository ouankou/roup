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

use crate::ir::{
    convert_directive, ClauseData as IrClauseData, DefaultKind, Language as IrLanguage,
    ParserConfig, ProcBind, ReductionOperator, ScheduleKind, SourceLocation,
};
use crate::lexer::Language;
use crate::parser::{openmp, parse_omp_directive};

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
// Clause Kind Constants - Source of Truth for Auto-Generation
// ============================================================================
// These constants are auto-generated to roup_constants.h as ROUP_CLAUSE_KIND_*
// NEVER use hard-coded numbers - always use these constants!

// ============================================================================
// Auto-Generated Constants Modules
// ============================================================================
// ALL constants are auto-generated to roup_constants.h during build
// Source of truth for each:
// - clause_kind: Defined in IR ClauseData enum variants
// - reduction_op: Defined in IR ReductionOperator enum
// - schedule_kind: Defined in IR ScheduleKind enum
// - default_kind: Defined in IR DefaultKind enum
//
// NO hard-coded numbers in source code!
// To add new constants: Update the IR enum, then rebuild

// Import auto-generated constants
include!(concat!(env!("OUT_DIR"), "/constants_modules.rs"));

// ============================================================================
// Constants Documentation
// ============================================================================
//
// SINGLE SOURCE OF TRUTH: This file defines all directive and clause kind codes.
//
// The constants are defined in:
// - clause_kind module above (clause codes, auto-generated to roup_constants.h)
// - IR DirectiveKind enum (directive codes, auto-generated from src/ir/directive.rs)
//
// For C/C++ usage:
// - build.rs auto-generates src/roup_constants.h with #define macros
// - The header provides compile-time constants for switch/case statements
// - Never modify roup_constants.h directly - edit this file instead
//
// Maintenance: When adding new clauses:
// 1. Update clause_kind module in this file
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
///
/// CRITICAL: Owns the DirectiveIR to keep clause pointers valid!
/// OmpClause structs contain pointers to IR data (Expression, ClauseItem).
/// We MUST keep the DirectiveIR alive for as long as OmpDirective exists.
#[repr(C)]
pub struct OmpDirective {
    kind: i32,                              // DirectiveKind enum value (e.g., Parallel=0, For=10, etc.)
    name: *const c_char,                    // Directive name for debugging (e.g., "parallel")
    parameter: *const c_char,               // Optional parameter (e.g., "test1" from "critical(test1)")
    clauses: Vec<OmpClause>,                // Associated clauses (contain IR pointers!)
    ir_directive: Box<crate::ir::DirectiveIR>,  // OWN the IR to keep pointers valid!
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
/// ARCHITECTURE: NO STRING OPERATIONS!
/// ===================================
/// This union stores POINTERS to IR objects, NOT converted strings.
/// The compat layer calls IR helper functions when it needs actual strings.
/// ALL string conversion happens in IR layer only.
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
    linear: ManuallyDrop<LinearData>,
    aligned: ManuallyDrop<AlignedData>,
    default: i32,
    items: *const Vec<crate::ir::ClauseItem>,    // Pointer to IR clause items (NO string conversion!)
    expression: *const crate::ir::Expression,     // Pointer to IR expression (NO string conversion!)
    identifier: *const crate::ir::Identifier,     // Pointer to IR identifier (NO string conversion!)
}

/// Schedule clause data (static, dynamic, guided, etc.)
#[repr(C)]
struct ScheduleData {
    kind: i32, // 0=static, 1=dynamic, 2=guided, 3=auto, 4=runtime
    chunk_size: *const crate::ir::Expression,  // Optional chunk size expression
}

/// Reduction clause data (operator and variables)
#[repr(C)]
struct ReductionData {
    operator: i32,                             // Reduction operator enum value
    items: *const Vec<crate::ir::ClauseItem>,  // Pointer to IR clause items (NO string conversion!)
}

/// Linear clause data (variables, step, modifier)
#[repr(C)]
struct LinearData {
    items: *const Vec<crate::ir::ClauseItem>,  // Pointer to variable list
    step: *const crate::ir::Expression,        // Pointer to step expression (nullable)
    modifier: i32,                             // Linear modifier: 0=val, 1=ref, 2=uval, -1=none
}

/// Aligned clause data (variables, alignment)
#[repr(C)]
struct AlignedData {
    items: *const Vec<crate::ir::ClauseItem>,  // Pointer to variable list
    alignment: *const crate::ir::Expression,   // Pointer to alignment expression (nullable)
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

    // Convert to IR to get DirectiveKind enum (avoids string operations in C API)
    let ir_directive = match convert_directive(
        &directive,
        SourceLocation::default(),
        IrLanguage::C,
        &ParserConfig::default(),
    ) {
        Ok(ir) => ir,
        Err(_) => return ptr::null_mut(), // Conversion error
    };

    // Extract DirectiveKind as integer (no string operations!)
    let kind = ir_directive.kind() as i32;

    // Convert IR clauses (ClauseData) to C-compatible format
    // Use IR layer's semantic clauses, NOT raw parser clauses!
    let clauses: Vec<OmpClause> = ir_directive
        .clauses()
        .iter()
        .map(|c| convert_clause_from_ir(c))
        .collect();

    // Convert to C-compatible format
    // CRITICAL: Store ir_directive so clause pointers remain valid!
    let c_directive = OmpDirective {
        kind,
        name: allocate_c_string(directive.name.as_ref()),
        parameter: ir_directive.parameter()
            .map(|p| allocate_c_string(p))
            .unwrap_or(ptr::null()),
        clauses,
        ir_directive: Box::new(ir_directive),  // Own the IR to keep pointers valid!
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

        // Free the parameter string (was allocated with CString::into_raw)
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

    // Convert lexer::Language to ir::Language
    let ir_lang = match lang {
        Language::C => IrLanguage::C,
        Language::FortranFree | Language::FortranFixed => IrLanguage::Fortran,
    };

    // Convert to IR to get DirectiveKind enum (avoids string operations in C API)
    let ir_directive = match convert_directive(
        &directive,
        SourceLocation::default(),
        ir_lang,
        &ParserConfig::default(),
    ) {
        Ok(ir) => ir,
        Err(_) => return ptr::null_mut(), // Conversion error
    };

    // Extract DirectiveKind as integer (no string operations!)
    let kind = ir_directive.kind() as i32;

    // Convert IR clauses (ClauseData) to C-compatible format
    // Use IR layer's semantic clauses, NOT raw parser clauses!
    let clauses: Vec<OmpClause> = ir_directive
        .clauses()
        .iter()
        .map(|c| convert_clause_from_ir(c))
        .collect();

    // Convert to C-compatible format
    // CRITICAL: Store ir_directive so clause pointers remain valid!
    let c_directive = OmpDirective {
        kind,
        name: allocate_c_string(directive.name.as_ref()),
        parameter: ir_directive.parameter()
            .map(|p| allocate_c_string(p))
            .unwrap_or(ptr::null()),
        clauses,
        ir_directive: Box::new(ir_directive),  // Own the IR to keep pointers valid!
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

/// Get directive kind as integer enum value.
///
/// Returns DirectiveKind enum value (e.g., Parallel=0, For=10, etc.).
/// Returns -1 if directive is NULL.
///
/// ## Example
/// ```c
/// OmpDirective* dir = roup_parse("#pragma omp parallel");
/// int32_t kind = roup_directive_kind(dir);
/// // kind == 0 (Parallel)
/// ```
#[no_mangle]
pub extern "C" fn roup_directive_kind(directive: *const OmpDirective) -> i32 {
    if directive.is_null() {
        return -1;
    }

    // UNSAFE BLOCK 4: Dereference pointer
    // Safety: Caller guarantees valid pointer from roup_parse
    unsafe {
        let dir = &*directive;
        dir.kind  // DirectiveKind enum value stored during parsing
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

/// Get directive parameter as a C string.
///
/// Returns the parameter for directives that support it:
/// - `critical(name)` returns "name"
/// - `critical` returns NULL
///
/// Returns NULL if:
/// - directive is NULL
/// - directive has no parameter
///
/// Returned pointer is valid until directive is freed.
///
/// ## Example
/// ```c
/// OmpDirective* dir = roup_parse("#pragma omp critical(test1)");
/// const char* param = roup_directive_parameter(dir);
/// if (param) {
///     printf("Parameter: %s\n", param);  // Prints: Parameter: test1
/// }
/// roup_directive_free(dir);
/// ```
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
        if c.kind != clause_kind::SCHEDULE {
            // Not a schedule clause
            return -1;
        }
        c.data.schedule.kind
    }
}

/// Get chunk size from schedule clause as a string.
///
/// Returns NULL if clause is NULL, not a schedule clause, or has no chunk size.
/// Caller must free the returned string with roup_string_free().
#[no_mangle]
pub extern "C" fn roup_clause_schedule_chunk_size(clause: *const OmpClause) -> *const c_char {
    if clause.is_null() {
        return ptr::null();
    }

    unsafe {
        let c = &*clause;
        if c.kind != clause_kind::SCHEDULE {
            return ptr::null();
        }

        let chunk_ptr = c.data.schedule.chunk_size;
        if chunk_ptr.is_null() {
            return ptr::null();
        }

        let chunk_expr = &*chunk_ptr;
        allocate_c_string(&chunk_expr.to_string())
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
        if c.kind != clause_kind::REDUCTION {
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
        if c.kind != clause_kind::DEFAULT {
            // Not a default clause
            return -1;
        }
        c.data.default
    }
}

/// Get proc bind kind from proc_bind clause.
///
/// Returns -1 if clause is NULL or not a proc_bind clause.
#[no_mangle]
pub extern "C" fn roup_clause_proc_bind_kind(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return -1;
    }

    unsafe {
        let c = &*clause;
        if c.kind != clause_kind::PROC_BIND {
            // Not a proc_bind clause
            return -1;
        }
        c.data.default
    }
}

/// Convert default kind integer to string (calls IR to_string).
///
/// Returns NULL if kind is invalid.
/// Caller must free the returned string with roup_string_free().
#[no_mangle]
pub extern "C" fn roup_default_kind_to_string(kind: i32) -> *const c_char {
    let default_kind = match kind {
        k if k == default_kind::SHARED => DefaultKind::Shared,
        k if k == default_kind::NONE => DefaultKind::None,
        k if k == default_kind::PRIVATE => DefaultKind::Private,
        k if k == default_kind::FIRSTPRIVATE => DefaultKind::Firstprivate,
        _ => return ptr::null(),
    };

    allocate_c_string(&default_kind.to_string())
}

/// Convert proc bind kind integer to string (calls IR to_string).
///
/// Returns NULL if kind is invalid.
/// Caller must free the returned string with roup_string_free().
#[no_mangle]
pub extern "C" fn roup_proc_bind_kind_to_string(kind: i32) -> *const c_char {
    let proc_bind = match kind {
        k if k == proc_bind_kind::MASTER => ProcBind::Master,
        k if k == proc_bind_kind::CLOSE => ProcBind::Close,
        k if k == proc_bind_kind::SPREAD => ProcBind::Spread,
        k if k == proc_bind_kind::PRIMARY => ProcBind::Primary,
        _ => return ptr::null(),
    };

    allocate_c_string(&proc_bind.to_string())
}

// ============================================================================
// IR Helper Functions - String Conversion (ONLY place with string operations!)
// ============================================================================
//
// ARCHITECTURE: NO STRING OPERATIONS in C API!
// ============================================
// These helper functions are the ONLY place where string conversion happens.
// They call IR layer's Display/to_string methods when compat layer needs strings.
// The C API itself NEVER converts to strings - it only exposes IR pointers.

/// Get count of items in clause (for private, shared, firstprivate, lastprivate, reduction).
///
/// Returns 0 if clause is NULL or has no items.
#[no_mangle]
pub extern "C" fn roup_clause_items_count(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return 0;
    }

    unsafe {
        let c = &*clause;

        // Variable list clauses: private, shared, firstprivate, lastprivate,
        // copyin, copyprivate, affinity
        if c.kind == clause_kind::PRIVATE
            || c.kind == clause_kind::SHARED
            || c.kind == clause_kind::FIRSTPRIVATE
            || c.kind == clause_kind::LASTPRIVATE
            || c.kind == clause_kind::COPYIN
            || c.kind == clause_kind::COPYPRIVATE
            || c.kind == clause_kind::AFFINITY {
            let items_ptr = c.data.items;
            if items_ptr.is_null() {
                return 0;
            }
            let items = &*items_ptr;
            return items.len() as i32;
        }

        // Linear clause: access items from specialized struct
        if c.kind == clause_kind::LINEAR {
            let linear_data = &*c.data.linear;
            if linear_data.items.is_null() {
                return 0;
            }
            let items = &*linear_data.items;
            return items.len() as i32;
        }

        // Aligned clause: access items from specialized struct
        if c.kind == clause_kind::ALIGNED {
            let aligned_data = &*c.data.aligned;
            if aligned_data.items.is_null() {
                return 0;
            }
            let items = &*aligned_data.items;
            return items.len() as i32;
        }

        // Reduction clause (6) has items in reduction struct
        if c.kind == clause_kind::REDUCTION {
            let items_ptr = c.data.reduction.items;
            if items_ptr.is_null() {
                return 0;
            }
            let items = &*items_ptr;
            return items.len() as i32;
        }

        // Generic/ItemList clause: used by allocate, threadprivate directives
        if c.kind == clause_kind::GENERIC {
            let items_ptr = c.data.items;
            if items_ptr.is_null() {
                return 0;
            }
            let items = &*items_ptr;
            return items.len() as i32;
        }

        0
    }
}

/// Convert clause item at index to C string (calls IR to_string).
///
/// This function calls the IR layer's Display trait to convert ClauseItem to string.
/// The compat layer calls this ONLY when it needs actual string representation.
///
/// Returns NULL if clause is NULL, has no items, or index out of bounds.
/// Caller must free the returned string with roup_string_free().
#[no_mangle]
pub extern "C" fn roup_clause_item_to_string(clause: *const OmpClause, index: i32) -> *const c_char {
    if clause.is_null() || index < 0 {
        return ptr::null();
    }

    unsafe {
        let c = &*clause;
        let idx = index as usize;

        // Variable list clauses: private, shared, firstprivate, lastprivate,
        // copyin, copyprivate, affinity
        if c.kind == clause_kind::PRIVATE
            || c.kind == clause_kind::SHARED
            || c.kind == clause_kind::FIRSTPRIVATE
            || c.kind == clause_kind::LASTPRIVATE
            || c.kind == clause_kind::COPYIN
            || c.kind == clause_kind::COPYPRIVATE
            || c.kind == clause_kind::AFFINITY {
            let items_ptr = c.data.items;
            if items_ptr.is_null() {
                return ptr::null();
            }
            let items = &*items_ptr;
            if idx >= items.len() {
                return ptr::null();
            }
            // Call IR layer's Display trait (ONLY place with string conversion!)
            return allocate_c_string(&items[idx].to_string());
        }

        // Linear clause: access items from specialized struct
        if c.kind == clause_kind::LINEAR {
            let linear_data = &*c.data.linear;
            if linear_data.items.is_null() {
                return ptr::null();
            }
            let items = &*linear_data.items;
            if idx >= items.len() {
                return ptr::null();
            }
            return allocate_c_string(&items[idx].to_string());
        }

        // Aligned clause: access items from specialized struct
        if c.kind == clause_kind::ALIGNED {
            let aligned_data = &*c.data.aligned;
            if aligned_data.items.is_null() {
                return ptr::null();
            }
            let items = &*aligned_data.items;
            if idx >= items.len() {
                return ptr::null();
            }
            return allocate_c_string(&items[idx].to_string());
        }

        // Reduction clause (6) has items in reduction struct
        if c.kind == clause_kind::REDUCTION {
            let items_ptr = c.data.reduction.items;
            if items_ptr.is_null() {
                return ptr::null();
            }
            let items = &*items_ptr;
            if idx >= items.len() {
                return ptr::null();
            }
            // Call IR layer's Display trait (ONLY place with string conversion!)
            return allocate_c_string(&items[idx].to_string());
        }

        // Generic/ItemList clause: used by allocate, threadprivate directives
        if c.kind == clause_kind::GENERIC {
            let items_ptr = c.data.items;
            if items_ptr.is_null() {
                return ptr::null();
            }
            let items = &*items_ptr;
            if idx >= items.len() {
                return ptr::null();
            }
            return allocate_c_string(&items[idx].to_string());
        }

        ptr::null()
    }
}

/// Convert clause expression to C string (calls IR to_string).
///
/// This function calls the IR layer's Display trait to convert Expression to string.
/// The compat layer calls this ONLY when it needs actual string representation.
///
/// Returns NULL if clause is NULL or has no expression.
/// Caller must free the returned string with roup_string_free().
#[no_mangle]
pub extern "C" fn roup_clause_expression_to_string(clause: *const OmpClause) -> *const c_char {
    if clause.is_null() {
        return ptr::null();
    }

    unsafe {
        let c = &*clause;

        // Expression clauses: num_threads(0), if(1), collapse(8), ordered(9),
        // safelen(16), simdlen(17), num_teams(20), thread_limit(21),
        // grainsize(22), num_tasks(23), filter(25), priority(26), device(27),
        // hint, align
        if c.kind == clause_kind::NUM_THREADS
            || c.kind == clause_kind::IF
            || c.kind == clause_kind::COLLAPSE
            || c.kind == clause_kind::ORDERED
            || c.kind == clause_kind::SAFELEN
            || c.kind == clause_kind::SIMDLEN
            || c.kind == clause_kind::NUM_TEAMS
            || c.kind == clause_kind::THREAD_LIMIT
            || c.kind == clause_kind::GRAINSIZE
            || c.kind == clause_kind::NUM_TASKS
            || c.kind == clause_kind::FILTER
            || c.kind == clause_kind::PRIORITY
            || c.kind == clause_kind::DEVICE
            || c.kind == clause_kind::HINT
            || c.kind == clause_kind::ALIGN
        {
            let expr_ptr = c.data.expression;
            if expr_ptr.is_null() {
                return ptr::null();
            }
            let expression = &*expr_ptr;
            // Call IR layer's Display trait (ONLY place with string conversion!)
            return allocate_c_string(&expression.to_string());
        }

        // Linear clause: return step expression
        if c.kind == clause_kind::LINEAR {
            let linear_data = &*c.data.linear;
            if linear_data.step.is_null() {
                return ptr::null();
            }
            let expression = &*linear_data.step;
            return allocate_c_string(&expression.to_string());
        }

        // Aligned clause: return alignment expression
        if c.kind == clause_kind::ALIGNED {
            let aligned_data = &*c.data.aligned;
            if aligned_data.alignment.is_null() {
                return ptr::null();
            }
            let expression = &*aligned_data.alignment;
            return allocate_c_string(&expression.to_string());
        }

        ptr::null()
    }
}

/// Get identifier string from clause (allocator, etc.)
///
/// Returns a C string containing the identifier name, or NULL if the clause
/// doesn't have an identifier or on error.
///
/// ## Memory Management
/// The returned string must be freed with `roup_string_free()`.
///
/// ## Example
/// ```c
/// const char* allocator = roup_clause_identifier_to_string(clause);
/// if (allocator) {
///     printf("Allocator: %s\n", allocator);
///     roup_string_free((char*)allocator);
/// }
/// ```
#[no_mangle]
pub extern "C" fn roup_clause_identifier_to_string(clause: *const OmpClause) -> *const c_char {
    if clause.is_null() {
        return ptr::null();
    }

    unsafe {
        let c = &*clause;

        // Allocator clause uses identifier
        if c.kind == clause_kind::ALLOCATOR {
            let id_ptr = c.data.identifier;
            if id_ptr.is_null() {
                return ptr::null();
            }
            let identifier = &*id_ptr;
            // Call IR layer's name() method to get the string
            return allocate_c_string(identifier.name());
        }

        ptr::null()
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

/// Convert IR ClauseData to C-compatible OmpClause.
///
/// Uses ROUP IR's structured ClauseData - NO string parsing!
/// All semantic information (variables, expressions, operators) comes from IR layer.
///
/// ## Clause Kind Mapping:
/// - 0 = num_threads    - 6 = reduction
/// - 1 = if             - 7 = schedule
/// - 2 = private        - 8 = collapse
/// - 3 = shared         - 9 = ordered
/// - 4 = firstprivate   - 10 = nowait
/// - 5 = lastprivate    - 11 = default
/// - 999 = unknown
/// Convert IR ClauseData to C API OmpClause
///
/// ARCHITECTURE: NO STRING OPERATIONS!
/// ===================================
/// This function stores POINTERS to IR objects, NOT converted strings.
/// NO .to_string(), NO allocate_c_string(), NO string comparisons.
/// The compat layer calls IR helper functions when it needs strings.
fn convert_clause_from_ir(clause: &IrClauseData) -> OmpClause {
    use IrClauseData::*;

    match clause {
        // NumThreads: expression clause - store pointer to IR Expression
        NumThreads { num } => OmpClause {
            kind: clause_kind::NUM_THREADS,
            data: ClauseData {
                expression: num as *const crate::ir::Expression,
            },
        },

        // If: expression clause - store pointer to IR Expression
        If { condition, .. } => OmpClause {
            kind: clause_kind::IF,
            data: ClauseData {
                expression: condition as *const crate::ir::Expression,
            },
        },

        // Private: variable list - store pointer to IR ClauseItem Vec
        Private { items } => OmpClause {
            kind: clause_kind::PRIVATE,
            data: ClauseData {
                items: items as *const Vec<crate::ir::ClauseItem>,
            },
        },

        // Shared: variable list - store pointer to IR ClauseItem Vec
        Shared { items } => OmpClause {
            kind: clause_kind::SHARED,
            data: ClauseData {
                items: items as *const Vec<crate::ir::ClauseItem>,
            },
        },

        // Firstprivate: variable list - store pointer to IR ClauseItem Vec
        Firstprivate { items } => OmpClause {
            kind: clause_kind::FIRSTPRIVATE,
            data: ClauseData {
                items: items as *const Vec<crate::ir::ClauseItem>,
            },
        },

        // Lastprivate: variable list - store pointer to IR ClauseItem Vec
        Lastprivate { items, .. } => OmpClause {
            kind: clause_kind::LASTPRIVATE,
            data: ClauseData {
                items: items as *const Vec<crate::ir::ClauseItem>,
            },
        },

        // Reduction: operator + variable list - store enum code + pointer to IR ClauseItem Vec
        Reduction { operator, items } => {
            let op_code = map_reduction_operator(operator);
            OmpClause {
                kind: clause_kind::REDUCTION,
                data: ClauseData {
                    reduction: ManuallyDrop::new(ReductionData {
                        operator: op_code,
                        items: items as *const Vec<crate::ir::ClauseItem>,
                    }),
                },
            }
        }

        // Schedule: kind + optional chunk size
        Schedule {
            kind: sched_kind,
            chunk_size,
            ..
        } => {
            let kind_code = map_schedule_kind(sched_kind);
            let chunk_ptr = chunk_size.as_ref().map_or(ptr::null(), |e| e as *const _);
            OmpClause {
                kind: clause_kind::SCHEDULE,
                data: ClauseData {
                    schedule: ManuallyDrop::new(ScheduleData {
                        kind: kind_code,
                        chunk_size: chunk_ptr,
                    }),
                },
            }
        }

        // Collapse: expression clause - store pointer to IR Expression
        Collapse { n } => OmpClause {
            kind: clause_kind::COLLAPSE,
            data: ClauseData {
                expression: n as *const crate::ir::Expression,
            },
        },

        // Ordered: bare or expression clause - store pointer to IR Expression or default
        Ordered { n } => {
            if let Some(expr) = n {
                OmpClause {
                    kind: clause_kind::ORDERED,
                    data: ClauseData {
                        expression: expr as *const crate::ir::Expression,
                    },
                }
            } else {
                OmpClause {
                    kind: clause_kind::ORDERED,
                    data: ClauseData { default: 0 },
                }
            }
        }

        // Nowait: bare clause - use specific enum variant
        Nowait => OmpClause {
            kind: clause_kind::NOWAIT,
            data: ClauseData { default: 0 },
        },

        // Memory order clauses - use specific enum variants
        SeqCst => OmpClause {
            kind: clause_kind::SEQ_CST,
            data: ClauseData { default: 0 },
        },

        AcqRel => OmpClause {
            kind: clause_kind::ACQ_REL,
            data: ClauseData { default: 0 },
        },

        Release => OmpClause {
            kind: clause_kind::RELEASE,
            data: ClauseData { default: 0 },
        },

        Acquire => OmpClause {
            kind: clause_kind::ACQUIRE,
            data: ClauseData { default: 0 },
        },

        Relaxed => OmpClause {
            kind: clause_kind::RELAXED,
            data: ClauseData { default: 0 },
        },

        // Atomic operation type clauses - use specific enum variants
        Read => OmpClause {
            kind: clause_kind::READ,
            data: ClauseData { default: 0 },
        },

        Write => OmpClause {
            kind: clause_kind::WRITE,
            data: ClauseData { default: 0 },
        },

        Update => OmpClause {
            kind: clause_kind::UPDATE,
            data: ClauseData { default: 0 },
        },

        Capture => OmpClause {
            kind: clause_kind::CAPTURE,
            data: ClauseData { default: 0 },
        },

        Compare => OmpClause {
            kind: clause_kind::COMPARE,
            data: ClauseData { default: 0 },
        },

        // Default: default kind - store enum code
        Default(default_kind) => {
            let kind_code = map_default_kind(default_kind);
            OmpClause {
                kind: clause_kind::DEFAULT,
                data: ClauseData {
                    default: kind_code,
                },
            }
        }

        // Copyin: variable list - store pointer to IR ClauseItem Vec
        Copyin { items } => OmpClause {
            kind: clause_kind::COPYIN,
            data: ClauseData {
                items: items as *const Vec<crate::ir::ClauseItem>,
            },
        },

        // ProcBind: proc bind kind - store enum code
        ProcBind(proc_bind_kind) => {
            let kind_code = map_proc_bind_kind(proc_bind_kind);
            OmpClause {
                kind: clause_kind::PROC_BIND,
                data: ClauseData {
                    default: kind_code,
                },
            }
        }

        // Linear: variable list + optional step + optional modifier
        Linear { items, step, modifier } => {
            let modifier_code = match modifier {
                Some(m) => *m as i32,
                None => -1,
            };
            let step_ptr = match step {
                Some(s) => s as *const crate::ir::Expression,
                None => std::ptr::null(),
            };
            OmpClause {
                kind: clause_kind::LINEAR,
                data: ClauseData {
                    linear: ManuallyDrop::new(LinearData {
                        items: items as *const Vec<crate::ir::ClauseItem>,
                        step: step_ptr,
                        modifier: modifier_code,
                    }),
                },
            }
        }

        // Aligned: variable list + optional alignment
        Aligned { items, alignment } => {
            let alignment_ptr = match alignment {
                Some(a) => a as *const crate::ir::Expression,
                None => std::ptr::null(),
            };
            OmpClause {
                kind: clause_kind::ALIGNED,
                data: ClauseData {
                    aligned: ManuallyDrop::new(AlignedData {
                        items: items as *const Vec<crate::ir::ClauseItem>,
                        alignment: alignment_ptr,
                    }),
                },
            }
        }

        // ItemList: generic variable list used by directives like allocate, threadprivate
        // These get added to the directive by the convert_directive function
        ItemList(items) => OmpClause {
            kind: clause_kind::GENERIC,  // ItemList is not a real clause kind, just a container
            data: ClauseData {
                items: items as *const Vec<crate::ir::ClauseItem>,
            },
        },

        // Hint: expression clause - store pointer to IR Expression
        Hint { hint_expr } => OmpClause {
            kind: clause_kind::HINT,
            data: ClauseData {
                expression: hint_expr as *const crate::ir::Expression,
            },
        },

        // Align: expression clause - store pointer to IR Expression
        Align { alignment } => OmpClause {
            kind: clause_kind::ALIGN,
            data: ClauseData {
                expression: alignment as *const crate::ir::Expression,
            },
        },

        // Allocator: identifier clause - store pointer to identifier
        Allocator { allocator } => OmpClause {
            kind: clause_kind::ALLOCATOR,
            data: ClauseData {
                identifier: allocator as *const crate::ir::Identifier,
            },
        },

        // Generic bare clauses (nogroup, untied, mergeable, etc.) - use GENERIC kind
        // NO STRING COMPARISON - handled as generic bare clause
        Bare(_identifier) => {
            OmpClause {
                kind: clause_kind::GENERIC,
                data: ClauseData { default: 0 },
            }
        },

        // Unknown/unsupported clauses
        _ => OmpClause {
            kind: clause_kind::UNKNOWN,
            data: ClauseData { default: 0 },
        },
    }
}

/// Map IR ReductionOperator to integer code (NO hard-coded numbers!)
fn map_reduction_operator(op: &ReductionOperator) -> i32 {
    match op {
        ReductionOperator::Add => reduction_op::ADD,
        ReductionOperator::Subtract => reduction_op::SUBTRACT,
        ReductionOperator::Multiply => reduction_op::MULTIPLY,
        ReductionOperator::BitwiseAnd => reduction_op::BITWISE_AND,
        ReductionOperator::BitwiseOr => reduction_op::BITWISE_OR,
        ReductionOperator::BitwiseXor => reduction_op::BITWISE_XOR,
        ReductionOperator::LogicalAnd => reduction_op::LOGICAL_AND,
        ReductionOperator::LogicalOr => reduction_op::LOGICAL_OR,
        ReductionOperator::Min => reduction_op::MIN,
        ReductionOperator::Max => reduction_op::MAX,
        _ => reduction_op::UNKNOWN,
    }
}

/// Map IR ScheduleKind to integer code (NO hard-coded numbers!)
fn map_schedule_kind(kind: &ScheduleKind) -> i32 {
    match kind {
        ScheduleKind::Static => schedule_kind::STATIC,
        ScheduleKind::Dynamic => schedule_kind::DYNAMIC,
        ScheduleKind::Guided => schedule_kind::GUIDED,
        ScheduleKind::Auto => schedule_kind::AUTO,
        ScheduleKind::Runtime => schedule_kind::RUNTIME,
    }
}

/// Map IR DefaultKind to integer code (NO hard-coded numbers!)
fn map_default_kind(kind: &DefaultKind) -> i32 {
    match kind {
        DefaultKind::Shared => default_kind::SHARED,
        DefaultKind::None => default_kind::NONE,
        DefaultKind::Private => default_kind::PRIVATE,
        DefaultKind::Firstprivate => default_kind::FIRSTPRIVATE,
    }
}

/// Map IR ProcBind to integer code (NO hard-coded numbers!)
fn map_proc_bind_kind(kind: &ProcBind) -> i32 {
    match kind {
        ProcBind::Master => proc_bind_kind::MASTER,
        ProcBind::Close => proc_bind_kind::CLOSE,
        ProcBind::Spread => proc_bind_kind::SPREAD,
        ProcBind::Primary => proc_bind_kind::PRIMARY,
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
/// Free clause data
///
/// ARCHITECTURE: NO STRING OPERATIONS!
/// ===================================
/// Since ClauseData now stores POINTERS to IR objects (not owned strings),
/// we do NOT free these pointers here. They are owned by DirectiveIR and
/// will be freed when the OmpDirective is freed.
///
/// This function is now a no-op but kept for API compatibility.
fn free_clause_data(_clause: &OmpClause) {
    // No-op: ClauseData stores borrowed pointers to IR objects, not owned data
    // IR objects are freed when DirectiveIR is dropped
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
