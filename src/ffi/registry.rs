//! Global registry for handle-based resource management
//!
//! This module implements the core of our safe FFI approach: a thread-safe
//! global registry that owns all resources and maps opaque handles to them.
//!
//! ## Learning Objectives
//!
//! 1. **Interior Mutability**: Using `Mutex` to mutate through shared reference
//! 2. **Lazy Static**: Delaying initialization until first use
//! 3. **Handle Allocation**: Generating unique IDs safely
//! 4. **Resource Lifecycle**: Managing creation, lookup, and deletion
//!
//! ## Design Decisions
//!
//! ### Why Mutex instead of RwLock?
//! - Simpler reasoning about lock ordering
//! - Shorter critical sections (just HashMap ops)
//! - Avoids read-write lock complexity
//!
//! ### Why HashMap instead of Slab?
//! - Simpler API (no generation counters needed)
//! - Handle reuse is acceptable (C doesn't expect stability after free)
//! - O(1) lookup is sufficient
//!
//! ### Handle Generation
//! - Wrapping counter starting at 1 (0 is invalid)
//! - No cryptographic guarantees needed
//! - Collisions are acceptable (old handle was freed)

use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::collections::HashMap;

use super::types::{Handle, INVALID_HANDLE};

/// Type-safe wrapper for different resource types
#[derive(Debug)]
pub enum Resource {
    /// UTF-8 byte string (built by C caller)
    String(ByteString),
    /// Parsed AST root
    Ast(Box<crate::ir::DirectiveIR<'static>>),
    /// Individual clause from a directive
    Clause(Box<crate::ir::ClauseData<'static>>),
    /// Cursor for iteration
    Cursor(Cursor),
}

/// UTF-8 byte string built incrementally
#[derive(Debug, Default, Clone)]
pub struct ByteString {
    pub bytes: Vec<u8>,
}

impl ByteString {
    /// Create a new empty byte string
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a single byte
    pub fn push(&mut self, byte: u8) {
        self.bytes.push(byte);
    }

    /// Clear all bytes
    pub fn clear(&mut self) {
        self.bytes.clear();
    }

    /// Get byte at index
    pub fn get(&self, idx: usize) -> Option<u8> {
        self.bytes.get(idx).copied()
    }

    /// Get length
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Attempt to convert to UTF-8 string
    pub fn to_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.bytes)
    }

    /// Convert to owned String, replacing invalid UTF-8
    pub fn to_string_lossy(&self) -> String {
        String::from_utf8_lossy(&self.bytes).into_owned()
    }
}

/// Cursor for iterating over a list of handles
#[derive(Debug, Clone)]
pub struct Cursor {
    /// List of handles to iterate over
    pub items: Vec<Handle>,
    /// Current position
    pub index: usize,
}

impl Cursor {
    /// Create a new cursor
    pub fn new(items: Vec<Handle>) -> Self {
        Self { items, index: 0 }
    }

    /// Get next handle, advancing cursor
    pub fn next(&mut self) -> Option<Handle> {
        if self.index < self.items.len() {
            let item = self.items[self.index];
            self.index += 1;
            Some(item)
        } else {
            None
        }
    }

    /// Check if cursor is exhausted
    pub fn is_done(&self) -> bool {
        self.index >= self.items.len()
    }

    /// Reset cursor to beginning
    pub fn reset(&mut self) {
        self.index = 0;
    }

    /// Get remaining count
    pub fn remaining(&self) -> usize {
        self.items.len().saturating_sub(self.index)
    }
}

/// Global registry of all FFI resources
#[derive(Debug, Default)]
pub struct Registry {
    /// Next handle to allocate (wrapping counter)
    next_handle: u64,
    /// All resources indexed by handle
    resources: HashMap<Handle, Resource>,
}

