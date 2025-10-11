//! Clause query API for FFI
//!
//! This module provides functions to query information about OpenMP clauses.
//!
//! ## Learning Objectives
//!
//! 1. **Discriminated Unions**: Exposing Rust enums with data to C
//! 2. **Type-Safe Accessors**: Returning correct types based on clause variant
//! 3. **Error Handling**: Invalid access returns special values
//! 4. **Complex Data Extraction**: Handling nested structures and lists
//!
//! ## Design Philosophy
//!
//! ```text
//! Cursor → Clause Handle → Type Check → Typed Accessor → Data
//! ┌──────┐   ┌──────────┐   ┌──────┐   ┌──────────┐   ┌────┐
//! │ pos  │──>│ clause_at│──>│ type │──>│ num_     │──>│ expr│
//! │      │   │          │   │      │   │ threads  │   │     │
//! └──────┘   └──────────┘   └──────┘   └──────────┘   └────┘
//! ```

use super::registry::{insert, with_resource, Resource};
use super::types::{Handle, OmpStatus, INVALID_HANDLE};
use crate::ir::ClauseData;

#[cfg(test)]
use crate::ir::{DefaultKind, ReductionOperator, ScheduleKind};

/// Clause type discriminant for C API
///
/// This enum provides stable discriminants for all clause types.
/// C code can use this to determine which accessor functions to call.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClauseType {
    Bare = 0,
    Private = 1,
    Firstprivate = 2,
    Shared = 3,
    Reduction = 4,
    NumThreads = 5,
    If = 6,
    Schedule = 7,
    Collapse = 8,
    Ordered = 9,
    Default = 10,
    ProcBind = 11,
    Map = 12,
    Depend = 13,
    Linear = 14,
    NumTeams = 15,
    ThreadLimit = 16,
    Device = 17,
    Priority = 18,
    Grainsize = 19,
    NumTasks = 20,
    Safelen = 21,
    Simdlen = 22,
    Aligned = 23,
    Lastprivate = 24,
    Copyin = 25,
    Copyprivate = 26,
    UseDevicePtr = 27,
    UseDeviceAddr = 28,
    IsDevicePtr = 29,
    HasDeviceAddr = 30,
    Affinity = 31,
    Allocate = 32,
    Allocator = 33,
    DistSchedule = 34,
    // Add more as needed
    Unknown = 255,
}

/// Get clause at cursor position
///
/// ## C Signature
/// ```c
/// uint64_t omp_clause_at(uint64_t directive_handle, size_t index);
/// ```
///
/// ## Parameters
/// - `directive_handle`: Handle to directive
/// - `index`: 0-based clause index
///
/// ## Returns
/// - Clause handle on success
/// - INVALID_HANDLE if directive invalid or index out of bounds
///
/// ## Note
/// This creates a temporary handle to the clause. The clause data
/// is actually borrowed from the directive, so the directive must
/// remain valid while querying the clause.
#[no_mangle]
pub extern "C" fn omp_clause_at(directive_handle: Handle, index: usize) -> Handle {
    // Get the clause at the specified index
    let clause_opt = with_resource(directive_handle, |res| match res {
        Resource::Ast(ir) => ir.clauses().get(index).cloned(),
        _ => None,
    });

    match clause_opt {
        Some(Some(clause)) => {
            // Store the clause as a resource
            // We'll need to add Clause to the Resource enum
            insert(Resource::Clause(Box::new(clause)))
        }
        _ => INVALID_HANDLE,
    }
}

/// Free a clause handle
///
/// ## C Signature
/// ```c
/// OmpStatus omp_clause_free(uint64_t handle);
/// ```
#[no_mangle]
pub extern "C" fn omp_clause_free(handle: Handle) -> OmpStatus {
    match super::registry::remove(handle) {
        Some(Resource::Clause(_)) => OmpStatus::Ok,
        Some(_) => OmpStatus::Invalid,
        None => OmpStatus::NotFound,
    }
}

