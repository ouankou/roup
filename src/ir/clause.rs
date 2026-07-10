//! Clause semantic types for OpenMP IR
//!
//! This module defines types for representing OpenMP clause semantics.
//! It captures the meaning of clauses, not just their syntax.
//!
//! ## Design Philosophy
//!
//! OpenMP has many clause modifiers that affect behavior:
//! - Reduction operators: `+`, `*`, `max`, `min`, etc.
//! - Map types: `to`, `from`, `tofrom`, `alloc`, etc.
//! - Schedule kinds: `static`, `dynamic`, `guided`, `auto`
//! - Depend types: `in`, `out`, `inout`, `mutexinoutset`
//!
//! Each modifier is represented as an ordinary Rust enum with clear variant
//! names and exhaustive matching. The optional `roup-capi` crate translates
//! these semantic values into its own stable ABI types; this safe parser crate
//! deliberately exposes no C layout.
//!
//! ## Corner Cases Handled
//!
//! - Unknown/custom operators via `Custom` variants
//! - Language-specific operators (C++ vs Fortran)
//! - OpenMP version-specific features
//! - User-defined reduction operators

use std::fmt;

use super::{Expression, Identifier, LValue, Variable};
use crate::ast::{
    OmpDirective, OmpInductionIdentifier, OmpInductorExpression, OmpReductionIdentifier,
};
use crate::host::{StringLiteral, TypeName};

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
pub enum MapType {
    /// Map data to device (host → device)
    To,
    /// Map data from device (device → host)
    From,
    /// Map data to and from device (bidirectional)
    ToFrom,
    /// Allocate or retain device storage without a transfer (OpenMP 6.0)
    Storage,
}

impl fmt::Display for MapType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MapType::To => write!(f, "to"),
            MapType::From => write!(f, "from"),
            MapType::ToFrom => write!(f, "tofrom"),
            MapType::Storage => write!(f, "storage"),
        }
    }
}

/// Exact source spelling that selected canonical storage map semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MapTypeSpelling {
    Canonical,
    Alloc,
    Release,
    Delete,
}

/// Map-type modifiers (e.g., `always`, `close`, `present`, `self`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MapModifier {
    Always,
    Close,
    Present,
    SelfMap,
    Iterator,
    Ref(MapRefKind),
    Delete,
}

/// Reference handling selected by an OpenMP 6.0 map `ref` modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MapRefKind {
    Pointee,
    Pointer,
    PointerAndPointee,
}

impl fmt::Display for MapModifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MapModifier::Always => write!(f, "always"),
            MapModifier::Close => write!(f, "close"),
            MapModifier::Present => write!(f, "present"),
            MapModifier::SelfMap => write!(f, "self"),
            MapModifier::Iterator => write!(f, "iterator"),
            MapModifier::Ref(MapRefKind::Pointee) => write!(f, "ref_ptee"),
            MapModifier::Ref(MapRefKind::Pointer) => write!(f, "ref_ptr"),
            MapModifier::Ref(MapRefKind::PointerAndPointee) => write!(f, "ref_ptr_ptee"),
            MapModifier::Delete => write!(f, "delete"),
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
pub enum ScheduleKind {
    /// Iterations divided into chunks of specified size, assigned statically
    Static,
    /// Iterations divided into chunks, assigned dynamically at runtime
    Dynamic,
    /// Similar to dynamic but chunk size decreases exponentially
    Guided,
    /// Implementation-defined scheduling
    Auto,
    /// Runtime determines schedule via environment variable
    Runtime,
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
pub enum ScheduleModifier {
    /// Iterations assigned in monotonically increasing order
    Monotonic,
    /// No ordering guarantee (allows optimizations)
    Nonmonotonic,
    /// SIMD execution of iterations
    Simd,
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
pub enum DependType {
    /// Read dependency
    In,
    /// Write dependency
    Out,
    /// Read-write dependency
    Inout,
    /// Read-write dependency that must not be executed concurrently
    Inoutset,
    /// Mutual exclusion with inout
    Mutexinoutset,
}

/// The two semantically distinct payloads of an OpenMP `depend` clause.
#[derive(Debug, Clone, PartialEq)]
pub enum OmpDependence {
    /// A task-dependence type applied to storage locators.
    Locators {
        kind: DependType,
        locators: Vec<OmpLocator>,
    },
    /// Initialized depend objects. Array sections and general expressions are
    /// excluded by construction.
    Depobjs { objects: Vec<Variable> },
}

/// Depobj update dependence types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DepobjUpdateDependence {
    In,
    Out,
    Inout,
    Inoutset,
    Mutexinoutset,
}

impl fmt::Display for DependType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DependType::In => write!(f, "in"),
            DependType::Out => write!(f, "out"),
            DependType::Inout => write!(f, "inout"),
            DependType::Inoutset => write!(f, "inoutset"),
            DependType::Mutexinoutset => write!(f, "mutexinoutset"),
        }
    }
}

impl fmt::Display for DepobjUpdateDependence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DepobjUpdateDependence::In => write!(f, "in"),
            DepobjUpdateDependence::Out => write!(f, "out"),
            DepobjUpdateDependence::Inout => write!(f, "inout"),
            DepobjUpdateDependence::Inoutset => write!(f, "inoutset"),
            DepobjUpdateDependence::Mutexinoutset => write!(f, "mutexinoutset"),
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
pub enum DefaultKind {
    /// Variables are shared by default
    Shared,
    /// No default (must specify for each variable)
    None,
    /// Variables are private by default (Fortran only)
    Private,
    /// Variables are firstprivate by default
    Firstprivate,
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
// Defaultmap Clause Attributes (OpenMP 5.2 spec section 2.21.7)
// ============================================================================

/// Behavior applied to implicit data mappings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefaultmapBehavior {
    Alloc,
    To,
    From,
    Tofrom,
    Firstprivate,
    None,
    Default,
    Present,
    Private,
    SelfMap,
    Storage,
}

impl fmt::Display for DefaultmapBehavior {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            DefaultmapBehavior::Alloc => "alloc",
            DefaultmapBehavior::To => "to",
            DefaultmapBehavior::From => "from",
            DefaultmapBehavior::Tofrom => "tofrom",
            DefaultmapBehavior::Firstprivate => "firstprivate",
            DefaultmapBehavior::None => "none",
            DefaultmapBehavior::Default => "default",
            DefaultmapBehavior::Present => "present",
            DefaultmapBehavior::Private => "private",
            DefaultmapBehavior::SelfMap => "self",
            DefaultmapBehavior::Storage => "storage",
        };
        write!(f, "{text}")
    }
}

/// Category of data to which the defaultmap clause applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefaultmapCategory {
    Scalar,
    Aggregate,
    Pointer,
    All,
    Allocatable,
}

