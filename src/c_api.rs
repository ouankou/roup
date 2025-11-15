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
    name: *const c_char,      // Directive name (e.g., "parallel")
    parameter: *const c_char, // Directive parameter (e.g., "(a,b,c)" for allocate/threadprivate)
    clauses: Vec<OmpClause>,  // Associated clauses
}

/// Opaque clause type (C-compatible)
///
/// Represents a single clause within a directive.
/// Uses tagged union pattern for clause-specific data.
#[repr(C)]
pub struct OmpClause {
    kind: i32,                // Clause type (num_threads=0, schedule=7, etc.)
    arguments: *const c_char, // Raw clause arguments (NULL for bare clauses)
    data: ClauseData,         // Clause-specific data (union)
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

/// Reduction clause data (operator, modifiers, and variables)
#[repr(C)]
struct ReductionData {
    operator: i32,      // 0=+, 1=-, 2=*, 6=&&, 7=||, 8=min, 9=max, -1=user-defined
    modifier_mask: u32, // bitmask of ReductionModifier values
    modifiers_text: *const c_char,
    user_identifier: *const c_char,
    variables: *mut OmpStringList,
    space_after_colon: bool,
}

const REDUCTION_MODIFIER_TASK: u32 = 1 << 0;
const REDUCTION_MODIFIER_INSCAN: u32 = 1 << 1;
const REDUCTION_MODIFIER_DEFAULT: u32 = 1 << 2;
const CLAUSE_KIND_REDUCTION: i32 = 8;
const CLAUSE_KIND_IN_REDUCTION: i32 = 45;
const CLAUSE_KIND_TASK_REDUCTION: i32 = 75;

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
    let mut directive = match parse_omp_directive(rust_str) {
        Ok((_, dir)) => dir,
        Err(_) => return ptr::null_mut(), // Parse error
    };

    // ompparser compatibility fix: Handle atomic variants
    // ROUP parses "atomic read" as compound directive DirectiveName::AtomicRead
    // but ompparser expects directive "atomic" with clause "read"
    let directive_name_str = directive.name.as_ref();
    let (final_name, extra_clause) = if directive_name_str.starts_with("atomic ") {
        let suffix = &directive_name_str[7..]; // Skip "atomic "
        match suffix {
            "read" | "write" | "update" | "capture" | "compare" | "compare capture" => {
                // Add the atomic variant as a clause
                use crate::parser::{Clause, ClauseKind};
                use std::borrow::Cow;
                let clause = Clause {
                    name: Cow::Borrowed(suffix),
                    kind: ClauseKind::Bare,
                };
                ("atomic", Some(clause))
            }
            _ => (directive_name_str, None),
        }
    } else {
        (directive_name_str, None)
    };

    // Add the extra clause if needed
    if let Some(clause) = extra_clause {
        directive.clauses.insert(0, clause);
    }

    // Convert to C-compatible format
    let c_directive = OmpDirective {
        name: allocate_c_string(final_name),
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

        // Free the parameter string if present
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
    let mut directive = match parser.parse(rust_str) {
        Ok((_, dir)) => dir,
        Err(_) => return ptr::null_mut(),
    };

    // ompparser compatibility fix: Handle atomic variants
    // ROUP parses "atomic read" as compound directive DirectiveName::AtomicRead
    // but ompparser expects directive "atomic" with clause "read"
    let directive_name_str = directive.name.as_ref();
    let (final_name, extra_clause) = if directive_name_str.starts_with("atomic ") {
        let suffix = &directive_name_str[7..]; // Skip "atomic "
        match suffix {
            "read" | "write" | "update" | "capture" | "compare" | "compare capture" => {
                // Add the atomic variant as a clause
                use crate::parser::{Clause, ClauseKind};
                use std::borrow::Cow;
                let clause = Clause {
                    name: Cow::Borrowed(suffix),
                    kind: ClauseKind::Bare,
                };
                ("atomic", Some(clause))
            }
            _ => (directive_name_str, None),
        }
    } else {
        (directive_name_str, None)
    };

    // Add the extra clause if needed
    if let Some(clause) = extra_clause {
        directive.clauses.insert(0, clause);
    }

    // Convert to C-compatible format
    let c_directive = OmpDirective {
        name: allocate_c_string(final_name),
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
        let kind = directive_name_enum_to_kind(dname);
        kind
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

/// Get directive parameter as a C string (e.g., "(a,b,c)" for allocate/threadprivate).
///
/// Returns NULL if directive is NULL or has no parameter.
/// Returned pointer is valid until directive is freed.
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
        if let Some(data) = get_reduction_data(c) {
            data.operator
        } else {
            -1
        }
    }
}

/// Get reduction modifier mask (bitfield of task/inscan/default).
#[no_mangle]
pub extern "C" fn roup_clause_reduction_modifier_mask(clause: *const OmpClause) -> u32 {
    if clause.is_null() {
        return 0;
    }

    unsafe {
        let c = &*clause;
        if let Some(data) = get_reduction_data(c) {
            data.modifier_mask
        } else {
            0
        }
    }
}

/// Get user-defined identifier for reduction operator (if any).
#[no_mangle]
pub extern "C" fn roup_clause_reduction_user_identifier(clause: *const OmpClause) -> *const c_char {
    if clause.is_null() {
        return ptr::null();
    }

    unsafe {
        let c = &*clause;
        if let Some(data) = get_reduction_data(c) {
            data.user_identifier
        } else {
            ptr::null()
        }
    }
}

/// Get comma-separated modifier text for reduction clause (if any).
#[no_mangle]
pub extern "C" fn roup_clause_reduction_modifiers_text(clause: *const OmpClause) -> *const c_char {
    if clause.is_null() {
        return ptr::null();
    }

    unsafe {
        let c = &*clause;
        if let Some(data) = get_reduction_data(c) {
            data.modifiers_text
        } else {
            ptr::null()
        }
    }
}

/// Whether a space existed after the colon in the reduction clause.
#[no_mangle]
pub extern "C" fn roup_clause_reduction_space_after_colon(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return 0;
    }

