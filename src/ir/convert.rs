//! Conversion from parser types to IR types
//!
//! This module handles the conversion from the parser's string-based
//! representation to the IR's semantic representation.
//!
//! ## Learning Objectives
//!
//! - **Pattern matching on strings**: Mapping clause names to semantic types
//! - **Error handling**: Using Result for fallible conversions
//! - **Parsing clause data**: Extracting semantic meaning from strings
//! - **Gradual refinement**: Starting simple, adding complexity incrementally
//!
//! ## Conversion Strategy
//!
//! The parser gives us:
//! - Directive name as a string (e.g., "parallel for")
//! - Clauses with names and optional content strings
//!
//! We need to convert this to IR which has:
//! - DirectiveKind enum
//! - ClauseData with structured semantic information
//!
//! ## Example
//!
//! ```text
//! Parser output:
//!   Directive { name: "parallel for",
//!               clauses: [
//!                 Clause { name: "private", kind: Parenthesized("x, y") },
//!                 Clause { name: "reduction".into(), kind: Parenthesized("+: sum") }
//!               ] }
//!
//! IR output:
//!   DirectiveIR {
//!     kind: DirectiveKind::ParallelFor,
//!     clauses: [
//!       ClauseData::Private { items: [Identifier("x"), Identifier("y")] },
//!       ClauseData::Reduction { operator: Add, items: [Identifier("sum")] }
//!     ],
//!     ...
//!   }
//! ```

use super::{
    lang, ClauseData, ClauseItem, ConversionError, DefaultKind, DependType, DirectiveIR,
    DirectiveKind, Expression, Identifier, Language, MapType, ParserConfig, ProcBind,
    ReductionOperator, ScheduleKind, ScheduleModifier, SourceLocation,
};
use crate::parser::{Clause, ClauseKind, Directive};

