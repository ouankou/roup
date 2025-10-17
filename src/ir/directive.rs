//! Directive IR types for complete OpenMP directive representation
//!
//! This module defines the top-level IR structures that represent complete
//! OpenMP directives with all their semantic information.
//!
//! ## Learning Objectives
//!
//! - **Box for heap allocation**: Managing large structures efficiently
//! - **Complex composition**: Combining all IR types into cohesive structures
//! - **Metadata handling**: Special cases for specific directive types
//! - **Query API**: Convenient methods for analyzing directives
//!
//! ## Design Philosophy
//!
//! A complete OpenMP directive consists of:
//! 1. **Kind**: What type of directive (parallel, for, task, etc.)
//! 2. **Clauses**: List of semantic clause data
//! 3. **Location**: Source position for error reporting
//! 4. **Language**: C, C++, or Fortran context
//! 5. **Metadata**: Special information for certain directives
//!
//! ## Example
//!
//! ```text
//! #pragma omp parallel for private(i) reduction(+: sum) schedule(static, 64)
//! ```
//!
//! Becomes:
//! ```ignore
//! DirectiveIR {
//!     kind: DirectiveKind::ParallelFor,
//!     clauses: vec![
//!         Private { items: [i] },
//!         Reduction { operator: Add, items: [sum] },
//!         Schedule { kind: Static, chunk_size: Some(64) },
//!     ],
//!     location: SourceLocation { line: 10, column: 1 },
//!     language: Language::C,
//!     metadata: None,
//! }
//! ```

use std::fmt;

use super::{ClauseData, Language, SourceLocation};

// ============================================================================
// DirectiveKind: All OpenMP directive types
// ============================================================================

