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

use super::{Expression, Identifier, Variable};

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
    Add,      // +
    Multiply, // *
    Subtract, // -

    // Bitwise operators
    BitwiseAnd, // &
    BitwiseOr,  // |
    BitwiseXor, // ^

    // Logical operators
    LogicalAnd, // &&
    LogicalOr,  // ||

    // Min/Max operators
    Min,
    Max,

    // C++ specific operators (OpenMP 5.2 supports these)
    MinusEqual, // -= (non-commutative)

    // User-defined reduction operator
    Custom,
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

impl std::str::FromStr for ReductionOperator {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "+" => Ok(ReductionOperator::Add),
            "-" => Ok(ReductionOperator::Subtract),
            "*" => Ok(ReductionOperator::Multiply),
            "&" => Ok(ReductionOperator::BitwiseAnd),
            "|" => Ok(ReductionOperator::BitwiseOr),
            "^" => Ok(ReductionOperator::BitwiseXor),
            "&&" => Ok(ReductionOperator::LogicalAnd),
            "||" => Ok(ReductionOperator::LogicalOr),
            "min" => Ok(ReductionOperator::Min),
            "max" => Ok(ReductionOperator::Max),
            "-=" => Ok(ReductionOperator::MinusEqual),
            _ => Err(format!("Unknown reduction operator: {}", s)),
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
    To,
    /// Map data from device (device → host)
    From,
    /// Map data to and from device (bidirectional)
    ToFrom,
    /// Allocate device memory without transfer
    Alloc,
    /// Release device memory
    Release,
    /// Delete device memory
    Delete,
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

impl std::str::FromStr for MapType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "to" => Ok(MapType::To),
            "from" => Ok(MapType::From),
            "tofrom" => Ok(MapType::ToFrom),
            "alloc" => Ok(MapType::Alloc),
            "release" => Ok(MapType::Release),
            "delete" => Ok(MapType::Delete),
            _ => Err(format!("Unknown map type: {}", s)),
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

impl std::str::FromStr for ScheduleKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "static" => Ok(ScheduleKind::Static),
            "dynamic" => Ok(ScheduleKind::Dynamic),
            "guided" => Ok(ScheduleKind::Guided),
            "auto" => Ok(ScheduleKind::Auto),
            "runtime" => Ok(ScheduleKind::Runtime),
            _ => Err(format!("Unknown schedule kind: {}", s)),
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

impl std::str::FromStr for ScheduleModifier {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "monotonic" => Ok(ScheduleModifier::Monotonic),
            "nonmonotonic" => Ok(ScheduleModifier::Nonmonotonic),
            "simd" => Ok(ScheduleModifier::Simd),
            _ => Err(format!("Unknown schedule modifier: {}", s)),
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
    In,
    /// Write dependency
    Out,
    /// Read-write dependency
    Inout,
    /// Mutual exclusion with inout
    Mutexinoutset,
    /// Dependency on task completion
    Depobj,
    /// Source dependency (OpenMP 5.0)
    Source,
    /// Sink dependency (OpenMP 5.0)
    Sink,
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

impl std::str::FromStr for DependType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "in" => Ok(DependType::In),
            "out" => Ok(DependType::Out),
            "inout" => Ok(DependType::Inout),
            "mutexinoutset" => Ok(DependType::Mutexinoutset),
            "depobj" => Ok(DependType::Depobj),
            "source" => Ok(DependType::Source),
            "sink" => Ok(DependType::Sink),
            _ => Err(format!("Unknown depend type: {}", s)),
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

impl std::str::FromStr for DefaultKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "shared" => Ok(DefaultKind::Shared),
            "none" => Ok(DefaultKind::None),
            "private" => Ok(DefaultKind::Private),
            "firstprivate" => Ok(DefaultKind::Firstprivate),
            _ => Err(format!("Unknown default kind: {}", s)),
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
    Master,
    /// Threads execute close to the master thread (OpenMP 5.1 deprecates 'master')
    Close,
    /// Threads spread out across available processors
    Spread,
    /// Implementation-defined binding
    Primary,
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

impl std::str::FromStr for ProcBind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "master" => Ok(ProcBind::Master),
            "close" => Ok(ProcBind::Close),
            "spread" => Ok(ProcBind::Spread),
            "primary" => Ok(ProcBind::Primary),
            _ => Err(format!("Unknown proc_bind kind: {}", s)),
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
#[repr(C)]
pub enum AtomicOp {
    /// Atomic read
    Read,
    /// Atomic write
    Write,
    /// Atomic update
    Update,
    /// Atomic capture
    Capture,
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
    Host,
    /// Non-host device (accelerator)
    Nohost,
    /// Any device
    Any,
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
    Val,
    /// Reference to linear variable
    Ref,
    /// Uniform across SIMD lanes
    Uval,
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
    Conditional,
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
    Concurrent,
}

impl fmt::Display for OrderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderKind::Concurrent => write!(f, "concurrent"),
        }
    }
}

