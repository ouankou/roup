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
//!                 Clause { name: "reduction", kind: Parenthesized("+: sum") }
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
    ClauseData, ClauseItem, DefaultKind, DirectiveIR, DirectiveKind, Expression, Identifier,
    Language, ParserConfig, SourceLocation,
};
use crate::parser::{Clause, ClauseKind, Directive};

/// Error type for conversion failures
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversionError {
    /// Unknown directive name
    UnknownDirective(String),
    /// Unknown clause name
    UnknownClause(String),
    /// Invalid clause syntax
    InvalidClauseSyntax(String),
    /// Unsupported feature
    Unsupported(String),
}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConversionError::UnknownDirective(name) => {
                write!(f, "Unknown directive: {}", name)
            }
            ConversionError::UnknownClause(name) => {
                write!(f, "Unknown clause: {}", name)
            }
            ConversionError::InvalidClauseSyntax(msg) => {
                write!(f, "Invalid clause syntax: {}", msg)
            }
            ConversionError::Unsupported(msg) => {
                write!(f, "Unsupported feature: {}", msg)
            }
        }
    }
}

impl std::error::Error for ConversionError {}

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
        "parallel for simd" => Ok(DirectiveKind::ParallelForSimd),
        "parallel sections" => Ok(DirectiveKind::ParallelSections),
        "parallel workshare" => Ok(DirectiveKind::ParallelWorkshare),
        "parallel loop" => Ok(DirectiveKind::ParallelLoop),
        "parallel masked" => Ok(DirectiveKind::ParallelMasked),
        "parallel master" => Ok(DirectiveKind::ParallelMaster),

        // Work-sharing constructs
        "for" => Ok(DirectiveKind::For),
        "for simd" => Ok(DirectiveKind::ForSimd),
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
        "taskyield" => Ok(DirectiveKind::Taskyield),
        "taskwait" => Ok(DirectiveKind::Taskwait),
        "taskgroup" => Ok(DirectiveKind::Taskgroup),

        // Target constructs
        "target" => Ok(DirectiveKind::Target),
        "target data" => Ok(DirectiveKind::TargetData),
        "target enter data" => Ok(DirectiveKind::TargetEnterData),
        "target exit data" => Ok(DirectiveKind::TargetExitData),
        "target update" => Ok(DirectiveKind::TargetUpdate),
        "target parallel" => Ok(DirectiveKind::TargetParallel),
        "target parallel for" => Ok(DirectiveKind::TargetParallelFor),
        "target parallel for simd" => Ok(DirectiveKind::TargetParallelForSimd),
        "target parallel loop" => Ok(DirectiveKind::TargetParallelLoop),
        "target simd" => Ok(DirectiveKind::TargetSimd),
        "target teams" => Ok(DirectiveKind::TargetTeams),
        "target teams distribute" => Ok(DirectiveKind::TargetTeamsDistribute),
        "target teams distribute simd" => Ok(DirectiveKind::TargetTeamsDistributeSimd),
        "target teams distribute parallel for" => {
            Ok(DirectiveKind::TargetTeamsDistributeParallelFor)
        }
        "target teams distribute parallel for simd" => {
            Ok(DirectiveKind::TargetTeamsDistributeParallelForSimd)
        }
        "target teams loop" => Ok(DirectiveKind::TargetTeamsLoop),

        // Teams constructs
        "teams" => Ok(DirectiveKind::Teams),
        "teams distribute" => Ok(DirectiveKind::TeamsDistribute),
        "teams distribute simd" => Ok(DirectiveKind::TeamsDistributeSimd),
        "teams distribute parallel for" => Ok(DirectiveKind::TeamsDistributeParallelFor),
        "teams distribute parallel for simd" => Ok(DirectiveKind::TeamsDistributeParallelForSimd),
        "teams loop" => Ok(DirectiveKind::TeamsLoop),

        // Synchronization constructs
        "barrier" => Ok(DirectiveKind::Barrier),
        "critical" => Ok(DirectiveKind::Critical),
        "atomic" => Ok(DirectiveKind::Atomic),
        "flush" => Ok(DirectiveKind::Flush),
        "ordered" => Ok(DirectiveKind::Ordered),
        "master" => Ok(DirectiveKind::Master),
        "masked" => Ok(DirectiveKind::Masked),

        // Declare constructs
        "declare reduction" => Ok(DirectiveKind::DeclareReduction),
        "declare mapper" => Ok(DirectiveKind::DeclareMapper),
        "declare target" => Ok(DirectiveKind::DeclareTarget),
        "declare variant" => Ok(DirectiveKind::DeclareVariant),

        // Distribute constructs
        "distribute" => Ok(DirectiveKind::Distribute),
        "distribute simd" => Ok(DirectiveKind::DistributeSimd),
        "distribute parallel for" => Ok(DirectiveKind::DistributeParallelFor),
        "distribute parallel for simd" => Ok(DirectiveKind::DistributeParallelForSimd),

        // Meta-directives
        "metadirective" => Ok(DirectiveKind::Metadirective),

        // Other constructs
        "threadprivate" => Ok(DirectiveKind::Threadprivate),
        "allocate" => Ok(DirectiveKind::Allocate),
        "requires" => Ok(DirectiveKind::Requires),
        "scan" => Ok(DirectiveKind::Scan),
        "depobj" => Ok(DirectiveKind::Depobj),
        "nothing" => Ok(DirectiveKind::Nothing),
        "error" => Ok(DirectiveKind::Error),

        _ => Err(ConversionError::UnknownDirective(name.to_string())),
    }
}

