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
    lang, AffinityModifier, AtomicOp, BindModifier, ClauseData, ClauseItem, ConversionError,
    DefaultKind, DefaultmapBehavior, DefaultmapCategory, DependIterator, DependType,
    DepobjUpdateDependence, DeviceModifier, DeviceType, DirectiveIR, DirectiveKind, Expression,
    GrainsizeModifier, Identifier, Language, LastprivateModifier, LinearModifier, MapModifier,
    MapType, MemoryOrder, NumTasksModifier, OrderKind, OrderModifier, ParserConfig, ProcBind,
    ReductionModifier, ReductionOperator, RequireModifier, ScheduleKind, ScheduleModifier,
    SourceLocation, UsesAllocatorBuiltin, UsesAllocatorKind, UsesAllocatorSpec,
};
use crate::ast::{
    OmpClauseKind, OmpDirective, OmpDirectiveKind, OmpSelector, OmpSelectorConstruct,
    OmpSelectorConstructs, OmpSelectorDevice, OmpSelectorImpl, OmpSelectorScoredValue,
    OmpSelectorUser,
};
use crate::lexer::Language as LexerLanguage;
use crate::parser::clause::lookup_clause_name;
use crate::parser::directive_kind::lookup_directive_name;
use crate::parser::{
    clause::{
        ReductionModifier as ParserReductionModifier, ReductionOperator as ParserReductionOperator,
    },
    directive_kind::DirectiveName,
    Clause, ClauseKind, Directive,
};
use crate::parser::{ClauseRegistry, DirectiveRegistry, Parser};
use std::collections::HashSet;

impl From<ParserReductionModifier> for ReductionModifier {
    fn from(value: ParserReductionModifier) -> Self {
        match value {
            ParserReductionModifier::Task => ReductionModifier::Task,
            ParserReductionModifier::Inscan => ReductionModifier::Inscan,
            ParserReductionModifier::Default => ReductionModifier::Default,
        }
    }
}

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
        ".and." => Ok(ReductionOperator::LogicalAnd),
        ".or." => Ok(ReductionOperator::LogicalOr),
        "iand" => Ok(ReductionOperator::BitwiseAnd),
        "ior" => Ok(ReductionOperator::BitwiseOr),
        "ieor" => Ok(ReductionOperator::BitwiseXor),
        "&" => Ok(ReductionOperator::BitwiseAnd),
        "|" => Ok(ReductionOperator::BitwiseOr),
        "^" => Ok(ReductionOperator::BitwiseXor),
        "&&" => Ok(ReductionOperator::LogicalAnd),
        "||" => Ok(ReductionOperator::LogicalOr),
        "min" => Ok(ReductionOperator::Min),
        "max" => Ok(ReductionOperator::Max),
        _ => Ok(ReductionOperator::Custom),
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

        let mut seen_modifiers: HashSet<ScheduleModifier> = HashSet::new();
        let mut mods: Vec<ScheduleModifier> = Vec::new();

        for raw in mod_str
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            let modifier = match raw.to_ascii_lowercase().as_str() {
                "monotonic" => ScheduleModifier::Monotonic,
                "nonmonotonic" => ScheduleModifier::Nonmonotonic,
                "simd" => ScheduleModifier::Simd,
                _ => {
                    return Err(ConversionError::InvalidClauseSyntax(format!(
                        "Unknown schedule modifier: {raw}"
                    )))
                }
            };

            if (modifier == ScheduleModifier::Monotonic
                && seen_modifiers.contains(&ScheduleModifier::Nonmonotonic))
                || (modifier == ScheduleModifier::Nonmonotonic
                    && seen_modifiers.contains(&ScheduleModifier::Monotonic))
            {
                return Err(ConversionError::InvalidClauseSyntax(
                    "schedule clause cannot combine monotonic and nonmonotonic modifiers"
                        .to_string(),
                ));
            }

            if !seen_modifiers.insert(modifier) {
                return Err(ConversionError::InvalidClauseSyntax(format!(
                    "Duplicate schedule modifier: {raw}"
                )));
            }

            mods.push(modifier);
        }

        (mods, kind_str)
    } else {
        (vec![], content)
    };

    // Parse kind and optional chunk size (comma-separated)
    let parts: Vec<&str> = rest.split(',').map(|s| s.trim()).collect();

    let kind_token = parts.first().map(|s| s.to_ascii_lowercase());

    let kind = match kind_token.as_deref() {
        Some("static") => ScheduleKind::Static,
        Some("dynamic") => ScheduleKind::Dynamic,
        Some("guided") => ScheduleKind::Guided,
        Some("auto") => ScheduleKind::Auto,
        Some("runtime") => ScheduleKind::Runtime,
        Some(_) => {
            let s = parts.first().copied().unwrap_or_default();
            return Err(ConversionError::InvalidClauseSyntax(format!(
                "Unknown schedule kind: {s}"
            )));
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
    let mut modifiers = Vec::new();
    let mut iterators = Vec::new();

    if let Some((iterator_content, rest)) = extract_iterator_block(remainder) {
        iterators = parse_iterator_block(&iterator_content, config)?;
        remainder = rest.trim_start();
        if remainder.starts_with(',') {
            remainder = remainder[1..].trim_start();
        }
    }

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
            let mut map_type = None;
            let tokens = type_str
                .split(',')
                .map(|t| t.trim())
                .filter(|t| !t.is_empty());
            for token in tokens {
                match token.to_ascii_lowercase().as_str() {
                    "to" => map_type = Some(MapType::To),
                    "from" => map_type = Some(MapType::From),
                    "tofrom" => map_type = Some(MapType::ToFrom),
                    "alloc" => map_type = Some(MapType::Alloc),
                    "release" => map_type = Some(MapType::Release),
                    "delete" => map_type = Some(MapType::Delete),
                    "always" => modifiers.push(MapModifier::Always),
                    "close" => modifiers.push(MapModifier::Close),
                    "present" => modifiers.push(MapModifier::Present),
                    "self" => modifiers.push(MapModifier::SelfMap),
                    "ompx_hold" => modifiers.push(MapModifier::OmpxHold),
                    other => {
                        return Err(ConversionError::InvalidClauseSyntax(format!(
                            "Unknown map modifier or type: {other}"
                        )))
                    }
                }
            }
            (map_type, items.trim())
        } else {
            (Some(MapType::ToFrom), remainder)
        };

    let items = parse_identifier_list(items_str, config)?;

    Ok(ClauseData::Map {
        map_type,
        modifiers,
        mapper,
        iterators,
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
    match type_str.trim().to_ascii_lowercase().as_str() {
        "in" => Ok(DependType::In),
        "out" => Ok(DependType::Out),
        "inout" => Ok(DependType::Inout),
        "inoutset" => Ok(DependType::Inoutset),
        "mutexinoutset" => Ok(DependType::Mutexinoutset),
        "depobj" => Ok(DependType::Depobj),
        "source" => Ok(DependType::Source),
        "sink" => Ok(DependType::Sink),
        _ => Err(ConversionError::InvalidClauseSyntax(format!(
            "Unknown depend type: {type_str}"
        ))),
    }
}

/// Extract a leading iterator(...) block, returning the inner text and the
/// remaining clause content after the closing parenthesis.
fn extract_iterator_block(content: &str) -> Option<(String, &str)> {
    let trimmed = content.trim_start();
    const KEYWORD: &str = "iterator";
    if !trimmed.starts_with(KEYWORD) {
        return None;
    }

    let mut idx = KEYWORD.len();
    let bytes = trimmed.as_bytes();

    // Skip whitespace between keyword and '('
    while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
        idx += 1;
    }
    if idx >= bytes.len() || bytes[idx] != b'(' {
        return None;
    }

    let mut depth = 1usize;
    let mut i = idx + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    let inner = trimmed[(idx + 1)..i].to_string();
                    let remainder = &trimmed[(i + 1)..];
                    return Some((inner, remainder));
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn parse_iterator_definition(
    def: &str,
    config: &ParserConfig,
) -> Result<DependIterator, ConversionError> {
    let (lhs, rhs) = def.split_once('=').ok_or_else(|| {
        ConversionError::InvalidClauseSyntax("iterator definition missing '='".into())
    })?;

    let mut lhs_tokens: Vec<&str> = lhs.split_whitespace().collect();
    if lhs_tokens.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "iterator definition missing variable name".into(),
        ));
    }

    let name = lhs_tokens.pop().unwrap().trim();
    let type_name = if lhs_tokens.is_empty() {
        None
    } else {
        Some(lhs_tokens.join(" "))
    };

    let range = rhs.trim();
    let mut parts = range.split(':');
    let start_str = parts.next().ok_or_else(|| {
        ConversionError::InvalidClauseSyntax("iterator missing start expression".into())
    })?;
    let end_str = parts.next().ok_or_else(|| {
        ConversionError::InvalidClauseSyntax("iterator missing end expression".into())
    })?;
    let step_str = parts.next();

    if parts.next().is_some() {
        return Err(ConversionError::InvalidClauseSyntax(
            "iterator has too many ':' separators".into(),
        ));
    }

    let start = Expression::new(start_str, config);
    let end = Expression::new(end_str, config);
    let step = step_str.map(|s| Expression::new(s, config));

    Ok(DependIterator {
        type_name,
        name: Identifier::new(name),
        start,
        end,
        step,
    })
}

