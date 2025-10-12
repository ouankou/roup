use super::{
    ClauseRegistry, ClauseRegistryBuilder, ClauseRule, DirectiveRegistry, DirectiveRegistryBuilder,
    Parser,
};

const OPENMP_DEFAULT_CLAUSE_RULE: ClauseRule = ClauseRule::Unsupported;

macro_rules! openmp_clauses {
    ($( $variant:ident => { name: $name:literal, rule: $rule:expr } ),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum OpenMpClause {
            $( $variant, )+
        }

        impl OpenMpClause {
            pub const ALL: &'static [OpenMpClause] = &[ $( OpenMpClause::$variant, )+ ];

            pub const fn name(self) -> &'static str {
                match self {
                    $( OpenMpClause::$variant => $name, )+
                }
            }

            pub const fn rule(self) -> ClauseRule {
                match self {
                    $( OpenMpClause::$variant => $rule, )+
                }
            }
        }
    };
}

openmp_clauses! {
    AcqRel => { name: "acq_rel", rule: ClauseRule::Bare },
    Acquire => { name: "acquire", rule: ClauseRule::Bare },
    Affinity => { name: "affinity", rule: ClauseRule::Parenthesized },
    Aligned => { name: "aligned", rule: ClauseRule::Parenthesized },
    Allocate => { name: "allocate", rule: ClauseRule::Parenthesized },
    Allocator => { name: "allocator", rule: ClauseRule::Parenthesized },
    AtomicDefaultMemOrder => { name: "atomic_default_mem_order", rule: ClauseRule::Parenthesized },
    Bind => { name: "bind", rule: ClauseRule::Parenthesized },
    Capture => { name: "capture", rule: ClauseRule::Flexible },
    Collapse => { name: "collapse", rule: ClauseRule::Parenthesized },
    Compare => { name: "compare", rule: ClauseRule::Flexible },
    Copyin => { name: "copyin", rule: ClauseRule::Parenthesized },
    Copyprivate => { name: "copyprivate", rule: ClauseRule::Parenthesized },
    Default => { name: "default", rule: ClauseRule::Parenthesized },
    Defaultmap => { name: "defaultmap", rule: ClauseRule::Parenthesized },
    Depend => { name: "depend", rule: ClauseRule::Parenthesized },
    Destroy => { name: "destroy", rule: ClauseRule::Flexible },
    Detach => { name: "detach", rule: ClauseRule::Parenthesized },
    Device => { name: "device", rule: ClauseRule::Parenthesized },
    DeviceResident => { name: "device_resident", rule: ClauseRule::Parenthesized },
    DeviceType => { name: "device_type", rule: ClauseRule::Parenthesized },
    DistSchedule => { name: "dist_schedule", rule: ClauseRule::Parenthesized },
    Doacross => { name: "doacross", rule: ClauseRule::Parenthesized },
    DynamicAllocators => { name: "dynamic_allocators", rule: ClauseRule::Bare },
    Exclusive => { name: "exclusive", rule: ClauseRule::Bare },
    Fail => { name: "fail", rule: ClauseRule::Flexible },
    Final => { name: "final", rule: ClauseRule::Parenthesized },
    Filter => { name: "filter", rule: ClauseRule::Parenthesized },
    Firstprivate => { name: "firstprivate", rule: ClauseRule::Parenthesized },
    From => { name: "from", rule: ClauseRule::Parenthesized },
    Grainsize => { name: "grainsize", rule: ClauseRule::Parenthesized },
    Hint => { name: "hint", rule: ClauseRule::Parenthesized },
    Holds => { name: "holds", rule: ClauseRule::Parenthesized },
    If => { name: "if", rule: ClauseRule::Parenthesized },
    InReduction => { name: "in_reduction", rule: ClauseRule::Parenthesized },
    Inbranch => { name: "inbranch", rule: ClauseRule::Bare },
    Inclusive => { name: "inclusive", rule: ClauseRule::Bare },
    Init => { name: "init", rule: ClauseRule::Parenthesized },
    Interop => { name: "interop", rule: ClauseRule::Parenthesized },
    IsDevicePtr => { name: "is_device_ptr", rule: ClauseRule::Parenthesized },
    Label => { name: "label", rule: ClauseRule::Parenthesized },
    Lastprivate => { name: "lastprivate", rule: ClauseRule::Parenthesized },
    Linear => { name: "linear", rule: ClauseRule::Parenthesized },
    Link => { name: "link", rule: ClauseRule::Parenthesized },
    Map => { name: "map", rule: ClauseRule::Parenthesized },
    Match => { name: "match", rule: ClauseRule::Parenthesized },
    Message => { name: "message", rule: ClauseRule::Parenthesized },
    Mergeable => { name: "mergeable", rule: ClauseRule::Bare },
    Nogroup => { name: "nogroup", rule: ClauseRule::Bare },
    NoOpenmp => { name: "no_openmp", rule: ClauseRule::Flexible },
    NoOpenmpRoutines => { name: "no_openmp_routines", rule: ClauseRule::Flexible },
    NoParallelism => { name: "no_parallelism", rule: ClauseRule::Flexible },
    Notinbranch => { name: "notinbranch", rule: ClauseRule::Bare },
    Novariants => { name: "novariants", rule: ClauseRule::Flexible },
    Nowait => { name: "nowait", rule: ClauseRule::Bare },
    NumTasks => { name: "num_tasks", rule: ClauseRule::Parenthesized },
    NumTeams => { name: "num_teams", rule: ClauseRule::Parenthesized },
    NumThreads => { name: "num_threads", rule: ClauseRule::Parenthesized },
    Nontemporal => { name: "nontemporal", rule: ClauseRule::Parenthesized },
    Order => { name: "order", rule: ClauseRule::Parenthesized },
    Ordered => { name: "ordered", rule: ClauseRule::Flexible },
    Partial => { name: "partial", rule: ClauseRule::Flexible },
    Priority => { name: "priority", rule: ClauseRule::Parenthesized },
    Private => { name: "private", rule: ClauseRule::Parenthesized },
    ProcBind => { name: "proc_bind", rule: ClauseRule::Parenthesized },
    Public => { name: "public", rule: ClauseRule::Flexible },
    Reduction => { name: "reduction", rule: ClauseRule::Parenthesized },
    Release => { name: "release", rule: ClauseRule::Bare },
    Relaxed => { name: "relaxed", rule: ClauseRule::Bare },
    Reverse => { name: "reverse", rule: ClauseRule::Flexible },
    Reproducible => { name: "reproducible", rule: ClauseRule::Bare },
    Safelen => { name: "safelen", rule: ClauseRule::Parenthesized },
    Schedule => { name: "schedule", rule: ClauseRule::Parenthesized },
    SeqCst => { name: "seq_cst", rule: ClauseRule::Bare },
    Shared => { name: "shared", rule: ClauseRule::Parenthesized },
    Simdlen => { name: "simdlen", rule: ClauseRule::Parenthesized },
    Sizes => { name: "sizes", rule: ClauseRule::Parenthesized },
    TaskReduction => { name: "task_reduction", rule: ClauseRule::Parenthesized },
    ThreadLimit => { name: "thread_limit", rule: ClauseRule::Parenthesized },
    Tile => { name: "tile", rule: ClauseRule::Parenthesized },
    To => { name: "to", rule: ClauseRule::Parenthesized },
    UnifiedAddress => { name: "unified_address", rule: ClauseRule::Flexible },
    UnifiedSharedMemory => { name: "unified_shared_memory", rule: ClauseRule::Flexible },
    Unroll => { name: "unroll", rule: ClauseRule::Flexible },
    Untied => { name: "untied", rule: ClauseRule::Bare },
    Update => { name: "update", rule: ClauseRule::Flexible },
    UseDeviceAddr => { name: "use_device_addr", rule: ClauseRule::Parenthesized },
    UseDevicePtr => { name: "use_device_ptr", rule: ClauseRule::Parenthesized },
    UsesAllocators => { name: "uses_allocators", rule: ClauseRule::Parenthesized },
    Weak => { name: "weak", rule: ClauseRule::Flexible },
    When => { name: "when", rule: ClauseRule::Parenthesized },
}

