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
    Absent => { name: "absent", rule: ClauseRule::Parenthesized },
    AcqRel => { name: "acq_rel", rule: ClauseRule::Bare },
    Acquire => { name: "acquire", rule: ClauseRule::Bare },
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
    Counts => { name: "counts", rule: ClauseRule::Parenthesized },
    Default => { name: "default", rule: ClauseRule::Parenthesized },
    Defaultmap => { name: "defaultmap", rule: ClauseRule::Parenthesized },
    Depend => { name: "depend", rule: ClauseRule::Parenthesized },
    Destroy => { name: "destroy", rule: ClauseRule::Flexible },
    Detach => { name: "detach", rule: ClauseRule::Parenthesized },
    Device => { name: "device", rule: ClauseRule::Parenthesized },
    DeviceResident => { name: "device_resident", rule: ClauseRule::Parenthesized },
    DeviceSafesync => { name: "device_safesync", rule: ClauseRule::Flexible },
    DeviceType => { name: "device_type", rule: ClauseRule::Parenthesized },
    DistSchedule => { name: "dist_schedule", rule: ClauseRule::Parenthesized },
    Doacross => { name: "doacross", rule: ClauseRule::Parenthesized },
    DynamicAllocators => { name: "dynamic_allocators", rule: ClauseRule::Bare },
    Enter => { name: "enter", rule: ClauseRule::Parenthesized },
    Exclusive => { name: "exclusive", rule: ClauseRule::Bare },
    Fail => { name: "fail", rule: ClauseRule::Flexible },
    Final => { name: "final", rule: ClauseRule::Parenthesized },
    Filter => { name: "filter", rule: ClauseRule::Parenthesized },
    Firstprivate => { name: "firstprivate", rule: ClauseRule::Parenthesized },
    From => { name: "from", rule: ClauseRule::Parenthesized },
    Full => { name: "full", rule: ClauseRule::Flexible },
    Grainsize => { name: "grainsize", rule: ClauseRule::Parenthesized },
    GraphId => { name: "graph_id", rule: ClauseRule::Parenthesized },
    GraphReset => { name: "graph_reset", rule: ClauseRule::Parenthesized },
    HasDeviceAddr => { name: "has_device_addr", rule: ClauseRule::Parenthesized },
    Hint => { name: "hint", rule: ClauseRule::Parenthesized },
    Holds => { name: "holds", rule: ClauseRule::Parenthesized },
    If => { name: "if", rule: ClauseRule::Parenthesized },
    InReduction => { name: "in_reduction", rule: ClauseRule::Parenthesized },
    Induction => { name: "induction", rule: ClauseRule::Parenthesized },
    Inductor => { name: "inductor", rule: ClauseRule::Parenthesized },
    Inbranch => { name: "inbranch", rule: ClauseRule::Bare },
    Inclusive => { name: "inclusive", rule: ClauseRule::Bare },
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
    Mergeable => { name: "mergeable", rule: ClauseRule::Bare },
    Nocontext => { name: "nocontext", rule: ClauseRule::Parenthesized },
    Nogroup => { name: "nogroup", rule: ClauseRule::Bare },
    NoOpenmp => { name: "no_openmp", rule: ClauseRule::Flexible },
    NoOpenmpConstructs => { name: "no_openmp_constructs", rule: ClauseRule::Flexible },
    NoOpenmpRoutines => { name: "no_openmp_routines", rule: ClauseRule::Flexible },
    NoParallelism => { name: "no_parallelism", rule: ClauseRule::Flexible },
    Nontemporal => { name: "nontemporal", rule: ClauseRule::Parenthesized },
    Notinbranch => { name: "notinbranch", rule: ClauseRule::Bare },
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
    Reduction => { name: "reduction", rule: ClauseRule::Parenthesized },
    Release => { name: "release", rule: ClauseRule::Bare },
    Relaxed => { name: "relaxed", rule: ClauseRule::Bare },
    Replayable => { name: "replayable", rule: ClauseRule::Flexible },
    Reproducible => { name: "reproducible", rule: ClauseRule::Bare },
    Reverse => { name: "reverse", rule: ClauseRule::Flexible },
    ReverseOffload => { name: "reverse_offload", rule: ClauseRule::Bare },
    Safelen => { name: "safelen", rule: ClauseRule::Parenthesized },
    Safesync => { name: "safesync", rule: ClauseRule::Flexible },
    Schedule => { name: "schedule", rule: ClauseRule::Parenthesized },
    SelfMaps => { name: "self_maps", rule: ClauseRule::Bare },
    SeqCst => { name: "seq_cst", rule: ClauseRule::Bare },
    Severity => { name: "severity", rule: ClauseRule::Parenthesized },
    Shared => { name: "shared", rule: ClauseRule::Parenthesized },
    Simd => { name: "simd", rule: ClauseRule::Bare },
    Simdlen => { name: "simdlen", rule: ClauseRule::Parenthesized },
    Sizes => { name: "sizes", rule: ClauseRule::Parenthesized },
    TaskReduction => { name: "task_reduction", rule: ClauseRule::Parenthesized },
    ThreadLimit => { name: "thread_limit", rule: ClauseRule::Parenthesized },
    Threads => { name: "threads", rule: ClauseRule::Bare },
    Threadset => { name: "threadset", rule: ClauseRule::Parenthesized },
    Tile => { name: "tile", rule: ClauseRule::Parenthesized },
    To => { name: "to", rule: ClauseRule::Parenthesized },
    Transparent => { name: "transparent", rule: ClauseRule::Flexible },
    UnifiedAddress => { name: "unified_address", rule: ClauseRule::Flexible },
    UnifiedSharedMemory => { name: "unified_shared_memory", rule: ClauseRule::Flexible },
    Uniform => { name: "uniform", rule: ClauseRule::Parenthesized },
    Unroll => { name: "unroll", rule: ClauseRule::Flexible },
    Untied => { name: "untied", rule: ClauseRule::Bare },
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
    EndAssumes => "end assumes",
    EndDeclareTarget => "end declare target",
    EndDeclareVariant => "end declare variant",
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
    Scan => "scan",
    Section => "section",
    Sections => "sections",
    Simd => "simd",
    Single => "single",
    Split => "split",
    Stripe => "stripe",
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

