use once_cell::sync::Lazy;
use std::borrow::Cow;
use std::collections::HashMap;

/// Typed representation of known directive names.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DirectiveName {
    Allocate,
    Allocators,
    Assume,
    EndAssume,
    Assumes,
    Atomic,
    AtomicCapture,
    AtomicCompareCapture,
    AtomicRead,
    AtomicUpdate,
    AtomicWrite,
    Barrier,
    BeginAssumes,
    BeginDeclareTarget,
    BeginDeclareVariant,
    Cancel,
    CancellationPoint,
    Critical,
    DeclareInduction,
    DeclareMapper,
    DeclareReduction,
    DeclareSimd,
    DeclareTarget,
    DeclareVariant,
    Depobj,
    Dispatch,
    Distribute,
    DistributeParallelFor,
    DistributeParallelForSimd,
    DistributeParallelLoop,
    DistributeParallelLoopSimd,
    DistributeSimd,
    // Fortran / composite variants
    DistributeParallelDo,
    DistributeParallelDoSimd,
    Do,
    DoSimd,
    EndAssumes,
    EndDeclareTarget,
    EndDeclareVariant,
    Error,
    Flush,
    Fuse,
    Groupprivate,
    For,
    ForSimd,
    Interchange,
    Interop,
    Loop,
    Reverse,
    Masked,
    MaskedTaskloop,
    MaskedTaskloopSimd,
    Master,
    MasterTaskloop,
    MasterTaskloopSimd,
    ParallelMaskedTaskloop,
    ParallelMaskedTaskloopSimd,
    Metadirective,
    BeginMetadirective,
    Nothing,
    Ordered,
    Parallel,
    ParallelDo,
    ParallelDoSimd,
    ParallelFor,
    ParallelForSimd,
    ParallelLoop,
    ParallelWorkshare,
    ParallelLoopSimd,
    Kernels,
    KernelsLoop,
    Data,
    EnterData,
    ExitData,
    HostData,

    Declare,
    Wait,
    End,
    // Fortran end directives - all map to OMPD_end but have unique enum variants
    EndParallel,
    EndDo,
    EndSimd,
    EndSections,
    EndSingle,
    EndWorkshare,
    EndOrdered,
    EndLoop,
    EndDistribute,
    EndTeams,
    EndTaskloop,
    EndTask,
    EndTaskgroup,
    EndMaster,
    EndMasked,
    EndCritical,
    EndAtomic,
    EndParallelDo,
    EndParallelFor,
    EndParallelSections,
    EndParallelWorkshare,
    EndParallelMaster,
    EndDoSimd,
    EndForSimd,
    EndParallelDoSimd,
    EndParallelForSimd,
    EndDistributeSimd,
    EndDistributeParallelDo,
    EndDistributeParallelFor,
    EndDistributeParallelDoSimd,
    EndDistributeParallelForSimd,
    EndTargetParallel,
    EndTargetParallelDo,
    EndTargetParallelFor,
    EndTargetParallelDoSimd,
    EndTargetParallelForSimd,
    EndTargetSimd,
    EndTargetTeams,
    EndTargetTeamsDistribute,
    EndTargetTeamsDistributeParallelDo,
    EndTargetTeamsDistributeParallelFor,
    EndTargetTeamsDistributeParallelDoSimd,
    EndTargetTeamsDistributeParallelForSimd,
    EndTargetTeamsDistributeSimd,
    EndTargetTeamsLoop,
    EndTeamsDistribute,
    EndTeamsDistributeParallelDo,
    EndTeamsDistributeParallelFor,
    EndTeamsDistributeParallelDoSimd,
    EndTeamsDistributeParallelForSimd,
    EndTeamsDistributeSimd,
    EndTeamsLoop,
    EndTaskloopSimd,
    EndMasterTaskloop,
    EndMasterTaskloopSimd,
    EndMaskedTaskloop,
    EndMaskedTaskloopSimd,
    EndParallelMasterTaskloop,
    EndParallelMasterTaskloopSimd,
    EndParallelMasked,
    EndParallelMaskedTaskloop,
    EndParallelMaskedTaskloopSimd,
    EndTargetParallelLoop,
    EndParallelLoop,
    EndTargetLoop,
    EndSection,
    EndScope,
    EndUnroll,
    EndTile,
    Update,
    Serial,
    SerialLoop,
    Routine,
    Set,
    Init,
    Shutdown,
    Cache,
    ParallelMasked,
    ParallelMaster,
    ParallelMasterTaskloop,
    ParallelMasterTaskloopSimd,
    ParallelSections,
    ParallelSingle,
    Requires,
    Scope,
    Scan,
    Section,
    Sections,
    Simd,
    Single,
    Split,
    Stripe,
    Target,
    TargetData,
    TargetDataComposite,
    TargetEnterData,
    TargetExitData,
    EndTarget,
    EndTargetData,
    EndTargetEnterData,
    EndTargetExitData,
    EndTargetUpdate,
    TargetLoop,
    TargetLoopSimd,
    TargetParallel,
    TargetParallelDo,
    TargetParallelDoSimd,
    TargetParallelFor,
    TargetParallelForSimd,
    TargetParallelLoop,
    TargetParallelLoopSimd,
    TargetSimd,
    TargetTeams,
    TargetTeamsDistribute,
    TargetTeamsDistributeParallelDo,
    TargetTeamsDistributeParallelDoSimd,
    TargetTeamsDistributeParallelFor,
    TargetTeamsDistributeParallelForSimd,
    TargetTeamsDistributeParallelLoop,
    TargetTeamsDistributeParallelLoopSimd,
    TargetTeamsDistributeSimd,
    TargetTeamsLoop,
    TargetTeamsLoopSimd,
    TargetUpdate,
    Task,
    TaskIteration,
    Taskgroup,
    Taskgraph,
    Taskloop,
    TaskloopSimd,
    Taskwait,
    Taskyield,
    Teams,
    TeamsDistribute,
    TeamsDistributeParallelDo,
    TeamsDistributeParallelDoSimd,
    TeamsDistributeParallelFor,
    TeamsDistributeParallelForSimd,
    TeamsDistributeParallelLoop,
    TeamsDistributeParallelLoopSimd,
    TeamsDistributeSimd,
    TeamsLoop,
    TeamsLoopSimd,
    Threadprivate,
    Tile,
    Unroll,
    Workdistribute,
    Workshare,
    NothingKnown,
    /// Unknown or unregistered directive
    Other(Cow<'static, str>),
}

