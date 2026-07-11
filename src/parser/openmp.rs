use super::{
    ClauseRegistry, ClauseRegistryBuilder, ClauseRule, DirectiveRegistry, DirectiveRegistryBuilder,
    Parser,
};
use crate::parser::clause::{
    ClauseKind, ReductionModifier, ReductionOperator, parse_variable_list,
};

// Tokenize unknown spellings so semantic lowering can recognize typed
// implementation-defined `requires` properties. In every other context the
// unknown spelling remains a hard semantic error.
const OPENMP_DEFAULT_CLAUSE_RULE: ClauseRule = ClauseRule::Flexible;

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
    Absent => { name: "absent", rule: ClauseRule::Parenthesized },
    AcqRel => { name: "acq_rel", rule: ClauseRule::Flexible },
    Acquire => { name: "acquire", rule: ClauseRule::Flexible },
    AdjustArgs => { name: "adjust_args", rule: ClauseRule::Parenthesized },
    Affinity => { name: "affinity", rule: ClauseRule::Parenthesized },
    Align => { name: "align", rule: ClauseRule::Parenthesized },
    Aligned => { name: "aligned", rule: ClauseRule::Parenthesized },
    Allocate => { name: "allocate", rule: ClauseRule::Parenthesized },
    Allocator => { name: "allocator", rule: ClauseRule::Parenthesized },
    AppendArgs => { name: "append_args", rule: ClauseRule::Parenthesized },
    Apply => { name: "apply", rule: ClauseRule::Parenthesized },
    At => { name: "at", rule: ClauseRule::Parenthesized },
    AtomicDefaultMemOrder => { name: "atomic_default_mem_order", rule: ClauseRule::Parenthesized },
    Bind => { name: "bind", rule: ClauseRule::Parenthesized },
    Capture => { name: "capture", rule: ClauseRule::Flexible },
    Collapse => { name: "collapse", rule: ClauseRule::Parenthesized },
    Collector => { name: "collector", rule: ClauseRule::Parenthesized },
    Combiner => { name: "combiner", rule: ClauseRule::Parenthesized },
    Compare => { name: "compare", rule: ClauseRule::Flexible },
    Contains => { name: "contains", rule: ClauseRule::Parenthesized },
    Copyin => { name: "copyin", rule: ClauseRule::Parenthesized },
    Copyprivate => { name: "copyprivate", rule: ClauseRule::Parenthesized },
    Parallel => { name: "parallel", rule: ClauseRule::Bare },
    Sections => { name: "sections", rule: ClauseRule::Bare },
    For => { name: "for", rule: ClauseRule::Bare },
    Do => { name: "do", rule: ClauseRule::Bare },
    Taskgroup => { name: "taskgroup", rule: ClauseRule::Bare },
    Counts => { name: "counts", rule: ClauseRule::Parenthesized },
    Default => { name: "default", rule: ClauseRule::Parenthesized },
    Defaultmap => { name: "defaultmap", rule: ClauseRule::Parenthesized },
    Depend => { name: "depend", rule: ClauseRule::Parenthesized },
    DepobjUpdate => { name: "depobj_update", rule: ClauseRule::Parenthesized },
    Destroy => { name: "destroy", rule: ClauseRule::Flexible },
    Detach => { name: "detach", rule: ClauseRule::Parenthesized },
    Device => { name: "device", rule: ClauseRule::Parenthesized },
    DeviceSafesync => { name: "device_safesync", rule: ClauseRule::Flexible },
    DeviceType => { name: "device_type", rule: ClauseRule::Parenthesized },
    DistSchedule => { name: "dist_schedule", rule: ClauseRule::Parenthesized },
    Doacross => { name: "doacross", rule: ClauseRule::Parenthesized },
    DynamicAllocators => { name: "dynamic_allocators", rule: ClauseRule::Flexible },
    ExtImplementationDefinedRequirement => { name: "ext_implementation_defined_requirement", rule: ClauseRule::Flexible },
    Enter => { name: "enter", rule: ClauseRule::Parenthesized },
    Exclusive => { name: "exclusive", rule: ClauseRule::Parenthesized },
    Fail => { name: "fail", rule: ClauseRule::Flexible },
    Final => { name: "final", rule: ClauseRule::Parenthesized },
    Filter => { name: "filter", rule: ClauseRule::Parenthesized },
    Firstprivate => { name: "firstprivate", rule: ClauseRule::Parenthesized },
    From => { name: "from", rule: ClauseRule::Parenthesized },
    Full => { name: "full", rule: ClauseRule::Flexible },
    Grainsize => { name: "grainsize", rule: ClauseRule::Parenthesized },
    GraphId => { name: "graph_id", rule: ClauseRule::Parenthesized },
    GraphReset => { name: "graph_reset", rule: ClauseRule::Flexible },
    HasDeviceAddr => { name: "has_device_addr", rule: ClauseRule::Parenthesized },
    Hint => { name: "hint", rule: ClauseRule::Parenthesized },
    Holds => { name: "holds", rule: ClauseRule::Parenthesized },
    If => { name: "if", rule: ClauseRule::Parenthesized },
    InReduction => { name: "in_reduction", rule: ClauseRule::Custom(parse_openmp_in_reduction_clause) },
    Induction => { name: "induction", rule: ClauseRule::Parenthesized },
    Inductor => { name: "inductor", rule: ClauseRule::Parenthesized },
    Inbranch => { name: "inbranch", rule: ClauseRule::Flexible },
    Inclusive => { name: "inclusive", rule: ClauseRule::Parenthesized },
    Init => { name: "init", rule: ClauseRule::Parenthesized },
    InitComplete => { name: "init_complete", rule: ClauseRule::Flexible },
    Initializer => { name: "initializer", rule: ClauseRule::Parenthesized },
    Indirect => { name: "indirect", rule: ClauseRule::Flexible },
    Interop => { name: "interop", rule: ClauseRule::Parenthesized },
    IsDevicePtr => { name: "is_device_ptr", rule: ClauseRule::Parenthesized },
    Label => { name: "label", rule: ClauseRule::Parenthesized },
    Lastprivate => { name: "lastprivate", rule: ClauseRule::Parenthesized },
    Linear => { name: "linear", rule: ClauseRule::Parenthesized },
    Link => { name: "link", rule: ClauseRule::Parenthesized },
    Local => { name: "local", rule: ClauseRule::Parenthesized },
    Looprange => { name: "looprange", rule: ClauseRule::Parenthesized },
    Map => { name: "map", rule: ClauseRule::Parenthesized },
    Match => { name: "match", rule: ClauseRule::Parenthesized },
    Message => { name: "message", rule: ClauseRule::Parenthesized },
    Memscope => { name: "memscope", rule: ClauseRule::Parenthesized },
    Mergeable => { name: "mergeable", rule: ClauseRule::Flexible },
    Nocontext => { name: "nocontext", rule: ClauseRule::Parenthesized },
    Nogroup => { name: "nogroup", rule: ClauseRule::Flexible },
    NoOpenmp => { name: "no_openmp", rule: ClauseRule::Flexible },
    NoOpenmpConstructs => { name: "no_openmp_constructs", rule: ClauseRule::Flexible },
    NoOpenmpRoutines => { name: "no_openmp_routines", rule: ClauseRule::Flexible },
    NoParallelism => { name: "no_parallelism", rule: ClauseRule::Flexible },
    Nontemporal => { name: "nontemporal", rule: ClauseRule::Parenthesized },
    Notinbranch => { name: "notinbranch", rule: ClauseRule::Flexible },
    Novariants => { name: "novariants", rule: ClauseRule::Flexible },
    Nowait => { name: "nowait", rule: ClauseRule::Flexible },
    NumTasks => { name: "num_tasks", rule: ClauseRule::Parenthesized },
    NumTeams => { name: "num_teams", rule: ClauseRule::Parenthesized },
    NumThreads => { name: "num_threads", rule: ClauseRule::Parenthesized },
    Order => { name: "order", rule: ClauseRule::Parenthesized },
    Ordered => { name: "ordered", rule: ClauseRule::Flexible },
    Otherwise => { name: "otherwise", rule: ClauseRule::Parenthesized },
    Partial => { name: "partial", rule: ClauseRule::Flexible },
    Permutation => { name: "permutation", rule: ClauseRule::Parenthesized },
    Priority => { name: "priority", rule: ClauseRule::Parenthesized },
    Private => { name: "private", rule: ClauseRule::Parenthesized },
    ProcBind => { name: "proc_bind", rule: ClauseRule::Parenthesized },
    Public => { name: "public", rule: ClauseRule::Flexible },
    Read => { name: "read", rule: ClauseRule::Flexible },
    Reduction => { name: "reduction", rule: ClauseRule::Custom(parse_openmp_reduction_clause) },
    Release => { name: "release", rule: ClauseRule::Flexible },
    Relaxed => { name: "relaxed", rule: ClauseRule::Flexible },
    Replayable => { name: "replayable", rule: ClauseRule::Flexible },
    Reproducible => { name: "reproducible", rule: ClauseRule::Bare },
    Reverse => { name: "reverse", rule: ClauseRule::Flexible },
    ReverseOffload => { name: "reverse_offload", rule: ClauseRule::Flexible },
    Safelen => { name: "safelen", rule: ClauseRule::Parenthesized },
    Safesync => { name: "safesync", rule: ClauseRule::Flexible },
    Schedule => { name: "schedule", rule: ClauseRule::Parenthesized },
    SelfMaps => { name: "self_maps", rule: ClauseRule::Flexible },
    SeqCst => { name: "seq_cst", rule: ClauseRule::Flexible },
    Severity => { name: "severity", rule: ClauseRule::Parenthesized },
    Shared => { name: "shared", rule: ClauseRule::Parenthesized },
    Simd => { name: "simd", rule: ClauseRule::Flexible },
    Simdlen => { name: "simdlen", rule: ClauseRule::Parenthesized },
    Sizes => { name: "sizes", rule: ClauseRule::Parenthesized },
    TaskReduction => { name: "task_reduction", rule: ClauseRule::Custom(parse_openmp_task_reduction_clause) },
    ThreadLimit => { name: "thread_limit", rule: ClauseRule::Parenthesized },
    Threads => { name: "threads", rule: ClauseRule::Flexible },
    Threadset => { name: "threadset", rule: ClauseRule::Parenthesized },
    To => { name: "to", rule: ClauseRule::Parenthesized },
    Transparent => { name: "transparent", rule: ClauseRule::Flexible },
    UnifiedAddress => { name: "unified_address", rule: ClauseRule::Flexible },
    UnifiedSharedMemory => { name: "unified_shared_memory", rule: ClauseRule::Flexible },
    Uniform => { name: "uniform", rule: ClauseRule::Parenthesized },
    Unroll => { name: "unroll", rule: ClauseRule::Flexible },
    Untied => { name: "untied", rule: ClauseRule::Flexible },
    Update => { name: "update", rule: ClauseRule::Flexible },
    Use => { name: "use", rule: ClauseRule::Parenthesized },
    UseDeviceAddr => { name: "use_device_addr", rule: ClauseRule::Parenthesized },
    UseDevicePtr => { name: "use_device_ptr", rule: ClauseRule::Parenthesized },
    UsesAllocators => { name: "uses_allocators", rule: ClauseRule::Parenthesized },
    Weak => { name: "weak", rule: ClauseRule::Flexible },
    When => { name: "when", rule: ClauseRule::Parenthesized },
    Write => { name: "write", rule: ClauseRule::Flexible },
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
    Allocate => "allocate",
    Allocators => "allocators",
    Assume => "assume",
    Assumes => "assumes",
    Atomic => "atomic",
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
    EndAssume => "end assume",
    EndAssumes => "end assumes",
    EndAllocators => "end allocators",
    EndDeclareTarget => "end declare target",
    EndDeclareVariant => "end declare variant",
    EndDispatch => "end dispatch",
    // Fortran end directives
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
    EndDoSimd => "end do simd",
    EndForSimd => "end for simd",
    EndParallelDoSimd => "end parallel do simd",
    EndParallelForSimd => "end parallel for simd",
    EndDistributeSimd => "end distribute simd",
    EndDistributeParallelDo => "end distribute parallel do",
    EndDistributeParallelFor => "end distribute parallel for",
    EndDistributeParallelDoSimd => "end distribute parallel do simd",
    EndDistributeParallelForSimd => "end distribute parallel for simd",
    EndTargetParallel => "end target parallel",
    EndTargetParallelDo => "end target parallel do",
    EndTargetParallelFor => "end target parallel for",
    EndTargetParallelDoSimd => "end target parallel do simd",
    EndTargetParallelForSimd => "end target parallel for simd",
    EndTargetSimd => "end target simd",
    EndTargetTeams => "end target teams",
    EndTargetTeamsDistribute => "end target teams distribute",
    EndTargetTeamsDistributeParallelDo => "end target teams distribute parallel do",
    EndTargetTeamsDistributeParallelFor => "end target teams distribute parallel for",
    EndTargetTeamsDistributeParallelDoSimd => "end target teams distribute parallel do simd",
    EndTargetTeamsDistributeParallelForSimd => "end target teams distribute parallel for simd",
    EndTargetTeamsDistributeSimd => "end target teams distribute simd",
    EndTargetTeamsLoop => "end target teams loop",
    EndTargetTeamsWorkdistribute => "end target teams workdistribute",
    EndTeamsDistribute => "end teams distribute",
    EndTeamsDistributeParallelDo => "end teams distribute parallel do",
    EndTeamsDistributeParallelFor => "end teams distribute parallel for",
    EndTeamsDistributeParallelDoSimd => "end teams distribute parallel do simd",
    EndTeamsDistributeParallelForSimd => "end teams distribute parallel for simd",
    EndTeamsDistributeSimd => "end teams distribute simd",
    EndTeamsLoop => "end teams loop",
    EndTaskloopSimd => "end taskloop simd",
    EndMasterTaskloop => "end master taskloop",
    EndMasterTaskloopSimd => "end master taskloop simd",
    EndMaskedTaskloop => "end masked taskloop",
    EndMaskedTaskloopSimd => "end masked taskloop simd",
    EndParallelMasterTaskloop => "end parallel master taskloop",
    EndParallelMasterTaskloopSimd => "end parallel master taskloop simd",
    EndParallelMasked => "end parallel masked",
    EndParallelMaskedTaskloop => "end parallel masked taskloop",
    EndParallelMaskedTaskloopSimd => "end parallel masked taskloop simd",
    EndTargetParallelLoop => "end target parallel loop",
    EndParallelLoop => "end parallel loop",
    EndTargetLoop => "end target loop",
    EndSection => "end section",
    EndScope => "end scope",
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
    EndMetadirective => "end metadirective",
    Nothing => "nothing",
    Ordered => "ordered",
    Parallel => "parallel",
    ParallelDo => "parallel do",  // Fortran equivalent of PARALLEL FOR
    ParallelDoCompact => "paralleldo",  // Compact Fortran spelling
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
    TargetDataUnderscore => "target_data",
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
    TargetTeamsWorkdistribute => "target teams workdistribute",
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
    Tile => "tile",
    Unroll => "unroll",
    Workdistribute => "workdistribute",
    Workshare => "workshare",
}

