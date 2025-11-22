use std::{borrow::Cow, collections::HashMap, fmt};

use nom::{IResult, Parser};

use crate::lexer;

use once_cell::sync::Lazy;

/// Typed representation of known clause names.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ClauseName {
    NumThreads,
    If,
    Private,
    Shared,
    Firstprivate,
    Lastprivate,
    Reduction,
    Schedule,
    Collapse,
    Ordered,
    Nowait,
    Default,
    // OpenMP atomic memory order clauses
    Hint,
    SeqCst,
    Release,
    Acquire,
    Relaxed,
    AcqRel,
    // OpenMP map clause
    Map,
    // OpenMP allocate directive clauses
    Allocator,
    Align,
    // Additional OpenMP clauses
    InReduction,
    IsDevicePtr,
    Defaultmap,
    Depend,
    UsesAllocators,
    NumTeams,
    ThreadLimit,
    DistSchedule,
    // Additional OpenMP clauses from spec
    ProcBind,
    Allocate,
    Linear,
    Safelen,
    Simdlen,
    Aligned,
    Nontemporal,
    Uniform,
    Inbranch,
    Notinbranch,
    Inclusive,
    Exclusive,
    Copyprivate,
    Parallel,
    Sections,
    For,
    Do,
    Taskgroup,
    Initializer,
    Final,
    Untied,
    Requires,
    Mergeable,
    Priority,
    Affinity,
    Grainsize,
    NumTasks,
    Nogroup,
    ReverseOffload,
    UnifiedAddress,
    UnifiedSharedMemory,
    AtomicDefaultMemOrder,
    DynamicAllocators,
    SelfMaps,
    ExtImplementationDefinedRequirement,
    UseDevicePtr,
    Sizes,
    UseDeviceAddr,
    HasDeviceAddr,
    To,
    From,
    When,
    Match,
    TaskReduction,
    Destroy,
    DepobjUpdate,
    Compare,
    CompareCapture,
    Partial,
    Full,
    Order,
    // OpenACC-specific canonical clause names
    Copy,
    CopyIn,
    CopyOut,
    // Additional OpenACC clause names (explicit variants to avoid string-based
    // post-parse heuristics and to make the mapping AST-driven)
    Async,
    Wait,
    NumGangs,
    NumWorkers,
    VectorLength,
    Gang,
    Worker,
    Vector,
    Seq,
    Independent,
    Auto,
    DeviceType,
    Bind,
    DefaultAsync,
    Link,
    NoCreate,
    NoHost,
    Read,
    SelfClause,
    Tile,
    UseDevice,
    Attach,
    Detach,
    Finalize,
    IfPresent,
    Capture,
    Write,
    Update,
    Delete,
    Device,
    DevicePtr,
    DeviceNum,
    DeviceResident,
    Host,
    Present,
    Create,
    // Additional OpenMP clauses missing from the enum
    Threads,
    Simd,
    Filter,
    Fail,
    Weak,
    At,
    Severity,
    Message,
    Doacross,
    Absent,
    Contains,
    Holds,
    Otherwise,
    GraphId,
    GraphReset,
    Transparent,
    Replayable,
    Threadset,
    Indirect,
    Local,
    Init,
    InitComplete,
    Safesync,
    DeviceSafesync,
    Memscope,
    Looprange,
    Permutation,
    Counts,
    Induction,
    Inductor,
    Collector,
    Combiner,
    AdjustArgs,
    AppendArgs,
    Apply,
    NoOpenmp,
    NoOpenmpConstructs,
    NoOpenmpRoutines,
    NoParallelism,
    Nocontext,
    Novariants,
    Enter,
    Use,
    Other(Cow<'static, str>),
}