/// OpenMP directive type
///
/// This enum covers all standard OpenMP directives from the 5.2 specification.
/// Each directive is represented as a unique variant.
///
/// ## Examples
///
/// ```
/// # use roup::ir::DirectiveKind;
/// let kind = DirectiveKind::Parallel;
/// assert_eq!(kind.to_string(), "parallel");
///
/// let kind = DirectiveKind::ParallelFor;
/// assert_eq!(kind.to_string(), "parallel for");
/// ```
///
/// ## Learning: Large Enum with Clear Organization
///
/// This enum demonstrates organizing a large number of variants (70+)
/// into logical categories using comments and grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub enum DirectiveKind {
    // ========================================================================
    // Parallel constructs
    // ========================================================================
    /// `#pragma omp parallel`
    Parallel = 0,
    /// `#pragma omp parallel for`
    ParallelFor = 1,
    /// `#pragma omp parallel for simd`
    ParallelForSimd = 2,
    /// `#pragma omp parallel sections`
    ParallelSections = 3,
    /// `#pragma omp parallel workshare` (Fortran)
    ParallelWorkshare = 4,
    /// `#pragma omp parallel loop`
    ParallelLoop = 5,
    /// `#pragma omp parallel loop simd`
    ParallelLoopSimd = 8,
    /// `#pragma omp parallel masked`
    ParallelMasked = 6,
    /// `#pragma omp parallel masked taskloop`
    ParallelMaskedTaskloop = 9,
    /// `#pragma omp parallel masked taskloop simd`
    ParallelMaskedTaskloopSimd = 17,
    /// `#pragma omp parallel master` (deprecated in 5.1)
    ParallelMaster = 7,
    /// `#pragma omp parallel master taskloop`
    ParallelMasterTaskloop = 18,
    /// `#pragma omp parallel master taskloop simd`
    ParallelMasterTaskloopSimd = 19,

    // ========================================================================
    // Work-sharing constructs
    // ========================================================================
    /// `#pragma omp for`
    For = 10,
    /// `#pragma omp for simd`
    ForSimd = 11,
    /// `#pragma omp sections`
    Sections = 12,
    /// `#pragma omp section`
    Section = 13,
    /// `#pragma omp single`
    Single = 14,
    /// `#pragma omp workshare` (Fortran)
    Workshare = 15,
    /// `#pragma omp loop`
    Loop = 16,

    // ========================================================================
    // SIMD constructs
    // ========================================================================
    /// `#pragma omp simd`
    Simd = 20,
    /// `#pragma omp declare simd`
    DeclareSimd = 21,

    // ========================================================================
    // Task constructs
    // ========================================================================
    /// `#pragma omp task`
    Task = 30,
    /// `#pragma omp taskloop`
    Taskloop = 31,
    /// `#pragma omp taskloop simd`
    TaskloopSimd = 32,
    /// `#pragma omp taskyield`
    Taskyield = 33,
    /// `#pragma omp taskwait`
    Taskwait = 34,
    /// `#pragma omp taskgroup`
    Taskgroup = 35,
    /// `#pragma omp taskgraph` (OpenMP 6.0)
    Taskgraph = 36,
    /// `#pragma omp task iteration` (OpenMP 6.0)
    TaskIteration = 37,
    /// `#pragma omp masked taskloop`
    MaskedTaskloop = 38,
    /// `#pragma omp masked taskloop simd`
    MaskedTaskloopSimd = 39,

    // ========================================================================
    // Target constructs
    // ========================================================================
    /// `#pragma omp target`
    Target = 40,
    /// `#pragma omp target data`
    TargetData = 41,
    /// `#pragma omp target enter data`
    TargetEnterData = 42,
    /// `#pragma omp target exit data`
    TargetExitData = 43,
    /// `#pragma omp target update`
    TargetUpdate = 44,
    /// `#pragma omp target parallel`
    TargetParallel = 45,
    /// `#pragma omp target parallel for`
    TargetParallelFor = 46,
    /// `#pragma omp target parallel for simd`
    TargetParallelForSimd = 47,
    /// `#pragma omp target parallel loop`
    TargetParallelLoop = 48,
    /// `#pragma omp target parallel loop simd`
    TargetParallelLoopSimd = 56,
    /// `#pragma omp target simd`
    TargetSimd = 49,
    /// `#pragma omp target loop`
    TargetLoop = 57,
    /// `#pragma omp target loop simd`
    TargetLoopSimd = 58,
    /// `#pragma omp target teams`
    TargetTeams = 50,
    /// `#pragma omp target teams distribute`
    TargetTeamsDistribute = 51,
    /// `#pragma omp target teams distribute simd`
    TargetTeamsDistributeSimd = 52,
    /// `#pragma omp target teams distribute parallel for`
    TargetTeamsDistributeParallelFor = 53,
    /// `#pragma omp target teams distribute parallel for simd`
    TargetTeamsDistributeParallelForSimd = 54,
    /// `#pragma omp target teams distribute parallel loop`
    TargetTeamsDistributeParallelLoop = 59,
    /// `#pragma omp target teams distribute parallel loop simd`
    TargetTeamsDistributeParallelLoopSimd = 69,
    /// `#pragma omp target teams loop`
    TargetTeamsLoop = 55,
    /// `#pragma omp target teams loop simd`
    TargetTeamsLoopSimd = 85,

    // ========================================================================
    // Teams constructs
    // ========================================================================
    /// `#pragma omp teams`
    Teams = 60,
    /// `#pragma omp teams distribute`
    TeamsDistribute = 61,
    /// `#pragma omp teams distribute simd`
    TeamsDistributeSimd = 62,
    /// `#pragma omp teams distribute parallel for`
    TeamsDistributeParallelFor = 63,
    /// `#pragma omp teams distribute parallel for simd`
    TeamsDistributeParallelForSimd = 64,
    /// `#pragma omp teams distribute parallel loop`
    TeamsDistributeParallelLoop = 66,
    /// `#pragma omp teams distribute parallel loop simd`
    TeamsDistributeParallelLoopSimd = 67,
    /// `#pragma omp teams loop`
    TeamsLoop = 65,
    /// `#pragma omp teams loop simd`
    TeamsLoopSimd = 68,

    // ========================================================================
    // Synchronization constructs
    // ========================================================================
    /// `#pragma omp barrier`
    Barrier = 70,
    /// `#pragma omp critical`
    Critical = 71,
    /// `#pragma omp atomic`
    Atomic = 72,
    /// `#pragma omp atomic read`
    AtomicRead = 77,
    /// `#pragma omp atomic write`
    AtomicWrite = 78,
    /// `#pragma omp atomic update`
    AtomicUpdate = 79,
    /// `#pragma omp atomic capture`
    AtomicCapture = 86,
    /// `#pragma omp atomic compare capture`
    AtomicCompareCapture = 87,
    /// `#pragma omp flush`
    Flush = 73,
    /// `#pragma omp ordered`
    Ordered = 74,
    /// `#pragma omp master`
    Master = 75,
    /// `#pragma omp masked`
    Masked = 76,

    // ========================================================================
    // Declare constructs
    // ========================================================================
    /// `#pragma omp declare reduction`
    DeclareReduction = 80,
    /// `#pragma omp declare mapper`
    DeclareMapper = 81,
    /// `#pragma omp declare target`
    DeclareTarget = 82,
    /// `#pragma omp declare variant`
    DeclareVariant = 83,
    /// `#pragma omp declare induction` (OpenMP 6.0)
    DeclareInduction = 84,

    // ========================================================================
    // Distribute constructs
    // ========================================================================
    /// `#pragma omp distribute`
    Distribute = 90,
    /// `#pragma omp distribute simd`
    DistributeSimd = 91,
    /// `#pragma omp distribute parallel for`
    DistributeParallelFor = 92,
    /// `#pragma omp distribute parallel for simd`
    DistributeParallelForSimd = 93,
    /// `#pragma omp distribute parallel loop`
    DistributeParallelLoop = 94,
    /// `#pragma omp distribute parallel loop simd`
    DistributeParallelLoopSimd = 95,

    // ========================================================================
    // Meta-directives
    // ========================================================================
    /// `#pragma omp metadirective`
    Metadirective = 100,
    /// `#pragma omp begin metadirective`
    BeginMetadirective = 103,
    /// `#pragma omp assume` (OpenMP 6.0)
    Assume = 101,
    /// `#pragma omp assumes` (OpenMP 6.0)
    Assumes = 102,
    /// `#pragma omp begin assumes`
    BeginAssumes = 104,
    /// `#pragma omp begin declare target`
    BeginDeclareTarget = 112,
    /// `#pragma omp end declare target`
    EndDeclareTarget = 113,
    /// `#pragma omp begin declare variant`
    BeginDeclareVariant = 114,
    /// `#pragma omp end declare variant`
    EndDeclareVariant = 115,

    // ========================================================================
    // Loop transformation directives (OpenMP 6.0)
    // ========================================================================
    /// `#pragma omp tile`
    Tile = 105,
    /// `#pragma omp unroll`
    Unroll = 106,
    /// `#pragma omp fuse`
    Fuse = 107,
    /// `#pragma omp split`
    Split = 108,
    /// `#pragma omp interchange`
    Interchange = 109,
    /// `#pragma omp reverse`
    Reverse = 110,
    /// `#pragma omp stripe`
    Stripe = 111,

    // ========================================================================
    // Other constructs
    // ========================================================================
    /// `#pragma omp threadprivate`
    Threadprivate = 120,
    /// `#pragma omp allocate`
    Allocate = 121,
    /// `#pragma omp allocators` (OpenMP 6.0)
    Allocators = 122,
    /// `#pragma omp requires`
    Requires = 123,
    /// `#pragma omp scan`
    Scan = 124,
    /// `#pragma omp depobj`
    Depobj = 125,
    /// `#pragma omp nothing`
    Nothing = 126,
    /// `#pragma omp error`
    Error = 127,
    /// `#pragma omp cancel` (OpenMP 4.0)
    Cancel = 128,
    /// `#pragma omp cancellation point` (OpenMP 4.0)
    CancellationPoint = 129,
    /// `#pragma omp dispatch` (OpenMP 6.0)
    Dispatch = 130,
    /// `#pragma omp interop` (OpenMP 5.1)
    Interop = 131,
    /// `#pragma omp scope` (OpenMP 5.1)
    Scope = 132,
    /// `#pragma omp groupprivate` (OpenMP 6.0)
    Groupprivate = 133,
    /// `#pragma omp workdistribute` (Fortran, OpenMP 6.0)
    Workdistribute = 134,

    // ========================================================================
    // Fortran "do" variants (Fortran equivalents of "for" directives)
    // ========================================================================
    /// `!$omp do` (Fortran equivalent of `for`)
    Do = 135,
    /// `!$omp do simd` (Fortran equivalent of `for simd`)
    DoSimd = 136,
    /// `!$omp parallel do` (Fortran equivalent of `parallel for`)
    ParallelDo = 137,
    /// `!$omp parallel do simd` (Fortran equivalent of `parallel for simd`)
    ParallelDoSimd = 138,
    /// `!$omp distribute parallel do` (Fortran)
    DistributeParallelDo = 139,
    /// `!$omp distribute parallel do simd` (Fortran)
    DistributeParallelDoSimd = 140,
    /// `!$omp teams distribute parallel do` (Fortran)
    TeamsDistributeParallelDo = 141,
    /// `!$omp teams distribute parallel do simd` (Fortran)
    TeamsDistributeParallelDoSimd = 142,
    /// `!$omp target parallel do` (Fortran)
    TargetParallelDo = 143,
    /// `!$omp target parallel do simd` (Fortran)
    TargetParallelDoSimd = 144,
    /// `!$omp target teams distribute parallel do` (Fortran)
    TargetTeamsDistributeParallelDo = 145,
    /// `!$omp target teams distribute parallel do simd` (Fortran)
    TargetTeamsDistributeParallelDoSimd = 146,

    /// Unknown or custom directive
    Unknown = 255,
}