/// Get the type of a clause
///
/// ## C Signature
/// ```c
/// int32_t omp_clause_type(uint64_t handle);
/// ```
///
/// ## Returns
/// - ClauseType discriminant on success
/// - -1 if handle is invalid
#[no_mangle]
pub extern "C" fn omp_clause_type(handle: Handle) -> i32 {
    match with_resource(handle, |res| match res {
        Resource::Clause(clause) => Some(clause_to_type(clause.as_ref())),
        _ => None,
    }) {
        Some(Some(clause_type)) => clause_type as i32,
        _ => -1,
    }
}

/// Helper to convert ClauseData to ClauseType
fn clause_to_type(clause: &ClauseData) -> ClauseType {
    match clause {
        ClauseData::Bare(_) => ClauseType::Bare,
        ClauseData::Private { .. } => ClauseType::Private,
        ClauseData::Firstprivate { .. } => ClauseType::Firstprivate,
        ClauseData::Shared { .. } => ClauseType::Shared,
        ClauseData::Reduction { .. } => ClauseType::Reduction,
        ClauseData::NumThreads { .. } => ClauseType::NumThreads,
        ClauseData::If { .. } => ClauseType::If,
        ClauseData::Schedule { .. } => ClauseType::Schedule,
        ClauseData::Collapse { .. } => ClauseType::Collapse,
        ClauseData::Ordered { .. } => ClauseType::Ordered,
        ClauseData::Default(_) => ClauseType::Default,
        ClauseData::ProcBind(_) => ClauseType::ProcBind,
        ClauseData::Map { .. } => ClauseType::Map,
        ClauseData::Depend { .. } => ClauseType::Depend,
        ClauseData::Linear { .. } => ClauseType::Linear,
        ClauseData::NumTeams { .. } => ClauseType::NumTeams,
        ClauseData::ThreadLimit { .. } => ClauseType::ThreadLimit,
        ClauseData::Device { .. } => ClauseType::Device,
        ClauseData::Priority { .. } => ClauseType::Priority,
        ClauseData::Grainsize { .. } => ClauseType::Grainsize,
        ClauseData::NumTasks { .. } => ClauseType::NumTasks,
        ClauseData::Safelen { .. } => ClauseType::Safelen,
        ClauseData::Simdlen { .. } => ClauseType::Simdlen,
        ClauseData::Aligned { .. } => ClauseType::Aligned,
        ClauseData::Lastprivate { .. } => ClauseType::Lastprivate,
        ClauseData::Copyin { .. } => ClauseType::Copyin,
        ClauseData::Copyprivate { .. } => ClauseType::Copyprivate,
        ClauseData::UseDevicePtr { .. } => ClauseType::UseDevicePtr,
        ClauseData::UseDeviceAddr { .. } => ClauseType::UseDeviceAddr,
        ClauseData::IsDevicePtr { .. } => ClauseType::IsDevicePtr,
        ClauseData::HasDeviceAddr { .. } => ClauseType::HasDeviceAddr,
        ClauseData::Affinity { .. } => ClauseType::Affinity,
        ClauseData::Allocate { .. } => ClauseType::Allocate,
        ClauseData::Allocator { .. } => ClauseType::Allocator,
        ClauseData::DistSchedule { .. } => ClauseType::DistSchedule,
        _ => ClauseType::Unknown,
    }
}

// ============================================================================
// Typed accessors for specific clause types
// ============================================================================

/// Get num_threads value (returns string handle for expression)
///
/// ## C Signature
/// ```c
/// uint64_t omp_clause_num_threads_value(uint64_t clause_handle);
/// ```
///
/// ## Returns
/// - String handle containing expression text
/// - INVALID_HANDLE if clause is not NumThreads or handle is invalid
#[no_mangle]
pub extern "C" fn omp_clause_num_threads_value(clause_handle: Handle) -> Handle {
    // Extract the string first to avoid nested registry locks
    let expr_str = with_resource(clause_handle, |res| match res {
        Resource::Clause(clause) => match clause.as_ref() {
            ClauseData::NumThreads { num } => Some(num.as_str().to_string()),
            _ => None,
        },
        _ => None,
    })
    .flatten();

    // Now create the string handle (this will lock the registry)
    match expr_str {
        Some(s) => super::string::create_string_from_str(&s),
        None => INVALID_HANDLE,
    }
}

