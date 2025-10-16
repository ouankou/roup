//! Builder pattern for constructing DirectiveIR
//!
//! This module provides a fluent API for building OpenMP directives
//! with compile-time validation and convenience methods.
//!
//! ## Learning Objectives
//!
//! - **Builder pattern**: Fluent API for object construction
//! - **Type-state pattern**: Compile-time validation of construction
//! - **Method chaining**: Ergonomic API design
//! - **Smart defaults**: Sensible default values
//!
//! ## Example
//!
//! ```
//! use roup::ir::{DirectiveBuilder, Language, SourceLocation};
//!
//! // Build a parallel directive with clauses
//! let directive = DirectiveBuilder::parallel()
//!     .default_shared()
//!     .private(&["x", "y"])
//!     .num_threads(4)
//!     .build(SourceLocation::start(), Language::C);
//!
//! assert!(directive.kind().is_parallel());
//! assert_eq!(directive.clauses().len(), 3);
//! ```

use super::{
    ClauseData, ClauseItem, DefaultKind, DependType, DirectiveIR, DirectiveKind, Expression,
    Identifier, Language, MapType, ProcBind, ReductionOperator, ScheduleKind, ScheduleModifier,
    SourceLocation,
};

/// Builder for constructing DirectiveIR with a fluent API
pub struct DirectiveBuilder {
    kind: DirectiveKind,
    name: String,
    clauses: Vec<ClauseData>,
}