fn parse_iterator_block(
    block: &str,
    config: &ParserConfig,
) -> Result<Vec<DependIterator>, ConversionError> {
    let mut iterators = Vec::new();
    let mut current = String::new();
    let mut depth: i32 = 0;

    for ch in block.chars() {
        match ch {
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                let def = current.trim();
                if !def.is_empty() {
                    iterators.push(parse_iterator_definition(def, config)?);
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    if !current.trim().is_empty() {
        iterators.push(parse_iterator_definition(current.trim(), config)?);
    }

    Ok(iterators)
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
    let mut remaining = content.trim();
    let mut modifier_items: Vec<ClauseItem> = Vec::new();
    let mut modifier: Option<LinearModifier> = None;

    // Handle modifier(list): prefix. Accept val/ref/uval for OpenMP 5.1+.
    let lower = remaining.to_ascii_lowercase();
    let (modifier_len, modifier_kind) = if lower.starts_with("val") {
        (3, Some(LinearModifier::Val))
    } else if lower.starts_with("ref") {
        (3, Some(LinearModifier::Ref))
    } else if lower.starts_with("uval") {
        (4, Some(LinearModifier::Uval))
    } else {
        (0, None)
    };

    if let Some(kind) = modifier_kind {
        let after_keyword = remaining[modifier_len..].trim_start();
        if after_keyword.starts_with('(') {
            let (inner, rest) = extract_parenthesized(after_keyword)?;
            modifier_items = parse_identifier_list(inner.trim(), config)?;
            modifier = Some(kind);
            remaining = rest.trim_start();
            if remaining.starts_with(':') {
                remaining = remaining[1..].trim_start();
            } else if !remaining.is_empty() {
                return Err(ConversionError::InvalidClauseSyntax(
                    "linear modifier list must be followed by ':'".to_string(),
                ));
            }
        }
    }

    let (items_str, step_str) =
        if let Some((items_part, step_part)) = lang::rsplit_once_top_level(remaining, ':') {
            (items_part.trim(), Some(step_part.trim()))
        } else if modifier.is_some() && !remaining.is_empty() {
            ("", Some(remaining))
        } else {
            (remaining, None)
        };

    let mut items = if items_str.is_empty() {
        modifier_items.clone()
    } else {
        parse_identifier_list(items_str, config)?
    };

    if items.is_empty() {
        items = modifier_items;
    }

    let step = step_str.map(|s| Expression::new(s, config));

    Ok(ClauseData::Linear {
        modifier,
        items,
        step,
    })
}

fn parse_defaultmap_clause(kind: &ClauseKind<'_>) -> Result<ClauseData, ConversionError> {
    if let ClauseKind::Parenthesized(ref content) = kind {
        let text = content.as_ref().trim();
        if text.is_empty() {
            return Ok(ClauseData::Defaultmap {
                behavior: DefaultmapBehavior::Unspecified,
                category: None,
            });
        }

        let (behavior_str, category_str) =
            if let Some((behavior, rest)) = lang::split_once_top_level(text, ':') {
                (behavior.trim(), Some(rest.trim()))
            } else {
                (text, None)
            };

        let behavior = parse_defaultmap_behavior(behavior_str)?;
        let category = match category_str {
            Some(value) if !value.is_empty() => Some(parse_defaultmap_category(value)?),
            _ => None,
        };

        Ok(ClauseData::Defaultmap { behavior, category })
    } else {
        Err(ConversionError::InvalidClauseSyntax(
            "defaultmap clause requires parenthesized content".to_string(),
        ))
    }
}

#[allow(dead_code)]
fn parse_metadirective_selector(
    clause: &Clause<'_>,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    if let ClauseKind::Parenthesized(ref content) = clause.kind {
        let raw = content.as_ref();
        let mut selector = parse_selector_content(raw, config)?;
        selector.raw = Some(raw.trim().to_string());
        Ok(ClauseData::MetadirectiveSelector { selector })
    } else {
        Err(ConversionError::InvalidClauseSyntax(
            "metadirective selector requires parentheses".to_string(),
        ))
    }
}

fn parse_selector_content(
    content: &str,
    config: &ParserConfig,
) -> Result<OmpSelector, ConversionError> {
    let trimmed = content.trim();
    let (selector_part, nested_directive_part) = split_selector_and_directive(trimmed);

    let mut selector = OmpSelector::default();

    // Parse selector key/value pairs (device, implementation, user, construct)
    if !selector_part.is_empty() {
        for entry in split_top_level_items(selector_part) {
            let entry = entry.trim();
            if entry.is_empty() {
                continue;
            }
            let (key, value) = entry.split_once('=').unwrap_or((entry, ""));
            let key = key.trim().to_ascii_lowercase();
            let value = value.trim();
            match key.as_str() {
                "device" => {
                    selector.device = Some(parse_device_selector(value, config)?);
                }
                "implementation" | "impl" => {
                    selector.implementation = Some(parse_impl_selector(value)?);
                }
                "user" => {
                    selector.user = Some(parse_user_selector(value, config)?);
                }
                "construct" | "constructs" => {
                    selector.constructs = Some(parse_constructs_selector(value, config)?);
                }
                "target_device" => {
                    selector.device = Some(parse_device_selector(value, config)?);
                    selector.is_target_device = true;
                }
                _ => {
                    // Unknown selector key; ignore but keep raw for round-trip
                }
            }
        }
    }

    // Nested directive after colon (parse into nested_directive AST)
    if let Some(nested) = nested_directive_part {
        let nested_trimmed = nested.trim();
        if !nested_trimmed.is_empty() {
            if let Some(dir) = parse_nested_directive(nested_trimmed, config)? {
                selector.nested_directive = Some(Box::new(dir));
            }
        }
    }

    Ok(selector)
}

fn parse_nested_directive(
    text: &str,
    config: &ParserConfig,
) -> Result<Option<OmpDirective>, ConversionError> {
    let parser = Parser::new(DirectiveRegistry::default(), ClauseRegistry::default())
        .with_language(map_ir_language_to_lexer(config.language()));
    match parser.parse(text) {
        Ok((_rest, directive)) => {
            let kind = lookup_directive_name(directive.name.as_ref());
            let directive_kind = OmpDirectiveKind::try_from(kind).map_err(|_| {
                ConversionError::InvalidClauseSyntax(format!(
                    "Unknown nested directive in selector: {}",
                    directive.name.as_ref()
                ))
            })?;
            let mut clauses = Vec::new();
            for clause in &directive.clauses {
                let payload = parse_clause_data(clause, config)?;
                let clause_name = lookup_clause_name(clause.name.as_ref());
                let kind = OmpClauseKind::try_from(clause_name.clone()).map_err(|_| {
                    ConversionError::InvalidClauseSyntax(format!(
                        "Unknown clause in nested directive: {}",
                        clause.name.as_ref()
                    ))
                })?;
                clauses.push(crate::ast::OmpClause { kind, payload });
            }
            Ok(Some(OmpDirective {
                kind: directive_kind,
                parameter: None,
                clauses,
            }))
        }
        Err(_e) => Ok(None),
    }
}

fn split_selector_and_directive(input: &str) -> (&str, Option<&str>) {
    if let Some(idx) = find_top_level_colon(input) {
        let left = input[..idx].trim();
        let right = input[idx + 1..].trim();
        (left, Some(right))
    } else {
        (input, None)
    }
}

fn find_top_level_colon(input: &str) -> Option<usize> {
    let mut depth = 0;
    for (idx, ch) in input.char_indices() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            ':' if depth == 0 => return Some(idx),
            _ => {}
        }
    }
    None
}

fn map_ir_language_to_lexer(lang: Language) -> LexerLanguage {
    match lang {
        Language::C => LexerLanguage::C,
        Language::Cpp => LexerLanguage::C,
        Language::Fortran => LexerLanguage::FortranFree,
        _ => LexerLanguage::C,
    }
}