impl fmt::Display for DirectiveKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Parallel constructs
            DirectiveKind::Parallel => write!(f, "parallel"),
            DirectiveKind::ParallelFor => write!(f, "parallel for"),
            DirectiveKind::ParallelForSimd => write!(f, "parallel for simd"),
            DirectiveKind::ParallelSections => write!(f, "parallel sections"),
            DirectiveKind::ParallelWorkshare => write!(f, "parallel workshare"),
            DirectiveKind::ParallelLoop => write!(f, "parallel loop"),
            DirectiveKind::ParallelLoopSimd => write!(f, "parallel loop simd"),
            DirectiveKind::ParallelMasked => write!(f, "parallel masked"),
            DirectiveKind::ParallelMaskedTaskloop => write!(f, "parallel masked taskloop"),
            DirectiveKind::ParallelMaskedTaskloopSimd => write!(f, "parallel masked taskloop simd"),
            DirectiveKind::ParallelMaster => write!(f, "parallel master"),
            DirectiveKind::ParallelMasterTaskloop => write!(f, "parallel master taskloop"),
            DirectiveKind::ParallelMasterTaskloopSimd => write!(f, "parallel master taskloop simd"),

            // Work-sharing constructs
            DirectiveKind::For => write!(f, "for"),
            DirectiveKind::ForSimd => write!(f, "for simd"),
            DirectiveKind::Sections => write!(f, "sections"),
            DirectiveKind::Section => write!(f, "section"),
            DirectiveKind::Single => write!(f, "single"),
            DirectiveKind::Workshare => write!(f, "workshare"),
            DirectiveKind::Loop => write!(f, "loop"),

            // SIMD constructs
            DirectiveKind::Simd => write!(f, "simd"),
            DirectiveKind::DeclareSimd => write!(f, "declare simd"),

            // Task constructs
            DirectiveKind::Task => write!(f, "task"),
            DirectiveKind::Taskloop => write!(f, "taskloop"),
            DirectiveKind::TaskloopSimd => write!(f, "taskloop simd"),
            DirectiveKind::Taskyield => write!(f, "taskyield"),
            DirectiveKind::Taskwait => write!(f, "taskwait"),
            DirectiveKind::Taskgroup => write!(f, "taskgroup"),
            DirectiveKind::Taskgraph => write!(f, "taskgraph"),
            DirectiveKind::TaskIteration => write!(f, "task iteration"),
            DirectiveKind::MaskedTaskloop => write!(f, "masked taskloop"),
            DirectiveKind::MaskedTaskloopSimd => write!(f, "masked taskloop simd"),

            // Target constructs
            DirectiveKind::Target => write!(f, "target"),
            DirectiveKind::TargetData => write!(f, "target data"),
            DirectiveKind::TargetEnterData => write!(f, "target enter data"),
            DirectiveKind::TargetExitData => write!(f, "target exit data"),
            DirectiveKind::TargetUpdate => write!(f, "target update"),
            DirectiveKind::TargetParallel => write!(f, "target parallel"),
            DirectiveKind::TargetParallelFor => write!(f, "target parallel for"),
            DirectiveKind::TargetParallelForSimd => write!(f, "target parallel for simd"),
            DirectiveKind::TargetParallelLoop => write!(f, "target parallel loop"),
            DirectiveKind::TargetParallelLoopSimd => write!(f, "target parallel loop simd"),
            DirectiveKind::TargetSimd => write!(f, "target simd"),
            DirectiveKind::TargetLoop => write!(f, "target loop"),
            DirectiveKind::TargetLoopSimd => write!(f, "target loop simd"),
            DirectiveKind::TargetTeams => write!(f, "target teams"),
            DirectiveKind::TargetTeamsDistribute => write!(f, "target teams distribute"),
            DirectiveKind::TargetTeamsDistributeSimd => write!(f, "target teams distribute simd"),
            DirectiveKind::TargetTeamsDistributeParallelFor => {
                write!(f, "target teams distribute parallel for")
            }
            DirectiveKind::TargetTeamsDistributeParallelForSimd => {
                write!(f, "target teams distribute parallel for simd")
            }
            DirectiveKind::TargetTeamsDistributeParallelLoop => {
                write!(f, "target teams distribute parallel loop")
            }
            DirectiveKind::TargetTeamsDistributeParallelLoopSimd => {
                write!(f, "target teams distribute parallel loop simd")
            }
            DirectiveKind::TargetTeamsLoop => write!(f, "target teams loop"),
            DirectiveKind::TargetTeamsLoopSimd => write!(f, "target teams loop simd"),

            // Teams constructs
            DirectiveKind::Teams => write!(f, "teams"),
            DirectiveKind::TeamsDistribute => write!(f, "teams distribute"),
            DirectiveKind::TeamsDistributeSimd => write!(f, "teams distribute simd"),
            DirectiveKind::TeamsDistributeParallelFor => {
                write!(f, "teams distribute parallel for")
            }
            DirectiveKind::TeamsDistributeParallelForSimd => {
                write!(f, "teams distribute parallel for simd")
            }
            DirectiveKind::TeamsDistributeParallelLoop => {
                write!(f, "teams distribute parallel loop")
            }
            DirectiveKind::TeamsDistributeParallelLoopSimd => {
                write!(f, "teams distribute parallel loop simd")
            }
            DirectiveKind::TeamsLoop => write!(f, "teams loop"),
            DirectiveKind::TeamsLoopSimd => write!(f, "teams loop simd"),

            // Synchronization constructs
            DirectiveKind::Barrier => write!(f, "barrier"),
            DirectiveKind::Critical => write!(f, "critical"),
            DirectiveKind::Atomic => write!(f, "atomic"),
            DirectiveKind::AtomicRead => write!(f, "atomic read"),
            DirectiveKind::AtomicWrite => write!(f, "atomic write"),
            DirectiveKind::AtomicUpdate => write!(f, "atomic update"),
            DirectiveKind::AtomicCapture => write!(f, "atomic capture"),
            DirectiveKind::AtomicCompareCapture => write!(f, "atomic compare capture"),
            DirectiveKind::Flush => write!(f, "flush"),
            DirectiveKind::Ordered => write!(f, "ordered"),
            DirectiveKind::Master => write!(f, "master"),
            DirectiveKind::Masked => write!(f, "masked"),

            // Declare constructs
            DirectiveKind::DeclareReduction => write!(f, "declare reduction"),
            DirectiveKind::DeclareMapper => write!(f, "declare mapper"),
            DirectiveKind::DeclareTarget => write!(f, "declare target"),
            DirectiveKind::DeclareVariant => write!(f, "declare variant"),
            DirectiveKind::DeclareInduction => write!(f, "declare induction"),

            // Distribute constructs
            DirectiveKind::Distribute => write!(f, "distribute"),
            DirectiveKind::DistributeSimd => write!(f, "distribute simd"),
            DirectiveKind::DistributeParallelFor => write!(f, "distribute parallel for"),
            DirectiveKind::DistributeParallelForSimd => {
                write!(f, "distribute parallel for simd")
            }
            DirectiveKind::DistributeParallelLoop => write!(f, "distribute parallel loop"),
            DirectiveKind::DistributeParallelLoopSimd => write!(f, "distribute parallel loop simd"),

            // Meta-directives
            DirectiveKind::Metadirective => write!(f, "metadirective"),
            DirectiveKind::BeginMetadirective => write!(f, "begin metadirective"),
            DirectiveKind::Assume => write!(f, "assume"),
            DirectiveKind::Assumes => write!(f, "assumes"),
            DirectiveKind::BeginAssumes => write!(f, "begin assumes"),
            DirectiveKind::BeginDeclareTarget => write!(f, "begin declare target"),
            DirectiveKind::EndDeclareTarget => write!(f, "end declare target"),
            DirectiveKind::BeginDeclareVariant => write!(f, "begin declare variant"),
            DirectiveKind::EndDeclareVariant => write!(f, "end declare variant"),

            // Loop transformations
            DirectiveKind::Tile => write!(f, "tile"),
            DirectiveKind::Unroll => write!(f, "unroll"),
            DirectiveKind::Fuse => write!(f, "fuse"),
            DirectiveKind::Split => write!(f, "split"),
            DirectiveKind::Interchange => write!(f, "interchange"),
            DirectiveKind::Reverse => write!(f, "reverse"),
            DirectiveKind::Stripe => write!(f, "stripe"),

            // Other constructs
            DirectiveKind::Threadprivate => write!(f, "threadprivate"),
            DirectiveKind::Allocate => write!(f, "allocate"),
            DirectiveKind::Allocators => write!(f, "allocators"),
            DirectiveKind::Requires => write!(f, "requires"),
            DirectiveKind::Scan => write!(f, "scan"),
            DirectiveKind::Depobj => write!(f, "depobj"),
            DirectiveKind::Nothing => write!(f, "nothing"),
            DirectiveKind::Error => write!(f, "error"),
            DirectiveKind::Cancel => write!(f, "cancel"),
            DirectiveKind::CancellationPoint => write!(f, "cancellation point"),
            DirectiveKind::Dispatch => write!(f, "dispatch"),
            DirectiveKind::Interop => write!(f, "interop"),
            DirectiveKind::Scope => write!(f, "scope"),
            DirectiveKind::Groupprivate => write!(f, "groupprivate"),
            DirectiveKind::Workdistribute => write!(f, "workdistribute"),

            // Fortran "do" variants
            DirectiveKind::Do => write!(f, "do"),
            DirectiveKind::DoSimd => write!(f, "do simd"),
            DirectiveKind::ParallelDo => write!(f, "parallel do"),
            DirectiveKind::ParallelDoSimd => write!(f, "parallel do simd"),
            DirectiveKind::DistributeParallelDo => write!(f, "distribute parallel do"),
            DirectiveKind::DistributeParallelDoSimd => write!(f, "distribute parallel do simd"),
            DirectiveKind::TeamsDistributeParallelDo => write!(f, "teams distribute parallel do"),
            DirectiveKind::TeamsDistributeParallelDoSimd => {
                write!(f, "teams distribute parallel do simd")
            }
            DirectiveKind::TargetParallelDo => write!(f, "target parallel do"),
            DirectiveKind::TargetParallelDoSimd => write!(f, "target parallel do simd"),
            DirectiveKind::TargetTeamsDistributeParallelDo => {
                write!(f, "target teams distribute parallel do")
            }
            DirectiveKind::TargetTeamsDistributeParallelDoSimd => {
                write!(f, "target teams distribute parallel do simd")
            }

            DirectiveKind::Unknown => write!(f, "unknown"),
        }
    }
}