impl fmt::Display for DefaultmapCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            DefaultmapCategory::Scalar => "scalar",
            DefaultmapCategory::Aggregate => "aggregate",
            DefaultmapCategory::Pointer => "pointer",
            DefaultmapCategory::All => "all",
            DefaultmapCategory::Allocatable => "allocatable",
        };
        write!(f, "{text}")
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
pub enum ProcBind {
    /// Threads execute close to the primary thread.
    Close,
    /// Threads spread out across available processors
    Spread,
    /// Implementation-defined binding
    Primary,
}

impl fmt::Display for ProcBind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcBind::Close => write!(f, "close"),
            ProcBind::Spread => write!(f, "spread"),
            ProcBind::Primary => write!(f, "primary"),
        }
    }
}

// ============================================================================
// Loop Bind (OpenMP 5.1 loop construct)
// ============================================================================

/// Binding for `bind(...)` on loop constructs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BindModifier {
    Teams,
    Parallel,
    Thread,
}

impl fmt::Display for BindModifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BindModifier::Teams => write!(f, "teams"),
            BindModifier::Parallel => write!(f, "parallel"),
            BindModifier::Thread => write!(f, "thread"),
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
pub enum MemoryOrder {
    /// Sequential consistency (strongest)
    SeqCst,
    /// Acquire-release ordering
    AcqRel,
    /// Release ordering
    Release,
    /// Acquire ordering
    Acquire,
    /// Relaxed ordering (weakest)
    Relaxed,
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
pub enum AtomicOp {
    /// Atomic read
    Read,
    /// Atomic write
    Write,
    /// Atomic update
    Update,
}

impl fmt::Display for AtomicOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AtomicOp::Read => write!(f, "read"),
            AtomicOp::Write => write!(f, "write"),
            AtomicOp::Update => write!(f, "update"),
        }
    }
}

/// Extended atomic operation modifiers represented independently from the
/// mutually exclusive read/write/update operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExtendedAtomicKind {
    Capture,
    Compare,
    Weak,
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
pub enum DeviceType {
    /// Host device
    Host,
    /// Non-host device (accelerator)
    Nohost,
    /// Any device
    Any,
}

// ============================================================================
// Uses Allocators Clause Helpers (OpenMP 5.2 spec section 2.11.5)
// ============================================================================

/// Built-in allocator identifiers recognized by the specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UsesAllocatorBuiltin {
    Null,
    Default,
    LargeCap,
    Const,
    HighBw,
    LowLat,
    Cgroup,
    Pteam,
    Thread,
}

impl UsesAllocatorBuiltin {
    pub fn as_str(self) -> &'static str {
        match self {
            UsesAllocatorBuiltin::Null => "omp_null_allocator",
            UsesAllocatorBuiltin::Default => "omp_default_mem_alloc",
            UsesAllocatorBuiltin::LargeCap => "omp_large_cap_mem_alloc",
            UsesAllocatorBuiltin::Const => "omp_const_mem_alloc",
            UsesAllocatorBuiltin::HighBw => "omp_high_bw_mem_alloc",
            UsesAllocatorBuiltin::LowLat => "omp_low_lat_mem_alloc",
            UsesAllocatorBuiltin::Cgroup => "omp_cgroup_mem_alloc",
            UsesAllocatorBuiltin::Pteam => "omp_pteam_mem_alloc",
            UsesAllocatorBuiltin::Thread => "omp_thread_mem_alloc",
        }
    }
}

impl fmt::Display for UsesAllocatorBuiltin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Kind of allocator referenced by a `uses_allocators` clause entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UsesAllocatorKind {
    Builtin(UsesAllocatorBuiltin),
    Custom(Identifier),
}

/// Predefined OpenMP memory-space handle accepted by `memspace(...)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OmpMemorySpace {
    Default,
    LargeCap,
    Const,
    HighBw,
    LowLat,
}

impl OmpMemorySpace {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Default => "omp_default_mem_space",
            Self::LargeCap => "omp_large_cap_mem_space",
            Self::Const => "omp_const_mem_space",
            Self::HighBw => "omp_high_bw_mem_space",
            Self::LowLat => "omp_low_lat_mem_space",
        }
    }
}

impl fmt::Display for OmpMemorySpace {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl UsesAllocatorKind {
    pub fn canonical_name(&self) -> &str {
        match self {
            UsesAllocatorKind::Builtin(builtin) => builtin.as_str(),
            UsesAllocatorKind::Custom(identifier) => identifier.as_str(),
        }
    }
}

impl fmt::Display for UsesAllocatorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.canonical_name())
    }
}

/// Parser-only provenance for version diagnostics. It is intentionally not
/// part of the public semantic AST or canonical rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum UsesAllocatorSourceSyntax {
    /// OpenMP 5.0 comma-list item: `allocator[(traits)]`.
    Historical,
    /// OpenMP 5.2 clause-argument specification: `[modifiers:] allocator`.
    Modifier,
}

/// Parsed `uses_allocators` clause entry.
#[derive(Debug, Clone)]
pub struct UsesAllocatorSpec {
    allocator: UsesAllocatorKind,
    traits: Option<Variable>,
    memspace: Option<OmpMemorySpace>,
    source_syntax: UsesAllocatorSourceSyntax,
}

impl UsesAllocatorSpec {
    pub(crate) fn new(
        allocator: UsesAllocatorKind,
        traits: Option<Variable>,
        memspace: Option<OmpMemorySpace>,
        source_syntax: UsesAllocatorSourceSyntax,
    ) -> Result<Self, &'static str> {
        if matches!(allocator, UsesAllocatorKind::Builtin(_))
            && (traits.is_some() || memspace.is_some())
        {
            return Err("predefined allocators cannot have uses_allocators modifiers");
        }
        if source_syntax == UsesAllocatorSourceSyntax::Historical && memspace.is_some() {
            return Err("historical uses_allocators syntax only supports a traits expression");
        }
        Ok(Self {
            allocator,
            traits,
            memspace,
            source_syntax,
        })
    }

    #[must_use]
    pub const fn allocator(&self) -> &UsesAllocatorKind {
        &self.allocator
    }

    #[must_use]
    pub const fn traits(&self) -> Option<&Variable> {
        self.traits.as_ref()
    }

    #[must_use]
    pub const fn memspace(&self) -> Option<OmpMemorySpace> {
        self.memspace
    }

    pub(crate) const fn source_syntax(&self) -> UsesAllocatorSourceSyntax {
        self.source_syntax
    }
}

impl PartialEq for UsesAllocatorSpec {
    fn eq(&self, other: &Self) -> bool {
        self.allocator == other.allocator
            && self.traits == other.traits
            && self.memspace == other.memspace
    }
}

/// Requires clause modifiers (OpenMP 5.x)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RequireModifier {
    ReverseOffload,
    UnifiedAddress,
    UnifiedSharedMemory,
    DynamicAllocators,
    SelfMaps,
    DeviceSafesync,
    AtomicDefaultMemOrder(MemoryOrder),
    ExtImplementationDefinedRequirement(Option<Identifier>),
}