fn parse_device_selector(
    value: &str,
    config: &ParserConfig,
) -> Result<OmpSelectorDevice, ConversionError> {
    let mut device = OmpSelectorDevice::default();
    let inner = strip_braces(value).trim();
    if inner.is_empty() {
        return Ok(device);
    }

    for item in split_top_level_items(inner) {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        if let Some(arg) = item.strip_prefix("kind") {
            let args = extract_paren_arg(arg).unwrap_or(arg);
            let (score, val) = parse_scored_value(args);
            device.kind = Some(OmpSelectorScoredValue { score, value: val });
        } else if let Some(arg) = item.strip_prefix("isa") {
            let args = extract_paren_arg(arg).unwrap_or(arg);
            for isa in split_top_level_items(args) {
                let isa = isa.trim();
                if !isa.is_empty() {
                    let (score, val) = parse_scored_value(isa);
                    device
                        .isa
                        .push(OmpSelectorScoredValue { score, value: val });
                }
            }
        } else if let Some(arg) = item.strip_prefix("arch") {
            let args = extract_paren_arg(arg).unwrap_or(arg);
            for arch in split_top_level_items(args) {
                let arch = arch.trim();
                if !arch.is_empty() {
                    let (score, val) = parse_scored_value(arch);
                    device
                        .arch
                        .push(OmpSelectorScoredValue { score, value: val });
                }
            }
        } else if let Some(arg) = item.strip_prefix("device_num") {
            if let Some(expr) = extract_paren_arg(arg) {
                let (score, val) = parse_scored_value(expr);
                let expr_text = val.trim();
                if !expr_text.is_empty() {
                    device.device_num = Some(Expression::new(expr_text, config));
                    device.device_num_score = score;
                }
            }
        }
    }

    Ok(device)
}

fn parse_impl_selector(value: &str) -> Result<OmpSelectorImpl, ConversionError> {
    let mut implementation = OmpSelectorImpl::default();
    let inner = strip_braces(value).trim();
    if inner.is_empty() {
        return Ok(implementation);
    }

    for item in split_top_level_items(inner) {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        if let Some(arg) = item.strip_prefix("vendor") {
            let args = extract_paren_arg(arg).unwrap_or(arg);
            let vendor = args.trim();
            if !vendor.is_empty() {
                let (score, val) = parse_scored_value(vendor);
                implementation.vendor = Some(val);
                implementation.vendor_score = score;
            }
        } else if let Some(arg) = item.strip_prefix("extension") {
            let args = extract_paren_arg(arg).unwrap_or(arg);
            for ext in split_top_level_items(args) {
                let ext = ext.trim();
                if !ext.is_empty() {
                    let (score, val) = parse_scored_value(ext);
                    implementation.extensions.push(val);
                    implementation.extension_scores.push(score);
                }
            }
        } else {
            // Treat as user-defined implementation expression
            let (score, val) = parse_scored_value(item);
            let expr = val.trim();
            if !expr.is_empty() {
                implementation.user_expression = Some(expr.to_string());
                implementation.user_expression_score = score;
            }
        }
    }

    Ok(implementation)
}

fn parse_user_selector(
    value: &str,
    config: &ParserConfig,
) -> Result<OmpSelectorUser, ConversionError> {
    let mut user = OmpSelectorUser::default();
    let inner = strip_braces(value).trim();
    if inner.is_empty() {
        return Ok(user);
    }

    if let Some(arg) = inner.strip_prefix("condition") {
        if let Some(expr_body) = extract_paren_arg(arg) {
            let expr_text = expr_body.trim();
            if !expr_text.is_empty() {
                user.condition = Some(Expression::new(expr_text, config));
            }
        }
    }

    Ok(user)
}

fn parse_constructs_selector(
    value: &str,
    config: &ParserConfig,
) -> Result<OmpSelectorConstructs, ConversionError> {
    let mut constructs = OmpSelectorConstructs::default();
    let inner = strip_braces(value).trim();
    if inner.is_empty() {
        return Ok(constructs);
    }

    for item in split_top_level_items(inner) {
        let text = item.trim();
        if text.is_empty() {
            continue;
        }

        let (score, directive_text, _rest) = split_score_and_value(text);
        if let Some(dir) = parse_nested_directive(directive_text, config)? {
            let kind = dir.kind;
            constructs.constructs.push(OmpSelectorConstruct {
                score: score.clone(),
                kind,
                directive: Box::new(dir),
            });
            constructs.scores.push(score);
        }
    }

    Ok(constructs)
}

#[allow(dead_code)]
fn lookup_omp_construct(name: &str) -> Option<OmpDirectiveKind> {
    let normalized = name.trim().replace(' ', "");
    for kind in OmpDirectiveKind::ALL {
        let candidate = kind.as_str().replace(' ', "");
        if candidate.eq_ignore_ascii_case(&normalized) {
            return Some(*kind);
        }
    }
    None
}

fn parse_scored_value(input: &str) -> (Option<String>, String) {
    let trimmed = input.trim();
    let (score, rest, _) = split_score_and_value(trimmed);
    (score, rest.to_string())
}

fn split_score_and_value(input: &str) -> (Option<String>, &str, Option<&str>) {
    let mut score = None;
    let mut remainder = input;
    if let Some(start) = input.find("score(") {
        if let Some(end) = input[start..].find(')') {
            let score_val = &input[start + 6..start + end].trim();
            if !score_val.is_empty() {
                score = Some(score_val.to_string());
            }
            let after = input
                .get(start + end + 1..)
                .unwrap_or("")
                .trim_start_matches(':');
            remainder = after.trim();
        }
    }
    // Also split on colon if present (nested directive hint)
    if let Some(colon) = remainder.find(':') {
        let val = remainder[..colon].trim();
        let rest = remainder.get(colon + 1..).map(str::trim);
        return (score, val, rest);
    }
    (score, remainder, None)
}

fn strip_braces(value: &str) -> &str {
    let trimmed = value.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') && trimmed.len() >= 2 {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    }
}

fn extract_paren_arg(input: &str) -> Option<&str> {
    let trimmed = input.trim();
    if let Some(start) = trimmed.find('(') {
        if trimmed.ends_with(')') && start + 1 <= trimmed.len() - 1 {
            return Some(&trimmed[start + 1..trimmed.len() - 1]);
        }
    }
    None
}

fn parse_defaultmap_behavior(value: &str) -> Result<DefaultmapBehavior, ConversionError> {
    let normalized = value.trim().to_ascii_lowercase();
    let behavior = match normalized.as_str() {
        "" | "unspecified" => DefaultmapBehavior::Unspecified,
        "alloc" => DefaultmapBehavior::Alloc,
        "to" => DefaultmapBehavior::To,
        "from" => DefaultmapBehavior::From,
        "tofrom" => DefaultmapBehavior::Tofrom,
        "firstprivate" => DefaultmapBehavior::Firstprivate,
        "none" => DefaultmapBehavior::None,
        "default" => DefaultmapBehavior::Default,
        "present" => DefaultmapBehavior::Present,
        other => {
            return Err(ConversionError::InvalidClauseSyntax(format!(
                "Unknown defaultmap behavior: {other}"
            )))
        }
    };
    Ok(behavior)
}

fn parse_defaultmap_category(value: &str) -> Result<DefaultmapCategory, ConversionError> {
    let normalized = value.trim().to_ascii_lowercase();
    let category = match normalized.as_str() {
        "" | "unspecified" => DefaultmapCategory::Unspecified,
        "scalar" => DefaultmapCategory::Scalar,
        "aggregate" => DefaultmapCategory::Aggregate,
        "pointer" => DefaultmapCategory::Pointer,
        "all" => DefaultmapCategory::All,
        "allocatable" => DefaultmapCategory::Allocatable,
        other => {
            return Err(ConversionError::InvalidClauseSyntax(format!(
                "Unknown defaultmap category: {other}"
            )))
        }
    };
    Ok(category)
}

fn parse_uses_allocators_clause(
    kind: &ClauseKind<'_>,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    if let ClauseKind::Parenthesized(ref content) = kind {
        let entries = split_top_level_items(content.as_ref());
        let mut allocators = Vec::new();
        for raw in entries {
            let entry = raw.trim();
            if entry.is_empty() {
                continue;
            }
            let (name, traits_expr) = split_allocator_entry(entry)?;
            let allocator_kind = classify_allocator_name(name);
            let traits = match traits_expr {
                Some(expr_text) if !expr_text.trim().is_empty() => {
                    Some(Expression::new(expr_text.trim(), config))
                }
                _ => None,
            };
            allocators.push(UsesAllocatorSpec {
                allocator: allocator_kind,
                traits,
            });
        }

        Ok(ClauseData::UsesAllocators { allocators })
    } else {
        Err(ConversionError::InvalidClauseSyntax(
            "uses_allocators clause requires parenthesized content".to_string(),
        ))
    }
}

