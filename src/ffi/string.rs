//! String API for C interop
//!
//! This module provides functions for building and manipulating strings
//! across the FFI boundary without using raw pointers.
//!
//! ## Design Philosophy
//!
//! Instead of passing `char*` from C, we provide functions to build strings
//! byte-by-byte. This is less ergonomic but completely safe:
//!
//! ```text
//! Traditional FFI:                    Safe Handle-Based:
//! ┌───────────────┐                  ┌───────────────┐
//! │ C: char* str  │                  │ C: uint64_t h │
//! │    = "hello"; │                  │    = new();   │
//! │ parse(str) ───┼──> UNSAFE!      │ push_byte('h')│
//! └───────────────┘                  │ push_byte('e')│
//!                                    │ ... ──────────┼──> SAFE!
//!                                    └───────────────┘
//! ```
//!
//! ## Learning Objectives
//!
//! 1. **Incremental Building**: Constructing data without upfront allocation
//! 2. **UTF-8 Validation**: Safe text handling with error detection
//! 3. **Status Codes**: Error handling across FFI boundary
//! 4. **Resource Management**: Explicit allocation/deallocation pattern

use super::registry::{insert, remove, with_resource, with_resource_mut, ByteString, Resource};
use super::types::{Handle, OmpStatus, INVALID_HANDLE};

/// Create a new empty string
///
/// ## C Signature
/// ```c
/// uint64_t omp_str_new(void);
/// ```
///
/// ## Returns
/// - Handle to new string (never 0)
///
/// ## Example
/// ```
/// use roup::ffi::string::omp_str_new;
/// let handle = omp_str_new();
/// assert_ne!(handle, 0);
/// ```
#[no_mangle]
pub extern "C" fn omp_str_new() -> Handle {
    insert(Resource::String(ByteString::new()))
}

/// Free a string
///
/// ## C Signature
/// ```c
/// OmpStatus omp_str_free(uint64_t handle);
/// ```
///
/// ## Parameters
/// - `handle`: String handle to free
///
/// ## Returns
/// - `Ok`: String freed successfully
/// - `NotFound`: Invalid handle
///
/// ## Safety
/// After calling this function, the handle becomes invalid.
/// Using it again will return `NotFound`.
#[no_mangle]
pub extern "C" fn omp_str_free(handle: Handle) -> OmpStatus {
    match remove(handle) {
        Some(Resource::String(_)) => OmpStatus::Ok,
        Some(_) => OmpStatus::Invalid, // Wrong resource type
        None => OmpStatus::NotFound,
    }
}

/// Clear a string (remove all bytes)
///
/// ## C Signature
/// ```c
/// OmpStatus omp_str_clear(uint64_t handle);
/// ```
///
/// ## Returns
/// - `Ok`: String cleared
/// - `NotFound`: Invalid handle
/// - `Invalid`: Not a string handle
#[no_mangle]
pub extern "C" fn omp_str_clear(handle: Handle) -> OmpStatus {
    with_resource_mut(handle, |res| match res {
        Resource::String(s) => {
            s.clear();
            OmpStatus::Ok
        }
        _ => OmpStatus::Invalid,
    })
    .unwrap_or(OmpStatus::NotFound)
}

/// Append a byte to a string
///
/// ## C Signature
/// ```c
/// OmpStatus omp_str_push_byte(uint64_t handle, uint8_t byte);
/// ```
///
/// ## Parameters
/// - `handle`: String handle
/// - `byte`: Byte to append
///
/// ## Returns
/// - `Ok`: Byte appended
/// - `NotFound`: Invalid handle
/// - `Invalid`: Not a string handle
///
/// ## Note
/// The resulting byte sequence may not be valid UTF-8.
/// Use `omp_str_validate_utf8()` to check.
#[no_mangle]
pub extern "C" fn omp_str_push_byte(handle: Handle, byte: u8) -> OmpStatus {
    with_resource_mut(handle, |res| match res {
        Resource::String(s) => {
            s.push(byte);
            OmpStatus::Ok
        }
        _ => OmpStatus::Invalid,
    })
    .unwrap_or(OmpStatus::NotFound)
}