pub(crate) fn clause_registry() -> ClauseRegistry {
    let mut builder = ClauseRegistryBuilder::new().with_default_rule(OPENMP_DEFAULT_CLAUSE_RULE);

    for clause in OpenMpClause::ALL {
        builder.register_with_rule_mut(clause.name(), clause.rule());
    }

    builder.build()
}

#[derive(Clone, Copy)]
struct Parenthesized<'a> {
    /// Exact bounded source, including the opening and closing parentheses.
    source: &'a str,
    /// Exact bounded source inside the parentheses, excluding outer space.
    content: &'a str,
}

// Parse balanced parentheses without rendering or normalizing their source.
// Public parsing has already converted the physical directive into one
// `LogicalSource`; every typed parameter therefore continues to point into
// that same checked buffer.
fn parse_parenthesized(
    input: &str,
    case_insensitive: bool,
) -> nom::IResult<&str, Parenthesized<'_>> {
    use crate::lexer;
    use nom::bytes::complete::tag;
    use nom::error::{Error, ErrorKind};

    let (input, _) = if case_insensitive {
        lexer::skip_fortran_space_and_comments(input)?
    } else {
        lexer::skip_space_and_comments(input)?
    };

    // Expect an opening parenthesis
    let parenthesized_source = input;
    let (input, _) = tag("(")(input)?;

    let end_index = lexer::find_matching_parenthesis(input, case_insensitive)
        .ok_or_else(|| nom::Err::Error(Error::new(input, ErrorKind::Fail)))?;

    let rest = &input[end_index + 1..];
    let source = &parenthesized_source[..end_index + 2];
    let content = input[..end_index].trim();

    Ok((rest, Parenthesized { source, content }))
}