impl DirectiveKind {
    /// Check if this is a parallel construct
    pub fn is_parallel(&self) -> bool {
        matches!(
            self,
            DirectiveKind::Parallel
                | DirectiveKind::ParallelFor
                | DirectiveKind::ParallelForSimd
                | DirectiveKind::ParallelSections
                | DirectiveKind::ParallelWorkshare
                | DirectiveKind::ParallelLoop
                | DirectiveKind::ParallelLoopSimd
                | DirectiveKind::ParallelMasked
                | DirectiveKind::ParallelMaskedTaskloop
                | DirectiveKind::ParallelMaskedTaskloopSimd
                | DirectiveKind::ParallelMaster
                | DirectiveKind::ParallelMasterTaskloop
                | DirectiveKind::ParallelMasterTaskloopSimd
                | DirectiveKind::ParallelDo
                | DirectiveKind::ParallelDoSimd
                | DirectiveKind::TargetParallel
                | DirectiveKind::TargetParallelFor
                | DirectiveKind::TargetParallelForSimd
                | DirectiveKind::TargetParallelLoop
                | DirectiveKind::TargetParallelLoopSimd
                | DirectiveKind::TargetParallelDo
                | DirectiveKind::TargetParallelDoSimd
        )
    }

    /// Check if this is a work-sharing construct
    pub fn is_worksharing(&self) -> bool {
        matches!(
            self,
            DirectiveKind::For
                | DirectiveKind::ForSimd
                | DirectiveKind::Sections
                | DirectiveKind::Section
                | DirectiveKind::Single
                | DirectiveKind::Workshare
                | DirectiveKind::Workdistribute
                | DirectiveKind::Do
                | DirectiveKind::DoSimd
        )
    }

    /// Check if this is a SIMD construct
    pub fn is_simd(&self) -> bool {
        matches!(
            self,
            DirectiveKind::Simd
                | DirectiveKind::DeclareSimd
                | DirectiveKind::ForSimd
                | DirectiveKind::ParallelForSimd
                | DirectiveKind::ParallelLoopSimd
                | DirectiveKind::ParallelMaskedTaskloopSimd
                | DirectiveKind::ParallelMasterTaskloopSimd
                | DirectiveKind::TaskloopSimd
                | DirectiveKind::MaskedTaskloopSimd
                | DirectiveKind::TargetSimd
                | DirectiveKind::TargetLoopSimd
                | DirectiveKind::TargetParallelForSimd
                | DirectiveKind::TargetParallelLoopSimd
                | DirectiveKind::TargetTeamsDistributeSimd
                | DirectiveKind::TargetTeamsDistributeParallelForSimd
                | DirectiveKind::TargetTeamsDistributeParallelLoopSimd
                | DirectiveKind::TargetTeamsLoopSimd
                | DirectiveKind::TeamsDistributeSimd
                | DirectiveKind::TeamsDistributeParallelForSimd
                | DirectiveKind::TeamsDistributeParallelLoopSimd
                | DirectiveKind::TeamsLoopSimd
                | DirectiveKind::DistributeSimd
                | DirectiveKind::DistributeParallelForSimd
                | DirectiveKind::DistributeParallelLoopSimd
                | DirectiveKind::DoSimd
                | DirectiveKind::ParallelDoSimd
                | DirectiveKind::DistributeParallelDoSimd
                | DirectiveKind::TeamsDistributeParallelDoSimd
                | DirectiveKind::TargetParallelDoSimd
                | DirectiveKind::TargetTeamsDistributeParallelDoSimd
        )
    }

    /// Check if this is a task construct
    pub fn is_task(&self) -> bool {
        matches!(
            self,
            DirectiveKind::Task
                | DirectiveKind::Taskloop
                | DirectiveKind::TaskloopSimd
                | DirectiveKind::Taskyield
                | DirectiveKind::Taskwait
                | DirectiveKind::Taskgroup
                | DirectiveKind::Taskgraph
                | DirectiveKind::TaskIteration
                | DirectiveKind::MaskedTaskloop
                | DirectiveKind::MaskedTaskloopSimd
        )
    }

