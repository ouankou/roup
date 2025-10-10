//! Clause semantic types for OpenMP IR
//!
//! This module defines types for representing OpenMP clause semantics.
//! It captures the meaning of clauses, not just their syntax.
//!
//! ## Learning Objectives
//!
//! - **Large enums**: Modeling many alternatives with discriminated unions
//! - **Semantic modeling**: Capturing intent, not just tokens
//! - **FFI readiness**: Using `repr(C)` with explicit discriminants
//! - **Exhaustive matching**: Compiler ensures all cases handled
//!
//! ## Design Philosophy
//!
//! OpenMP has many clause modifiers that affect behavior:
//! - Reduction operators: `+`, `*`, `max`, `min`, etc.
//! - Map types: `to`, `from`, `tofrom`, `alloc`, etc.
//! - Schedule kinds: `static`, `dynamic`, `guided`, `auto`
//! - Depend types: `in`, `out`, `inout`, `mutexinoutset`
//!
//! Each modifier is represented as a Rust enum with:
//! 1. Clear variant names (e.g., `ReductionOperator::Add`)
//! 2. FFI-compatible layout (`repr(C)`)
//! 3. Explicit discriminant values for C interop
//! 4. Display trait for pretty-printing
//!
//! ## Corner Cases Handled
//!
//! - Unknown/custom operators via `Custom` variants
//! - Language-specific operators (C++ vs Fortran)
//! - OpenMP version-specific features
//! - User-defined reduction operators

use std::fmt;

// ============================================================================
// Reduction Operators (OpenMP 5.2 spec section 5.5.5)
// ============================================================================

/// Reduction operator for reduction clauses
///
/// OpenMP supports built-in reduction operators and user-defined operators.
/// This enum covers the standard operators defined in the OpenMP specification.
///
/// ## Examples
///
/// ```
/// # use roup::ir::ReductionOperator;
/// let op = ReductionOperator::Add;
/// assert_eq!(op.to_string(), "+");
///
/// let op = ReductionOperator::Max;
/// assert_eq!(op.to_string(), "max");
/// ```
///
/// ## Learning: FFI-Compatible Enums
///
/// The `repr(C)` attribute ensures this enum has the same memory layout
/// as a C enum, making it safe to pass across FFI boundaries.
/// Explicit discriminants ensure stable values across versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub enum ReductionOperator {
    // Arithmetic operators
    Add = 0,      // +
    Multiply = 1, // *
    Subtract = 2, // -

    // Bitwise operators
    BitwiseAnd = 10, // &
    BitwiseOr = 11,  // |
    BitwiseXor = 12, // ^

    // Logical operators
    LogicalAnd = 20, // &&
    LogicalOr = 21,  // ||

    // Min/Max operators
    Min = 30,
    Max = 31,

    // C++ specific operators (OpenMP 5.2 supports these)
    MinusEqual = 40, // -= (non-commutative)

    // User-defined reduction operator
    Custom = 100,
}

impl fmt::Display for ReductionOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReductionOperator::Add => write!(f, "+"),
            ReductionOperator::Multiply => write!(f, "*"),
            ReductionOperator::Subtract => write!(f, "-"),
            ReductionOperator::BitwiseAnd => write!(f, "&"),
            ReductionOperator::BitwiseOr => write!(f, "|"),
            ReductionOperator::BitwiseXor => write!(f, "^"),
            ReductionOperator::LogicalAnd => write!(f, "&&"),
            ReductionOperator::LogicalOr => write!(f, "||"),
            ReductionOperator::Min => write!(f, "min"),
            ReductionOperator::Max => write!(f, "max"),
            ReductionOperator::MinusEqual => write!(f, "-="),
            ReductionOperator::Custom => write!(f, "custom"),
        }
    }
}

// ============================================================================
// Map Type (OpenMP 5.2 spec section 5.8.3)
// ============================================================================

/// Map type for map clauses in target directives
///
/// Specifies how data is mapped between host and device memory.
///
/// ## Examples
///
/// ```
/// # use roup::ir::MapType;
/// let mt = MapType::To;
/// assert_eq!(mt.to_string(), "to");
///
/// let mt = MapType::ToFrom;
/// assert_eq!(mt.to_string(), "tofrom");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub enum MapType {
    /// Map data to device (host → device)
    To = 0,
    /// Map data from device (device → host)
    From = 1,
    /// Map data to and from device (bidirectional)
    ToFrom = 2,
    /// Allocate device memory without transfer
    Alloc = 3,
    /// Release device memory
    Release = 4,
    /// Delete device memory
    Delete = 5,
}