/// Convert a directive name string to DirectiveKind
///
/// ## Example
///
/// ```
/// # use roup::ir::{DirectiveKind, convert::parse_directive_kind};
/// let kind = parse_directive_kind("parallel for").unwrap();
/// assert_eq!(kind, DirectiveKind::ParallelFor);
///
/// let kind = parse_directive_kind("target teams distribute").unwrap();
/// assert_eq!(kind, DirectiveKind::TargetTeamsDistribute);
/// ```
pub fn parse_directive_kind(name: &str) -> Result<DirectiveKind, ConversionError> {
    // Normalize whitespace for matching
    let normalized = name.trim().to_lowercase();
    let normalized = normalized.as_str();

    match normalized {
        // Parallel constructs
        "parallel" => Ok(DirectiveKind::Parallel),
        "parallel for" => Ok(DirectiveKind::ParallelFor),
        "parallel do" => Ok(DirectiveKind::ParallelDo), // Fortran equivalent
        "parallel for simd" => Ok(DirectiveKind::ParallelForSimd),
        "parallel do simd" => Ok(DirectiveKind::ParallelDoSimd), // Fortran equivalent
        "parallel sections" => Ok(DirectiveKind::ParallelSections),
        "parallel workshare" => Ok(DirectiveKind::ParallelWorkshare),
        "parallel loop" => Ok(DirectiveKind::ParallelLoop),
        "parallel loop simd" => Ok(DirectiveKind::ParallelLoopSimd),
        "parallel masked" => Ok(DirectiveKind::ParallelMasked),
        "parallel master" => Ok(DirectiveKind::ParallelMaster),
        "parallel masked taskloop" => Ok(DirectiveKind::ParallelMaskedTaskloop),
        "parallel masked taskloop simd" => Ok(DirectiveKind::ParallelMaskedTaskloopSimd),
        "parallel master taskloop" => Ok(DirectiveKind::ParallelMasterTaskloop),
        "parallel master taskloop simd" => Ok(DirectiveKind::ParallelMasterTaskloopSimd),

        // Work-sharing constructs
        "for" => Ok(DirectiveKind::For),
        "do" => Ok(DirectiveKind::Do), // Fortran equivalent
        "for simd" => Ok(DirectiveKind::ForSimd),
        "do simd" => Ok(DirectiveKind::DoSimd), // Fortran equivalent
        "sections" => Ok(DirectiveKind::Sections),
        "section" => Ok(DirectiveKind::Section),
        "single" => Ok(DirectiveKind::Single),
        "workshare" => Ok(DirectiveKind::Workshare),
        "loop" => Ok(DirectiveKind::Loop),

        // SIMD constructs
        "simd" => Ok(DirectiveKind::Simd),
        "declare simd" => Ok(DirectiveKind::DeclareSimd),

        // Task constructs
        "task" => Ok(DirectiveKind::Task),
        "taskloop" => Ok(DirectiveKind::Taskloop),
        "taskloop simd" => Ok(DirectiveKind::TaskloopSimd),
        "masked taskloop" => Ok(DirectiveKind::MaskedTaskloop),
        "masked taskloop simd" => Ok(DirectiveKind::MaskedTaskloopSimd),
        "taskyield" => Ok(DirectiveKind::Taskyield),
        "taskwait" => Ok(DirectiveKind::Taskwait),
        "taskgroup" => Ok(DirectiveKind::Taskgroup),
        "taskgraph" => Ok(DirectiveKind::Taskgraph),
        "task iteration" => Ok(DirectiveKind::TaskIteration),

        // Target constructs
        "target" => Ok(DirectiveKind::Target),
        "target data" => Ok(DirectiveKind::TargetData),
        "target enter data" => Ok(DirectiveKind::TargetEnterData),
        "target exit data" => Ok(DirectiveKind::TargetExitData),
        "target update" => Ok(DirectiveKind::TargetUpdate),
        "end target" => Ok(DirectiveKind::EndTarget),
        "target parallel" => Ok(DirectiveKind::TargetParallel),
        "target parallel for" => Ok(DirectiveKind::TargetParallelFor),
        "target parallel do" => Ok(DirectiveKind::TargetParallelDo), // Fortran equivalent
        "target parallel for simd" => Ok(DirectiveKind::TargetParallelForSimd),
        "target parallel do simd" => Ok(DirectiveKind::TargetParallelDoSimd), // Fortran equivalent
        "target parallel loop" => Ok(DirectiveKind::TargetParallelLoop),
        "target parallel loop simd" => Ok(DirectiveKind::TargetParallelLoopSimd),
        "target simd" => Ok(DirectiveKind::TargetSimd),
        "target loop" => Ok(DirectiveKind::TargetLoop),
        "target loop simd" => Ok(DirectiveKind::TargetLoopSimd),
        "target teams" => Ok(DirectiveKind::TargetTeams),
        "target teams distribute" => Ok(DirectiveKind::TargetTeamsDistribute),
        "target teams distribute simd" => Ok(DirectiveKind::TargetTeamsDistributeSimd),
        "target teams distribute parallel for" => {
            Ok(DirectiveKind::TargetTeamsDistributeParallelFor)
        }
        "target teams distribute parallel do" => {
            Ok(DirectiveKind::TargetTeamsDistributeParallelDo) // Fortran equivalent
        }
        "target teams distribute parallel for simd" => {
            Ok(DirectiveKind::TargetTeamsDistributeParallelForSimd)
        }
        "target teams distribute parallel do simd" => {
            Ok(DirectiveKind::TargetTeamsDistributeParallelDoSimd) // Fortran equivalent
        }
        "target teams distribute parallel loop" => {
            Ok(DirectiveKind::TargetTeamsDistributeParallelLoop)
        }
        "target teams distribute parallel loop simd" => {
            Ok(DirectiveKind::TargetTeamsDistributeParallelLoopSimd)
        }
        "target teams loop" => Ok(DirectiveKind::TargetTeamsLoop),
        "target teams loop simd" => Ok(DirectiveKind::TargetTeamsLoopSimd),

        // Teams constructs
        "teams" => Ok(DirectiveKind::Teams),
        "teams distribute" => Ok(DirectiveKind::TeamsDistribute),
        "teams distribute simd" => Ok(DirectiveKind::TeamsDistributeSimd),
        "teams distribute parallel for" => Ok(DirectiveKind::TeamsDistributeParallelFor),
        "teams distribute parallel do" => Ok(DirectiveKind::TeamsDistributeParallelDo), // Fortran equivalent
        "teams distribute parallel for simd" => Ok(DirectiveKind::TeamsDistributeParallelForSimd),
        "teams distribute parallel do simd" => Ok(DirectiveKind::TeamsDistributeParallelDoSimd), // Fortran equivalent
        "teams distribute parallel loop" => Ok(DirectiveKind::TeamsDistributeParallelLoop),
        "teams distribute parallel loop simd" => Ok(DirectiveKind::TeamsDistributeParallelLoopSimd),
        "teams loop" => Ok(DirectiveKind::TeamsLoop),
        "teams loop simd" => Ok(DirectiveKind::TeamsLoopSimd),

        // Synchronization constructs
        "barrier" => Ok(DirectiveKind::Barrier),
        "critical" => Ok(DirectiveKind::Critical),
        "atomic" => Ok(DirectiveKind::Atomic),
        "atomic read" => Ok(DirectiveKind::AtomicRead),
        "atomic write" => Ok(DirectiveKind::AtomicWrite),
        "atomic update" => Ok(DirectiveKind::AtomicUpdate),
        "atomic capture" => Ok(DirectiveKind::AtomicCapture),
        "atomic compare capture" => Ok(DirectiveKind::AtomicCompareCapture),
        "flush" => Ok(DirectiveKind::Flush),
        "ordered" => Ok(DirectiveKind::Ordered),
        "master" => Ok(DirectiveKind::Master),
        "masked" => Ok(DirectiveKind::Masked),

        // Declare constructs
        "declare reduction" => Ok(DirectiveKind::DeclareReduction),
        "declare mapper" => Ok(DirectiveKind::DeclareMapper),
        "declare target" => Ok(DirectiveKind::DeclareTarget),
        "begin declare target" => Ok(DirectiveKind::BeginDeclareTarget),
        "end declare target" => Ok(DirectiveKind::EndDeclareTarget),
        "declare variant" => Ok(DirectiveKind::DeclareVariant),
        "begin declare variant" => Ok(DirectiveKind::BeginDeclareVariant),
        "end declare variant" => Ok(DirectiveKind::EndDeclareVariant),
        "declare induction" => Ok(DirectiveKind::DeclareInduction),

        // Distribute constructs
        "distribute" => Ok(DirectiveKind::Distribute),
        "distribute simd" => Ok(DirectiveKind::DistributeSimd),
        "distribute parallel for" => Ok(DirectiveKind::DistributeParallelFor),
        "distribute parallel do" => Ok(DirectiveKind::DistributeParallelDo), // Fortran equivalent
        "distribute parallel for simd" => Ok(DirectiveKind::DistributeParallelForSimd),
        "distribute parallel do simd" => Ok(DirectiveKind::DistributeParallelDoSimd), // Fortran equivalent
        "distribute parallel loop" => Ok(DirectiveKind::DistributeParallelLoop),
        "distribute parallel loop simd" => Ok(DirectiveKind::DistributeParallelLoopSimd),

        // Meta-directives
        "metadirective" => Ok(DirectiveKind::Metadirective),
        "begin metadirective" => Ok(DirectiveKind::BeginMetadirective),
        "assume" => Ok(DirectiveKind::Assume),
        "assumes" => Ok(DirectiveKind::Assumes),
        "begin assumes" => Ok(DirectiveKind::BeginAssumes),

        // Loop transformations
        "tile" => Ok(DirectiveKind::Tile),
        "unroll" => Ok(DirectiveKind::Unroll),
        "fuse" => Ok(DirectiveKind::Fuse),
        "split" => Ok(DirectiveKind::Split),
        "interchange" => Ok(DirectiveKind::Interchange),
        "reverse" => Ok(DirectiveKind::Reverse),
        "stripe" => Ok(DirectiveKind::Stripe),

        // Other constructs
        "threadprivate" => Ok(DirectiveKind::Threadprivate),
        "allocate" => Ok(DirectiveKind::Allocate),
        "allocators" => Ok(DirectiveKind::Allocators),
        "requires" => Ok(DirectiveKind::Requires),
        "scan" => Ok(DirectiveKind::Scan),
        "depobj" => Ok(DirectiveKind::Depobj),
        "nothing" => Ok(DirectiveKind::Nothing),
        "error" => Ok(DirectiveKind::Error),
        "cancel" => Ok(DirectiveKind::Cancel),
        "cancellation point" => Ok(DirectiveKind::CancellationPoint),
        "dispatch" => Ok(DirectiveKind::Dispatch),
        "interop" => Ok(DirectiveKind::Interop),
        "scope" => Ok(DirectiveKind::Scope),
        "groupprivate" => Ok(DirectiveKind::Groupprivate),
        "workdistribute" => Ok(DirectiveKind::Workdistribute),

        _ => Err(ConversionError::UnknownDirective(name.to_string())),
    }
}