fn parse_openmp_reduction_clause<'a>(
    name: std::borrow::Cow<'a, str>,
    input: &'a str,
    case_insensitive: bool,
) -> nom::IResult<&'a str, super::Clause<'a>> {
    parse_openmp_reduction_like_clause(name, input, true, case_insensitive)
}

fn parse_openmp_in_reduction_clause<'a>(
    name: std::borrow::Cow<'a, str>,
    input: &'a str,
    case_insensitive: bool,
) -> nom::IResult<&'a str, super::Clause<'a>> {
    parse_openmp_reduction_like_clause(name, input, false, case_insensitive)
}

fn parse_openmp_task_reduction_clause<'a>(
    name: std::borrow::Cow<'a, str>,
    input: &'a str,
    case_insensitive: bool,
) -> nom::IResult<&'a str, super::Clause<'a>> {
    parse_openmp_reduction_like_clause(name, input, false, case_insensitive)
}

fn parse_openmp_reduction_like_clause<'a>(
    name: std::borrow::Cow<'a, str>,
    input: &'a str,
    allow_modifiers: bool,
    case_insensitive: bool,
) -> nom::IResult<&'a str, super::Clause<'a>> {
    use nom::error::{Error, ErrorKind};

    let (rest, parenthesized) = parse_parenthesized(input, case_insensitive)?;
    let content = parenthesized.content;
    let (first_segment, first_values_segment) = crate::ir::lang::split_once_top_level(content, ':')
        .map_err(|_| nom::Err::Error(Error::new(rest, ErrorKind::Tag)))?
        .ok_or_else(|| nom::Err::Error(Error::new(rest, ErrorKind::Tag)))?;
    let (directive_name_modifier, modifiers_segment, values_segment) =
        if let Some((reduction_header, values)) =
            crate::ir::lang::split_once_top_level(first_values_segment, ':')
                .map_err(|_| nom::Err::Error(Error::new(rest, ErrorKind::Tag)))?
        {
            (
                Some(std::borrow::Cow::Borrowed(first_segment.trim())),
                reduction_header,
                values,
            )
        } else {
            (None, first_segment, first_values_segment)
        };
    let modifiers_segment = modifiers_segment.trim();

    let variables_source = values_segment.trim();
    if parse_variable_list(variables_source).is_err() {
        return Err(nom::Err::Error(Error::new(rest, ErrorKind::Tag)));
    }

    let tokens = parse_variable_list(modifiers_segment)
        .map_err(|_| nom::Err::Error(Error::new(rest, ErrorKind::Tag)))?;

    let Some(operator_token) = tokens.last().map(|token| token.trim()) else {
        return Err(nom::Err::Error(Error::new(rest, ErrorKind::Tag)));
    };
    let modifier_tokens = if allow_modifiers && tokens.len() > 1 {
        &tokens[..tokens.len() - 1]
    } else if tokens.len() == 1 {
        &[][..]
    } else {
        return Err(nom::Err::Error(Error::new(rest, ErrorKind::Tag)));
    };

    let mut modifiers = Vec::new();
    let mut modifier_items = Vec::new();
    for modifier_token in modifier_tokens {
        let Some((modifier, items)) =
            map_reduction_modifier(modifier_token.trim(), case_insensitive)
        else {
            return Err(nom::Err::Error(Error::new(rest, ErrorKind::Tag)));
        };
        modifiers.push(modifier);
        modifier_items.push(items);
    }

    let Some((operator, user_identifier)) =
        map_reduction_operator(operator_token, case_insensitive)
    else {
        return Err(nom::Err::Error(Error::new(rest, ErrorKind::Tag)));
    };

    Ok((
        rest,
        super::Clause {
            name,
            kind: ClauseKind::ReductionClause {
                directive_name_modifier,
                modifiers,
                modifier_items,
                operator,
                user_defined_identifier: user_identifier,
                variables_source: std::borrow::Cow::Borrowed(variables_source),
            },
        },
    ))
}