impl Registry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocate a new unique handle
    ///
    /// Handles start at 1 and wrap around. Handle 0 is reserved as invalid.
    fn allocate_handle(&mut self) -> Handle {
        loop {
            self.next_handle = self.next_handle.wrapping_add(1);
            if self.next_handle != INVALID_HANDLE {
                // Check for collision (unlikely but possible after wraparound)
                if !self.resources.contains_key(&self.next_handle) {
                    return self.next_handle;
                }
            }
        }
    }

    /// Insert a resource and return its handle
    pub fn insert(&mut self, resource: Resource) -> Handle {
        let handle = self.allocate_handle();
        self.resources.insert(handle, resource);
        handle
    }

    /// Get a reference to a resource
    pub fn get(&self, handle: Handle) -> Option<&Resource> {
        self.resources.get(&handle)
    }

    /// Get a mutable reference to a resource
    pub fn get_mut(&mut self, handle: Handle) -> Option<&mut Resource> {
        self.resources.get_mut(&handle)
    }

    /// Remove a resource and return it
    pub fn remove(&mut self, handle: Handle) -> Option<Resource> {
        self.resources.remove(&handle)
    }

    /// Check if a handle exists
    pub fn contains(&self, handle: Handle) -> bool {
        self.resources.contains_key(&handle)
    }

    /// Get the number of resources
    pub fn len(&self) -> usize {
        self.resources.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.resources.is_empty()
    }

    /// Clear all resources (for testing)
    #[cfg(test)]
    pub fn clear(&mut self) {
        self.resources.clear();
    }
}

/// Global registry instance
///
/// This is the single source of truth for all FFI resources.
/// Access is protected by a Mutex for thread safety.
pub static REGISTRY: Lazy<Mutex<Registry>> = Lazy::new(|| Mutex::new(Registry::new()));

/// Insert a resource into the global registry
pub fn insert(resource: Resource) -> Handle {
    REGISTRY.lock().insert(resource)
}

/// Get a resource from the global registry
///
/// Returns None if handle is invalid or not found.
pub fn get(handle: Handle) -> Option<Resource> {
    // We need to clone to avoid holding the lock
    // This is safe because we clone the resource data
    let reg = REGISTRY.lock();
    match reg.get(handle)? {
        Resource::String(s) => Some(Resource::String(s.clone())),
        Resource::Cursor(c) => Some(Resource::Cursor(c.clone())),
        // AST and Clause can't be easily cloned, so we can't return them by value
        // Callers should use with_resource for AST/Clause access
        Resource::Ast(_) => None,
        Resource::Clause(_) => None,
    }
}

/// Execute a function with access to a resource
///
/// This avoids cloning non-clonable resources like AST.
pub fn with_resource<F, R>(handle: Handle, f: F) -> Option<R>
where
    F: FnOnce(&Resource) -> R,
{
    let reg = REGISTRY.lock();
    reg.get(handle).map(f)
}

/// Execute a function with mutable access to a resource
pub fn with_resource_mut<F, R>(handle: Handle, f: F) -> Option<R>
where
    F: FnOnce(&mut Resource) -> R,
{
    let mut reg = REGISTRY.lock();
    reg.get_mut(handle).map(f)
}

/// Remove a resource from the global registry
pub fn remove(handle: Handle) -> Option<Resource> {
    REGISTRY.lock().remove(handle)
}

