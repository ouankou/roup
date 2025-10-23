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

use super::clause::ClauseDisplayMode;

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
    /// `#pragma omp parallel masked`
    ParallelMasked = 6,
    /// `#pragma omp parallel master` (deprecated in 5.1)
    ParallelMaster = 7,

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
    /// `#pragma omp target simd`
    TargetSimd = 49,
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
    /// `#pragma omp target teams loop`
    TargetTeamsLoop = 55,

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
    /// `#pragma omp teams loop`
    TeamsLoop = 65,

    // ========================================================================
    // Synchronization constructs
    // ========================================================================
    /// `#pragma omp barrier`
    Barrier = 70,
    /// `#pragma omp critical`
    Critical = 71,
    /// `#pragma omp atomic`
    Atomic = 72,
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

    // ========================================================================
    // Meta-directives
    // ========================================================================
    /// `#pragma omp metadirective`
    Metadirective = 100,

    // ========================================================================
    // Other constructs
    // ========================================================================
    /// `#pragma omp threadprivate`
    Threadprivate = 110,
    /// `#pragma omp allocate`
    Allocate = 111,
    /// `#pragma omp requires`
    Requires = 112,
    /// `#pragma omp scan`
    Scan = 113,
    /// `#pragma omp depobj`
    Depobj = 114,
    /// `#pragma omp nothing`
    Nothing = 115,
    /// `#pragma omp error`
    Error = 116,

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
            DirectiveKind::ParallelMasked => write!(f, "parallel masked"),
            DirectiveKind::ParallelMaster => write!(f, "parallel master"),

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
            DirectiveKind::TargetSimd => write!(f, "target simd"),
            DirectiveKind::TargetTeams => write!(f, "target teams"),
            DirectiveKind::TargetTeamsDistribute => write!(f, "target teams distribute"),
            DirectiveKind::TargetTeamsDistributeSimd => write!(f, "target teams distribute simd"),
            DirectiveKind::TargetTeamsDistributeParallelFor => {
                write!(f, "target teams distribute parallel for")
            }
            DirectiveKind::TargetTeamsDistributeParallelForSimd => {
                write!(f, "target teams distribute parallel for simd")
            }
            DirectiveKind::TargetTeamsLoop => write!(f, "target teams loop"),

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
            DirectiveKind::TeamsLoop => write!(f, "teams loop"),

            // Synchronization constructs
            DirectiveKind::Barrier => write!(f, "barrier"),
            DirectiveKind::Critical => write!(f, "critical"),
            DirectiveKind::Atomic => write!(f, "atomic"),
            DirectiveKind::Flush => write!(f, "flush"),
            DirectiveKind::Ordered => write!(f, "ordered"),
            DirectiveKind::Master => write!(f, "master"),
            DirectiveKind::Masked => write!(f, "masked"),

            // Declare constructs
            DirectiveKind::DeclareReduction => write!(f, "declare reduction"),
            DirectiveKind::DeclareMapper => write!(f, "declare mapper"),
            DirectiveKind::DeclareTarget => write!(f, "declare target"),
            DirectiveKind::DeclareVariant => write!(f, "declare variant"),

            // Distribute constructs
            DirectiveKind::Distribute => write!(f, "distribute"),
            DirectiveKind::DistributeSimd => write!(f, "distribute simd"),
            DirectiveKind::DistributeParallelFor => write!(f, "distribute parallel for"),
            DirectiveKind::DistributeParallelForSimd => {
                write!(f, "distribute parallel for simd")
            }

            // Meta-directives
            DirectiveKind::Metadirective => write!(f, "metadirective"),

            // Other constructs
            DirectiveKind::Threadprivate => write!(f, "threadprivate"),
            DirectiveKind::Allocate => write!(f, "allocate"),
            DirectiveKind::Requires => write!(f, "requires"),
            DirectiveKind::Scan => write!(f, "scan"),
            DirectiveKind::Depobj => write!(f, "depobj"),
            DirectiveKind::Nothing => write!(f, "nothing"),
            DirectiveKind::Error => write!(f, "error"),

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
                | DirectiveKind::ParallelMasked
                | DirectiveKind::ParallelMaster
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
                | DirectiveKind::TaskloopSimd
                | DirectiveKind::TargetSimd
                | DirectiveKind::TargetParallelForSimd
                | DirectiveKind::TargetTeamsDistributeSimd
                | DirectiveKind::TargetTeamsDistributeParallelForSimd
                | DirectiveKind::TeamsDistributeSimd
                | DirectiveKind::TeamsDistributeParallelForSimd
                | DirectiveKind::DistributeSimd
                | DirectiveKind::DistributeParallelForSimd
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
                | DirectiveKind::TargetSimd
                | DirectiveKind::TargetTeams
                | DirectiveKind::TargetTeamsDistribute
                | DirectiveKind::TargetTeamsDistributeSimd
                | DirectiveKind::TargetTeamsDistributeParallelFor
                | DirectiveKind::TargetTeamsDistributeParallelForSimd
                | DirectiveKind::TargetTeamsLoop
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
                | DirectiveKind::TeamsLoop
                | DirectiveKind::TargetTeams
                | DirectiveKind::TargetTeamsDistribute
                | DirectiveKind::TargetTeamsDistributeSimd
                | DirectiveKind::TargetTeamsDistributeParallelFor
                | DirectiveKind::TargetTeamsDistributeParallelForSimd
                | DirectiveKind::TargetTeamsLoop
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
                | DirectiveKind::Simd
                | DirectiveKind::Taskloop
                | DirectiveKind::TaskloopSimd
                | DirectiveKind::Distribute
                | DirectiveKind::DistributeSimd
                | DirectiveKind::DistributeParallelFor
                | DirectiveKind::DistributeParallelForSimd
        )
    }

    /// Check if this is a synchronization construct
    pub fn is_synchronization(&self) -> bool {
        matches!(
            self,
            DirectiveKind::Barrier
                | DirectiveKind::Critical
                | DirectiveKind::Atomic
                | DirectiveKind::Flush
                | DirectiveKind::Ordered
                | DirectiveKind::Master
                | DirectiveKind::Masked
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

impl<'a> DirectiveIR {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DirectiveDisplayMode {
    Full,
    Plain,
}

impl DirectiveDisplayMode {
    #[inline]
    const fn clause_mode(self) -> ClauseDisplayMode {
        match self {
            DirectiveDisplayMode::Full => ClauseDisplayMode::Full,
            DirectiveDisplayMode::Plain => ClauseDisplayMode::Plain,
        }
    }
}

impl DirectiveIR {
    fn fmt_with_mode(&self, f: &mut fmt::Formatter<'_>, mode: DirectiveDisplayMode) -> fmt::Result {
        write!(f, "{}{}", self.language.pragma_prefix(), self.kind)?;

        let clause_mode = mode.clause_mode();
        for clause in self.clauses.iter() {
            write!(f, " ")?;
            clause.fmt_with_mode(f, clause_mode)?;
        }

        Ok(())
    }

    /// Generate a normalized directive string without user-provided symbols.
    ///
    /// This replaces identifiers, variables, and expressions in clause payloads
    /// with empty slots while preserving the directive structure. Enumerated
    /// modifiers (e.g., `tofrom`, `monotonic`) remain intact.
    pub fn to_plain_string(&self) -> String {
        format!("{}", DirectivePlainDisplay { directive: self })
    }
}

impl<'a> fmt::Display for DirectiveIR {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_with_mode(f, DirectiveDisplayMode::Full)
    }
}

