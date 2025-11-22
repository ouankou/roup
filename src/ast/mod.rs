//! Enum-based AST shared between the parser, IR, and compat layers.
//!
//! This module defines language-specific directive/clause enums plus the
//! strongly typed payload structures that downstream consumers will rely on.
//! The goal is to eliminate every post-parse string/number inspection so the
//! parser becomes the single place that interprets tokens.

use std::convert::TryFrom;

use crate::ir::{ClauseData, ClauseItem, Expression, Identifier, SourceLocation, Variable};
use crate::parser::directive_kind::DirectiveName;
use crate::parser::ClauseName;

/// Re-export OpenMP clause payload primitives so downstream users can
/// centralize on this module while the eventual refactor migrates more IR
/// definitions here.
pub type OmpClausePayload = ClauseData;
pub type OmpClauseItem = ClauseItem;
pub type OmpIdentifier = Identifier;
pub type OmpVariable = Variable;

/// Language identifier used throughout the enum-based AST.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub enum RoupLanguage {
    OpenMp = 0,
    OpenAcc = 1,
}

/// Clause normalization strategy (mirrors ompparser/accparser behavior).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ClauseNormalizationMode {
    /// Keep clauses exactly as written (no merging).
    Disabled,
    /// Merge compatible clauses by concatenating their variable lists.
    MergeVariableLists,
    /// Match the upstream ompparser/accparser defaults.
    #[default]
    ParserParity,
}

/// Top-level directive wrapper that records the language and source location.
#[derive(Debug, Clone)]
pub struct RoupDirective {
    pub language: RoupLanguage,
    pub source: SourceLocation,
    pub body: DirectiveBody,
}

/// Language-specific directive bodies.
#[derive(Debug, Clone)]
pub enum DirectiveBody {
    OpenMp(OmpDirective),
    OpenAcc(AccDirective),
}

/// Fully structured OpenMP directive.
#[derive(Debug, Clone, PartialEq)]
pub struct OmpDirective {
    pub kind: OmpDirectiveKind,
    pub parameter: Option<OmpDirectiveParameter>,
    pub clauses: Vec<OmpClause>,
}

/// Fully structured OpenACC directive.
#[derive(Debug, Clone, PartialEq)]
pub struct AccDirective {
    pub kind: AccDirectiveKind,
    pub parameter: Option<AccDirectiveParameter>,
    pub clauses: Vec<AccClause>,
}

/// Typed OpenMP clause record.
#[derive(Debug, Clone, PartialEq)]
pub struct OmpClause {
    pub kind: OmpClauseKind,
    pub payload: OmpClausePayload,
}

/// Typed OpenACC clause record.
#[derive(Debug, Clone, PartialEq)]
pub struct AccClause {
    pub kind: AccClauseKind,
    pub payload: AccClausePayload,
}