pub fn clause_registry() -> ClauseRegistry {
    let mut builder = ClauseRegistryBuilder::new().with_default_rule(OPENMP_DEFAULT_CLAUSE_RULE);

    for clause in OpenMpClause::ALL {
        builder.register_with_rule_mut(clause.name(), clause.rule());
    }

    builder.build()
}

// Helper function to parse balanced parentheses and extract content
fn parse_parenthesized_content(input: &str) -> nom::IResult<&str, String> {
    use crate::lexer;
    use nom::bytes::complete::tag;
    use nom::error::{Error, ErrorKind};

    // Skip whitespace and comments before the opening parenthesis
    let (input, _) = lexer::skip_space_and_comments(input)?;

    // Expect an opening parenthesis
    let (input, _) = tag("(")(input)?;

    // Find the matching closing parenthesis, tracking depth
    let mut depth = 1;
    let mut end_index = None;

    for (idx, ch) in input.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    end_index = Some(idx);
                    break;
                }
            }
            _ => {}
        }
    }

    let end_index = end_index.ok_or_else(|| nom::Err::Error(Error::new(input, ErrorKind::Fail)))?;

    let raw_content = &input[..end_index];
    let trimmed = raw_content.trim();
    let normalized = lexer::collapse_line_continuations(trimmed);
    let rest = &input[end_index + 1..];

    Ok((rest, normalized.into_owned()))
}