impl fmt::Display for RequireModifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReverseOffload => formatter.write_str("reverse_offload"),
            Self::UnifiedAddress => formatter.write_str("unified_address"),
            Self::UnifiedSharedMemory => formatter.write_str("unified_shared_memory"),
            Self::DynamicAllocators => formatter.write_str("dynamic_allocators"),
            Self::SelfMaps => formatter.write_str("self_maps"),
            Self::DeviceSafesync => formatter.write_str("device_safesync"),
            Self::AtomicDefaultMemOrder(order) => {
                write!(formatter, "atomic_default_mem_order({order})")
            }
            Self::ExtImplementationDefinedRequirement(Some(name)) => write!(formatter, "{name}"),
            Self::ExtImplementationDefinedRequirement(None) => {
                formatter.write_str("ext_implementation_defined_requirement")
            }
        }
    }
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
pub enum LinearModifier {
    /// Linear variable value
    Val,
    /// Reference to linear variable
    Ref,
    /// Uniform across SIMD lanes
    Uval,
}

/// Standardized source grammar used for a linear clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinearSourceSyntax {
    Historical,
    ModifierPrefix,
    CanonicalModifiers,
}

/// Standardized source grammar used for an `allocate` clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AllocateSourceSyntax {
    /// `allocate(list)` with no allocator or alignment modifier.
    Unmodified,
    /// OpenMP 5.0 `allocate(allocator-expression: list)`.
    SimpleAllocator,
    /// OpenMP 5.1 `allocate(allocator(expr), align(expr): list)` grammar.
    Modifiers,
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
pub enum LastprivateModifier {
    /// Update only if condition is true
    Conditional,
}

/// OpenMP 6.0 modifier that reads firstprivate originals from saved state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FirstprivateModifier {
    Saved,
}

impl fmt::Display for FirstprivateModifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Saved => f.write_str("saved"),
        }
    }
}

/// Thread set selected by the OpenMP 6.0 `threadset` clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThreadsetKind {
    OmpPool,
    OmpTeam,
}

impl fmt::Display for ThreadsetKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::OmpPool => "omp_pool",
            Self::OmpTeam => "omp_team",
        })
    }
}

/// Binding thread set selected by the OpenMP 6.0 `memscope` clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemscopeKind {
    All,
    Cgroup,
    Device,
}

impl fmt::Display for MemscopeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::All => "all",
            Self::Cgroup => "cgroup",
            Self::Device => "device",
        })
    }
}

// ============================================================================
// Reduction Modifiers (OpenMP 5.x)
// ============================================================================

/// Reduction clause modifiers (`task`, `inscan`, `default`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReductionModifier {
    Task,
    Inscan,
    Default,
    Original(OriginalSharing),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OriginalSharing {
    Default,
    Private,
    Shared,
}

impl fmt::Display for ReductionModifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReductionModifier::Task => write!(f, "task"),
            ReductionModifier::Inscan => write!(f, "inscan"),
            ReductionModifier::Default => write!(f, "default"),
            ReductionModifier::Original(sharing) => write!(f, "original(sharing={sharing})"),
        }
    }
}

impl fmt::Display for OriginalSharing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OriginalSharing::Default => write!(f, "default"),
            OriginalSharing::Private => write!(f, "private"),
            OriginalSharing::Shared => write!(f, "shared"),
        }
    }
}

/// Device clause modifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceModifier {
    Ancestor,
    DeviceNum,
}

impl fmt::Display for DeviceModifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceModifier::Ancestor => write!(f, "ancestor"),
            DeviceModifier::DeviceNum => write!(f, "device_num"),
        }
    }
}

/// Grainsize clause modifier (OpenMP 5.1)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GrainsizeModifier {
    Strict,
}

impl fmt::Display for GrainsizeModifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GrainsizeModifier::Strict => write!(f, "strict"),
        }
    }
}

/// Num_tasks clause modifier (OpenMP 5.1)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NumTasksModifier {
    Strict,
}

impl fmt::Display for NumTasksModifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NumTasksModifier::Strict => write!(f, "strict"),
        }
    }
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
pub enum OrderKind {
    /// Iterations may execute concurrently
    Concurrent,
}

/// Order clause execution modifiers (OpenMP 5.1)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderModifier {
    /// Enforce reproducible execution.
    Reproducible,
    /// Allow unconstrained execution.
    Unconstrained,
}

impl fmt::Display for OrderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderKind::Concurrent => write!(f, "concurrent"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DoacrossType {
    Source,
    Sink,
}

/// A signed constant offset from one loop-iteration variable.
#[derive(Debug, Clone, PartialEq)]
pub enum OmpDoacrossOffset {
    Add(Expression),
    Subtract(Expression),
}

/// One dimension in a doacross loop-iteration vector.
#[derive(Debug, Clone, PartialEq)]
pub struct OmpDoacrossVectorItem {
    pub variable: Identifier,
    pub offset: Option<OmpDoacrossOffset>,
}

/// The iteration selected by a doacross dependence.
#[derive(Debug, Clone, PartialEq)]
pub enum OmpDoacrossIteration {
    /// The predefined `omp_cur_iteration` value.
    Current,
    /// Exactly `omp_cur_iteration - 1`.
    PreviousCurrent,
    /// A non-empty vector of iteration variables with optional signed offsets.
    Vector(Vec<OmpDoacrossVectorItem>),
}

impl fmt::Display for DoacrossType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DoacrossType::Source => write!(f, "source"),
            DoacrossType::Sink => write!(f, "sink"),
        }
    }
}

/// Scan clause mode (inclusive/exclusive) used for `scan` clauses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScanClauseMode {
    Inclusive,
    Exclusive,
}

impl fmt::Display for ScanClauseMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScanClauseMode::Inclusive => write!(f, "inclusive"),
            ScanClauseMode::Exclusive => write!(f, "exclusive"),
        }
    }
}

impl fmt::Display for OrderModifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderModifier::Reproducible => write!(f, "reproducible"),
            OrderModifier::Unconstrained => write!(f, "unconstrained"),
        }
    }
}

// ============================================================================
// ClauseItem: Items that appear in clause lists
// ============================================================================

/// Iterator definition used by depend/affinity iterator modifiers.
///
/// Example: `iterator(int i=0:N:1)` yields a type of `int`, name `i`,
/// start `0`, end `N`, and step `1`.
#[derive(Debug, Clone, PartialEq)]
pub struct DependIterator {
    /// Optional type name (e.g., `int` or `double`).
    type_name: Option<TypeName>,
    /// Iterator induction variable.
    name: Identifier,
    /// Starting expression.
    start: Expression,
    /// Ending expression.
    end: Expression,
    /// Optional step expression.
    step: Option<Expression>,
}