struct DirectivePlainDisplay<'a> {
    directive: &'a DirectiveIR,
}

impl fmt::Display for DirectivePlainDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.directive.fmt_with_mode(f, DirectiveDisplayMode::Plain)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{
        ArraySection, ClauseItem, DefaultKind, Expression, Identifier, Language, MapType,
        ReductionOperator, ScheduleKind, ScheduleModifier, SourceLocation, Variable,
    };

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

    #[test]
    fn directive_plain_string_removes_user_symbols_from_map() {
        let array_section = ArraySection::new(
            Some(Expression::unparsed("0")),
            Some(Expression::unparsed("ARRAY_SIZE")),
            None,
        );

        let map_clause_1 = ClauseData::Map {
            map_type: Some(MapType::ToFrom),
            mapper: None,
            items: vec![
                ClauseItem::from(Variable::with_sections("a", vec![array_section.clone()])),
                ClauseItem::from(Variable::new("num_teams")),
            ],
        };

        let map_clause_2 = ClauseData::Map {
            map_type: Some(MapType::To),
            mapper: None,
            items: vec![ClauseItem::from(Variable::with_sections(
                "b",
                vec![array_section],
            ))],
        };

        let directive = DirectiveIR::new(
            DirectiveKind::TargetData,
            "target data",
            vec![map_clause_1, map_clause_2],
            SourceLocation::start(),
            Language::C,
        );

        assert_eq!(
            directive.to_plain_string(),
            "#pragma omp target data map(tofrom: ) map(to: )"
        );
    }

    #[test]
    fn directive_plain_string_handles_mixed_clauses() {
        let clauses = vec![
            ClauseData::NumThreads {
                num: Expression::unparsed("4"),
            },
            ClauseData::If {
                directive_name: Some(Identifier::new("parallel")),
                condition: Expression::unparsed("flag"),
            },
            ClauseData::Reduction {
                operator: ReductionOperator::Add,
                items: vec![ClauseItem::from(Identifier::new("sum"))],
            },
            ClauseData::Schedule {
                kind: ScheduleKind::Static,
                modifiers: vec![ScheduleModifier::Monotonic],
                chunk_size: Some(Expression::unparsed("16")),
            },
        ];

        let directive = DirectiveIR::new(
            DirectiveKind::ParallelFor,
            "parallel for",
            clauses,
            SourceLocation::start(),
            Language::C,
        );

        assert_eq!(
            directive.to_plain_string(),
            "#pragma omp parallel for num_threads() if(parallel: ) reduction(+: ) schedule(monotonic: static, )"
        );
    }

    #[test]
    fn directive_plain_string_uses_language_prefix() {
        let directive = DirectiveIR::simple(
            DirectiveKind::Barrier,
            "barrier",
            SourceLocation::start(),
            Language::Fortran,
        );

        assert_eq!(directive.to_plain_string(), "!$omp barrier");
    }
}