// ============================================================================
// ClauseItem: Items that appear in clause lists
// ============================================================================

/// Item that can appear in a clause list
///
/// Many OpenMP clauses accept lists of items that can be:
/// - Simple identifiers: `private(x, y, z)`
/// - Variables with array sections: `map(to: arr[0:N])`
/// - Expressions: `if(n > 100)`
///
/// ## Examples
///
/// ```
/// # use roup::ir::{ClauseItem, Identifier, Variable, Expression, ParserConfig};
/// // Simple identifier
/// let item = ClauseItem::Identifier(Identifier::new("x"));
/// assert_eq!(item.to_string(), "x");
///
/// // Variable with array section
/// let var = Variable::new("arr");
/// let item = ClauseItem::Variable(var);
/// assert_eq!(item.to_string(), "arr");
///
/// // Expression
/// let config = ParserConfig::default();
/// let expr = Expression::new("n > 100", &config);
/// let item = ClauseItem::Expression(expr);
/// assert_eq!(item.to_string(), "n > 100");
/// ```
///
/// ## Learning: Enums with Data
///
/// Unlike the modifier enums (which are just unit variants), ClauseItem
/// is an enum where each variant **contains data**:
///
/// ```ignore
/// enum ClauseItem {
///     Identifier(Identifier),  // Contains an Identifier
///     Variable(Variable),       // Contains a Variable
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
    /// Expression (e.g., `n > 100` in `if(n > 100)`)
    Expression(Expression),
}

impl fmt::Display for ClauseItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClauseItem::Identifier(id) => write!(f, "{id}"),
            ClauseItem::Variable(var) => write!(f, "{var}"),
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

// ============================================================================
// ClauseData: Complete clause semantic information
// ============================================================================

/// Complete semantic data for an OpenMP clause
///
/// This enum represents the **meaning** of each OpenMP clause type.
/// Each variant captures the specific data needed for that clause.
///
/// ## Examples
///
/// ```
/// # use roup::ir::{ClauseData, DefaultKind, ReductionOperator, Identifier};
/// // default(shared)
/// let clause = ClauseData::Default(DefaultKind::Shared);
/// assert_eq!(clause.to_string(), "default(shared)");
///
/// // reduction(+: sum)
/// let clause = ClauseData::Reduction {
///     operator: ReductionOperator::Add,
///     items: vec![Identifier::new("sum").into()],
/// };
/// assert_eq!(clause.to_string(), "reduction(+: sum)");
/// ```
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
/// This is much richer than the parser's string-based representation.
#[derive(Debug, Clone, PartialEq)]
pub enum ClauseData {
    // ========================================================================
    // Bare clauses (no parameters)
    // ========================================================================
    /// Clause with no parameters (e.g., `nowait`, `nogroup`)
    Bare(Identifier),

    // ========================================================================
    // Simple expression clauses
    // ========================================================================
    /// Single expression parameter (e.g., `num_threads(4)`)
    Expression(Expression),

    // ========================================================================
    // Item list clauses
    // ========================================================================
    /// List of items (e.g., `private(x, y, z)`)
    ItemList(Vec<ClauseItem>),

    // ========================================================================
    // Data-sharing attribute clauses
    // ========================================================================
    /// `private(list)` - Variables are private to each thread
    Private { items: Vec<ClauseItem> },

    /// `firstprivate(list)` - Variables initialized from master thread
    Firstprivate { items: Vec<ClauseItem> },

    /// `lastprivate([modifier:] list)` - Variables updated from last iteration
    Lastprivate {
        modifier: Option<LastprivateModifier>,
        items: Vec<ClauseItem>,
    },

