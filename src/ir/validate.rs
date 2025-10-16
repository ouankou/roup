//! IR validation utilities
//!
//! This module provides validation for OpenMP directives and clauses,
//! ensuring semantic correctness beyond just syntax.
//!
//! ## Learning Objectives
//!
//! - **Context-sensitive validation**: Rules depend on directive type
//! - **Semantic checking**: Beyond syntax, check meaning and compatibility
//! - **Error reporting**: Clear, actionable error messages
//! - **Builder pattern**: Fluent API for constructing valid IR
//!
//! ## Validation Levels
//!
//! 1. **Syntax validation**: Already handled by parser
//! 2. **Structural validation**: Clause exists, has required parts
//! 3. **Semantic validation**: Clause allowed for this directive
//! 4. **Consistency validation**: Clauses don't conflict with each other
//!
//! ## Example
//!
//! ```
//! use roup::ir::{DirectiveKind, ClauseData, Identifier, ValidationContext};
//!
//! let context = ValidationContext::new(DirectiveKind::For);
//! assert!(context.is_clause_allowed(&ClauseData::Bare(Identifier::new("nowait"))).is_ok());
//! ```

use super::{ClauseData, DirectiveIR, DirectiveKind};
use std::fmt;

/// Validation error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// Clause not allowed on this directive
    ClauseNotAllowed {
        clause_name: String,
        directive: String,
        reason: String,
    },
    /// Conflicting clauses
    ConflictingClauses {
        clause1: String,
        clause2: String,
        reason: String,
    },
    /// Missing required clause
    MissingRequiredClause {
        directive: String,
        required_clause: String,
    },
    /// Invalid clause combination
    InvalidCombination {
        clauses: Vec<String>,
        reason: String,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::ClauseNotAllowed {
                clause_name,
                directive,
                reason,
            } => {
                write!(
                    f,
                    "Clause '{clause_name}' not allowed on '{directive}' directive: {reason}"
                )
            }
            ValidationError::ConflictingClauses {
                clause1,
                clause2,
                reason,
            } => {
                write!(
                    f,
                    "Conflicting clauses '{clause1}' and '{clause2}': {reason}"
                )
            }
            ValidationError::MissingRequiredClause {
                directive,
                required_clause,
            } => {
                write!(
                    f,
                    "Directive '{directive}' requires clause '{required_clause}'"
                )
            }
            ValidationError::InvalidCombination { clauses, reason } => {
                write!(
                    f,
                    "Invalid combination of clauses [{}]: {}",
                    clauses.join(", "),
                    reason
                )
            }
        }
    }
}

impl std::error::Error for ValidationError {}

/// Validation context for checking clause compatibility
pub struct ValidationContext {
    directive: DirectiveKind,
}

impl ValidationContext {
    /// Create a new validation context for a directive
    pub fn new(directive: DirectiveKind) -> Self {
        Self { directive }
    }