fn parse_requires_clause(
    kind: &ClauseKind<'_>,
    _config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    let raw_content = match kind {
        ClauseKind::Parenthesized(ref content) => content.as_ref().to_string(),
        ClauseKind::VariableList(list) => list
            .iter()
            .map(|c| c.as_ref().to_string())
            .collect::<Vec<_>>()
            .join(", "),
        ClauseKind::Bare => String::new(),
        _ => String::new(),
    };

    let items = split_requires_items(raw_content.as_str());
    let mut requirements = Vec::new();
    for raw in items {
        let token = raw.trim();
        if token.is_empty() {
            continue;
        }
        let normalized = token.to_ascii_lowercase();
        match normalized.as_str() {
            "reverse_offload" => requirements.push(RequireModifier::ReverseOffload),
            "unified_address" => requirements.push(RequireModifier::UnifiedAddress),
            "unified_shared_memory" => requirements.push(RequireModifier::UnifiedSharedMemory),
            "dynamic_allocators" => requirements.push(RequireModifier::DynamicAllocators),
            "self_maps" => requirements.push(RequireModifier::SelfMaps),
            "atomic_default_mem_order" => {
                return Err(ConversionError::InvalidClauseSyntax(
                    "atomic_default_mem_order requires a value".to_string(),
                ))
            }
            value if value.starts_with("atomic_default_mem_order") => {
                if let Some((_, order)) = value.split_once('(') {
                    let order = order.trim_end_matches(')').trim();
                    let mo = parse_memory_order(order)?;
                    requirements.push(RequireModifier::AtomicDefaultMemOrder(mo));
                } else {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "atomic_default_mem_order requires a value".to_string(),
                    ));
                }
            }
            "ext_implementation_defined_requirement" => {
                requirements.push(RequireModifier::ExtImplementationDefinedRequirement(None))
            }
            other => {
                let mo = parse_memory_order(other).ok();
                if let Some(order) = mo {
                    requirements.push(RequireModifier::AtomicDefaultMemOrder(order));
                } else {
                    requirements.push(RequireModifier::ExtImplementationDefinedRequirement(Some(
                        Identifier::new(token),
                    )));
                }
            }
        }
    }
    if requirements.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "requires clause must specify at least one requirement".to_string(),
        ));
    }
    Ok(ClauseData::Requires { requirements })
}

fn split_requires_items(input: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut depth = 0i32;

    for (idx, ch) in input.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' if depth > 0 => depth -= 1,
            ',' => {
                if depth == 0 {
                    if let Some(seg) = input.get(start..idx) {
                        let trimmed = seg.trim();
                        if !trimmed.is_empty() {
                            parts.push(trimmed);
                        }
                    }
                    start = idx + 1;
                }
            }
            _ if ch.is_whitespace() && depth == 0 => {
                if let Some(seg) = input.get(start..idx) {
                    let trimmed = seg.trim();
                    if !trimmed.is_empty() {
                        parts.push(trimmed);
                    }
                }
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }

    if let Some(seg) = input.get(start..) {
        let trimmed = seg.trim();
        if !trimmed.is_empty() {
            parts.push(trimmed);
        }
    }

    parts
}

fn parse_memory_order(value: &str) -> Result<MemoryOrder, ConversionError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "seq_cst" => Ok(MemoryOrder::SeqCst),
        "acq_rel" => Ok(MemoryOrder::AcqRel),
        "release" => Ok(MemoryOrder::Release),
        "acquire" => Ok(MemoryOrder::Acquire),
        "relaxed" => Ok(MemoryOrder::Relaxed),
        other => Err(ConversionError::InvalidClauseSyntax(format!(
            "Unknown memory order: {other}"
        ))),
    }
}

fn parse_device_clause(
    kind: &ClauseKind<'_>,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    if let ClauseKind::Parenthesized(ref content) = kind {
        let text = content.as_ref().trim();
        let (modifier, expr_text) = if let Some((m, rest)) = text.split_once(':') {
            let modifier = match m.trim() {
                "ancestor" => DeviceModifier::Ancestor,
                "device_num" => DeviceModifier::DeviceNum,
                other => {
                    return Err(ConversionError::InvalidClauseSyntax(format!(
                        "Unknown device modifier: {other}"
                    )))
                }
            };
            (modifier, rest.trim())
        } else {
            (DeviceModifier::Unspecified, text)
        };

        Ok(ClauseData::Device {
            modifier,
            device_num: Expression::new(expr_text, config),
        })
    } else {
        Err(ConversionError::InvalidClauseSyntax(
            "device clause requires parenthesized expression".to_string(),
        ))
    }
}

fn split_allocator_entry(input: &str) -> Result<(&str, Option<&str>), ConversionError> {
    if let Some(start) = input.find('(') {
        let mut depth = 0;
        let mut end = None;
        for (idx, ch) in input.char_indices().skip(start) {
            match ch {
                '(' => {
                    depth += 1;
                }
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(idx);
                        break;
                    }
                }
                _ => {}
            }
        }

        let end_idx = end.ok_or_else(|| {
            ConversionError::InvalidClauseSyntax(
                "uses_allocators clause has unmatched parentheses".to_string(),
            )
        })?;

        let name = input[..start].trim();
        let traits = input[start + 1..end_idx].trim();
        Ok((name, Some(traits)))
    } else {
        Ok((input.trim(), None))
    }
}

fn split_top_level_items(input: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (idx, ch) in input.char_indices() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            ',' if depth == 0 => {
                parts.push(&input[start..idx]);
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }
    if start < input.len() {
        parts.push(&input[start..]);
    }
    parts
}