// Custom parser for allocate directive: allocate(list) [clauses] or bare allocate
fn parse_allocate_directive<'a>(
    name: std::borrow::Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::Directive;

    // Try to parse parenthesized list (extended form)
    if let Ok((rest, list_content)) = parse_parenthesized_content(input) {
        let (rest, clauses) = clause_registry.parse_sequence(rest)?;

        Ok((
            rest,
            Directive {
                name: std::borrow::Cow::Borrowed("allocate"),
                parameter: Some(std::borrow::Cow::Owned(format!("({})", list_content))),
                clauses,
            },
        ))
    } else {
        // Fall back to standard clause parsing (bare form)
        let (rest, clauses) = clause_registry.parse_sequence(input)?;
        Ok((
            rest,
            Directive {
                name,
                parameter: None,
                clauses,
            },
        ))
    }
}

// Custom parser for threadprivate directive: threadprivate(list) or bare threadprivate
fn parse_threadprivate_directive<'a>(
    name: std::borrow::Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::Directive;

    // Try to parse parenthesized list (extended form)
    if let Ok((rest, list_content)) = parse_parenthesized_content(input) {
        Ok((
            rest,
            Directive {
                name: std::borrow::Cow::Borrowed("threadprivate"),
                parameter: Some(std::borrow::Cow::Owned(format!("({})", list_content))),
                clauses: vec![],
            },
        ))
    } else {
        // Fall back to standard clause parsing (bare form)
        let (rest, clauses) = clause_registry.parse_sequence(input)?;
        Ok((
            rest,
            Directive {
                name,
                parameter: None,
                clauses,
            },
        ))
    }
}

// Custom parser for declare target extended form: declare target(list)
fn parse_declare_target_extended<'a>(
    name: std::borrow::Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::Directive;

    // Try to parse parenthesized list (extended form)
    if let Ok((rest, list_content)) = parse_parenthesized_content(input) {
        let (rest, clauses) = clause_registry.parse_sequence(rest)?;

        Ok((
            rest,
            Directive {
                name: std::borrow::Cow::Borrowed("declare target"),
                parameter: Some(std::borrow::Cow::Owned(format!("({})", list_content))),
                clauses,
            },
        ))
    } else {
        // Fall back to standard clause parsing (basic form)
        let (rest, clauses) = clause_registry.parse_sequence(input)?;
        Ok((
            rest,
            Directive {
                name,
                parameter: None,
                clauses,
            },
        ))
    }
}

// Custom parser for declare mapper directive: declare mapper(mapper-id) map-clause or bare
fn parse_declare_mapper_directive<'a>(
    name: std::borrow::Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::Directive;

    // Try to parse parenthesized mapper ID (extended form)
    if let Ok((rest, mapper_id)) = parse_parenthesized_content(input) {
        let (rest, clauses) = clause_registry.parse_sequence(rest)?;

        Ok((
            rest,
            Directive {
                name: std::borrow::Cow::Borrowed("declare mapper"),
                parameter: Some(std::borrow::Cow::Owned(format!("({})", mapper_id))),
                clauses,
            },
        ))
    } else {
        // Fall back to standard clause parsing (bare form)
        let (rest, clauses) = clause_registry.parse_sequence(input)?;
        Ok((
            rest,
            Directive {
                name,
                parameter: None,
                clauses,
            },
        ))
    }
}

// Custom parser for declare variant directive: declare variant(function) match(...) or bare
fn parse_declare_variant_directive<'a>(
    name: std::borrow::Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::Directive;

    // Try to parse parenthesized function name (extended form)
    if let Ok((rest, variant_func)) = parse_parenthesized_content(input) {
        let (rest, clauses) = clause_registry.parse_sequence(rest)?;

        Ok((
            rest,
            Directive {
                name: std::borrow::Cow::Borrowed("declare variant"),
                parameter: Some(std::borrow::Cow::Owned(format!("({})", variant_func))),
                clauses,
            },
        ))
    } else {
        // Fall back to standard clause parsing (bare form)
        let (rest, clauses) = clause_registry.parse_sequence(input)?;
        Ok((
            rest,
            Directive {
                name,
                parameter: None,
                clauses,
            },
        ))
    }
}