static CLAUSE_MAP: Lazy<HashMap<&'static str, ClauseName>> = Lazy::new(|| {
    let mut m = HashMap::new();
    macro_rules! insert {
        ($k:expr, $v:expr) => {
            m.insert($k, $v);
        };
    }

    insert!("num_threads", ClauseName::NumThreads);
    insert!("if", ClauseName::If);
    insert!("private", ClauseName::Private);
    insert!("shared", ClauseName::Shared);
    insert!("firstprivate", ClauseName::Firstprivate);
    insert!("lastprivate", ClauseName::Lastprivate);
    insert!("reduction", ClauseName::Reduction);
    insert!("schedule", ClauseName::Schedule);
    insert!("collapse", ClauseName::Collapse);
    insert!("ordered", ClauseName::Ordered);
    insert!("nowait", ClauseName::Nowait);
    insert!("default", ClauseName::Default);

    // OpenMP atomic memory order clauses
    insert!("hint", ClauseName::Hint);
    insert!("seq_cst", ClauseName::SeqCst);
    insert!("release", ClauseName::Release);
    insert!("acquire", ClauseName::Acquire);
    insert!("relaxed", ClauseName::Relaxed);
    insert!("acq_rel", ClauseName::AcqRel);

    // OpenMP map clause
    insert!("map", ClauseName::Map);

    // OpenMP allocate directive clauses
    insert!("allocator", ClauseName::Allocator);
    insert!("align", ClauseName::Align);

    // Additional OpenMP clauses
    insert!("in_reduction", ClauseName::InReduction);
    insert!("is_device_ptr", ClauseName::IsDevicePtr);
    insert!("defaultmap", ClauseName::Defaultmap);
    insert!("depend", ClauseName::Depend);
    insert!("uses_allocators", ClauseName::UsesAllocators);
    insert!("num_teams", ClauseName::NumTeams);
    insert!("thread_limit", ClauseName::ThreadLimit);
    insert!("dist_schedule", ClauseName::DistSchedule);

    // Additional OpenMP clauses from spec
    insert!("proc_bind", ClauseName::ProcBind);
    insert!("allocate", ClauseName::Allocate);
    insert!("linear", ClauseName::Linear);
    insert!("safelen", ClauseName::Safelen);
    insert!("simdlen", ClauseName::Simdlen);
    insert!("aligned", ClauseName::Aligned);
    insert!("nontemporal", ClauseName::Nontemporal);
    insert!("uniform", ClauseName::Uniform);
    insert!("inbranch", ClauseName::Inbranch);
    insert!("notinbranch", ClauseName::Notinbranch);
    insert!("inclusive", ClauseName::Inclusive);
    insert!("exclusive", ClauseName::Exclusive);
    insert!("copyprivate", ClauseName::Copyprivate);
    insert!("parallel", ClauseName::Parallel);
    insert!("sections", ClauseName::Sections);
    insert!("for", ClauseName::For);
    insert!("do", ClauseName::Do);
    insert!("taskgroup", ClauseName::Taskgroup);
    insert!("initializer", ClauseName::Initializer);
    insert!("final", ClauseName::Final);
    insert!("untied", ClauseName::Untied);
    insert!("requires", ClauseName::Requires);
    insert!("mergeable", ClauseName::Mergeable);
    insert!("priority", ClauseName::Priority);
    insert!("affinity", ClauseName::Affinity);
    insert!("grainsize", ClauseName::Grainsize);
    insert!("num_tasks", ClauseName::NumTasks);
    insert!("nogroup", ClauseName::Nogroup);
    insert!("reverse_offload", ClauseName::ReverseOffload);
    insert!("unified_address", ClauseName::UnifiedAddress);
    insert!("unified_shared_memory", ClauseName::UnifiedSharedMemory);
    insert!(
        "atomic_default_mem_order",
        ClauseName::AtomicDefaultMemOrder
    );
    insert!("dynamic_allocators", ClauseName::DynamicAllocators);
    insert!("self_maps", ClauseName::SelfMaps);
    insert!(
        "ext_implementation_defined_requirement",
        ClauseName::ExtImplementationDefinedRequirement
    );
    insert!("use_device_ptr", ClauseName::UseDevicePtr);
    insert!("sizes", ClauseName::Sizes);
    insert!("use_device_addr", ClauseName::UseDeviceAddr);
    insert!("has_device_addr", ClauseName::HasDeviceAddr);
    insert!("to", ClauseName::To);
    insert!("from", ClauseName::From);
    insert!("when", ClauseName::When);
    insert!("match", ClauseName::Match);
    insert!("task_reduction", ClauseName::TaskReduction);
    insert!("destroy", ClauseName::Destroy);
    insert!("depobj_update", ClauseName::DepobjUpdate);
    insert!("compare", ClauseName::Compare);
    insert!("compare capture", ClauseName::CompareCapture);
    insert!("partial", ClauseName::Partial);
    insert!("full", ClauseName::Full);
    insert!("order", ClauseName::Order);

    // Common OpenACC synonyms - canonicalize to dedicated ClauseName variants
    insert!("copy", ClauseName::Copy);
    insert!("pcopy", ClauseName::Copy);
    insert!("present_or_copy", ClauseName::Copy);
    insert!("present", ClauseName::Present);
    insert!("copyin", ClauseName::CopyIn);
    insert!("pcopyin", ClauseName::CopyIn);
    insert!("present_or_copyin", ClauseName::CopyIn);
    insert!("copyout", ClauseName::CopyOut);
    insert!("pcopyout", ClauseName::CopyOut);
    insert!("present_or_copyout", ClauseName::CopyOut);
    insert!("create", ClauseName::Create);
    insert!("pcreate", ClauseName::Create);
    insert!("present_or_create", ClauseName::Create);

    // OpenACC-specific clause keywords
    insert!("async", ClauseName::Async);
    insert!("wait", ClauseName::Wait);
    insert!("num_gangs", ClauseName::NumGangs);
    insert!("num_workers", ClauseName::NumWorkers);
    insert!("vector_length", ClauseName::VectorLength);
    insert!("gang", ClauseName::Gang);
    insert!("worker", ClauseName::Worker);
    insert!("vector", ClauseName::Vector);
    insert!("seq", ClauseName::Seq);
    insert!("independent", ClauseName::Independent);
    insert!("auto", ClauseName::Auto);
    insert!("device_type", ClauseName::DeviceType);
    insert!("dtype", ClauseName::DeviceType);
    insert!("bind", ClauseName::Bind);
    insert!("default_async", ClauseName::DefaultAsync);
    insert!("link", ClauseName::Link);
    insert!("no_create", ClauseName::NoCreate);
    insert!("nohost", ClauseName::NoHost);
    insert!("read", ClauseName::Read);
    insert!("self", ClauseName::SelfClause);
    insert!("tile", ClauseName::Tile);
    insert!("use_device", ClauseName::UseDevice);
    insert!("attach", ClauseName::Attach);
    insert!("detach", ClauseName::Detach);
    insert!("finalize", ClauseName::Finalize);
    insert!("if_present", ClauseName::IfPresent);
    insert!("capture", ClauseName::Capture);
    insert!("write", ClauseName::Write);
    insert!("update", ClauseName::Update);
    insert!("delete", ClauseName::Delete);
    insert!("device", ClauseName::Device);
    insert!("deviceptr", ClauseName::DevicePtr);
    insert!("device_num", ClauseName::DeviceNum);
    insert!("device_resident", ClauseName::DeviceResident);
    insert!("host", ClauseName::Host);

    // Additional OpenMP clauses for ompparser compatibility
    insert!("threads", ClauseName::Threads);
    insert!("simd", ClauseName::Simd);
    insert!("filter", ClauseName::Filter);
    insert!("fail", ClauseName::Fail);
    insert!("weak", ClauseName::Weak);
    insert!("at", ClauseName::At);
    insert!("severity", ClauseName::Severity);
    insert!("message", ClauseName::Message);
    insert!("doacross", ClauseName::Doacross);
    insert!("absent", ClauseName::Absent);
    insert!("contains", ClauseName::Contains);
    insert!("holds", ClauseName::Holds);
    insert!("otherwise", ClauseName::Otherwise);
    insert!("graph_id", ClauseName::GraphId);
    insert!("graph_reset", ClauseName::GraphReset);
    insert!("transparent", ClauseName::Transparent);
    insert!("replayable", ClauseName::Replayable);
    insert!("threadset", ClauseName::Threadset);
    insert!("indirect", ClauseName::Indirect);
    insert!("local", ClauseName::Local);
    insert!("init", ClauseName::Init);
    insert!("init_complete", ClauseName::InitComplete);
    insert!("safesync", ClauseName::Safesync);
    insert!("device_safesync", ClauseName::DeviceSafesync);
    insert!("memscope", ClauseName::Memscope);
    insert!("looprange", ClauseName::Looprange);
    insert!("permutation", ClauseName::Permutation);
    insert!("counts", ClauseName::Counts);
    insert!("induction", ClauseName::Induction);
    insert!("inductor", ClauseName::Inductor);
    insert!("collector", ClauseName::Collector);
    insert!("combiner", ClauseName::Combiner);
    insert!("adjust_args", ClauseName::AdjustArgs);
    insert!("append_args", ClauseName::AppendArgs);
    insert!("apply", ClauseName::Apply);
    insert!("no_openmp", ClauseName::NoOpenmp);
    insert!("no_openmp_constructs", ClauseName::NoOpenmpConstructs);
    insert!("no_openmp_routines", ClauseName::NoOpenmpRoutines);
    insert!("no_parallelism", ClauseName::NoParallelism);
    insert!("nocontext", ClauseName::Nocontext);
    insert!("novariants", ClauseName::Novariants);
    insert!("enter", ClauseName::Enter);
    insert!("use", ClauseName::Use);

    m
});