impl fmt::Display for MapType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MapType::To => write!(f, "to"),
            MapType::From => write!(f, "from"),
            MapType::ToFrom => write!(f, "tofrom"),
            MapType::Alloc => write!(f, "alloc"),
            MapType::Release => write!(f, "release"),
            MapType::Delete => write!(f, "delete"),
        }
    }
}

// ============================================================================
// Schedule Kind (OpenMP 5.2 spec section 2.9.2)
// ============================================================================

/// Schedule kind for loop scheduling
///
/// Determines how loop iterations are distributed among threads.
///
/// ## Examples
///
/// ```
/// # use roup::ir::ScheduleKind;
/// let sk = ScheduleKind::Static;
/// assert_eq!(sk.to_string(), "static");
///
/// let sk = ScheduleKind::Dynamic;
/// assert_eq!(sk.to_string(), "dynamic");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub enum ScheduleKind {
    /// Iterations divided into chunks of specified size, assigned statically
    Static = 0,
    /// Iterations divided into chunks, assigned dynamically at runtime
    Dynamic = 1,
    /// Similar to dynamic but chunk size decreases exponentially
    Guided = 2,
    /// Implementation-defined scheduling
    Auto = 3,
    /// Runtime determines schedule via environment variable
    Runtime = 4,
}

impl fmt::Display for ScheduleKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScheduleKind::Static => write!(f, "static"),
            ScheduleKind::Dynamic => write!(f, "dynamic"),
            ScheduleKind::Guided => write!(f, "guided"),
            ScheduleKind::Auto => write!(f, "auto"),
            ScheduleKind::Runtime => write!(f, "runtime"),
        }
    }
}

// ============================================================================
// Schedule Modifier (OpenMP 5.2 spec section 2.9.2)
// ============================================================================

/// Schedule modifier for schedule clause
///
/// Modifiers that affect how the schedule is applied.
///
/// ## Examples
///
/// ```
/// # use roup::ir::ScheduleModifier;
/// let sm = ScheduleModifier::Monotonic;
/// assert_eq!(sm.to_string(), "monotonic");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub enum ScheduleModifier {
    /// Iterations assigned in monotonically increasing order
    Monotonic = 0,
    /// No ordering guarantee (allows optimizations)
    Nonmonotonic = 1,
    /// SIMD execution of iterations
    Simd = 2,
}

impl fmt::Display for ScheduleModifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScheduleModifier::Monotonic => write!(f, "monotonic"),
            ScheduleModifier::Nonmonotonic => write!(f, "nonmonotonic"),
            ScheduleModifier::Simd => write!(f, "simd"),
        }
    }
}

// ============================================================================
// Depend Type (OpenMP 5.2 spec section 2.17.11)
// ============================================================================

/// Dependence type for task dependencies
///
/// Specifies the type of data dependency between tasks.
///
/// ## Examples
///
/// ```
/// # use roup::ir::DependType;
/// let dt = DependType::In;
/// assert_eq!(dt.to_string(), "in");
///
/// let dt = DependType::Inout;
/// assert_eq!(dt.to_string(), "inout");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub enum DependType {
    /// Read dependency
    In = 0,
    /// Write dependency
    Out = 1,
    /// Read-write dependency
    Inout = 2,
    /// Mutual exclusion with inout
    Mutexinoutset = 3,
    /// Dependency on task completion
    Depobj = 4,
    /// Source dependency (OpenMP 5.0)
    Source = 5,
    /// Sink dependency (OpenMP 5.0)
    Sink = 6,
}

impl fmt::Display for DependType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DependType::In => write!(f, "in"),
            DependType::Out => write!(f, "out"),
            DependType::Inout => write!(f, "inout"),
            DependType::Mutexinoutset => write!(f, "mutexinoutset"),
            DependType::Depobj => write!(f, "depobj"),
            DependType::Source => write!(f, "source"),
            DependType::Sink => write!(f, "sink"),
        }
    }
}

// ============================================================================
// Default Kind (OpenMP 5.2 spec section 2.9.3.1)
// ============================================================================