    /// Check if this is a target construct
    pub fn is_target(&self) -> bool {
        matches!(
            self,
            DirectiveKind::Target
                | DirectiveKind::TargetData
                | DirectiveKind::TargetEnterData
                | DirectiveKind::TargetExitData
                | DirectiveKind::TargetUpdate
                | DirectiveKind::TargetParallel
                | DirectiveKind::TargetParallelFor
                | DirectiveKind::TargetParallelForSimd
                | DirectiveKind::TargetParallelLoop
                | DirectiveKind::TargetParallelLoopSimd
                | DirectiveKind::TargetSimd
                | DirectiveKind::TargetLoop
                | DirectiveKind::TargetLoopSimd
                | DirectiveKind::TargetTeams
                | DirectiveKind::TargetTeamsDistribute
                | DirectiveKind::TargetTeamsDistributeSimd
                | DirectiveKind::TargetTeamsDistributeParallelFor
                | DirectiveKind::TargetTeamsDistributeParallelForSimd
                | DirectiveKind::TargetTeamsDistributeParallelLoop
                | DirectiveKind::TargetTeamsDistributeParallelLoopSimd
                | DirectiveKind::TargetTeamsLoop
                | DirectiveKind::TargetTeamsLoopSimd
                | DirectiveKind::TargetParallelDo
                | DirectiveKind::TargetParallelDoSimd
                | DirectiveKind::TargetTeamsDistributeParallelDo
                | DirectiveKind::TargetTeamsDistributeParallelDoSimd
        )
    }

    /// Check if this is a teams construct
    pub fn is_teams(&self) -> bool {
        matches!(
            self,
            DirectiveKind::Teams
                | DirectiveKind::TeamsDistribute
                | DirectiveKind::TeamsDistributeSimd
                | DirectiveKind::TeamsDistributeParallelFor
                | DirectiveKind::TeamsDistributeParallelForSimd
                | DirectiveKind::TeamsDistributeParallelLoop
                | DirectiveKind::TeamsDistributeParallelLoopSimd
                | DirectiveKind::TeamsLoop
                | DirectiveKind::TeamsLoopSimd
                | DirectiveKind::TargetTeams
                | DirectiveKind::TargetTeamsDistribute
                | DirectiveKind::TargetTeamsDistributeSimd
                | DirectiveKind::TargetTeamsDistributeParallelFor
                | DirectiveKind::TargetTeamsDistributeParallelForSimd
                | DirectiveKind::TargetTeamsDistributeParallelLoop
                | DirectiveKind::TargetTeamsDistributeParallelLoopSimd
                | DirectiveKind::TargetTeamsLoop
                | DirectiveKind::TargetTeamsLoopSimd
                | DirectiveKind::TeamsDistributeParallelDo
                | DirectiveKind::TeamsDistributeParallelDoSimd
                | DirectiveKind::TargetTeamsDistributeParallelDo
                | DirectiveKind::TargetTeamsDistributeParallelDoSimd
        )
    }

    /// Check if this is a loop construct
    pub fn is_loop(&self) -> bool {
        matches!(
            self,
            DirectiveKind::For
                | DirectiveKind::ForSimd
                | DirectiveKind::Loop
                | DirectiveKind::ParallelFor
                | DirectiveKind::ParallelForSimd
                | DirectiveKind::ParallelLoop
                | DirectiveKind::ParallelLoopSimd
                | DirectiveKind::Simd
                | DirectiveKind::Taskloop
                | DirectiveKind::TaskloopSimd
                | DirectiveKind::MaskedTaskloop
                | DirectiveKind::MaskedTaskloopSimd
                | DirectiveKind::TargetLoop
                | DirectiveKind::TargetLoopSimd
                | DirectiveKind::TargetParallelLoop
                | DirectiveKind::TargetParallelLoopSimd
                | DirectiveKind::TargetTeamsLoop
                | DirectiveKind::TargetTeamsLoopSimd
                | DirectiveKind::TargetTeamsDistributeParallelLoop
                | DirectiveKind::TargetTeamsDistributeParallelLoopSimd
                | DirectiveKind::TeamsLoop
                | DirectiveKind::TeamsLoopSimd
                | DirectiveKind::TeamsDistributeParallelLoop
                | DirectiveKind::TeamsDistributeParallelLoopSimd
                | DirectiveKind::Distribute
                | DirectiveKind::DistributeSimd
                | DirectiveKind::DistributeParallelFor
                | DirectiveKind::DistributeParallelForSimd
                | DirectiveKind::DistributeParallelLoop
                | DirectiveKind::DistributeParallelLoopSimd
                | DirectiveKind::Do
                | DirectiveKind::DoSimd
                | DirectiveKind::ParallelDo
                | DirectiveKind::ParallelDoSimd
                | DirectiveKind::DistributeParallelDo
                | DirectiveKind::DistributeParallelDoSimd
                | DirectiveKind::TeamsDistributeParallelDo
                | DirectiveKind::TeamsDistributeParallelDoSimd
                | DirectiveKind::TargetParallelDo
                | DirectiveKind::TargetParallelDoSimd
                | DirectiveKind::TargetTeamsDistributeParallelDo
                | DirectiveKind::TargetTeamsDistributeParallelDoSimd
        )
    }

    /// Check if this is a synchronization construct
    pub fn is_synchronization(&self) -> bool {
        matches!(
            self,
            DirectiveKind::Barrier
                | DirectiveKind::Critical
                | DirectiveKind::Atomic
                | DirectiveKind::AtomicRead
                | DirectiveKind::AtomicWrite
                | DirectiveKind::AtomicUpdate
                | DirectiveKind::AtomicCapture
                | DirectiveKind::AtomicCompareCapture
                | DirectiveKind::Flush
                | DirectiveKind::Ordered
                | DirectiveKind::Master
                | DirectiveKind::Masked
                | DirectiveKind::Taskwait
                | DirectiveKind::Taskyield
        )
    }

    /// Check if this is a declare construct
    pub fn is_declare(&self) -> bool {
        matches!(
            self,
            DirectiveKind::DeclareReduction
                | DirectiveKind::DeclareMapper
                | DirectiveKind::DeclareTarget
                | DirectiveKind::DeclareVariant
                | DirectiveKind::DeclareSimd
        )
    }

    /// Check if this directive has a structured block (requires end directive)
    pub fn has_structured_block(&self) -> bool {
        !matches!(
            self,
            DirectiveKind::Barrier
                | DirectiveKind::Taskyield
                | DirectiveKind::Taskwait
                | DirectiveKind::Flush
                | DirectiveKind::TargetEnterData
                | DirectiveKind::TargetExitData
                | DirectiveKind::TargetUpdate
                | DirectiveKind::Threadprivate
                | DirectiveKind::DeclareSimd
                | DirectiveKind::DeclareReduction
                | DirectiveKind::DeclareMapper
                | DirectiveKind::DeclareTarget
                | DirectiveKind::DeclareVariant
                | DirectiveKind::Scan
                | DirectiveKind::Depobj
                | DirectiveKind::Nothing
                | DirectiveKind::Error
                | DirectiveKind::Section
        )
    }
}

