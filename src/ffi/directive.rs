//! Directive query API for FFI
//!
//! This module provides functions to query information about parsed
//! OpenMP directives.
//!
//! ## Learning Objectives
//!
//! 1. **Enum Mapping**: Exposing Rust enums to C with repr(C)
//! 2. **Data Extraction**: Safely retrieving fields from complex structures
//! 3. **Cursor Pattern**: Implementing iteration without raw pointers
//! 4. **Type Safety**: Ensuring C code gets correct data types
//!
//! ## Design Philosophy
//!
//! ```text
//! Directive Handle → Query Functions → Primitive Data
//! ┌──────────────┐   ┌──────────────┐   ┌──────────────┐
//! │ u64 handle   │──>│ kind()       │──>│ DirectiveKind│
//! │              │   │ clause_count │──>│ usize        │
//! │              │   │ location()   │──>│ line, column │
//! └──────────────┘   └──────────────┘   └──────────────┘
//! ```

use super::registry::{insert, with_resource, Cursor, Resource};
use super::types::{Handle, OmpStatus, INVALID_HANDLE};
use crate::ir::DirectiveKind;

/// Get the kind of a directive
///
/// ## C Signature
/// ```c
/// int32_t omp_directive_kind(uint64_t handle);
/// ```
///
/// ## Returns
/// - DirectiveKind discriminant (0-255) on success
/// - -1 if handle is invalid or not a directive
///
/// ## Example
/// ```
/// use roup::ffi::{omp_str_new, omp_str_push_byte, omp_parse, omp_take_last_parse_result};
/// use roup::ffi::{omp_directive_kind, omp_directive_free};
/// use roup::ir::DirectiveKind;
///
/// let str_h = omp_str_new();
/// for &b in b"#pragma omp parallel" {
///     omp_str_push_byte(str_h, b);
/// }
///
/// omp_parse(str_h, std::ptr::null_mut());
/// let dir_h = omp_take_last_parse_result();
///
/// let kind = omp_directive_kind(dir_h);
/// assert_eq!(kind, DirectiveKind::Parallel as i32);
///
/// omp_directive_free(dir_h);
/// # use roup::ffi::omp_str_free;
/// # omp_str_free(str_h);
/// ```
#[no_mangle]
pub extern "C" fn omp_directive_kind(handle: Handle) -> i32 {
    match with_resource(handle, |res| match res {
        Resource::Ast(ir) => Some(ir.kind() as i32),
        _ => None,
    }) {
        Some(Some(kind)) => kind,
        _ => -1,
    }
}

/// Get the number of clauses in a directive
///
/// ## C Signature
/// ```c
/// size_t omp_directive_clause_count(uint64_t handle);
/// ```
///
/// ## Returns
/// - Number of clauses (0 or more) on success
/// - 0 if handle is invalid (indistinguishable from empty directive)
///
/// ## Example
/// ```
/// use roup::ffi::*;
///
/// let str_h = omp_str_new();
/// for &b in b"#pragma omp parallel num_threads(4) default(shared)" {
///     omp_str_push_byte(str_h, b);
/// }
///
/// omp_parse(str_h, std::ptr::null_mut());
/// let dir_h = omp_take_last_parse_result();
///
/// let count = omp_directive_clause_count(dir_h);
/// assert_eq!(count, 2);
///
/// omp_directive_free(dir_h);
/// omp_str_free(str_h);
/// ```
#[no_mangle]
pub extern "C" fn omp_directive_clause_count(handle: Handle) -> usize {
    match with_resource(handle, |res| match res {
        Resource::Ast(ir) => Some(ir.clauses().len()),
        _ => None,
    }) {
        Some(Some(count)) => count,
        _ => 0,
    }
}

/// Get the source location (line number) of a directive
///
/// ## C Signature
/// ```c
/// uint32_t omp_directive_line(uint64_t handle);
/// ```
///
/// ## Returns
/// - Line number (1-based) on success
/// - 0 if handle is invalid
#[no_mangle]
pub extern "C" fn omp_directive_line(handle: Handle) -> u32 {
    match with_resource(handle, |res| match res {
        Resource::Ast(ir) => Some(ir.location().line),
        _ => None,
    }) {
        Some(Some(line)) => line,
        _ => 0,
    }
}