/// Default data-sharing attribute
///
/// Specifies the default data-sharing attribute for variables.
///
/// ## Examples
///
/// ```
/// # use roup::ir::DefaultKind;
/// let dk = DefaultKind::Shared;
/// assert_eq!(dk.to_string(), "shared");
///
/// let dk = DefaultKind::None;
/// assert_eq!(dk.to_string(), "none");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub enum DefaultKind {
    /// Variables are shared by default
    Shared = 0,
    /// No default (must specify for each variable)
    None = 1,
    /// Variables are private by default (Fortran only)
    Private = 2,
    /// Variables are firstprivate by default
    Firstprivate = 3,
}

impl fmt::Display for DefaultKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DefaultKind::Shared => write!(f, "shared"),
            DefaultKind::None => write!(f, "none"),
            DefaultKind::Private => write!(f, "private"),
            DefaultKind::Firstprivate => write!(f, "firstprivate"),
        }
    }
}

// ============================================================================
// Proc Bind (OpenMP 5.2 spec section 2.6.2)
// ============================================================================

/// Thread affinity policy
///
/// Specifies how threads are bound to processors.
///
/// ## Examples
///
/// ```
/// # use roup::ir::ProcBind;
/// let pb = ProcBind::Close;
/// assert_eq!(pb.to_string(), "close");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub enum ProcBind {
    /// Threads execute close to the master thread
    Master = 0,
    /// Threads execute close to the master thread (OpenMP 5.1 deprecates 'master')
    Close = 1,
    /// Threads spread out across available processors
    Spread = 2,
    /// Implementation-defined binding
    Primary = 3,
}

impl fmt::Display for ProcBind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcBind::Master => write!(f, "master"),
            ProcBind::Close => write!(f, "close"),
            ProcBind::Spread => write!(f, "spread"),
            ProcBind::Primary => write!(f, "primary"),
        }
    }
}

// ============================================================================
// Atomic Default Memory Order (OpenMP 5.2 spec section 2.17.7)
// ============================================================================

/// Default memory order for atomic operations
///
/// Specifies the default memory ordering semantics for atomic operations.
///
/// ## Examples
///
/// ```
/// # use roup::ir::MemoryOrder;
/// let mo = MemoryOrder::SeqCst;
/// assert_eq!(mo.to_string(), "seq_cst");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub enum MemoryOrder {
    /// Sequential consistency (strongest)
    SeqCst = 0,
    /// Acquire-release ordering
    AcqRel = 1,
    /// Release ordering
    Release = 2,
    /// Acquire ordering
    Acquire = 3,
    /// Relaxed ordering (weakest)
    Relaxed = 4,
}

impl fmt::Display for MemoryOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryOrder::SeqCst => write!(f, "seq_cst"),
            MemoryOrder::AcqRel => write!(f, "acq_rel"),
            MemoryOrder::Release => write!(f, "release"),
            MemoryOrder::Acquire => write!(f, "acquire"),
            MemoryOrder::Relaxed => write!(f, "relaxed"),
        }
    }
}

// ============================================================================
// Atomic Operation (OpenMP 5.2 spec section 2.17.7)
// ============================================================================

/// Atomic operation type
///
/// Specifies the type of atomic operation.
///
/// ## Examples
///
/// ```
/// # use roup::ir::AtomicOp;
/// let ao = AtomicOp::Read;
/// assert_eq!(ao.to_string(), "read");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub enum AtomicOp {
    /// Atomic read
    Read = 0,
    /// Atomic write
    Write = 1,
    /// Atomic update
    Update = 2,
    /// Atomic capture
    Capture = 3,
}

impl fmt::Display for AtomicOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AtomicOp::Read => write!(f, "read"),
            AtomicOp::Write => write!(f, "write"),
            AtomicOp::Update => write!(f, "update"),
            AtomicOp::Capture => write!(f, "capture"),
        }
    }
}

// ============================================================================
// Device Type (OpenMP 5.2 spec section 2.14.1)
// ============================================================================

/// Device type for device-specific constructs
///
/// Specifies the target device type.
///
/// ## Examples
///
/// ```
/// # use roup::ir::DeviceType;
/// let dt = DeviceType::Host;
/// assert_eq!(dt.to_string(), "host");
///
/// let dt = DeviceType::Nohost;
/// assert_eq!(dt.to_string(), "nohost");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub enum DeviceType {
    /// Host device
    Host = 0,
    /// Non-host device (accelerator)
    Nohost = 1,
    /// Any device
    Any = 2,
}