impl DependIterator {
    pub(crate) fn new(
        type_name: Option<TypeName>,
        name: Identifier,
        start: Expression,
        end: Expression,
        step: Option<Expression>,
    ) -> Self {
        Self {
            type_name,
            name,
            start,
            end,
            step,
        }
    }

    #[must_use]
    pub const fn type_name(&self) -> Option<&TypeName> {
        self.type_name.as_ref()
    }

    #[must_use]
    pub const fn name(&self) -> &Identifier {
        &self.name
    }

    #[must_use]
    pub const fn start(&self) -> &Expression {
        &self.start
    }

    #[must_use]
    pub const fn end(&self) -> &Expression {
        &self.end
    }

    #[must_use]
    pub const fn step(&self) -> Option<&Expression> {
        self.step.as_ref()
    }
}

impl fmt::Display for DependIterator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref ty) = self.type_name {
            write!(f, "{ty} {}", self.name)?;
        } else {
            write!(f, "{}", self.name)?;
        }
        write!(f, "={}", self.start)?;
        write!(f, ":{}", self.end)?;
        if let Some(step) = &self.step {
            write!(f, ":{step}")?;
        }
        Ok(())
    }
}

/// Item that can appear in a clause list
///
/// Many OpenMP clauses accept lists of items that can be:
/// - Simple identifiers: `private(x, y, z)`
/// - Variables with array sections: `map(to: arr[0:N])`
/// - Fortran named common blocks: `private(/workspace/)`
/// - Expressions: `if(n > 100)`
///
/// ## Examples
///
/// ```
/// # use roup::ir::{ClauseItem, Identifier, Variable, Expression, ParserConfig};
/// // Simple identifier
/// let item = ClauseItem::Identifier(Identifier::new("x").expect("valid identifier"));
/// assert_eq!(item.to_string(), "x");
///
/// // Variable with array section
/// let config = ParserConfig::c();
/// let var = Variable::parse("arr", &config).expect("valid variable");
/// let item = ClauseItem::Variable(var);
/// assert_eq!(item.to_string(), "arr");
///
/// // Expression
/// let expr = Expression::new("n > 100", &config).unwrap();
/// let item = ClauseItem::Expression(expr);
/// assert_eq!(item.to_string(), "n > 100");
/// ```
///
/// ## Learning: Enums with Data
///
/// Unlike the modifier enums (which are just unit variants), ClauseItem
/// is an enum where each variant **contains data**:
///
/// ```text
/// enum ClauseItem {
///     Identifier(Identifier),  // Contains an Identifier
///     Variable(Variable),       // Contains a Variable
///     FortranCommonBlock(Identifier), // Contains a named common block
///     Expression(Expression),   // Contains an Expression
/// }
/// ```
///
/// This is like a tagged union in C, but type-safe.
#[derive(Debug, Clone, PartialEq)]
pub enum ClauseItem {
    /// Simple identifier (e.g., `x` in `private(x)`)
    Identifier(Identifier),
    /// Variable with optional array sections (e.g., `arr[0:N]` in `map(to: arr[0:N])`)
    Variable(Variable),
    /// Fortran named common block (e.g., `/workspace/` in `private(/workspace/)`).
    FortranCommonBlock(Identifier),
    /// Expression (e.g., `n > 100` in `if(n > 100)`)
    Expression(Expression),
}

impl fmt::Display for ClauseItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClauseItem::Identifier(id) => write!(f, "{id}"),
            ClauseItem::Variable(var) => write!(f, "{var}"),
            ClauseItem::FortranCommonBlock(name) => write!(f, "/{name}/"),
            ClauseItem::Expression(expr) => write!(f, "{expr}"),
        }
    }
}

impl From<Identifier> for ClauseItem {
    fn from(id: Identifier) -> Self {
        ClauseItem::Identifier(id)
    }
}

impl From<Variable> for ClauseItem {
    fn from(var: Variable) -> Self {
        ClauseItem::Variable(var)
    }
}

impl From<Expression> for ClauseItem {
    fn from(expr: Expression) -> Self {
        ClauseItem::Expression(expr)
    }
}

// ========================================================================
// Argument-adjustment modifiers
// ========================================================================

/// Modifier for `adjust_args` clauses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AdjustArgsModifier {
    Nothing,
    NeedDevicePtr,
    NeedDeviceAddr,
}

/// One parameter selected by an OpenMP `adjust_args` clause.
///
/// OpenMP 5.1 and 5.2 accept only [`Self::Named`] items. OpenMP 6.0 adds
/// one-based absolute positions and inclusive ranges. Range bounds remain
/// typed host expressions because the specification permits constant integer
/// expressions, including expressions relative to `omp_num_args`.
#[derive(Debug, Clone, PartialEq)]
pub enum OmpParameterListItem {
    /// A named function parameter or Fortran dummy argument.
    Named(Identifier),
    /// A one-based absolute position in the parameter list.
    Position(u64),
    /// An inclusive range. An omitted lower bound means the first parameter;
    /// an omitted upper bound means `omp_num_args`.
    Range(Box<OmpParameterRange>),
}

/// Inclusive positional range selected by `adjust_args`.
///
/// This is boxed by [`OmpParameterListItem::Range`] so the common named and
/// absolute-position variants remain compact while both optional bounds retain
/// their fully typed host-expression trees.
#[derive(Debug, Clone, PartialEq)]
pub struct OmpParameterRange {
    lower: Option<Expression>,
    upper: Option<Expression>,
}

impl OmpParameterRange {
    pub(crate) fn new(lower: Option<Expression>, upper: Option<Expression>) -> Option<Self> {
        (lower.is_some() || upper.is_some()).then_some(Self { lower, upper })
    }

    #[must_use]
    pub const fn lower(&self) -> Option<&Expression> {
        self.lower.as_ref()
    }

    #[must_use]
    pub const fn upper(&self) -> Option<&Expression> {
        self.upper.as_ref()
    }
}

/// Severity levels for the `error` directive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SeverityKind {
    Fatal,
    Warning,
}

impl fmt::Display for SeverityKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SeverityKind::Fatal => write!(f, "fatal"),
            SeverityKind::Warning => write!(f, "warning"),
        }
    }
}

/// Error location for the `error` directive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AtKind {
    Compilation,
    Execution,
}

impl fmt::Display for AtKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AtKind::Compilation => write!(f, "compilation"),
            AtKind::Execution => write!(f, "execution"),
        }
    }
}

/// An interoperability property set requested by an OpenMP `init` clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OmpInteropType {
    Target,
    Targetsync,
}

impl fmt::Display for OmpInteropType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Target => write!(f, "target"),
            Self::Targetsync => write!(f, "targetsync"),
        }
    }
}