/// Get the source location (column number) of a directive
///
/// ## C Signature
/// ```c
/// uint32_t omp_directive_column(uint64_t handle);
/// ```
///
/// ## Returns
/// - Column number (1-based) on success
/// - 0 if handle is invalid
#[no_mangle]
pub extern "C" fn omp_directive_column(handle: Handle) -> u32 {
    match with_resource(handle, |res| match res {
        Resource::Ast(ir) => Some(ir.location().column),
        _ => None,
    }) {
        Some(Some(column)) => column,
        _ => 0,
    }
}

/// Get the language of a directive
///
/// ## C Signature
/// ```c
/// int32_t omp_directive_language(uint64_t handle);
/// ```
///
/// ## Returns
/// - Language discriminant (0=C, 1=Cpp, 2=Fortran) on success
/// - -1 if handle is invalid
#[no_mangle]
pub extern "C" fn omp_directive_language(handle: Handle) -> i32 {
    match with_resource(handle, |res| match res {
        Resource::Ast(ir) => Some(ir.language() as i32),
        _ => None,
    }) {
        Some(Some(lang)) => lang,
        _ => -1,
    }
}

/// Create a cursor for iterating over directive clauses
///
/// ## C Signature
/// ```c
/// uint64_t omp_directive_clauses_cursor(uint64_t directive_handle);
/// ```
///
/// ## Returns
/// - Cursor handle on success
/// - 0 (INVALID_HANDLE) if directive handle is invalid
///
/// ## Note
/// Caller must free the cursor with omp_cursor_free() when done.
///
/// ## Example
/// ```
/// use roup::ffi::*;
///
/// let str_h = omp_str_new();
/// for &b in b"#pragma omp parallel num_threads(4)" {
///     omp_str_push_byte(str_h, b);
/// }
///
/// omp_parse(str_h, std::ptr::null_mut());
/// let dir_h = omp_take_last_parse_result();
///
/// let cursor_h = omp_directive_clauses_cursor(dir_h);
/// assert_ne!(cursor_h, INVALID_HANDLE);
///
/// // Use cursor to iterate (future API)...
///
/// omp_cursor_free(cursor_h);
/// omp_directive_free(dir_h);
/// omp_str_free(str_h);
/// ```
#[no_mangle]
pub extern "C" fn omp_directive_clauses_cursor(directive_handle: Handle) -> Handle {
    // Get the clause count to create a cursor with the right size
    let count = match with_resource(directive_handle, |res| match res {
        Resource::Ast(ir) => Some(ir.clauses().len()),
        _ => None,
    }) {
        Some(Some(c)) => c,
        _ => return INVALID_HANDLE,
    };

    // Create cursor with empty handles (we'll use index to track position)
    // The cursor just needs to know the count for iteration
    let cursor = Cursor::new(vec![directive_handle; count]);
    insert(Resource::Cursor(cursor))
}

/// Free a cursor
///
/// ## C Signature
/// ```c
/// OmpStatus omp_cursor_free(uint64_t handle);
/// ```
///
/// ## Returns
/// - Ok: Cursor freed
/// - NotFound: Invalid handle
/// - Invalid: Not a cursor handle
#[no_mangle]
pub extern "C" fn omp_cursor_free(handle: Handle) -> OmpStatus {
    match super::registry::remove(handle) {
        Some(Resource::Cursor(_)) => OmpStatus::Ok,
        Some(_) => OmpStatus::Invalid,
        None => OmpStatus::NotFound,
    }
}

/// Check if cursor has more items
///
/// ## C Signature
/// ```c
/// int32_t omp_cursor_has_next(uint64_t handle);
/// ```
///
/// ## Returns
/// - 1 if cursor has more items
/// - 0 if cursor is exhausted or handle is invalid
#[no_mangle]
pub extern "C" fn omp_cursor_has_next(handle: Handle) -> i32 {
    match with_resource(handle, |res| match res {
        Resource::Cursor(cursor) => Some(!cursor.is_done()),
        _ => None,
    }) {
        Some(Some(has_next)) => has_next as i32,
        _ => 0,
    }
}

/// Get current cursor position
///
/// ## C Signature
/// ```c
/// size_t omp_cursor_position(uint64_t handle);
/// ```
///
/// ## Returns
/// - Current position (0-based index) on success
/// - 0 if handle is invalid (indistinguishable from position 0)
#[no_mangle]
pub extern "C" fn omp_cursor_position(handle: Handle) -> usize {
    match with_resource(handle, |res| match res {
        Resource::Cursor(cursor) => Some(cursor.index),
        _ => None,
    }) {
        Some(Some(pos)) => pos,
        _ => 0,
    }
}