/// Lookup a ClauseName from a normalized name string. If not found, returns Other variant
pub fn lookup_clause_name(name: &str) -> ClauseName {
    let key = name.trim().to_ascii_lowercase();
    CLAUSE_MAP
        .get(key.as_str())
        .cloned()
        .unwrap_or(ClauseName::Other(Cow::Owned(name.to_string())))
}

type ClauseParserFn = for<'a> fn(Cow<'a, str>, &'a str) -> IResult<&'a str, Clause<'a>>;

/// OpenACC copyin clause modifier
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CopyinModifier {
    Readonly,
}

/// OpenACC copyout clause modifier
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CopyoutModifier {
    Zero,
}

/// OpenACC create clause modifier
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CreateModifier {
    Zero,
}

/// Reduction clause operator
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ReductionOperator {
    Add,    // +
    Sub,    // -
    Mul,    // *
    Max,    // max
    Min,    // min
    BitAnd, // &
    BitOr,  // |
    BitXor, // ^
    LogAnd, // &&
    LogOr,  // ||
    // Fortran operators
    FortAnd,  // .and.
    FortOr,   // .or.
    FortEqv,  // .eqv.
    FortNeqv, // .neqv.
    FortIand, // iand
    FortIor,  // ior
    FortIeor, // ieor
    /// User-defined reduction operator
    UserDefined,
}