/// A foreign runtime identifier in an interoperability preference.
///
/// String literals are retained as typed literals. Every other permitted form
/// is a typed host expression whose constant-integral status is established by
/// semantic validation.
#[derive(Debug, Clone, PartialEq)]
pub enum OmpForeignRuntimeIdentifier {
    StringLiteral(StringLiteral),
    ConstantExpression(Expression),
}

/// One selector in an OpenMP 6.0 interoperability preference specification.
#[derive(Debug, Clone, PartialEq)]
pub enum OmpPreferenceSelector {
    ForeignRuntime(OmpForeignRuntimeIdentifier),
    Attributes(Vec<StringLiteral>),
}

/// One preference specification in the `prefer_type` modifier.
#[derive(Debug, Clone, PartialEq)]
pub enum OmpPreferenceSpecification {
    /// The OpenMP 5.1 spelling, which remains a canonical shorthand for an
    /// `fr(...)` selector in later specifications.
    ForeignRuntime(OmpForeignRuntimeIdentifier),
    /// OpenMP 6.0 brace-delimited selector syntax.
    Selectors(Vec<OmpPreferenceSelector>),
}

/// Interoperability modifiers shared by `init` and operations that request
/// the same foreign-runtime property sets.
#[derive(Debug, Clone, PartialEq)]
pub struct OmpInteropInitModifiers {
    pub interop_types: Vec<OmpInteropType>,
    pub preferences: Vec<OmpPreferenceSpecification>,
}

/// One operation that appends an argument to an OpenMP function variant.
#[derive(Debug, Clone, PartialEq)]
pub enum OmpAppendOperation {
    /// Construct or consume an interoperability object using the same typed
    /// modifiers accepted by the `init` clause.
    Interop(OmpInteropInitModifiers),
}

/// The generated-loop group selected by an OpenMP 6.0 `apply` clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OmpApplyLoopKind {
    Fused,
    Grid,
    Identity,
    Interchanged,
    Intratile,
    Offsets,
    Reversed,
    Split,
    Unrolled,
}

/// Optional generated-loop modifier on an OpenMP 6.0 `apply` clause.
#[derive(Debug, Clone, PartialEq)]
pub struct OmpApplyLoopModifier {
    pub kind: OmpApplyLoopKind,
    pub indices: Vec<Expression>,
}

/// Execution guarantee selected by an OpenMP 6.0 `induction` clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OmpInductionModifier {
    Relaxed,
    Strict,
}

/// One locator accepted by OpenMP data-motion clauses.
///
/// C and C++ locator lists admit any lvalue expression, not only a variable
/// designator. Fortran common blocks remain a distinct standardized list item
/// and are never reconstructed from a string in downstream consumers.
#[derive(Debug, Clone, PartialEq)]
pub enum OmpLocator {
    /// The standardized `omp_all_memory` reserved locator.
    AllMemory,
    LValue(LValue),
    /// A host expression whose lvalue/glvalue category depends on type and
    /// symbol information unavailable to the standalone parser.
    PotentialLValue(Expression),
    FortranCommonBlock(Identifier),
}

impl fmt::Display for OmpLocator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AllMemory => formatter.write_str("omp_all_memory"),
            Self::LValue(value) => value.fmt(formatter),
            Self::PotentialLValue(value) => value.fmt(formatter),
            Self::FortranCommonBlock(name) => write!(formatter, "/{name}/"),
        }
    }
}

/// One entry in the OpenMP 6.0 `counts` clause.
#[derive(Debug, Clone, PartialEq)]
pub enum OmpCount {
    /// The predefined `omp_fill` identifier.
    Fill,
    /// A non-negative constant integer expression. Constant/value checking
    /// that requires host semantic information is performed by validation.
    Expression(Expression),
}

// ============================================================================
// ClauseData: Complete clause semantic information
// ============================================================================

/// Complete semantic data for an OpenMP clause
///
/// This enum represents the **meaning** of each OpenMP clause type.
/// Each variant captures the specific data needed for that clause.
///
/// ## Learning: Large Enums with Complex Data
///
/// This enum demonstrates several advanced Rust patterns:
///
/// 1. **Many variants**: ~30 variants for different clause types
/// 2. **Variants with data**: Most variants contain structured data
/// 3. **Named fields**: Using struct-like syntax for clarity
/// 4. **Vec for lists**: Variable-length lists of items
/// 5. **Option for optionals**: Optional parameters
/// 6. **Composition**: Combines all previous IR types
///
/// ## Design Philosophy
///
/// Each variant captures exactly what's needed for semantic analysis:
/// - `Private`: List of variables to make private
/// - `Reduction`: Operator + list of reduction variables
/// - `Map`: Map type + list of variables to map
/// - `Schedule`: Schedule kind + optional modifiers + optional chunk size
///
/// This is much richer than the parser's textual parser representation.
#[derive(Debug, Clone, PartialEq)]
pub enum ClauseData {
    // ========================================================================
    // Bare clauses (no parameters)
    // ========================================================================
    /// Clause with no parameters. Its name is owned by the enclosing clause kind.
    Bare,
    /// `nowait[(do_not_synchronize)]`.
    Nowait {
        do_not_synchronize: Option<Expression>,
    },
    /// `nogroup[(do_not_synchronize)]`.
    Nogroup {
        do_not_synchronize: Option<Expression>,
    },

    // ========================================================================
    // Item list clauses
    // ========================================================================
    /// A remaining ordinary variable-list payload. Clauses whose list grammar
    /// is narrower or wider have dedicated variants below.
    ItemList(Vec<ClauseItem>),

    /// `sizes(size-list)` on `tile` or `stripe`.
    Sizes { sizes: Vec<Expression> },

    /// `permutation(permutation-list)` on `interchange`.
    Permutation { positions: Vec<Expression> },

    /// `counts(count-list)` on `split`.
    Counts { counts: Vec<OmpCount> },

    /// `align(alignment)` on `allocate`.
    Align { alignment: Expression },

    /// `destroy[(destroy-var)]`; omission is valid only for `depobj`.
    Destroy { variable: Option<Variable> },

    /// `final(final-expr)`.
    Final { condition: Expression },

    /// `graph_id(graph-id-value)`.
    GraphId { value: Expression },

    /// `hint(hint-expr)`.
    Hint { value: Expression },

    /// `holds(hold-expr)`.
    Holds { condition: Expression },

    /// `message(msg-string)`. This remains an expression because execution-
    /// time messages may use a string-typed variable.
    Message { value: Expression },

    /// `nocontext(do-not-update-context)`.
    Nocontext { condition: Expression },

    /// `novariants(do-not-use-variant)`.
    Novariants { condition: Expression },

    /// `uniform(parameter-list)`; every entry is a named parameter.
    Uniform { parameters: Vec<Identifier> },

    /// `use(interop-var)` with exactly one variable.
    Use { interop_var: Variable },