// ============================================================================
// DirectiveIR: Complete directive representation
// ============================================================================

/// Complete IR representation of an OpenMP directive
///
/// This is the top-level structure that combines all IR components:
/// - Directive type (kind)
/// - Semantic clause data
/// - Source location
/// - Language context
/// - Optional metadata for special directives
///
/// ## Examples
///
/// ```
/// # use roup::ir::{DirectiveIR, DirectiveKind, ClauseData, DefaultKind, Language, SourceLocation};
/// let dir = DirectiveIR::new(
///     DirectiveKind::Parallel,
///     "parallel",
///     vec![ClauseData::Default(DefaultKind::Shared)],
///     SourceLocation::new(10, 1),
///     Language::C,
/// );
///
/// assert_eq!(dir.kind(), DirectiveKind::Parallel);
/// assert_eq!(dir.clauses().len(), 1);
/// assert!(dir.kind().is_parallel());
/// ```
///
/// ## Learning: Box for Large Structures
///
/// Since `DirectiveIR` can contain a large `Vec<ClauseData>`, and `ClauseData`
/// variants can themselves be large, we use `Box<[ClauseData]>` instead of
/// `Vec<ClauseData>` for the final immutable representation. This:
///
/// 1. Reduces struct size (Box is one pointer)
/// 2. Signals immutability (boxed slice can't grow/shrink)
/// 3. Saves memory (no extra capacity like Vec)
///
/// We still accept `Vec` in constructors for convenience, then convert to Box.
///
/// ## Memory Model (Safety Fix)
///
/// **IMPORTANT**: This struct now stores an owned `name: String` to prevent use-after-free bugs.
///
/// **Why?**
/// - Directive names from line continuations are stored in `Cow::Owned`
/// - Previously, IR borrowed from this `Cow` via `'a` lifetime
/// - When `Directive` dropped, `Cow::Owned` was freed → dangling pointers
/// - **Solution**: DirectiveIR now owns the normalized directive name
///
/// **Performance**: Minimal overhead (~50ns String allocation). See `docs/PERFORMANCE_ANALYSIS.md`.
#[derive(Debug, Clone, PartialEq)]
pub struct DirectiveIR {
    /// The kind of directive
    kind: DirectiveKind,

    /// The normalized directive name (owned to prevent use-after-free)
    ///
    /// This is cloned from the parser's `Cow<'a, str>` during conversion.
    /// Storing it here ensures the IR is self-contained and doesn't depend
    /// on the parser's lifetime.
    ///
    /// Examples: "parallel", "parallel for", "target teams distribute"
    name: String,

    /// Semantic clause data
    ///
    /// Using `Box<[ClauseData]>` instead of `Vec<ClauseData>` for the final representation:
    /// - Smaller size (one pointer vs three)
    /// - Signals immutability (can't grow)
    /// - Saves memory (no unused capacity)
    clauses: Box<[ClauseData]>,

    /// Source location where this directive appears
    location: SourceLocation,

    /// Language context (C, C++, Fortran)
    language: Language,
}

impl DirectiveIR {
    /// Create a new directive IR
    ///
    /// ## Example
    ///
    /// ```
    /// # use roup::ir::{DirectiveIR, DirectiveKind, ClauseData, ReductionOperator, Identifier, Language, SourceLocation};
    /// let clauses = vec![
    ///     ClauseData::Reduction {
    ///         operator: ReductionOperator::Add,
    ///         items: vec![Identifier::new("sum").into()],
    ///     },
    /// ];
    ///
    /// let dir = DirectiveIR::new(
    ///     DirectiveKind::ParallelFor,
    ///     "parallel for",
    ///     clauses,
    ///     SourceLocation::new(42, 1),
    ///     Language::C,
    /// );
    ///
    /// assert_eq!(dir.kind(), DirectiveKind::ParallelFor);
    /// assert_eq!(dir.name(), "parallel for");
    /// ```
    pub fn new(
        kind: DirectiveKind,
        name: &str,
        clauses: Vec<ClauseData>,
        location: SourceLocation,
        language: Language,
    ) -> Self {
        Self {
            kind,
            name: name.to_string(),
            clauses: clauses.into_boxed_slice(),
            location,
            language,
        }
    }

    // ========================================================================
    // Convenience constructors
    // ========================================================================

    /// Create a simple directive with no clauses
    ///
    /// ## Example
    ///
    /// ```
    /// # use roup::ir::{DirectiveIR, DirectiveKind, Language, SourceLocation};
    /// let dir = DirectiveIR::simple(DirectiveKind::Barrier, "barrier", SourceLocation::start(), Language::C);
    /// assert_eq!(dir.clauses().len(), 0);
    /// ```
    pub fn simple(
        kind: DirectiveKind,
        name: &str,
        location: SourceLocation,
        language: Language,
    ) -> Self {
        Self::new(kind, name, vec![], location, language)
    }

    /// Create a parallel directive with common clauses
    ///
    /// Convenience constructor for the most common OpenMP pattern.
    ///
    /// ## Example
    ///
    /// ```
    /// # use roup::ir::{DirectiveIR, DefaultKind, Language, SourceLocation};
    /// let dir = DirectiveIR::parallel(
    ///     Some(DefaultKind::Shared),
    ///     SourceLocation::start(),
    ///     Language::C
    /// );
    /// assert!(dir.has_clause(|c| c.is_default()));
    /// ```
    pub fn parallel(
        default: Option<super::DefaultKind>,
        location: SourceLocation,
        language: Language,
    ) -> Self {
        let mut clauses = vec![];
        if let Some(kind) = default {
            clauses.push(ClauseData::Default(kind));
        }
        Self::new(
            DirectiveKind::Parallel,
            "parallel",
            clauses,
            location,
            language,
        )
    }

    /// Create a for loop directive with schedule
    ///
    /// ## Example
    ///
    /// ```
    /// # use roup::ir::{DirectiveIR, ScheduleKind, Language, SourceLocation};
    /// let dir = DirectiveIR::for_loop(
    ///     ScheduleKind::Static,
    ///     None,
    ///     SourceLocation::start(),
    ///     Language::C
    /// );
    /// assert!(dir.has_clause(|c| c.is_schedule()));
    /// ```
    pub fn for_loop(
        schedule: super::ScheduleKind,
        chunk_size: Option<super::Expression>,
        location: SourceLocation,
        language: Language,
    ) -> Self {
        let clauses = vec![ClauseData::Schedule {
            kind: schedule,
            modifiers: vec![],
            chunk_size,
        }];
        Self::new(DirectiveKind::For, "for", clauses, location, language)
    }

    /// Create a barrier directive (always simple)
    ///
    /// ## Example
    ///
    /// ```
    /// # use roup::ir::{DirectiveIR, DirectiveKind, Language, SourceLocation};
    /// let dir = DirectiveIR::barrier(SourceLocation::start(), Language::C);
    /// assert_eq!(dir.kind(), DirectiveKind::Barrier);
    /// assert_eq!(dir.clauses().len(), 0);
    /// ```
    pub fn barrier(location: SourceLocation, language: Language) -> Self {
        Self::simple(DirectiveKind::Barrier, "barrier", location, language)
    }