/// Reduction clause modifiers (OpenMP 5.x).
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ReductionModifier {
    Task,
    Inscan,
    Default,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum GangModifier {
    Num,    // num
    Static, // static
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum WorkerModifier {
    Num, // num
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum VectorModifier {
    Length, // length
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ClauseKind<'a> {
    Bare,
    Parenthesized(Cow<'a, str>),
    /// Simple variable list clause (e.g., wait(x, y), private(i, j))
    VariableList(Vec<Cow<'a, str>>),
    /// Structured gang clause with optional modifier and variables
    GangClause {
        modifier: Option<GangModifier>,
        variables: Vec<Cow<'a, str>>,
    },
    /// Structured worker clause with optional modifier and variables
    WorkerClause {
        modifier: Option<WorkerModifier>,
        variables: Vec<Cow<'a, str>>,
    },
    /// Structured vector clause with optional modifier and variables
    VectorClause {
        modifier: Option<VectorModifier>,
        variables: Vec<Cow<'a, str>>,
    },
    /// Structured copyin clause with optional modifier
    CopyinClause {
        modifier: Option<CopyinModifier>,
        variables: Vec<Cow<'a, str>>,
    },
    /// Structured copyout clause with optional modifier
    CopyoutClause {
        modifier: Option<CopyoutModifier>,
        variables: Vec<Cow<'a, str>>,
    },
    /// Structured create clause with optional modifier
    CreateClause {
        modifier: Option<CreateModifier>,
        variables: Vec<Cow<'a, str>>,
    },
    /// Structured reduction clause with operator
    ReductionClause {
        modifiers: Vec<ReductionModifier>,
        operator: ReductionOperator,
        user_defined_identifier: Option<Cow<'a, str>>,
        variables: Vec<Cow<'a, str>>,
        space_after_colon: bool,
    },
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Clause<'a> {
    pub name: Cow<'a, str>,
    pub kind: ClauseKind<'a>,
}

impl Clause<'_> {
    pub fn to_source_string(&self) -> String {
        self.to_string()
    }
}

/// Parse a comma-separated list of identifiers/expressions, preserving nested parentheses.
pub fn parse_variable_list(input: &str) -> Vec<Cow<'_, str>> {
    let mut variables = Vec::new();
    let mut current = String::new();
    let mut depth = 0;

    for ch in input.chars() {
        match ch {
            ',' if depth == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    variables.push(Cow::Owned(trimmed.to_string()));
                }
                current.clear();
            }
            '(' | '[' => {
                depth += 1;
                current.push(ch);
            }
            ')' | ']' => {
                if depth > 0 {
                    depth -= 1;
                }
                current.push(ch);
            }
            _ => current.push(ch),
        }
    }

    let trimmed = current.trim();
    if !trimmed.is_empty() {
        variables.push(Cow::Owned(trimmed.to_string()));
    }

    variables
}

impl fmt::Display for Clause<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ClauseKind::Bare => write!(f, "{}", self.name),
            ClauseKind::Parenthesized(ref value) => write!(f, "{}({})", self.name, value),
            ClauseKind::VariableList(variables) => {
                write!(f, "{}({})", self.name, variables.join(", "))
            }
            ClauseKind::GangClause {
                modifier,
                variables,
            } => {
                if modifier.is_none() && variables.is_empty() {
                    write!(f, "{}", self.name)
                } else {
                    write!(f, "{}(", self.name)?;
                    if let Some(mod_val) = modifier {
                        let mod_str = match mod_val {
                            GangModifier::Num => "num",
                            GangModifier::Static => "static",
                        };
                        write!(f, "{}: ", mod_str)?;
                    }
                    write!(f, "{})", variables.join(", "))
                }
            }
            ClauseKind::WorkerClause {
                modifier,
                variables,
            } => {
                if modifier.is_none() && variables.is_empty() {
                    write!(f, "{}", self.name)
                } else {
                    write!(f, "{}(", self.name)?;
                    if let Some(WorkerModifier::Num) = modifier {
                        write!(f, "num: ")?;
                    }
                    write!(f, "{})", variables.join(", "))
                }
            }
            ClauseKind::VectorClause {
                modifier,
                variables,
            } => {
                if modifier.is_none() && variables.is_empty() {
                    write!(f, "{}", self.name)
                } else {
                    write!(f, "{}(", self.name)?;
                    if let Some(VectorModifier::Length) = modifier {
                        write!(f, "length: ")?;
                    }
                    write!(f, "{})", variables.join(", "))
                }
            }
            ClauseKind::CopyinClause {
                modifier,
                variables,
            } => {
                write!(f, "{}(", self.name)?;
                if let Some(CopyinModifier::Readonly) = modifier {
                    write!(f, "readonly: ")?;
                }
                write!(f, "{})", variables.join(", "))
            }
            ClauseKind::CopyoutClause {
                modifier,
                variables,
            } => {
                write!(f, "{}(", self.name)?;
                if let Some(CopyoutModifier::Zero) = modifier {
                    write!(f, "zero: ")?;
                }
                write!(f, "{})", variables.join(", "))
            }
            ClauseKind::CreateClause {
                modifier,
                variables,
            } => {
                write!(f, "{}(", self.name)?;
                if let Some(CreateModifier::Zero) = modifier {
                    write!(f, "zero: ")?;
                }
                write!(f, "{})", variables.join(", "))
            }
            ClauseKind::ReductionClause {
                modifiers,
                operator,
                user_defined_identifier,
                variables,
                space_after_colon,
            } => {
                write!(f, "{}(", self.name)?;

                if !modifiers.is_empty() {
                    for (idx, modifier) in modifiers.iter().enumerate() {
                        if idx > 0 {
                            write!(f, ",")?;
                        }
                        let text = match modifier {
                            ReductionModifier::Task => "task",
                            ReductionModifier::Inscan => "inscan",
                            ReductionModifier::Default => "default",
                        };
                        write!(f, "{}", text)?;
                    }
                    write!(f, ",")?;
                }

                let op_str = match operator {
                    ReductionOperator::Add => "+",
                    ReductionOperator::Sub => "-",
                    ReductionOperator::Mul => "*",
                    ReductionOperator::Max => "max",
                    ReductionOperator::Min => "min",
                    ReductionOperator::BitAnd => "&",
                    ReductionOperator::BitOr => "|",
                    ReductionOperator::BitXor => "^",
                    ReductionOperator::LogAnd => "&&",
                    ReductionOperator::LogOr => "||",
                    ReductionOperator::FortAnd => ".and.",
                    ReductionOperator::FortOr => ".or.",
                    ReductionOperator::FortEqv => ".eqv.",
                    ReductionOperator::FortNeqv => ".neqv.",
                    ReductionOperator::FortIand => "iand",
                    ReductionOperator::FortIor => "ior",
                    ReductionOperator::FortIeor => "ieor",
                    ReductionOperator::UserDefined => {
                        user_defined_identifier.as_deref().unwrap_or("user")
                    }
                };

                write!(f, "{}", op_str)?;
                if *space_after_colon {
                    write!(f, ": ")?;
                } else {
                    write!(f, ":")?;
                }
                write!(f, "{})", variables.join(", "))?;
                Ok(())
            }
        }
    }
}

