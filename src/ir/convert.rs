//! Conversion from parser types to IR types
//!
//! This module handles the conversion from the parser's textual
//! representation to the IR's semantic representation.
//!
//! ## Learning Objectives
//!
//! - **Pattern matching on enums**: Mapping clause names to semantic types
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
//!                 Clause { separator: crate::parser::ClauseSeparator::Space, name: "private", kind: Parenthesized("x, y") },
//!                 Clause { separator: crate::parser::ClauseSeparator::Space, name: "reduction".into(), kind: Parenthesized("+: sum") }
//!               ] }
//!
//! IR output:
//!   DirectiveIR {
//!     kind: DirectiveKind::ParallelFor,
//!     clauses: [
//!       ClauseData::Private { items: [Identifier("x"), Identifier("y")] },
//!       ClauseData::Reduction { modifier_items: Vec::new(), operator: Add, items: [Identifier("sum")] }
//!     ],
//!     ...
//!   }
//! ```

use super::{
    lang, ClauseData, ClauseItem, ConversionError, DependType, DirectiveIR, DirectiveKind,
    Language, ParserConfig, ReductionModifier, ReductionOperator, SourceLocation,
};
use crate::ast::OmpDirective;
use crate::parser::{
    clause::ReductionModifier as ParserReductionModifier, directive_kind::DirectiveName, Clause,
    Directive,
};