// Custom parser for depobj directive: depobj(depobj-object) [clauses] or bare depobj
fn parse_depobj_directive<'a>(
    name: std::borrow::Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::Directive;

    // Try to parse parenthesized depobj identifier (extended form)
    if let Ok((rest, depobj_id)) = parse_parenthesized_content(input) {
        let (rest, clauses) = clause_registry.parse_sequence(rest)?;

        Ok((
            rest,
            Directive {
                name: std::borrow::Cow::Borrowed("depobj"),
                parameter: Some(std::borrow::Cow::Owned(format!("({})", depobj_id))),
                clauses,
            },
        ))
    } else {
        // Fall back to standard clause parsing (bare form)
        let (rest, clauses) = clause_registry.parse_sequence(input)?;
        Ok((
            rest,
            Directive {
                name,
                parameter: None,
                clauses,
            },
        ))
    }
}

// Custom parser for scan directive: scan exclusive(list) or scan inclusive(list) or bare scan
fn parse_scan_directive<'a>(
    name: std::borrow::Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::Directive;
    use nom::bytes::complete::tag;

    // Trim leading whitespace for reliable token matching
    let input_trimmed = input.trim_start();

    // Try exclusive first: `exclusive(list)` optionally followed by clauses
    if let Ok((rest_after_tag, _)) =
        tag::<_, _, nom::error::Error<&str>>("exclusive")(input_trimmed)
    {
        if let Ok((rest_after_paren, list_content)) = parse_parenthesized_content(rest_after_tag) {
            // After the parenthesized list we may have clauses. Parse them from the remaining input.
            let (rest, clauses) = match clause_registry.parse_sequence(rest_after_paren) {
                Ok((r, c)) => (r, c),
                Err(e) => {
                    // Distinguish between: (A) Truly no clauses (only whitespace/comments remaining)
                    // and (B) a parse error. If the remainder contains only whitespace/comments,
                    // treat as no clauses; otherwise propagate the parse error so malformed
                    // clauses aren't silently accepted.
                    if let Ok((remaining_after_skipping, _)) =
                        crate::lexer::skip_space_and_comments(rest_after_paren)
                    {
                        if remaining_after_skipping.is_empty() {
                            (rest_after_paren, vec![])
                        } else {
                            return Err(e);
                        }
                    } else {
                        return Err(e);
                    }
                }
            };

            return Ok((
                rest,
                Directive {
                    name: std::borrow::Cow::Borrowed("scan"),
                    // Store the parameter without a leading space; the display/formatting layer
                    // is responsible for spacing when rendering the directive.
                    parameter: Some(std::borrow::Cow::Owned(format!(
                        "exclusive({})",
                        list_content
                    ))),
                    clauses,
                },
            ));
        }
    }

    // Try inclusive: `inclusive(list)` optionally followed by clauses
    if let Ok((rest_after_tag, _)) =
        tag::<_, _, nom::error::Error<&str>>("inclusive")(input_trimmed)
    {
        if let Ok((rest_after_paren, list_content)) = parse_parenthesized_content(rest_after_tag) {
            let (rest, clauses) = match clause_registry.parse_sequence(rest_after_paren) {
                Ok((r, c)) => (r, c),
                Err(e) => {
                    if let Ok((remaining_after_skipping, _)) =
                        crate::lexer::skip_space_and_comments(rest_after_paren)
                    {
                        if remaining_after_skipping.is_empty() {
                            (rest_after_paren, vec![])
                        } else {
                            return Err(e);
                        }
                    } else {
                        return Err(e);
                    }
                }
            };

            return Ok((
                rest,
                Directive {
                    name: std::borrow::Cow::Borrowed("scan"),
                    parameter: Some(std::borrow::Cow::Owned(format!(
                        "inclusive({})",
                        list_content
                    ))),
                    clauses,
                },
            ));
        }
    }

    // Fall back to standard clause parsing (bare form)
    let (rest, clauses) = clause_registry.parse_sequence(input)?;
    Ok((
        rest,
        Directive {
            name,
            parameter: None,
            clauses,
        },
    ))
}