/// Metadirective selector payload (typed, no post-parse strings).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OmpSelector {
    pub device: Option<OmpSelectorDevice>,
    pub implementation: Option<OmpSelectorImpl>,
    pub user: Option<OmpSelectorUser>,
    pub constructs: Option<OmpSelectorConstructs>,
    pub nested_directive: Option<Box<OmpDirective>>,
    pub is_target_device: bool,
    pub raw: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct OmpSelectorDevice {
    pub kind: Option<OmpSelectorScoredValue>,
    pub isa: Vec<OmpSelectorScoredValue>,
    pub arch: Vec<OmpSelectorScoredValue>,
    pub device_num: Option<Expression>,
    pub device_num_score: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct OmpSelectorImpl {
    pub vendor: Option<String>,
    pub extensions: Vec<String>,
    pub vendor_score: Option<String>,
    pub extension_scores: Vec<Option<String>>,
    pub user_expression: Option<String>,
    pub user_expression_score: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct OmpSelectorUser {
    pub condition: Option<Expression>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct OmpSelectorConstructs {
    pub constructs: Vec<OmpSelectorConstruct>,
    pub scores: Vec<Option<String>>,
}

/// Selector value with optional score annotation.
#[derive(Debug, Clone, PartialEq)]
pub struct OmpSelectorScoredValue {
    pub score: Option<String>,
    pub value: String,
}

/// Construct selector entry with optional score.
#[derive(Debug, Clone, PartialEq)]
pub struct OmpSelectorConstruct {
    pub score: Option<String>,
    pub kind: OmpDirectiveKind,
    pub directive: Box<OmpDirective>,
}

/// Additional syntax carried by OpenMP directives that accept custom
/// parameters outside the clause stream.
#[derive(Debug, Clone, PartialEq)]
pub enum OmpDirectiveParameter {
    IdentifierList(Vec<OmpIdentifier>),
    Identifier(OmpIdentifier),
    Mapper(OmpIdentifier),
    DeclareMapper(OmpDeclareMapper),
    VariantFunction(OmpIdentifier),
    Depobj(OmpIdentifier),
    Scan(OmpScanDirective),
    Construct(OmpConstructType),
    CriticalSection(OmpIdentifier),
    FlushList(Vec<OmpIdentifier>),
    DeclareReduction(OmpDeclareReduction),
    DeclareSimd(OmpSimdTarget),
}

/// OpenMP constructs accepted by `cancel` / `cancellation point` parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OmpConstructType {
    Parallel,
    Sections,
    For,
    Taskgroup,
    Other(String),
}

/// Structured data for `scan` directives.
#[derive(Debug, Clone, PartialEq)]
pub struct OmpScanDirective {
    pub mode: OmpScanMode,
    pub variables: Vec<OmpIdentifier>,
}

/// Scan mode (exclusive/inclusive).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OmpScanMode {
    Exclusive,
    Inclusive,
}

/// Declare reduction signature.
#[derive(Debug, Clone, PartialEq)]
pub struct OmpDeclareReduction {
    pub operator: ReductionOperatorToken,
    pub type_names: Vec<String>,
    pub combiner: String,
    pub initializer: Option<String>,
}

/// Reduction operator token (builtin or user-defined identifier).
#[derive(Debug, Clone, PartialEq)]
pub enum ReductionOperatorToken {
    Builtin(crate::ir::ReductionOperator),
    Identifier(OmpIdentifier),
}

/// Declare simd target (optional function identifier).
#[derive(Debug, Clone, PartialEq)]
pub struct OmpSimdTarget {
    pub function: Option<OmpIdentifier>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OmpDeclareMapperId {
    Unspecified,
    Default,
    User(OmpIdentifier),
}

#[derive(Debug, Clone, PartialEq)]
pub struct OmpDeclareMapper {
    pub identifier: OmpDeclareMapperId,
    pub type_name: String,
    pub variable: String,
}

/// OpenACC directive parameter payloads.
#[derive(Debug, Clone, PartialEq)]
pub enum AccDirectiveParameter {
    Cache(AccCacheDirective),
    Wait(AccWaitDirective),
    Routine(AccRoutineDirective),
    End(AccDirectiveKind),
}

/// `cache` directive payload.
#[derive(Debug, Clone, PartialEq)]
pub struct AccCacheDirective {
    pub readonly: bool,
    pub variables: Vec<Identifier>,
}

/// `wait` directive payload.
#[derive(Debug, Clone, PartialEq)]
pub struct AccWaitDirective {
    pub devnum: Option<Expression>,
    pub queues: Vec<Expression>,
    pub explicit_queues: bool,
}

/// `routine` directive payload (optional binding).
#[derive(Debug, Clone, PartialEq)]
pub struct AccRoutineDirective {
    pub name: Option<Identifier>,
}

/// OpenACC clause payloads covering the clauses that require structured data.
#[derive(Debug, Clone, PartialEq)]
pub enum AccClausePayload {
    Bare,
    Expression(Expression),
    IdentifierList(Vec<Identifier>),
    Default(AccDefaultKind),
    Copy(AccCopyClause),
    Create(AccCreateClause),
    Data(AccDataClause),
    DeviceType(Vec<String>),
    Gang(AccGangClause),
    Worker(AccGangClause),
    Vector(AccGangClause),
    Wait(AccWaitClause),
    Reduction(AccReductionClause),
}

/// Copy-like clause payload (`copy`, `pcopy`, `present_or_copy`).
#[derive(Debug, Clone, PartialEq)]
pub struct AccCopyClause {
    pub kind: AccCopyKind,
    pub modifier: Option<AccCopyModifier>,
    pub variables: Vec<Identifier>,
}

/// Create-like clause payload (`create`, `pcreate`, `present_or_create`).
#[derive(Debug, Clone, PartialEq)]
pub struct AccCreateClause {
    pub kind: AccCreateKind,
    pub modifier: Option<AccCreateModifier>,
    pub variables: Vec<Identifier>,
}

/// Generic data movement clauses (`attach`, `detach`, `link`, etc.).
#[derive(Debug, Clone, PartialEq)]
pub struct AccDataClause {
    pub kind: AccDataKind,
    pub variables: Vec<Identifier>,
}

/// Gang/worker/vector clause payload with optional modifiers.
#[derive(Debug, Clone, PartialEq)]
pub struct AccGangClause {
    pub modifier: Option<String>,
    pub values: Vec<Expression>,
}

/// Wait clause payload (mirrors directive form).
#[derive(Debug, Clone, PartialEq)]
pub struct AccWaitClause {
    pub devnum: Option<Expression>,
    pub queues: Vec<Expression>,
    pub explicit_queues: bool,
}

/// OpenACC reduction clause payload.
#[derive(Debug, Clone, PartialEq)]
pub struct AccReductionClause {
    pub operator: String,
    pub variables: Vec<Identifier>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccDefaultKind {
    Unspecified,
    None,
    Present,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccCopyKind {
    Copy,
    PCopy,
    PresentOrCopy,
    CopyIn,
    PCopyIn,
    PresentOrCopyIn,
    CopyOut,
    PCopyOut,
    PresentOrCopyOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccCopyModifier {
    Readonly,
    Zero,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccCreateKind {
    Create,
    PCreate,
    PresentOrCreate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccCreateModifier {
    Zero,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccDataKind {
    Attach,
    Detach,
    UseDevice,
    Link,
    DeviceResident,
    Host,
    Device,
    Delete,
}

// -------------------------------------------------------------------------
// Enum generation helpers (data sourced from parser tables)
// -------------------------------------------------------------------------

macro_rules! define_omp_directive_kind {
    ($( $variant:ident => $name:expr ),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum OmpDirectiveKind {
            $( $variant, )+
        }

        impl OmpDirectiveKind {
            pub const ALL: &'static [OmpDirectiveKind] = &[ $( OmpDirectiveKind::$variant, )+ ];

            pub const fn as_str(self) -> &'static str {
                match self {
                    $( OmpDirectiveKind::$variant => $name, )+
                }
            }
        }

        impl From<OmpDirectiveKind> for DirectiveName {
            fn from(kind: OmpDirectiveKind) -> Self {
                match kind {
                    $( OmpDirectiveKind::$variant => DirectiveName::$variant, )+
                }
            }
        }

        impl TryFrom<DirectiveName> for OmpDirectiveKind {
            type Error = DirectiveName;

            fn try_from(value: DirectiveName) -> Result<Self, DirectiveName> {
                match value {
                    $( DirectiveName::$variant => Ok(OmpDirectiveKind::$variant), )+
                    other => Err(other),
                }
            }
        }
    };
}

macro_rules! define_omp_clause_kind {
    ($( $variant:ident => $name:expr ),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum OmpClauseKind {
            $( $variant, )+
        }

        impl OmpClauseKind {
            pub const ALL: &'static [OmpClauseKind] = &[ $( OmpClauseKind::$variant, )+ ];

            pub const fn as_str(self) -> &'static str {
                match self {
                    $( OmpClauseKind::$variant => $name, )+
                }
            }
        }

        impl From<OmpClauseKind> for ClauseName {
            fn from(kind: OmpClauseKind) -> Self {
                match kind {
                    $( OmpClauseKind::$variant => ClauseName::$variant, )+
                }
            }
        }

        impl TryFrom<ClauseName> for OmpClauseKind {
            type Error = ClauseName;

            fn try_from(value: ClauseName) -> Result<Self, ClauseName> {
                match value {
                    $( ClauseName::$variant => Ok(OmpClauseKind::$variant), )+
                    other => Err(other),
                }
            }
        }
    };
}

macro_rules! define_acc_directive_kind {
    ($( $variant:ident => $name:expr ),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum AccDirectiveKind {
            $( $variant, )+
        }

        impl AccDirectiveKind {
            pub const ALL: &'static [AccDirectiveKind] = &[ $( AccDirectiveKind::$variant, )+ ];

            pub const fn as_str(self) -> &'static str {
                match self {
                    $( AccDirectiveKind::$variant => $name, )+
                }
            }
        }

        impl From<AccDirectiveKind> for DirectiveName {
            fn from(kind: AccDirectiveKind) -> Self {
                match kind {
                    $( AccDirectiveKind::$variant => DirectiveName::$variant, )+
                }
            }
        }

        impl TryFrom<DirectiveName> for AccDirectiveKind {
            type Error = DirectiveName;

            fn try_from(value: DirectiveName) -> Result<Self, DirectiveName> {
                match value {
                    $( DirectiveName::$variant => Ok(AccDirectiveKind::$variant), )+
                    other => Err(other),
                }
            }
        }
    };
}

macro_rules! define_acc_clause_kind {
    ($( $variant:ident => $name:expr ),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum AccClauseKind {
            $( $variant, )+
        }

        impl AccClauseKind {
            pub const ALL: &'static [AccClauseKind] = &[ $( AccClauseKind::$variant, )+ ];

            pub const fn as_str(self) -> &'static str {
                match self {
                    $( AccClauseKind::$variant => $name, )+
                }
            }
        }

        impl From<AccClauseKind> for ClauseName {
            fn from(kind: AccClauseKind) -> Self {
                match kind {
                    $( AccClauseKind::$variant => ClauseName::$variant, )+
                }
            }
        }

        impl TryFrom<ClauseName> for AccClauseKind {
            type Error = ClauseName;

            fn try_from(value: ClauseName) -> Result<Self, ClauseName> {
                match value {
                    $( ClauseName::$variant => Ok(AccClauseKind::$variant), )+
                    other => Err(other),
                }
            }
        }
    };
}

// --- Data generated
define_omp_directive_kind! {
    Allocate => "allocate",
    Allocators => "allocators",
    Assume => "assume",
    Assumes => "assumes",
    Atomic => "atomic",
    AtomicCapture => "atomic capture",
    AtomicCompareCapture => "atomic compare capture",
    AtomicRead => "atomic read",
    AtomicUpdate => "atomic update",
    AtomicWrite => "atomic write",
    Barrier => "barrier",
    BeginAssumes => "begin assumes",
    BeginDeclareTarget => "begin declare target",
    BeginDeclareVariant => "begin declare variant",
    Cancel => "cancel",
    CancellationPoint => "cancellation point",
    Critical => "critical",
    DeclareInduction => "declare induction",
    DeclareMapper => "declare mapper",
    DeclareReduction => "declare reduction",
    DeclareSimd => "declare simd",
    DeclareTarget => "declare target",
    DeclareVariant => "declare variant",
    Depobj => "depobj",
    Dispatch => "dispatch",
    Distribute => "distribute",
    DistributeParallelDo => "distribute parallel do",
    DistributeParallelDoSimd => "distribute parallel do simd",
    DistributeParallelFor => "distribute parallel for",
    DistributeParallelForSimd => "distribute parallel for simd",
    DistributeParallelLoop => "distribute parallel loop",
    DistributeParallelLoopSimd => "distribute parallel loop simd",
    DistributeSimd => "distribute simd",
    Do => "do",
    DoSimd => "do simd",
    EndAssume => "end assume",
    EndAssumes => "end assumes",
    End => "end",
    EndDeclareTarget => "end declare target",
    EndDeclareVariant => "end declare variant",
    EndParallel => "end parallel",
    EndDo => "end do",
    EndSimd => "end simd",
    EndSections => "end sections",
    EndSingle => "end single",
    EndWorkshare => "end workshare",
    EndOrdered => "end ordered",
    EndLoop => "end loop",
    EndDistribute => "end distribute",
    EndTeams => "end teams",
    EndTaskloop => "end taskloop",
    EndTask => "end task",
    EndTaskgroup => "end taskgroup",
    EndMaster => "end master",
    EndMasked => "end masked",
    EndUnroll => "end unroll",
    EndCritical => "end critical",
    EndAtomic => "end atomic",
    EndParallelDo => "end parallel do",
    EndParallelFor => "end parallel for",
    EndParallelSections => "end parallel sections",
    EndParallelWorkshare => "end parallel workshare",
    EndParallelMaster => "end parallel master",
    EndParallelMasterTaskloop => "end parallel master taskloop",
    EndParallelMasterTaskloopSimd => "end parallel master taskloop simd",
    EndDoSimd => "end do simd",
    EndForSimd => "end for simd",
    EndParallelDoSimd => "end parallel do simd",
    EndParallelForSimd => "end parallel for simd",
    EndDistributeSimd => "end distribute simd",
    EndDistributeParallelDo => "end distribute parallel do",
    EndDistributeParallelDoSimd => "end distribute parallel do simd",
    EndDistributeParallelFor => "end distribute parallel for",
    EndDistributeParallelForSimd => "end distribute parallel for simd",
    EndTargetParallel => "end target parallel",
    EndTargetParallelDo => "end target parallel do",
    EndTargetParallelDoSimd => "end target parallel do simd",
    EndTargetParallelFor => "end target parallel for",
    EndTargetParallelForSimd => "end target parallel for simd",
    EndTargetParallelLoop => "end target parallel loop",
    EndTargetSimd => "end target simd",
    EndTargetTeams => "end target teams",
    EndTargetTeamsDistribute => "end target teams distribute",
    EndTargetTeamsDistributeParallelDo => "end target teams distribute parallel do",
    EndTargetTeamsDistributeParallelDoSimd => "end target teams distribute parallel do simd",
    EndTargetTeamsDistributeParallelFor => "end target teams distribute parallel for",
    EndTargetTeamsDistributeParallelForSimd => "end target teams distribute parallel for simd",
    EndTargetTeamsDistributeSimd => "end target teams distribute simd",
    EndTargetTeamsLoop => "end target teams loop",
    EndTeamsDistribute => "end teams distribute",
    EndTeamsDistributeParallelDo => "end teams distribute parallel do",
    EndTeamsDistributeParallelDoSimd => "end teams distribute parallel do simd",
    EndTeamsDistributeParallelFor => "end teams distribute parallel for",
    EndTeamsDistributeParallelForSimd => "end teams distribute parallel for simd",
    EndTeamsDistributeSimd => "end teams distribute simd",
    EndTeamsLoop => "end teams loop",
    EndTaskloopSimd => "end taskloop simd",
    EndMasterTaskloop => "end master taskloop",
    EndMasterTaskloopSimd => "end master taskloop simd",
    EndMaskedTaskloop => "end masked taskloop",
    EndMaskedTaskloopSimd => "end masked taskloop simd",
    EndParallelMasked => "end parallel masked",
    EndParallelMaskedTaskloop => "end parallel masked taskloop",
    EndParallelMaskedTaskloopSimd => "end parallel masked taskloop simd",
    EndParallelLoop => "end parallel loop",
    EndTargetLoop => "end target loop",
    EndSection => "end section",
    EndTile => "end tile",
    Error => "error",
    Flush => "flush",
    Fuse => "fuse",
    Groupprivate => "groupprivate",
    For => "for",
    ForSimd => "for simd",
    Interchange => "interchange",
    Interop => "interop",
    Loop => "loop",
    Reverse => "reverse",
    Masked => "masked",
    MaskedTaskloop => "masked taskloop",
    MaskedTaskloopSimd => "masked taskloop simd",
    Master => "master",
    MasterTaskloop => "master taskloop",
    MasterTaskloopSimd => "master taskloop simd",
    Metadirective => "metadirective",
    BeginMetadirective => "begin metadirective",
    Nothing => "nothing",
    Ordered => "ordered",
    Parallel => "parallel",
    ParallelDo => "parallel do",
    ParallelDoSimd => "parallel do simd",
    ParallelFor => "parallel for",
    ParallelForSimd => "parallel for simd",
    ParallelLoop => "parallel loop",
    ParallelLoopSimd => "parallel loop simd",
    ParallelMasked => "parallel masked",
    ParallelMaskedTaskloop => "parallel masked taskloop",
    ParallelMaskedTaskloopSimd => "parallel masked taskloop simd",
    ParallelMaster => "parallel master",
    ParallelMasterTaskloop => "parallel master taskloop",
    ParallelMasterTaskloopSimd => "parallel master taskloop simd",
    ParallelSections => "parallel sections",
    ParallelSingle => "parallel single",
    ParallelWorkshare => "parallel workshare",
    Requires => "requires",
    Scope => "scope",
    Scan => "scan",
    Section => "section",
    Sections => "sections",
    Simd => "simd",
    Single => "single",
    Split => "split",
    Stripe => "stripe",
    Target => "target",
    TargetData => "target data",
    TargetDataComposite => "target data composite",
    TargetEnterData => "target enter data",
    TargetExitData => "target exit data",
    EndTarget => "end target",
    EndTargetData => "end target data",
    EndTargetEnterData => "end target enter data",
    EndTargetExitData => "end target exit data",
    EndTargetUpdate => "end target update",
    TargetLoop => "target loop",
    TargetLoopSimd => "target loop simd",
    TargetParallel => "target parallel",
    TargetParallelDo => "target parallel do",
    TargetParallelDoSimd => "target parallel do simd",
    TargetParallelFor => "target parallel for",
    TargetParallelForSimd => "target parallel for simd",
    TargetParallelLoop => "target parallel loop",
    TargetParallelLoopSimd => "target parallel loop simd",
    TargetSimd => "target simd",
    TargetTeams => "target teams",
    TargetTeamsDistribute => "target teams distribute",
    TargetTeamsDistributeParallelDo => "target teams distribute parallel do",
    TargetTeamsDistributeParallelDoSimd => "target teams distribute parallel do simd",
    TargetTeamsDistributeParallelFor => "target teams distribute parallel for",
    TargetTeamsDistributeParallelForSimd => "target teams distribute parallel for simd",
    TargetTeamsDistributeParallelLoop => "target teams distribute parallel loop",
    TargetTeamsDistributeParallelLoopSimd => "target teams distribute parallel loop simd",
    TargetTeamsDistributeSimd => "target teams distribute simd",
    TargetTeamsLoop => "target teams loop",
    TargetTeamsLoopSimd => "target teams loop simd",
    TargetUpdate => "target update",
    Task => "task",
    TaskIteration => "task iteration",
    Taskgroup => "taskgroup",
    Taskgraph => "taskgraph",
    Taskloop => "taskloop",
    TaskloopSimd => "taskloop simd",
    Taskwait => "taskwait",
    Taskyield => "taskyield",
    Teams => "teams",
    TeamsDistribute => "teams distribute",
    TeamsDistributeParallelDo => "teams distribute parallel do",
    TeamsDistributeParallelDoSimd => "teams distribute parallel do simd",
    TeamsDistributeParallelFor => "teams distribute parallel for",
    TeamsDistributeParallelForSimd => "teams distribute parallel for simd",
    TeamsDistributeParallelLoop => "teams distribute parallel loop",
    TeamsDistributeParallelLoopSimd => "teams distribute parallel loop simd",
    TeamsDistributeSimd => "teams distribute simd",
    TeamsLoop => "teams loop",
    TeamsLoopSimd => "teams loop simd",
    Threadprivate => "threadprivate",
    Tile => "tile",
    Unroll => "unroll",
    Workdistribute => "workdistribute",
    Workshare => "workshare",
}

define_omp_clause_kind! {
    Absent => "absent",
    AcqRel => "acq_rel",
    Acquire => "acquire",
    AdjustArgs => "adjust_args",
    Affinity => "affinity",
    Align => "align",
    Aligned => "aligned",
    Allocate => "allocate",
    Allocator => "allocator",
    AppendArgs => "append_args",
    Apply => "apply",
    At => "at",
    Bind => "bind",
    Capture => "capture",
    Collapse => "collapse",
    Collector => "collector",
    Combiner => "combiner",
    Compare => "compare",
    Contains => "contains",
    CopyIn => "copyin",
    Copyprivate => "copyprivate",
    Counts => "counts",
    Default => "default",
    Defaultmap => "defaultmap",
    Depend => "depend",
    Destroy => "destroy",
    Detach => "detach",
    Device => "device",
    DeviceResident => "device_resident",
    DeviceSafesync => "device_safesync",
    DeviceType => "device_type",
    DistSchedule => "dist_schedule",
    Doacross => "doacross",
    DynamicAllocators => "dynamic_allocators",
    Enter => "enter",
    Exclusive => "exclusive",
    Fail => "fail",
    Final => "final",
    Filter => "filter",
    Firstprivate => "firstprivate",
    From => "from",
    Full => "full",
    Grainsize => "grainsize",
    GraphId => "graph_id",
    GraphReset => "graph_reset",
    HasDeviceAddr => "has_device_addr",
    Hint => "hint",
    Holds => "holds",
    If => "if",
    InReduction => "in_reduction",
    Induction => "induction",
    Inductor => "inductor",
    Inbranch => "inbranch",
    Inclusive => "inclusive",
    Init => "init",
    InitComplete => "init_complete",
    Initializer => "initializer",
    Indirect => "indirect",
    IsDevicePtr => "is_device_ptr",
    Lastprivate => "lastprivate",
    Linear => "linear",
    Link => "link",
    Local => "local",
    Looprange => "looprange",
    Map => "map",
    Match => "match",
    Message => "message",
    Memscope => "memscope",
    Mergeable => "mergeable",
    Nocontext => "nocontext",
    Nogroup => "nogroup",
    NoOpenmp => "no_openmp",
    NoOpenmpConstructs => "no_openmp_constructs",
    NoOpenmpRoutines => "no_openmp_routines",
    NoParallelism => "no_parallelism",
    Nontemporal => "nontemporal",
    Notinbranch => "notinbranch",
    Novariants => "novariants",
    Nowait => "nowait",
    NumTasks => "num_tasks",
    NumTeams => "num_teams",
    NumThreads => "num_threads",
    Order => "order",
    Ordered => "ordered",
    Otherwise => "otherwise",
    Partial => "partial",
    Permutation => "permutation",
    Priority => "priority",
    Private => "private",
    ProcBind => "proc_bind",
    Read => "read",
    Reduction => "reduction",
    Release => "release",
    Relaxed => "relaxed",
    Replayable => "replayable",
    Requires => "requires",
    ReverseOffload => "reverse_offload",
    Safelen => "safelen",
    Safesync => "safesync",
    Schedule => "schedule",
    SelfMaps => "self_maps",
    SeqCst => "seq_cst",
    Severity => "severity",
    Shared => "shared",
    Simd => "simd",
    Simdlen => "simdlen",
    Sizes => "sizes",
    TaskReduction => "task_reduction",
    ThreadLimit => "thread_limit",
    Threads => "threads",
    Threadset => "threadset",
    Tile => "tile",
    To => "to",
    Transparent => "transparent",
    UnifiedAddress => "unified_address",
    UnifiedSharedMemory => "unified_shared_memory",
    Uniform => "uniform",
    Untied => "untied",
    Update => "update",
    Use => "use",
    UseDeviceAddr => "use_device_addr",
    UseDevicePtr => "use_device_ptr",
    UsesAllocators => "uses_allocators",
    Weak => "weak",
    When => "when",
    Write => "write",
}

define_acc_directive_kind! {
    Atomic => "atomic",
    Cache => "cache",
    Data => "data",
    Declare => "declare",
    End => "end",
    EnterData => "enter data",
    ExitData => "exit data",
    HostData => "host_data",
    Init => "init",
    Kernels => "kernels",
    KernelsLoop => "kernels loop",
    Loop => "loop",
    Parallel => "parallel",
    ParallelLoop => "parallel loop",
    Routine => "routine",
    Serial => "serial",
    SerialLoop => "serial loop",
    Set => "set",
    Shutdown => "shutdown",
    Update => "update",
    Wait => "wait",
}

define_acc_clause_kind! {
    Async => "async",
    Attach => "attach",
    Auto => "auto",
    Bind => "bind",
    Capture => "capture",
    Collapse => "collapse",
    Copy => "copy",
    CopyIn => "copyin",
    CopyOut => "copyout",
    Create => "create",
    Default => "default",
    DefaultAsync => "default_async",
    Delete => "delete",
    Detach => "detach",
    Device => "device",
    DeviceNum => "device_num",
    DeviceResident => "device_resident",
    DeviceType => "device_type",
    DevicePtr => "deviceptr",
    Finalize => "finalize",
    Firstprivate => "firstprivate",
    Gang => "gang",
    Host => "host",
    If => "if",
    IfPresent => "if_present",
    Independent => "independent",
    Link => "link",
    NoCreate => "no_create",
    NoHost => "nohost",
    NumGangs => "num_gangs",
    NumWorkers => "num_workers",
    Present => "present",
    Private => "private",
    Reduction => "reduction",
    Read => "read",
    SelfClause => "self",
    Seq => "seq",
    Tile => "tile",
    Update => "update",
    UseDevice => "use_device",
    Vector => "vector",
    VectorLength => "vector_length",
    Wait => "wait",
    Worker => "worker",
    Write => "write",
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omp_directive_conversion_round_trip() {
        for kind in OmpDirectiveKind::ALL {
            let dn: DirectiveName = (*kind).into();
            let back = OmpDirectiveKind::try_from(dn.clone()).expect("should convert");
            assert_eq!(*kind, back);
        }
    }

    #[test]
    fn acc_clause_conversion_round_trip() {
        for kind in AccClauseKind::ALL {
            let cn: ClauseName = (*kind).into();
            let back = AccClauseKind::try_from(cn.clone()).expect("should convert");
            assert_eq!(*kind, back);
        }
    }
}