fn classify_allocator_name(name: &str) -> UsesAllocatorKind {
    let trimmed = name.trim();
    let lower = trimmed.to_ascii_lowercase();
    match lower.as_str() {
        "omp_default_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::Default),
        "omp_large_cap_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::LargeCap),
        "omp_const_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::Const),
        "omp_high_bw_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::HighBw),
        "omp_low_lat_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::LowLat),
        "omp_cgroup_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::Cgroup),
        "omp_pteam_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::Pteam),
        "omp_thread_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::Thread),
        _ => UsesAllocatorKind::Custom(Identifier::new(trimmed)),
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
        "nowait" => match &clause.kind {
            ClauseKind::Bare => Ok(ClauseData::Bare(Identifier::new(clause_name))),
            ClauseKind::Parenthesized(content)
                if content.as_ref().trim().eq_ignore_ascii_case("is_deferred") =>
            {
                Ok(ClauseData::Bare(Identifier::new(clause_name)))
            }
            _ => Err(ConversionError::InvalidClauseSyntax(
                "nowait clause accepts only optional is_deferred modifier".to_string(),
            )),
        },

        "nogroup" | "untied" | "mergeable" | "seq_cst" | "relaxed" | "release" | "acquire"
        | "acq_rel" => match &clause.kind {
            ClauseKind::Bare => Ok(ClauseData::Bare(Identifier::new(clause_name))),
            _ => Err(ConversionError::InvalidClauseSyntax(format!(
                "{clause_name} clause does not take arguments"
            ))),
        },

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
                    _ => return Ok(ClauseData::Expression(Expression::new(kind_str, config))),
                };
                Ok(ClauseData::Default(kind))
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "default clause requires parenthesized content".to_string(),
                ))
            }
        }

        // Metadirective selectors: parse into typed selector data (raw today)
        "when" | "otherwise" | "match" => parse_metadirective_selector(clause, config),

        // defaultmap(behavior[:category])
        "defaultmap" => parse_defaultmap_clause(&clause.kind),

        // sizes(list) on tile directive
        "sizes" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let items = parse_identifier_list(content.as_ref(), config)?;
                Ok(ClauseData::ItemList(items))
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "sizes clause requires a parenthesized list".to_string(),
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

        // to/from/link(list) used by declare target and friends
        "to" | "from" | "link" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                match parse_identifier_list(content.as_ref(), config) {
                    Ok(items) => {
                        if items.is_empty() {
                            Err(ConversionError::InvalidClauseSyntax(format!(
                                "{clause_name} clause requires a non-empty variable list"
                            )))
                        } else {
                            Ok(ClauseData::ItemList(items))
                        }
                    }
                    Err(_) => Ok(ClauseData::Expression(Expression::new(
                        content.as_ref().trim(),
                        config,
                    ))),
                }
            } else {
                Err(ConversionError::InvalidClauseSyntax(format!(
                    "{clause_name} clause requires parenthesized content"
                )))
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
        "reduction" => match &clause.kind {
            ClauseKind::Parenthesized(ref content) => {
                let content = content.as_ref();
                // Find the colon separator between operator and list
                if let Some((op_str, items_str)) = lang::split_once_top_level(content, ':') {
                    let tokens: Vec<&str> = op_str
                        .split(',')
                        .map(|t| t.trim())
                        .filter(|t| !t.is_empty())
                        .collect();
                    let (modifier_tokens, op_token) = if tokens.len() > 1 {
                        (
                            tokens[..tokens.len() - 1].to_vec(),
                            tokens.last().copied().unwrap(),
                        )
                    } else {
                        (Vec::new(), op_str.trim())
                    };

                    let modifiers: Vec<ReductionModifier> = modifier_tokens
                        .iter()
                        .filter_map(|m| match *m {
                            "task" => Some(ReductionModifier::Task),
                            "inscan" => Some(ReductionModifier::Inscan),
                            "default" => Some(ReductionModifier::Default),
                            _ => None,
                        })
                        .collect();

                    let operator = parse_reduction_operator(op_token)?;
                    let user_identifier = match operator {
                        ReductionOperator::Custom => Some(Identifier::new(op_token)),
                        _ => None,
                    };
                    let items = parse_identifier_list(items_str.trim(), config)?;
                    let space_after_colon = items_str.starts_with(' ');
                    Ok(ClauseData::Reduction {
                        modifiers,
                        operator,
                        user_identifier,
                        items,
                        space_after_colon,
                    })
                } else {
                    Err(ConversionError::InvalidClauseSyntax(
                        "reduction clause requires 'operator: list' format".to_string(),
                    ))
                }
            }
            ClauseKind::ReductionClause {
                modifiers,
                operator,
                user_defined_identifier,
                variables,
                space_after_colon,
            } => {
                let op_text = match operator {
                    ParserReductionOperator::Add => "+",
                    ParserReductionOperator::Sub => "-",
                    ParserReductionOperator::Mul => "*",
                    ParserReductionOperator::Max => "max",
                    ParserReductionOperator::Min => "min",
                    ParserReductionOperator::BitAnd => "&",
                    ParserReductionOperator::BitOr => "|",
                    ParserReductionOperator::BitXor => "^",
                    ParserReductionOperator::LogAnd => "&&",
                    ParserReductionOperator::LogOr => "||",
                    ParserReductionOperator::FortAnd => ".and.",
                    ParserReductionOperator::FortOr => ".or.",
                    ParserReductionOperator::FortEqv => ".eqv.",
                    ParserReductionOperator::FortNeqv => ".neqv.",
                    ParserReductionOperator::FortIand => "iand",
                    ParserReductionOperator::FortIor => "ior",
                    ParserReductionOperator::FortIeor => "ieor",
                    ParserReductionOperator::UserDefined => {
                        user_defined_identifier.as_deref().unwrap_or("user")
                    }
                };
                let operator = parse_reduction_operator(op_text.trim())?;
                let items = variables
                    .iter()
                    .map(|item| ClauseItem::Identifier(Identifier::new(item.as_ref())))
                    .collect();
                Ok(ClauseData::Reduction {
                    modifiers: modifiers.iter().map(|m| (*m).into()).collect(),
                    operator,
                    user_identifier: user_defined_identifier
                        .as_ref()
                        .map(|id| Identifier::new(id.as_ref())),
                    items,
                    space_after_colon: *space_after_colon,
                })
            }
            _ => Err(ConversionError::InvalidClauseSyntax(
                "reduction clause requires parenthesized content".to_string(),
            )),
        },

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

        // depend(dependence-type: list) or depend(source) or depend(sink)
        "depend" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let mut remaining = content.as_ref().trim();
                let mut iterators = Vec::new();

                if let Some((iterator_content, rest)) = extract_iterator_block(remaining) {
                    iterators = parse_iterator_block(&iterator_content, config)?;
                    remaining = rest.trim_start();
                    if remaining.starts_with(',') {
                        remaining = remaining[1..].trim_start();
                    }
                }

                // Find the colon separator using top-level detection
                if let Some((type_str, items_str)) = lang::split_once_top_level(remaining, ':') {
                    // Parse the dependence type
                    let depend_type = parse_depend_type(type_str.trim())?;

                    // Parse the item list
                    let items = parse_identifier_list(items_str.trim(), config)?;

                    Ok(ClauseData::Depend {
                        depend_type,
                        items,
                        iterators,
                    })
                } else {
                    // No colon found - could be depend(source) or depend(sink) without items
                    let type_str = remaining.trim();
                    let depend_type = parse_depend_type(type_str)?;

                    // Empty items list for source/sink without variables
                    Ok(ClauseData::Depend {
                        depend_type,
                        items: vec![],
                        iterators,
                    })
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

        // bind(parallel|teams|thread|user)
        "bind" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let kind_str = content.as_ref().trim().to_ascii_lowercase();
                let binding = match kind_str.as_str() {
                    "teams" => BindModifier::Teams,
                    "parallel" => BindModifier::Parallel,
                    "thread" => BindModifier::Thread,
                    "user" => BindModifier::User,
                    _ => {
                        return Err(ConversionError::InvalidClauseSyntax(format!(
                            "Unknown bind kind: {kind_str}"
                        )))
                    }
                };
                Ok(ClauseData::Bind(binding))
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "bind clause requires parenthesized content".to_string(),
                ))
            }
        }

        // proc_bind(master|close|spread|primary)
        "proc_bind" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref();
                let kind_str = content.trim().to_ascii_lowercase();
                let proc_bind = match kind_str {
                    ref s if s == "master" => ProcBind::Master,
                    ref s if s == "close" => ProcBind::Close,
                    ref s if s == "spread" => ProcBind::Spread,
                    ref s if s == "primary" => ProcBind::Primary,
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

        // lastprivate([modifier:] list)
        "lastprivate" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref();
                let (modifier, list_str) =
                    if let Some((modifier, rest)) = lang::split_once_top_level(content, ':') {
                        (Some(modifier.trim()), rest)
                    } else {
                        (None, content)
                    };

                let modifier = match modifier {
                    Some("") => None,
                    Some("conditional") => Some(LastprivateModifier::Conditional),
                    Some(other) => {
                        return Err(ConversionError::InvalidClauseSyntax(format!(
                            "Unknown lastprivate modifier: {other}"
                        )))
                    }
                    None => None,
                };

                let items = parse_identifier_list(list_str.trim(), config)?;
                Ok(ClauseData::Lastprivate { modifier, items })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "lastprivate clause requires parenthesized content".to_string(),
                ))
            }
        }

        // copyin(list)
        "copyin" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let items = parse_identifier_list(content.as_ref(), config)?;
                Ok(ClauseData::Copyin { items })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "copyin clause requires a variable list".to_string(),
                ))
            }
        }

        // copyprivate(list)
        "copyprivate" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let items = parse_identifier_list(content.as_ref(), config)?;
                Ok(ClauseData::Copyprivate { items })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "copyprivate clause requires a variable list".to_string(),
                ))
            }
        }

        // num_teams(expr)
        "num_teams" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                Ok(ClauseData::NumTeams {
                    num: Expression::new(content.as_ref().trim(), config),
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "num_teams clause requires parenthesized expression".to_string(),
                ))
            }
        }

        // thread_limit(expr)
        "thread_limit" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                Ok(ClauseData::ThreadLimit {
                    limit: Expression::new(content.as_ref().trim(), config),
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "thread_limit clause requires parenthesized expression".to_string(),
                ))
            }
        }

        // aligned(list[:alignment])
        "aligned" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref();
                let (items_part, alignment_part) =
                    if let Some((items, alignment)) = lang::split_once_top_level(content, ':') {
                        (items, Some(alignment))
                    } else {
                        (content, None)
                    };

                let items = parse_identifier_list(items_part.trim(), config)?;
                if items.is_empty() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "aligned clause requires at least one variable".to_string(),
                    ));
                }
                let alignment =
                    match alignment_part {
                        Some(value) if !value.trim().is_empty() => {
                            Some(Expression::new(value.trim(), config))
                        }
                        Some(_) => return Err(ConversionError::InvalidClauseSyntax(
                            "aligned clause requires a non-empty alignment expression after ':'"
                                .to_string(),
                        )),
                        None => None,
                    };
                Ok(ClauseData::Aligned { items, alignment })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "aligned clause requires parenthesized content".to_string(),
                ))
            }
        }

        // safelen(length)
        "safelen" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                if content.as_ref().trim().is_empty() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "safelen clause requires a length expression".to_string(),
                    ));
                }
                Ok(ClauseData::Safelen {
                    length: Expression::new(content.as_ref().trim(), config),
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "safelen clause requires parenthesized expression".to_string(),
                ))
            }
        }

        // simdlen(length)
        "simdlen" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                if content.as_ref().trim().is_empty() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "simdlen clause requires a length expression".to_string(),
                    ));
                }
                Ok(ClauseData::Simdlen {
                    length: Expression::new(content.as_ref().trim(), config),
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "simdlen clause requires parenthesized expression".to_string(),
                ))
            }
        }

        // in_reduction/task_reduction share the reduction parser
        "in_reduction" | "task_reduction" => match &clause.kind {
            ClauseKind::Parenthesized(ref content) => {
                let content = content.as_ref();
                if let Some((op_str, items_str)) = lang::split_once_top_level(content, ':') {
                    let operator = parse_reduction_operator(op_str.trim())?;
                    let items = parse_identifier_list(items_str.trim(), config)?;
                    let space_after_colon = items_str.starts_with(' ');
                    Ok(ClauseData::Reduction {
                        modifiers: Vec::new(),
                        operator,
                        user_identifier: None,
                        items,
                        space_after_colon,
                    })
                } else {
                    Err(ConversionError::InvalidClauseSyntax(
                        "reduction-style clauses require 'operator: list' syntax".to_string(),
                    ))
                }
            }
            ClauseKind::ReductionClause {
                modifiers,
                operator,
                user_defined_identifier,
                variables,
                space_after_colon,
            } => {
                let op_text = match operator {
                    ParserReductionOperator::Add => "+",
                    ParserReductionOperator::Sub => "-",
                    ParserReductionOperator::Mul => "*",
                    ParserReductionOperator::Max => "max",
                    ParserReductionOperator::Min => "min",
                    ParserReductionOperator::BitAnd => "&",
                    ParserReductionOperator::BitOr => "|",
                    ParserReductionOperator::BitXor => "^",
                    ParserReductionOperator::LogAnd => "&&",
                    ParserReductionOperator::LogOr => "||",
                    ParserReductionOperator::FortAnd => ".and.",
                    ParserReductionOperator::FortOr => ".or.",
                    ParserReductionOperator::FortEqv => ".eqv.",
                    ParserReductionOperator::FortNeqv => ".neqv.",
                    ParserReductionOperator::FortIand => "iand",
                    ParserReductionOperator::FortIor => "ior",
                    ParserReductionOperator::FortIeor => "ieor",
                    ParserReductionOperator::UserDefined => {
                        user_defined_identifier.as_deref().unwrap_or("user")
                    }
                };
                let operator = parse_reduction_operator(op_text.trim())?;
                let items = variables
                    .iter()
                    .map(|item| ClauseItem::Identifier(Identifier::new(item.as_ref())))
                    .collect();
                Ok(ClauseData::Reduction {
                    modifiers: modifiers.iter().map(|m| (*m).into()).collect(),
                    operator,
                    user_identifier: user_defined_identifier
                        .as_ref()
                        .map(|id| Identifier::new(id.as_ref())),
                    items,
                    space_after_colon: *space_after_colon,
                })
            }
            _ => Err(ConversionError::InvalidClauseSyntax(
                "reduction-style clauses require parenthesized content".to_string(),
            )),
        },

        // dist_schedule(kind[, chunk])
        "dist_schedule" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let parts: Vec<&str> = content
                    .as_ref()
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect();
                if parts.is_empty() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "dist_schedule requires a schedule kind".to_string(),
                    ));
                }
                let kind = match parts[0] {
                    "static" => ScheduleKind::Static,
                    "dynamic" => ScheduleKind::Dynamic,
                    "guided" => ScheduleKind::Guided,
                    other => {
                        return Err(ConversionError::InvalidClauseSyntax(format!(
                            "Unknown dist_schedule kind: {other}"
                        )))
                    }
                };
                let chunk_size = parts
                    .get(1)
                    .map(|value| Expression::new(value.trim(), config));
                Ok(ClauseData::DistSchedule { kind, chunk_size })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "dist_schedule clause requires parenthesized content".to_string(),
                ))
            }
        }

        // grainsize(expression)
        "grainsize" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let trimmed = content.as_ref().trim();
                let mut modifier = GrainsizeModifier::Unspecified;
                let mut expr_text = trimmed;

                if let Some(rest) = trimmed.strip_prefix("strict") {
                    let after = rest.trim_start();
                    if let Some(after_colon) = after.strip_prefix(':') {
                        modifier = GrainsizeModifier::Strict;
                        expr_text = after_colon.trim_start();
                    }
                }

                Ok(ClauseData::Grainsize {
                    modifier,
                    grain: Expression::new(expr_text, config),
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "grainsize clause requires parenthesized expression".to_string(),
                ))
            }
        }

        // num_tasks(expression)
        "num_tasks" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let trimmed = content.as_ref().trim();
                let mut modifier = NumTasksModifier::Unspecified;
                let mut expr_text = trimmed;

                if let Some(rest) = trimmed.strip_prefix("strict") {
                    let after = rest.trim_start();
                    if let Some(after_colon) = after.strip_prefix(':') {
                        modifier = NumTasksModifier::Strict;
                        expr_text = after_colon.trim_start();
                    }
                }

                Ok(ClauseData::NumTasks {
                    modifier,
                    num: Expression::new(expr_text, config),
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "num_tasks clause requires parenthesized expression".to_string(),
                ))
            }
        }

        // filter(expression)
        "filter" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                Ok(ClauseData::Filter {
                    thread_num: Expression::new(content.as_ref().trim(), config),
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "filter clause requires parenthesized expression".to_string(),
                ))
            }
        }

        // affinity(list)
        "affinity" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let mut modifier = AffinityModifier::Unspecified;
                let mut iterators = Vec::new();
                let mut remaining = content.as_ref().trim();

                if let Some((iterator_content, rest)) = extract_iterator_block(remaining) {
                    modifier = AffinityModifier::Iterator;
                    iterators = parse_iterator_block(&iterator_content, config)?;
                    remaining = rest.trim_start();
                    if remaining.starts_with(':') {
                        remaining = remaining[1..].trim_start();
                    }
                }

                let items = parse_identifier_list(remaining, config)?;
                Ok(ClauseData::Affinity {
                    modifier,
                    iterators,
                    items,
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "affinity clause requires a variable list".to_string(),
                ))
            }
        }

        // depobj_update(kind)
        "depobj_update" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let dep = match content.as_ref().trim() {
                    "in" => DepobjUpdateDependence::In,
                    "out" => DepobjUpdateDependence::Out,
                    "inout" => DepobjUpdateDependence::Inout,
                    "inoutset" => DepobjUpdateDependence::Inoutset,
                    "mutexinoutset" => DepobjUpdateDependence::Mutexinoutset,
                    "depobj" => DepobjUpdateDependence::Depobj,
                    "sink" => DepobjUpdateDependence::Sink,
                    "source" => DepobjUpdateDependence::Source,
                    other => {
                        return Err(ConversionError::InvalidClauseSyntax(format!(
                            "Unknown depobj_update dependence: {other}"
                        )))
                    }
                };
                Ok(ClauseData::DepobjUpdate { dependence: dep })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "depobj_update clause requires parenthesized content".to_string(),
                ))
            }
        }

        // priority(expression)
        "priority" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                Ok(ClauseData::Priority {
                    priority: Expression::new(content.as_ref().trim(), config),
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "priority clause requires parenthesized expression".to_string(),
                ))
            }
        }

        // device(expression)
        "device" => parse_device_clause(&clause.kind, config),

        // device_type(host|nohost|any)
        "device_type" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let value = content.as_ref().trim();
                let device_type = match value {
                    "host" => DeviceType::Host,
                    "nohost" => DeviceType::Nohost,
                    "any" => DeviceType::Any,
                    other => {
                        return Err(ConversionError::InvalidClauseSyntax(format!(
                            "Unknown device_type value: {other}"
                        )))
                    }
                };
                Ok(ClauseData::DeviceType(device_type))
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "device_type clause requires parenthesized value".to_string(),
                ))
            }
        }

        // use_device_ptr(list)
        "use_device_ptr" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let items = parse_identifier_list(content.as_ref(), config)?;
                Ok(ClauseData::UseDevicePtr { items })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "use_device_ptr clause requires a variable list".to_string(),
                ))
            }
        }

        // use_device_addr(list)
        "use_device_addr" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let items = parse_identifier_list(content.as_ref(), config)?;
                Ok(ClauseData::UseDeviceAddr { items })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "use_device_addr clause requires a variable list".to_string(),
                ))
            }
        }

        // is_device_ptr(list)
        "is_device_ptr" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let items = parse_identifier_list(content.as_ref(), config)?;
                Ok(ClauseData::IsDevicePtr { items })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "is_device_ptr clause requires a variable list".to_string(),
                ))
            }
        }

        // has_device_addr(list)
        "has_device_addr" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let items = parse_identifier_list(content.as_ref(), config)?;
                Ok(ClauseData::HasDeviceAddr { items })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "has_device_addr clause requires a variable list".to_string(),
                ))
            }
        }

        // allocate([allocator:] list)
        "allocate" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref();
                let (allocator_part, list_part) =
                    if let Some((alloc, rest)) = lang::split_once_top_level(content, ':') {
                        (Some(alloc.trim()), rest)
                    } else {
                        (None, content)
                    };
                let allocator = allocator_part
                    .filter(|value| !value.is_empty())
                    .map(Identifier::new);
                let items = parse_identifier_list(list_part.trim(), config)?;
                Ok(ClauseData::Allocate { allocator, items })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "allocate clause requires parenthesized content".to_string(),
                ))
            }
        }

        // allocator(allocator-handle)
        "allocator" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref().trim();
                if content.contains(':') {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "allocator clause must not contain ':' separators".to_string(),
                    ));
                }
                Ok(ClauseData::Allocator {
                    allocator: Identifier::new(content),
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "allocator clause requires parenthesized content".to_string(),
                ))
            }
        }

        // order(concurrent)
        "order" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let value = content.as_ref().trim();
                let (modifier, kind_str) =
                    if let Some((modifier_str, rest)) = lang::split_once_top_level(value, ':') {
                        let modifier = match modifier_str.trim().to_ascii_lowercase().as_str() {
                            "" => OrderModifier::Unspecified,
                            "reproducible" => OrderModifier::Reproducible,
                            "unconstrained" => OrderModifier::Unconstrained,
                            other => {
                                return Err(ConversionError::InvalidClauseSyntax(format!(
                                    "Unknown order modifier: {other}"
                                )))
                            }
                        };
                        (modifier, rest.trim())
                    } else {
                        (OrderModifier::Unspecified, value)
                    };

                match kind_str.to_ascii_lowercase().as_str() {
                    "concurrent" => Ok(ClauseData::Order {
                        modifier,
                        kind: OrderKind::Concurrent,
                    }),
                    other => Err(ConversionError::InvalidClauseSyntax(format!(
                        "Unknown order value: {other}"
                    ))),
                }
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "order clause requires parenthesized value".to_string(),
                ))
            }
        }

        // atomic_default_mem_order(seq_cst|acq_rel|...)
        "atomic_default_mem_order" => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let order = match content.as_ref().trim() {
                    "seq_cst" => MemoryOrder::SeqCst,
                    "acq_rel" => MemoryOrder::AcqRel,
                    "release" => MemoryOrder::Release,
                    "acquire" => MemoryOrder::Acquire,
                    "relaxed" => MemoryOrder::Relaxed,
                    other => {
                        return Err(ConversionError::InvalidClauseSyntax(format!(
                            "Unknown atomic default memory order: {other}"
                        )))
                    }
                };
                Ok(ClauseData::AtomicDefaultMemOrder(order))
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "atomic_default_mem_order clause requires parenthesized value".to_string(),
                ))
            }
        }

        // atomic operation clauses (read/write/update/capture)
        "read" => Ok(ClauseData::AtomicOperation {
            op: AtomicOp::Read,
            memory_order: None,
        }),
        "write" => Ok(ClauseData::AtomicOperation {
            op: AtomicOp::Write,
            memory_order: None,
        }),
        "update" => match &clause.kind {
            ClauseKind::Parenthesized(ref content) => {
                let dep = match content.as_ref().trim() {
                    "in" => DepobjUpdateDependence::In,
                    "out" => DepobjUpdateDependence::Out,
                    "inout" => DepobjUpdateDependence::Inout,
                    "inoutset" => DepobjUpdateDependence::Inoutset,
                    "mutexinoutset" => DepobjUpdateDependence::Mutexinoutset,
                    "depobj" => DepobjUpdateDependence::Depobj,
                    "sink" => DepobjUpdateDependence::Sink,
                    "source" => DepobjUpdateDependence::Source,
                    other => {
                        return Err(ConversionError::InvalidClauseSyntax(format!(
                            "Unknown depobj update dependence: {other}"
                        )))
                    }
                };
                Ok(ClauseData::DepobjUpdate { dependence: dep })
            }
            _ => Ok(ClauseData::AtomicOperation {
                op: AtomicOp::Update,
                memory_order: None,
            }),
        },
        "capture" => Ok(ClauseData::AtomicOperation {
            op: AtomicOp::Capture,
            memory_order: None,
        }),

        // branch hints and SIMD modifiers
        "nontemporal" | "uniform" => match &clause.kind {
            ClauseKind::Parenthesized(content) => {
                let items = parse_identifier_list(content.as_ref(), config)?;
                Ok(ClauseData::ItemList(items))
            }
            ClauseKind::VariableList(vars) => {
                let joined = vars.join(", ");
                let items = parse_identifier_list(&joined, config)?;
                Ok(ClauseData::ItemList(items))
            }
            ClauseKind::Bare => Ok(ClauseData::ItemList(Vec::new())),
            _ => Err(ConversionError::InvalidClauseSyntax(format!(
                "{clause_name} clause requires a variable list"
            ))),
        },
        "inbranch" => Ok(ClauseData::Bare(Identifier::new("inbranch"))),
        "notinbranch" => Ok(ClauseData::Bare(Identifier::new("notinbranch"))),
        "inclusive" => Ok(ClauseData::Bare(Identifier::new("inclusive"))),
        "exclusive" => Ok(ClauseData::Bare(Identifier::new("exclusive"))),

        // uses_allocators(allocator[(traits)], ...)
        "uses_allocators" => parse_uses_allocators_clause(&clause.kind, config),

        // requires(...) with modifiers
        "requires" => parse_requires_clause(&clause.kind, config),

        // --------------------------------------------------------------------
        // Generic fallback: preserve clause structure even when the clause
        // name is not explicitly handled above.
        //
        // Ompparser accepts a long tail of OpenMP clauses that map cleanly
        // onto one of our existing payload shapes (bare flag, expression, or
        // variable list). Instead of failing the entire parse, materialize a
        // reasonable ClauseData so downstream consumers can round-trip the
        // pragma text without string parsing.
        // --------------------------------------------------------------------
        _ => match &clause.kind {
            ClauseKind::Bare => Ok(ClauseData::Bare(Identifier::new(clause_name))),
            ClauseKind::Parenthesized(content) => Ok(ClauseData::Expression(Expression::new(
                content.as_ref().trim(),
                config,
            ))),
            ClauseKind::VariableList(vars) => {
                let joined = vars.join(", ");
                let items = parse_identifier_list(&joined, config)?;
                Ok(ClauseData::ItemList(items))
            }
            ClauseKind::ReductionClause {
                modifiers,
                operator,
                user_defined_identifier,
                variables,
                space_after_colon,
            } => {
                let op_text = match operator {
                    ParserReductionOperator::Add => "+",
                    ParserReductionOperator::Sub => "-",
                    ParserReductionOperator::Mul => "*",
                    ParserReductionOperator::Max => "max",
                    ParserReductionOperator::Min => "min",
                    ParserReductionOperator::BitAnd => "&",
                    ParserReductionOperator::BitOr => "|",
                    ParserReductionOperator::BitXor => "^",
                    ParserReductionOperator::LogAnd => "&&",
                    ParserReductionOperator::LogOr => "||",
                    ParserReductionOperator::FortAnd => ".and.",
                    ParserReductionOperator::FortOr => ".or.",
                    ParserReductionOperator::FortEqv => ".eqv.",
                    ParserReductionOperator::FortNeqv => ".neqv.",
                    ParserReductionOperator::FortIand => "iand",
                    ParserReductionOperator::FortIor => "ior",
                    ParserReductionOperator::FortIeor => "ieor",
                    ParserReductionOperator::UserDefined => {
                        user_defined_identifier.as_deref().unwrap_or("user")
                    }
                };
                let operator = parse_reduction_operator(op_text.trim())?;
                let items = variables
                    .iter()
                    .map(|item| ClauseItem::Identifier(Identifier::new(item.as_ref())))
                    .collect();
                Ok(ClauseData::Reduction {
                    modifiers: modifiers.iter().map(|m| (*m).into()).collect(),
                    operator,
                    user_identifier: user_defined_identifier
                        .as_ref()
                        .map(|id| Identifier::new(id.as_ref())),
                    items,
                    space_after_colon: *space_after_colon,
                })
            }
            // Keep unhandled structured clause kinds as a hard error to avoid
            // silently dropping semantic data.
            other => Err(ConversionError::UnknownClause(format!(
                "{clause_name} ({other:?})"
            ))),
        },
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