    /// `enter([automap:] extended-list)` on `declare_target`. Historical
    /// `to(list)` source syntax is canonicalized to this same semantic form.
    Enter {
        automap: bool,
        items: Vec<ClauseItem>,
    },

    /// `to([present, mapper(...), iterator(...):] locator-list)`.
    To {
        present: bool,
        mapper: Option<crate::ast::OmpMapperId>,
        iterators: Vec<DependIterator>,
        locators: Vec<OmpLocator>,
    },

    /// `from([present, mapper(...), iterator(...):] locator-list)`.
    From {
        present: bool,
        mapper: Option<crate::ast::OmpMapperId>,
        iterators: Vec<DependIterator>,
        locators: Vec<OmpLocator>,
    },
    /// `scan` clause operands with inclusive/exclusive mode
    Scan {
        mode: ScanClauseMode,
        items: Vec<ClauseItem>,
    },

    /// `init_complete[(create-init-phase)]` on a scan directive.
    InitComplete {
        create_init_phase: Option<Expression>,
    },

    /// `inbranch[(condition)]` or `notinbranch[(condition)]`.
    Branch { condition: Option<Expression> },

    /// `full[(fully_unroll)]` on an unroll directive.
    Full { fully_unroll: Option<Expression> },

    /// `partial[(unroll_factor)]` on an unroll directive.
    Partial { unroll_factor: Option<Expression> },

    /// `mergeable[(can_merge)]` on a task-generating directive.
    Mergeable { can_merge: Option<Expression> },

    /// `untied[(can_change_threads)]` on a task-generating directive.
    Untied {
        can_change_threads: Option<Expression>,
    },

    /// `simd[(apply_to_simd)]` in a parallelization-level clause group.
    Simd { apply_to_simd: Option<Expression> },

    /// `threads[(apply_to_threads)]` in a parallelization-level clause group.
    Threads {
        apply_to_threads: Option<Expression>,
    },

    /// One of the `no_openmp*` or `no_parallelism` assumption clauses.
    Assumption { can_assume: Option<Expression> },

    /// `indirect[(invoked_by_fptr)]` on a declare-target directive.
    Indirect { invoked_by_fptr: Option<Expression> },

    /// `replayable[(replayable_expression)]`.
    Replayable {
        replayable_expression: Option<Expression>,
    },

    /// `safesync[(width)]` on a parallel directive.
    Safesync { width: Option<Expression> },

    /// `transparent[(impex_type)]` on a task-generating directive.
    Transparent { impex_type: Option<Expression> },

    // ========================================================================
    // Directive-name list clauses (OpenMP 5.1 assume/assumes)
    // ========================================================================
    /// `absent(directive-name-list)` - list of directives assumed to be absent.
    Absent {
        directives: Vec<crate::ast::OmpDirectiveKind>,
    },
    /// `contains(directive-name-list)` - list of directives assumed to appear.
    Contains {
        directives: Vec<crate::ast::OmpDirectiveKind>,
    },

    // ========================================================================
    // Argument-adjustment clauses
    // ========================================================================
    /// `adjust_args(adjust-op: parameter-list)` on `declare_variant`.
    AdjustArgs {
        operation: AdjustArgsModifier,
        parameters: Vec<OmpParameterListItem>,
    },

    /// `append_args(interop(init-modifier-list), ...)` on `declare_variant`.
    AppendArgs { operations: Vec<OmpAppendOperation> },

    /// `collector(expr)` for declare induction.
    Collector { expression: Expression },

    /// `inductor(expr)` for declare induction, with Fortran assignment
    /// statements distinguished from base-language expressions.
    Inductor { expression: OmpInductorExpression },

    /// `apply([loop-modifier:] applied-directive-list)`.
    Apply {
        loop_modifier: Option<OmpApplyLoopModifier>,
        applied_directives: Vec<OmpDirective>,
    },

    /// `induction([strict|relaxed,] step(expr), identifier: variable-list)`.
    Induction {
        modifier: Option<OmpInductionModifier>,
        step: Expression,
        identifier: OmpInductionIdentifier,
        items: Vec<ClauseItem>,
    },

    // ========================================================================
    // Data-sharing attribute clauses
    // ========================================================================
    /// `private(list)` - Variables are private to each thread
    Private { items: Vec<ClauseItem> },

    /// `firstprivate([saved:] list)`. The universal directive-name modifier is
    /// stored once on the enclosing [`crate::ast::OmpClause`].
    Firstprivate {
        modifier: Option<FirstprivateModifier>,
        items: Vec<ClauseItem>,
    },

    /// `lastprivate([modifier:] list)` - Variables updated from last iteration
    Lastprivate {
        modifier: Option<LastprivateModifier>,
        items: Vec<ClauseItem>,
    },

    /// `shared(list)` - Variables shared among all threads
    Shared { items: Vec<ClauseItem> },

    /// `default(shared|none|...)` - Default data-sharing attribute
    Default {
        category: Option<DefaultmapCategory>,
        kind: DefaultKind,
    },

    /// `defaultmap(behavior[:category])` - Default mapping semantics
    Defaultmap {
        behavior: DefaultmapBehavior,
        category: Option<DefaultmapCategory>,
    },

    // ========================================================================
    // Reduction clause
    // ========================================================================
    /// `reduction([modifier,]operator: list)` - Reduction operation
    Reduction {
        modifiers: Vec<ReductionModifier>,
        operator: OmpReductionIdentifier,
        items: Vec<ClauseItem>,
    },

    // ========================================================================
    // Device data clauses
    // ========================================================================
    /// `map([[mapper(id),] map-type:] list)` - Map variables to device
    Map {
        map_type: Option<MapType>,
        map_type_spelling: MapTypeSpelling,
        modifiers: Vec<MapModifier>,
        mapper: Option<crate::ast::OmpMapperId>,
        /// Optional iterator definitions (OpenMP 5.1)
        iterators: Vec<DependIterator>,
        locators: Vec<OmpLocator>,
    },

    /// `use_device_ptr(list)` - Use device pointers
    UseDevicePtr { items: Vec<ClauseItem> },

    /// `use_device_addr(list)` - Use device addresses
    UseDeviceAddr { items: Vec<ClauseItem> },

    /// `is_device_ptr(list)` - Variables are device pointers
    IsDevicePtr { items: Vec<ClauseItem> },

    /// `has_device_addr(list)` - Variables have device addresses
    HasDeviceAddr { items: Vec<ClauseItem> },

    // ========================================================================
    // Task clauses
    // ========================================================================
    /// `depend([modifier,] type: list)` - Task dependencies
    Depend {
        dependence: OmpDependence,
        /// Iterator definitions associated with the clause (OpenMP 5.1)
        iterators: Vec<DependIterator>,
    },

    /// `doacross(source|sink : iteration-specifier)`.
    Doacross {
        kind: DoacrossType,
        iteration: OmpDoacrossIteration,
    },