fn map_reduction_modifier<'a>(
    token: &'a str,
    case_insensitive: bool,
) -> Option<(ReductionModifier, Vec<std::borrow::Cow<'a, str>>)> {
    let canonical = if case_insensitive {
        std::borrow::Cow::Owned(token.to_ascii_lowercase())
    } else {
        std::borrow::Cow::Borrowed(token)
    };
    if canonical == "original" {
        return Some((ReductionModifier::Original, Vec::new()));
    }
    if canonical.starts_with("original") {
        let parenthesized = token["original".len()..].trim_start();
        let inner_start = parenthesized.strip_prefix('(')?;
        let close = crate::lexer::find_matching_parenthesis(inner_start, case_insensitive)?;
        if !inner_start[close + 1..].trim().is_empty() {
            return None;
        }
        let inner = inner_start[..close].trim();
        let items = crate::parser::clause::parse_variable_list(inner)
            .ok()?
            .into_iter()
            .map(std::borrow::Cow::Borrowed)
            .collect();
        return Some((ReductionModifier::Original, items));
    }
    match canonical.as_ref() {
        "task" => Some((ReductionModifier::Task, Vec::new())),
        "inscan" => Some((ReductionModifier::Inscan, Vec::new())),
        "default" => Some((ReductionModifier::Default, Vec::new())),
        _ => None,
    }
}

fn map_reduction_operator(
    token: &str,
    case_insensitive: bool,
) -> Option<(ReductionOperator, Option<std::borrow::Cow<'_, str>>)> {
    let normalized = token.trim();
    let lower = normalized.to_ascii_lowercase();
    let canonical = if case_insensitive {
        lower.as_str()
    } else {
        normalized
    };
    let operator = match canonical {
        "+" => ReductionOperator::Add,
        "-" => ReductionOperator::Sub,
        "*" => ReductionOperator::Mul,
        "max" => ReductionOperator::Max,
        "min" => ReductionOperator::Min,
        "&" => ReductionOperator::BitAnd,
        "|" => ReductionOperator::BitOr,
        "^" => ReductionOperator::BitXor,
        "&&" => ReductionOperator::LogAnd,
        "||" => ReductionOperator::LogOr,
        ".and." => ReductionOperator::FortAnd,
        ".or." => ReductionOperator::FortOr,
        ".eqv." => ReductionOperator::FortEqv,
        ".neqv." => ReductionOperator::FortNeqv,
        "iand" => ReductionOperator::FortIand,
        "ior" => ReductionOperator::FortIor,
        "ieor" => ReductionOperator::FortIeor,
        _ => ReductionOperator::UserDefined,
    };

    if matches!(operator, ReductionOperator::UserDefined) {
        Some((operator, Some(std::borrow::Cow::Borrowed(normalized))))
    } else {
        Some((operator, None))
    }
}