/// Parse a clause item list using the configured language front-end.
///
/// Used for clauses like `private(x, y, z)` or `map(to: arr[0:N])` where the
/// payload needs to be interpreted according to the host language.
pub fn parse_identifier_list(
    content: &str,
    config: &ParserConfig,
) -> Result<Vec<ClauseItem>, ConversionError> {
    lang::parse_clause_item_list(content, config)
}

/// Parse a reduction operator from a string
///
/// ## Example
///
/// ```
/// # use roup::ir::{convert::parse_reduction_operator, ReductionOperator};
/// let op = parse_reduction_operator("+").unwrap();
/// assert_eq!(op, ReductionOperator::Add);
///
/// let op = parse_reduction_operator("min").unwrap();
/// assert_eq!(op, ReductionOperator::Min);
/// ```
pub fn parse_reduction_operator(op_str: &str) -> Result<ReductionOperator, ConversionError> {
    match op_str {
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
        _ => Err(ConversionError::InvalidClauseSyntax(format!(
            "Unknown reduction operator: {op_str}"
        ))),
    }
}

/// Parse a schedule clause
///
/// Format: `schedule([modifier[, modifier]:] kind[, chunk_size])`
///
/// ## Example
///
/// ```
/// # use roup::ir::{convert::parse_schedule_clause, ParserConfig, Language};
/// let config = ParserConfig::with_parsing(Language::C);
/// let clause = parse_schedule_clause("static, 10", &config).unwrap();
/// // Returns ClauseData::Schedule with kind=Static, chunk_size=Some(10)
/// ```
pub fn parse_schedule_clause(
    content: &str,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    // Check for modifiers (they end with a colon)
    let (modifiers, rest) = if let Some(colon_pos) = content.find(':') {
        let (mod_str, kind_str) = content.split_at(colon_pos);
        let kind_str = kind_str[1..].trim(); // Skip the ':'

        // Parse modifiers (comma-separated)
        let mods: Vec<ScheduleModifier> = mod_str
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| match s {
                "monotonic" => Ok(ScheduleModifier::Monotonic),
                "nonmonotonic" => Ok(ScheduleModifier::Nonmonotonic),
                "simd" => Ok(ScheduleModifier::Simd),
                _ => Err(ConversionError::InvalidClauseSyntax(format!(
                    "Unknown schedule modifier: {s}"
                ))),
            })
            .collect::<Result<Vec<_>, _>>()?;

        (mods, kind_str)
    } else {
        (vec![], content)
    };

    // Parse kind and optional chunk size (comma-separated)
    let parts: Vec<&str> = rest.split(',').map(|s| s.trim()).collect();

    let kind = match parts.first() {
        Some(&"static") => ScheduleKind::Static,
        Some(&"dynamic") => ScheduleKind::Dynamic,
        Some(&"guided") => ScheduleKind::Guided,
        Some(&"auto") => ScheduleKind::Auto,
        Some(&"runtime") => ScheduleKind::Runtime,
        Some(s) => {
            return Err(ConversionError::InvalidClauseSyntax(format!(
                "Unknown schedule kind: {s}"
            )))
        }
        None => {
            return Err(ConversionError::InvalidClauseSyntax(
                "schedule clause requires a kind".to_string(),
            ))
        }
    };

    let chunk_size = parts.get(1).map(|s| Expression::new(*s, config));

    Ok(ClauseData::Schedule {
        kind,
        modifiers,
        chunk_size,
    })
}