    /// Create a taskwait directive (always simple)
    pub fn taskwait(location: SourceLocation, language: Language) -> Self {
        Self::simple(DirectiveKind::Taskwait, "taskwait", location, language)
    }

    /// Create a taskyield directive (always simple)
    pub fn taskyield(location: SourceLocation, language: Language) -> Self {
        Self::simple(DirectiveKind::Taskyield, "taskyield", location, language)
    }

    // ========================================================================
    // Query API
    // ========================================================================

    /// Get the directive kind
    pub fn kind(&self) -> DirectiveKind {
        self.kind
    }

    /// Get the normalized directive name
    ///
    /// This returns the directive name as it appears in the source,
    /// after normalization (e.g., line continuations collapsed).
    ///
    /// ## Example
    ///
    /// ```
    /// # use roup::ir::{DirectiveIR, DirectiveKind, Language, SourceLocation};
    /// let dir = DirectiveIR::simple(DirectiveKind::ParallelFor, "parallel for", SourceLocation::start(), Language::C);
    /// assert_eq!(dir.name(), "parallel for");
    /// ```
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the clauses
    pub fn clauses(&self) -> &[ClauseData] {
        &self.clauses
    }

    /// Get the source location
    pub fn location(&self) -> SourceLocation {
        self.location
    }

    /// Get the language
    pub fn language(&self) -> Language {
        self.language
    }

    /// Check if this directive has a specific clause type
    ///
    /// ## Example
    ///
    /// ```
    /// # use roup::ir::{DirectiveIR, DirectiveKind, ClauseData, DefaultKind, Language, SourceLocation};
    /// let dir = DirectiveIR::new(
    ///     DirectiveKind::Parallel,
    ///     "parallel",
    ///     vec![ClauseData::Default(DefaultKind::Shared)],
    ///     SourceLocation::start(),
    ///     Language::C,
    /// );
    ///
    /// assert!(dir.has_clause(|c| matches!(c, ClauseData::Default(_))));
    /// assert!(!dir.has_clause(|c| matches!(c, ClauseData::Private { .. })));
    /// ```
    pub fn has_clause<F>(&self, predicate: F) -> bool
    where
        F: Fn(&ClauseData) -> bool,
    {
        self.clauses.iter().any(predicate)
    }

    /// Find first clause matching predicate
    pub fn find_clause<F>(&self, predicate: F) -> Option<&ClauseData>
    where
        F: Fn(&ClauseData) -> bool,
    {
        self.clauses.iter().find(|c| predicate(c))
    }

    /// Count clauses matching predicate
    pub fn count_clauses<F>(&self, predicate: F) -> usize
    where
        F: Fn(&ClauseData) -> bool,
    {
        self.clauses.iter().filter(|c| predicate(c)).count()
    }

    /// Get all clauses matching predicate
    pub fn filter_clauses<F>(&self, predicate: F) -> Vec<&ClauseData>
    where
        F: Fn(&ClauseData) -> bool,
    {
        self.clauses.iter().filter(|c| predicate(c)).collect()
    }
}

impl fmt::Display for DirectiveIR {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Write pragma prefix (already includes "omp ")
        write!(f, "{}{}", self.language.pragma_prefix(), self.kind)?;

        // Write clauses
        for clause in self.clauses.iter() {
            write!(f, " {clause}")?;
        }

        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{ClauseItem, DefaultKind, Identifier, ReductionOperator};

    // DirectiveKind tests
    #[test]
    fn test_directive_kind_display() {
        assert_eq!(DirectiveKind::Parallel.to_string(), "parallel");
        assert_eq!(DirectiveKind::ParallelFor.to_string(), "parallel for");
        assert_eq!(DirectiveKind::For.to_string(), "for");
        assert_eq!(DirectiveKind::Target.to_string(), "target");
        assert_eq!(
            DirectiveKind::TargetTeamsDistributeParallelForSimd.to_string(),
            "target teams distribute parallel for simd"
        );
    }

    #[test]
    fn test_directive_kind_is_parallel() {
        assert!(DirectiveKind::Parallel.is_parallel());
        assert!(DirectiveKind::ParallelFor.is_parallel());
        assert!(DirectiveKind::ParallelForSimd.is_parallel());
        assert!(!DirectiveKind::For.is_parallel());
        assert!(!DirectiveKind::Target.is_parallel());
    }

    #[test]
    fn test_directive_kind_is_worksharing() {
        assert!(DirectiveKind::For.is_worksharing());
        assert!(DirectiveKind::Sections.is_worksharing());
        assert!(DirectiveKind::Single.is_worksharing());
        assert!(DirectiveKind::Workshare.is_worksharing());
        assert!(DirectiveKind::Workdistribute.is_worksharing());
        assert!(!DirectiveKind::Parallel.is_worksharing());
    }

    #[test]
    fn test_directive_kind_is_simd() {
        assert!(DirectiveKind::Simd.is_simd());
        assert!(DirectiveKind::ForSimd.is_simd());
        assert!(DirectiveKind::ParallelForSimd.is_simd());
        assert!(!DirectiveKind::For.is_simd());
        assert!(!DirectiveKind::Parallel.is_simd());
    }

    #[test]
    fn test_directive_kind_is_task() {
        assert!(DirectiveKind::Task.is_task());
        assert!(DirectiveKind::Taskloop.is_task());
        assert!(DirectiveKind::Taskyield.is_task());
        assert!(DirectiveKind::Taskgraph.is_task());
        assert!(DirectiveKind::TaskIteration.is_task());
        assert!(!DirectiveKind::Parallel.is_task());
    }

    #[test]
    fn test_directive_kind_is_target() {
        assert!(DirectiveKind::Target.is_target());
        assert!(DirectiveKind::TargetData.is_target());
        assert!(DirectiveKind::TargetTeams.is_target());
        assert!(!DirectiveKind::Teams.is_target());
        assert!(!DirectiveKind::Parallel.is_target());
    }

    #[test]
    fn test_directive_kind_is_teams() {
        assert!(DirectiveKind::Teams.is_teams());
        assert!(DirectiveKind::TeamsDistribute.is_teams());
        assert!(DirectiveKind::TargetTeams.is_teams());
        assert!(!DirectiveKind::Target.is_teams());
    }

    #[test]
    fn test_directive_kind_is_loop() {
        assert!(DirectiveKind::For.is_loop());
        assert!(DirectiveKind::Loop.is_loop());
        assert!(DirectiveKind::Simd.is_loop());
        assert!(!DirectiveKind::Parallel.is_loop());
        assert!(!DirectiveKind::Barrier.is_loop());
    }

    #[test]
    fn test_directive_kind_is_synchronization() {
        assert!(DirectiveKind::Barrier.is_synchronization());
        assert!(DirectiveKind::Critical.is_synchronization());
        assert!(DirectiveKind::Atomic.is_synchronization());
        assert!(!DirectiveKind::Parallel.is_synchronization());
    }