/// Parse a simple identifier list from a string
///
/// Used for clauses like `private(x, y, z)`
///
/// ## Example
///
/// ```
/// # use roup::ir::convert::parse_identifier_list;
/// let items = parse_identifier_list("x, y, z");
/// assert_eq!(items.len(), 3);
/// ```
pub fn parse_identifier_list(content: &str) -> Vec<ClauseItem<'_>> {
    content
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| ClauseItem::Identifier(Identifier::new(s)))
        .collect()
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
    clause: &Clause<'a>,
    _config: &ParserConfig,
) -> Result<ClauseData<'a>, ConversionError> {
    match clause.name {
        // Bare clauses (no parameters)
        "nowait" | "nogroup" | "untied" | "mergeable" | "seq_cst" | "relaxed" | "release"
        | "acquire" | "acq_rel" => Ok(ClauseData::Bare(Identifier::new(clause.name))),

        // default(kind)
        "default" => {
            if let ClauseKind::Parenthesized(content) = clause.kind {
                let kind_str = content.trim();
                let kind = match kind_str {
                    "shared" => DefaultKind::Shared,
                    "none" => DefaultKind::None,
                    "private" => DefaultKind::Private,
                    "firstprivate" => DefaultKind::Firstprivate,
                    _ => {
                        return Err(ConversionError::InvalidClauseSyntax(format!(
                            "Unknown default kind: {}",
                            kind_str
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
            if let ClauseKind::Parenthesized(content) = clause.kind {
                let items = parse_identifier_list(content);
                Ok(ClauseData::Private { items })
            } else {
                Ok(ClauseData::Private { items: vec![] })
            }
        }

        // firstprivate(list)
        "firstprivate" => {
            if let ClauseKind::Parenthesized(content) = clause.kind {
                let items = parse_identifier_list(content);
                Ok(ClauseData::Firstprivate { items })
            } else {
                Ok(ClauseData::Firstprivate { items: vec![] })
            }
        }

        // shared(list)
        "shared" => {
            if let ClauseKind::Parenthesized(content) = clause.kind {
                let items = parse_identifier_list(content);
                Ok(ClauseData::Shared { items })
            } else {
                Ok(ClauseData::Shared { items: vec![] })
            }
        }

        // num_threads(expr)
        "num_threads" => {
            if let ClauseKind::Parenthesized(content) = clause.kind {
                Ok(ClauseData::NumThreads {
                    num: Expression::unparsed(content.trim()),
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "num_threads requires expression".to_string(),
                ))
            }
        }

        // if(expr)
        "if" => {
            if let ClauseKind::Parenthesized(content) = clause.kind {
                // Check for directive-name modifier: "if(parallel: condition)"
                if let Some(colon_pos) = content.find(':') {
                    let (modifier, condition) = content.split_at(colon_pos);
                    let condition = &condition[1..].trim(); // Skip the ':'
                    Ok(ClauseData::If {
                        directive_name: Some(Identifier::new(modifier.trim())),
                        condition: Expression::unparsed(condition),
                    })
                } else {
                    Ok(ClauseData::If {
                        directive_name: None,
                        condition: Expression::unparsed(content.trim()),
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
            if let ClauseKind::Parenthesized(content) = clause.kind {
                Ok(ClauseData::Collapse {
                    n: Expression::unparsed(content.trim()),
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
            ClauseKind::Parenthesized(content) => Ok(ClauseData::Ordered {
                n: Some(Expression::unparsed(content.trim())),
            }),
        },

        // For unsupported clauses, return a generic representation
        _ => Ok(ClauseData::Generic {
            name: Identifier::new(clause.name),
            data: match clause.kind {
                ClauseKind::Bare => None,
                ClauseKind::Parenthesized(content) => Some(content),
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
///     name: "parallel",
///     clauses: vec![
///         Clause { name: "default", kind: ClauseKind::Parenthesized("shared") },
///     ],
/// };
///
/// let config = ParserConfig::default();
/// let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config).unwrap();
/// assert!(ir.kind().is_parallel());
/// ```
pub fn convert_directive<'a>(
    directive: &Directive<'a>,
    location: SourceLocation,
    language: Language,
    config: &ParserConfig,
) -> Result<DirectiveIR<'a>, ConversionError> {
    // Convert directive kind
    let kind = parse_directive_kind(directive.name)?;

    // Convert clauses
    let mut clauses = Vec::new();
    for clause in &directive.clauses {
        let clause_data = parse_clause_data(clause, config)?;
        clauses.push(clause_data);
    }

    Ok(DirectiveIR::new(kind, clauses, location, language))
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
        let items = parse_identifier_list("x");
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_parse_identifier_list_multiple() {
        let items = parse_identifier_list("x, y, z");
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn test_parse_identifier_list_with_spaces() {
        let items = parse_identifier_list("  x  ,  y  ,  z  ");
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn test_parse_identifier_list_empty() {
        let items = parse_identifier_list("");
        assert_eq!(items.len(), 0);
    }

    #[test]
    fn test_parse_clause_data_bare() {
        let clause = Clause {
            name: "nowait",
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
            name: "default",
            kind: ClauseKind::Parenthesized("shared"),
        };
        let config = ParserConfig::default();
        let data = parse_clause_data(&clause, &config).unwrap();
        assert_eq!(data, ClauseData::Default(DefaultKind::Shared));
    }

    #[test]
    fn test_parse_clause_data_private() {
        let clause = Clause {
            name: "private",
            kind: ClauseKind::Parenthesized("x, y"),
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
            name: "num_threads",
            kind: ClauseKind::Parenthesized("4"),
        };
        let config = ParserConfig::default();
        let data = parse_clause_data(&clause, &config).unwrap();
        assert!(matches!(data, ClauseData::NumThreads { .. }));
    }

    #[test]
    fn test_parse_clause_data_if_simple() {
        let clause = Clause {
            name: "if",
            kind: ClauseKind::Parenthesized("n > 100"),
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
            name: "if",
            kind: ClauseKind::Parenthesized("parallel: n > 100"),
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
            name: "parallel",
            clauses: vec![],
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
            name: "parallel",
            clauses: vec![
                Clause {
                    name: "default",
                    kind: ClauseKind::Parenthesized("shared"),
                },
                Clause {
                    name: "private",
                    kind: ClauseKind::Parenthesized("x"),
                },
            ],
        };
        let config = ParserConfig::default();
        let ir =
            convert_directive(&directive, SourceLocation::start(), Language::C, &config).unwrap();
        assert_eq!(ir.kind(), DirectiveKind::Parallel);
        assert_eq!(ir.clauses().len(), 2);
    }
}