    /// Check if a clause is allowed on this directive
    pub fn is_clause_allowed(&self, clause: &ClauseData) -> Result<(), ValidationError> {
        // Get clause name for error reporting
        let clause_name = self.clause_name(clause);

        match clause {
            // nowait is only for worksharing, not parallel
            ClauseData::Bare(name) if name.to_string() == "nowait" => {
                if self.directive.is_worksharing() || self.directive == DirectiveKind::Target {
                    Ok(())
                } else {
                    Err(ValidationError::ClauseNotAllowed {
                        clause_name,
                        directive: self.directive.to_string(),
                        reason: "nowait only allowed on worksharing constructs (for, sections, single) or target".to_string(),
                    })
                }
            }

            // reduction requires parallel or worksharing or simd
            ClauseData::Reduction { .. } => {
                if self.directive.is_parallel()
                    || self.directive.is_worksharing()
                    || self.directive.is_simd()
                    || self.directive.is_teams()
                {
                    Ok(())
                } else {
                    Err(ValidationError::ClauseNotAllowed {
                        clause_name,
                        directive: self.directive.to_string(),
                        reason: "reduction requires parallel, worksharing, simd, or teams context"
                            .to_string(),
                    })
                }
            }

            // schedule is only for loop constructs
            ClauseData::Schedule { .. } => {
                if self.directive.is_loop() || self.directive.is_worksharing() {
                    Ok(())
                } else {
                    Err(ValidationError::ClauseNotAllowed {
                        clause_name,
                        directive: self.directive.to_string(),
                        reason:
                            "schedule only allowed on loop constructs (for, parallel for, etc.)"
                                .to_string(),
                    })
                }
            }

            // num_threads is for parallel constructs
            ClauseData::NumThreads { .. } => {
                if self.directive.is_parallel() {
                    Ok(())
                } else {
                    Err(ValidationError::ClauseNotAllowed {
                        clause_name,
                        directive: self.directive.to_string(),
                        reason: "num_threads only allowed on parallel constructs".to_string(),
                    })
                }
            }

            // map is for target constructs
            ClauseData::Map { .. } => {
                if self.directive.is_target() {
                    Ok(())
                } else {
                    Err(ValidationError::ClauseNotAllowed {
                        clause_name,
                        directive: self.directive.to_string(),
                        reason: "map only allowed on target constructs".to_string(),
                    })
                }
            }

            // depend is for task constructs
            ClauseData::Depend { .. } => {
                if self.directive.is_task() || self.directive == DirectiveKind::Ordered {
                    Ok(())
                } else {
                    Err(ValidationError::ClauseNotAllowed {
                        clause_name,
                        directive: self.directive.to_string(),
                        reason: "depend only allowed on task constructs or ordered".to_string(),
                    })
                }
            }

            // linear is for simd/loop constructs
            ClauseData::Linear { .. } => {
                if self.directive.is_simd() || self.directive.is_loop() {
                    Ok(())
                } else {
                    Err(ValidationError::ClauseNotAllowed {
                        clause_name,
                        directive: self.directive.to_string(),
                        reason: "linear only allowed on simd or loop constructs".to_string(),
                    })
                }
            }

            // collapse is for loop constructs
            ClauseData::Collapse { .. } => {
                if self.directive.is_loop() || self.directive.is_worksharing() {
                    Ok(())
                } else {
                    Err(ValidationError::ClauseNotAllowed {
                        clause_name,
                        directive: self.directive.to_string(),
                        reason: "collapse only allowed on loop constructs".to_string(),
                    })
                }
            }

            // ordered is for loop constructs
            ClauseData::Ordered { .. } => {
                if self.directive.is_loop() || self.directive.is_worksharing() {
                    Ok(())
                } else {
                    Err(ValidationError::ClauseNotAllowed {
                        clause_name,
                        directive: self.directive.to_string(),
                        reason: "ordered only allowed on loop constructs".to_string(),
                    })
                }
            }

            // proc_bind is for parallel constructs
            ClauseData::ProcBind(_) => {
                if self.directive.is_parallel() {
                    Ok(())
                } else {
                    Err(ValidationError::ClauseNotAllowed {
                        clause_name,
                        directive: self.directive.to_string(),
                        reason: "proc_bind only allowed on parallel constructs".to_string(),
                    })
                }
            }

            // Data-sharing clauses (private, shared, etc.) allowed on most constructs
            ClauseData::Private { .. }
            | ClauseData::Firstprivate { .. }
            | ClauseData::Lastprivate { .. }
            | ClauseData::Shared { .. } => Ok(()),

            // Default clause for parallel and task
            ClauseData::Default(_) => {
                if self.directive.is_parallel() || self.directive.is_task() {
                    Ok(())
                } else {
                    Err(ValidationError::ClauseNotAllowed {
                        clause_name,
                        directive: self.directive.to_string(),
                        reason: "default only allowed on parallel or task constructs".to_string(),
                    })
                }
            }

            // if clause allowed on most constructs
            ClauseData::If { .. } => Ok(()),

            // Generic clauses we don't validate yet
            ClauseData::Generic { .. } => Ok(()),

            // Other clauses default to allowed
            _ => Ok(()),
        }
    }

    /// Get a displayable name for a clause
    fn clause_name(&self, clause: &ClauseData) -> String {
        match clause {
            ClauseData::Bare(name) => name.to_string(),
            ClauseData::Private { .. } => "private".to_string(),
            ClauseData::Firstprivate { .. } => "firstprivate".to_string(),
            ClauseData::Lastprivate { .. } => "lastprivate".to_string(),
            ClauseData::Shared { .. } => "shared".to_string(),
            ClauseData::Default(_) => "default".to_string(),
            ClauseData::Reduction { .. } => "reduction".to_string(),
            ClauseData::Map { .. } => "map".to_string(),
            ClauseData::Schedule { .. } => "schedule".to_string(),
            ClauseData::Linear { .. } => "linear".to_string(),
            ClauseData::If { .. } => "if".to_string(),
            ClauseData::NumThreads { .. } => "num_threads".to_string(),
            ClauseData::ProcBind(_) => "proc_bind".to_string(),
            ClauseData::Collapse { .. } => "collapse".to_string(),
            ClauseData::Ordered { .. } => "ordered".to_string(),
            ClauseData::Depend { .. } => "depend".to_string(),
            ClauseData::Generic { name, .. } => name.to_string(),
            _ => "<unknown>".to_string(),
        }
    }