// Custom parser for cancel directive: cancel construct-type-clause or bare cancel
fn parse_cancel_directive<'a>(
    name: std::borrow::Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::Directive;
    use crate::lexer::lex_identifier_token;

    let input_trimmed = input.trim_start();

    // Try to parse the construct type (parallel, sections, for, taskgroup)
    if let Ok((rest, construct_type)) = lex_identifier_token(input_trimmed) {
        // Parse any additional clauses
        let (rest, clauses) = clause_registry.parse_sequence(rest)?;

        Ok((
            rest,
            Directive {
                name: std::borrow::Cow::Borrowed("cancel"),
                // Store the construct type without a leading space; presentation spacing
                // should be handled by the renderer that prints directives.
                parameter: Some(std::borrow::Cow::Owned(construct_type.to_string())),
                clauses,
            },
        ))
    } else {
        // Fall back to standard clause parsing (bare form)
        let (rest, clauses) = clause_registry.parse_sequence(input)?;
        Ok((
            rest,
            Directive {
                name,
                parameter: None,
                clauses,
            },
        ))
    }
}

// Custom parser for groupprivate directive: groupprivate(list) [clauses] or bare groupprivate
fn parse_groupprivate_directive<'a>(
    name: std::borrow::Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::Directive;

    // Try to parse parenthesized list (extended form)
    if let Ok((rest, list_content)) = parse_parenthesized_content(input) {
        let (rest, clauses) = clause_registry.parse_sequence(rest)?;

        Ok((
            rest,
            Directive {
                name: std::borrow::Cow::Borrowed("groupprivate"),
                parameter: Some(std::borrow::Cow::Owned(format!("({})", list_content))),
                clauses,
            },
        ))
    } else {
        // Fall back to standard clause parsing (bare form)
        let (rest, clauses) = clause_registry.parse_sequence(input)?;
        Ok((
            rest,
            Directive {
                name,
                parameter: None,
                clauses,
            },
        ))
    }
}

// Custom parser for target_data directive (handles underscore variant)
// Preserves the underscore form for round-trip correctness
fn parse_target_data_directive<'a>(
    _name: std::borrow::Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::Directive;

    let (rest, clauses) = clause_registry.parse_sequence(input)?;

    Ok((
        rest,
        Directive {
            name: std::borrow::Cow::Borrowed("target_data"),
            parameter: None,
            clauses,
        },
    ))
}

// Directive names that have custom parsers (excluding target_data underscore variant)
const CUSTOM_PARSER_DIRECTIVES: &[&str] = &[
    "allocate",
    "threadprivate",
    "declare target",
    "declare mapper",
    "declare variant",
    "depobj",
    "scan",
    "cancel",
    "groupprivate",
];

pub fn directive_registry() -> DirectiveRegistry {
    let mut builder = DirectiveRegistryBuilder::new();

    // Register custom parsers for directives with special syntax
    builder = builder.register_custom("allocate", parse_allocate_directive);
    builder = builder.register_custom("threadprivate", parse_threadprivate_directive);
    builder = builder.register_custom("declare target", parse_declare_target_extended);
    builder = builder.register_custom("declare mapper", parse_declare_mapper_directive);
    builder = builder.register_custom("declare variant", parse_declare_variant_directive);
    builder = builder.register_custom("depobj", parse_depobj_directive);
    builder = builder.register_custom("scan", parse_scan_directive);
    builder = builder.register_custom("cancel", parse_cancel_directive);
    builder = builder.register_custom("groupprivate", parse_groupprivate_directive);

    // Handle underscore variant of "target data"
    builder = builder.register_custom("target_data", parse_target_data_directive);

    // Register remaining directives as generic
    for directive in OpenMpDirective::ALL {
        let name = directive.as_str();
        // Skip directives that already have custom parsers
        if !CUSTOM_PARSER_DIRECTIVES.contains(&name) {
            builder = builder.register_generic(name);
        }
    }

    builder.build()
}

pub fn parser() -> Parser {
    Parser::new(directive_registry(), clause_registry())
}