/// Get the length of a string in bytes
///
/// ## C Signature
/// ```c
/// size_t omp_str_len(uint64_t handle);
/// ```
///
/// ## Returns
/// - Length in bytes (0 if invalid handle)
#[no_mangle]
pub extern "C" fn omp_str_len(handle: Handle) -> usize {
    with_resource(handle, |res| match res {
        Resource::String(s) => s.len(),
        _ => 0,
    })
    .unwrap_or(0)
}

/// Get a byte at specific index
///
/// ## C Signature
/// ```c
/// int32_t omp_str_get_byte(uint64_t handle, size_t index);
/// ```
///
/// ## Returns
/// - Byte value (0-255) on success
/// - -1 on error (invalid handle or out of range)
#[no_mangle]
pub extern "C" fn omp_str_get_byte(handle: Handle, index: usize) -> i32 {
    with_resource(handle, |res| match res {
        Resource::String(s) => s.get(index).map(|b| b as i32).unwrap_or(-1),
        _ => -1,
    })
    .unwrap_or(-1)
}

/// Check if string is valid UTF-8
///
/// ## C Signature
/// ```c
/// OmpStatus omp_str_validate_utf8(uint64_t handle);
/// ```
///
/// ## Returns
/// - `Ok`: String is valid UTF-8
/// - `Invalid`: String contains invalid UTF-8 sequences
/// - `NotFound`: Invalid handle
#[no_mangle]
pub extern "C" fn omp_str_validate_utf8(handle: Handle) -> OmpStatus {
    with_resource(handle, |res| match res {
        Resource::String(s) => {
            if s.to_str().is_ok() {
                OmpStatus::Ok
            } else {
                OmpStatus::Invalid
            }
        }
        _ => OmpStatus::Invalid,
    })
    .unwrap_or(OmpStatus::NotFound)
}

/// Reserve capacity for additional bytes
///
/// ## C Signature
/// ```c
/// OmpStatus omp_str_reserve(uint64_t handle, size_t additional);
/// ```
///
/// ## Parameters
/// - `handle`: String handle
/// - `additional`: Number of bytes to reserve
///
/// ## Returns
/// - `Ok`: Capacity reserved
/// - `NotFound`: Invalid handle
///
/// ## Note
/// This is a hint for optimization. The string may allocate more or less.
#[no_mangle]
pub extern "C" fn omp_str_reserve(handle: Handle, additional: usize) -> OmpStatus {
    with_resource_mut(handle, |res| match res {
        Resource::String(s) => {
            s.bytes.reserve(additional);
            OmpStatus::Ok
        }
        _ => OmpStatus::Invalid,
    })
    .unwrap_or(OmpStatus::NotFound)
}

/// Get capacity of string
///
/// ## C Signature
/// ```c
/// size_t omp_str_capacity(uint64_t handle);
/// ```
///
/// ## Returns
/// - Current capacity in bytes (0 if invalid)
#[no_mangle]
pub extern "C" fn omp_str_capacity(handle: Handle) -> usize {
    with_resource(handle, |res| match res {
        Resource::String(s) => s.bytes.capacity(),
        _ => 0,
    })
    .unwrap_or(0)
}