    /// Validate all clauses in a directive
    pub fn validate_all(&self, clauses: &[ClauseData]) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        // Check each clause individually
        for clause in clauses {
            if let Err(e) = self.is_clause_allowed(clause) {
                errors.push(e);
            }
        }

        // Check for conflicting clauses
        if let Err(mut conflicts) = self.check_conflicts(clauses) {
            errors.append(&mut conflicts);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Check for conflicting clauses
    fn check_conflicts(&self, clauses: &[ClauseData]) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        // Check for multiple default clauses
        let default_count = clauses.iter().filter(|c| c.is_default()).count();
        if default_count > 1 {
            errors.push(ValidationError::InvalidCombination {
                clauses: vec!["default".to_string(); default_count],
                reason: "only one default clause allowed".to_string(),
            });
        }

        // Check for multiple num_threads clauses
        let num_threads_count = clauses.iter().filter(|c| c.is_num_threads()).count();
        if num_threads_count > 1 {
            errors.push(ValidationError::InvalidCombination {
                clauses: vec!["num_threads".to_string(); num_threads_count],
                reason: "only one num_threads clause allowed".to_string(),
            });
        }

        // Check for multiple proc_bind clauses
        let proc_bind_count = clauses.iter().filter(|c| c.is_proc_bind()).count();
        if proc_bind_count > 1 {
            errors.push(ValidationError::InvalidCombination {
                clauses: vec!["proc_bind".to_string(); proc_bind_count],
                reason: "only one proc_bind clause allowed".to_string(),
            });
        }

        // Check for ordered and schedule(auto/runtime) conflict
        let has_ordered = clauses.iter().any(|c| c.is_ordered());
        let has_auto_runtime = clauses.iter().any(|c| {
            if let ClauseData::Schedule { kind, .. } = c {
                matches!(
                    kind,
                    super::ScheduleKind::Auto | super::ScheduleKind::Runtime
                )
            } else {
                false
            }
        });

        if has_ordered && has_auto_runtime {
            errors.push(ValidationError::ConflictingClauses {
                clause1: "ordered".to_string(),
                clause2: "schedule(auto/runtime)".to_string(),
                reason: "ordered not compatible with schedule(auto) or schedule(runtime)"
                    .to_string(),
            });
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl DirectiveIR {
    /// Validate this directive and its clauses
    ///
    /// ## Example
    ///
    /// ```
    /// use roup::ir::{DirectiveIR, DirectiveKind, ClauseData, DefaultKind, Language, SourceLocation};
    ///
    /// let ir = DirectiveIR::new(
    ///     DirectiveKind::Parallel,
    ///     "parallel",
    ///     vec![ClauseData::Default(DefaultKind::Shared)],
    ///     SourceLocation::start(),
    ///     Language::C,
    /// );
    ///
    /// // This will validate successfully
    /// assert!(ir.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), Vec<ValidationError>> {
        let context = ValidationContext::new(self.kind());
        context.validate_all(self.clauses())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{
        ClauseItem, DefaultKind, DependType, Identifier, Language, MapType, ReductionOperator,
        ScheduleKind, SourceLocation,
    };

    #[test]
    fn test_nowait_allowed_on_for() {
        let context = ValidationContext::new(DirectiveKind::For);
        let clause = ClauseData::Bare(Identifier::new("nowait"));
        assert!(context.is_clause_allowed(&clause).is_ok());
    }

    #[test]
    fn test_nowait_not_allowed_on_parallel() {
        let context = ValidationContext::new(DirectiveKind::Parallel);
        let clause = ClauseData::Bare(Identifier::new("nowait"));
        assert!(context.is_clause_allowed(&clause).is_err());
    }

    #[test]
    fn test_reduction_allowed_on_parallel() {
        let context = ValidationContext::new(DirectiveKind::Parallel);
        let clause = ClauseData::Reduction {
            operator: ReductionOperator::Add,
            items: vec![ClauseItem::Identifier(Identifier::new("sum"))],
        };
        assert!(context.is_clause_allowed(&clause).is_ok());
    }

    #[test]
    fn test_reduction_allowed_on_for() {
        let context = ValidationContext::new(DirectiveKind::For);
        let clause = ClauseData::Reduction {
            operator: ReductionOperator::Add,
            items: vec![ClauseItem::Identifier(Identifier::new("sum"))],
        };
        assert!(context.is_clause_allowed(&clause).is_ok());
    }

    #[test]
    fn test_schedule_allowed_on_for() {
        let context = ValidationContext::new(DirectiveKind::For);
        let clause = ClauseData::Schedule {
            kind: ScheduleKind::Static,
            modifiers: vec![],
            chunk_size: None,
        };
        assert!(context.is_clause_allowed(&clause).is_ok());
    }

    #[test]
    fn test_schedule_not_allowed_on_parallel() {
        let context = ValidationContext::new(DirectiveKind::Parallel);
        let clause = ClauseData::Schedule {
            kind: ScheduleKind::Static,
            modifiers: vec![],
            chunk_size: None,
        };
        assert!(context.is_clause_allowed(&clause).is_err());
    }

    #[test]
    fn test_num_threads_allowed_on_parallel() {
        let context = ValidationContext::new(DirectiveKind::Parallel);
        let clause = ClauseData::NumThreads {
            num: crate::ir::Expression::unparsed("4"),
        };
        assert!(context.is_clause_allowed(&clause).is_ok());
    }

    #[test]
    fn test_num_threads_not_allowed_on_for() {
        let context = ValidationContext::new(DirectiveKind::For);
        let clause = ClauseData::NumThreads {
            num: crate::ir::Expression::unparsed("4"),
        };
        assert!(context.is_clause_allowed(&clause).is_err());
    }

    #[test]
    fn test_map_allowed_on_target() {
        let context = ValidationContext::new(DirectiveKind::Target);
        let clause = ClauseData::Map {
            map_type: Some(MapType::To),
            mapper: None,
            items: vec![ClauseItem::Identifier(Identifier::new("arr"))],
        };
        assert!(context.is_clause_allowed(&clause).is_ok());
    }

    #[test]
    fn test_map_not_allowed_on_parallel() {
        let context = ValidationContext::new(DirectiveKind::Parallel);
        let clause = ClauseData::Map {
            map_type: Some(MapType::To),
            mapper: None,
            items: vec![ClauseItem::Identifier(Identifier::new("arr"))],
        };
        assert!(context.is_clause_allowed(&clause).is_err());
    }

    #[test]
    fn test_depend_allowed_on_task() {
        let context = ValidationContext::new(DirectiveKind::Task);
        let clause = ClauseData::Depend {
            depend_type: DependType::In,
            items: vec![ClauseItem::Identifier(Identifier::new("x"))],
        };
        assert!(context.is_clause_allowed(&clause).is_ok());
    }

    #[test]
    fn test_private_allowed_on_most_constructs() {
        let clause = ClauseData::Private {
            items: vec![ClauseItem::Identifier(Identifier::new("x"))],
        };

        assert!(ValidationContext::new(DirectiveKind::Parallel)
            .is_clause_allowed(&clause)
            .is_ok());
        assert!(ValidationContext::new(DirectiveKind::For)
            .is_clause_allowed(&clause)
            .is_ok());
        assert!(ValidationContext::new(DirectiveKind::Task)
            .is_clause_allowed(&clause)
            .is_ok());
    }

    #[test]
    fn test_multiple_default_clauses_conflict() {
        let context = ValidationContext::new(DirectiveKind::Parallel);
        let clauses = vec![
            ClauseData::Default(DefaultKind::Shared),
            ClauseData::Default(DefaultKind::None),
        ];

        let result = context.validate_all(&clauses);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0],
            ValidationError::InvalidCombination { .. }
        ));
    }

    #[test]
    fn test_ordered_schedule_auto_conflict() {
        let context = ValidationContext::new(DirectiveKind::For);
        let clauses = vec![
            ClauseData::Ordered { n: None },
            ClauseData::Schedule {
                kind: ScheduleKind::Auto,
                modifiers: vec![],
                chunk_size: None,
            },
        ];

        let result = context.validate_all(&clauses);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| matches!(e, ValidationError::ConflictingClauses { .. })));
    }

    #[test]
    fn test_directive_ir_validate() {
        let ir = DirectiveIR::new(
            DirectiveKind::Parallel,
            "parallel",
            vec![ClauseData::Default(DefaultKind::Shared)],
            SourceLocation::start(),
            Language::C,
        );

        assert!(ir.validate().is_ok());
    }

    #[test]
    fn test_directive_ir_validate_invalid() {
        let ir = DirectiveIR::new(
            DirectiveKind::Parallel,
            "parallel",
            vec![ClauseData::Bare(Identifier::new("nowait"))],
            SourceLocation::start(),
            Language::C,
        );

        assert!(ir.validate().is_err());
    }
}