    #[test]
    fn test_directive_kind_is_declare() {
        assert!(DirectiveKind::DeclareReduction.is_declare());
        assert!(DirectiveKind::DeclareTarget.is_declare());
        assert!(DirectiveKind::DeclareSimd.is_declare());
        assert!(!DirectiveKind::Parallel.is_declare());
    }

    #[test]
    fn test_directive_kind_has_structured_block() {
        assert!(DirectiveKind::Parallel.has_structured_block());
        assert!(DirectiveKind::For.has_structured_block());
        assert!(DirectiveKind::Critical.has_structured_block());
        assert!(!DirectiveKind::Barrier.has_structured_block());
        assert!(!DirectiveKind::Taskyield.has_structured_block());
        assert!(!DirectiveKind::DeclareTarget.has_structured_block());
    }

    // DirectiveIR tests
    #[test]
    fn test_directive_ir_new() {
        let dir = DirectiveIR::new(
            DirectiveKind::Parallel,
            "parallel",
            vec![],
            SourceLocation::new(10, 1),
            Language::C,
        );

        assert_eq!(dir.kind(), DirectiveKind::Parallel);
        assert_eq!(dir.clauses().len(), 0);
        assert_eq!(dir.location(), SourceLocation::new(10, 1));
        assert_eq!(dir.language(), Language::C);
    }

    #[test]
    fn test_directive_ir_with_clauses() {
        let clauses = vec![
            ClauseData::Default(DefaultKind::Shared),
            ClauseData::Private {
                items: vec![ClauseItem::Identifier(Identifier::new("x"))],
            },
        ];

        let dir = DirectiveIR::new(
            DirectiveKind::Parallel,
            "parallel",
            clauses,
            SourceLocation::start(),
            Language::C,
        );

        assert_eq!(dir.clauses().len(), 2);
    }

    #[test]
    fn test_directive_ir_has_clause() {
        let dir = DirectiveIR::new(
            DirectiveKind::Parallel,
            "parallel",
            vec![ClauseData::Default(DefaultKind::Shared)],
            SourceLocation::start(),
            Language::C,
        );

        assert!(dir.has_clause(|c| matches!(c, ClauseData::Default(_))));
        assert!(!dir.has_clause(|c| matches!(c, ClauseData::Private { .. })));
    }

    #[test]
    fn test_directive_ir_find_clause() {
        let dir = DirectiveIR::new(
            DirectiveKind::Parallel,
            "parallel",
            vec![
                ClauseData::Default(DefaultKind::Shared),
                ClauseData::Private { items: vec![] },
            ],
            SourceLocation::start(),
            Language::C,
        );

        let found = dir.find_clause(|c| matches!(c, ClauseData::Default(_)));
        assert!(found.is_some());
        assert!(matches!(found.unwrap(), ClauseData::Default(_)));
    }

    #[test]
    fn test_directive_ir_count_clauses() {
        let dir = DirectiveIR::new(
            DirectiveKind::Parallel,
            "parallel",
            vec![
                ClauseData::Private { items: vec![] },
                ClauseData::Default(DefaultKind::Shared),
                ClauseData::Private { items: vec![] },
            ],
            SourceLocation::start(),
            Language::C,
        );

        assert_eq!(
            dir.count_clauses(|c| matches!(c, ClauseData::Private { .. })),
            2
        );
        assert_eq!(
            dir.count_clauses(|c| matches!(c, ClauseData::Default(_))),
            1
        );
    }

    #[test]
    fn test_directive_ir_filter_clauses() {
        let dir = DirectiveIR::new(
            DirectiveKind::Parallel,
            "parallel",
            vec![
                ClauseData::Private { items: vec![] },
                ClauseData::Default(DefaultKind::Shared),
                ClauseData::Private { items: vec![] },
            ],
            SourceLocation::start(),
            Language::C,
        );

        let private_clauses = dir.filter_clauses(|c| matches!(c, ClauseData::Private { .. }));
        assert_eq!(private_clauses.len(), 2);
    }

    #[test]
    fn test_directive_ir_display() {
        let dir = DirectiveIR::new(
            DirectiveKind::Parallel,
            "parallel",
            vec![ClauseData::Default(DefaultKind::Shared)],
            SourceLocation::start(),
            Language::C,
        );

        let display = dir.to_string();
        assert!(display.contains("pragma"));
        assert!(display.contains("omp"));
        assert!(display.contains("parallel"));
        assert!(display.contains("default"));
    }

    #[test]
    fn test_directive_ir_display_with_reduction() {
        let clauses = vec![ClauseData::Reduction {
            operator: ReductionOperator::Add,
            items: vec![ClauseItem::Identifier(Identifier::new("sum"))],
        }];

        let dir = DirectiveIR::new(
            DirectiveKind::ParallelFor,
            "parallel for",
            clauses,
            SourceLocation::start(),
            Language::C,
        );

        let display = dir.to_string();
        assert!(display.contains("parallel for"));
        assert!(display.contains("reduction"));
        assert!(display.contains("+"));
        assert!(display.contains("sum"));
    }

    #[test]
    fn test_directive_ir_clone() {
        let dir1 = DirectiveIR::new(
            DirectiveKind::Parallel,
            "parallel",
            vec![ClauseData::Default(DefaultKind::Shared)],
            SourceLocation::start(),
            Language::C,
        );

        let dir2 = dir1.clone();
        assert_eq!(dir1, dir2);
    }

    #[test]
    fn test_directive_ir_equality() {
        let dir1 = DirectiveIR::new(
            DirectiveKind::Parallel,
            "parallel",
            vec![],
            SourceLocation::start(),
            Language::C,
        );

        let dir2 = DirectiveIR::new(
            DirectiveKind::Parallel,
            "parallel",
            vec![],
            SourceLocation::start(),
            Language::C,
        );

        let dir3 = DirectiveIR::new(
            DirectiveKind::For,
            "for",
            vec![],
            SourceLocation::start(),
            Language::C,
        );

        assert_eq!(dir1, dir2);
        assert_ne!(dir1, dir3);
    }

    // Corner case: very long directive name
    #[test]
    fn test_directive_kind_longest_name() {
        let kind = DirectiveKind::TargetTeamsDistributeParallelForSimd;
        assert_eq!(
            kind.to_string(),
            "target teams distribute parallel for simd"
        );
        assert!(kind.is_target());
        assert!(kind.is_teams());
        assert!(kind.is_simd());
    }

    // Corner case: deprecated construct
    #[test]
    fn test_directive_kind_deprecated() {
        let kind = DirectiveKind::ParallelMaster;
        assert_eq!(kind.to_string(), "parallel master");
        assert!(kind.is_parallel());
    }

    // Corner case: empty directive
    #[test]
    fn test_directive_ir_no_clauses() {
        let dir = DirectiveIR::new(
            DirectiveKind::Barrier,
            "barrier",
            vec![],
            SourceLocation::start(),
            Language::C,
        );

        assert_eq!(dir.clauses().len(), 0);
        assert!(!dir.has_clause(|_| true));
    }
}