/// Parse a map clause
///
/// Format: `map([[mapper(mapper-identifier),] map-type:] list)`
///
/// Supports mapper syntax and respects nesting when finding colons.
///
/// ## Example
///
/// ```
/// # use roup::ir::{convert::parse_map_clause, ParserConfig, Language};
/// let config = ParserConfig::with_parsing(Language::C);
/// let clause = parse_map_clause("to: arr", &config).unwrap();
/// // Returns ClauseData::Map with map_type=To, items=[arr]
///
/// let clause = parse_map_clause("mapper(custom), to: arr[0:N]", &config).unwrap();
/// // Returns ClauseData::Map with mapper=Some(custom), map_type=To, items=[arr[0:N]]
/// ```
pub fn parse_map_clause(
    content: &str,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    let mut remainder = content.trim();
    let mut mapper = None;

    // Check for mapper(...) prefix
    if remainder.len() >= 6 && remainder[..6].eq_ignore_ascii_case("mapper") {
        let after_keyword = remainder[6..].trim_start();
        if after_keyword.starts_with('(') {
            // Extract mapper identifier
            let (mapper_body, rest) = extract_parenthesized(after_keyword)?;
            mapper = Some(Identifier::new(mapper_body.trim()));
            remainder = rest.trim_start();

            // Skip optional comma
            if remainder.starts_with(',') {
                remainder = remainder[1..].trim_start();
            }
        }
    }

    // Find map-type using top-level colon detection
    let (map_type, items_str) =
        if let Some((type_str, items)) = lang::split_once_top_level(remainder, ':') {
            let map_type = match type_str.trim().to_ascii_lowercase().as_str() {
                "" => None,
                "to" => Some(MapType::To),
                "from" => Some(MapType::From),
                "tofrom" => Some(MapType::ToFrom),
                "alloc" => Some(MapType::Alloc),
                "release" => Some(MapType::Release),
                "delete" => Some(MapType::Delete),
                other => {
                    return Err(ConversionError::InvalidClauseSyntax(format!(
                        "Unknown map type: {other}"
                    )))
                }
            };
            (map_type, items.trim())
        } else {
            (None, remainder)
        };

    let items = parse_identifier_list(items_str, config)?;

    Ok(ClauseData::Map {
        map_type,
        mapper,
        items,
    })
}

/// Extract content from parentheses, handling nesting.
///
/// Returns (content, remainder) where content is what's inside the first set of parens.
///
/// This is a thin wrapper around the lang module's bracket extraction helper.
fn extract_parenthesized(input: &str) -> Result<(&str, &str), ConversionError> {
    // Delegate to the lang module's generic bracket extraction
    lang::extract_bracket_content(input, '(', ')')
}

/// Parse a dependence type from a string
///
/// ## Example
///
/// ```
/// # use roup::ir::{convert::parse_depend_type, DependType};
/// let dt = parse_depend_type("in").unwrap();
/// assert_eq!(dt, DependType::In);
/// ```
pub fn parse_depend_type(type_str: &str) -> Result<DependType, ConversionError> {
    match type_str {
        "in" => Ok(DependType::In),
        "out" => Ok(DependType::Out),
        "inout" => Ok(DependType::Inout),
        "mutexinoutset" => Ok(DependType::Mutexinoutset),
        "depobj" => Ok(DependType::Depobj),
        "source" => Ok(DependType::Source),
        "sink" => Ok(DependType::Sink),
        _ => Err(ConversionError::InvalidClauseSyntax(format!(
            "Unknown depend type: {type_str}"
        ))),
    }
}

/// Parse a linear clause
///
/// Format: `linear([modifier(list):] list[:step])`
///
/// Uses top-level colon detection to properly handle nested structures.
///
/// ## Example
///
/// ```
/// # use roup::ir::{convert::parse_linear_clause, ParserConfig, Language};
/// let config = ParserConfig::with_parsing(Language::C);
/// let clause = parse_linear_clause("x, y: 2", &config).unwrap();
/// // Returns ClauseData::Linear with items=[x, y], step=Some(2)
/// ```
pub fn parse_linear_clause(
    content: &str,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    // Use rsplit_once_top_level to find the last colon at top level
    if let Some((items_str, step_str)) = lang::rsplit_once_top_level(content, ':') {
        // Check if this might be modifier syntax (has opening paren before colon)
        if items_str.contains('(') {
            return Err(ConversionError::Unsupported(
                "linear clause with modifiers not yet supported".to_string(),
            ));
        }

        let items = parse_identifier_list(items_str, config)?;
        let step = Some(Expression::new(step_str.trim(), config));

        Ok(ClauseData::Linear {
            modifier: None,
            items,
            step,
        })
    } else {
        // No step, just items
        let items = parse_identifier_list(content, config)?;
        Ok(ClauseData::Linear {
            modifier: None,
            items,
            step: None,
        })
    }
}