/// Return the canonical OpenACCKinds.h token (UPPER_SNAKE) for a given
/// enum variant name. This helps the generator decide which directive
/// variants correspond to the same header token (for example, both
/// `EnterData` should map to `ENTER_DATA`.
pub fn canonical_header_token_for_variant(variant: &str) -> Option<&'static str> {
    match variant {
        "EnterData" => Some("ENTER_DATA"),
        "ExitData" => Some("EXIT_DATA"),
        "HostData" => Some("HOST_DATA"),
        _ => None,
    }
}

/// Typed representation for directive parameters when structured.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DirectiveParameter<'a> {
    None,
    Parenthesized(Cow<'a, str>),
    ScanExclusive(Cow<'a, str>),
    ScanInclusive(Cow<'a, str>),
    CancelConstruct(Cow<'a, str>),
    Depobj(Cow<'a, str>),
    DeclareMapper(Cow<'a, str>),
    DeclareVariant(Cow<'a, str>),
    Threadprivate(Cow<'a, str>),
    Unstructured(Cow<'a, str>),
}

// Build a static map of normalized directive names to DirectiveName variants
static DIRECTIVE_MAP: Lazy<HashMap<&'static str, DirectiveName>> = Lazy::new(|| {
    let mut m = HashMap::new();
    macro_rules! insert {
        ($k:expr, $v:expr) => {
            m.insert($k, $v);
        };
    }

    insert!("allocate", DirectiveName::Allocate);
    insert!("allocators", DirectiveName::Allocators);
    insert!("assume", DirectiveName::Assume);
    insert!("end assume", DirectiveName::EndAssume);
    insert!("assumes", DirectiveName::Assumes);
    insert!("atomic", DirectiveName::Atomic);
    insert!("atomic capture", DirectiveName::AtomicCapture);
    insert!(
        "atomic compare capture",
        DirectiveName::AtomicCompareCapture
    );
    insert!("atomic read", DirectiveName::AtomicRead);
    insert!("atomic update", DirectiveName::AtomicUpdate);
    insert!("atomic write", DirectiveName::AtomicWrite);
    insert!("barrier", DirectiveName::Barrier);
    insert!("begin assumes", DirectiveName::BeginAssumes);
    insert!("begin declare target", DirectiveName::BeginDeclareTarget);
    insert!("begin declare variant", DirectiveName::BeginDeclareVariant);
    insert!("cancel", DirectiveName::Cancel);
    insert!("cancellation point", DirectiveName::CancellationPoint);
    insert!("critical", DirectiveName::Critical);
    insert!("declare induction", DirectiveName::DeclareInduction);
    insert!("declare mapper", DirectiveName::DeclareMapper);
    insert!("declare reduction", DirectiveName::DeclareReduction);
    insert!("declare simd", DirectiveName::DeclareSimd);
    insert!("declare target", DirectiveName::DeclareTarget);
    insert!("declare variant", DirectiveName::DeclareVariant);
    insert!("depobj", DirectiveName::Depobj);
    insert!("dispatch", DirectiveName::Dispatch);
    insert!("distribute", DirectiveName::Distribute);
    insert!(
        "distribute parallel for",
        DirectiveName::DistributeParallelFor
    );
    insert!(
        "distribute parallel for simd",
        DirectiveName::DistributeParallelForSimd
    );
    insert!(
        "distribute parallel loop",
        DirectiveName::DistributeParallelLoop
    );
    insert!(
        "distribute parallel loop simd",
        DirectiveName::DistributeParallelLoopSimd
    );
    insert!("distribute simd", DirectiveName::DistributeSimd);
    insert!(
        "distribute parallel do",
        DirectiveName::DistributeParallelDo
    );
    insert!(
        "distribute parallel do simd",
        DirectiveName::DistributeParallelDoSimd
    );
    insert!("do", DirectiveName::Do);
    insert!("do simd", DirectiveName::DoSimd);
    insert!("end assumes", DirectiveName::EndAssumes);
    insert!("end declare target", DirectiveName::EndDeclareTarget);
    insert!("end declare variant", DirectiveName::EndDeclareVariant);
    insert!("error", DirectiveName::Error);
    insert!("flush", DirectiveName::Flush);
    insert!("fuse", DirectiveName::Fuse);
    insert!("groupprivate", DirectiveName::Groupprivate);
    insert!("for", DirectiveName::For);
    insert!("for simd", DirectiveName::ForSimd);
    insert!("interchange", DirectiveName::Interchange);
    insert!("interop", DirectiveName::Interop);
    insert!("loop", DirectiveName::Loop);
    insert!("reverse", DirectiveName::Reverse);
    insert!("masked", DirectiveName::Masked);
    insert!("masked taskloop", DirectiveName::MaskedTaskloop);
    insert!("masked taskloop simd", DirectiveName::MaskedTaskloopSimd);
    insert!(
        "parallel masked taskloop",
        DirectiveName::ParallelMaskedTaskloop
    );
    insert!(
        "parallel masked taskloop simd",
        DirectiveName::ParallelMaskedTaskloopSimd
    );
    insert!("master", DirectiveName::Master);
    insert!("master taskloop", DirectiveName::MasterTaskloop);
    insert!("master taskloop simd", DirectiveName::MasterTaskloopSimd);
    insert!("metadirective", DirectiveName::Metadirective);
    insert!("begin metadirective", DirectiveName::BeginMetadirective);
    insert!("nothing", DirectiveName::Nothing);
    insert!("ordered", DirectiveName::Ordered);
    insert!("parallel", DirectiveName::Parallel);
    insert!("parallel do", DirectiveName::ParallelDo);
    insert!("parallel do simd", DirectiveName::ParallelDoSimd);
    insert!("parallel for", DirectiveName::ParallelFor);
    insert!("parallel for simd", DirectiveName::ParallelForSimd);
    insert!("parallel loop", DirectiveName::ParallelLoop);
    insert!("parallel loop simd", DirectiveName::ParallelLoopSimd);
    insert!("kernels", DirectiveName::Kernels);
    insert!("kernels loop", DirectiveName::KernelsLoop);
    insert!("data", DirectiveName::Data);
    // Accept only canonical OpenACC inputs for these directives:
    // - "enter data" (space-separated)
    // - "exit data"  (space-separated)
    // - "host_data"  (canonical underscore form for host_data per spec)
    insert!("enter data", DirectiveName::EnterData);
    insert!("exit data", DirectiveName::ExitData);
    insert!("host_data", DirectiveName::HostData);
    insert!("declare", DirectiveName::Declare);
    insert!("wait", DirectiveName::Wait);
    insert!("end", DirectiveName::End);
    // Fortran end directives (with space and without space variants)
    insert!("end parallel", DirectiveName::EndParallel);
    insert!("endparallel", DirectiveName::EndParallel);
    insert!("end do", DirectiveName::EndDo);
    insert!("enddo", DirectiveName::EndDo);
    insert!("end simd", DirectiveName::EndSimd);
    insert!("endsimd", DirectiveName::EndSimd);
    insert!("end sections", DirectiveName::EndSections);
    insert!("endsections", DirectiveName::EndSections);
    insert!("end single", DirectiveName::EndSingle);
    insert!("endsingle", DirectiveName::EndSingle);
    insert!("end workshare", DirectiveName::EndWorkshare);
    insert!("endworkshare", DirectiveName::EndWorkshare);
    insert!("end ordered", DirectiveName::EndOrdered);
    insert!("endordered", DirectiveName::EndOrdered);
    insert!("end loop", DirectiveName::EndLoop);
    insert!("endloop", DirectiveName::EndLoop);
    insert!("end distribute", DirectiveName::EndDistribute);
    insert!("enddistribute", DirectiveName::EndDistribute);
    insert!("end teams", DirectiveName::EndTeams);
    insert!("endteams", DirectiveName::EndTeams);
    insert!("end taskloop", DirectiveName::EndTaskloop);
    insert!("endtaskloop", DirectiveName::EndTaskloop);
    insert!("end task", DirectiveName::EndTask);
    insert!("endtask", DirectiveName::EndTask);
    insert!("end taskgroup", DirectiveName::EndTaskgroup);
    insert!("endtaskgroup", DirectiveName::EndTaskgroup);
    insert!("end master", DirectiveName::EndMaster);
    insert!("endmaster", DirectiveName::EndMaster);
    insert!("end masked", DirectiveName::EndMasked);
    insert!("endmasked", DirectiveName::EndMasked);
    insert!("end critical", DirectiveName::EndCritical);
    insert!("endcritical", DirectiveName::EndCritical);
    insert!("end atomic", DirectiveName::EndAtomic);
    insert!("endatomic", DirectiveName::EndAtomic);
    insert!("end parallel do", DirectiveName::EndParallelDo);
    insert!("endparalleldo", DirectiveName::EndParallelDo);
    insert!("end parallel for", DirectiveName::EndParallelFor);
    insert!("end parallel sections", DirectiveName::EndParallelSections);
    insert!("endparallelsections", DirectiveName::EndParallelSections);
    insert!(
        "end parallel workshare",
        DirectiveName::EndParallelWorkshare
    );
    insert!("endparallelworkshare", DirectiveName::EndParallelWorkshare);
    insert!("end parallel master", DirectiveName::EndParallelMaster);
    insert!("endparallelmaster", DirectiveName::EndParallelMaster);
    insert!("end do simd", DirectiveName::EndDoSimd);
    insert!("enddosimd", DirectiveName::EndDoSimd);
    insert!("end for simd", DirectiveName::EndForSimd);
    insert!("end parallel do simd", DirectiveName::EndParallelDoSimd);
    insert!("endparalleldosimd", DirectiveName::EndParallelDoSimd);
    insert!("end parallel for simd", DirectiveName::EndParallelForSimd);
    insert!("end distribute simd", DirectiveName::EndDistributeSimd);
    insert!("enddistributesimd", DirectiveName::EndDistributeSimd);
    insert!(
        "end distribute parallel do",
        DirectiveName::EndDistributeParallelDo
    );
    insert!(
        "enddistributeparalleldo",
        DirectiveName::EndDistributeParallelDo
    );
    insert!(
        "end distribute parallel for",
        DirectiveName::EndDistributeParallelFor
    );
    insert!(
        "end distribute parallel do simd",
        DirectiveName::EndDistributeParallelDoSimd
    );
    insert!(
        "enddistributeparalleldosimd",
        DirectiveName::EndDistributeParallelDoSimd
    );
    insert!(
        "end distribute parallel for simd",
        DirectiveName::EndDistributeParallelForSimd
    );
    insert!("end target parallel", DirectiveName::EndTargetParallel);
    insert!("end target parallel do", DirectiveName::EndTargetParallelDo);
    insert!(
        "end target parallel for",
        DirectiveName::EndTargetParallelFor
    );
    insert!(
        "end target parallel do simd",
        DirectiveName::EndTargetParallelDoSimd
    );
    insert!(
        "end target parallel for simd",
        DirectiveName::EndTargetParallelForSimd
    );
    insert!("end target simd", DirectiveName::EndTargetSimd);
    insert!("end target teams", DirectiveName::EndTargetTeams);
    insert!(
        "end target teams distribute",
        DirectiveName::EndTargetTeamsDistribute
    );
    insert!(
        "end target teams distribute parallel do",
        DirectiveName::EndTargetTeamsDistributeParallelDo
    );
    insert!(
        "end target teams distribute parallel for",
        DirectiveName::EndTargetTeamsDistributeParallelFor
    );
    insert!(
        "end target teams distribute parallel do simd",
        DirectiveName::EndTargetTeamsDistributeParallelDoSimd
    );
    insert!(
        "end target teams distribute parallel for simd",
        DirectiveName::EndTargetTeamsDistributeParallelForSimd
    );
    insert!(
        "end target teams distribute simd",
        DirectiveName::EndTargetTeamsDistributeSimd
    );
    insert!("end target teams loop", DirectiveName::EndTargetTeamsLoop);
    insert!("end teams distribute", DirectiveName::EndTeamsDistribute);
    insert!(
        "end teams distribute parallel do",
        DirectiveName::EndTeamsDistributeParallelDo
    );
    insert!(
        "end teams distribute parallel for",
        DirectiveName::EndTeamsDistributeParallelFor
    );
    insert!(
        "end teams distribute parallel do simd",
        DirectiveName::EndTeamsDistributeParallelDoSimd
    );
    insert!(
        "end teams distribute parallel for simd",
        DirectiveName::EndTeamsDistributeParallelForSimd
    );
    insert!(
        "end teams distribute simd",
        DirectiveName::EndTeamsDistributeSimd
    );
    insert!("end teams loop", DirectiveName::EndTeamsLoop);
    insert!("end taskloop simd", DirectiveName::EndTaskloopSimd);
    insert!("end master taskloop", DirectiveName::EndMasterTaskloop);
    insert!(
        "end master taskloop simd",
        DirectiveName::EndMasterTaskloopSimd
    );
    insert!("end masked taskloop", DirectiveName::EndMaskedTaskloop);
    insert!(
        "end masked taskloop simd",
        DirectiveName::EndMaskedTaskloopSimd
    );
    insert!(
        "end parallel master taskloop",
        DirectiveName::EndParallelMasterTaskloop
    );
    insert!(
        "end parallel master taskloop simd",
        DirectiveName::EndParallelMasterTaskloopSimd
    );
    insert!("end parallel masked", DirectiveName::EndParallelMasked);
    insert!(
        "end parallel masked taskloop",
        DirectiveName::EndParallelMaskedTaskloop
    );
    insert!(
        "end parallel masked taskloop simd",
        DirectiveName::EndParallelMaskedTaskloopSimd
    );
    insert!(
        "end target parallel loop",
        DirectiveName::EndTargetParallelLoop
    );
    insert!("end parallel loop", DirectiveName::EndParallelLoop);
    insert!("end target loop", DirectiveName::EndTargetLoop);
    insert!("end section", DirectiveName::EndSection);
    insert!("end scope", DirectiveName::EndScope);
    insert!("end unroll", DirectiveName::EndUnroll);
    insert!("end tile", DirectiveName::EndTile);
    insert!("update", DirectiveName::Update);
    insert!("serial", DirectiveName::Serial);
    insert!("serial loop", DirectiveName::SerialLoop);
    insert!("routine", DirectiveName::Routine);
    insert!("set", DirectiveName::Set);
    insert!("init", DirectiveName::Init);
    insert!("shutdown", DirectiveName::Shutdown);
    insert!("cache", DirectiveName::Cache);
    insert!("parallel masked", DirectiveName::ParallelMasked);
    insert!("parallel master", DirectiveName::ParallelMaster);
    insert!(
        "parallel master taskloop",
        DirectiveName::ParallelMasterTaskloop
    );
    insert!(
        "parallel master taskloop simd",
        DirectiveName::ParallelMasterTaskloopSimd
    );
    insert!("parallel sections", DirectiveName::ParallelSections);
    insert!("parallel single", DirectiveName::ParallelSingle);
    insert!("parallel workshare", DirectiveName::ParallelWorkshare);
    insert!("requires", DirectiveName::Requires);
    insert!("scope", DirectiveName::Scope);
    insert!("scan", DirectiveName::Scan);
    insert!("section", DirectiveName::Section);
    insert!("sections", DirectiveName::Sections);
    insert!("simd", DirectiveName::Simd);
    insert!("single", DirectiveName::Single);
    insert!("split", DirectiveName::Split);
    insert!("stripe", DirectiveName::Stripe);
    insert!("target", DirectiveName::Target);
    insert!("target data", DirectiveName::TargetData);
    insert!("target data composite", DirectiveName::TargetDataComposite);
    insert!("target enter data", DirectiveName::TargetEnterData);
    insert!("target exit data", DirectiveName::TargetExitData);
    insert!("end target data", DirectiveName::EndTargetData);
    insert!("end target enter data", DirectiveName::EndTargetEnterData);
    insert!("end target exit data", DirectiveName::EndTargetExitData);
    insert!("end target update", DirectiveName::EndTargetUpdate);
    insert!("end target", DirectiveName::EndTarget);
    insert!("target loop", DirectiveName::TargetLoop);
    insert!("target loop simd", DirectiveName::TargetLoopSimd);
    insert!("target parallel", DirectiveName::TargetParallel);
    insert!("target parallel do", DirectiveName::TargetParallelDo);
    insert!(
        "target parallel do simd",
        DirectiveName::TargetParallelDoSimd
    );
    insert!("target parallel for", DirectiveName::TargetParallelFor);
    insert!(
        "target parallel for simd",
        DirectiveName::TargetParallelForSimd
    );
    insert!("target parallel loop", DirectiveName::TargetParallelLoop);
    insert!(
        "target parallel loop simd",
        DirectiveName::TargetParallelLoopSimd
    );
    insert!("target simd", DirectiveName::TargetSimd);
    insert!("target teams", DirectiveName::TargetTeams);
    insert!(
        "target teams distribute",
        DirectiveName::TargetTeamsDistribute
    );
    insert!(
        "target teams distribute parallel for",
        DirectiveName::TargetTeamsDistributeParallelFor
    );
    insert!(
        "target teams distribute parallel for simd",
        DirectiveName::TargetTeamsDistributeParallelForSimd
    );
    insert!(
        "target teams distribute parallel loop",
        DirectiveName::TargetTeamsDistributeParallelLoop
    );
    insert!(
        "target teams distribute parallel loop simd",
        DirectiveName::TargetTeamsDistributeParallelLoopSimd
    );
    insert!(
        "target teams distribute parallel do",
        DirectiveName::TargetTeamsDistributeParallelDo
    );
    insert!(
        "target teams distribute parallel do simd",
        DirectiveName::TargetTeamsDistributeParallelDoSimd
    );
    insert!(
        "target teams distribute simd",
        DirectiveName::TargetTeamsDistributeSimd
    );
    insert!("target teams loop", DirectiveName::TargetTeamsLoop);
    insert!("target teams loop simd", DirectiveName::TargetTeamsLoopSimd);
    insert!("target update", DirectiveName::TargetUpdate);
    insert!("task", DirectiveName::Task);
    insert!("task iteration", DirectiveName::TaskIteration);
    insert!("task_iteration", DirectiveName::TaskIteration);
    insert!("taskgroup", DirectiveName::Taskgroup);
    insert!("taskgraph", DirectiveName::Taskgraph);
    insert!("taskloop", DirectiveName::Taskloop);
    insert!("taskloop simd", DirectiveName::TaskloopSimd);
    insert!("taskwait", DirectiveName::Taskwait);
    insert!("taskyield", DirectiveName::Taskyield);
    insert!("teams", DirectiveName::Teams);
    insert!("teams distribute", DirectiveName::TeamsDistribute);
    insert!(
        "teams distribute parallel for",
        DirectiveName::TeamsDistributeParallelFor
    );
    insert!(
        "teams distribute parallel for simd",
        DirectiveName::TeamsDistributeParallelForSimd
    );
    insert!(
        "teams distribute parallel loop",
        DirectiveName::TeamsDistributeParallelLoop
    );
    insert!(
        "teams distribute parallel loop simd",
        DirectiveName::TeamsDistributeParallelLoopSimd
    );
    insert!(
        "teams distribute parallel do",
        DirectiveName::TeamsDistributeParallelDo
    );
    insert!(
        "teams distribute parallel do simd",
        DirectiveName::TeamsDistributeParallelDoSimd
    );
    insert!("teams distribute simd", DirectiveName::TeamsDistributeSimd);
    insert!("teams loop", DirectiveName::TeamsLoop);
    insert!("teams loop simd", DirectiveName::TeamsLoopSimd);
    insert!("threadprivate", DirectiveName::Threadprivate);
    insert!("tile", DirectiveName::Tile);
    insert!("unroll", DirectiveName::Unroll);
    insert!("workdistribute", DirectiveName::Workdistribute);
    insert!("workshare", DirectiveName::Workshare);

    m
});

/// Normalize directive names by trimming, replacing underscores with spaces,
/// and collapsing repeated whitespace.
fn normalize_directive_key(name: &str) -> String {
    let replaced = name.trim().replace('_', " ");
    replaced.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Lookup a DirectiveName from a normalized name string. If not found, returns Other variant
pub fn lookup_directive_name(name: &str) -> DirectiveName {
    // Some OpenACC directives intentionally require underscores rather than
    // accepting space-separated aliases (e.g., `host_data`). Recognize the
    // canonical underscore form before the general normalization that replaces
    // underscores with spaces.
    if name.trim().eq_ignore_ascii_case("host_data") {
        return DirectiveName::HostData;
    }

    let normalized = normalize_directive_key(name);
    let key = normalized.to_ascii_lowercase();

    // Gracefully accept block-ending aliases even when the dedicated enum
    // variant does not exist yet. Treat them as their opening directive so
    // downstream conversion can proceed instead of aborting the parse.
    if key == "end metadirective" {
        return DirectiveName::Metadirective;
    }
    if key == "end scope" {
        return DirectiveName::EndScope;
    }
    if key == "end parallel single" {
        return DirectiveName::EndParallel;
    }

    DIRECTIVE_MAP
        .get(key.as_str())
        .cloned()
        .unwrap_or(DirectiveName::Other(Cow::Owned(normalized)))
}

impl DirectiveName {
    /// Return a string slice for this directive name.
    pub fn as_str(&self) -> &str {
        match self {
            DirectiveName::Other(s) => s.as_ref(),
            // Known variants - map to canonical literals
            DirectiveName::Allocate => "allocate",
            DirectiveName::Allocators => "allocators",
            DirectiveName::Assume => "assume",
            DirectiveName::EndAssume => "end assume",
            DirectiveName::Assumes => "assumes",
            DirectiveName::Atomic => "atomic",
            DirectiveName::AtomicCapture => "atomic capture",
            DirectiveName::AtomicCompareCapture => "atomic compare capture",
            DirectiveName::AtomicRead => "atomic read",
            DirectiveName::AtomicUpdate => "atomic update",
            DirectiveName::AtomicWrite => "atomic write",
            DirectiveName::Barrier => "barrier",
            DirectiveName::BeginAssumes => "begin assumes",
            DirectiveName::BeginDeclareTarget => "begin declare target",
            DirectiveName::BeginDeclareVariant => "begin declare variant",
            DirectiveName::Cancel => "cancel",
            DirectiveName::CancellationPoint => "cancellation point",
            DirectiveName::Critical => "critical",
            DirectiveName::DeclareInduction => "declare induction",
            DirectiveName::DeclareMapper => "declare mapper",
            DirectiveName::DeclareReduction => "declare reduction",
            DirectiveName::DeclareSimd => "declare simd",
            DirectiveName::DeclareTarget => "declare target",
            DirectiveName::DeclareVariant => "declare variant",
            DirectiveName::Depobj => "depobj",
            DirectiveName::Dispatch => "dispatch",
            DirectiveName::Distribute => "distribute",
            DirectiveName::DistributeParallelFor => "distribute parallel for",
            DirectiveName::DistributeParallelForSimd => "distribute parallel for simd",
            DirectiveName::DistributeParallelLoop => "distribute parallel loop",
            DirectiveName::DistributeParallelLoopSimd => "distribute parallel loop simd",
            DirectiveName::DistributeSimd => "distribute simd",
            DirectiveName::DistributeParallelDo => "distribute parallel do",
            DirectiveName::DistributeParallelDoSimd => "distribute parallel do simd",
            DirectiveName::Do => "do",
            DirectiveName::DoSimd => "do simd",
            DirectiveName::EndAssumes => "end assumes",
            DirectiveName::EndDeclareTarget => "end declare target",
            DirectiveName::EndDeclareVariant => "end declare variant",
            DirectiveName::Error => "error",
            DirectiveName::Flush => "flush",
            DirectiveName::Fuse => "fuse",
            DirectiveName::Groupprivate => "groupprivate",
            DirectiveName::For => "for",
            DirectiveName::ForSimd => "for simd",
            DirectiveName::Interchange => "interchange",
            DirectiveName::Interop => "interop",
            DirectiveName::Loop => "loop",
            DirectiveName::Reverse => "reverse",
            DirectiveName::Masked => "masked",
            DirectiveName::MaskedTaskloop => "masked taskloop",
            DirectiveName::MaskedTaskloopSimd => "masked taskloop simd",
            DirectiveName::ParallelMaskedTaskloop => "parallel masked taskloop",
            DirectiveName::ParallelMaskedTaskloopSimd => "parallel masked taskloop simd",
            DirectiveName::Master => "master",
            DirectiveName::MasterTaskloop => "master taskloop",
            DirectiveName::MasterTaskloopSimd => "master taskloop simd",
            DirectiveName::Metadirective => "metadirective",
            DirectiveName::BeginMetadirective => "begin metadirective",
            DirectiveName::Nothing => "nothing",
            DirectiveName::Ordered => "ordered",
            DirectiveName::Parallel => "parallel",
            DirectiveName::ParallelDo => "parallel do",
            DirectiveName::ParallelDoSimd => "parallel do simd",
            DirectiveName::ParallelFor => "parallel for",
            DirectiveName::ParallelForSimd => "parallel for simd",
            DirectiveName::ParallelLoop => "parallel loop",
            DirectiveName::ParallelWorkshare => "parallel workshare",
            DirectiveName::ParallelLoopSimd => "parallel loop simd",
            DirectiveName::ParallelMasked => "parallel masked",
            DirectiveName::ParallelMaster => "parallel master",
            DirectiveName::ParallelMasterTaskloop => "parallel master taskloop",
            DirectiveName::ParallelMasterTaskloopSimd => "parallel master taskloop simd",
            DirectiveName::ParallelSections => "parallel sections",
            DirectiveName::ParallelSingle => "parallel single",
            DirectiveName::Requires => "requires",
            DirectiveName::Scope => "scope",
            DirectiveName::Scan => "scan",
            DirectiveName::Section => "section",
            DirectiveName::Sections => "sections",
            DirectiveName::Simd => "simd",
            DirectiveName::Single => "single",
            DirectiveName::Split => "split",
            DirectiveName::Stripe => "stripe",
            DirectiveName::Target => "target",
            DirectiveName::TargetData => "target data",
            DirectiveName::TargetDataComposite => "target data composite",
            DirectiveName::TargetEnterData => "target enter data",
            DirectiveName::TargetExitData => "target exit data",
            DirectiveName::EndTarget => "end target",
            DirectiveName::EndTargetData => "end target data",
            DirectiveName::EndTargetEnterData => "end target enter data",
            DirectiveName::EndTargetExitData => "end target exit data",
            DirectiveName::EndTargetUpdate => "end target update",
            DirectiveName::TargetLoop => "target loop",
            DirectiveName::TargetLoopSimd => "target loop simd",
            DirectiveName::TargetParallel => "target parallel",
            DirectiveName::TargetParallelDo => "target parallel do",
            DirectiveName::TargetParallelDoSimd => "target parallel do simd",
            DirectiveName::TargetParallelFor => "target parallel for",
            DirectiveName::TargetParallelForSimd => "target parallel for simd",
            DirectiveName::TargetParallelLoop => "target parallel loop",
            DirectiveName::TargetParallelLoopSimd => "target parallel loop simd",
            DirectiveName::TargetSimd => "target simd",
            DirectiveName::TargetTeams => "target teams",
            DirectiveName::TargetTeamsDistribute => "target teams distribute",
            DirectiveName::TargetTeamsDistributeParallelDo => "target teams distribute parallel do",
            DirectiveName::TargetTeamsDistributeParallelDoSimd => {
                "target teams distribute parallel do simd"
            }
            DirectiveName::TargetTeamsDistributeParallelFor => {
                "target teams distribute parallel for"
            }
            DirectiveName::TargetTeamsDistributeParallelForSimd => {
                "target teams distribute parallel for simd"
            }
            DirectiveName::TargetTeamsDistributeParallelLoop => {
                "target teams distribute parallel loop"
            }
            DirectiveName::TargetTeamsDistributeParallelLoopSimd => {
                "target teams distribute parallel loop simd"
            }
            DirectiveName::TargetTeamsDistributeSimd => "target teams distribute simd",
            DirectiveName::TargetTeamsLoop => "target teams loop",
            DirectiveName::TargetTeamsLoopSimd => "target teams loop simd",
            DirectiveName::TargetUpdate => "target update",
            DirectiveName::Kernels => "kernels",
            DirectiveName::KernelsLoop => "kernels loop",
            DirectiveName::Data => "data",
            DirectiveName::EnterData => "enter data",
            DirectiveName::ExitData => "exit data",
            DirectiveName::HostData => "host_data",
            DirectiveName::Declare => "declare",
            DirectiveName::Wait => "wait",
            DirectiveName::End => "end",
            DirectiveName::EndParallel => "end parallel",
            DirectiveName::EndDo => "end do",
            DirectiveName::EndSimd => "end simd",
            DirectiveName::EndSections => "end sections",
            DirectiveName::EndSingle => "end single",
            DirectiveName::EndWorkshare => "end workshare",
            DirectiveName::EndOrdered => "end ordered",
            DirectiveName::EndLoop => "end loop",
            DirectiveName::EndDistribute => "end distribute",
            DirectiveName::EndTeams => "end teams",
            DirectiveName::EndTaskloop => "end taskloop",
            DirectiveName::EndTask => "end task",
            DirectiveName::EndTaskgroup => "end taskgroup",
            DirectiveName::EndMaster => "end master",
            DirectiveName::EndMasked => "end masked",
            DirectiveName::EndCritical => "end critical",
            DirectiveName::EndAtomic => "end atomic",
            DirectiveName::EndParallelDo => "end parallel do",
            DirectiveName::EndParallelFor => "end parallel for",
            DirectiveName::EndParallelSections => "end parallel sections",
            DirectiveName::EndParallelWorkshare => "end parallel workshare",
            DirectiveName::EndParallelMaster => "end parallel master",
            DirectiveName::EndDoSimd => "end do simd",
            DirectiveName::EndForSimd => "end for simd",
            DirectiveName::EndParallelDoSimd => "end parallel do simd",
            DirectiveName::EndParallelForSimd => "end parallel for simd",
            DirectiveName::EndDistributeSimd => "end distribute simd",
            DirectiveName::EndDistributeParallelDo => "end distribute parallel do",
            DirectiveName::EndDistributeParallelFor => "end distribute parallel for",
            DirectiveName::EndDistributeParallelDoSimd => "end distribute parallel do simd",
            DirectiveName::EndDistributeParallelForSimd => "end distribute parallel for simd",
            DirectiveName::EndTargetParallel => "end target parallel",
            DirectiveName::EndTargetParallelDo => "end target parallel do",
            DirectiveName::EndTargetParallelFor => "end target parallel for",
            DirectiveName::EndTargetParallelDoSimd => "end target parallel do simd",
            DirectiveName::EndTargetParallelForSimd => "end target parallel for simd",
            DirectiveName::EndTargetSimd => "end target simd",
            DirectiveName::EndTargetTeams => "end target teams",
            DirectiveName::EndTargetTeamsDistribute => "end target teams distribute",
            DirectiveName::EndTargetTeamsDistributeParallelDo => {
                "end target teams distribute parallel do"
            }
            DirectiveName::EndTargetTeamsDistributeParallelFor => {
                "end target teams distribute parallel for"
            }
            DirectiveName::EndTargetTeamsDistributeParallelDoSimd => {
                "end target teams distribute parallel do simd"
            }
            DirectiveName::EndTargetTeamsDistributeParallelForSimd => {
                "end target teams distribute parallel for simd"
            }
            DirectiveName::EndTargetTeamsDistributeSimd => "end target teams distribute simd",
            DirectiveName::EndTargetTeamsLoop => "end target teams loop",
            DirectiveName::EndTeamsDistribute => "end teams distribute",
            DirectiveName::EndTeamsDistributeParallelDo => "end teams distribute parallel do",
            DirectiveName::EndTeamsDistributeParallelFor => "end teams distribute parallel for",
            DirectiveName::EndTeamsDistributeParallelDoSimd => {
                "end teams distribute parallel do simd"
            }
            DirectiveName::EndTeamsDistributeParallelForSimd => {
                "end teams distribute parallel for simd"
            }
            DirectiveName::EndTeamsDistributeSimd => "end teams distribute simd",
            DirectiveName::EndTeamsLoop => "end teams loop",
            DirectiveName::EndTaskloopSimd => "end taskloop simd",
            DirectiveName::EndMasterTaskloop => "end master taskloop",
            DirectiveName::EndMasterTaskloopSimd => "end master taskloop simd",
            DirectiveName::EndMaskedTaskloop => "end masked taskloop",
            DirectiveName::EndMaskedTaskloopSimd => "end masked taskloop simd",
            DirectiveName::EndParallelMasterTaskloop => "end parallel master taskloop",
            DirectiveName::EndParallelMasterTaskloopSimd => "end parallel master taskloop simd",
            DirectiveName::EndParallelMasked => "end parallel masked",
            DirectiveName::EndParallelMaskedTaskloop => "end parallel masked taskloop",
            DirectiveName::EndParallelMaskedTaskloopSimd => "end parallel masked taskloop simd",
            DirectiveName::EndTargetParallelLoop => "end target parallel loop",
            DirectiveName::EndParallelLoop => "end parallel loop",
            DirectiveName::EndTargetLoop => "end target loop",
            DirectiveName::EndSection => "end section",
            DirectiveName::EndScope => "end scope",
            DirectiveName::EndUnroll => "end unroll",
            DirectiveName::EndTile => "end tile",
            DirectiveName::Update => "update",
            DirectiveName::Serial => "serial",
            DirectiveName::SerialLoop => "serial loop",
            DirectiveName::Routine => "routine",
            DirectiveName::Set => "set",
            DirectiveName::Init => "init",
            DirectiveName::Shutdown => "shutdown",
            DirectiveName::Cache => "cache",
            DirectiveName::Task => "task",
            DirectiveName::TaskIteration => "task iteration",
            DirectiveName::Taskgroup => "taskgroup",
            DirectiveName::Taskgraph => "taskgraph",
            DirectiveName::Taskloop => "taskloop",
            DirectiveName::TaskloopSimd => "taskloop simd",
            DirectiveName::Taskwait => "taskwait",
            DirectiveName::Taskyield => "taskyield",
            DirectiveName::Teams => "teams",
            DirectiveName::TeamsDistribute => "teams distribute",
            DirectiveName::TeamsDistributeParallelDo => "teams distribute parallel do",
            DirectiveName::TeamsDistributeParallelDoSimd => "teams distribute parallel do simd",
            DirectiveName::TeamsDistributeParallelFor => "teams distribute parallel for",
            DirectiveName::TeamsDistributeParallelForSimd => "teams distribute parallel for simd",
            DirectiveName::TeamsDistributeParallelLoop => "teams distribute parallel loop",
            DirectiveName::TeamsDistributeParallelLoopSimd => "teams distribute parallel loop simd",
            DirectiveName::TeamsDistributeSimd => "teams distribute simd",
            DirectiveName::TeamsLoop => "teams loop",
            DirectiveName::TeamsLoopSimd => "teams loop simd",
            DirectiveName::Threadprivate => "threadprivate",
            DirectiveName::Tile => "tile",
            DirectiveName::Unroll => "unroll",
            DirectiveName::Workdistribute => "workdistribute",
            DirectiveName::Workshare => "workshare",
            DirectiveName::NothingKnown => "",
        }
    }

    /// Return the lowercase string
    pub fn to_lowercase(&self) -> String {
        self.as_str().to_ascii_lowercase()
    }

    /// Case-insensitive compare
    pub fn eq_ignore_ascii_case(&self, other: &str) -> bool {
        self.as_str().eq_ignore_ascii_case(other)
    }
}

impl std::fmt::Display for DirectiveName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl PartialEq<&str> for DirectiveName {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

// tests moved to end of file to satisfy clippy items-after-test-module

impl PartialEq<str> for DirectiveName {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

// Common conversions and helpers so callers that previously relied on
// string-like behaviour of `Cow<'a, str>` continue to work with
// `DirectiveName<'a>` during migration.
impl AsRef<str> for DirectiveName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<&str> for DirectiveName {
    fn from(s: &str) -> Self {
        lookup_directive_name(s)
    }
}

impl<'b> From<std::borrow::Cow<'b, str>> for DirectiveName {
    fn from(c: std::borrow::Cow<'b, str>) -> Self {
        // Use the string slice to lookup and produce an owned variant when needed.
        lookup_directive_name(c.as_ref())
    }
}

impl DirectiveName {
    /// Return the length (number of bytes) of this directive name string.
    pub fn len(&self) -> usize {
        self.as_str().len()
    }

    /// Return true if this directive name is empty.
    pub fn is_empty(&self) -> bool {
        self.as_str().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_canonical_teams_distribute_simd() {
        let d = lookup_directive_name("teams distribute simd");
        assert_eq!(d, DirectiveName::TeamsDistributeSimd);
    }
}