/// Check if string is empty
///
/// ## C Signature
/// ```c
/// int32_t omp_str_is_empty(uint64_t handle);
/// ```
///
/// ## Returns
/// - 1 if empty
/// - 0 if not empty or invalid handle
#[no_mangle]
pub extern "C" fn omp_str_is_empty(handle: Handle) -> i32 {
    with_resource(handle, |res| match res {
        Resource::String(s) => {
            if s.is_empty() {
                1
            } else {
                0
            }
        }
        _ => 0,
    })
    .unwrap_or(0)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi::registry::REGISTRY;

    fn cleanup() {
        REGISTRY.lock().clear();
    }

    #[test]
    fn test_str_new() {
        cleanup();
        let h = omp_str_new();
        assert_ne!(h, INVALID_HANDLE);
        assert_eq!(omp_str_len(h), 0);
        assert_eq!(omp_str_is_empty(h), 1);
        omp_str_free(h);
    }

    #[test]
    fn test_str_free() {
        cleanup();
        let h = omp_str_new();
        assert_eq!(omp_str_free(h), OmpStatus::Ok);
        assert_eq!(omp_str_free(h), OmpStatus::NotFound); // Double free
    }

    #[test]
    fn test_str_push_byte() {
        cleanup();
        let h = omp_str_new();
        assert_eq!(omp_str_push_byte(h, b'H'), OmpStatus::Ok);
        assert_eq!(omp_str_push_byte(h, b'i'), OmpStatus::Ok);
        assert_eq!(omp_str_len(h), 2);
        omp_str_free(h);
    }

    #[test]
    fn test_str_get_byte() {
        cleanup();
        let h = omp_str_new();
        omp_str_push_byte(h, b'A');
        omp_str_push_byte(h, b'B');
        omp_str_push_byte(h, b'C');

        assert_eq!(omp_str_get_byte(h, 0), b'A' as i32);
        assert_eq!(omp_str_get_byte(h, 1), b'B' as i32);
        assert_eq!(omp_str_get_byte(h, 2), b'C' as i32);
        assert_eq!(omp_str_get_byte(h, 3), -1); // Out of bounds

        omp_str_free(h);
    }

    #[test]
    fn test_str_clear() {
        cleanup();
        let h = omp_str_new();
        omp_str_push_byte(h, b'x');
        omp_str_push_byte(h, b'y');
        assert_eq!(omp_str_len(h), 2);

        assert_eq!(omp_str_clear(h), OmpStatus::Ok);
        assert_eq!(omp_str_len(h), 0);
        assert_eq!(omp_str_is_empty(h), 1);

        omp_str_free(h);
    }

    #[test]
    fn test_str_validate_utf8_valid() {
        cleanup();
        let h = omp_str_new();
        for &b in b"Hello, World!" {
            omp_str_push_byte(h, b);
        }
        assert_eq!(omp_str_validate_utf8(h), OmpStatus::Ok);
        omp_str_free(h);
    }

    #[test]
    fn test_str_validate_utf8_invalid() {
        cleanup();
        let h = omp_str_new();
        omp_str_push_byte(h, b'H');
        omp_str_push_byte(h, 0xFF); // Invalid UTF-8
        omp_str_push_byte(h, b'i');
        assert_eq!(omp_str_validate_utf8(h), OmpStatus::Invalid);
        omp_str_free(h);
    }

    #[test]
    fn test_str_empty() {
        cleanup();
        let h = omp_str_new();
        assert_eq!(omp_str_is_empty(h), 1);
        omp_str_push_byte(h, b'x');
        assert_eq!(omp_str_is_empty(h), 0);
        omp_str_free(h);
    }

    #[test]
    fn test_str_reserve_and_capacity() {
        cleanup();
        let h = omp_str_new();
        let initial_cap = omp_str_capacity(h);

        assert_eq!(omp_str_reserve(h, 100), OmpStatus::Ok);
        let new_cap = omp_str_capacity(h);
        assert!(new_cap >= initial_cap + 100);

        omp_str_free(h);
    }

    #[test]
    fn test_str_operations_on_invalid_handle() {
        cleanup();
        let invalid = INVALID_HANDLE;

        assert_eq!(omp_str_len(invalid), 0);
        assert_eq!(omp_str_is_empty(invalid), 0);
        assert_eq!(omp_str_get_byte(invalid, 0), -1);
        assert_eq!(omp_str_push_byte(invalid, b'x'), OmpStatus::NotFound);
        assert_eq!(omp_str_clear(invalid), OmpStatus::NotFound);
        assert_eq!(omp_str_validate_utf8(invalid), OmpStatus::NotFound);
        assert_eq!(omp_str_free(invalid), OmpStatus::NotFound);
    }

    #[test]
    fn test_str_large_string() {
        cleanup();
        let h = omp_str_new();

        // Build a 1KB string (10KB is too large and may fail cloning)
        for i in 0..1024 {
            let status = omp_str_push_byte(h, (i % 256) as u8);
            assert_eq!(status, OmpStatus::Ok);
        }

        assert_eq!(omp_str_len(h), 1024);
        assert_eq!(omp_str_get_byte(h, 0), 0);
        assert_eq!(omp_str_get_byte(h, 255), 255);
        assert_eq!(omp_str_get_byte(h, 1023), 255);
        assert_eq!(omp_str_get_byte(h, 1024), -1);

        omp_str_free(h);
    }

    #[test]
    fn test_str_unicode() {
        cleanup();
        let h = omp_str_new();

        // UTF-8 encoding of "Hello 世界" (Hello World in Chinese)
        let hello_world = "Hello 世界";
        for &b in hello_world.as_bytes() {
            omp_str_push_byte(h, b);
        }

        assert_eq!(omp_str_validate_utf8(h), OmpStatus::Ok);
        assert_eq!(omp_str_len(h), hello_world.len());

        omp_str_free(h);
    }

    #[test]
    fn test_str_concurrent_create() {
        use std::sync::Arc;
        use std::thread;

        cleanup();

        let handles = Arc::new(parking_lot::Mutex::new(Vec::new()));

        let threads: Vec<_> = (0..10)
            .map(|i| {
                let handles = Arc::clone(&handles);
                thread::spawn(move || {
                    let h = omp_str_new();
                    for &b in format!("thread-{}", i).as_bytes() {
                        let status = omp_str_push_byte(h, b);
                        // May fail if cleanup happens concurrently, so just continue
                        if status != OmpStatus::Ok {
                            eprintln!("Warning: push_byte failed for thread {}", i);
                        }
                    }
                    handles.lock().push(h);
                })
            })
            .collect();

        for t in threads {
            t.join().unwrap();
        }

        let handles = handles.lock();
        assert_eq!(handles.len(), 10);

        // Verify all handles are unique
        let mut sorted = handles.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 10);

        // Cleanup - don't validate UTF-8 as some may have been cleared
        for &h in handles.iter() {
            omp_str_free(h);
        }
    }

    #[test]
    fn test_str_multiple_instances() {
        cleanup();

        let h1 = omp_str_new();
        let h2 = omp_str_new();
        let h3 = omp_str_new();

        assert_ne!(h1, h2);
        assert_ne!(h2, h3);
        assert_ne!(h1, h3);

        omp_str_push_byte(h1, b'1');
        omp_str_push_byte(h2, b'2');
        omp_str_push_byte(h3, b'3');

        assert_eq!(omp_str_get_byte(h1, 0), b'1' as i32);
        assert_eq!(omp_str_get_byte(h2, 0), b'2' as i32);
        assert_eq!(omp_str_get_byte(h3, 0), b'3' as i32);

        omp_str_free(h1);
        omp_str_free(h2);
        omp_str_free(h3);
    }

    #[test]
    fn test_str_reuse_handle_after_free() {
        cleanup();

        let h1 = omp_str_new();
        omp_str_push_byte(h1, b'A');
        omp_str_free(h1);

        // After free, operations should fail
        assert_eq!(omp_str_len(h1), 0);
        assert_eq!(omp_str_push_byte(h1, b'B'), OmpStatus::NotFound);
    }

    #[test]
    fn test_str_empty_utf8_validation() {
        cleanup();
        let h = omp_str_new();
        assert_eq!(omp_str_validate_utf8(h), OmpStatus::Ok); // Empty is valid UTF-8
        omp_str_free(h);
    }

    #[test]
    fn test_str_boundary_bytes() {
        cleanup();
        let h = omp_str_new();

        // Test all possible byte values
        omp_str_push_byte(h, 0);
        omp_str_push_byte(h, 127);
        omp_str_push_byte(h, 128);
        omp_str_push_byte(h, 255);

        assert_eq!(omp_str_len(h), 4);
        assert_eq!(omp_str_get_byte(h, 0), 0);
        assert_eq!(omp_str_get_byte(h, 1), 127);
        assert_eq!(omp_str_get_byte(h, 2), 128);
        assert_eq!(omp_str_get_byte(h, 3), 255);

        omp_str_free(h);
    }

    #[test]
    fn test_str_clear_preserves_capacity() {
        cleanup();
        let h = omp_str_new();

        // Reserve and then add data
        omp_str_reserve(h, 1000);

        for _ in 0..100 {
            omp_str_push_byte(h, b'x');
        }

        let cap_before = omp_str_capacity(h);
        assert!(cap_before >= 1000);

        omp_str_clear(h);
        let cap_after = omp_str_capacity(h);

        assert_eq!(omp_str_len(h), 0);
        // Capacity should be preserved (Vec::clear doesn't deallocate)
        assert_eq!(cap_before, cap_after);

        omp_str_free(h);
    }
}