/// Advance cursor to next item
///
/// ## C Signature
/// ```c
/// OmpStatus omp_cursor_next(uint64_t handle);
/// ```
///
/// ## Returns
/// - Ok: Cursor advanced
/// - OutOfRange: Cursor already at end
/// - NotFound: Invalid handle
/// - Invalid: Not a cursor handle
#[no_mangle]
pub extern "C" fn omp_cursor_next(handle: Handle) -> OmpStatus {
    match super::registry::with_resource_mut(handle, |res| match res {
        Resource::Cursor(cursor) => {
            if cursor.next().is_some() {
                Some(())
            } else {
                None
            }
        }
        _ => None,
    }) {
        Some(Some(())) => OmpStatus::Ok,
        Some(None) => OmpStatus::OutOfRange,
        None => OmpStatus::NotFound,
    }
}

/// Reset cursor to beginning
///
/// ## C Signature
/// ```c
/// OmpStatus omp_cursor_reset(uint64_t handle);
/// ```
///
/// ## Returns
/// - Ok: Cursor reset
/// - NotFound: Invalid handle
/// - Invalid: Not a cursor handle
#[no_mangle]
pub extern "C" fn omp_cursor_reset(handle: Handle) -> OmpStatus {
    match super::registry::with_resource_mut(handle, |res| match res {
        Resource::Cursor(cursor) => {
            cursor.reset();
            Some(())
        }
        _ => None,
    }) {
        Some(Some(())) => OmpStatus::Ok,
        Some(None) => OmpStatus::Invalid,
        None => OmpStatus::NotFound,
    }
}