impl fmt::Display for DeviceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceType::Host => write!(f, "host"),
            DeviceType::Nohost => write!(f, "nohost"),
            DeviceType::Any => write!(f, "any"),
        }
    }
}

// ============================================================================
// Linear Step (OpenMP 5.2 spec section 2.9.2)
// ============================================================================

/// Linear clause modifier
///
/// Specifies how the linear variable is updated.
///
/// ## Examples
///
/// ```
/// # use roup::ir::LinearModifier;
/// let lm = LinearModifier::Val;
/// assert_eq!(lm.to_string(), "val");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub enum LinearModifier {
    /// Linear variable value
    Val = 0,
    /// Reference to linear variable
    Ref = 1,
    /// Uniform across SIMD lanes
    Uval = 2,
}

impl fmt::Display for LinearModifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LinearModifier::Val => write!(f, "val"),
            LinearModifier::Ref => write!(f, "ref"),
            LinearModifier::Uval => write!(f, "uval"),
        }
    }
}

// ============================================================================
// Lastprivate Modifier (OpenMP 5.2 spec section 2.21.4)
// ============================================================================

/// Lastprivate clause modifier
///
/// Specifies when the lastprivate update occurs.
///
/// ## Examples
///
/// ```
/// # use roup::ir::LastprivateModifier;
/// let lm = LastprivateModifier::Conditional;
/// assert_eq!(lm.to_string(), "conditional");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub enum LastprivateModifier {
    /// Update only if condition is true
    Conditional = 0,
}

impl fmt::Display for LastprivateModifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LastprivateModifier::Conditional => write!(f, "conditional"),
        }
    }
}

// ============================================================================
// Order (OpenMP 5.2 spec section 2.9.6)
// ============================================================================

/// Order clause value
///
/// Specifies iteration execution order constraints.
///
/// ## Examples
///
/// ```
/// # use roup::ir::OrderKind;
/// let ok = OrderKind::Concurrent;
/// assert_eq!(ok.to_string(), "concurrent");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub enum OrderKind {
    /// Iterations may execute concurrently
    Concurrent = 0,
}