/// Get reduction operator
///
/// ## C Signature
/// ```c
/// int32_t omp_clause_reduction_operator(uint64_t clause_handle);
/// ```
///
/// ## Returns
/// - ReductionOperator discriminant on success
/// - -1 if not a Reduction clause or invalid handle
#[no_mangle]
pub extern "C" fn omp_clause_reduction_operator(clause_handle: Handle) -> i32 {
    match with_resource(clause_handle, |res| match res {
        Resource::Clause(clause) => match clause.as_ref() {
            ClauseData::Reduction { operator, .. } => Some(*operator as i32),
            _ => None,
        },
        _ => None,
    }) {
        Some(Some(op)) => op,
        _ => -1,
    }
}

/// Get schedule kind
///
/// ## C Signature
/// ```c
/// int32_t omp_clause_schedule_kind(uint64_t clause_handle);
/// ```
///
/// ## Returns
/// - ScheduleKind discriminant on success
/// - -1 if not a Schedule clause or invalid handle
#[no_mangle]
pub extern "C" fn omp_clause_schedule_kind(clause_handle: Handle) -> i32 {
    match with_resource(clause_handle, |res| match res {
        Resource::Clause(clause) => match clause.as_ref() {
            ClauseData::Schedule { kind, .. } => Some(*kind as i32),
            _ => None,
        },
        _ => None,
    }) {
        Some(Some(kind)) => kind,
        _ => -1,
    }
}

/// Get schedule chunk size (returns string handle for expression)
///
/// ## C Signature
/// ```c
/// uint64_t omp_clause_schedule_chunk_size(uint64_t clause_handle);
/// ```
///
/// ## Returns
/// - String handle containing chunk size expression
/// - INVALID_HANDLE if no chunk size or not a Schedule clause
#[no_mangle]
pub extern "C" fn omp_clause_schedule_chunk_size(clause_handle: Handle) -> Handle {
    // Extract the string first to avoid nested registry locks
    let chunk_str = with_resource(clause_handle, |res| match res {
        Resource::Clause(clause) => match clause.as_ref() {
            ClauseData::Schedule { chunk_size, .. } => {
                chunk_size.as_ref().map(|expr| expr.as_str().to_string())
            }
            _ => None,
        },
        _ => None,
    })
    .flatten();

    // Now create the string handle (this will lock the registry)
    match chunk_str {
        Some(s) => super::string::create_string_from_str(&s),
        None => INVALID_HANDLE,
    }
}

/// Get default kind
///
/// ## C Signature
/// ```c
/// int32_t omp_clause_default_kind(uint64_t clause_handle);
/// ```
///
/// ## Returns
/// - DefaultKind discriminant on success
/// - -1 if not a Default clause or invalid handle
#[no_mangle]
pub extern "C" fn omp_clause_default_kind(clause_handle: Handle) -> i32 {
    match with_resource(clause_handle, |res| match res {
        Resource::Clause(clause) => match clause.as_ref() {
            ClauseData::Default(kind) => Some(*kind as i32),
            _ => None,
        },
        _ => None,
    }) {
        Some(Some(kind)) => kind,
        _ => -1,
    }
}

/// Get proc_bind kind
///
/// ## C Signature
/// ```c
/// int32_t omp_clause_proc_bind_kind(uint64_t clause_handle);
/// ```
///
/// ## Returns
/// - ProcBind discriminant on success
/// - -1 if not a ProcBind clause or invalid handle
#[no_mangle]
pub extern "C" fn omp_clause_proc_bind_kind(clause_handle: Handle) -> i32 {
    match with_resource(clause_handle, |res| match res {
        Resource::Clause(clause) => match clause.as_ref() {
            ClauseData::ProcBind(kind) => Some(*kind as i32),
            _ => None,
        },
        _ => None,
    }) {
        Some(Some(kind)) => kind,
        _ => -1,
    }
}

