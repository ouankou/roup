use super::{
    ClauseRegistry, ClauseRegistryBuilder, ClauseRule, DirectiveRegistry, DirectiveRegistryBuilder,
    Parser,
};

const OPENMP_DEFAULT_CLAUSE_RULE: ClauseRule = ClauseRule::Unsupported;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OpenMpClause {
    Nowait,
    Untied,
    Mergeable,
    Inbranch,
    Notinbranch,
    Nogroup,
    Inclusive,
    Exclusive,
    Reproducible,
    Affinity,
    Aligned,
    Allocate,
    AtomicDefaultMemOrder,
    Bind,
    Collapse,
    Copyin,
    Copyprivate,
    Default,
    Defaultmap,
    DistSchedule,
    Depend,
    Detach,
    Device,
    DeviceType,
    DynamicAllocators,
    Final,
    Firstprivate,
    Grainsize,
    Hint,
    If,
    InReduction,
    IsDevicePtr,
    Label,
    Lastprivate,
    Linear,
    Map,
    Nontemporal,
    NumTasks,
    NumThreads,
    NumTeams,
    Order,
    Ordered,
    Priority,
    ProcBind,
    Private,
    Reduction,
    Safelen,
    Schedule,
    Shared,
    Simdlen,
    ThreadLimit,
    Tile,
    Unroll,
    UsesAllocators,
    UseDeviceAddr,
    UseDevicePtr,
}