// Custom parser for allocate directive: allocate(list) [clauses].
fn parse_allocate_directive<'a>(
    name: std::borrow::Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::Directive;

    let (rest, parameter) = parse_parenthesized(input, clause_registry.is_case_insensitive())?;
    let (rest, clauses) = clause_registry.parse_sequence(rest)?;

    Ok((
        rest,
        Directive {
            name: name.into(),
            parameter: Some(std::borrow::Cow::Borrowed(parameter.source)),
            clauses,
        },
    ))
}

// Custom parser for threadprivate directive: threadprivate(list).
fn parse_threadprivate_directive<'a>(
    name: std::borrow::Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::Directive;

    let (rest, parameter) = parse_parenthesized(input, clause_registry.is_case_insensitive())?;
    Ok((
        rest,
        Directive::new(
            name,
            Some(std::borrow::Cow::Borrowed(parameter.source)),
            vec![],
        ),
    ))
}

// Custom parser for declare target extended form: declare target(list)
fn parse_declare_target_extended<'a>(
    name: std::borrow::Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::Directive;

    if input.trim_start().starts_with('(') {
        let (rest, parameter) = parse_parenthesized(input, clause_registry.is_case_insensitive())?;
        let (rest, clauses) = clause_registry.parse_sequence(rest)?;

        return Ok((
            rest,
            Directive {
                name: name.clone().into(),
                parameter: Some(std::borrow::Cow::Borrowed(parameter.source)),
                clauses,
            },
        ));
    }

    // Standard clause form. In particular, `enter(...)`, `link(...)`, and
    // historical `to(...)` are clauses, never an unparenthesized raw
    // directive parameter.
    let (rest, clauses) = clause_registry.parse_sequence(input)?;
    Ok((rest, Directive::new(name, None, clauses)))
}

// Custom parser for declare mapper directive: declare mapper(declaration) map-clause.
fn parse_declare_mapper_directive<'a>(
    name: std::borrow::Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::Directive;

    let (rest, parameter) = parse_parenthesized(input, clause_registry.is_case_insensitive())?;
    let (rest, clauses) = clause_registry.parse_sequence(rest)?;

    Ok((
        rest,
        Directive {
            name: name.into(),
            parameter: Some(std::borrow::Cow::Borrowed(parameter.source)),
            clauses,
        },
    ))
}

// Custom parser for declare variant directive: declare variant(function) match(...).
fn parse_declare_variant_directive<'a>(
    name: std::borrow::Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::Directive;

    let (rest, parameter) = parse_parenthesized(input, clause_registry.is_case_insensitive())?;
    let (rest, clauses) = clause_registry.parse_sequence(rest)?;

    Ok((
        rest,
        Directive {
            name: name.into(),
            parameter: Some(std::borrow::Cow::Borrowed(parameter.source)),
            clauses,
        },
    ))
}

// OpenMP 6.0 permits clause-only `depobj init(...)`; the historical
// `depobj(depend-object)` form remains accepted and canonicalized.
fn parse_depobj_directive<'a>(
    name: std::borrow::Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::Directive;

    let trimmed = input.trim_start();
    let (rest, parameter) = if trimmed.starts_with('(') {
        let (rest, parameter) =
            parse_parenthesized(trimmed, clause_registry.is_case_insensitive())?;
        (rest, Some(std::borrow::Cow::Borrowed(parameter.source)))
    } else {
        (trimmed, None)
    };
    let (rest, mut clauses) = clause_registry.parse_sequence(rest)?;
    remap_depobj_update_clauses(&mut clauses);

    Ok((
        rest,
        Directive {
            name: name.into(),
            parameter,
            clauses,
        },
    ))
}

fn remap_depobj_update_clauses(clauses: &mut [crate::parser::clause::LocatedClause<'_>]) {
    for clause in clauses {
        let kind = crate::parser::lookup_clause_name(clause.name.as_ref());
        if matches!(kind, crate::parser::ClauseName::Update) {
            clause.name = std::borrow::Cow::Borrowed(OpenMpClause::DepobjUpdate.name());
        }
    }
}

// Custom parser for cancel directive: cancel construct-type [clauses].
fn parse_cancel_directive<'a>(
    name: std::borrow::Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::Directive;
    use crate::lexer::lex_identifier_token;

    let input_trimmed = input.trim_start();

    let (rest, construct_type) = lex_identifier_token(input_trimmed)?;
    let (rest, clauses) = clause_registry.parse_sequence(rest)?;

    Ok((
        rest,
        Directive {
            name: name.into(),
            parameter: Some(std::borrow::Cow::Borrowed(construct_type)),
            clauses,
        },
    ))
}

// Custom parser for cancellation point directive: cancellation point construct-type-clause
fn parse_cancellation_point_directive<'a>(
    name: std::borrow::Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::Directive;
    use crate::lexer::lex_identifier_token;

    let input_trimmed = input.trim_start();

    let (rest, construct_type) = lex_identifier_token(input_trimmed)?;
    let (rest, clauses) = clause_registry.parse_sequence(rest)?;

    Ok((
        rest,
        Directive {
            name: name.into(),
            parameter: Some(std::borrow::Cow::Borrowed(construct_type)),
            clauses,
        },
    ))
}