impl From<ParserReductionModifier> for ReductionModifier {
    fn from(value: ParserReductionModifier) -> Self {
        match value {
            ParserReductionModifier::Task => ReductionModifier::Task,
            ParserReductionModifier::Inscan => ReductionModifier::Inscan,
            ParserReductionModifier::Default => ReductionModifier::Default,
            ParserReductionModifier::Original => ReductionModifier::Original,
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
        DirectiveName::ParallelDoCompact => Ok(DirectiveKind::ParallelDo),
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
        DirectiveName::TargetDataUnderscore => Ok(DirectiveKind::TargetData),
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
        DirectiveName::TargetTeamsWorkdistribute => Ok(DirectiveKind::TargetTeamsWorkdistribute),

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
        DirectiveName::DeclareTarget | DirectiveName::DeclareTargetUnderscore => {
            Ok(DirectiveKind::DeclareTarget)
        }
        DirectiveName::BeginDeclareTarget | DirectiveName::BeginDeclareTargetUnderscore => {
            Ok(DirectiveKind::BeginDeclareTarget)
        }
        DirectiveName::EndDeclareTarget | DirectiveName::EndDeclareTargetUnderscore => {
            Ok(DirectiveKind::EndDeclareTarget)
        }
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
        DirectiveName::Scope | DirectiveName::EndScope => Ok(DirectiveKind::Scope),
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
/// Compatibility wrapper around parser-boundary semantic parsing.
pub fn parse_reduction_operator(op_str: &str) -> Result<ReductionOperator, ConversionError> {
    crate::parser::semantic::parse_reduction_operator(op_str)
}

/// Parse a schedule clause.
///
/// Compatibility wrapper around parser-boundary semantic parsing.
pub fn parse_schedule_clause(
    content: &str,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    crate::parser::semantic::parse_schedule_clause(content, config)
}

/// Parse a map clause.
///
/// Compatibility wrapper around parser-boundary semantic parsing.
pub fn parse_map_clause(
    content: &str,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    crate::parser::semantic::parse_map_clause(content, config)
}

/// Parse a dependence type from a string.
///
/// Compatibility wrapper around parser-boundary semantic parsing.
pub fn parse_depend_type(type_str: &str) -> Result<DependType, ConversionError> {
    crate::parser::semantic::parse_depend_type(type_str)
}

/// Parse a linear clause.
///
/// Compatibility wrapper around parser-boundary semantic parsing.
pub fn parse_linear_clause(
    content: &str,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    crate::parser::semantic::parse_linear_clause(content, config)
}

pub fn parse_clause_data<'a>(
    clause: &Clause<'a>,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    crate::parser::semantic::parse_clause_data(clause, config)
}

/// Convert a parser Directive to IR DirectiveIR
///
/// ## Example
///
/// ```
/// # use roup::parser::{Clause, ClauseKind, ClauseSeparator, Directive};
/// # use roup::ir::{convert::convert_directive, Language, SourceLocation, ParserConfig};
/// let directive = Directive {
///     name: "parallel".into(),
///     parameter: None,
///     clauses: vec![
///         Clause {
///             separator: ClauseSeparator::Space,
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
    use crate::ir::{
        DefaultKind, IfModifier, LastprivateModifier, MapType, ProcBind, ScheduleKind,
        ScheduleModifier,
    };
    use crate::parser::{lookup_clause_name, ClauseKind, ClauseName};

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
                payload: ClauseData::Nowait { modifier: None },
                separator: crate::ast::OmpClauseSeparator::Space,
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
            separator: crate::parser::ClauseSeparator::Space,
            name: "nowait".into(),
            kind: ClauseKind::Bare,
        };
        let config = ParserConfig::default();
        let data = parse_clause_data(&clause, &config).unwrap();
        assert!(matches!(data, ClauseData::Nowait { modifier: None }));
        assert_eq!(data.to_string(), "nowait");
    }

    #[test]
    fn test_parse_clause_data_uniform_list() {
        let clause = Clause {
            separator: crate::parser::ClauseSeparator::Space,
            name: "uniform".into(),
            kind: ClauseKind::Parenthesized("*a, &b".into()),
        };
        let config = ParserConfig::with_parsing(Language::C);
        let data = parse_clause_data(&clause, &config).unwrap();
        match data {
            ClauseData::ItemList(items) => {
                assert_eq!(items.len(), 2);
            }
            other => panic!("expected ItemList, got {other:?}"),
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
            .find(|c| lookup_clause_name(c.name.as_ref()) == ClauseName::Uniform)
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
            other => panic!("expected ItemList, got {other:?}"),
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
                    other => panic!("expected ItemList, got {other:?}"),
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
                    other => panic!("unexpected payload: {other:?}"),
                }
            }
            _ => panic!("expected OpenMP AST"),
        }
    }

    #[test]
    fn test_parse_clause_data_default_shared() {
        let clause = Clause {
            separator: crate::parser::ClauseSeparator::Space,
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
            separator: crate::parser::ClauseSeparator::Space,
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
            separator: crate::parser::ClauseSeparator::Space,
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
            separator: crate::parser::ClauseSeparator::Space,
            name: "if".into(),
            kind: ClauseKind::Parenthesized("n > 100".into()),
        };
        let config = ParserConfig::default();
        let data = parse_clause_data(&clause, &config).unwrap();
        if let ClauseData::If {
            modifier,
            condition,
        } = data
        {
            assert!(modifier.is_none());
            assert_eq!(condition.to_string(), "n > 100");
        } else {
            panic!("Expected If clause");
        }
    }

    #[test]
    fn test_parse_clause_data_if_with_modifier() {
        let clause = Clause {
            separator: crate::parser::ClauseSeparator::Space,
            name: "if".into(),
            kind: ClauseKind::Parenthesized("parallel: n > 100".into()),
        };
        let config = ParserConfig::default();
        let data = parse_clause_data(&clause, &config).unwrap();
        if let ClauseData::If {
            modifier,
            condition,
        } = data
        {
            assert_eq!(modifier, Some(IfModifier::Parallel));
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
                    separator: crate::parser::ClauseSeparator::Space,
                    name: "default".into(),
                    kind: ClauseKind::Parenthesized("shared".into()),
                },
                Clause {
                    separator: crate::parser::ClauseSeparator::Space,
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
            separator: crate::parser::ClauseSeparator::Space,
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
            separator: crate::parser::ClauseSeparator::Space,
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
            separator: crate::parser::ClauseSeparator::Space,
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
            assert_eq!(map_type, None);
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
            separator: crate::parser::ClauseSeparator::Space,
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
            separator: crate::parser::ClauseSeparator::Space,
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
            separator: crate::parser::ClauseSeparator::Space,
            name: "proc_bind".into(),
            kind: ClauseKind::Parenthesized("close".into()),
        };
        let config = ParserConfig::default();
        let data = parse_clause_data(&clause, &config).unwrap();
        assert_eq!(data, ClauseData::ProcBind(ProcBind::Close));
    }
}