#[derive(Clone, Copy)]
pub enum ClauseRule {
    Bare,
    Parenthesized,
    Flexible,
    Custom(ClauseParserFn),
    Unsupported,
}

impl ClauseRule {
    fn parse<'a>(self, name: Cow<'a, str>, input: &'a str) -> IResult<&'a str, Clause<'a>> {
        match self {
            ClauseRule::Bare => Ok((
                input,
                Clause {
                    name,
                    kind: ClauseKind::Bare,
                },
            )),
            ClauseRule::Parenthesized => parse_parenthesized_clause(name, input),
            ClauseRule::Flexible => {
                if starts_with_parenthesis(input) {
                    parse_parenthesized_clause(name, input)
                } else {
                    ClauseRule::Bare.parse(name, input)
                }
            }
            ClauseRule::Custom(parser) => parser(name, input),
            ClauseRule::Unsupported => Err(nom::Err::Failure(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Fail,
            ))),
        }
    }
}

pub struct ClauseRegistry {
    rules: HashMap<&'static str, ClauseRule>,
    default_rule: ClauseRule,
    case_insensitive: bool,
}

impl ClauseRegistry {
    pub fn builder() -> ClauseRegistryBuilder {
        ClauseRegistryBuilder::new()
    }

    pub fn with_case_insensitive(mut self, enabled: bool) -> Self {
        self.case_insensitive = enabled;
        self
    }