// Custom parser for groupprivate directive: groupprivate(list) [clauses].
fn parse_groupprivate_directive<'a>(
    name: std::borrow::Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::Directive;

    let (rest, parameter) = parse_parenthesized(input, clause_registry.is_case_insensitive())?;
    let (rest, clauses) = clause_registry.parse_sequence(rest)?;

    Ok((
        rest,
        Directive {
            name: name.into(),
            parameter: Some(std::borrow::Cow::Borrowed(parameter.source)),
            clauses,
        },
    ))
}

// Custom parser for critical directive: critical [(name)] [hint(...)] or bare critical
fn parse_critical_directive<'a>(
    name: std::borrow::Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::Directive;

    if input.trim_start().starts_with('(') {
        let (rest, parameter) = parse_parenthesized(input, clause_registry.is_case_insensitive())?;
        // After the parenthesized name, parse any clauses (like hint)
        let (rest, clauses) = clause_registry.parse_sequence(rest)?;

        return Ok((
            rest,
            Directive {
                name: crate::parser::directive_kind::lookup_directive_name(name.as_ref()),
                parameter: Some(std::borrow::Cow::Borrowed(parameter.source)),
                clauses,
            },
        ));
    }

    let (rest, clauses) = clause_registry.parse_sequence(input)?;
    Ok((rest, Directive::new(name, None, clauses)))
}

// Custom parser for the two token orders used by standardized flush grammars:
//
// - through 5.2: `flush [memory-order-clause] [(list)]`
// - 6.0: `flush[(list)] [memory-order-clause[(use-semantics)]]`
//
// A parenthesized argument immediately after a memory-order keyword is retained
// explicitly as a 6.0 use-semantics argument. AST construction hard-rejects
// that shape under exact 5.2-and-earlier policies: those specifications'
// restriction forbids combining a memory-order clause with a flush list.
fn parse_flush_directive<'a>(
    name: std::borrow::Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::Directive;
    use nom::error::{Error, ErrorKind};

    let trimmed = input.trim_start();
    let keyword_end = trimmed
        .char_indices()
        .take_while(|(_, ch)| ch.is_ascii_alphanumeric() || *ch == '_')
        .last()
        .map_or(0, |(index, ch)| index + ch.len_utf8());
    let keyword = &trimmed[..keyword_end];
    if ["seq_cst", "acq_rel", "release", "acquire"]
        .into_iter()
        .any(|expected| clause_registry.keyword_eq(keyword, expected))
    {
        let after_keyword = &trimmed[keyword_end..];
        if after_keyword.trim_start().starts_with('(') {
            let (after_parameter, parameter) =
                parse_parenthesized(after_keyword, clause_registry.is_case_insensitive())?;
            let (order_rest, mut clauses) = clause_registry.parse_sequence(keyword)?;
            if !order_rest.trim().is_empty() || clauses.len() != 1 {
                return Err(nom::Err::Failure(Error::new(keyword, ErrorKind::Fail)));
            }
            clauses[0].kind =
                ClauseKind::FlushMemoryOrderArgument(std::borrow::Cow::Borrowed(parameter.content));

            let (rest, mut trailing_clauses) = clause_registry.parse_sequence(after_parameter)?;
            clauses.append(&mut trailing_clauses);
            return Ok((
                rest,
                Directive {
                    name: name.into(),
                    parameter: Some(std::borrow::Cow::Borrowed(parameter.source)),
                    clauses,
                },
            ));
        }
    }

    if input.trim_start().starts_with('(') {
        let (rest, parameter) = parse_parenthesized(input, clause_registry.is_case_insensitive())?;
        let (rest, clauses) = clause_registry.parse_sequence(rest)?;
        return Ok((
            rest,
            Directive {
                name: name.into(),
                parameter: Some(std::borrow::Cow::Borrowed(parameter.source)),
                clauses,
            },
        ));
    }

    let (rest, clauses) = clause_registry.parse_sequence(input)?;
    Ok((rest, Directive::new(name, None, clauses)))
}

// Custom parser for declare reduction directive
// Syntax: declare reduction(operator : type-list : combiner) initializer(...)
fn parse_declare_reduction_directive<'a>(
    name: std::borrow::Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    let trimmed = input.trim_start();
    let (rest, signature) = parse_parenthesized(trimmed, clause_registry.is_case_insensitive())?;
    let (rest, clauses) = clause_registry.parse_sequence(rest)?;

    Ok((
        rest,
        super::Directive {
            name: crate::parser::directive_kind::lookup_directive_name(name.as_ref()),
            parameter: Some(std::borrow::Cow::Borrowed(signature.source)),
            clauses,
        },
    ))
}

// Custom parser for declare simd directive
// Syntax: declare simd [(proc-name)] [clause[[,] clause]...]
fn parse_declare_simd_directive<'a>(
    _name: std::borrow::Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::Directive;

    let trimmed = input.trim_start();

    let (remaining, parameter) = if trimmed.starts_with('(') {
        let (rest, parameter) =
            parse_parenthesized(trimmed, clause_registry.is_case_insensitive())?;
        (rest, Some(std::borrow::Cow::Borrowed(parameter.source)))
    } else {
        (trimmed, None)
    };

    // Parse clauses from the remaining input
    let (rest, clauses) = clause_registry.parse_sequence(remaining)?;

    Ok((
        rest,
        Directive {
            name: crate::parser::directive_kind::lookup_directive_name("declare simd"),
            parameter,
            clauses,
        },
    ))
}

// Custom parser for target data directive. The name is retained so the typed
// lowering can distinguish the 6.0 underscore spelling from the historical
// spaced spelling for availability checks.
fn parse_target_data_directive<'a>(
    name: std::borrow::Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::Directive;

    let (rest, clauses) = clause_registry.parse_sequence(input)?;

    Ok((rest, Directive::new(name, None, clauses)))
}