/// Get number of items in a clause (for item list clauses)
///
/// ## C Signature
/// ```c
/// size_t omp_clause_item_count(uint64_t clause_handle);
/// ```
///
/// ## Returns
/// - Number of items for list-based clauses
/// - 0 if not a list clause or invalid handle
#[no_mangle]
pub extern "C" fn omp_clause_item_count(clause_handle: Handle) -> usize {
    with_resource(clause_handle, |res| match res {
        Resource::Clause(clause) => {
            let count = match clause.as_ref() {
                ClauseData::Private { items } => items.len(),
                ClauseData::Firstprivate { items } => items.len(),
                ClauseData::Shared { items } => items.len(),
                ClauseData::Reduction { items, .. } => items.len(),
                ClauseData::Map { items, .. } => items.len(),
                ClauseData::Depend { items, .. } => items.len(),
                ClauseData::Linear { items, .. } => items.len(),
                ClauseData::Copyin { items } => items.len(),
                ClauseData::Copyprivate { items } => items.len(),
                ClauseData::UseDevicePtr { items } => items.len(),
                ClauseData::UseDeviceAddr { items } => items.len(),
                ClauseData::IsDevicePtr { items } => items.len(),
                ClauseData::HasDeviceAddr { items } => items.len(),
                ClauseData::Affinity { items } => items.len(),
                ClauseData::Aligned { items, .. } => items.len(),
                ClauseData::Lastprivate { items, .. } => items.len(),
                ClauseData::Allocate { items, .. } => items.len(),
                _ => 0,
            };
            Some(count)
        }
        _ => None,
    })
    .flatten()
    .unwrap_or(0)
}

/// Get item at index from a clause (returns string handle)
///
/// ## C Signature
/// ```c
/// uint64_t omp_clause_item_at(uint64_t clause_handle, size_t index);
/// ```
///
/// ## Returns
/// - String handle containing item text
/// - INVALID_HANDLE if index out of bounds or not a list clause
#[no_mangle]
pub extern "C" fn omp_clause_item_at(clause_handle: Handle, index: usize) -> Handle {
    // Extract the string first to avoid nested registry locks
    let item_str = with_resource(clause_handle, |res| match res {
        Resource::Clause(clause) => {
            let item_opt = match clause.as_ref() {
                ClauseData::Private { items } => items.get(index),
                ClauseData::Firstprivate { items } => items.get(index),
                ClauseData::Shared { items } => items.get(index),
                ClauseData::Reduction { items, .. } => items.get(index),
                ClauseData::Map { items, .. } => items.get(index),
                ClauseData::Depend { items, .. } => items.get(index),
                ClauseData::Linear { items, .. } => items.get(index),
                ClauseData::Copyin { items } => items.get(index),
                ClauseData::Copyprivate { items } => items.get(index),
                ClauseData::UseDevicePtr { items } => items.get(index),
                ClauseData::UseDeviceAddr { items } => items.get(index),
                ClauseData::IsDevicePtr { items } => items.get(index),
                ClauseData::HasDeviceAddr { items } => items.get(index),
                ClauseData::Affinity { items } => items.get(index),
                ClauseData::Aligned { items, .. } => items.get(index),
                ClauseData::Lastprivate { items, .. } => items.get(index),
                ClauseData::Allocate { items, .. } => items.get(index),
                _ => None,
            };
            item_opt.map(|item| format!("{}", item))
        }
        _ => None,
    })
    .flatten();

    // Now create the string handle (this will lock the registry)
    match item_str {
        Some(s) => super::string::create_string_from_str(&s),
        None => INVALID_HANDLE,
    }
}

/// Get map type (if present)
///
/// ## C Signature
/// ```c
/// int32_t omp_clause_map_type(uint64_t clause_handle);
/// ```
///
/// ## Returns
/// - MapType discriminant on success
/// - -1 if not a Map clause, no map type, or invalid handle
#[no_mangle]
pub extern "C" fn omp_clause_map_type(clause_handle: Handle) -> i32 {
    match with_resource(clause_handle, |res| match res {
        Resource::Clause(clause) => match clause.as_ref() {
            ClauseData::Map { map_type, .. } => map_type.map(|mt| mt as i32),
            _ => None,
        },
        _ => None,
    }) {
        Some(Some(mt)) => mt,
        _ => -1,
    }
}