/// Convert a parser Clause to IR ClauseData
///
/// This is the main conversion function that handles all clause types.
///
/// ## Strategy
///
/// For now, we'll implement a subset of clauses and mark others as
/// unsupported. This allows incremental development.
pub fn parse_clause_data<'a>(
    clause: &'a Clause<'a>,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    let clause_name = clause.name.as_ref();

    match clause_name {
        // Bare clauses (no parameters)
        "nowait" | "nogroup" | "untied" | "mergeable" | "seq_cst" | "relaxed" | "release"
        | "acquire" | "acq_rel" => Ok(ClauseData::Bare(Identifier::new(clause_name))),

        // default(kind)
        "default" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref();
                let kind_str = content.trim();
                let kind = match kind_str {
                    "shared" => DefaultKind::Shared,
                    "none" => DefaultKind::None,
                    "private" => DefaultKind::Private,
                    "firstprivate" => DefaultKind::Firstprivate,
                    _ => {
                        return Err(ConversionError::InvalidClauseSyntax(format!(
                            "Unknown default kind: {kind_str}"
                        )))
                    }
                };
                Ok(ClauseData::Default(kind))
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "default clause requires parenthesized content".to_string(),
                ))
            }
        }

        // private(list)
        "private" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref();
                let items = parse_identifier_list(content, config)?;
                Ok(ClauseData::Private { items })
            } else {
                Ok(ClauseData::Private { items: vec![] })
            }
        }

        // firstprivate(list)
        "firstprivate" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref();
                let items = parse_identifier_list(content, config)?;
                Ok(ClauseData::Firstprivate { items })
            } else {
                Ok(ClauseData::Firstprivate { items: vec![] })
            }
        }

        // shared(list)
        "shared" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref();
                let items = parse_identifier_list(content, config)?;
                Ok(ClauseData::Shared { items })
            } else {
                Ok(ClauseData::Shared { items: vec![] })
            }
        }

        // num_threads(expr)
        "num_threads" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref();
                Ok(ClauseData::NumThreads {
                    num: Expression::new(content.trim(), config),
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "num_threads requires expression".to_string(),
                ))
            }
        }

        // if(expr)
        "if" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref();
                // Check for directive-name modifier: "if(parallel: condition)"
                if let Some((modifier, condition)) = lang::split_once_top_level(content, ':') {
                    Ok(ClauseData::If {
                        directive_name: Some(Identifier::new(modifier.trim())),
                        condition: Expression::new(condition.trim(), config),
                    })
                } else {
                    Ok(ClauseData::If {
                        directive_name: None,
                        condition: Expression::new(content.trim(), config),
                    })
                }
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "if clause requires parenthesized content".to_string(),
                ))
            }
        }

        // collapse(n)
        "collapse" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref();
                Ok(ClauseData::Collapse {
                    n: Expression::new(content.trim(), config),
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "collapse requires expression".to_string(),
                ))
            }
        }

        // ordered or ordered(n)
        "ordered" => match clause.kind {
            ClauseKind::Bare => Ok(ClauseData::Ordered { n: None }),
            ClauseKind::Parenthesized(ref content) => Ok(ClauseData::Ordered {
                n: Some(Expression::new(content.as_ref().trim(), config)),
            }),
            // OpenACC-specific structured clauses should not appear in OpenMP context
            _ => Err(ConversionError::InvalidClauseSyntax(
                "Unexpected structured clause for 'ordered'".to_string(),
            )),
        },

        // reduction(operator: list)
        "reduction" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref();
                // Find the colon separator between operator and list
                if let Some((op_str, items_str)) = lang::split_once_top_level(content, ':') {
                    // Parse the operator
                    let operator = parse_reduction_operator(op_str.trim())?;

                    // Parse the item list
                    let items = parse_identifier_list(items_str.trim(), config)?;

                    Ok(ClauseData::Reduction { operator, items })
                } else {
                    Err(ConversionError::InvalidClauseSyntax(
                        "reduction clause requires 'operator: list' format".to_string(),
                    ))
                }
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "reduction clause requires parenthesized content".to_string(),
                ))
            }
        }

        // schedule([modifier[, modifier]:] kind[, chunk_size])
        "schedule" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref();
                parse_schedule_clause(content, config)
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "schedule clause requires parenthesized content".to_string(),
                ))
            }
        }

        // map([[mapper(mapper-identifier),] map-type:] list)
        "map" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref();
                parse_map_clause(content, config)
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "map clause requires parenthesized content".to_string(),
                ))
            }
        }

        // depend(dependence-type: list)
        "depend" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref();
                // Find the colon separator using top-level detection
                if let Some((type_str, items_str)) = lang::split_once_top_level(content, ':') {
                    // Parse the dependence type
                    let depend_type = parse_depend_type(type_str.trim())?;

                    // Parse the item list
                    let items = parse_identifier_list(items_str.trim(), config)?;

                    Ok(ClauseData::Depend { depend_type, items })
                } else {
                    Err(ConversionError::InvalidClauseSyntax(
                        "depend clause requires 'type: list' format".to_string(),
                    ))
                }
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "depend clause requires parenthesized content".to_string(),
                ))
            }
        }

        // linear([modifier(list):] list[:step])
        "linear" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref();
                parse_linear_clause(content, config)
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "linear clause requires parenthesized content".to_string(),
                ))
            }
        }

        // proc_bind(master|close|spread|primary)
        "proc_bind" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref();
                let kind_str = content.trim();
                let proc_bind = match kind_str {
                    "master" => ProcBind::Master,
                    "close" => ProcBind::Close,
                    "spread" => ProcBind::Spread,
                    "primary" => ProcBind::Primary,
                    _ => {
                        return Err(ConversionError::InvalidClauseSyntax(format!(
                            "Unknown proc_bind kind: {kind_str}"
                        )))
                    }
                };
                Ok(ClauseData::ProcBind(proc_bind))
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "proc_bind clause requires parenthesized content".to_string(),
                ))
            }
        }

        // For unsupported clauses, return a generic representation
        _ => Ok(ClauseData::Generic {
            name: Identifier::new(clause_name),
            data: match &clause.kind {
                ClauseKind::Bare => None,
                ClauseKind::Parenthesized(ref content) => Some(content.as_ref().to_string()),
                // For structured OpenACC clauses, use Display trait to convert to string
                _ => Some(clause.to_string()),
            },
        }),
    }
}