// Custom parser for declare induction. Its signature is required and every
// following byte must be consumed by a registered clause.
fn parse_declare_induction_directive<'a>(
    name: std::borrow::Cow<'a, str>,
    input: &'a str,
    _clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    let trimmed = input.trim_start();
    let (rest, signature) = parse_parenthesized(trimmed, _clause_registry.is_case_insensitive())?;
    let (rest, clauses) = _clause_registry.parse_sequence(rest)?;

    Ok((
        rest,
        super::Directive {
            name: crate::parser::directive_kind::lookup_directive_name(name.as_ref()),
            parameter: Some(std::borrow::Cow::Borrowed(signature.source)),
            clauses,
        },
    ))
}

// Directives that have custom parsers (enum-based, no string comparisons).
const CUSTOM_PARSER_DIRECTIVES: &[OpenMpDirective] = &[
    OpenMpDirective::Allocate,
    OpenMpDirective::Threadprivate,
    OpenMpDirective::DeclareTarget,
    OpenMpDirective::DeclareInduction,
    OpenMpDirective::DeclareMapper,
    OpenMpDirective::DeclareVariant,
    OpenMpDirective::DeclareReduction,
    OpenMpDirective::DeclareSimd,
    OpenMpDirective::Depobj,
    OpenMpDirective::Cancel,
    OpenMpDirective::CancellationPoint,
    OpenMpDirective::Groupprivate,
    OpenMpDirective::Critical,
    OpenMpDirective::EndCritical,
    OpenMpDirective::Flush,
    OpenMpDirective::TargetData,
    OpenMpDirective::TargetDataUnderscore,
];

pub(crate) fn directive_registry() -> DirectiveRegistry {
    let mut builder = DirectiveRegistryBuilder::new();

    // Register custom parsers for directives with special syntax
    builder = builder.register_custom("allocate", parse_allocate_directive);
    builder = builder.register_custom("threadprivate", parse_threadprivate_directive);
    builder = builder.register_custom("declare target", parse_declare_target_extended);
    builder = builder.register_custom("declare_target", parse_declare_target_extended);
    builder = builder.register_custom("declare induction", parse_declare_induction_directive);
    builder = builder.register_custom("declare_induction", parse_declare_induction_directive);
    builder = builder.register_custom("declare mapper", parse_declare_mapper_directive);
    builder = builder.register_custom("declare_mapper", parse_declare_mapper_directive);
    builder = builder.register_custom("declare variant", parse_declare_variant_directive);
    builder = builder.register_custom("declare_variant", parse_declare_variant_directive);
    builder = builder.register_custom("declare reduction", parse_declare_reduction_directive);
    builder = builder.register_custom("declare_reduction", parse_declare_reduction_directive);
    builder = builder.register_custom("declare simd", parse_declare_simd_directive);
    builder = builder.register_custom("declare_simd", parse_declare_simd_directive);
    builder = builder.register_custom("depobj", parse_depobj_directive);
    builder = builder.register_custom("cancel", parse_cancel_directive);
    builder = builder.register_custom("cancellation point", parse_cancellation_point_directive);
    builder = builder.register_custom("cancellation_point", parse_cancellation_point_directive);
    builder = builder.register_custom("groupprivate", parse_groupprivate_directive);
    builder = builder.register_custom("critical", parse_critical_directive);
    builder = builder.register_custom("end critical", parse_critical_directive);
    builder = builder.register_custom("endcritical", parse_critical_directive);
    builder = builder.register_custom("flush", parse_flush_directive);
    builder = builder.register_generic("end parallel single");
    builder = builder.register_generic("omp teams");

    // OpenMP 6.0 made these underscore-bearing directive names canonical and
    // retained the historical spaced spellings as explicit alternatives. Both
    // forms are registered; typed lowering records which syntax was used so
    // exact-version checks do not mistake a 6.0 spelling for older syntax.
    builder = builder.register_custom("target data", parse_target_data_directive);
    builder = builder.register_custom("target_data", parse_target_data_directive);
    builder = builder.register_generic("begin declare_variant");
    builder = builder.register_generic("end declare_variant");
    builder = builder.register_generic("begin declare_target");
    builder = builder.register_generic("end declare_target");
    builder = builder.register_generic("task_iteration");
    builder = builder.register_generic("target_enter_data");
    builder = builder.register_generic("target_exit_data");
    builder = builder.register_generic("end target_data");
    builder = builder.register_generic("target_update");

    // Register remaining directives as generic
    for directive in OpenMpDirective::ALL {
        let name = directive.as_str();
        // Skip directives that already have custom parsers
        if !CUSTOM_PARSER_DIRECTIVES.contains(directive) {
            builder = builder.register_generic(name);
        }
    }

    // Register no-space Fortran end directive variants
    // These are aliases that map to the same DirectiveName enum variants
    builder = builder.register_generic("endparallel");
    builder = builder.register_generic("enddo");
    builder = builder.register_generic("endsimd");
    builder = builder.register_generic("endsections");
    builder = builder.register_generic("endsingle");
    builder = builder.register_generic("endworkshare");
    builder = builder.register_generic("endordered");
    builder = builder.register_generic("endloop");
    builder = builder.register_generic("enddistribute");
    builder = builder.register_generic("endteams");
    builder = builder.register_generic("endtaskloop");
    builder = builder.register_generic("endtask");
    builder = builder.register_generic("endtaskgroup");
    builder = builder.register_generic("endallocators");
    builder = builder.register_generic("enddispatch");
    builder = builder.register_generic("endmaster");
    builder = builder.register_generic("endatomic");
    builder = builder.register_generic("endparalleldo");
    builder = builder.register_generic("endparallelsections");
    builder = builder.register_generic("endparallelworkshare");
    builder = builder.register_generic("endparallelmaster");
    builder = builder.register_generic("enddosimd");
    builder = builder.register_generic("endparalleldosimd");
    builder = builder.register_generic("enddistributesimd");
    builder = builder.register_generic("enddistributeparalleldo");
    builder = builder.register_generic("enddistributeparalleldosimd");

    builder.build()
}