    /// `priority(expression)` - Task priority
    Priority { priority: Expression },

    /// `detach(event-handle)` with one checked variable designator.
    Detach { event: Variable },

    /// `affinity([iterator(...),] locator-list)` - Task affinity
    Affinity {
        iterators: Vec<DependIterator>,
        locators: Vec<OmpLocator>,
    },

    // ========================================================================
    // Loop scheduling clauses
    // ========================================================================
    /// `schedule([modifier [, modifier]:]kind[, chunk_size])` - Loop schedule
    Schedule {
        kind: ScheduleKind,
        modifiers: Vec<ScheduleModifier>,
        chunk_size: Option<Expression>,
    },

    /// `collapse(n)` - Collapse nested loops
    Collapse { n: Expression },

    /// `ordered[(n)]` - Ordered iterations
    Ordered { n: Option<Expression> },

    // ========================================================================
    // SIMD clauses
    // ========================================================================
    /// `linear(list[:step])` - Linear variables in SIMD
    Linear {
        modifier: Option<LinearModifier>,
        items: Vec<ClauseItem>,
        step: Option<Expression>,
        source_syntax: LinearSourceSyntax,
    },

    /// `aligned(list[:alignment])` - Aligned variables
    Aligned {
        items: Vec<ClauseItem>,
        alignment: Option<Expression>,
    },

    /// `safelen(length)` - Safe SIMD vector length
    Safelen { length: Expression },

    /// `simdlen(length)` - Preferred SIMD vector length
    Simdlen { length: Expression },

    // ========================================================================
    // Conditional clauses
    // ========================================================================
    /// `if(expression)` - Conditional execution. The directive-name modifier
    /// is stored once on the enclosing [`crate::ast::OmpClause`].
    If { condition: Expression },

    /// `threadset(omp_pool|omp_team)`.
    Threadset(ThreadsetKind),

    /// `memscope(all|cgroup|device)`.
    Memscope(MemscopeKind),

    /// `looprange(first, count)` on a `fuse` directive.
    Looprange {
        first: Expression,
        count: Expression,
    },

    /// `graph_reset[(condition)]`; a missing condition means true.
    GraphReset { condition: Option<Expression> },

    // ========================================================================
    // Thread binding clauses
    // ========================================================================
    /// `proc_bind(master|close|spread|primary)` - Thread affinity policy
    ProcBind(ProcBind),

    /// `bind(parallel|teams|thread|user)` - Loop binding
    Bind(BindModifier),

    /// `num_threads([strict:] nthreads-list)`.
    NumThreads {
        strict: bool,
        nthreads: Vec<Expression>,
    },

    // ========================================================================
    // Device clauses
    // ========================================================================
    /// `device(expression)` - Target device
    Device {
        modifier: Option<DeviceModifier>,
        device_num: Expression,
    },

    /// `device_type(host|nohost|any)` - Device type specifier
    DeviceType(DeviceType),

    /// `at(compilation|execution)` - Error location
    At(AtKind),

    /// `severity(fatal|warning)` - Error directive severity
    Severity(SeverityKind),

    /// Interoperability-object initialization.
    InitInterop {
        interop_types: Vec<OmpInteropType>,
        preferences: Vec<OmpPreferenceSpecification>,
        variable: Variable,
    },

    /// Depend-object initialization introduced by OpenMP 6.0.
    InitDepobj {
        dependence: DepobjUpdateDependence,
        locator: OmpLocator,
        variable: Variable,
    },

    // ========================================================================
    // Atomic clauses
    // ========================================================================
    /// `fail(memory-order)` for atomic compare fail behavior
    Fail { order: MemoryOrder },

    /// Memory-order clause with its OpenMP 6.0 semantic condition.
    MemoryOrder {
        order: MemoryOrder,
        use_semantics: Option<Expression>,
    },

    /// Atomic operation modifier with its OpenMP 6.0 semantic condition.
    AtomicOperation {
        op: AtomicOp,
        use_semantics: Option<Expression>,
    },

    /// `capture`, `compare`, or `weak`, optionally with OpenMP 6.0
    /// `use_semantics`.
    ExtendedAtomic {
        kind: ExtendedAtomicKind,
        use_semantics: Option<Expression>,
    },

    // ========================================================================
    // Order clause
    // ========================================================================
    /// `order([modifier:]concurrent)` - Iteration execution order
    Order {
        modifier: Option<OrderModifier>,
        kind: OrderKind,
    },

    // ========================================================================
    // Teams clauses
    // ========================================================================
    /// `num_teams([lower-bound:] upper-bound)`.
    NumTeams {
        lower_bound: Option<Expression>,
        upper_bound: Expression,
    },

    /// `thread_limit(expression)` - Thread limit per team
    ThreadLimit { limit: Expression },

    // ========================================================================
    // Allocator clauses
    // ========================================================================
    /// `allocate([allocator-expression:] list)` or the OpenMP 5.1+
    /// `allocate([allocator(expr),] [align(expr):] list)` form.
    Allocate {
        allocator: Option<Expression>,
        alignment: Option<Expression>,
        items: Vec<ClauseItem>,
        source_syntax: AllocateSourceSyntax,
    },

    /// `allocator(allocator-expression)` - Specify allocator.
    Allocator { allocator: Expression },

    // ========================================================================
    // Other clauses
    // ========================================================================
    /// `copyin(list)` - Copy master thread value to team threads
    Copyin { items: Vec<ClauseItem> },

    /// `copyprivate(list)` - Broadcast value from one thread
    Copyprivate { items: Vec<ClauseItem> },

    /// `dist_schedule(kind[, chunk_size])` - Distribute schedule
    DistSchedule {
        kind: ScheduleKind,
        chunk_size: Option<Expression>,
    },

    /// `grainsize(expression)` - Taskloop grainsize
    Grainsize {
        modifier: Option<GrainsizeModifier>,
        grain: Expression,
    },

    /// `num_tasks(expression)` - Number of tasks
    NumTasks {
        modifier: Option<NumTasksModifier>,
        num: Expression,
    },

    /// `filter(thread-num)` - Thread filter for masked construct
    Filter { thread_num: Expression },

    /// `uses_allocators(list)` - Allocator selection
    UsesAllocators { allocators: Vec<UsesAllocatorSpec> },

    /// One requirement clause on a `requires` directive. OpenMP 6.0 permits
    /// an optional `required` expression on feature requirements.
    Requirement {
        requirement: RequireModifier,
        required: Option<Expression>,
    },

    /// `update([task-dependence-type:] update-var)` on `depobj`.
    ///
    /// The update variable may be omitted only by the historical
    /// `depobj(depend-object) update(task-dependence-type)` form, where the
    /// directive argument supplies it.
    DepobjUpdate {
        dependence: DepobjUpdateDependence,
        variable: Option<Variable>,
    },