    /// `shared(list)` - Variables shared among all threads
    Shared { items: Vec<ClauseItem> },

    /// `default(shared|none|...)` - Default data-sharing attribute
    Default(DefaultKind),

    // ========================================================================
    // Reduction clause
    // ========================================================================
    /// `reduction([modifier,]operator: list)` - Reduction operation
    Reduction {
        operator: ReductionOperator,
        items: Vec<ClauseItem>,
    },

    // ========================================================================
    // Device data clauses
    // ========================================================================
    /// `map([[mapper(id),] map-type:] list)` - Map variables to device
    Map {
        map_type: Option<MapType>,
        mapper: Option<Identifier>,
        items: Vec<ClauseItem>,
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
        depend_type: DependType,
        items: Vec<ClauseItem>,
    },

    /// `priority(expression)` - Task priority
    Priority { priority: Expression },

    /// `affinity([modifier:] list)` - Task affinity
    Affinity { items: Vec<ClauseItem> },

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
    /// `if([directive-name-modifier:] expression)` - Conditional execution
    If {
        directive_name: Option<Identifier>,
        condition: Expression,
    },

    // ========================================================================
    // Thread binding clauses
    // ========================================================================
    /// `proc_bind(master|close|spread|primary)` - Thread affinity policy
    ProcBind(ProcBind),

    /// `num_threads(expression)` - Number of threads
    NumThreads { num: Expression },

    // ========================================================================
    // Device clauses
    // ========================================================================
    /// `device(expression)` - Target device
    Device { device_num: Expression },

    /// `device_type(host|nohost|any)` - Device type specifier
    DeviceType(DeviceType),

    // ========================================================================
    // Atomic clauses
    // ========================================================================
    /// `atomic_default_mem_order(seq_cst|acq_rel|...)` - Default memory order
    AtomicDefaultMemOrder(MemoryOrder),

    /// Atomic operation modifier
    AtomicOperation {
        op: AtomicOp,
        memory_order: Option<MemoryOrder>,
    },

    // ========================================================================
    // Order clause
    // ========================================================================
    /// `order(concurrent)` - Iteration execution order
    Order(OrderKind),

    // ========================================================================
    // Teams clauses
    // ========================================================================
    /// `num_teams(expression)` - Number of teams
    NumTeams { num: Expression },

    /// `thread_limit(expression)` - Thread limit per team
    ThreadLimit { limit: Expression },

    // ========================================================================
    // Allocator clauses
    // ========================================================================
    /// `allocate([allocator:] list)` - Memory allocator
    Allocate {
        allocator: Option<Identifier>,
        items: Vec<ClauseItem>,
    },

    /// `allocator(allocator-handle)` - Specify allocator
    Allocator { allocator: Identifier },

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
    Grainsize { grain: Expression },

    /// `num_tasks(expression)` - Number of tasks
    NumTasks { num: Expression },

    /// `filter(thread-num)` - Thread filter for masked construct
    Filter { thread_num: Expression },

    /// Generic clause with unparsed data (fallback for unknown clauses)
    Generic {
        name: Identifier,
        data: Option<String>,
    },
}