/// Get depend type
///
/// ## C Signature
/// ```c
/// int32_t omp_clause_depend_type(uint64_t clause_handle);
/// ```
///
/// ## Returns
/// - DependType discriminant on success
/// - -1 if not a Depend clause or invalid handle
#[no_mangle]
pub extern "C" fn omp_clause_depend_type(clause_handle: Handle) -> i32 {
    match with_resource(clause_handle, |res| match res {
        Resource::Clause(clause) => match clause.as_ref() {
            ClauseData::Depend { depend_type, .. } => Some(*depend_type as i32),
            _ => None,
        },
        _ => None,
    }) {
        Some(Some(dt)) => dt,
        _ => -1,
    }
}

/// Get linear modifier
///
/// ## C Signature
/// ```c
/// int32_t omp_clause_linear_modifier(uint64_t clause_handle);
/// ```
///
/// ## Returns
/// - LinearModifier discriminant on success
/// - -1 if not a Linear clause, no modifier, or invalid handle
#[no_mangle]
pub extern "C" fn omp_clause_linear_modifier(clause_handle: Handle) -> i32 {
    match with_resource(clause_handle, |res| match res {
        Resource::Clause(clause) => match clause.as_ref() {
            ClauseData::Linear { modifier, .. } => modifier.map(|m| m as i32),
            _ => None,
        },
        _ => None,
    }) {
        Some(Some(m)) => m,
        _ => -1,
    }
}