/// Convert a structured OpenMP directive AST into DirectiveIR.
pub fn convert_from_omp_ast(
    directive: &OmpDirective,
    location: SourceLocation,
    language: Language,
) -> Result<DirectiveIR, ConversionError> {
    let directive_name: DirectiveName = directive.kind.into();
    let kind = parse_directive_kind(directive_name)?;
    let clauses = directive
        .clauses
        .iter()
        .map(|clause| clause.payload.clone())
        .collect();

    Ok(DirectiveIR::new(
        kind,
        directive.kind.as_str(),
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
    use crate::ast::{
        ClauseNormalizationMode, DirectiveBody, OmpClause, OmpClauseKind, OmpDirective,
        OmpDirectiveKind,
    };

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
    fn convert_from_omp_ast_parallel_nowait() {
        let directive = OmpDirective {
            kind: OmpDirectiveKind::Parallel,
            parameter: None,
            clauses: vec![OmpClause {
                kind: OmpClauseKind::Nowait,
                payload: ClauseData::Bare(Identifier::new("nowait")),
            }],
        };

        let ir = convert_from_omp_ast(&directive, SourceLocation::start(), Language::C)
            .expect("conversion should succeed");

        assert!(ir.kind().is_parallel());
        assert_eq!(ir.clauses().len(), 1);
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
    fn test_parse_clause_data_uniform_list() {
        let clause = Clause {
            name: "uniform".into(),
            kind: ClauseKind::Parenthesized("*a, &b".into()),
        };
        let config = ParserConfig::with_parsing(Language::C);
        let data = parse_clause_data(&clause, &config).unwrap();
        match data {
            ClauseData::ItemList(items) => {
                assert_eq!(items.len(), 2);
            }
            other => panic!("expected ItemList, got {:?}", other),
        }
    }

    #[test]
    fn parse_declare_simd_uniform_clause_preserves_items() {
        let parser = crate::parser::openmp::parser();
        let (_, directive) = parser
            .parse("#pragma omp declare simd uniform(*a,&b)")
            .expect("directive should parse");
        let clause = directive
            .clauses
            .iter()
            .find(|c| c.name.as_ref() == "uniform")
            .expect("uniform clause present");

        if let ClauseKind::Parenthesized(content) = &clause.kind {
            assert!(!content.is_empty());
        } else {
            panic!(
                "expected parenthesized uniform clause, got {:?}",
                clause.kind
            );
        }

        let config = ParserConfig::with_parsing(Language::C);
        let data = parse_clause_data(clause, &config).unwrap();
        match data {
            ClauseData::ItemList(items) => assert_eq!(items.len(), 2),
            other => panic!("expected ItemList, got {:?}", other),
        }
    }

    #[test]
    fn parse_ast_preserves_uniform_clause_items() {
        let parser = crate::parser::openmp::parser();
        let ast = parser
            .parse_ast(
                "#pragma omp declare simd uniform(*a,&b)",
                ClauseNormalizationMode::ParserParity,
                &ParserConfig::default(),
            )
            .expect("parse_ast should succeed");
        match ast.body {
            DirectiveBody::OpenMp(dir) => {
                let uniform_clause = dir
                    .clauses
                    .iter()
                    .find(|c| matches!(c.kind, OmpClauseKind::Uniform))
                    .expect("uniform clause present");
                match &uniform_clause.payload {
                    ClauseData::ItemList(items) => assert_eq!(items.len(), 2),
                    other => panic!("expected ItemList, got {:?}", other),
                }
            }
            _ => panic!("expected OpenMP AST"),
        }
    }

    #[test]
    fn atomic_clause_order_is_preserved() {
        let parser = crate::parser::openmp::parser();
        let ast = parser
            .parse_ast(
                "#pragma omp atomic read hint(abc) seq_cst",
                ClauseNormalizationMode::ParserParity,
                &ParserConfig::default(),
            )
            .expect("parse_ast should succeed");
        match ast.body {
            DirectiveBody::OpenMp(dir) => {
                let kinds: Vec<_> = dir.clauses.iter().map(|c| c.kind).collect();
                assert_eq!(kinds, vec![OmpClauseKind::Hint, OmpClauseKind::SeqCst]);
            }
            _ => panic!("expected OpenMP AST"),
        }
    }

    #[test]
    fn lastprivate_modifier_and_items_preserved() {
        let parser = crate::parser::openmp::parser();
        let ast = parser
            .parse_ast(
                "#pragma omp for lastprivate(conditional:a,b,c)",
                ClauseNormalizationMode::ParserParity,
                &ParserConfig::default(),
            )
            .expect("parse_ast should succeed");
        match ast.body {
            DirectiveBody::OpenMp(dir) => {
                let lp = dir
                    .clauses
                    .iter()
                    .find(|c| matches!(c.kind, OmpClauseKind::Lastprivate))
                    .expect("lastprivate clause present");
                match &lp.payload {
                    ClauseData::Lastprivate { modifier, items } => {
                        assert_eq!(*modifier, Some(LastprivateModifier::Conditional));
                        assert_eq!(items.len(), 3);
                    }
                    other => panic!("unexpected payload: {:?}", other),
                }
            }
            _ => panic!("expected OpenMP AST"),
        }
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
        assert_eq!(
            parse_reduction_operator("unknown").unwrap(),
            ReductionOperator::Custom
        );
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
        if let ClauseData::Reduction {
            operator, items, ..
        } = data
        {
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
        if let ClauseData::Reduction {
            operator, items, ..
        } = data
        {
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
        if let ClauseData::Reduction {
            operator, items, ..
        } = data
        {
            assert_eq!(operator, ReductionOperator::Min);
            assert_eq!(items.len(), 1);
        } else {
            panic!("Expected Reduction clause");
        }
    }

    #[test]
    fn reduction_preserves_user_defined_operator_and_modifiers() {
        let parser = crate::parser::openmp::parser();
        let ast = parser
            .parse_ast(
                "#pragma omp parallel reduction(abc: x, y) reduction(task, user_defined: a, b)",
                ClauseNormalizationMode::ParserParity,
                &ParserConfig::default(),
            )
            .expect("parse_ast should succeed");

        match ast.body {
            DirectiveBody::OpenMp(dir) => {
                println!(
                    "clause kinds: {:?}",
                    dir.clauses.iter().map(|c| c.kind).collect::<Vec<_>>()
                );
                let reductions: Vec<_> = dir
                    .clauses
                    .iter()
                    .filter_map(|c| {
                        if let ClauseData::Reduction {
                            modifiers,
                            operator,
                            user_identifier,
                            items,
                            ..
                        } = &c.payload
                        {
                            Some((
                                modifiers.clone(),
                                *operator,
                                user_identifier.clone(),
                                items.len(),
                            ))
                        } else {
                            None
                        }
                    })
                    .collect();
                assert_eq!(reductions.len(), 2);

                assert_eq!(reductions[0].1, ReductionOperator::Custom);
                assert_eq!(reductions[0].2.as_ref().map(|id| id.name()), Some("abc"));

                assert_eq!(reductions[1].0, vec![ReductionModifier::Task]);
                assert_eq!(reductions[1].1, ReductionOperator::Custom);
                assert_eq!(
                    reductions[1].2.as_ref().map(|id| id.name()),
                    Some("user_defined")
                );
            }
            _ => panic!("expected OpenMP directive"),
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
            ..
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
            ..
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
            ..
        } = data
        {
            assert_eq!(map_type, Some(MapType::ToFrom));
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
            ..
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
        if let ClauseData::Depend {
            depend_type,
            items,
            iterators,
        } = data
        {
            assert_eq!(depend_type, DependType::In);
            assert_eq!(items.len(), 2);
            assert!(iterators.is_empty());
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