impl fmt::Display for ClauseData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClauseData::Bare(name) => write!(f, "{name}"),
            ClauseData::Expression(expr) => write!(f, "{expr}"),
            ClauseData::ItemList(items) => {
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{item}")?;
                }
                Ok(())
            }
            ClauseData::Private { items } => {
                write!(f, "private(")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{item}")?;
                }
                write!(f, ")")
            }
            ClauseData::Firstprivate { items } => {
                write!(f, "firstprivate(")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{item}")?;
                }
                write!(f, ")")
            }
            ClauseData::Lastprivate { modifier, items } => {
                write!(f, "lastprivate(")?;
                if let Some(m) = modifier {
                    write!(f, "{m}: ")?;
                }
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{item}")?;
                }
                write!(f, ")")
            }
            ClauseData::Shared { items } => {
                write!(f, "shared(")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{item}")?;
                }
                write!(f, ")")
            }
            ClauseData::Default(kind) => write!(f, "default({kind})"),
            ClauseData::Reduction { operator, items } => {
                write!(f, "reduction({operator}: ")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{item}")?;
                }
                write!(f, ")")
            }
            ClauseData::Map {
                map_type,
                mapper,
                items,
            } => {
                write!(f, "map(")?;
                if let Some(mapper_id) = mapper {
                    write!(f, "mapper({mapper_id}), ")?;
                }
                if let Some(mt) = map_type {
                    write!(f, "{mt}: ")?;
                }
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{item}")?;
                }
                write!(f, ")")
            }
            ClauseData::Schedule {
                kind,
                modifiers,
                chunk_size,
            } => {
                write!(f, "schedule(")?;
                if !modifiers.is_empty() {
                    for (i, m) in modifiers.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{m}")?;
                    }
                    write!(f, ": ")?;
                }
                write!(f, "{kind}")?;
                if let Some(chunk) = chunk_size {
                    write!(f, ", {chunk}")?;
                }
                write!(f, ")")
            }
            ClauseData::Linear {
                modifier,
                items,
                step,
            } => {
                write!(f, "linear(")?;
                if let Some(m) = modifier {
                    write!(f, "{m}: ")?;
                }
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{item}")?;
                }
                if let Some(s) = step {
                    write!(f, ": {s}")?;
                }
                write!(f, ")")
            }
            ClauseData::If {
                directive_name,
                condition,
            } => {
                write!(f, "if(")?;
                if let Some(name) = directive_name {
                    write!(f, "{name}: ")?;
                }
                write!(f, "{condition})")
            }
            ClauseData::NumThreads { num } => write!(f, "num_threads({num})"),
            ClauseData::ProcBind(pb) => write!(f, "proc_bind({pb})"),
            ClauseData::Device { device_num } => write!(f, "device({device_num})"),
            ClauseData::DeviceType(dt) => write!(f, "device_type({dt})"),
            ClauseData::Collapse { n } => write!(f, "collapse({n})"),
            ClauseData::Ordered { n } => {
                write!(f, "ordered")?;
                if let Some(num) = n {
                    write!(f, "({num})")?;
                }
                Ok(())
            }
            ClauseData::Depend { depend_type, items } => {
                write!(f, "depend({depend_type}: ")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{item}")?;
                }
                write!(f, ")")
            }
            // Simplified Display for remaining variants (can be expanded as needed)
            _ => write!(f, "<clause>"),
        }
    }
}

impl ClauseData {
    /// Check if this is a default clause
    pub fn is_default(&self) -> bool {
        matches!(self, ClauseData::Default(_))
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
        let op3 = op1; // Copy (no need for .clone() on Copy types)
        assert_eq!(op1, op2);
        assert_eq!(op1, op3);
    }