impl fmt::Display for OrderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderKind::Concurrent => write!(f, "concurrent"),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Test ReductionOperator
    #[test]
    fn test_reduction_operator_display() {
        assert_eq!(ReductionOperator::Add.to_string(), "+");
        assert_eq!(ReductionOperator::Multiply.to_string(), "*");
        assert_eq!(ReductionOperator::Subtract.to_string(), "-");
        assert_eq!(ReductionOperator::BitwiseAnd.to_string(), "&");
        assert_eq!(ReductionOperator::BitwiseOr.to_string(), "|");
        assert_eq!(ReductionOperator::BitwiseXor.to_string(), "^");
        assert_eq!(ReductionOperator::LogicalAnd.to_string(), "&&");
        assert_eq!(ReductionOperator::LogicalOr.to_string(), "||");
        assert_eq!(ReductionOperator::Min.to_string(), "min");
        assert_eq!(ReductionOperator::Max.to_string(), "max");
        assert_eq!(ReductionOperator::MinusEqual.to_string(), "-=");
        assert_eq!(ReductionOperator::Custom.to_string(), "custom");
    }

    #[test]
    fn test_reduction_operator_equality() {
        assert_eq!(ReductionOperator::Add, ReductionOperator::Add);
        assert_ne!(ReductionOperator::Add, ReductionOperator::Multiply);
    }

    #[test]
    fn test_reduction_operator_copy_clone() {
        let op1 = ReductionOperator::Max;
        let op2 = op1; // Copy
        let op3 = op1.clone(); // Clone
        assert_eq!(op1, op2);
        assert_eq!(op1, op3);
    }

    #[test]
    fn test_reduction_operator_discriminants() {
        // Ensure discriminants are stable for FFI
        assert_eq!(ReductionOperator::Add as i32, 0);
        assert_eq!(ReductionOperator::Multiply as i32, 1);
        assert_eq!(ReductionOperator::BitwiseAnd as i32, 10);
        assert_eq!(ReductionOperator::LogicalAnd as i32, 20);
        assert_eq!(ReductionOperator::Min as i32, 30);
        assert_eq!(ReductionOperator::Custom as i32, 100);
    }

    // Test MapType
    #[test]
    fn test_map_type_display() {
        assert_eq!(MapType::To.to_string(), "to");
        assert_eq!(MapType::From.to_string(), "from");
        assert_eq!(MapType::ToFrom.to_string(), "tofrom");
        assert_eq!(MapType::Alloc.to_string(), "alloc");
        assert_eq!(MapType::Release.to_string(), "release");
        assert_eq!(MapType::Delete.to_string(), "delete");
    }

    #[test]
    fn test_map_type_all_variants() {
        // Ensure all variants are covered
        let all_types = vec![
            MapType::To,
            MapType::From,
            MapType::ToFrom,
            MapType::Alloc,
            MapType::Release,
            MapType::Delete,
        ];
        for mt in all_types {
            assert!(!mt.to_string().is_empty());
        }
    }

    // Test ScheduleKind
    #[test]
    fn test_schedule_kind_display() {
        assert_eq!(ScheduleKind::Static.to_string(), "static");
        assert_eq!(ScheduleKind::Dynamic.to_string(), "dynamic");
        assert_eq!(ScheduleKind::Guided.to_string(), "guided");
        assert_eq!(ScheduleKind::Auto.to_string(), "auto");
        assert_eq!(ScheduleKind::Runtime.to_string(), "runtime");
    }

    #[test]
    fn test_schedule_kind_equality() {
        assert_eq!(ScheduleKind::Static, ScheduleKind::Static);
        assert_ne!(ScheduleKind::Static, ScheduleKind::Dynamic);
    }

    // Test ScheduleModifier
    #[test]
    fn test_schedule_modifier_display() {
        assert_eq!(ScheduleModifier::Monotonic.to_string(), "monotonic");
        assert_eq!(ScheduleModifier::Nonmonotonic.to_string(), "nonmonotonic");
        assert_eq!(ScheduleModifier::Simd.to_string(), "simd");
    }

    #[test]
    fn test_schedule_modifier_all_variants() {
        let all_mods = vec![
            ScheduleModifier::Monotonic,
            ScheduleModifier::Nonmonotonic,
            ScheduleModifier::Simd,
        ];
        for sm in all_mods {
            assert!(!sm.to_string().is_empty());
        }
    }

    // Test DependType
    #[test]
    fn test_depend_type_display() {
        assert_eq!(DependType::In.to_string(), "in");
        assert_eq!(DependType::Out.to_string(), "out");
        assert_eq!(DependType::Inout.to_string(), "inout");
        assert_eq!(DependType::Mutexinoutset.to_string(), "mutexinoutset");
        assert_eq!(DependType::Depobj.to_string(), "depobj");
        assert_eq!(DependType::Source.to_string(), "source");
        assert_eq!(DependType::Sink.to_string(), "sink");
    }

    #[test]
    fn test_depend_type_all_variants() {
        let all_types = vec![
            DependType::In,
            DependType::Out,
            DependType::Inout,
            DependType::Mutexinoutset,
            DependType::Depobj,
            DependType::Source,
            DependType::Sink,
        ];
        for dt in all_types {
            assert!(!dt.to_string().is_empty());
        }
    }

    // Test DefaultKind
    #[test]
    fn test_default_kind_display() {
        assert_eq!(DefaultKind::Shared.to_string(), "shared");
        assert_eq!(DefaultKind::None.to_string(), "none");
        assert_eq!(DefaultKind::Private.to_string(), "private");
        assert_eq!(DefaultKind::Firstprivate.to_string(), "firstprivate");
    }

    #[test]
    fn test_default_kind_language_specific() {
        // Private is Fortran-only, but we can represent it
        let dk = DefaultKind::Private;
        assert_eq!(dk.to_string(), "private");
    }

    // Test ProcBind
    #[test]
    fn test_proc_bind_display() {
        assert_eq!(ProcBind::Master.to_string(), "master");
        assert_eq!(ProcBind::Close.to_string(), "close");
        assert_eq!(ProcBind::Spread.to_string(), "spread");
        assert_eq!(ProcBind::Primary.to_string(), "primary");
    }

    #[test]
    fn test_proc_bind_deprecated_master() {
        // OpenMP 5.1+ deprecates 'master', prefers 'primary'
        // But we still support both for backwards compatibility
        assert_eq!(ProcBind::Master.to_string(), "master");
        assert_eq!(ProcBind::Primary.to_string(), "primary");
    }

    // Test MemoryOrder
    #[test]
    fn test_memory_order_display() {
        assert_eq!(MemoryOrder::SeqCst.to_string(), "seq_cst");
        assert_eq!(MemoryOrder::AcqRel.to_string(), "acq_rel");
        assert_eq!(MemoryOrder::Release.to_string(), "release");
        assert_eq!(MemoryOrder::Acquire.to_string(), "acquire");
        assert_eq!(MemoryOrder::Relaxed.to_string(), "relaxed");
    }

    #[test]
    fn test_memory_order_strength() {
        // SeqCst is strongest, Relaxed is weakest
        // Just verify they all exist
        let all_orders = vec![
            MemoryOrder::SeqCst,
            MemoryOrder::AcqRel,
            MemoryOrder::Release,
            MemoryOrder::Acquire,
            MemoryOrder::Relaxed,
        ];
        assert_eq!(all_orders.len(), 5);
    }

    // Test AtomicOp
    #[test]
    fn test_atomic_op_display() {
        assert_eq!(AtomicOp::Read.to_string(), "read");
        assert_eq!(AtomicOp::Write.to_string(), "write");
        assert_eq!(AtomicOp::Update.to_string(), "update");
        assert_eq!(AtomicOp::Capture.to_string(), "capture");
    }

    #[test]
    fn test_atomic_op_all_variants() {
        let all_ops = vec![
            AtomicOp::Read,
            AtomicOp::Write,
            AtomicOp::Update,
            AtomicOp::Capture,
        ];
        for ao in all_ops {
            assert!(!ao.to_string().is_empty());
        }
    }

    // Test DeviceType
    #[test]
    fn test_device_type_display() {
        assert_eq!(DeviceType::Host.to_string(), "host");
        assert_eq!(DeviceType::Nohost.to_string(), "nohost");
        assert_eq!(DeviceType::Any.to_string(), "any");
    }

    #[test]
    fn test_device_type_all_variants() {
        let all_types = vec![DeviceType::Host, DeviceType::Nohost, DeviceType::Any];
        for dt in all_types {
            assert!(!dt.to_string().is_empty());
        }
    }

    // Test LinearModifier
    #[test]
    fn test_linear_modifier_display() {
        assert_eq!(LinearModifier::Val.to_string(), "val");
        assert_eq!(LinearModifier::Ref.to_string(), "ref");
        assert_eq!(LinearModifier::Uval.to_string(), "uval");
    }

    // Test LastprivateModifier
    #[test]
    fn test_lastprivate_modifier_display() {
        assert_eq!(LastprivateModifier::Conditional.to_string(), "conditional");
    }

    // Test OrderKind
    #[test]
    fn test_order_kind_display() {
        assert_eq!(OrderKind::Concurrent.to_string(), "concurrent");
    }

    // Corner case: enum size consistency for FFI
    #[test]
    fn test_enum_sizes_for_ffi() {
        use std::mem::size_of;

        // All enums should be pointer-sized or smaller for FFI
        assert!(size_of::<ReductionOperator>() <= size_of::<usize>());
        assert!(size_of::<MapType>() <= size_of::<usize>());
        assert!(size_of::<ScheduleKind>() <= size_of::<usize>());
        assert!(size_of::<DependType>() <= size_of::<usize>());
        assert!(size_of::<DefaultKind>() <= size_of::<usize>());
        assert!(size_of::<ProcBind>() <= size_of::<usize>());
        assert!(size_of::<MemoryOrder>() <= size_of::<usize>());
        assert!(size_of::<AtomicOp>() <= size_of::<usize>());
        assert!(size_of::<DeviceType>() <= size_of::<usize>());
        assert!(size_of::<LinearModifier>() <= size_of::<usize>());
        assert!(size_of::<LastprivateModifier>() <= size_of::<usize>());
        assert!(size_of::<OrderKind>() <= size_of::<usize>());
    }

    // Corner case: hash consistency
    #[test]
    fn test_enum_hash_consistency() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let op1 = ReductionOperator::Add;
        let op2 = ReductionOperator::Add;

        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();

        op1.hash(&mut hasher1);
        op2.hash(&mut hasher2);

        assert_eq!(hasher1.finish(), hasher2.finish());
    }

    // Corner case: debug formatting
    #[test]
    fn test_enum_debug_formatting() {
        let op = ReductionOperator::Add;
        let debug_str = format!("{:?}", op);
        assert!(debug_str.contains("Add"));
    }
}