pub(crate) fn parser() -> Parser {
    Parser::new(
        directive_registry(),
        clause_registry(),
        crate::lexer::Language::C,
        super::Dialect::OpenMp,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openmp_six_underscore_names_canonicalize_to_typed_kinds() {
        use crate::ast::OmpDirectiveKind as K;

        let registry = directive_registry();
        for (spelling, expected) in [
            ("cancellation_point", K::CancellationPoint),
            ("declare_induction", K::DeclareInduction),
            ("declare_mapper", K::DeclareMapper),
            ("declare_reduction", K::DeclareReduction),
            ("declare_simd", K::DeclareSimd),
            ("declare_target", K::DeclareTarget),
            ("declare_variant", K::DeclareVariant),
            ("begin declare_target", K::BeginDeclareTarget),
            ("end declare_target", K::EndDeclareTarget),
            ("begin declare_variant", K::BeginDeclareVariant),
            ("end declare_variant", K::EndDeclareVariant),
            ("task_iteration", K::TaskIteration),
            ("target_enter_data", K::TargetEnterData),
            ("target_exit_data", K::TargetExitData),
            ("target_data", K::TargetData),
            ("end target_data", K::EndTargetData),
            ("target_update", K::TargetUpdate),
        ] {
            let (rest, (name, source)) = registry
                .lex_name(spelling)
                .unwrap_or_else(|error| panic!("failed to lex {spelling:?}: {error:?}"));
            assert!(rest.is_empty(), "unconsumed text for {spelling:?}");
            assert_eq!(source, spelling);
            let raw = crate::parser::directive_kind::lookup_directive_name(name.as_ref());
            assert_eq!(
                K::try_from(raw),
                Ok(expected),
                "wrong canonical kind for {spelling:?}"
            );
        }
    }

    #[test]
    fn fortran_compact_name_uses_the_registered_canonical_rule() {
        use crate::parser::Language;

        let parser = parser().with_language(Language::FortranFree);
        let (_, directive) = parser
            .parse("!$omp cancellationpoint parallel")
            .expect("Fortran permits omitted blanks between directive-name keywords");
        assert_eq!(
            directive.name,
            crate::parser::directive_kind::DirectiveName::CancellationPoint
        );
        assert_eq!(directive.name_source(), "cancellationpoint");
    }

    #[test]
    fn test_directive_registry_has_end_atomic() {
        // Check that "end atomic" is in the OpenMpDirective enum
        let found = OpenMpDirective::ALL
            .iter()
            .any(|d| matches!(d, OpenMpDirective::EndAtomic));
        assert!(found, "end atomic not found in OpenMpDirective::ALL");
    }

    #[test]
    fn test_end_atomic_directive() {
        use crate::parser::Language;
        let parser = parser().with_language(Language::FortranFree);
        let input = "!$omp end atomic";
        let result = parser.parse(input);
        if let Err(ref e) = result {
            eprintln!("Parse error for '{input}': {e:?}");
        }
        assert!(result.is_ok(), "Failed to parse: {input}");
        let (_, directive) = result.unwrap();
        assert_eq!(
            directive.name,
            crate::parser::directive_kind::DirectiveName::EndAtomic
        );
    }

    #[test]
    fn test_end_critical_directive() {
        use crate::parser::Language;
        let parser = parser().with_language(Language::FortranFree);
        let input = "!$omp end critical";
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse: {input}");
        let (_, directive) = result.unwrap();
        assert_eq!(
            directive.name,
            crate::parser::directive_kind::DirectiveName::EndCritical
        );
    }

    #[test]
    fn test_end_distribute_directive() {
        use crate::parser::Language;
        let parser = parser().with_language(Language::FortranFree);
        let input = "!$omp end distribute";
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse: {input}");
        let (_, directive) = result.unwrap();
        assert_eq!(
            directive.name,
            crate::parser::directive_kind::DirectiveName::EndDistribute
        );
    }

    #[test]
    fn test_end_parallel_directive() {
        use crate::parser::Language;
        let parser = parser().with_language(Language::FortranFree);
        let input = "!$omp end parallel";
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse: {input}");
        let (_, directive) = result.unwrap();
        assert_eq!(
            directive.name,
            crate::parser::directive_kind::DirectiveName::EndParallel
        );
    }

    #[test]
    fn test_end_target_directive() {
        use crate::parser::Language;
        let parser = parser().with_language(Language::C);
        let input = "#pragma omp end target";
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse: {input}");
        let (_, directive) = result.unwrap();
        assert_eq!(
            directive.name,
            crate::parser::directive_kind::DirectiveName::EndTarget
        );
    }

    #[test]
    fn test_end_do_directive() {
        use crate::parser::Language;
        let parser = parser().with_language(Language::FortranFree);
        let input = "!$omp end do";
        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse: {input}");
        let (_, directive) = result.unwrap();
        assert_eq!(
            directive.name,
            crate::parser::directive_kind::DirectiveName::EndDo
        );
    }

    #[test]
    fn test_masked_directive() {
        use crate::parser::Language;
        let parser = parser().with_language(Language::FortranFree);
        let input = "!$omp masked";
        let result = parser.parse(input);
        if let Err(ref e) = result {
            eprintln!("Parse error for '{input}': {e:?}");
        }
        assert!(result.is_ok(), "Failed to parse: {input}");
        let (_, directive) = result.unwrap();
        assert_eq!(
            directive.name,
            crate::parser::directive_kind::DirectiveName::Masked
        );
    }
}