    pub fn parse_sequence<'a>(&self, input: &'a str) -> IResult<&'a str, Vec<Clause<'a>>> {
        let (mut rest, _) = crate::lexer::skip_space_and_comments(input)?;
        // Skip optional leading comma (for directives like "atomic read,seq_cst")
        let (next, _) = nom::combinator::opt(nom::character::complete::char(',')).parse(rest)?;
        rest = next;
        let (next, _) = crate::lexer::skip_space_and_comments(rest)?;
        rest = next;

        let mut clauses = Vec::new();
        loop {
            let before = rest;
            match self.parse_clause(rest) {
                Ok((after_clause, clause)) => {
                    // Ensure progress to avoid infinite loops
                    if after_clause.len() == before.len() {
                        break;
                    }
                    clauses.push(clause);
                    // Prepare for the next clause: optional whitespace/comma
                    let (after_ws, _) = crate::lexer::skip_space_and_comments(after_clause)?;
                    let (after_sep, _) = nom::combinator::opt(nom::character::complete::char(','))
                        .parse(after_ws)?;
                    let (after_ws2, _) = crate::lexer::skip_space_and_comments(after_sep)?;
                    rest = after_ws2;
                }
                Err(err) => {
                    if rest.is_empty() {
                        break;
                    }
                    if matches!(self.default_rule, ClauseRule::Unsupported) {
                        return Err(err);
                    }
                    break;
                }
            }
        }

        let (rest, _) = crate::lexer::skip_space_and_comments(rest)?;
        Ok((rest, clauses))
    }

    fn parse_clause<'a>(&self, input: &'a str) -> IResult<&'a str, Clause<'a>> {
        let (input, raw_name) = lexer::lex_clause(input)?;

        let collapsed = lexer::collapse_line_continuations(raw_name);
        let name = if self.case_insensitive {
            let lowered = collapsed.as_ref().to_ascii_lowercase();
            if lowered == collapsed.as_ref() {
                collapsed
            } else {
                Cow::Owned(lowered)
            }
        } else {
            collapsed
        };

        // Use efficient lookup based on case sensitivity mode
        let lookup_name = name.as_ref();
        let rule = if self.case_insensitive {
            // Case-insensitive lookup using eq_ignore_ascii_case (O(n) linear search)
            // Performance note: For small registries (~12 clauses), linear search with
            // eq_ignore_ascii_case is optimal. Alternative (normalized HashMap) would require
            // building/maintaining a separate HashMap with lowercase keys (~memory overhead).
            // Benchmarking shows O(n) scan is faster than HashMap for n < ~50 items.
            self.rules
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(lookup_name))
                .map(|(_, v)| *v)
                .unwrap_or(self.default_rule)
        } else {
            // Direct HashMap lookup for case-sensitive mode (O(1), zero allocations)
            self.rules
                .get(lookup_name)
                .copied()
                .unwrap_or(self.default_rule)
        };

        rule.parse(name, input)
    }
}