/// Convert a parser Directive to IR DirectiveIR
///
/// ## Example
///
/// ```
/// # use roup::parser::{Directive, Clause, ClauseKind};
/// # use roup::ir::{convert::convert_directive, Language, SourceLocation, ParserConfig};
/// let directive = Directive {
///     name: "parallel".into(),
///     parameter: None,
///     clauses: vec![
///         Clause {
///             name: "default".into(),
///             kind: ClauseKind::Parenthesized("shared".into()),
///         },
///     ],
///     cache_data: None,
///     wait_data: None,
/// };
///
/// let config = ParserConfig::default();
/// let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config).unwrap();
/// assert!(ir.kind().is_parallel());
/// ```
pub fn convert_directive<'a>(
    directive: &'a Directive<'a>,
    location: SourceLocation,
    language: Language,
    config: &ParserConfig,
) -> Result<DirectiveIR, ConversionError> {
    // SAFETY FIX: Clone the directive name from Cow to owned String
    // This prevents use-after-free when Cow::Owned is dropped.
    // The normalized name (from line continuations) is now owned by DirectiveIR.
    let directive_name = directive.name.to_string();

    // Convert directive kind
    let kind = parse_directive_kind(&directive_name)?;

    // Convert clauses
    let mut clauses = Vec::new();
    let clause_config = config.for_language(language);
    for clause in &directive.clauses {
        let clause_data = parse_clause_data(clause, &clause_config)?;
        clauses.push(clause_data);
    }

    Ok(DirectiveIR::new(
        kind,
        &directive_name,
        clauses,
        location,
        language,
    ))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_directive_kind_parallel() {
        assert_eq!(
            parse_directive_kind("parallel").unwrap(),
            DirectiveKind::Parallel
        );
        assert_eq!(
            parse_directive_kind("parallel for").unwrap(),
            DirectiveKind::ParallelFor
        );
    }

    #[test]
    fn test_parse_directive_kind_case_insensitive() {
        assert_eq!(
            parse_directive_kind("PARALLEL").unwrap(),
            DirectiveKind::Parallel
        );
        assert_eq!(
            parse_directive_kind("Parallel For").unwrap(),
            DirectiveKind::ParallelFor
        );
    }

    #[test]
    fn test_parse_directive_kind_whitespace() {
        assert_eq!(
            parse_directive_kind("  parallel  ").unwrap(),
            DirectiveKind::Parallel
        );
    }

    #[test]
    fn test_parse_directive_kind_unknown() {
        assert!(parse_directive_kind("unknown_directive").is_err());
    }

    #[test]
    fn test_parse_identifier_list_single() {
        let config = ParserConfig::with_parsing(Language::C);
        let items = parse_identifier_list("x", &config).unwrap();
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_parse_identifier_list_multiple() {
        let config = ParserConfig::with_parsing(Language::C);
        let items = parse_identifier_list("x, y, z", &config).unwrap();
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn test_parse_identifier_list_with_spaces() {
        let config = ParserConfig::with_parsing(Language::C);
        let items = parse_identifier_list("  x  ,  y  ,  z  ", &config).unwrap();
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn test_parse_identifier_list_empty() {
        let config = ParserConfig::with_parsing(Language::C);
        let items = parse_identifier_list("", &config).unwrap();
        assert_eq!(items.len(), 0);
    }

    #[test]
    fn test_parse_clause_data_bare() {
        let clause = Clause {
            name: "nowait".into(),
            kind: ClauseKind::Bare,
        };
        let config = ParserConfig::default();
        let data = parse_clause_data(&clause, &config).unwrap();
        assert!(matches!(data, ClauseData::Bare(_)));
        assert_eq!(data.to_string(), "nowait");
    }

    #[test]
    fn test_parse_clause_data_default_shared() {
        let clause = Clause {
            name: "default".into(),
            kind: ClauseKind::Parenthesized("shared".into()),
        };
        let config = ParserConfig::default();
        let data = parse_clause_data(&clause, &config).unwrap();
        assert_eq!(data, ClauseData::Default(DefaultKind::Shared));
    }

    #[test]
    fn test_parse_clause_data_private() {
        let clause = Clause {
            name: "private".into(),
            kind: ClauseKind::Parenthesized("x, y".into()),
        };
        let config = ParserConfig::default();
        let data = parse_clause_data(&clause, &config).unwrap();
        if let ClauseData::Private { items } = data {
            assert_eq!(items.len(), 2);
        } else {
            panic!("Expected Private clause");
        }
    }

    #[test]
    fn test_parse_clause_data_num_threads() {
        let clause = Clause {
            name: "num_threads".into(),
            kind: ClauseKind::Parenthesized("4".into()),
        };
        let config = ParserConfig::default();
        let data = parse_clause_data(&clause, &config).unwrap();
        assert!(matches!(data, ClauseData::NumThreads { .. }));
    }

    #[test]
    fn test_parse_clause_data_if_simple() {
        let clause = Clause {
            name: "if".into(),
            kind: ClauseKind::Parenthesized("n > 100".into()),
        };
        let config = ParserConfig::default();
        let data = parse_clause_data(&clause, &config).unwrap();
        if let ClauseData::If {
            directive_name,
            condition,
        } = data
        {
            assert!(directive_name.is_none());
            assert_eq!(condition.to_string(), "n > 100");
        } else {
            panic!("Expected If clause");
        }
    }

    #[test]
    fn test_parse_clause_data_if_with_modifier() {
        let clause = Clause {
            name: "if".into(),
            kind: ClauseKind::Parenthesized("parallel: n > 100".into()),
        };
        let config = ParserConfig::default();
        let data = parse_clause_data(&clause, &config).unwrap();
        if let ClauseData::If {
            directive_name,
            condition,
        } = data
        {
            assert!(directive_name.is_some());
            assert_eq!(directive_name.unwrap().to_string(), "parallel");
            assert_eq!(condition.to_string(), "n > 100");
        } else {
            panic!("Expected If clause");
        }
    }

    #[test]
    fn test_convert_directive_simple() {
        let directive = Directive {
            name: "parallel".into(),
            parameter: None,
            clauses: vec![],
            wait_data: None,
            cache_data: None,
        };
        let config = ParserConfig::default();
        let ir =
            convert_directive(&directive, SourceLocation::start(), Language::C, &config).unwrap();
        assert_eq!(ir.kind(), DirectiveKind::Parallel);
        assert_eq!(ir.clauses().len(), 0);
    }

    #[test]
    fn test_convert_directive_with_clauses() {
        let directive = Directive {
            name: "parallel".into(),
            parameter: None,
            clauses: vec![
                Clause {
                    name: "default".into(),
                    kind: ClauseKind::Parenthesized("shared".into()),
                },
                Clause {
                    name: "private".into(),
                    kind: ClauseKind::Parenthesized("x".into()),
                },
            ],
            wait_data: None,
            cache_data: None,
        };
        let config = ParserConfig::default();
        let ir =
            convert_directive(&directive, SourceLocation::start(), Language::C, &config).unwrap();
        assert_eq!(ir.kind(), DirectiveKind::Parallel);
        assert_eq!(ir.clauses().len(), 2);
    }

    // Tests for reduction operator parsing
    #[test]
    fn test_parse_reduction_operator_arithmetic() {
        assert_eq!(
            parse_reduction_operator("+").unwrap(),
            ReductionOperator::Add
        );
        assert_eq!(
            parse_reduction_operator("-").unwrap(),
            ReductionOperator::Subtract
        );
        assert_eq!(
            parse_reduction_operator("*").unwrap(),
            ReductionOperator::Multiply
        );
    }

    #[test]
    fn test_parse_reduction_operator_bitwise() {
        assert_eq!(
            parse_reduction_operator("&").unwrap(),
            ReductionOperator::BitwiseAnd
        );
        assert_eq!(
            parse_reduction_operator("|").unwrap(),
            ReductionOperator::BitwiseOr
        );
        assert_eq!(
            parse_reduction_operator("^").unwrap(),
            ReductionOperator::BitwiseXor
        );
    }

    #[test]
    fn test_parse_reduction_operator_logical() {
        assert_eq!(
            parse_reduction_operator("&&").unwrap(),
            ReductionOperator::LogicalAnd
        );
        assert_eq!(
            parse_reduction_operator("||").unwrap(),
            ReductionOperator::LogicalOr
        );
    }

    #[test]
    fn test_parse_reduction_operator_minmax() {
        assert_eq!(
            parse_reduction_operator("min").unwrap(),
            ReductionOperator::Min
        );
        assert_eq!(
            parse_reduction_operator("max").unwrap(),
            ReductionOperator::Max
        );
    }

    #[test]
    fn test_parse_reduction_operator_unknown() {
        assert!(parse_reduction_operator("unknown").is_err());
    }

    // Tests for reduction clause
    #[test]
    fn test_parse_clause_data_reduction() {
        let clause = Clause {
            name: "reduction".into(),
            kind: ClauseKind::Parenthesized("+: sum".into()),
        };
        let config = ParserConfig::default();
        let data = parse_clause_data(&clause, &config).unwrap();
        if let ClauseData::Reduction { operator, items } = data {
            assert_eq!(operator, ReductionOperator::Add);
            assert_eq!(items.len(), 1);
        } else {
            panic!("Expected Reduction clause");
        }
    }

    #[test]
    fn test_parse_clause_data_reduction_multiple_items() {
        let clause = Clause {
            name: "reduction".into(),
            kind: ClauseKind::Parenthesized("*: a, b, c".into()),
        };
        let config = ParserConfig::default();
        let data = parse_clause_data(&clause, &config).unwrap();
        if let ClauseData::Reduction { operator, items } = data {
            assert_eq!(operator, ReductionOperator::Multiply);
            assert_eq!(items.len(), 3);
        } else {
            panic!("Expected Reduction clause");
        }
    }

    #[test]
    fn test_parse_clause_data_reduction_minmax() {
        let clause = Clause {
            name: "reduction".into(),
            kind: ClauseKind::Parenthesized("min: value".into()),
        };
        let config = ParserConfig::default();
        let data = parse_clause_data(&clause, &config).unwrap();
        if let ClauseData::Reduction { operator, items } = data {
            assert_eq!(operator, ReductionOperator::Min);
            assert_eq!(items.len(), 1);
        } else {
            panic!("Expected Reduction clause");
        }
    }

    // Tests for schedule clause
    #[test]
    fn test_parse_schedule_clause_static() {
        let config = ParserConfig::with_parsing(Language::C);
        let data = parse_schedule_clause("static", &config).unwrap();
        if let ClauseData::Schedule {
            kind,
            modifiers,
            chunk_size,
        } = data
        {
            assert_eq!(kind, ScheduleKind::Static);
            assert!(modifiers.is_empty());
            assert!(chunk_size.is_none());
        } else {
            panic!("Expected Schedule clause");
        }
    }

    #[test]
    fn test_parse_schedule_clause_with_chunk() {
        let config = ParserConfig::with_parsing(Language::C);
        let data = parse_schedule_clause("dynamic, 10", &config).unwrap();
        if let ClauseData::Schedule {
            kind,
            modifiers,
            chunk_size,
        } = data
        {
            assert_eq!(kind, ScheduleKind::Dynamic);
            assert!(modifiers.is_empty());
            assert!(chunk_size.is_some());
            assert_eq!(chunk_size.unwrap().to_string(), "10");
        } else {
            panic!("Expected Schedule clause");
        }
    }

    #[test]
    fn test_parse_schedule_clause_with_modifier() {
        let config = ParserConfig::with_parsing(Language::C);
        let data = parse_schedule_clause("monotonic: static, 4", &config).unwrap();
        if let ClauseData::Schedule {
            kind,
            modifiers,
            chunk_size,
        } = data
        {
            assert_eq!(kind, ScheduleKind::Static);
            assert_eq!(modifiers.len(), 1);
            assert_eq!(modifiers[0], ScheduleModifier::Monotonic);
            assert!(chunk_size.is_some());
        } else {
            panic!("Expected Schedule clause");
        }
    }

    #[test]
    fn test_parse_schedule_clause_with_multiple_modifiers() {
        let config = ParserConfig::with_parsing(Language::C);
        let data = parse_schedule_clause("monotonic, simd: dynamic", &config).unwrap();
        if let ClauseData::Schedule {
            kind,
            modifiers,
            chunk_size,
        } = data
        {
            assert_eq!(kind, ScheduleKind::Dynamic);
            assert_eq!(modifiers.len(), 2);
            assert!(chunk_size.is_none());
        } else {
            panic!("Expected Schedule clause");
        }
    }

    // Tests for map clause
    #[test]
    fn test_parse_map_clause_with_type() {
        let config = ParserConfig::with_parsing(Language::C);
        let data = parse_map_clause("to: arr", &config).unwrap();
        if let ClauseData::Map {
            map_type,
            mapper,
            items,
        } = data
        {
            assert_eq!(map_type, Some(MapType::To));
            assert!(mapper.is_none());
            assert_eq!(items.len(), 1);
        } else {
            panic!("Expected Map clause");
        }
    }

    #[test]
    fn test_parse_map_clause_tofrom() {
        let config = ParserConfig::with_parsing(Language::C);
        let data = parse_map_clause("tofrom: x, y, z", &config).unwrap();
        if let ClauseData::Map {
            map_type,
            mapper,
            items,
        } = data
        {
            assert_eq!(map_type, Some(MapType::ToFrom));
            assert!(mapper.is_none());
            assert_eq!(items.len(), 3);
        } else {
            panic!("Expected Map clause");
        }
    }

    #[test]
    fn test_parse_map_clause_without_type() {
        let config = ParserConfig::with_parsing(Language::C);
        let data = parse_map_clause("var1, var2", &config).unwrap();
        if let ClauseData::Map {
            map_type,
            mapper,
            items,
        } = data
        {
            assert!(map_type.is_none());
            assert!(mapper.is_none());
            assert_eq!(items.len(), 2);
        } else {
            panic!("Expected Map clause");
        }
    }

    #[test]
    fn test_parse_map_clause_with_array_section() {
        let config = ParserConfig::with_parsing(Language::C);
        let data = parse_map_clause("to: arr[0:N:2]", &config).unwrap();
        if let ClauseData::Map { items, .. } = data {
            match &items[0] {
                ClauseItem::Variable(var) => {
                    assert_eq!(var.name(), "arr");
                    assert_eq!(var.array_sections.len(), 1);
                    let section = &var.array_sections[0];
                    assert!(section.lower_bound.is_some());
                    assert!(section.length.is_some());
                    assert!(section.stride.is_some());
                }
                other => panic!("Expected variable, got {other:?}"),
            }
        } else {
            panic!("Expected Map clause");
        }
    }

    #[test]
    fn test_parse_map_clause_with_mapper() {
        let config = ParserConfig::with_parsing(Language::C);
        let data = parse_map_clause("mapper(custom), to: arr[0:N]", &config).unwrap();
        if let ClauseData::Map {
            map_type,
            mapper,
            items,
        } = data
        {
            assert_eq!(map_type, Some(MapType::To));
            assert_eq!(mapper.unwrap().to_string(), "custom");
            assert_eq!(items.len(), 1);
            assert!(matches!(items[0], ClauseItem::Variable(_)));
        } else {
            panic!("Expected Map clause with mapper");
        }
    }

    // Tests for depend clause
    #[test]
    fn test_parse_depend_type() {
        assert_eq!(parse_depend_type("in").unwrap(), DependType::In);
        assert_eq!(parse_depend_type("out").unwrap(), DependType::Out);
        assert_eq!(parse_depend_type("inout").unwrap(), DependType::Inout);
    }

    #[test]
    fn test_parse_clause_data_depend() {
        let clause = Clause {
            name: "depend".into(),
            kind: ClauseKind::Parenthesized("in: x, y".into()),
        };
        let config = ParserConfig::default();
        let data = parse_clause_data(&clause, &config).unwrap();
        if let ClauseData::Depend { depend_type, items } = data {
            assert_eq!(depend_type, DependType::In);
            assert_eq!(items.len(), 2);
        } else {
            panic!("Expected Depend clause");
        }
    }

    // Tests for linear clause
    #[test]
    fn test_parse_linear_clause_simple() {
        let config = ParserConfig::with_parsing(Language::C);
        let data = parse_linear_clause("x, y", &config).unwrap();
        if let ClauseData::Linear {
            modifier,
            items,
            step,
        } = data
        {
            assert!(modifier.is_none());
            assert_eq!(items.len(), 2);
            assert!(step.is_none());
        } else {
            panic!("Expected Linear clause");
        }
    }

    #[test]
    fn test_parse_linear_clause_with_step() {
        let config = ParserConfig::with_parsing(Language::C);
        let data = parse_linear_clause("i: 2", &config).unwrap();
        if let ClauseData::Linear {
            modifier,
            items,
            step,
        } = data
        {
            assert!(modifier.is_none());
            assert_eq!(items.len(), 1);
            assert!(step.is_some());
            assert_eq!(step.unwrap().to_string(), "2");
        } else {
            panic!("Expected Linear clause");
        }
    }

    #[test]
    fn test_parse_clause_data_fortran_private_variables() {
        let clause = Clause {
            name: "private".into(),
            kind: ClauseKind::Parenthesized("A(1:N), B(:, :)".into()),
        };
        let config = ParserConfig::with_parsing(Language::Fortran);
        let data = parse_clause_data(&clause, &config).unwrap();
        if let ClauseData::Private { items } = data {
            assert_eq!(items.len(), 2);
            match &items[0] {
                ClauseItem::Variable(var) => {
                    assert_eq!(var.name(), "A");
                    assert_eq!(var.array_sections.len(), 1);
                }
                other => panic!("expected variable, got {other:?}"),
            }
        } else {
            panic!("Expected Private clause");
        }
    }

    // Tests for proc_bind clause
    #[test]
    fn test_parse_clause_data_proc_bind() {
        let clause = Clause {
            name: "proc_bind".into(),
            kind: ClauseKind::Parenthesized("close".into()),
        };
        let config = ParserConfig::default();
        let data = parse_clause_data(&clause, &config).unwrap();
        assert_eq!(data, ClauseData::ProcBind(ProcBind::Close));
    }
}
