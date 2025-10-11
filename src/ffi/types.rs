//! C-compatible types and enums for FFI
//!
//! All types here use `#[repr(C)]` to ensure stable memory layout
//! compatible with the C ABI.

/// Status codes returned by FFI functions
///
/// All FFI functions return a status code to indicate success or failure.
/// This avoids panics crossing the FFI boundary.
///
/// ## Example
///
/// ```
/// # use roup::ffi::OmpStatus;
/// let status = OmpStatus::Ok;
/// assert_eq!(status as u32, 0);
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OmpStatus {
    /// Operation succeeded
    Ok = 0,
    /// Handle not found in registry
    NotFound = 1,
    /// Invalid parameter or state
    Invalid = 2,
    /// Parse error
    ParseError = 3,
    /// Index out of range
    OutOfRange = 4,
    /// Internal error (should not occur)
    Internal = 255,
}

impl OmpStatus {
    /// Check if status indicates success
    pub fn is_ok(self) -> bool {
        matches!(self, OmpStatus::Ok)
    }

    /// Check if status indicates an error
    pub fn is_err(self) -> bool {
        !self.is_ok()
    }
}

/// Opaque handle type for all FFI resources
///
/// Handles are never 0; 0 indicates an invalid/null handle.
pub type Handle = u64;

/// Sentinel value for invalid handles
pub const INVALID_HANDLE: Handle = 0;

/// Check if a handle is valid (non-zero)
#[inline]
pub fn is_valid_handle(h: Handle) -> bool {
    h != INVALID_HANDLE
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_is_ok() {
        assert!(OmpStatus::Ok.is_ok());
        assert!(!OmpStatus::NotFound.is_ok());
        assert!(!OmpStatus::ParseError.is_ok());
    }

    #[test]
    fn test_status_is_err() {
        assert!(!OmpStatus::Ok.is_err());
        assert!(OmpStatus::NotFound.is_err());
        assert!(OmpStatus::Invalid.is_err());
    }

    #[test]
    fn test_status_discriminants() {
        // Ensure discriminants are stable for C ABI
        assert_eq!(OmpStatus::Ok as u32, 0);
        assert_eq!(OmpStatus::NotFound as u32, 1);
        assert_eq!(OmpStatus::Invalid as u32, 2);
        assert_eq!(OmpStatus::ParseError as u32, 3);
        assert_eq!(OmpStatus::OutOfRange as u32, 4);
        assert_eq!(OmpStatus::Internal as u32, 255);
    }

    #[test]
    fn test_invalid_handle() {
        assert_eq!(INVALID_HANDLE, 0);
        assert!(!is_valid_handle(INVALID_HANDLE));
        assert!(is_valid_handle(1));
        assert!(is_valid_handle(u64::MAX));
    }

    #[test]
    fn test_status_size() {
        // Status is repr(C) so size depends on platform C enum size
        // Typically 4 bytes on most platforms
        assert!(std::mem::size_of::<OmpStatus>() <= 8);
    }

    #[test]
    fn test_handle_size() {
        // Handle is exactly 64 bits
        assert_eq!(std::mem::size_of::<Handle>(), 8);
    }
}