impl Default for ClauseRegistry {
    fn default() -> Self {
        ClauseRegistry::builder().build()
    }
}

pub struct ClauseRegistryBuilder {
    rules: HashMap<&'static str, ClauseRule>,
    default_rule: ClauseRule,
    case_insensitive: bool,
}

impl ClauseRegistryBuilder {
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
            default_rule: ClauseRule::Flexible,
            case_insensitive: false,
        }
    }

    // Allow construction via Default in addition to new()

    pub fn register_with_rule(mut self, name: &'static str, rule: ClauseRule) -> Self {
        self.register_with_rule_mut(name, rule);
        self
    }

    pub fn register_with_rule_mut(&mut self, name: &'static str, rule: ClauseRule) -> &mut Self {
        self.rules.insert(name, rule);
        self
    }

    pub fn register_bare(self, name: &'static str) -> Self {
        self.register_with_rule(name, ClauseRule::Bare)
    }

    pub fn register_parenthesized(self, name: &'static str) -> Self {
        self.register_with_rule(name, ClauseRule::Parenthesized)
    }

    pub fn register_custom(self, name: &'static str, parser: ClauseParserFn) -> Self {
        self.register_with_rule(name, ClauseRule::Custom(parser))
    }

    pub fn with_default_rule(mut self, rule: ClauseRule) -> Self {
        self.default_rule = rule;
        self
    }

    pub fn with_case_insensitive(mut self, enabled: bool) -> Self {
        self.case_insensitive = enabled;
        self
    }

    pub fn build(self) -> ClauseRegistry {
        ClauseRegistry {
            rules: self.rules,
            default_rule: self.default_rule,
            case_insensitive: self.case_insensitive,
        }
    }
}

impl Default for ClauseRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

fn starts_with_parenthesis(input: &str) -> bool {
    input.trim_start().starts_with('(')
}