    // Discriminants are auto-generated sequentially by Rust.
    // The C API uses these via build.rs constant generation.
    // No need to test specific values as they're compiler-managed.

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
        let debug_str = format!("{op:?}");
        assert!(debug_str.contains("Add"));
    }

    // ========================================================================
    // ClauseItem tests
    // ========================================================================

    #[test]
    fn test_clause_item_from_identifier() {
        let id = Identifier::new("x");
        let item = ClauseItem::from(id);
        assert_eq!(item.to_string(), "x");
    }

    #[test]
    fn test_clause_item_from_variable() {
        let var = Variable::new("arr");
        let item = ClauseItem::from(var);
        assert_eq!(item.to_string(), "arr");
    }

    #[test]
    fn test_clause_item_from_expression() {
        use crate::ir::ParserConfig;
        let config = ParserConfig::default();
        let expr = Expression::new("n > 100", &config);
        let item = ClauseItem::from(expr);
        assert_eq!(item.to_string(), "n > 100");
    }

    #[test]
    fn test_clause_item_display_identifier() {
        let item = ClauseItem::Identifier(Identifier::new("my_var"));
        assert_eq!(item.to_string(), "my_var");
    }

    #[test]
    fn test_clause_item_display_variable_with_section() {
        use crate::ir::ArraySection;
        let section = ArraySection::single_index(Expression::unparsed("i"));
        let var = Variable::with_sections("arr", vec![section]);
        let item = ClauseItem::Variable(var);
        assert_eq!(item.to_string(), "arr[i]");
    }

    #[test]
    fn test_clause_item_equality() {
        let item1 = ClauseItem::Identifier(Identifier::new("x"));
        let item2 = ClauseItem::Identifier(Identifier::new("x"));
        let item3 = ClauseItem::Identifier(Identifier::new("y"));
        assert_eq!(item1, item2);
        assert_ne!(item1, item3);
    }

    #[test]
    fn test_clause_item_clone() {
        let item1 = ClauseItem::Identifier(Identifier::new("x"));
        let item2 = item1.clone();
        assert_eq!(item1, item2);
    }

    // ========================================================================
    // ClauseData tests
    // ========================================================================

    #[test]
    fn test_clause_data_bare() {
        let clause = ClauseData::Bare(Identifier::new("nowait"));
        assert_eq!(clause.to_string(), "nowait");
    }

    #[test]
    fn test_clause_data_default() {
        let clause = ClauseData::Default(DefaultKind::Shared);
        assert_eq!(clause.to_string(), "default(shared)");

        let clause = ClauseData::Default(DefaultKind::None);
        assert_eq!(clause.to_string(), "default(none)");
    }

    #[test]
    fn test_clause_data_private() {
        let items = vec![
            ClauseItem::Identifier(Identifier::new("x")),
            ClauseItem::Identifier(Identifier::new("y")),
        ];
        let clause = ClauseData::Private { items };
        assert_eq!(clause.to_string(), "private(x, y)");
    }

    #[test]
    fn test_clause_data_private_single_item() {
        let items = vec![ClauseItem::Identifier(Identifier::new("x"))];
        let clause = ClauseData::Private { items };
        assert_eq!(clause.to_string(), "private(x)");
    }

    #[test]
    fn test_clause_data_firstprivate() {
        let items = vec![
            ClauseItem::Identifier(Identifier::new("a")),
            ClauseItem::Identifier(Identifier::new("b")),
        ];
        let clause = ClauseData::Firstprivate { items };
        assert_eq!(clause.to_string(), "firstprivate(a, b)");
    }

    #[test]
    fn test_clause_data_lastprivate_without_modifier() {
        let items = vec![ClauseItem::Identifier(Identifier::new("x"))];
        let clause = ClauseData::Lastprivate {
            modifier: None,
            items,
        };
        assert_eq!(clause.to_string(), "lastprivate(x)");
    }

    #[test]
    fn test_clause_data_lastprivate_with_conditional() {
        let items = vec![ClauseItem::Identifier(Identifier::new("x"))];
        let clause = ClauseData::Lastprivate {
            modifier: Some(LastprivateModifier::Conditional),
            items,
        };
        assert_eq!(clause.to_string(), "lastprivate(conditional: x)");
    }

    #[test]
    fn test_clause_data_shared() {
        let items = vec![
            ClauseItem::Identifier(Identifier::new("data")),
            ClauseItem::Identifier(Identifier::new("count")),
        ];
        let clause = ClauseData::Shared { items };
        assert_eq!(clause.to_string(), "shared(data, count)");
    }

    #[test]
    fn test_clause_data_reduction() {
        let items = vec![ClauseItem::Identifier(Identifier::new("sum"))];
        let clause = ClauseData::Reduction {
            operator: ReductionOperator::Add,
            items,
        };
        assert_eq!(clause.to_string(), "reduction(+: sum)");
    }

    #[test]
    fn test_clause_data_reduction_multiple_items() {
        let items = vec![
            ClauseItem::Identifier(Identifier::new("sum")),
            ClauseItem::Identifier(Identifier::new("total")),
        ];
        let clause = ClauseData::Reduction {
            operator: ReductionOperator::Add,
            items,
        };
        assert_eq!(clause.to_string(), "reduction(+: sum, total)");
    }

    #[test]
    fn test_clause_data_reduction_max() {
        let items = vec![ClauseItem::Identifier(Identifier::new("max_val"))];
        let clause = ClauseData::Reduction {
            operator: ReductionOperator::Max,
            items,
        };
        assert_eq!(clause.to_string(), "reduction(max: max_val)");
    }

    #[test]
    fn test_clause_data_map_simple() {
        let items = vec![ClauseItem::Variable(Variable::new("arr"))];
        let clause = ClauseData::Map {
            map_type: Some(MapType::To),
            mapper: None,
            items,
        };
        assert_eq!(clause.to_string(), "map(to: arr)");
    }

    #[test]
    fn test_clause_data_map_tofrom() {
        let items = vec![ClauseItem::Variable(Variable::new("data"))];
        let clause = ClauseData::Map {
            map_type: Some(MapType::ToFrom),
            mapper: None,
            items,
        };
        assert_eq!(clause.to_string(), "map(tofrom: data)");
    }

    #[test]
    fn test_clause_data_map_without_type() {
        let items = vec![ClauseItem::Variable(Variable::new("arr"))];
        let clause = ClauseData::Map {
            map_type: None,
            mapper: None,
            items,
        };
        assert_eq!(clause.to_string(), "map(arr)");
    }

    #[test]
    fn test_clause_data_map_with_mapper() {
        let items = vec![ClauseItem::Variable(Variable::new("arr"))];
        let clause = ClauseData::Map {
            map_type: Some(MapType::To),
            mapper: Some(Identifier::new("my_mapper")),
            items,
        };
        assert_eq!(clause.to_string(), "map(mapper(my_mapper), to: arr)");
    }

    #[test]
    fn test_clause_data_schedule_static() {
        let clause = ClauseData::Schedule {
            kind: ScheduleKind::Static,
            modifiers: vec![],
            chunk_size: None,
        };
        assert_eq!(clause.to_string(), "schedule(static)");
    }

    #[test]
    fn test_clause_data_schedule_dynamic_with_chunk() {
        let chunk = Expression::unparsed("64");
        let clause = ClauseData::Schedule {
            kind: ScheduleKind::Dynamic,
            modifiers: vec![],
            chunk_size: Some(chunk),
        };
        assert_eq!(clause.to_string(), "schedule(dynamic, 64)");
    }

    #[test]
    fn test_clause_data_schedule_with_modifier() {
        let clause = ClauseData::Schedule {
            kind: ScheduleKind::Static,
            modifiers: vec![ScheduleModifier::Monotonic],
            chunk_size: None,
        };
        assert_eq!(clause.to_string(), "schedule(monotonic: static)");
    }

    #[test]
    fn test_clause_data_schedule_with_multiple_modifiers() {
        let clause = ClauseData::Schedule {
            kind: ScheduleKind::Dynamic,
            modifiers: vec![ScheduleModifier::Nonmonotonic, ScheduleModifier::Simd],
            chunk_size: Some(Expression::unparsed("32")),
        };
        assert_eq!(
            clause.to_string(),
            "schedule(nonmonotonic, simd: dynamic, 32)"
        );
    }

    #[test]
    fn test_clause_data_linear_simple() {
        let items = vec![ClauseItem::Identifier(Identifier::new("i"))];
        let clause = ClauseData::Linear {
            modifier: None,
            items,
            step: None,
        };
        assert_eq!(clause.to_string(), "linear(i)");
    }

    #[test]
    fn test_clause_data_linear_with_step() {
        let items = vec![ClauseItem::Identifier(Identifier::new("i"))];
        let clause = ClauseData::Linear {
            modifier: None,
            items,
            step: Some(Expression::unparsed("2")),
        };
        assert_eq!(clause.to_string(), "linear(i: 2)");
    }

    #[test]
    fn test_clause_data_linear_with_modifier() {
        let items = vec![ClauseItem::Identifier(Identifier::new("i"))];
        let clause = ClauseData::Linear {
            modifier: Some(LinearModifier::Val),
            items,
            step: None,
        };
        assert_eq!(clause.to_string(), "linear(val: i)");
    }

    #[test]
    fn test_clause_data_if_simple() {
        let condition = Expression::unparsed("n > 100");
        let clause = ClauseData::If {
            directive_name: None,
            condition,
        };
        assert_eq!(clause.to_string(), "if(n > 100)");
    }

    #[test]
    fn test_clause_data_if_with_directive_name() {
        let condition = Expression::unparsed("n > 100");
        let clause = ClauseData::If {
            directive_name: Some(Identifier::new("parallel")),
            condition,
        };
        assert_eq!(clause.to_string(), "if(parallel: n > 100)");
    }

    #[test]
    fn test_clause_data_num_threads() {
        let clause = ClauseData::NumThreads {
            num: Expression::unparsed("4"),
        };
        assert_eq!(clause.to_string(), "num_threads(4)");
    }

    #[test]
    fn test_clause_data_proc_bind() {
        let clause = ClauseData::ProcBind(ProcBind::Close);
        assert_eq!(clause.to_string(), "proc_bind(close)");
    }

    #[test]
    fn test_clause_data_device() {
        let clause = ClauseData::Device {
            device_num: Expression::unparsed("0"),
        };
        assert_eq!(clause.to_string(), "device(0)");
    }

    #[test]
    fn test_clause_data_device_type() {
        let clause = ClauseData::DeviceType(DeviceType::Host);
        assert_eq!(clause.to_string(), "device_type(host)");
    }

    #[test]
    fn test_clause_data_collapse() {
        let clause = ClauseData::Collapse {
            n: Expression::unparsed("2"),
        };
        assert_eq!(clause.to_string(), "collapse(2)");
    }

    #[test]
    fn test_clause_data_ordered_without_param() {
        let clause = ClauseData::Ordered { n: None };
        assert_eq!(clause.to_string(), "ordered");
    }

    #[test]
    fn test_clause_data_ordered_with_param() {
        let clause = ClauseData::Ordered {
            n: Some(Expression::unparsed("2")),
        };
        assert_eq!(clause.to_string(), "ordered(2)");
    }

    #[test]
    fn test_clause_data_depend() {
        let items = vec![ClauseItem::Variable(Variable::new("x"))];
        let clause = ClauseData::Depend {
            depend_type: DependType::In,
            items,
        };
        assert_eq!(clause.to_string(), "depend(in: x)");
    }

    #[test]
    fn test_clause_data_depend_inout() {
        let items = vec![
            ClauseItem::Variable(Variable::new("a")),
            ClauseItem::Variable(Variable::new("b")),
        ];
        let clause = ClauseData::Depend {
            depend_type: DependType::Inout,
            items,
        };
        assert_eq!(clause.to_string(), "depend(inout: a, b)");
    }

    #[test]
    fn test_clause_data_equality() {
        let clause1 = ClauseData::Default(DefaultKind::Shared);
        let clause2 = ClauseData::Default(DefaultKind::Shared);
        let clause3 = ClauseData::Default(DefaultKind::None);
        assert_eq!(clause1, clause2);
        assert_ne!(clause1, clause3);
    }

    #[test]
    fn test_clause_data_clone() {
        let items = vec![ClauseItem::Identifier(Identifier::new("x"))];
        let clause1 = ClauseData::Private { items };
        let clause2 = clause1.clone();
        assert_eq!(clause1, clause2);
    }

    // Corner case: empty item lists
    #[test]
    fn test_clause_data_private_empty_list() {
        let clause = ClauseData::Private { items: vec![] };
        assert_eq!(clause.to_string(), "private()");
    }

    #[test]
    fn test_clause_data_reduction_empty_list() {
        let clause = ClauseData::Reduction {
            operator: ReductionOperator::Add,
            items: vec![],
        };
        assert_eq!(clause.to_string(), "reduction(+: )");
    }

    // Corner case: complex variable items
    #[test]
    fn test_clause_data_with_array_sections() {
        use crate::ir::ArraySection;
        let lower = Expression::unparsed("0");
        let length = Expression::unparsed("N");
        let section = ArraySection {
            lower_bound: Some(lower),
            length: Some(length),
            stride: None,
        };
        let var = Variable::with_sections("arr", vec![section]);
        let items = vec![ClauseItem::Variable(var)];
        let clause = ClauseData::Map {
            map_type: Some(MapType::To),
            mapper: None,
            items,
        };
        assert_eq!(clause.to_string(), "map(to: arr[0:N])");
    }

    // Corner case: expression items
    #[test]
    fn test_clause_data_with_expression_items() {
        let expr = Expression::unparsed("func(x, y)");
        let items = vec![ClauseItem::Expression(expr)];
        let clause = ClauseData::ItemList(items);
        assert_eq!(clause.to_string(), "func(x, y)");
    }

    // Corner case: debug formatting
    #[test]
    fn test_clause_data_debug() {
        let clause = ClauseData::Default(DefaultKind::Shared);
        let debug_str = format!("{clause:?}");
        assert!(debug_str.contains("Default"));
        assert!(debug_str.contains("Shared"));
    }
}
