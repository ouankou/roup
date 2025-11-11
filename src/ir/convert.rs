//! Conversion from parser types to IR types
//!
//! This module handles the conversion from the parser's textual
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
/// # use roup::ir::{DirectiveKind, convert::parse_directive_kind_from_str};
/// let kind = parse_directive_kind_from_str("parallel for").unwrap();
/// assert_eq!(kind, DirectiveKind::ParallelFor);
///
/// let kind = parse_directive_kind_from_str("target teams distribute").unwrap();
/// assert_eq!(kind, DirectiveKind::TargetTeamsDistribute);
/// ```
/// Compatibility helper: accept an &str and lookup the DirectiveName, then call the
/// enum-based `parse_directive_kind`.
pub fn parse_directive_kind_from_str(name: &str) -> Result<DirectiveKind, ConversionError> {
    parse_directive_kind(crate::parser::directive_kind::lookup_directive_name(name))
}

pub fn parse_directive_kind(
    name: crate::parser::directive_kind::DirectiveName,
) -> Result<DirectiveKind, ConversionError> {
    use crate::parser::directive_kind::DirectiveName;

    match name {
        // Parallel constructs
        DirectiveName::Parallel => Ok(DirectiveKind::Parallel),
        DirectiveName::ParallelFor => Ok(DirectiveKind::ParallelFor),
        DirectiveName::ParallelDo => Ok(DirectiveKind::ParallelDo),
        DirectiveName::ParallelForSimd => Ok(DirectiveKind::ParallelForSimd),
        DirectiveName::ParallelDoSimd => Ok(DirectiveKind::ParallelDoSimd),
        DirectiveName::ParallelSections => Ok(DirectiveKind::ParallelSections),
        DirectiveName::ParallelLoop => Ok(DirectiveKind::ParallelLoop),
        DirectiveName::ParallelWorkshare => Ok(DirectiveKind::ParallelWorkshare),
        DirectiveName::ParallelLoopSimd => Ok(DirectiveKind::ParallelLoopSimd),
        DirectiveName::ParallelMasked => Ok(DirectiveKind::ParallelMasked),
        DirectiveName::ParallelMaster => Ok(DirectiveKind::ParallelMaster),

        DirectiveName::ParallelMasterTaskloop => Ok(DirectiveKind::ParallelMasterTaskloop),
        DirectiveName::ParallelMasterTaskloopSimd => Ok(DirectiveKind::ParallelMasterTaskloopSimd),

        // Work-sharing constructs
        DirectiveName::For => Ok(DirectiveKind::For),
        DirectiveName::Do => Ok(DirectiveKind::Do),
        DirectiveName::ForSimd => Ok(DirectiveKind::ForSimd),
        DirectiveName::DoSimd => Ok(DirectiveKind::DoSimd),
        DirectiveName::Sections => Ok(DirectiveKind::Sections),
        DirectiveName::Section => Ok(DirectiveKind::Section),
        DirectiveName::Single => Ok(DirectiveKind::Single),
        DirectiveName::Workshare => Ok(DirectiveKind::Workshare),
        DirectiveName::Loop => Ok(DirectiveKind::Loop),

        // SIMD constructs
        DirectiveName::Simd => Ok(DirectiveKind::Simd),
        DirectiveName::DeclareSimd => Ok(DirectiveKind::DeclareSimd),

        // Task constructs
        DirectiveName::Task => Ok(DirectiveKind::Task),
        DirectiveName::Taskloop => Ok(DirectiveKind::Taskloop),
        DirectiveName::TaskloopSimd => Ok(DirectiveKind::TaskloopSimd),
        DirectiveName::MaskedTaskloop => Ok(DirectiveKind::MaskedTaskloop),
        DirectiveName::MaskedTaskloopSimd => Ok(DirectiveKind::MaskedTaskloopSimd),
        DirectiveName::ParallelMaskedTaskloop => Ok(DirectiveKind::ParallelMaskedTaskloop),
        DirectiveName::ParallelMaskedTaskloopSimd => Ok(DirectiveKind::ParallelMaskedTaskloopSimd),
        DirectiveName::Taskyield => Ok(DirectiveKind::Taskyield),
        DirectiveName::Taskwait => Ok(DirectiveKind::Taskwait),
        DirectiveName::Taskgroup => Ok(DirectiveKind::Taskgroup),
        DirectiveName::Taskgraph => Ok(DirectiveKind::Taskgraph),
        DirectiveName::TaskIteration => Ok(DirectiveKind::TaskIteration),

        // Target constructs
        DirectiveName::Target => Ok(DirectiveKind::Target),
        DirectiveName::TargetData => Ok(DirectiveKind::TargetData),
        DirectiveName::TargetEnterData => Ok(DirectiveKind::TargetEnterData),
        DirectiveName::TargetExitData => Ok(DirectiveKind::TargetExitData),
        DirectiveName::TargetUpdate => Ok(DirectiveKind::TargetUpdate),
        DirectiveName::EndTarget => Ok(DirectiveKind::EndTarget),
        DirectiveName::TargetParallel => Ok(DirectiveKind::TargetParallel),
        DirectiveName::TargetParallelFor => Ok(DirectiveKind::TargetParallelFor),
        DirectiveName::TargetParallelDo => Ok(DirectiveKind::TargetParallelDo),
        DirectiveName::TargetParallelForSimd => Ok(DirectiveKind::TargetParallelForSimd),
        DirectiveName::TargetParallelDoSimd => Ok(DirectiveKind::TargetParallelDoSimd),
        DirectiveName::TargetParallelLoop => Ok(DirectiveKind::TargetParallelLoop),
        DirectiveName::TargetParallelLoopSimd => Ok(DirectiveKind::TargetParallelLoopSimd),
        DirectiveName::TargetSimd => Ok(DirectiveKind::TargetSimd),
        DirectiveName::TargetLoop => Ok(DirectiveKind::TargetLoop),
        DirectiveName::TargetLoopSimd => Ok(DirectiveKind::TargetLoopSimd),
        DirectiveName::TargetTeams => Ok(DirectiveKind::TargetTeams),
        DirectiveName::TargetTeamsDistribute => Ok(DirectiveKind::TargetTeamsDistribute),
        DirectiveName::TargetTeamsDistributeSimd => Ok(DirectiveKind::TargetTeamsDistributeSimd),
        DirectiveName::TargetTeamsDistributeParallelFor => {
            Ok(DirectiveKind::TargetTeamsDistributeParallelFor)
        }
        DirectiveName::TargetTeamsDistributeParallelForSimd => {
            Ok(DirectiveKind::TargetTeamsDistributeParallelForSimd)
        }
        DirectiveName::TargetTeamsDistributeParallelLoop => {
            Ok(DirectiveKind::TargetTeamsDistributeParallelLoop)
        }
        DirectiveName::TargetTeamsDistributeParallelLoopSimd => {
            Ok(DirectiveKind::TargetTeamsDistributeParallelLoopSimd)
        }
        DirectiveName::TargetTeamsDistributeParallelDo => {
            Ok(DirectiveKind::TargetTeamsDistributeParallelDo)
        }
        DirectiveName::TargetTeamsDistributeParallelDoSimd => {
            Ok(DirectiveKind::TargetTeamsDistributeParallelDoSimd)
        }
        DirectiveName::TargetTeamsLoop => Ok(DirectiveKind::TargetTeamsLoop),
        DirectiveName::TargetTeamsLoopSimd => Ok(DirectiveKind::TargetTeamsLoopSimd),

        // Teams constructs
        DirectiveName::Teams => Ok(DirectiveKind::Teams),
        DirectiveName::TeamsDistribute => Ok(DirectiveKind::TeamsDistribute),
        DirectiveName::TeamsDistributeSimd => Ok(DirectiveKind::TeamsDistributeSimd),
        DirectiveName::TeamsDistributeParallelFor => Ok(DirectiveKind::TeamsDistributeParallelFor),
        DirectiveName::TeamsDistributeParallelDo => Ok(DirectiveKind::TeamsDistributeParallelDo),
        DirectiveName::TeamsDistributeParallelForSimd => {
            Ok(DirectiveKind::TeamsDistributeParallelForSimd)
        }
        DirectiveName::TeamsDistributeParallelDoSimd => {
            Ok(DirectiveKind::TeamsDistributeParallelDoSimd)
        }
        DirectiveName::TeamsDistributeParallelLoop => {
            Ok(DirectiveKind::TeamsDistributeParallelLoop)
        }
        DirectiveName::TeamsDistributeParallelLoopSimd => {
            Ok(DirectiveKind::TeamsDistributeParallelLoopSimd)
        }
        DirectiveName::TeamsLoop => Ok(DirectiveKind::TeamsLoop),
        DirectiveName::TeamsLoopSimd => Ok(DirectiveKind::TeamsLoopSimd),

        // Synchronization constructs
        DirectiveName::Barrier => Ok(DirectiveKind::Barrier),
        DirectiveName::Critical => Ok(DirectiveKind::Critical),
        DirectiveName::Atomic => Ok(DirectiveKind::Atomic),
        DirectiveName::AtomicRead => Ok(DirectiveKind::AtomicRead),
        DirectiveName::AtomicWrite => Ok(DirectiveKind::AtomicWrite),
        DirectiveName::AtomicUpdate => Ok(DirectiveKind::AtomicUpdate),
        DirectiveName::AtomicCapture => Ok(DirectiveKind::AtomicCapture),
        DirectiveName::AtomicCompareCapture => Ok(DirectiveKind::AtomicCompareCapture),
        DirectiveName::Flush => Ok(DirectiveKind::Flush),
        DirectiveName::Ordered => Ok(DirectiveKind::Ordered),
        DirectiveName::Master => Ok(DirectiveKind::Master),
        DirectiveName::Masked => Ok(DirectiveKind::Masked),

        // Declare constructs
        DirectiveName::DeclareReduction => Ok(DirectiveKind::DeclareReduction),
        DirectiveName::DeclareMapper => Ok(DirectiveKind::DeclareMapper),
        DirectiveName::DeclareTarget => Ok(DirectiveKind::DeclareTarget),
        DirectiveName::BeginDeclareTarget => Ok(DirectiveKind::BeginDeclareTarget),
        DirectiveName::EndDeclareTarget => Ok(DirectiveKind::EndDeclareTarget),
        DirectiveName::DeclareVariant => Ok(DirectiveKind::DeclareVariant),
        DirectiveName::BeginDeclareVariant => Ok(DirectiveKind::BeginDeclareVariant),
        DirectiveName::EndDeclareVariant => Ok(DirectiveKind::EndDeclareVariant),
        DirectiveName::DeclareInduction => Ok(DirectiveKind::DeclareInduction),

        // Distribute constructs
        DirectiveName::Distribute => Ok(DirectiveKind::Distribute),
        DirectiveName::DistributeSimd => Ok(DirectiveKind::DistributeSimd),
        DirectiveName::DistributeParallelFor => Ok(DirectiveKind::DistributeParallelFor),
        DirectiveName::DistributeParallelForSimd => Ok(DirectiveKind::DistributeParallelForSimd),
        DirectiveName::DistributeParallelDo => Ok(DirectiveKind::DistributeParallelDo),
        DirectiveName::DistributeParallelDoSimd => Ok(DirectiveKind::DistributeParallelDoSimd),
        DirectiveName::DistributeParallelLoop => Ok(DirectiveKind::DistributeParallelLoop),
        DirectiveName::DistributeParallelLoopSimd => Ok(DirectiveKind::DistributeParallelLoopSimd),

        // Meta-directives
        DirectiveName::Metadirective => Ok(DirectiveKind::Metadirective),
        DirectiveName::BeginMetadirective => Ok(DirectiveKind::BeginMetadirective),
        DirectiveName::Assume => Ok(DirectiveKind::Assume),
        DirectiveName::Assumes => Ok(DirectiveKind::Assumes),
        DirectiveName::BeginAssumes => Ok(DirectiveKind::BeginAssumes),

        // Loop transformations
        DirectiveName::Tile => Ok(DirectiveKind::Tile),
        DirectiveName::Unroll => Ok(DirectiveKind::Unroll),
        DirectiveName::Fuse => Ok(DirectiveKind::Fuse),
        DirectiveName::Split => Ok(DirectiveKind::Split),
        DirectiveName::Interchange => Ok(DirectiveKind::Interchange),
        DirectiveName::Reverse => Ok(DirectiveKind::Reverse),
        DirectiveName::Stripe => Ok(DirectiveKind::Stripe),

        // Other constructs
        DirectiveName::Threadprivate => Ok(DirectiveKind::Threadprivate),
        DirectiveName::Allocate => Ok(DirectiveKind::Allocate),
        DirectiveName::Allocators => Ok(DirectiveKind::Allocators),
        DirectiveName::Requires => Ok(DirectiveKind::Requires),
        DirectiveName::Scan => Ok(DirectiveKind::Scan),
        DirectiveName::Depobj => Ok(DirectiveKind::Depobj),
        DirectiveName::Nothing => Ok(DirectiveKind::Nothing),
        DirectiveName::Error => Ok(DirectiveKind::Error),
        DirectiveName::Cancel => Ok(DirectiveKind::Cancel),
        DirectiveName::CancellationPoint => Ok(DirectiveKind::CancellationPoint),
        DirectiveName::Dispatch => Ok(DirectiveKind::Dispatch),
        DirectiveName::Interop => Ok(DirectiveKind::Interop),
        DirectiveName::Scope => Ok(DirectiveKind::Scope),
        DirectiveName::Groupprivate => Ok(DirectiveKind::Groupprivate),
        DirectiveName::Workdistribute => Ok(DirectiveKind::Workdistribute),

        // No fallback: unknown directive names must be handled explicitly.
        //
        // Rationale: We intentionally prefer an explicit error for unknown
        // directives (ConversionError::UnknownDirective) instead of silently
        // falling back to a textual (string) mapping. This ensures missing
        // mappings are visible during development and tests, and prevents
        // surprising behavior across the FFI boundary.
        DirectiveName::Other(s) => Err(ConversionError::UnknownDirective(s.as_ref().to_string())),
        // Catch-all for any DirectiveName variants not explicitly handled above
        _ => Err(ConversionError::UnknownDirective(name.as_ref().to_string())),
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
        // Check if this might be modifier syntax (has opening paren at top level before colon)
        if lang::split_once_top_level(items_str, '(').is_some() {
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
    // Use the directive name as &str via DirectiveName::as_ref()
    // Convert directive kind using the typed DirectiveName directly
    let kind = parse_directive_kind(directive.name_kind())?;

    // Convert clauses
    let mut clauses = Vec::new();
    let clause_config = config.for_language(language);
    for clause in &directive.clauses {
        let clause_data = parse_clause_data(clause, &clause_config)?;
        clauses.push(clause_data);
    }

    Ok(DirectiveIR::new(
        kind,
        directive.name.as_ref(),
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
            parse_directive_kind_from_str("parallel").unwrap(),
            DirectiveKind::Parallel
        );
        assert_eq!(
            parse_directive_kind_from_str("parallel for").unwrap(),
            DirectiveKind::ParallelFor
        );
    }

    #[test]
    fn test_parse_directive_kind_case_insensitive() {
        assert_eq!(
            parse_directive_kind_from_str("PARALLEL").unwrap(),
            DirectiveKind::Parallel
        );
        assert_eq!(
            parse_directive_kind_from_str("Parallel For").unwrap(),
            DirectiveKind::ParallelFor
        );
    }

    #[test]
    fn test_parse_directive_kind_whitespace() {
        assert_eq!(
            parse_directive_kind_from_str("  parallel  ").unwrap(),
            DirectiveKind::Parallel
        );
    }

    #[test]
    fn test_parse_directive_kind_unknown() {
        assert!(parse_directive_kind_from_str("unknown_directive").is_err());
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