    unsafe {
        let c = &*clause;
        if let Some(data) = get_reduction_data(c) {
            if data.space_after_colon {
                1
            } else {
                0
            }
        } else {
            0
        }
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

/// Get clause arguments as a string.
///
/// Returns NULL if clause is NULL or has no arguments.
/// Returned pointer is valid until clause is freed.
///
/// For bare clauses (nowait, ordered, etc.), returns NULL.
/// For parenthesized clauses, returns the content inside parentheses.
/// For example: "private(a, b)" returns "a, b"
#[no_mangle]
pub extern "C" fn roup_clause_arguments(clause: *const OmpClause) -> *const c_char {
    if clause.is_null() {
        return ptr::null();
    }

    unsafe {
        let c = &*clause;
        c.arguments
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
        // Kinds 3,4,5,13 are private/firstprivate/shared/lastprivate
        if is_reduction_clause_kind(c.kind) {
            if let Some(data) = get_reduction_data(c) {
                return clone_string_list(data.variables);
            }
        }

        if !((c.kind >= 3 && c.kind <= 5) || c.kind == 13) {
            return ptr::null_mut();
        }

        // Variable list extraction for private/firstprivate/shared/lastprivate is not yet implemented.
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

fn build_string_list(items: &[std::borrow::Cow<'_, str>]) -> *mut OmpStringList {
    if items.is_empty() {
        return ptr::null_mut();
    }

    let mut list = OmpStringList { items: Vec::new() };
    for item in items {
        let c_string = match CString::new(item.as_ref()) {
            Ok(value) => value,
            Err(_) => continue,
        };
        list.items.push(c_string.into_raw());
    }

    Box::into_raw(Box::new(list))
}

unsafe fn clone_string_list(src: *mut OmpStringList) -> *mut OmpStringList {
    if src.is_null() {
        return ptr::null_mut();
    }

    let src_ref = &*src;
    if src_ref.items.is_empty() {
        return ptr::null_mut();
    }

    let mut list = OmpStringList { items: Vec::new() };
    for &ptr in &src_ref.items {
        if ptr.is_null() {
            continue;
        }
        let c_str = std::ffi::CStr::from_ptr(ptr);
        if let Ok(cloned) = CString::new(c_str.to_bytes()) {
            list.items.push(cloned.into_raw());
        }
    }

    Box::into_raw(Box::new(list))
}

fn reduction_operator_code(op: crate::parser::clause::ReductionOperator) -> i32 {
    match op {
        crate::parser::clause::ReductionOperator::Add => 0,
        crate::parser::clause::ReductionOperator::Sub => 1,
        crate::parser::clause::ReductionOperator::Mul => 2,
        crate::parser::clause::ReductionOperator::BitAnd => 3,
        crate::parser::clause::ReductionOperator::BitOr => 4,
        crate::parser::clause::ReductionOperator::BitXor => 5,
        crate::parser::clause::ReductionOperator::LogAnd => 6,
        crate::parser::clause::ReductionOperator::LogOr => 7,
        crate::parser::clause::ReductionOperator::Min => 8,
        crate::parser::clause::ReductionOperator::Max => 9,
        crate::parser::clause::ReductionOperator::FortAnd => 6,
        crate::parser::clause::ReductionOperator::FortOr => 7,
        crate::parser::clause::ReductionOperator::FortEqv => 10,
        crate::parser::clause::ReductionOperator::FortNeqv => 11,
        crate::parser::clause::ReductionOperator::FortIand => 12,
        crate::parser::clause::ReductionOperator::FortIor => 13,
        crate::parser::clause::ReductionOperator::FortIeor => 14,
        crate::parser::clause::ReductionOperator::UserDefined => -1,
    }
}

fn reduction_modifier_mask(modifiers: &[crate::parser::clause::ReductionModifier]) -> u32 {
    use crate::parser::clause::ReductionModifier;
    let mut mask = 0;
    for modifier in modifiers {
        match modifier {
            ReductionModifier::Task => mask |= REDUCTION_MODIFIER_TASK,
            ReductionModifier::Inscan => mask |= REDUCTION_MODIFIER_INSCAN,
            ReductionModifier::Default => mask |= REDUCTION_MODIFIER_DEFAULT,
        }
    }
    mask
}

fn build_reduction_data(clause: &Clause) -> ReductionData {
    if let ClauseKind::ReductionClause {
        modifiers,
        operator,
        user_defined_identifier,
        variables,
        space_after_colon,
    } = &clause.kind
    {
        let user_identifier_ptr = user_defined_identifier
            .as_ref()
            .map(|value| CString::new(value.as_ref()).unwrap().into_raw())
            .unwrap_or(ptr::null_mut()) as *const c_char;

        let modifiers_text_ptr = if modifiers.is_empty() {
            ptr::null()
        } else {
            let joined = modifiers
                .iter()
                .map(|m| match m {
                    crate::parser::clause::ReductionModifier::Task => "task",
                    crate::parser::clause::ReductionModifier::Inscan => "inscan",
                    crate::parser::clause::ReductionModifier::Default => "default",
                })
                .collect::<Vec<_>>()
                .join(",");
            CString::new(joined).unwrap().into_raw()
        };

        let vars_ptr = build_string_list(variables);

        ReductionData {
            operator: reduction_operator_code(*operator),
            modifier_mask: reduction_modifier_mask(modifiers),
            modifiers_text: modifiers_text_ptr,
            user_identifier: user_identifier_ptr,
            variables: vars_ptr,
            space_after_colon: *space_after_colon,
        }
    } else {
        ReductionData {
            operator: -1,
            modifier_mask: 0,
            modifiers_text: ptr::null(),
            user_identifier: ptr::null(),
            variables: ptr::null_mut(),
            space_after_colon: true,
        }
    }
}

fn is_reduction_clause_kind(kind: i32) -> bool {
    matches!(
        kind,
        CLAUSE_KIND_REDUCTION | CLAUSE_KIND_IN_REDUCTION | CLAUSE_KIND_TASK_REDUCTION
    )
}

unsafe fn get_reduction_data<'a>(clause: &'a OmpClause) -> Option<&'a ReductionData> {
    if is_reduction_clause_kind(clause.kind) {
        Some(&*clause.data.reduction)
    } else {
        None
    }
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
        // Generated from compat/ompparser/ompparser/src/OpenMPKinds.h enum OpenMPClauseKind
        // The enum starts at index 0, incrementing by 1 for each OPENMP_CLAUSE entry
        // Only clauses that exist in ROUP's ClauseName enum (src/parser/clause.rs) are mapped
        crate::parser::ClauseName::If => (0, ClauseData { default: 0 }),
        crate::parser::ClauseName::NumThreads => (1, ClauseData { default: 0 }),
        crate::parser::ClauseName::Default => {
            let default_kind = parse_default_kind(clause);
            (
                2,
                ClauseData {
                    default: default_kind,
                },
            )
        }
        crate::parser::ClauseName::Private => (
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
        crate::parser::ClauseName::Shared => (
            5,
            ClauseData {
                variables: ptr::null_mut(),
            },
        ),
        // copyin is 6 in both OpenMP and OpenACC, but with different semantics
        crate::parser::ClauseName::CopyIn => (6, ClauseData { default: 0 }),
        crate::parser::ClauseName::Align => (7, ClauseData { default: 0 }),
        crate::parser::ClauseName::Reduction => (
            8,
            ClauseData {
                reduction: ManuallyDrop::new(build_reduction_data(clause)),
            },
        ),
        // proc_bind = 9 (not in ROUP ClauseName yet, mapped via Other below)
        // allocate = 10 (not in ROUP ClauseName yet, mapped via Other below)
        crate::parser::ClauseName::NumTeams => (11, ClauseData { default: 0 }),
        crate::parser::ClauseName::ThreadLimit => (12, ClauseData { default: 0 }),
        crate::parser::ClauseName::Lastprivate => (
            13,
            ClauseData {
                variables: ptr::null_mut(),
            },
        ),
        crate::parser::ClauseName::Collapse => (14, ClauseData { default: 0 }),
        crate::parser::ClauseName::Ordered => (15, ClauseData { default: 0 }),
        // partial = 16 (not in ROUP ClauseName yet, mapped via Other below)
        crate::parser::ClauseName::Nowait => (17, ClauseData { default: 0 }),
        // full = 18 (not in ROUP ClauseName yet, mapped via Other below)
        // order = 19 (not in ROUP ClauseName yet, mapped via Other below)
        // linear = 20 (not in ROUP ClauseName yet, mapped via Other below)
        crate::parser::ClauseName::Schedule => {
            let schedule_kind = parse_schedule_kind(clause);
            (
                21,
                ClauseData {
                    schedule: ManuallyDrop::new(ScheduleData {
                        kind: schedule_kind,
                    }),
                },
            )
        }
        // safelen to bind = 22-30
        crate::parser::ClauseName::DistSchedule => (29, ClauseData { default: 0 }),
        // 31-44 (not in ROUP ClauseName yet, mapped via Other below)
        crate::parser::ClauseName::InReduction => (
            45,
            ClauseData {
                reduction: ManuallyDrop::new(build_reduction_data(clause)),
            },
        ),
        crate::parser::ClauseName::Depend => (46, ClauseData { default: 0 }),
        // 47-65 (not in ROUP ClauseName yet, mapped via Other below)
        crate::parser::ClauseName::IsDevicePtr => (66, ClauseData { default: 0 }),
        // 67 (not in ROUP ClauseName yet, mapped via Other below)
        crate::parser::ClauseName::Defaultmap => (68, ClauseData { default: 0 }),
        // 69-70 (not in ROUP ClauseName yet, mapped via Other below)
        crate::parser::ClauseName::UsesAllocators => (71, ClauseData { default: 0 }),
        // Additional OpenMP clauses from spec
        crate::parser::ClauseName::ProcBind => (9, ClauseData { default: 0 }),
        crate::parser::ClauseName::Allocate => (10, ClauseData { default: 0 }),
        crate::parser::ClauseName::Partial => (16, ClauseData { default: 0 }),
        crate::parser::ClauseName::Full => (18, ClauseData { default: 0 }),
        crate::parser::ClauseName::Order => (19, ClauseData { default: 0 }),
        crate::parser::ClauseName::Linear => (20, ClauseData { default: 0 }),
        crate::parser::ClauseName::Safelen => (22, ClauseData { default: 0 }),
        crate::parser::ClauseName::Simdlen => (23, ClauseData { default: 0 }),
        crate::parser::ClauseName::Aligned => (24, ClauseData { default: 0 }),
        crate::parser::ClauseName::Nontemporal => (25, ClauseData { default: 0 }),
        crate::parser::ClauseName::Uniform => (26, ClauseData { default: 0 }),
        crate::parser::ClauseName::Inbranch => (27, ClauseData { default: 0 }),
        crate::parser::ClauseName::Notinbranch => (28, ClauseData { default: 0 }),
        crate::parser::ClauseName::Bind => (30, ClauseData { default: 0 }),
        crate::parser::ClauseName::Inclusive => (31, ClauseData { default: 0 }),
        crate::parser::ClauseName::Exclusive => (32, ClauseData { default: 0 }),
        crate::parser::ClauseName::Copyprivate => (33, ClauseData { default: 0 }),
        crate::parser::ClauseName::Parallel => (34, ClauseData { default: 0 }),
        crate::parser::ClauseName::Sections => (35, ClauseData { default: 0 }),
        crate::parser::ClauseName::For => (36, ClauseData { default: 0 }),
        crate::parser::ClauseName::Do => (37, ClauseData { default: 0 }),
        crate::parser::ClauseName::Taskgroup => (38, ClauseData { default: 0 }),
        crate::parser::ClauseName::Initializer => (40, ClauseData { default: 0 }),
        crate::parser::ClauseName::Final => (41, ClauseData { default: 0 }),
        crate::parser::ClauseName::Untied => (42, ClauseData { default: 0 }),
        crate::parser::ClauseName::Requires => (43, ClauseData { default: 0 }),
        crate::parser::ClauseName::Mergeable => (44, ClauseData { default: 0 }),
        crate::parser::ClauseName::Priority => (90, ClauseData { default: 0 }),
        crate::parser::ClauseName::Affinity => (48, ClauseData { default: 0 }),
        crate::parser::ClauseName::Detach => (49, ClauseData { default: 0 }),
        crate::parser::ClauseName::Grainsize => (50, ClauseData { default: 0 }),
        crate::parser::ClauseName::NumTasks => (51, ClauseData { default: 0 }),
        crate::parser::ClauseName::Nogroup => (52, ClauseData { default: 0 }),
        crate::parser::ClauseName::ReverseOffload => (53, ClauseData { default: 0 }),
        crate::parser::ClauseName::UnifiedAddress => (54, ClauseData { default: 0 }),
        crate::parser::ClauseName::UnifiedSharedMemory => (55, ClauseData { default: 0 }),
        crate::parser::ClauseName::AtomicDefaultMemOrder => (56, ClauseData { default: 0 }),
        crate::parser::ClauseName::DynamicAllocators => (57, ClauseData { default: 0 }),
        crate::parser::ClauseName::SelfMaps => (58, ClauseData { default: 0 }),
        crate::parser::ClauseName::ExtImplementationDefinedRequirement => {
            (59, ClauseData { default: 0 })
        }
        crate::parser::ClauseName::UseDevicePtr => (62, ClauseData { default: 0 }),
        crate::parser::ClauseName::Sizes => (63, ClauseData { default: 0 }),
        crate::parser::ClauseName::UseDeviceAddr => (64, ClauseData { default: 0 }),
        crate::parser::ClauseName::HasDeviceAddr => (91, ClauseData { default: 0 }),
        crate::parser::ClauseName::To => (92, ClauseData { default: 0 }),
        crate::parser::ClauseName::From => (69, ClauseData { default: 0 }),
        crate::parser::ClauseName::When => (70, ClauseData { default: 0 }),
        crate::parser::ClauseName::Match => (72, ClauseData { default: 0 }),
        crate::parser::ClauseName::TaskReduction => (
            75,
            ClauseData {
                reduction: ManuallyDrop::new(build_reduction_data(clause)),
            },
        ),
        crate::parser::ClauseName::Compare => (86, ClauseData { default: 0 }),
        crate::parser::ClauseName::CompareCapture => (87, ClauseData { default: 0 }),
        crate::parser::ClauseName::Destroy => (88, ClauseData { default: 0 }),
        crate::parser::ClauseName::DepobjUpdate => (89, ClauseData { default: 0 }),
        // OpenACC/OpenMP device clauses
        crate::parser::ClauseName::Device => (60, ClauseData { default: 0 }),
        crate::parser::ClauseName::Map => (61, ClauseData { default: 0 }),
        // OpenMP atomic memory order clauses (bare clauses, no parameters)
        crate::parser::ClauseName::AcqRel => (76, ClauseData { default: 0 }),
        crate::parser::ClauseName::Release => (77, ClauseData { default: 0 }),
        crate::parser::ClauseName::Acquire => (78, ClauseData { default: 0 }),
        // Note: Read, Write, Update, Capture are handled below (OpenMP codes 79-82)
        crate::parser::ClauseName::SeqCst => (83, ClauseData { default: 0 }),
        crate::parser::ClauseName::Relaxed => (84, ClauseData { default: 0 }),
        crate::parser::ClauseName::Hint => (85, ClauseData { default: 0 }),
        // OpenMP atomic operation clauses (these are used for both OpenMP and OpenACC)
        crate::parser::ClauseName::Read => (79, ClauseData { default: 0 }),
        crate::parser::ClauseName::Write => (80, ClauseData { default: 0 }),
        crate::parser::ClauseName::Update => (81, ClauseData { default: 0 }),
        crate::parser::ClauseName::Capture => (82, ClauseData { default: 0 }),
        // OpenMP allocate directive clauses
        crate::parser::ClauseName::Allocator => (39, ClauseData { default: 0 }),
        // OpenMP clauses that are also in OpenACC (but with different codes)
        crate::parser::ClauseName::Link => (73, ClauseData { default: 0 }),
        crate::parser::ClauseName::DeviceType => (74, ClauseData { default: 0 }),
        // OpenACC-only clause names map to -1 in the OpenMP C API layer
        crate::parser::ClauseName::Copy
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
        | crate::parser::ClauseName::DefaultAsync
        | crate::parser::ClauseName::NoCreate
        | crate::parser::ClauseName::NoHost
        | crate::parser::ClauseName::SelfClause
        | crate::parser::ClauseName::Tile
        | crate::parser::ClauseName::UseDevice
        | crate::parser::ClauseName::Attach
        | crate::parser::ClauseName::Finalize
        | crate::parser::ClauseName::IfPresent
        | crate::parser::ClauseName::Delete
        | crate::parser::ClauseName::DevicePtr
        | crate::parser::ClauseName::DeviceNum
        | crate::parser::ClauseName::DeviceResident
        | crate::parser::ClauseName::Host => (-1, ClauseData { default: 0 }),

        // Additional OpenMP clauses for ompparser compatibility
        // Starting from 133 to avoid conflicts with existing clauses (highest is 132)
        crate::parser::ClauseName::Threads => (133, ClauseData { default: 0 }),
        crate::parser::ClauseName::Simd => (134, ClauseData { default: 0 }),
        crate::parser::ClauseName::Filter => (135, ClauseData { default: 0 }),
        crate::parser::ClauseName::Fail => (93, ClauseData { default: 0 }), // Keep 93 (ROUP_OMPC_fail)
        crate::parser::ClauseName::Weak => (136, ClauseData { default: 0 }),
        crate::parser::ClauseName::At => (137, ClauseData { default: 0 }),
        crate::parser::ClauseName::Severity => (138, ClauseData { default: 0 }),
        crate::parser::ClauseName::Message => (139, ClauseData { default: 0 }),
        crate::parser::ClauseName::Doacross => (140, ClauseData { default: 0 }),
        crate::parser::ClauseName::Absent => (141, ClauseData { default: 0 }),
        crate::parser::ClauseName::Contains => (142, ClauseData { default: 0 }),
        crate::parser::ClauseName::Holds => (143, ClauseData { default: 0 }),
        crate::parser::ClauseName::Otherwise => (144, ClauseData { default: 0 }),
        crate::parser::ClauseName::GraphId => (145, ClauseData { default: 0 }),
        crate::parser::ClauseName::GraphReset => (146, ClauseData { default: 0 }),
        crate::parser::ClauseName::Transparent => (147, ClauseData { default: 0 }),
        crate::parser::ClauseName::Replayable => (148, ClauseData { default: 0 }),
        crate::parser::ClauseName::Threadset => (149, ClauseData { default: 0 }),
        crate::parser::ClauseName::Indirect => (108, ClauseData { default: 0 }),
        crate::parser::ClauseName::Local => (109, ClauseData { default: 0 }),
        crate::parser::ClauseName::Init => (110, ClauseData { default: 0 }),
        crate::parser::ClauseName::InitComplete => (111, ClauseData { default: 0 }),
        crate::parser::ClauseName::Safesync => (112, ClauseData { default: 0 }),
        crate::parser::ClauseName::DeviceSafesync => (113, ClauseData { default: 0 }),
        crate::parser::ClauseName::Memscope => (114, ClauseData { default: 0 }),
        crate::parser::ClauseName::Looprange => (115, ClauseData { default: 0 }),
        crate::parser::ClauseName::Permutation => (116, ClauseData { default: 0 }),
        crate::parser::ClauseName::Counts => (117, ClauseData { default: 0 }),
        crate::parser::ClauseName::Induction => (118, ClauseData { default: 0 }),
        crate::parser::ClauseName::Inductor => (119, ClauseData { default: 0 }),
        crate::parser::ClauseName::Collector => (120, ClauseData { default: 0 }),
        crate::parser::ClauseName::Combiner => (121, ClauseData { default: 0 }),
        crate::parser::ClauseName::AdjustArgs => (122, ClauseData { default: 0 }),
        crate::parser::ClauseName::AppendArgs => (123, ClauseData { default: 0 }),
        crate::parser::ClauseName::Apply => (124, ClauseData { default: 0 }),
        crate::parser::ClauseName::NoOpenmp => (125, ClauseData { default: 0 }),
        crate::parser::ClauseName::NoOpenmpConstructs => (126, ClauseData { default: 0 }),
        crate::parser::ClauseName::NoOpenmpRoutines => (127, ClauseData { default: 0 }),
        crate::parser::ClauseName::NoParallelism => (128, ClauseData { default: 0 }),
        crate::parser::ClauseName::Nocontext => (129, ClauseData { default: 0 }),
        crate::parser::ClauseName::Novariants => (130, ClauseData { default: 0 }),
        crate::parser::ClauseName::Enter => (131, ClauseData { default: 0 }),
        crate::parser::ClauseName::Use => (132, ClauseData { default: 0 }),

        crate::parser::ClauseName::Other(ref s) => {
            // Map common OpenMP clauses that aren't yet in ROUP's ClauseName enum
            let normalized = s.to_ascii_lowercase();
            match normalized.as_ref() {
                // Atomic operation variants (for ompparser compatibility)
                "read" => (79, ClauseData { default: 0 }),
                "write" => (80, ClauseData { default: 0 }),
                "update" => (81, ClauseData { default: 0 }),
                "capture" => (82, ClauseData { default: 0 }),
                "compare" => (86, ClauseData { default: 0 }),
                "compare capture" => (87, ClauseData { default: 0 }),
                "proc_bind" => (9, ClauseData { default: 0 }),
                "allocate" => (10, ClauseData { default: 0 }),
                "num_teams" => (11, ClauseData { default: 0 }),
                "thread_limit" => (12, ClauseData { default: 0 }),
                "partial" => (16, ClauseData { default: 0 }),
                "full" => (18, ClauseData { default: 0 }),
                "order" => (19, ClauseData { default: 0 }),
                "linear" => (20, ClauseData { default: 0 }),
                "safelen" => (22, ClauseData { default: 0 }),
                "simdlen" => (23, ClauseData { default: 0 }),
                "aligned" => (24, ClauseData { default: 0 }),
                "nontemporal" => (25, ClauseData { default: 0 }),
                "uniform" => (26, ClauseData { default: 0 }),
                "inbranch" => (27, ClauseData { default: 0 }),
                "notinbranch" => (28, ClauseData { default: 0 }),
                "dist_schedule" => (29, ClauseData { default: 0 }),
                "bind" => (30, ClauseData { default: 0 }),
                "inclusive" => (31, ClauseData { default: 0 }),
                "exclusive" => (32, ClauseData { default: 0 }),
                "copyprivate" => (33, ClauseData { default: 0 }),
                "parallel" => (34, ClauseData { default: 0 }),
                "sections" => (35, ClauseData { default: 0 }),
                "for" => (36, ClauseData { default: 0 }),
                "do" => (37, ClauseData { default: 0 }),
                "taskgroup" => (38, ClauseData { default: 0 }),
                "initializer" => (40, ClauseData { default: 0 }),
                "final" => (41, ClauseData { default: 0 }),
                "untied" => (42, ClauseData { default: 0 }),
                "requires" => (43, ClauseData { default: 0 }),
                "mergeable" => (44, ClauseData { default: 0 }),
                "in_reduction" => (45, ClauseData { default: 0 }),
                "depend" => (46, ClauseData { default: 0 }),
                "priority" => (47, ClauseData { default: 0 }),
                "affinity" => (48, ClauseData { default: 0 }),
                "detach" => (49, ClauseData { default: 0 }),
                "grainsize" => (50, ClauseData { default: 0 }),
                "num_tasks" => (51, ClauseData { default: 0 }),
                "nogroup" => (52, ClauseData { default: 0 }),
                "reverse_offload" => (53, ClauseData { default: 0 }),
                "unified_address" => (54, ClauseData { default: 0 }),
                "unified_shared_memory" => (55, ClauseData { default: 0 }),
                "atomic_default_mem_order" => (56, ClauseData { default: 0 }),
                "dynamic_allocators" => (57, ClauseData { default: 0 }),
                "self_maps" => (58, ClauseData { default: 0 }),
                "ext_implementation_defined_requirement" => (59, ClauseData { default: 0 }),
                "use_device_ptr" => (62, ClauseData { default: 0 }),
                "sizes" => (63, ClauseData { default: 0 }),
                "use_device_addr" => (64, ClauseData { default: 0 }),
                "is_device_ptr" => (65, ClauseData { default: 0 }),
                "has_device_addr" => (66, ClauseData { default: 0 }),
                "defaultmap" => (67, ClauseData { default: 0 }),
                "to" => (68, ClauseData { default: 0 }),
                "from" => (69, ClauseData { default: 0 }),
                "uses_allocators" => (70, ClauseData { default: 0 }),
                "when" => (71, ClauseData { default: 0 }),
                "match" => (72, ClauseData { default: 0 }),
                "link" => (73, ClauseData { default: 0 }),
                "device_type" => (74, ClauseData { default: 0 }),
                "task_reduction" => (75, ClauseData { default: 0 }),
                "destroy" => (86, ClauseData { default: 0 }),
                "depobj_update" => (87, ClauseData { default: 0 }),
                // Additional OpenMP clauses for ompparser compatibility (fallback for Other)
                "threads" => (89, ClauseData { default: 0 }),
                "simd" => (90, ClauseData { default: 0 }),
                "filter" => (91, ClauseData { default: 0 }),
                "fail" => (93, ClauseData { default: 0 }),
                "weak" => (94, ClauseData { default: 0 }),
                "at" => (95, ClauseData { default: 0 }),
                "severity" => (96, ClauseData { default: 0 }),
                "message" => (97, ClauseData { default: 0 }),
                "doacross" => (98, ClauseData { default: 0 }),
                "absent" => (99, ClauseData { default: 0 }),
                "contains" => (100, ClauseData { default: 0 }),
                "holds" => (101, ClauseData { default: 0 }),
                "otherwise" => (102, ClauseData { default: 0 }),
                "graph_id" => (103, ClauseData { default: 0 }),
                "graph_reset" => (104, ClauseData { default: 0 }),
                "transparent" => (105, ClauseData { default: 0 }),
                "replayable" => (106, ClauseData { default: 0 }),
                "threadset" => (107, ClauseData { default: 0 }),
                "indirect" => (108, ClauseData { default: 0 }),
                "local" => (109, ClauseData { default: 0 }),
                "init" => (110, ClauseData { default: 0 }),
                "init_complete" => (111, ClauseData { default: 0 }),
                "safesync" => (112, ClauseData { default: 0 }),
                "device_safesync" => (113, ClauseData { default: 0 }),
                "memscope" => (114, ClauseData { default: 0 }),
                "looprange" => (115, ClauseData { default: 0 }),
                "permutation" => (116, ClauseData { default: 0 }),
                "counts" => (117, ClauseData { default: 0 }),
                "induction" => (118, ClauseData { default: 0 }),
                "inductor" => (119, ClauseData { default: 0 }),
                "collector" => (120, ClauseData { default: 0 }),
                "combiner" => (121, ClauseData { default: 0 }),
                "adjust_args" => (122, ClauseData { default: 0 }),
                "append_args" => (123, ClauseData { default: 0 }),
                "apply" => (124, ClauseData { default: 0 }),
                "no_openmp" => (125, ClauseData { default: 0 }),
                "no_openmp_constructs" => (126, ClauseData { default: 0 }),
                "no_openmp_routines" => (127, ClauseData { default: 0 }),
                "no_parallelism" => (128, ClauseData { default: 0 }),
                "nocontext" => (129, ClauseData { default: 0 }),
                "novariants" => (130, ClauseData { default: 0 }),
                "enter" => (131, ClauseData { default: 0 }),
                "use" => (132, ClauseData { default: 0 }),
                _ => {
                    // Unknown/user-extension clauses should be mapped to ext_implementation_defined_requirement
                    // per OpenMP spec section 2.3.1
                    eprintln!("[c_api] mapping unknown clause '{}' to ext_implementation_defined_requirement", s.as_ref());
                    (59, ClauseData { default: 0 })
                }
            }
        }
    };

    // Extract clause arguments based on ClauseKind
    // For simplicity, we reconstruct the clause as a string using the Display trait
    // This avoids having to handle all the complex ClauseKind variants
    let arguments = match &clause.kind {
        ClauseKind::Bare => {
            // For extension clauses (kind == 59), pass the clause name as the argument
            // IMPORTANT: ompparser expects the name WITHOUT the "ext_" prefix
            // e.g., "ext_user_test" → "user_test" (ompparser adds "ext_" when printing)
            if kind == 59 {
                let name = clause.name.as_ref();
                if name.starts_with("ext_") {
                    allocate_c_string(&name[4..]) // Skip "ext_" prefix
                } else {
                    allocate_c_string(name)
                }
            } else {
                ptr::null()
            }
        }
        ClauseKind::Parenthesized(ref args) => {
            // For extension clauses with parentheses, prepend the clause name
            // Format: "clause_name(args)" → "clause_name" as the argument
            if kind == 59 {
                // Include both name and arguments: "name(args)"
                // Strip "ext_" prefix from the name, if present
                let name = clause.name.as_ref();
                let stripped_name = if name.starts_with("ext_") {
                    &name[4..]
                } else {
                    name
                };
                let full_clause = format!("{}({})", stripped_name, args.as_ref());
                allocate_c_string(&full_clause)
            } else {
                allocate_c_string(args.as_ref())
            }
        }
        _ => {
            // Use the Display implementation to get a string representation
            let clause_str = clause.to_source_string();
            // Extract content after clause name (inside parentheses)
            if let Some(start) = clause_str.find('(') {
                if let Some(end) = clause_str.rfind(')') {
                    let args_str = &clause_str[start + 1..end];
                    allocate_c_string(args_str)
                } else {
                    ptr::null()
                }
            } else {
                ptr::null()
            }
        }
    };

    OmpClause {
        kind,
        arguments,
        data,
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

    // Map to ompparser's OpenMPDirectiveKind enum values
    // These must match the sequential order in OpenMPKinds.h exactly
    // Generated from compat/ompparser/ompparser/src/OpenMPKinds.h
    let result = match name {
        Parallel => 0,                    // OMPD_parallel
        For => 1,                         // OMPD_for
        Do => 2,                          // OMPD_do
        Simd => 3,                        // OMPD_simd
        ForSimd => 4,                     // OMPD_for_simd
        DoSimd => 5,                      // OMPD_do_simd
        ParallelForSimd => 6,             // OMPD_parallel_for_simd
        ParallelDoSimd => 7,              // OMPD_parallel_do_simd
        DeclareSimd => 8,                 // OMPD_declare_simd
        Distribute => 9,                  // OMPD_distribute
        DistributeSimd => 10,             // OMPD_distribute_simd
        DistributeParallelFor => 11,      // OMPD_distribute_parallel_for
        DistributeParallelDo => 12,       // OMPD_distribute_parallel_do
        DistributeParallelForSimd => 13,  // OMPD_distribute_parallel_for_simd
        DistributeParallelDoSimd => 14,   // OMPD_distribute_parallel_do_simd
        Loop => 15,                       // OMPD_loop
        Scan => 16,                       // OMPD_scan
        Sections => 17,                   // OMPD_sections
        Section => 18,                    // OMPD_section
        Single => 19,                     // OMPD_single
        Workshare => 20,                  // OMPD_workshare
        Cancel => 21,                     // OMPD_cancel
        CancellationPoint => 22,          // OMPD_cancellation_point
        Allocate => 23,                   // OMPD_allocate
        Threadprivate => 24,              // OMPD_threadprivate
        DeclareReduction => 25,           // OMPD_declare_reduction
        DeclareMapper => 26,              // OMPD_declare_mapper
        ParallelFor => 27,                // OMPD_parallel_for
        ParallelDo => 28,                 // OMPD_parallel_do
        ParallelLoop => 29,               // OMPD_parallel_loop
        ParallelSections => 30,           // OMPD_parallel_sections
        ParallelSingle => 31,             // OMPD_parallel_single
        ParallelWorkshare => 32,          // OMPD_parallel_workshare
        ParallelMaster => 33,             // OMPD_parallel_master
        MasterTaskloop => 34,             // OMPD_master_taskloop
        MasterTaskloopSimd => 35,         // OMPD_master_taskloop_simd
        ParallelMasterTaskloop => 36,     // OMPD_parallel_master_taskloop
        ParallelMasterTaskloopSimd => 37, // OMPD_parallel_master_taskloop_simd
        Teams => 38,                      // OMPD_teams
        Metadirective => 39,              // OMPD_metadirective
        DeclareVariant => 40,             // OMPD_declare_variant
        BeginDeclareVariant => 41,        // OMPD_begin_declare_variant
        EndDeclareVariant => 42,          // OMPD_end_declare_variant
        Task => 43,                       // OMPD_task
        Taskloop => 44,                   // OMPD_taskloop
        TaskloopSimd => 45,               // OMPD_taskloop_simd
        Taskyield => 46,                  // OMPD_taskyield
        Requires => 47,                   // OMPD_requires
        TargetData => 48,                 // OMPD_target_data
        TargetDataComposite => 49,        // OMPD_target_data_composite
        TargetEnterData => 50,            // OMPD_target_enter_data
        TargetUpdate => 51,               // OMPD_target_update
        TargetExitData => 52,             // OMPD_target_exit_data
        Target => 53,                     // OMPD_target
        DeclareTarget => 54,              // OMPD_declare_target
        BeginDeclareTarget => 55,         // OMPD_begin_declare_target
        EndDeclareTarget => 56,           // OMPD_end_declare_target
        Master => 57,                     // OMPD_master
        End => 58,                        // OMPD_end (bare "end" only)
        // End directives - each gets unique constant for enum-based compat layer
        // These map to OMPD_end in ompparser but need unique ROUP constants
        EndParallel => 131,
        EndDo => 132,
        EndSimd => 133,
        EndSections => 134,
        EndSingle => 135,
        EndWorkshare => 136,
        EndOrdered => 137,
        EndLoop => 138,
        EndDistribute => 139,
        EndTeams => 140,
        EndTaskloop => 141,
        EndTask => 142,
        EndTaskgroup => 143,
        EndMaster => 144,
        EndCritical => 145,
        EndAtomic => 146,
        EndParallelDo => 147,
        EndParallelFor => 148,
        EndParallelSections => 149,
        EndParallelWorkshare => 150,
        EndParallelMaster => 151,
        EndDoSimd => 152,
        EndForSimd => 153,
        EndParallelDoSimd => 154,
        EndParallelForSimd => 155,
        EndDistributeSimd => 156,
        EndDistributeParallelDo => 157,
        EndDistributeParallelFor => 158,
        EndDistributeParallelDoSimd => 159,
        EndDistributeParallelForSimd => 160,
        EndTargetParallel => 161,
        EndTargetParallelDo => 162,
        EndTargetParallelFor => 163,
        EndTargetParallelDoSimd => 164,
        EndTargetParallelForSimd => 165,
        EndTargetSimd => 166,
        EndTargetTeams => 167,
        EndTargetTeamsDistribute => 168,
        EndTargetTeamsDistributeParallelDo => 169,
        EndTargetTeamsDistributeParallelFor => 170,
        EndTargetTeamsDistributeParallelDoSimd => 171,
        EndTargetTeamsDistributeParallelForSimd => 172,
        EndTargetTeamsDistributeSimd => 173,
        EndTargetTeamsLoop => 174,
        EndTeamsDistribute => 175,
        EndTeamsDistributeParallelDo => 176,
        EndTeamsDistributeParallelFor => 177,
        EndTeamsDistributeParallelDoSimd => 178,
        EndTeamsDistributeParallelForSimd => 179,
        EndTeamsDistributeSimd => 180,
        EndTeamsLoop => 181,
        EndTaskloopSimd => 182,
        EndMasterTaskloop => 183,
        EndMasterTaskloopSimd => 184,
        EndParallelMasterTaskloop => 185,
        EndParallelMasterTaskloopSimd => 186,
        EndTargetParallelLoop => 187,
        EndParallelLoop => 188,
        EndTargetLoop => 189,
        EndSection => 190,
        EndUnroll => 196,
        EndTile => 197,
        EndMasked => 198,
        EndMaskedTaskloop => 199,
        EndMaskedTaskloopSimd => 200,
        EndParallelMasked => 201,
        EndParallelMaskedTaskloop => 202,
        EndParallelMaskedTaskloopSimd => 203,
        Barrier => 59,   // OMPD_barrier
        Taskwait => 60,  // OMPD_taskwait
        Unroll => 61,    // OMPD_unroll
        Tile => 62,      // OMPD_tile
        Taskgroup => 63, // OMPD_taskgroup
        Flush => 64,     // OMPD_flush
        Atomic => 65,    // OMPD_atomic
        // All atomic variants map to OMPD_atomic (read/write/update/capture are clauses)
        AtomicRead => 65,
        AtomicWrite => 65,
        AtomicUpdate => 65,
        AtomicCapture => 65,
        AtomicCompareCapture => 65,
        Critical => 66,                               // OMPD_critical
        Depobj => 67,                                 // OMPD_depobj
        Ordered => 68,                                // OMPD_ordered
        TeamsDistribute => 69,                        // OMPD_teams_distribute
        TeamsDistributeSimd => 70,                    // OMPD_teams_distribute_simd
        TeamsDistributeParallelFor => 71,             // OMPD_teams_distribute_parallel_for
        TeamsDistributeParallelForSimd => 72,         // OMPD_teams_distribute_parallel_for_simd
        TeamsLoop => 73,                              // OMPD_teams_loop
        TargetParallel => 74,                         // OMPD_target_parallel
        TargetParallelFor => 75,                      // OMPD_target_parallel_for
        TargetParallelForSimd => 76,                  // OMPD_target_parallel_for_simd
        TargetParallelLoop => 77,                     // OMPD_target_parallel_loop
        TargetSimd => 78,                             // OMPD_target_simd
        TargetTeams => 79,                            // OMPD_target_teams
        TargetTeamsDistribute => 80,                  // OMPD_target_teams_distribute
        TargetTeamsDistributeSimd => 81,              // OMPD_target_teams_distribute_simd
        TargetTeamsLoop => 82,                        // OMPD_target_teams_loop
        TargetTeamsDistributeParallelFor => 83,       // OMPD_target_teams_distribute_parallel_for
        TargetTeamsDistributeParallelForSimd => 84, // OMPD_target_teams_distribute_parallel_for_simd
        TeamsDistributeParallelDo => 85,            // OMPD_teams_distribute_parallel_do
        TeamsDistributeParallelDoSimd => 86,        // OMPD_teams_distribute_parallel_do_simd
        TargetParallelDo => 87,                     // OMPD_target_parallel_do
        TargetParallelDoSimd => 88,                 // OMPD_target_parallel_do_simd
        TargetTeamsDistributeParallelDo => 89,      // OMPD_target_teams_distribute_parallel_do
        TargetTeamsDistributeParallelDoSimd => 90,  // OMPD_target_teams_distribute_parallel_do_simd
        Error => 91,                                // OMPD_error
        Nothing => 92,                              // OMPD_nothing
        Masked => 93,                               // OMPD_masked
        Scope => 94,                                // OMPD_scope
        MaskedTaskloop => 95,                       // OMPD_masked_taskloop
        MaskedTaskloopSimd => 96,                   // OMPD_masked_taskloop_simd
        ParallelMasked => 97,                       // OMPD_parallel_masked
        ParallelMaskedTaskloop => 98,               // OMPD_parallel_masked_taskloop
        ParallelMaskedTaskloopSimd => 99,           // OMPD_parallel_masked_taskloop_simd
        Interop => 100,                             // OMPD_interop
        Assume => 101,                              // OMPD_assume
        EndAssume => 102,                           // OMPD_end_assume
        Assumes => 103,                             // OMPD_assumes
        BeginAssumes => 104,                        // OMPD_begin_assumes
        EndAssumes => 105,                          // OMPD_end_assumes
        Allocators => 106,                          // OMPD_allocators
        Taskgraph => 107,                           // OMPD_taskgraph
        TaskIteration => 108,                       // OMPD_task_iteration
        Dispatch => 109,                            // OMPD_dispatch
        Groupprivate => 110,                        // OMPD_groupprivate
        Workdistribute => 111,                      // OMPD_workdistribute
        Fuse => 112,                                // OMPD_fuse
        Interchange => 113,                         // OMPD_interchange
        Reverse => 114,                             // OMPD_reverse
        Split => 115,                               // OMPD_split
        Stripe => 116,                              // OMPD_stripe
        DeclareInduction => 117,                    // OMPD_declare_induction
        BeginMetadirective => 118,                  // OMPD_begin_metadirective
        ParallelLoopSimd => 119,                    // OMPD_parallel_loop_simd
        TeamsLoopSimd => 120,                       // OMPD_teams_loop_simd
        TargetLoop => 121,                          // OMPD_target_loop
        TargetLoopSimd => 122,                      // OMPD_target_loop_simd
        TargetParallelLoopSimd => 123,              // OMPD_target_parallel_loop_simd
        TargetTeamsLoopSimd => 124,                 // OMPD_target_teams_loop_simd
        DistributeParallelLoop => 125,              // OMPD_distribute_parallel_loop
        DistributeParallelLoopSimd => 126,          // OMPD_distribute_parallel_loop_simd
        TeamsDistributeParallelLoop => 127,         // OMPD_teams_distribute_parallel_loop
        TeamsDistributeParallelLoopSimd => 128,     // OMPD_teams_distribute_parallel_loop_simd
        TargetTeamsDistributeParallelLoop => 129,   // OMPD_target_teams_distribute_parallel_loop
        TargetTeamsDistributeParallelLoopSimd => 130, // OMPD_target_teams_distribute_parallel_loop_simd

        // OpenACC-specific directives: these are not part of the OpenMP C API
        Data | EnterData | ExitData | HostData | Kernels | KernelsLoop | Update | Serial
        | SerialLoop | Routine | Set | Init | Shutdown | Cache | Wait | Declare => -1,

        // EndTarget and related end directives - unique constants
        EndTarget => 191,          // Maps to OMPD_end in ompparser
        EndTargetData => 192,      // Maps to OMPD_end in ompparser
        EndTargetEnterData => 193, // Maps to OMPD_end in ompparser
        EndTargetExitData => 194,  // Maps to OMPD_end in ompparser
        EndTargetUpdate => 195,    // Maps to OMPD_end in ompparser

        // NothingKnown is a placeholder
        NothingKnown => -1,

        // Unknown / unhandled directive — treat as error so maintainers notice
        Other(s) => {
            eprintln!(
                "[c_api] unknown directive mapping requested: {}",
                s.as_ref()
            );
            -1
        }
    };
    result
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
        // Free arguments string if present
        if !clause.arguments.is_null() {
            drop(CString::from_raw(clause.arguments as *mut c_char));
        }

        // Free variable lists if present
        // Clause kinds with variable lists (see convert_clause):
        //   3 = private, 4 = firstprivate, 5 = shared, 13 = lastprivate
        // Other kinds use different union fields:
        //   2 = default (uses .default field, NOT .variables)
        //   8 = reduction (uses .reduction field, NOT .variables)
        //   21 = schedule (uses .schedule field, NOT .variables)
        if is_reduction_clause_kind(clause.kind) {
            if let Some(data) = get_reduction_data(clause) {
                if !data.user_identifier.is_null() {
                    drop(CString::from_raw(data.user_identifier as *mut c_char));
                }
                if !data.modifiers_text.is_null() {
                    drop(CString::from_raw(data.modifiers_text as *mut c_char));
                }
                if !data.variables.is_null() {
                    roup_string_list_free(data.variables);
                }
            }
        } else if (clause.kind >= 3 && clause.kind <= 5) || clause.kind == 13 {
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