impl OpenMpClause {
    pub const ALL: &'static [OpenMpClause] = &[
        OpenMpClause::Nowait,
        OpenMpClause::Untied,
        OpenMpClause::Mergeable,
        OpenMpClause::Inbranch,
        OpenMpClause::Notinbranch,
        OpenMpClause::Nogroup,
        OpenMpClause::Inclusive,
        OpenMpClause::Exclusive,
        OpenMpClause::Reproducible,
        OpenMpClause::Affinity,
        OpenMpClause::Aligned,
        OpenMpClause::Allocate,
        OpenMpClause::AtomicDefaultMemOrder,
        OpenMpClause::Bind,
        OpenMpClause::Collapse,
        OpenMpClause::Copyin,
        OpenMpClause::Copyprivate,
        OpenMpClause::Default,
        OpenMpClause::Defaultmap,
        OpenMpClause::DistSchedule,
        OpenMpClause::Depend,
        OpenMpClause::Detach,
        OpenMpClause::Device,
        OpenMpClause::DeviceType,
        OpenMpClause::DynamicAllocators,
        OpenMpClause::Final,
        OpenMpClause::Firstprivate,
        OpenMpClause::Grainsize,
        OpenMpClause::Hint,
        OpenMpClause::If,
        OpenMpClause::InReduction,
        OpenMpClause::IsDevicePtr,
        OpenMpClause::Label,
        OpenMpClause::Lastprivate,
        OpenMpClause::Linear,
        OpenMpClause::Map,
        OpenMpClause::Nontemporal,
        OpenMpClause::NumTasks,
        OpenMpClause::NumThreads,
        OpenMpClause::NumTeams,
        OpenMpClause::Order,
        OpenMpClause::Ordered,
        OpenMpClause::Priority,
        OpenMpClause::ProcBind,
        OpenMpClause::Private,
        OpenMpClause::Reduction,
        OpenMpClause::Safelen,
        OpenMpClause::Schedule,
        OpenMpClause::Shared,
        OpenMpClause::Simdlen,
        OpenMpClause::ThreadLimit,
        OpenMpClause::Tile,
        OpenMpClause::Unroll,
        OpenMpClause::UsesAllocators,
        OpenMpClause::UseDeviceAddr,
        OpenMpClause::UseDevicePtr,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            OpenMpClause::Nowait => "nowait",
            OpenMpClause::Untied => "untied",
            OpenMpClause::Mergeable => "mergeable",
            OpenMpClause::Inbranch => "inbranch",
            OpenMpClause::Notinbranch => "notinbranch",
            OpenMpClause::Nogroup => "nogroup",
            OpenMpClause::Inclusive => "inclusive",
            OpenMpClause::Exclusive => "exclusive",
            OpenMpClause::Reproducible => "reproducible",
            OpenMpClause::Affinity => "affinity",
            OpenMpClause::Aligned => "aligned",
            OpenMpClause::Allocate => "allocate",
            OpenMpClause::AtomicDefaultMemOrder => "atomic_default_mem_order",
            OpenMpClause::Bind => "bind",
            OpenMpClause::Collapse => "collapse",
            OpenMpClause::Copyin => "copyin",
            OpenMpClause::Copyprivate => "copyprivate",
            OpenMpClause::Default => "default",
            OpenMpClause::Defaultmap => "defaultmap",
            OpenMpClause::DistSchedule => "dist_schedule",
            OpenMpClause::Depend => "depend",
            OpenMpClause::Detach => "detach",
            OpenMpClause::Device => "device",
            OpenMpClause::DeviceType => "device_type",
            OpenMpClause::DynamicAllocators => "dynamic_allocators",
            OpenMpClause::Final => "final",
            OpenMpClause::Firstprivate => "firstprivate",
            OpenMpClause::Grainsize => "grainsize",
            OpenMpClause::Hint => "hint",
            OpenMpClause::If => "if",
            OpenMpClause::InReduction => "in_reduction",
            OpenMpClause::IsDevicePtr => "is_device_ptr",
            OpenMpClause::Label => "label",
            OpenMpClause::Lastprivate => "lastprivate",
            OpenMpClause::Linear => "linear",
            OpenMpClause::Map => "map",
            OpenMpClause::Nontemporal => "nontemporal",
            OpenMpClause::NumTasks => "num_tasks",
            OpenMpClause::NumThreads => "num_threads",
            OpenMpClause::NumTeams => "num_teams",
            OpenMpClause::Order => "order",
            OpenMpClause::Ordered => "ordered",
            OpenMpClause::Priority => "priority",
            OpenMpClause::ProcBind => "proc_bind",
            OpenMpClause::Private => "private",
            OpenMpClause::Reduction => "reduction",
            OpenMpClause::Safelen => "safelen",
            OpenMpClause::Schedule => "schedule",
            OpenMpClause::Shared => "shared",
            OpenMpClause::Simdlen => "simdlen",
            OpenMpClause::ThreadLimit => "thread_limit",
            OpenMpClause::Tile => "tile",
            OpenMpClause::Unroll => "unroll",
            OpenMpClause::UsesAllocators => "uses_allocators",
            OpenMpClause::UseDeviceAddr => "use_device_addr",
            OpenMpClause::UseDevicePtr => "use_device_ptr",
        }
    }

    pub const fn rule(self) -> ClauseRule {
        match self {
            OpenMpClause::Nowait
            | OpenMpClause::Untied
            | OpenMpClause::Mergeable
            | OpenMpClause::Inbranch
            | OpenMpClause::Notinbranch
            | OpenMpClause::Nogroup
            | OpenMpClause::Inclusive
            | OpenMpClause::Exclusive
            | OpenMpClause::Reproducible
            | OpenMpClause::DynamicAllocators => ClauseRule::Bare,
            // The 'ordered' and 'unroll' clauses can appear both with and without parentheses
            // (e.g.,'ordered' vs 'ordered(2)'), so we use the Flexible rule to support both forms.
            OpenMpClause::Ordered | OpenMpClause::Unroll => ClauseRule::Flexible,
            _ => ClauseRule::Parenthesized,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OpenMpDirective {
    Parallel,
    ParallelFor,
    ParallelForSimd,
    ParallelLoop,
    ParallelMasked,
    ParallelMaskedTaskloop,
    ParallelMaskedTaskloopSimd,
    ParallelMasterTaskloop,
    ParallelMasterTaskloopSimd,
    Target,
    TargetLoop,
    TargetSimd,
    TargetParallel,
    TargetParallelFor,
    TargetParallelForSimd,
    TargetParallelLoop,
    TargetTeams,
    TargetTeamsDistribute,
    TargetTeamsDistributeParallelFor,
    TargetTeamsDistributeParallelForSimd,
    TargetTeamsDistributeParallelLoop,
    TargetTeamsLoop,
    TargetTeamsLoopSimd,
    Teams,
    TeamsDistribute,
    TeamsDistributeParallelFor,
    TeamsDistributeParallelForSimd,
    TeamsDistributeParallelLoop,
    TeamsDistributeSimd,
    TeamsLoop,
    TeamsLoopSimd,
    For,
    ForSimd,
    Task,
    Taskloop,
    TaskloopSimd,
}

impl OpenMpDirective {
    pub const ALL: &'static [OpenMpDirective] = &[
        OpenMpDirective::Parallel,
        OpenMpDirective::ParallelFor,
        OpenMpDirective::ParallelForSimd,
        OpenMpDirective::ParallelLoop,
        OpenMpDirective::ParallelMasked,
        OpenMpDirective::ParallelMaskedTaskloop,
        OpenMpDirective::ParallelMaskedTaskloopSimd,
        OpenMpDirective::ParallelMasterTaskloop,
        OpenMpDirective::ParallelMasterTaskloopSimd,
        OpenMpDirective::Target,
        OpenMpDirective::TargetLoop,
        OpenMpDirective::TargetSimd,
        OpenMpDirective::TargetParallel,
        OpenMpDirective::TargetParallelFor,
        OpenMpDirective::TargetParallelForSimd,
        OpenMpDirective::TargetParallelLoop,
        OpenMpDirective::TargetTeams,
        OpenMpDirective::TargetTeamsDistribute,
        OpenMpDirective::TargetTeamsDistributeParallelFor,
        OpenMpDirective::TargetTeamsDistributeParallelForSimd,
        OpenMpDirective::TargetTeamsDistributeParallelLoop,
        OpenMpDirective::TargetTeamsLoop,
        OpenMpDirective::TargetTeamsLoopSimd,
        OpenMpDirective::Teams,
        OpenMpDirective::TeamsDistribute,
        OpenMpDirective::TeamsDistributeParallelFor,
        OpenMpDirective::TeamsDistributeParallelForSimd,
        OpenMpDirective::TeamsDistributeParallelLoop,
        OpenMpDirective::TeamsDistributeSimd,
        OpenMpDirective::TeamsLoop,
        OpenMpDirective::TeamsLoopSimd,
        OpenMpDirective::For,
        OpenMpDirective::ForSimd,
        OpenMpDirective::Task,
        OpenMpDirective::Taskloop,
        OpenMpDirective::TaskloopSimd,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            OpenMpDirective::Parallel => "parallel",
            OpenMpDirective::ParallelFor => "parallel for",
            OpenMpDirective::ParallelForSimd => "parallel for simd",
            OpenMpDirective::ParallelLoop => "parallel loop",
            OpenMpDirective::ParallelMasked => "parallel masked",
            OpenMpDirective::ParallelMaskedTaskloop => "parallel masked taskloop",
            OpenMpDirective::ParallelMaskedTaskloopSimd => "parallel masked taskloop simd",
            OpenMpDirective::ParallelMasterTaskloop => "parallel master taskloop",
            OpenMpDirective::ParallelMasterTaskloopSimd => "parallel master taskloop simd",
            OpenMpDirective::Target => "target",
            OpenMpDirective::TargetLoop => "target loop",
            OpenMpDirective::TargetSimd => "target simd",
            OpenMpDirective::TargetParallel => "target parallel",
            OpenMpDirective::TargetParallelFor => "target parallel for",
            OpenMpDirective::TargetParallelForSimd => "target parallel for simd",
            OpenMpDirective::TargetParallelLoop => "target parallel loop",
            OpenMpDirective::TargetTeams => "target teams",
            OpenMpDirective::TargetTeamsDistribute => "target teams distribute",
            OpenMpDirective::TargetTeamsDistributeParallelFor => {
                "target teams distribute parallel for"
            }
            OpenMpDirective::TargetTeamsDistributeParallelForSimd => {
                "target teams distribute parallel for simd"
            }
            OpenMpDirective::TargetTeamsDistributeParallelLoop => {
                "target teams distribute parallel loop"
            }
            OpenMpDirective::TargetTeamsLoop => "target teams loop",
            OpenMpDirective::TargetTeamsLoopSimd => "target teams loop simd",
            OpenMpDirective::Teams => "teams",
            OpenMpDirective::TeamsDistribute => "teams distribute",
            OpenMpDirective::TeamsDistributeParallelFor => "teams distribute parallel for",
            OpenMpDirective::TeamsDistributeParallelForSimd => "teams distribute parallel for simd",
            OpenMpDirective::TeamsDistributeParallelLoop => "teams distribute parallel loop",
            OpenMpDirective::TeamsDistributeSimd => "teams distribute simd",
            OpenMpDirective::TeamsLoop => "teams loop",
            OpenMpDirective::TeamsLoopSimd => "teams loop simd",
            OpenMpDirective::For => "for",
            OpenMpDirective::ForSimd => "for simd",
            OpenMpDirective::Task => "task",
            OpenMpDirective::Taskloop => "taskloop",
            OpenMpDirective::TaskloopSimd => "taskloop simd",
        }
    }
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