impl<'a> DirectiveBuilder {
    /// Create a new builder for a parallel directive
    ///
    /// ## Example
    ///
    /// ```
    /// use roup::ir::{DirectiveBuilder, Language, SourceLocation};
    ///
    /// let directive = DirectiveBuilder::parallel()
    ///     .default_shared()
    ///     .build(SourceLocation::start(), Language::C);
    ///
    /// assert!(directive.kind().is_parallel());
    /// ```
    pub fn parallel() -> Self {
        Self {
            kind: DirectiveKind::Parallel,
            name: "parallel".to_string(),
            clauses: Vec::new(),
        }
    }

    /// Create a new builder for a parallel for directive
    pub fn parallel_for() -> Self {
        Self {
            kind: DirectiveKind::ParallelFor,
            name: "parallel for".to_string(),
            clauses: Vec::new(),
        }
    }

    /// Create a new builder for a for directive
    pub fn for_loop() -> Self {
        Self {
            kind: DirectiveKind::For,
            name: "for".to_string(),
            clauses: Vec::new(),
        }
    }

    /// Create a new builder for a task directive
    pub fn task() -> Self {
        Self {
            kind: DirectiveKind::Task,
            name: "task".to_string(),
            clauses: Vec::new(),
        }
    }

    /// Create a new builder for a target directive
    pub fn target() -> Self {
        Self {
            kind: DirectiveKind::Target,
            name: "target".to_string(),
            clauses: Vec::new(),
        }
    }

    /// Create a new builder for a teams directive
    pub fn teams() -> Self {
        Self {
            kind: DirectiveKind::Teams,
            name: "teams".to_string(),
            clauses: Vec::new(),
        }
    }

    /// Create a new builder for any directive kind
    pub fn new(kind: DirectiveKind) -> Self {
        let name = format!("{kind:?}").to_lowercase();
        Self {
            kind,
            name,
            clauses: Vec::new(),
        }
    }

    // ========================================================================
    // Clause builders
    // ========================================================================

    /// Add a default(shared) clause
    pub fn default_shared(mut self) -> Self {
        self.clauses.push(ClauseData::Default(DefaultKind::Shared));
        self
    }

    /// Add a default(none) clause
    pub fn default_none(mut self) -> Self {
        self.clauses.push(ClauseData::Default(DefaultKind::None));
        self
    }

    /// Add a default clause with specified kind
    pub fn default(mut self, kind: DefaultKind) -> Self {
        self.clauses.push(ClauseData::Default(kind));
        self
    }

    /// Add a private clause
    ///
    /// ## Example
    ///
    /// ```
    /// use roup::ir::{DirectiveBuilder, Language, SourceLocation};
    ///
    /// let directive = DirectiveBuilder::parallel()
    ///     .private(&["x", "y", "z"])
    ///     .build(SourceLocation::start(), Language::C);
    /// ```
    pub fn private(mut self, vars: &[&'a str]) -> Self {
        let items = vars
            .iter()
            .map(|&v| ClauseItem::Identifier(Identifier::new(v)))
            .collect();
        self.clauses.push(ClauseData::Private { items });
        self
    }

    /// Add a firstprivate clause
    pub fn firstprivate(mut self, vars: &[&'a str]) -> Self {
        let items = vars
            .iter()
            .map(|&v| ClauseItem::Identifier(Identifier::new(v)))
            .collect();
        self.clauses.push(ClauseData::Firstprivate { items });
        self
    }

    /// Add a shared clause
    pub fn shared(mut self, vars: &[&'a str]) -> Self {
        let items = vars
            .iter()
            .map(|&v| ClauseItem::Identifier(Identifier::new(v)))
            .collect();
        self.clauses.push(ClauseData::Shared { items });
        self
    }

    /// Add a reduction clause
    ///
    /// ## Example
    ///
    /// ```
    /// use roup::ir::{DirectiveBuilder, ReductionOperator, Language, SourceLocation};
    ///
    /// let directive = DirectiveBuilder::parallel()
    ///     .reduction(ReductionOperator::Add, &["sum"])
    ///     .build(SourceLocation::start(), Language::C);
    /// ```
    pub fn reduction(mut self, operator: ReductionOperator, vars: &[&'a str]) -> Self {
        let items = vars
            .iter()
            .map(|&v| ClauseItem::Identifier(Identifier::new(v)))
            .collect();
        self.clauses.push(ClauseData::Reduction { operator, items });
        self
    }

    /// Add a num_threads clause
    ///
    /// Note: This creates an unparsed expression. For better control,
    /// use `num_threads_expr()` with a static string.
    pub fn num_threads(self, num: i32) -> Self {
        // Note: For production use, consider requiring static strings
        // or storing owned strings. This is a convenience method.
        self.num_threads_expr(Box::leak(Box::new(num.to_string())))
    }

    /// Add a num_threads clause with expression
    pub fn num_threads_expr(mut self, expr: &'a str) -> Self {
        self.clauses.push(ClauseData::NumThreads {
            num: Expression::unparsed(expr),
        });
        self
    }

    /// Add an if clause
    pub fn if_clause(mut self, condition: &'a str) -> Self {
        self.clauses.push(ClauseData::If {
            directive_name: None,
            condition: Expression::unparsed(condition),
        });
        self
    }

    /// Add a schedule clause
    ///
    /// ## Example
    ///
    /// ```
    /// use roup::ir::{DirectiveBuilder, ScheduleKind, Language, SourceLocation};
    ///
    /// let directive = DirectiveBuilder::for_loop()
    ///     .schedule_simple(ScheduleKind::Static)
    ///     .build(SourceLocation::start(), Language::C);
    /// ```
    pub fn schedule_simple(mut self, kind: ScheduleKind) -> Self {
        self.clauses.push(ClauseData::Schedule {
            kind,
            modifiers: vec![],
            chunk_size: None,
        });
        self
    }

    /// Add a schedule clause with chunk size expression
    pub fn schedule(mut self, kind: ScheduleKind, chunk_size: Option<&'a str>) -> Self {
        self.clauses.push(ClauseData::Schedule {
            kind,
            modifiers: vec![],
            chunk_size: chunk_size.map(Expression::unparsed),
        });
        self
    }

    /// Add a schedule clause with modifiers
    pub fn schedule_with_modifiers(
        mut self,
        kind: ScheduleKind,
        modifiers: Vec<ScheduleModifier>,
        chunk_size: Option<&'a str>,
    ) -> Self {
        self.clauses.push(ClauseData::Schedule {
            kind,
            modifiers,
            chunk_size: chunk_size.map(Expression::unparsed),
        });
        self
    }

    /// Add a collapse clause with expression
    pub fn collapse(mut self, n: &'a str) -> Self {
        self.clauses.push(ClauseData::Collapse {
            n: Expression::unparsed(n),
        });
        self
    }

    /// Add an ordered clause
    pub fn ordered(mut self) -> Self {
        self.clauses.push(ClauseData::Ordered { n: None });
        self
    }

    /// Add an ordered clause with parameter
    pub fn ordered_n(mut self, n: &'a str) -> Self {
        self.clauses.push(ClauseData::Ordered {
            n: Some(Expression::unparsed(n)),
        });
        self
    }

    /// Add a nowait clause
    pub fn nowait(mut self) -> Self {
        self.clauses
            .push(ClauseData::Bare(Identifier::new("nowait")));
        self
    }

    /// Add a map clause
    pub fn map(mut self, map_type: MapType, vars: &[&'a str]) -> Self {
        let items = vars
            .iter()
            .map(|&v| ClauseItem::Identifier(Identifier::new(v)))
            .collect();
        self.clauses.push(ClauseData::Map {
            map_type: Some(map_type),
            mapper: None,
            items,
        });
        self
    }

    /// Add a depend clause
    pub fn depend(mut self, depend_type: DependType, vars: &[&'a str]) -> Self {
        let items = vars
            .iter()
            .map(|&v| ClauseItem::Identifier(Identifier::new(v)))
            .collect();
        self.clauses.push(ClauseData::Depend { depend_type, items });
        self
    }

    /// Add a proc_bind clause
    pub fn proc_bind(mut self, kind: ProcBind) -> Self {
        self.clauses.push(ClauseData::ProcBind(kind));
        self
    }

    /// Build the DirectiveIR
    ///
    /// ## Example
    ///
    /// ```
    /// use roup::ir::{DirectiveBuilder, Language, SourceLocation};
    ///
    /// let directive = DirectiveBuilder::parallel()
    ///     .default_shared()
    ///     .private(&["x"])
    ///     .build(SourceLocation::start(), Language::C);
    ///
    /// assert_eq!(directive.clauses().len(), 2);
    /// ```
    pub fn build(self, location: SourceLocation, language: Language) -> DirectiveIR {
        DirectiveIR::new(self.kind, &self.name, self.clauses, location, language)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_parallel_simple() {
        let directive = DirectiveBuilder::parallel().build(SourceLocation::start(), Language::C);

        assert_eq!(directive.kind(), DirectiveKind::Parallel);
        assert_eq!(directive.clauses().len(), 0);
    }

    #[test]
    fn test_builder_parallel_with_default() {
        let directive = DirectiveBuilder::parallel()
            .default_shared()
            .build(SourceLocation::start(), Language::C);

        assert_eq!(directive.clauses().len(), 1);
        assert!(directive.has_clause(|c| c.is_default()));
    }

    #[test]
    fn test_builder_parallel_with_multiple_clauses() {
        let directive = DirectiveBuilder::parallel()
            .default_shared()
            .private(&["x", "y"])
            .num_threads(4)
            .build(SourceLocation::start(), Language::C);

        assert_eq!(directive.clauses().len(), 3);
        assert!(directive.has_clause(|c| c.is_default()));
        assert!(directive.has_clause(|c| c.is_private()));
        assert!(directive.has_clause(|c| c.is_num_threads()));
    }

    #[test]
    fn test_builder_parallel_for() {
        let directive = DirectiveBuilder::parallel_for()
            .schedule(ScheduleKind::Static, Some("16"))
            .reduction(ReductionOperator::Add, &["sum"])
            .build(SourceLocation::start(), Language::C);

        assert_eq!(directive.kind(), DirectiveKind::ParallelFor);
        assert_eq!(directive.clauses().len(), 2);
    }

    #[test]
    fn test_builder_for_with_schedule() {
        let directive = DirectiveBuilder::for_loop()
            .schedule(ScheduleKind::Dynamic, Some("10"))
            .collapse("2")
            .build(SourceLocation::start(), Language::C);

        assert_eq!(directive.kind(), DirectiveKind::For);
        assert!(directive.has_clause(|c| c.is_schedule()));
        assert!(directive.has_clause(|c| c.is_collapse()));
    }

    #[test]
    fn test_builder_target_with_map() {
        let directive = DirectiveBuilder::target()
            .map(MapType::To, &["arr"])
            .build(SourceLocation::start(), Language::C);

        assert_eq!(directive.kind(), DirectiveKind::Target);
        assert!(directive.has_clause(|c| c.is_map()));
    }

    #[test]
    fn test_builder_task_with_depend() {
        let directive = DirectiveBuilder::task()
            .depend(DependType::In, &["x", "y"])
            .private(&["temp"])
            .build(SourceLocation::start(), Language::C);

        assert_eq!(directive.kind(), DirectiveKind::Task);
        assert!(directive.has_clause(|c| c.is_depend()));
        assert!(directive.has_clause(|c| c.is_private()));
    }

    #[test]
    fn test_builder_method_chaining() {
        let directive = DirectiveBuilder::parallel()
            .default_shared()
            .private(&["i", "j"])
            .shared(&["data"])
            .reduction(ReductionOperator::Add, &["sum"])
            .num_threads(8)
            .if_clause("n > 100")
            .build(SourceLocation::start(), Language::C);

        assert_eq!(directive.clauses().len(), 6);
    }

    #[test]
    fn test_builder_for_with_nowait() {
        let directive = DirectiveBuilder::for_loop()
            .schedule_simple(ScheduleKind::Static)
            .nowait()
            .build(SourceLocation::start(), Language::C);

        assert_eq!(directive.clauses().len(), 2);
    }

    #[test]
    fn test_builder_display_roundtrip() {
        let directive = DirectiveBuilder::parallel()
            .default_shared()
            .private(&["x"])
            .build(SourceLocation::start(), Language::C);

        let output = directive.to_string();
        assert!(output.contains("parallel"));
        assert!(output.contains("default(shared)"));
        assert!(output.contains("private(x)"));
    }
}