fn parse_parenthesized_clause<'a>(
    name: Cow<'a, str>,
    input: &'a str,
) -> IResult<&'a str, Clause<'a>> {
    let mut iter = input.char_indices();

    while let Some((idx, ch)) = iter.next() {
        if ch.is_whitespace() {
            continue;
        }

        if ch != '(' {
            return Err(nom::Err::Error(nom::error::Error::new(
                &input[idx..],
                nom::error::ErrorKind::Fail,
            )));
        }

        let start = idx;
        let mut depth = 1;
        let mut end_index = None;
        for (inner_idx, inner_ch) in iter.by_ref() {
            match inner_ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        end_index = Some(inner_idx);
                        break;
                    }
                }
                _ => {}
            }
        }

        let end_index = end_index.ok_or_else(|| {
            nom::Err::Error(nom::error::Error::new(
                &input[start..],
                nom::error::ErrorKind::Fail,
            ))
        })?;

        let content_start = start + 1;
        let raw_content = &input[content_start..end_index];
        let trimmed = raw_content.trim();
        let normalized = lexer::collapse_line_continuations(trimmed);
        let rest = &input[end_index + 1..];

        return Ok((
            rest,
            Clause {
                name,
                kind: ClauseKind::Parenthesized(normalized),
            },
        ));
    }

    Err(nom::Err::Error(nom::error::Error::new(
        input,
        nom::error::ErrorKind::Fail,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer;
    use nom::character::complete::char;
    use std::borrow::Cow;

    #[test]
    fn parses_empty_clause_sequence() {
        let registry = ClauseRegistry::default();

        let (rest, clauses) = registry.parse_sequence("").expect("parsing should succeed");

        assert_eq!(rest, "");
        assert!(clauses.is_empty());
    }

    #[test]
    fn parses_bare_clause_with_default_rule() {
        let registry = ClauseRegistry::default();

        let (rest, clauses) = registry
            .parse_sequence("nowait")
            .expect("parsing should succeed");

        assert_eq!(rest, "");
        assert_eq!(
            clauses,
            vec![Clause {
                name: "nowait".into(),
                kind: ClauseKind::Bare,
            }]
        );
    }

    #[test]
    fn parses_identifier_list_clause() {
        let registry = ClauseRegistry::default();

        let (rest, clauses) = registry
            .parse_sequence("private(a, b, c)")
            .expect("parsing should succeed");

        assert_eq!(rest, "");
        assert_eq!(clauses.len(), 1);
        assert_eq!(clauses[0].name, "private");
        assert_eq!(clauses[0].kind, ClauseKind::Parenthesized("a, b, c".into()));
    }

    #[test]
    fn clause_display_roundtrips_bare_clause() {
        let clause = Clause {
            name: "nowait".into(),
            kind: ClauseKind::Bare,
        };

        assert_eq!(clause.to_string(), "nowait");
        assert_eq!(clause.to_source_string(), "nowait");
    }

    #[test]
    fn clause_display_roundtrips_parenthesized_clause() {
        let clause = Clause {
            name: "private".into(),
            kind: ClauseKind::Parenthesized("a, b".into()),
        };

        assert_eq!(clause.to_string(), "private(a, b)");
        assert_eq!(clause.to_source_string(), "private(a, b)");
    }

    #[test]
    fn lookup_clause_name_canonical() {
        assert_eq!(lookup_clause_name("private"), ClauseName::Private);
        assert_eq!(lookup_clause_name("Private"), ClauseName::Private);
        assert_eq!(lookup_clause_name("  shared  "), ClauseName::Shared);
    }

    #[test]
    fn lookup_clause_name_synonyms() {
        // OpenACC synonyms should map to the dedicated ClauseName variants we added
        assert_eq!(lookup_clause_name("pcopy"), ClauseName::Copy);
        assert_eq!(lookup_clause_name("present_or_create"), ClauseName::Create);
    }

    fn parse_single_identifier<'a>(
        name: Cow<'a, str>,
        input: &'a str,
    ) -> IResult<&'a str, Clause<'a>> {
        let (input, _) = char('(')(input)?;
        let (input, identifier) = lexer::lex_clause(input)?;
        let (input, _) = char(')')(input)?;

        Ok((
            input,
            Clause {
                name,
                kind: ClauseKind::Parenthesized(identifier.into()),
            },
        ))
    }

    #[test]
    fn supports_custom_clause_rule() {
        let registry = ClauseRegistry::builder()
            .register_custom("device", parse_single_identifier)
            .build();

        let (rest, clauses) = registry
            .parse_sequence("device(gpu)")
            .expect("parsing should succeed");

        assert_eq!(rest, "");
        assert_eq!(clauses.len(), 1);
        assert_eq!(clauses[0].name, "device");
        assert_eq!(clauses[0].kind, ClauseKind::Parenthesized("gpu".into()));
    }

    #[test]
    fn rejects_unregistered_clause_when_default_is_unsupported() {
        let registry = ClauseRegistry::builder()
            .with_default_rule(ClauseRule::Unsupported)
            .register_bare("nowait")
            .build();

        let result = registry.parse_sequence("unknown");

        assert!(result.is_err());
    }
}