    /// Metadirective/variant selector with fully typed payload.
    MetadirectiveSelector {
        selector: Box<crate::ast::OmpSelector>,
    },
}

impl ClauseData {
    /// Check if this is a default clause
    pub fn is_default(&self) -> bool {
        matches!(self, ClauseData::Default { .. })
    }

    /// Check if this is a private clause
    pub fn is_private(&self) -> bool {
        matches!(self, ClauseData::Private { .. })
    }

    /// Check if this is a firstprivate clause
    pub fn is_firstprivate(&self) -> bool {
        matches!(self, ClauseData::Firstprivate { .. })
    }

    /// Check if this is a lastprivate clause
    pub fn is_lastprivate(&self) -> bool {
        matches!(self, ClauseData::Lastprivate { .. })
    }

    /// Check if this is a shared clause
    pub fn is_shared(&self) -> bool {
        matches!(self, ClauseData::Shared { .. })
    }

    /// Check if this is a reduction clause
    pub fn is_reduction(&self) -> bool {
        matches!(self, ClauseData::Reduction { .. })
    }

    /// Check if this is a map clause
    pub fn is_map(&self) -> bool {
        matches!(self, ClauseData::Map { .. })
    }

    /// Check if this is an if clause
    pub fn is_if(&self) -> bool {
        matches!(self, ClauseData::If { .. })
    }

    /// Check if this is a num_threads clause
    pub fn is_num_threads(&self) -> bool {
        matches!(self, ClauseData::NumThreads { .. })
    }

    /// Check if this is a collapse clause
    pub fn is_collapse(&self) -> bool {
        matches!(self, ClauseData::Collapse { .. })
    }

    /// Check if this is an ordered clause
    pub fn is_ordered(&self) -> bool {
        matches!(self, ClauseData::Ordered { .. })
    }

    /// Check if this is a schedule clause
    pub fn is_schedule(&self) -> bool {
        matches!(self, ClauseData::Schedule { .. })
    }

    /// Check if this is a device clause
    pub fn is_device(&self) -> bool {
        matches!(self, ClauseData::Device { .. })
    }

    /// Check if this is a depend clause
    pub fn is_depend(&self) -> bool {
        matches!(self, ClauseData::Depend { .. })
    }

    /// Check if this is a linear clause
    pub fn is_linear(&self) -> bool {
        matches!(self, ClauseData::Linear { .. })
    }

    /// Check if this is a proc_bind clause
    pub fn is_proc_bind(&self) -> bool {
        matches!(self, ClauseData::ProcBind(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_variable(source: &str) -> Variable {
        Variable::parse(source, &crate::ir::ParserConfig::c()).expect("valid test variable")
    }

    // Test MapType
    #[test]
    fn test_map_type_display() {
        assert_eq!(MapType::To.to_string(), "to");
        assert_eq!(MapType::From.to_string(), "from");
        assert_eq!(MapType::ToFrom.to_string(), "tofrom");
        assert_eq!(MapType::Storage.to_string(), "storage");
    }

    #[test]
    fn test_map_type_all_variants() {
        // Ensure all variants are covered
        let all_types = vec![
            MapType::To,
            MapType::From,
            MapType::ToFrom,
            MapType::Storage,
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
        assert_eq!(DependType::Inoutset.to_string(), "inoutset");
    }

    #[test]
    fn test_depend_type_all_variants() {
        let all_types = vec![
            DependType::In,
            DependType::Out,
            DependType::Inout,
            DependType::Mutexinoutset,
            DependType::Inoutset,
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
        assert_eq!(ProcBind::Close.to_string(), "close");
        assert_eq!(ProcBind::Spread.to_string(), "spread");
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
        let all_orders = [
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
    }

    #[test]
    fn test_atomic_op_all_variants() {
        let all_ops = vec![AtomicOp::Read, AtomicOp::Write, AtomicOp::Update];
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

    // ========================================================================
    // ClauseItem tests
    // ========================================================================

    #[test]
    fn test_clause_item_from_identifier() {
        let id = Identifier::new("x").expect("valid identifier");
        let item = ClauseItem::from(id);
        assert_eq!(item.to_string(), "x");
    }

    #[test]
    fn test_clause_item_from_variable() {
        let var = test_variable("arr");
        let item = ClauseItem::from(var);
        assert_eq!(item.to_string(), "arr");
    }

    #[test]
    fn test_clause_item_from_expression() {
        use crate::ir::ParserConfig;
        let config = ParserConfig::c();
        let expr = Expression::new("n > 100", &config).unwrap();
        let item = ClauseItem::from(expr);
        assert_eq!(item.to_string(), "n > 100");
    }

    #[test]
    fn test_clause_item_display_identifier() {
        let item = ClauseItem::Identifier(Identifier::new("my_var").expect("valid identifier"));
        assert_eq!(item.to_string(), "my_var");
    }

    #[test]
    fn test_clause_item_display_variable_with_section() {
        let var = test_variable("arr[i]");
        let item = ClauseItem::Variable(var);
        assert_eq!(item.to_string(), "arr[i]");
    }

    #[test]
    fn test_clause_item_equality() {
        let item1 = ClauseItem::Identifier(Identifier::new("x").expect("valid identifier"));
        let item2 = ClauseItem::Identifier(Identifier::new("x").expect("valid identifier"));
        let item3 = ClauseItem::Identifier(Identifier::new("y").expect("valid identifier"));
        assert_eq!(item1, item2);
        assert_ne!(item1, item3);
    }

    #[test]
    fn test_clause_item_clone() {
        let item1 = ClauseItem::Identifier(Identifier::new("x").expect("valid identifier"));
        let item2 = item1.clone();
        assert_eq!(item1, item2);
    }

    #[test]
    fn test_clause_data_equality() {
        let clause1 = ClauseData::Default {
            category: None,
            kind: DefaultKind::Shared,
        };
        let clause2 = ClauseData::Default {
            category: None,
            kind: DefaultKind::Shared,
        };
        let clause3 = ClauseData::Default {
            category: None,
            kind: DefaultKind::None,
        };
        assert_eq!(clause1, clause2);
        assert_ne!(clause1, clause3);
    }

    #[test]
    fn test_clause_data_clone() {
        let items = vec![ClauseItem::Identifier(
            Identifier::new("x").expect("valid identifier"),
        )];
        let clause1 = ClauseData::Private { items };
        let clause2 = clause1.clone();
        assert_eq!(clause1, clause2);
    }

    // Corner case: debug formatting
    #[test]
    fn test_clause_data_debug() {
        let clause = ClauseData::Default {
            category: None,
            kind: DefaultKind::Shared,
        };
        let debug_str = format!("{clause:?}");
        assert!(debug_str.contains("Default"));
        assert!(debug_str.contains("Shared"));
    }
}