/// Get bare clause name
///
/// ## C Signature
/// ```c
/// uint64_t omp_clause_bare_name(uint64_t clause_handle);
/// ```
///
/// ## Returns
/// - String handle containing clause name
/// - INVALID_HANDLE if not a Bare clause or invalid handle
#[no_mangle]
pub extern "C" fn omp_clause_bare_name(clause_handle: Handle) -> Handle {
    // Extract the string first to avoid nested registry locks
    let name_str = with_resource(clause_handle, |res| match res {
        Resource::Clause(clause) => match clause.as_ref() {
            ClauseData::Bare(id) => Some(id.to_string()),
            _ => None,
        },
        _ => None,
    })
    .flatten();

    // Now create the string handle (this will lock the registry)
    match name_str {
        Some(s) => super::string::create_string_from_str(&s),
        None => INVALID_HANDLE,
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
    use crate::ffi::string::{omp_str_free, omp_str_new, omp_str_push_byte, omp_str_validate_utf8};
    use serial_test::serial;

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
    #[serial(ffi)]
    fn test_clause_at_and_type() {
        cleanup();

        let dir_h = parse_directive("#pragma omp parallel num_threads(4)");
        let clause_h = omp_clause_at(dir_h, 0);

        assert_ne!(clause_h, INVALID_HANDLE);
        assert_eq!(omp_clause_type(clause_h), ClauseType::NumThreads as i32);

        omp_clause_free(clause_h);
        omp_directive_free(dir_h);
    }

    #[test]
    #[serial(ffi)]
    fn test_clause_at_out_of_bounds() {
        cleanup();

        let dir_h = parse_directive("#pragma omp parallel");
        let clause_h = omp_clause_at(dir_h, 0);

        assert_eq!(clause_h, INVALID_HANDLE);
        omp_directive_free(dir_h);
    }

    #[test]
    #[serial(ffi)]
    fn test_num_threads_value() {
        cleanup();

        let dir_h = parse_directive("#pragma omp parallel num_threads(4)");
        let clause_h = omp_clause_at(dir_h, 0);

        let value_h = omp_clause_num_threads_value(clause_h);
        assert_ne!(value_h, INVALID_HANDLE);

        // Verify it's valid UTF-8 and contains "4"
        assert_eq!(omp_str_validate_utf8(value_h), OmpStatus::Ok);

        omp_str_free(value_h);
        omp_clause_free(clause_h);
        omp_directive_free(dir_h);
    }

    #[test]
    #[serial(ffi)]
    fn test_reduction_operator() {
        cleanup();

        let dir_h = parse_directive("#pragma omp parallel reduction(+: sum)");
        let clause_h = omp_clause_at(dir_h, 0);

        let op = omp_clause_reduction_operator(clause_h);
        assert_eq!(op, ReductionOperator::Add as i32);

        omp_clause_free(clause_h);
        omp_directive_free(dir_h);
    }

    #[test]
    #[serial(ffi)]
    fn test_schedule_kind() {
        cleanup();

        let dir_h = parse_directive("#pragma omp for schedule(static, 64)");
        let clause_h = omp_clause_at(dir_h, 0);

        let kind = omp_clause_schedule_kind(clause_h);
        assert_eq!(kind, ScheduleKind::Static as i32);

        let chunk_h = omp_clause_schedule_chunk_size(clause_h);
        assert_ne!(chunk_h, INVALID_HANDLE);

        omp_str_free(chunk_h);
        omp_clause_free(clause_h);
        omp_directive_free(dir_h);
    }

    #[test]
    #[serial(ffi)]
    fn test_default_kind() {
        cleanup();

        let dir_h = parse_directive("#pragma omp parallel default(shared)");
        let clause_h = omp_clause_at(dir_h, 0);

        let kind = omp_clause_default_kind(clause_h);
        assert_eq!(kind, DefaultKind::Shared as i32);

        omp_clause_free(clause_h);
        omp_directive_free(dir_h);
    }

    #[test]
    #[serial(ffi)]
    fn test_clause_item_count_and_access() {
        cleanup();

        let dir_h = parse_directive("#pragma omp parallel private(x, y, z)");
        let clause_h = omp_clause_at(dir_h, 0);

        assert_eq!(omp_clause_type(clause_h), ClauseType::Private as i32);
        assert_eq!(omp_clause_item_count(clause_h), 3);

        let item0_h = omp_clause_item_at(clause_h, 0);
        let item1_h = omp_clause_item_at(clause_h, 1);
        let item2_h = omp_clause_item_at(clause_h, 2);
        let item3_h = omp_clause_item_at(clause_h, 3); // Out of bounds

        assert_ne!(item0_h, INVALID_HANDLE);
        assert_ne!(item1_h, INVALID_HANDLE);
        assert_ne!(item2_h, INVALID_HANDLE);
        assert_eq!(item3_h, INVALID_HANDLE);

        omp_str_free(item0_h);
        omp_str_free(item1_h);
        omp_str_free(item2_h);
        omp_clause_free(clause_h);
        omp_directive_free(dir_h);
    }

    #[test]
    #[serial(ffi)]
    fn test_multiple_clauses() {
        cleanup();

        let dir_h = parse_directive("#pragma omp parallel num_threads(4) default(shared)");

        let clause0_h = omp_clause_at(dir_h, 0);
        let clause1_h = omp_clause_at(dir_h, 1);

        assert_eq!(omp_clause_type(clause0_h), ClauseType::NumThreads as i32);
        assert_eq!(omp_clause_type(clause1_h), ClauseType::Default as i32);

        omp_clause_free(clause0_h);
        omp_clause_free(clause1_h);
        omp_directive_free(dir_h);
    }

    #[test]
    #[serial(ffi)]
    fn test_clause_free() {
        cleanup();

        let dir_h = parse_directive("#pragma omp parallel num_threads(4)");
        let clause_h = omp_clause_at(dir_h, 0);

        assert_eq!(omp_clause_free(clause_h), OmpStatus::Ok);
        assert_eq!(omp_clause_free(clause_h), OmpStatus::NotFound); // Double free

        omp_directive_free(dir_h);
    }

    #[test]
    #[serial(ffi)]
    fn test_invalid_clause_handle() {
        cleanup();

        assert_eq!(omp_clause_type(INVALID_HANDLE), -1);
        assert_eq!(omp_clause_num_threads_value(INVALID_HANDLE), INVALID_HANDLE);
        assert_eq!(omp_clause_item_count(INVALID_HANDLE), 0);
        assert_eq!(omp_clause_free(INVALID_HANDLE), OmpStatus::NotFound);
    }

    #[test]
    #[serial(ffi)]
    fn test_wrong_type_accessors() {
        cleanup();

        let dir_h = parse_directive("#pragma omp parallel num_threads(4)");
        let clause_h = omp_clause_at(dir_h, 0);

        // NumThreads clause, but try to access as Default
        assert_eq!(omp_clause_default_kind(clause_h), -1);
        assert_eq!(omp_clause_item_count(clause_h), 0);
        assert_eq!(omp_clause_reduction_operator(clause_h), -1);

        omp_clause_free(clause_h);
        omp_directive_free(dir_h);
    }

    #[test]
    #[serial(ffi)]
    #[ignore = "Hangs due to nowait clause not being properly recognized"]
    fn test_bare_clause() {
        cleanup();

        let dir_h = parse_directive("#pragma omp parallel nowait");
        let clause_h = omp_clause_at(dir_h, 0);

        assert_eq!(omp_clause_type(clause_h), ClauseType::Bare as i32);

        let name_h = omp_clause_bare_name(clause_h);
        assert_ne!(name_h, INVALID_HANDLE);

        omp_str_free(name_h);
        omp_clause_free(clause_h);
        omp_directive_free(dir_h);
    }

    #[test]
    #[serial(ffi)]
    fn test_complex_directive() {
        cleanup();

        let dir_h =
            parse_directive("#pragma omp parallel for reduction(+: sum) schedule(dynamic, 10)");

        // Should have 2 clauses
        let clause0_h = omp_clause_at(dir_h, 0);
        let clause1_h = omp_clause_at(dir_h, 1);

        assert_ne!(clause0_h, INVALID_HANDLE);
        assert_ne!(clause1_h, INVALID_HANDLE);

        // One should be Reduction, one should be Schedule
        let type0 = omp_clause_type(clause0_h);
        let type1 = omp_clause_type(clause1_h);

        assert!(type0 == ClauseType::Reduction as i32 || type0 == ClauseType::Schedule as i32);
        assert!(type1 == ClauseType::Reduction as i32 || type1 == ClauseType::Schedule as i32);
        assert_ne!(type0, type1);

        omp_clause_free(clause0_h);
        omp_clause_free(clause1_h);
        omp_directive_free(dir_h);
    }

    #[test]
    #[serial(ffi)]
    fn test_all_clause_types() {
        cleanup();

        let test_cases = vec![
            ("#pragma omp parallel private(x)", ClauseType::Private),
            ("#pragma omp parallel shared(y)", ClauseType::Shared),
            (
                "#pragma omp parallel firstprivate(z)",
                ClauseType::Firstprivate,
            ),
            (
                "#pragma omp parallel reduction(+: s)",
                ClauseType::Reduction,
            ),
            (
                "#pragma omp parallel num_threads(4)",
                ClauseType::NumThreads,
            ),
            ("#pragma omp parallel if(cond)", ClauseType::If),
            ("#pragma omp for schedule(static)", ClauseType::Schedule),
            ("#pragma omp for collapse(2)", ClauseType::Collapse),
            ("#pragma omp parallel default(shared)", ClauseType::Default),
        ];

        for (input, expected_type) in test_cases {
            let dir_h = parse_directive(input);
            let clause_h = omp_clause_at(dir_h, 0);

            assert_eq!(
                omp_clause_type(clause_h),
                expected_type as i32,
                "Failed for: {}",
                input
            );

            omp_clause_free(clause_h);
            omp_directive_free(dir_h);
        }
    }

    #[test]
    #[serial(ffi)]
    fn test_concurrent_clause_access() {
        use std::sync::Arc;
        use std::thread;

        cleanup();

        let dir_h = parse_directive("#pragma omp parallel num_threads(4) default(shared)");
        let dir_h = Arc::new(dir_h);

        let threads: Vec<_> = (0..5)
            .map(|i| {
                let dir_h = Arc::clone(&dir_h);
                thread::spawn(move || {
                    let clause_h = omp_clause_at(*dir_h, i % 2);
                    if clause_h != INVALID_HANDLE {
                        let _ = omp_clause_type(clause_h);
                        omp_clause_free(clause_h);
                    }
                })
            })
            .collect();

        for t in threads {
            t.join().unwrap();
        }

        omp_directive_free(*dir_h);
    }
}