/// Get the total number of items in cursor
///
/// ## C Signature
/// ```c
/// size_t omp_cursor_count(uint64_t handle);
/// ```
///
/// ## Returns
/// - Total number of items on success
/// - 0 if handle is invalid (indistinguishable from empty cursor)
#[no_mangle]
pub extern "C" fn omp_cursor_count(handle: Handle) -> usize {
    match with_resource(handle, |res| match res {
        Resource::Cursor(cursor) => Some(cursor.items.len()),
        _ => None,
    }) {
        Some(Some(count)) => count,
        _ => 0,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi::parse::{omp_directive_free, omp_parse, omp_take_last_parse_result};
    use crate::ffi::registry::REGISTRY;
    use crate::ffi::string::{omp_str_free, omp_str_new, omp_str_push_byte};
    use crate::ir::Language;

    fn cleanup() {
        REGISTRY.lock().clear();
    }

    fn build_string(text: &str) -> Handle {
        let h = omp_str_new();
        for &b in text.as_bytes() {
            omp_str_push_byte(h, b);
        }
        h
    }

    fn parse_directive(text: &str) -> Handle {
        let str_h = build_string(text);
        omp_parse(str_h, std::ptr::null_mut());
        let dir_h = omp_take_last_parse_result();
        omp_str_free(str_h);
        dir_h
    }

    #[test]
    fn test_directive_kind() {
        cleanup();

        let dir_h = parse_directive("#pragma omp parallel");
        assert_eq!(omp_directive_kind(dir_h), DirectiveKind::Parallel as i32);
        omp_directive_free(dir_h);

        let dir_h = parse_directive("#pragma omp for");
        assert_eq!(omp_directive_kind(dir_h), DirectiveKind::For as i32);
        omp_directive_free(dir_h);

        let dir_h = parse_directive("#pragma omp barrier");
        assert_eq!(omp_directive_kind(dir_h), DirectiveKind::Barrier as i32);
        omp_directive_free(dir_h);
    }

    #[test]
    fn test_directive_kind_invalid_handle() {
        cleanup();
        assert_eq!(omp_directive_kind(INVALID_HANDLE), -1);
        assert_eq!(omp_directive_kind(9999), -1);
    }

    #[test]
    fn test_clause_count() {
        cleanup();

        let dir_h = parse_directive("#pragma omp parallel");
        assert_eq!(omp_directive_clause_count(dir_h), 0);
        omp_directive_free(dir_h);

        let dir_h = parse_directive("#pragma omp parallel num_threads(4)");
        assert_eq!(omp_directive_clause_count(dir_h), 1);
        omp_directive_free(dir_h);

        let dir_h = parse_directive("#pragma omp parallel default(shared) num_threads(4)");
        assert_eq!(omp_directive_clause_count(dir_h), 2);
        omp_directive_free(dir_h);
    }

    #[test]
    fn test_clause_count_invalid_handle() {
        cleanup();
        assert_eq!(omp_directive_clause_count(INVALID_HANDLE), 0);
    }

    #[test]
    fn test_directive_location() {
        cleanup();

        let dir_h = parse_directive("#pragma omp parallel");
        assert_eq!(omp_directive_line(dir_h), 1);
        assert_eq!(omp_directive_column(dir_h), 1);
        omp_directive_free(dir_h);
    }

    #[test]
    fn test_directive_location_invalid_handle() {
        cleanup();
        assert_eq!(omp_directive_line(INVALID_HANDLE), 0);
        assert_eq!(omp_directive_column(INVALID_HANDLE), 0);
    }

    #[test]
    fn test_directive_language() {
        cleanup();

        let dir_h = parse_directive("#pragma omp parallel");
        assert_eq!(omp_directive_language(dir_h), Language::C as i32);
        omp_directive_free(dir_h);
    }

    #[test]
    fn test_directive_language_invalid_handle() {
        cleanup();
        assert_eq!(omp_directive_language(INVALID_HANDLE), -1);
    }

    #[test]
    fn test_clauses_cursor() {
        cleanup();

        let dir_h = parse_directive("#pragma omp parallel num_threads(4) default(shared)");
        let cursor_h = omp_directive_clauses_cursor(dir_h);

        assert_ne!(cursor_h, INVALID_HANDLE);
        assert_eq!(omp_cursor_count(cursor_h), 2);
        assert_eq!(omp_cursor_position(cursor_h), 0);
        assert_eq!(omp_cursor_has_next(cursor_h), 1);

        omp_cursor_free(cursor_h);
        omp_directive_free(dir_h);
    }

    #[test]
    fn test_cursor_iteration() {
        cleanup();

        let dir_h = parse_directive("#pragma omp parallel num_threads(4) default(shared)");
        let cursor_h = omp_directive_clauses_cursor(dir_h);

        // Initial state
        assert_eq!(omp_cursor_position(cursor_h), 0);
        assert_eq!(omp_cursor_has_next(cursor_h), 1);

        // Advance once
        assert_eq!(omp_cursor_next(cursor_h), OmpStatus::Ok);
        assert_eq!(omp_cursor_position(cursor_h), 1);
        assert_eq!(omp_cursor_has_next(cursor_h), 1);

        // Advance again
        assert_eq!(omp_cursor_next(cursor_h), OmpStatus::Ok);
        assert_eq!(omp_cursor_position(cursor_h), 2);
        assert_eq!(omp_cursor_has_next(cursor_h), 0);

        // Try to advance past end
        assert_eq!(omp_cursor_next(cursor_h), OmpStatus::OutOfRange);

        omp_cursor_free(cursor_h);
        omp_directive_free(dir_h);
    }

    #[test]
    fn test_cursor_reset() {
        cleanup();

        let dir_h = parse_directive("#pragma omp parallel num_threads(4)");
        let cursor_h = omp_directive_clauses_cursor(dir_h);

        // Advance
        omp_cursor_next(cursor_h);
        assert_eq!(omp_cursor_position(cursor_h), 1);

        // Reset
        assert_eq!(omp_cursor_reset(cursor_h), OmpStatus::Ok);
        assert_eq!(omp_cursor_position(cursor_h), 0);
        assert_eq!(omp_cursor_has_next(cursor_h), 1);

        omp_cursor_free(cursor_h);
        omp_directive_free(dir_h);
    }

    #[test]
    fn test_cursor_empty() {
        cleanup();

        let dir_h = parse_directive("#pragma omp barrier");
        let cursor_h = omp_directive_clauses_cursor(dir_h);

        assert_eq!(omp_cursor_count(cursor_h), 0);
        assert_eq!(omp_cursor_has_next(cursor_h), 0);
        assert_eq!(omp_cursor_next(cursor_h), OmpStatus::OutOfRange);

        omp_cursor_free(cursor_h);
        omp_directive_free(dir_h);
    }

    #[test]
    fn test_cursor_free() {
        cleanup();

        let dir_h = parse_directive("#pragma omp parallel");
        let cursor_h = omp_directive_clauses_cursor(dir_h);

        assert_eq!(omp_cursor_free(cursor_h), OmpStatus::Ok);
        assert_eq!(omp_cursor_free(cursor_h), OmpStatus::NotFound); // Double free

        omp_directive_free(dir_h);
    }

    #[test]
    fn test_cursor_invalid_handle() {
        cleanup();

        assert_eq!(omp_cursor_has_next(INVALID_HANDLE), 0);
        assert_eq!(omp_cursor_position(INVALID_HANDLE), 0);
        assert_eq!(omp_cursor_count(INVALID_HANDLE), 0);
        assert_eq!(omp_cursor_next(INVALID_HANDLE), OmpStatus::NotFound);
        assert_eq!(omp_cursor_reset(INVALID_HANDLE), OmpStatus::NotFound);
        assert_eq!(omp_cursor_free(INVALID_HANDLE), OmpStatus::NotFound);
    }

    #[test]
    fn test_all_directive_kinds() {
        cleanup();

        let test_cases = vec![
            ("#pragma omp parallel", DirectiveKind::Parallel),
            ("#pragma omp for", DirectiveKind::For),
            ("#pragma omp task", DirectiveKind::Task),
            ("#pragma omp barrier", DirectiveKind::Barrier),
            ("#pragma omp taskwait", DirectiveKind::Taskwait),
            ("#pragma omp critical", DirectiveKind::Critical),
            ("#pragma omp atomic", DirectiveKind::Atomic),
            ("#pragma omp master", DirectiveKind::Master),
            ("#pragma omp single", DirectiveKind::Single),
            ("#pragma omp simd", DirectiveKind::Simd),
        ];

        for (input, expected_kind) in test_cases {
            let dir_h = parse_directive(input);
            assert_eq!(
                omp_directive_kind(dir_h),
                expected_kind as i32,
                "Failed for: {}",
                input
            );
            omp_directive_free(dir_h);
        }
    }

    #[test]
    fn test_multiple_cursors() {
        cleanup();

        let dir_h = parse_directive("#pragma omp parallel num_threads(4) default(shared)");

        let cursor1_h = omp_directive_clauses_cursor(dir_h);
        let cursor2_h = omp_directive_clauses_cursor(dir_h);

        assert_ne!(cursor1_h, cursor2_h);
        assert_eq!(omp_cursor_count(cursor1_h), 2);
        assert_eq!(omp_cursor_count(cursor2_h), 2);

        // Advance cursor1
        omp_cursor_next(cursor1_h);
        assert_eq!(omp_cursor_position(cursor1_h), 1);
        assert_eq!(omp_cursor_position(cursor2_h), 0); // cursor2 unchanged

        omp_cursor_free(cursor1_h);
        omp_cursor_free(cursor2_h);
        omp_directive_free(dir_h);
    }

    #[test]
    fn test_cursor_concurrent() {
        use std::sync::Arc;
        use std::thread;

        cleanup();

        let dir_h = parse_directive("#pragma omp parallel num_threads(4)");
        let dir_h = Arc::new(dir_h);

        let threads: Vec<_> = (0..5)
            .map(|_| {
                let dir_h = Arc::clone(&dir_h);
                thread::spawn(move || {
                    let cursor_h = omp_directive_clauses_cursor(*dir_h);
                    assert_ne!(cursor_h, INVALID_HANDLE);
                    assert_eq!(omp_cursor_count(cursor_h), 1);
                    omp_cursor_free(cursor_h);
                })
            })
            .collect();

        for t in threads {
            t.join().unwrap();
        }

        omp_directive_free(*dir_h);
    }

    #[test]
    fn test_directive_query_combined() {
        cleanup();

        let dir_h = parse_directive("#pragma omp parallel for num_threads(8) schedule(static)");

        // Check all properties
        assert_eq!(omp_directive_kind(dir_h), DirectiveKind::ParallelFor as i32);
        assert_eq!(omp_directive_clause_count(dir_h), 2);
        assert_eq!(omp_directive_line(dir_h), 1);
        assert_eq!(omp_directive_column(dir_h), 1);
        assert_eq!(omp_directive_language(dir_h), Language::C as i32);

        // Check cursor
        let cursor_h = omp_directive_clauses_cursor(dir_h);
        assert_eq!(omp_cursor_count(cursor_h), 2);

        omp_cursor_free(cursor_h);
        omp_directive_free(dir_h);
    }
}