macro_rules! openmp_directives {
    ($( $variant:ident => $name:literal ),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum OpenMpDirective {
            $( $variant, )+
        }

        impl OpenMpDirective {
            pub const ALL: &'static [OpenMpDirective] = &[ $( OpenMpDirective::$variant, )+ ];

            pub const fn as_str(self) -> &'static str {
                match self {
                    $( OpenMpDirective::$variant => $name, )+
                }
            }
        }
    };
}

openmp_directives! {
    Assume => "assume",
    Atomic => "atomic",
    AtomicCapture => "atomic capture",
    AtomicCompareCapture => "atomic compare capture",
    AtomicRead => "atomic read",
    AtomicUpdate => "atomic update",
    AtomicWrite => "atomic write",
    Barrier => "barrier",
    BeginDeclareTarget => "begin declare target",
    Cancel => "cancel",
    CancellationPoint => "cancellation point",
    Critical => "critical",
    DeclareMapper => "declare mapper",
    DeclareReduction => "declare reduction",
    DeclareSimd => "declare simd",
    DeclareTarget => "declare target",
    DeclareVariant => "declare variant",
    Depobj => "depobj",
    Dispatch => "dispatch",
    Distribute => "distribute",
    DistributeParallelFor => "distribute parallel for",
    DistributeParallelForSimd => "distribute parallel for simd",
    DistributeParallelLoop => "distribute parallel loop",
    DistributeParallelLoopSimd => "distribute parallel loop simd",
    DistributeSimd => "distribute simd",
    // --- Fortran variants ---
    DistributeParallelDo => "distribute parallel do",  // Fortran variant
    DistributeParallelDoSimd => "distribute parallel do simd",  // Fortran variant
    Do => "do",  // Fortran equivalent of FOR
    DoSimd => "do simd",  // Fortran equivalent of FOR SIMD
    EndDeclareTarget => "end declare target",
    Error => "error",
    Flush => "flush",
    For => "for",
    ForSimd => "for simd",
    Interop => "interop",
    Loop => "loop",
    Masked => "masked",
    MaskedTaskloop => "masked taskloop",
    MaskedTaskloopSimd => "masked taskloop simd",
    Master => "master",
    Metadirective => "metadirective",
    Nothing => "nothing",
    Ordered => "ordered",
    Parallel => "parallel",
    ParallelDo => "parallel do",  // Fortran equivalent of PARALLEL FOR
    ParallelDoSimd => "parallel do simd",  // Fortran equivalent of PARALLEL FOR SIMD
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
    Requires => "requires",
    Scope => "scope",
    Sections => "sections",
    Simd => "simd",
    Single => "single",
    Target => "target",
    TargetData => "target data",
    TargetEnterData => "target enter data",
    TargetExitData => "target exit data",
    TargetLoop => "target loop",
    TargetLoopSimd => "target loop simd",
    TargetParallel => "target parallel",
    TargetParallelDo => "target parallel do",  // Fortran variant
    TargetParallelDoSimd => "target parallel do simd",  // Fortran variant
    TargetParallelFor => "target parallel for",
    TargetParallelForSimd => "target parallel for simd",
    TargetParallelLoop => "target parallel loop",
    TargetParallelLoopSimd => "target parallel loop simd",
    TargetSimd => "target simd",
    TargetTeams => "target teams",
    TargetTeamsDistribute => "target teams distribute",
    TargetTeamsDistributeParallelDo => "target teams distribute parallel do",  // Fortran variant
    TargetTeamsDistributeParallelDoSimd => "target teams distribute parallel do simd",  // Fortran variant
    TargetTeamsDistributeParallelFor => "target teams distribute parallel for",
    TargetTeamsDistributeParallelForSimd => "target teams distribute parallel for simd",
    TargetTeamsDistributeParallelLoop => "target teams distribute parallel loop",
    TargetTeamsDistributeParallelLoopSimd => "target teams distribute parallel loop simd",
    TargetTeamsDistributeSimd => "target teams distribute simd",
    TargetTeamsLoop => "target teams loop",
    TargetTeamsLoopSimd => "target teams loop simd",
    TargetUpdate => "target update",
    Task => "task",
    Taskgroup => "taskgroup",
    Taskgraph => "taskgraph",
    Taskloop => "taskloop",
    TaskloopSimd => "taskloop simd",
    Taskwait => "taskwait",
    Taskyield => "taskyield",
    Teams => "teams",
    TeamsDistribute => "teams distribute",
    TeamsDistributeParallelDo => "teams distribute parallel do",  // Fortran variant
    TeamsDistributeParallelDoSimd => "teams distribute parallel do simd",  // Fortran variant
    TeamsDistributeParallelFor => "teams distribute parallel for",
    TeamsDistributeParallelForSimd => "teams distribute parallel for simd",
    TeamsDistributeParallelLoop => "teams distribute parallel loop",
    TeamsDistributeParallelLoopSimd => "teams distribute parallel loop simd",
    TeamsDistributeSimd => "teams distribute simd",
    TeamsLoop => "teams loop",
    TeamsLoopSimd => "teams loop simd",
    Threadprivate => "threadprivate",
}

pub fn clause_registry() -> ClauseRegistry {
    let mut builder = ClauseRegistryBuilder::new().with_default_rule(OPENMP_DEFAULT_CLAUSE_RULE);

    for clause in OpenMpClause::ALL {
        builder.register_with_rule_mut(clause.name(), clause.rule());
    }

    builder.build()
}

pub fn directive_registry() -> DirectiveRegistry {
    let mut builder = DirectiveRegistryBuilder::new();

    for directive in OpenMpDirective::ALL {
        builder = builder.register_generic(directive.as_str());
    }

    builder.build()
}

pub fn parser() -> Parser {
    Parser::new(directive_registry(), clause_registry())
}