/// Check if a handle exists in the global registry
pub fn contains(handle: Handle) -> bool {
    REGISTRY.lock().contains(handle)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_string_new() {
        let s = ByteString::new();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn test_byte_string_push() {
        let mut s = ByteString::new();
        s.push(b'a');
        s.push(b'b');
        s.push(b'c');
        assert_eq!(s.len(), 3);
        assert_eq!(s.bytes, b"abc");
    }

    #[test]
    fn test_byte_string_get() {
        let mut s = ByteString::new();
        s.push(b'x');
        s.push(b'y');
        assert_eq!(s.get(0), Some(b'x'));
        assert_eq!(s.get(1), Some(b'y'));
        assert_eq!(s.get(2), None);
    }

    #[test]
    fn test_byte_string_clear() {
        let mut s = ByteString::new();
        s.push(b'a');
        s.clear();
        assert!(s.is_empty());
    }

    #[test]
    fn test_byte_string_to_str_valid() {
        let mut s = ByteString::new();
        for &b in b"hello" {
            s.push(b);
        }
        assert_eq!(s.to_str().unwrap(), "hello");
    }

    #[test]
    fn test_byte_string_to_str_invalid() {
        let mut s = ByteString::new();
        s.push(0xFF); // Invalid UTF-8
        assert!(s.to_str().is_err());
    }

    #[test]
    fn test_byte_string_to_string_lossy() {
        let mut s = ByteString::new();
        s.push(b'H');
        s.push(0xFF); // Invalid UTF-8
        s.push(b'i');
        let lossy = s.to_string_lossy();
        assert!(lossy.contains('H'));
        assert!(lossy.contains('i'));
    }

    #[test]
    fn test_cursor_new() {
        let cursor = Cursor::new(vec![1, 2, 3]);
        assert_eq!(cursor.items, vec![1, 2, 3]);
        assert_eq!(cursor.index, 0);
        assert_eq!(cursor.remaining(), 3);
    }

    #[test]
    fn test_cursor_next() {
        let mut cursor = Cursor::new(vec![10, 20, 30]);
        assert_eq!(cursor.next(), Some(10));
        assert_eq!(cursor.next(), Some(20));
        assert_eq!(cursor.next(), Some(30));
        assert_eq!(cursor.next(), None);
    }

    #[test]
    fn test_cursor_is_done() {
        let mut cursor = Cursor::new(vec![1, 2]);
        assert!(!cursor.is_done());
        cursor.next();
        assert!(!cursor.is_done());
        cursor.next();
        assert!(cursor.is_done());
    }

    #[test]
    fn test_cursor_reset() {
        let mut cursor = Cursor::new(vec![1, 2, 3]);
        cursor.next();
        cursor.next();
        cursor.reset();
        assert_eq!(cursor.index, 0);
        assert_eq!(cursor.next(), Some(1));
    }

    #[test]
    fn test_cursor_remaining() {
        let mut cursor = Cursor::new(vec![1, 2, 3, 4]);
        assert_eq!(cursor.remaining(), 4);
        cursor.next();
        assert_eq!(cursor.remaining(), 3);
        cursor.next();
        cursor.next();
        assert_eq!(cursor.remaining(), 1);
        cursor.next();
        assert_eq!(cursor.remaining(), 0);
    }

    #[test]
    fn test_registry_new() {
        let reg = Registry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }

    #[test]
    fn test_registry_allocate_handle() {
        let mut reg = Registry::new();
        let h1 = reg.allocate_handle();
        let h2 = reg.allocate_handle();
        assert_ne!(h1, INVALID_HANDLE);
        assert_ne!(h2, INVALID_HANDLE);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_registry_insert_and_get() {
        let mut reg = Registry::new();
        let s = ByteString::from_bytes(b"test");
        let handle = reg.insert(Resource::String(s));

        assert!(reg.contains(handle));
        assert_eq!(reg.len(), 1);

        match reg.get(handle) {
            Some(Resource::String(s)) => assert_eq!(s.to_str().unwrap(), "test"),
            _ => panic!("Expected string resource"),
        }
    }

    #[test]
    fn test_registry_get_mut() {
        let mut reg = Registry::new();
        let s = ByteString::new();
        let handle = reg.insert(Resource::String(s));

        if let Some(Resource::String(s)) = reg.get_mut(handle) {
            s.push(b'X');
        }

        match reg.get(handle) {
            Some(Resource::String(s)) => assert_eq!(s.bytes, b"X"),
            _ => panic!("Expected string resource"),
        }
    }

    #[test]
    fn test_registry_remove() {
        let mut reg = Registry::new();
        let s = ByteString::from_bytes(b"temp");
        let handle = reg.insert(Resource::String(s));

        assert!(reg.contains(handle));
        let removed = reg.remove(handle);
        assert!(removed.is_some());
        assert!(!reg.contains(handle));
        assert!(reg.is_empty());
    }

    #[test]
    fn test_registry_remove_nonexistent() {
        let mut reg = Registry::new();
        let removed = reg.remove(999);
        assert!(removed.is_none());
    }

    #[test]
    fn test_registry_multiple_resources() {
        let mut reg = Registry::new();
        let h1 = reg.insert(Resource::String(ByteString::from_bytes(b"one")));
        let h2 = reg.insert(Resource::String(ByteString::from_bytes(b"two")));
        let h3 = reg.insert(Resource::Cursor(Cursor::new(vec![1, 2, 3])));

        assert_eq!(reg.len(), 3);
        assert!(reg.contains(h1));
        assert!(reg.contains(h2));
        assert!(reg.contains(h3));
    }

    #[test]
    fn test_global_registry_insert() {
        // Clean up first
        REGISTRY.lock().clear();

        let handle = insert(Resource::String(ByteString::from_bytes(b"global")));
        assert_ne!(handle, INVALID_HANDLE);
        assert!(contains(handle));

        // Cleanup
        remove(handle);
    }

    #[test]
    fn test_global_registry_with_resource() {
        REGISTRY.lock().clear();

        let handle = insert(Resource::String(ByteString::from_bytes(b"test")));

        let result = with_resource(handle, |res| match res {
            Resource::String(s) => s.to_str().unwrap().to_string(),
            _ => panic!("Expected string"),
        });

        assert_eq!(result, Some("test".to_string()));

        // Cleanup
        remove(handle);
    }

    #[test]
    fn test_global_registry_with_resource_mut() {
        REGISTRY.lock().clear();

        let handle = insert(Resource::String(ByteString::new()));

        with_resource_mut(handle, |res| {
            if let Resource::String(s) = res {
                s.push(b'A');
                s.push(b'B');
            }
        });

        let result = with_resource(handle, |res| match res {
            Resource::String(s) => s.to_str().unwrap().to_string(),
            _ => panic!("Expected string"),
        });

        assert_eq!(result, Some("AB".to_string()));

        // Cleanup
        remove(handle);
    }

    #[test]
    fn test_global_registry_remove() {
        REGISTRY.lock().clear();

        let handle = insert(Resource::String(ByteString::from_bytes(b"temp")));
        assert!(contains(handle));

        let removed = remove(handle);
        assert!(removed.is_some());
        assert!(!contains(handle));
    }

    #[test]
    fn test_handle_wraparound() {
        let mut reg = Registry::new();
        reg.next_handle = u64::MAX - 1;

        let h1 = reg.allocate_handle();
        let h2 = reg.allocate_handle(); // Should wrap to 1
        let h3 = reg.allocate_handle();

        assert_ne!(h1, INVALID_HANDLE);
        assert_ne!(h2, INVALID_HANDLE);
        assert_ne!(h3, INVALID_HANDLE);
        assert_ne!(h1, h2);
        assert_ne!(h2, h3);
    }

    #[test]
    fn test_concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        REGISTRY.lock().clear();

        let handles = Arc::new(Mutex::new(Vec::new()));

        let threads: Vec<_> = (0..10)
            .map(|i| {
                let handles = Arc::clone(&handles);
                thread::spawn(move || {
                    let handle = insert(Resource::String(ByteString::from_bytes(
                        format!("thread-{}", i).as_bytes(),
                    )));
                    handles.lock().push(handle);
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

        // Cleanup
        for &h in handles.iter() {
            remove(h);
        }
    }

    #[test]
    fn test_double_free_prevention() {
        REGISTRY.lock().clear();

        let handle = insert(Resource::String(ByteString::from_bytes(b"once")));
        let first = remove(handle);
        let second = remove(handle);

        assert!(first.is_some());
        assert!(second.is_none());
    }

    #[test]
    fn test_invalid_handle_operations() {
        assert!(!contains(INVALID_HANDLE));
        assert!(with_resource(INVALID_HANDLE, |_| ()).is_none());
        assert!(with_resource_mut(INVALID_HANDLE, |_| ()).is_none());
        assert!(remove(INVALID_HANDLE).is_none());
    }
}

// Helper for tests
impl ByteString {
    #[cfg(test)]
    fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            bytes: bytes.to_vec(),
        }
    }
}
