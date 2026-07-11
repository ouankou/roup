//! Strict typed legality and context validation.
//!
//! Parsing establishes syntactic structure. This module rejects locally illegal
//! clause combinations and exposes the external semantic facts that a standalone
//! directive parser cannot derive. Unknown catalog entries are errors: validation
//! never treats an unclassified clause as allowed.

use std::collections::{HashMap, HashSet};

use crate::ast::{
    AccClause, AccClauseKind, AccClausePayload, AccDeviceType, AccDirective, AccDirectiveKind,
    AccGangArgument, AccVectorClause, AccWorkerClause, OmpClauseKind, OmpDirective,
    OmpDirectiveKind, OmpDirectiveParameter, OmpSelector, OmpSelectorDeviceTrait, OmpSelectorEntry,
    OmpSelectorExtensionProperty, OmpSelectorImplementationTraitKind,
};
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::ir::{
    ClauseData, DefaultmapCategory, Expression, OmpApplyLoopKind, OmpCount, OmpLocator,
    OmpParameterListItem, ScheduleKind,
};
use crate::source::Span;
use crate::version::{OpenAccVersion, OpenMpVersion, VersionPolicy};

/// Association whose truth must be supplied by a compiler or enclosing parser.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AssociationKind {
    /// A `section` directive is lexically associated with `sections`.
    SectionRegion,
    /// A `scan` directive is associated with a loop carrying an `inscan` reduction.
    ScanWithInscanLoop,
    /// An `ordered doacross(...)` construct binds to a loop carrying the
    /// required `ordered(n)` clause.
    DoacrossLoop,
}

/// Location of one clause within a directive.
///
/// `occurrence` is zero based among clauses with the same kind.  Keeping it in
/// the key prevents semantic facts for repeated clauses from being silently
/// reused for a different clause.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct OmpClauseSite {
    kind: OmpClauseKind,
    occurrence: usize,
}

impl OmpClauseSite {
    #[must_use]
    pub const fn new(kind: OmpClauseKind, occurrence: usize) -> Self {
        Self { kind, occurrence }
    }

    #[must_use]
    pub const fn kind(self) -> OmpClauseKind {
        self.kind
    }

    #[must_use]
    pub const fn occurrence(self) -> usize {
        self.occurrence
    }
}

/// Location of one expression within a specific clause occurrence.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct OmpExpressionSite {
    clause: OmpClauseSite,
    expression_index: usize,
}

impl OmpExpressionSite {
    #[must_use]
    pub const fn new(clause: OmpClauseSite, expression_index: usize) -> Self {
        Self {
            clause,
            expression_index,
        }
    }

    #[must_use]
    pub const fn clause(self) -> OmpClauseSite {
        self.clause
    }

    #[must_use]
    pub const fn expression_index(self) -> usize {
        self.expression_index
    }
}

/// Location of one list item within a specific clause occurrence.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct OmpClauseItemSite {
    clause: OmpClauseSite,
    item_index: usize,
}

impl OmpClauseItemSite {
    #[must_use]
    pub const fn new(clause: OmpClauseSite, item_index: usize) -> Self {
        Self { clause, item_index }
    }

    #[must_use]
    pub const fn clause(self) -> OmpClauseSite {
        self.clause
    }

    #[must_use]
    pub const fn item_index(self) -> usize {
        self.item_index
    }
}

/// Location of one locator within a specific data-motion clause occurrence.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct OmpLocatorSite {
    clause: OmpClauseSite,
    locator_index: usize,
}

/// Location of one OpenACC clause within a directive.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AccClauseSite {
    kind: AccClauseKind,
    occurrence: usize,
}

impl AccClauseSite {
    #[must_use]
    pub const fn new(kind: AccClauseKind, occurrence: usize) -> Self {
        Self { kind, occurrence }
    }

    #[must_use]
    pub const fn kind(self) -> AccClauseKind {
        self.kind
    }

    #[must_use]
    pub const fn occurrence(self) -> usize {
        self.occurrence
    }
}

/// Location of one expression within a specific OpenACC clause occurrence.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AccExpressionSite {
    clause: AccClauseSite,
    expression_index: usize,
}

impl AccExpressionSite {
    #[must_use]
    pub const fn new(clause: AccClauseSite, expression_index: usize) -> Self {
        Self {
            clause,
            expression_index,
        }
    }

    #[must_use]
    pub const fn clause(self) -> AccClauseSite {
        self.clause
    }

    #[must_use]
    pub const fn expression_index(self) -> usize {
        self.expression_index
    }
}

impl OmpLocatorSite {
    #[must_use]
    pub const fn new(clause: OmpClauseSite, locator_index: usize) -> Self {
        Self {
            clause,
            locator_index,
        }
    }

    #[must_use]
    pub const fn clause(self) -> OmpClauseSite {
        self.clause
    }

    #[must_use]
    pub const fn locator_index(self) -> usize {
        self.locator_index
    }
}

/// Result of evaluating a constant-expression candidate as an integer.
///
/// A negative value does not need its magnitude for any OpenMP restriction
/// currently checked here.  `NotInteger` is distinct from a missing fact and
/// therefore produces a type diagnostic instead of being treated as unknown.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntegerEvaluation {
    NotInteger,
    Negative,
    NonNegative(u128),
}

/// Result of evaluating a constant-logical expression.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogicalEvaluation {
    NotLogical,
    False,
    True,
}

/// Compiler-owned classification of the statement associated with an atomic
/// directive whose clause restrictions depend on the update form.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AtomicUpdateForm {
    Unconditional,
    EqualityConditional,
    OtherConditional,
    FortranMaxMin,
}

/// Compiler-owned type and initialization state for a `depend(depobj: ...)`
/// list item.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DependObjectState {
    WrongType,
    Uninitialized,
    Initialized,
}

/// Compiler-owned classification of a variable named by an interoperability
/// action clause.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InteropObjectState {
    WrongType,
    Uninitialized,
    Initialized,
}

/// Compiler-owned classification of a `detach` event-handle variable.
///
/// `Invalid` covers the non-type restrictions that cannot be derived from one
/// directive: aggregate subobjects and, in Fortran, disallowed pointer or
/// allocation-state changes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DetachEventStatus {
    WrongType,
    Invalid,
    Valid,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct OmpExpressionFactRecord {
    constant: Option<bool>,
    integer_evaluation: Option<IntegerEvaluation>,
    integer: Option<bool>,
    nonnegative_integer: Option<bool>,
    positive_integer: Option<bool>,
    string: Option<bool>,
    logical: Option<bool>,
    logical_evaluation: Option<LogicalEvaluation>,
    region_invariant: Option<bool>,
    ultimate: Option<bool>,
    linear_step: Option<bool>,
    induction_step: Option<bool>,
    collector_expression: Option<bool>,
    inductor_expression: Option<bool>,
    synchronization_hint: Option<bool>,
    safesync_compatible: Option<bool>,
    impex: Option<bool>,
    allocator_handle: Option<bool>,
    binding_set_invariant: Option<bool>,
    conforming_device_number: Option<bool>,
}

/// Semantic facts unavailable from one standalone directive.
///
/// Every field starts unknown. Validators return `MissingSemanticFact` instead
/// of inferring a favorable value.
#[derive(Clone, Debug, Default)]
pub struct SemanticFacts {
    declaration_position: Option<bool>,
    inside_target_region: Option<bool>,
    dynamic_allocators_requirement: Option<bool>,
    encountering_final_task: Option<bool>,
    associations: HashMap<AssociationKind, bool>,
    omp_expressions: HashMap<OmpExpressionSite, OmpExpressionFactRecord>,
    acc_integer_evaluations: HashMap<AccExpressionSite, IntegerEvaluation>,
    allocatable_items: HashMap<OmpClauseItemSite, bool>,
    procedure_parameters: HashMap<OmpClauseItemSite, bool>,
    linear_items: HashMap<OmpClauseItemSite, bool>,
    induction_items: HashMap<OmpClauseItemSite, bool>,
    allocator_traits: HashMap<OmpClauseItemSite, bool>,
    depend_objects: HashMap<OmpClauseItemSite, DependObjectState>,
    interop_objects: HashMap<OmpClauseItemSite, InteropObjectState>,
    detach_events: HashMap<OmpClauseItemSite, DetachEventStatus>,
    modifiable_items: HashMap<OmpClauseItemSite, bool>,
    interop_targetsync: HashMap<OmpClauseSite, bool>,
    lvalue_locators: HashMap<OmpLocatorSite, bool>,
    ordered_bounds: HashMap<OmpClauseSite, bool>,
    atomic_update_form: Option<AtomicUpdateForm>,
    associated_ordered_parameter: Option<usize>,
}

impl SemanticFacts {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_declaration_position(mut self, is_valid: bool) -> Self {
        self.declaration_position = Some(is_valid);
        self
    }

    /// Supply whether the directive is lexically contained in a target region.
    #[must_use]
    pub fn with_inside_target_region(mut self, is_inside: bool) -> Self {
        self.inside_target_region = Some(is_inside);
        self
    }

    /// Supply whether a governing `requires dynamic_allocators` requirement is
    /// present for allocator use in a target context.
    #[must_use]
    pub fn with_dynamic_allocators_requirement(mut self, is_present: bool) -> Self {
        self.dynamic_allocators_requirement = Some(is_present);
        self
    }

    /// Supply whether the task encountering a `detach` clause is final.
    #[must_use]
    pub const fn with_encountering_final_task(mut self, is_final: bool) -> Self {
        self.encountering_final_task = Some(is_final);
        self
    }

    #[must_use]
    pub fn with_association(mut self, association: AssociationKind, is_valid: bool) -> Self {
        self.associations.insert(association, is_valid);
        self
    }

    #[must_use]
    pub fn with_constant_expression(mut self, site: OmpExpressionSite, is_valid: bool) -> Self {
        self.omp_expressions.entry(site).or_default().constant = Some(is_valid);
        self
    }

    #[must_use]
    pub fn with_integer_evaluation(
        mut self,
        site: OmpExpressionSite,
        evaluation: IntegerEvaluation,
    ) -> Self {
        self.omp_expressions
            .entry(site)
            .or_default()
            .integer_evaluation = Some(evaluation);
        self
    }

    #[must_use]
    pub fn with_acc_integer_evaluation(
        mut self,
        site: AccExpressionSite,
        evaluation: IntegerEvaluation,
    ) -> Self {
        self.acc_integer_evaluations.insert(site, evaluation);
        self
    }

    #[must_use]
    pub fn with_integer_expression(mut self, site: OmpExpressionSite, is_valid: bool) -> Self {
        self.omp_expressions.entry(site).or_default().integer = Some(is_valid);
        self
    }

    /// Supply whether an integer expression identifies a conforming OpenMP
    /// device and is not `omp_invalid_device`.
    #[must_use]
    pub fn with_conforming_device_number(
        mut self,
        site: OmpExpressionSite,
        is_valid: bool,
    ) -> Self {
        self.omp_expressions
            .entry(site)
            .or_default()
            .conforming_device_number = Some(is_valid);
        self
    }

    #[must_use]
    pub fn with_nonnegative_integer_expression(
        mut self,
        site: OmpExpressionSite,
        is_valid: bool,
    ) -> Self {
        self.omp_expressions
            .entry(site)
            .or_default()
            .nonnegative_integer = Some(is_valid);
        self
    }

    #[must_use]
    pub fn with_positive_integer_expression(
        mut self,
        site: OmpExpressionSite,
        is_valid: bool,
    ) -> Self {
        self.omp_expressions
            .entry(site)
            .or_default()
            .positive_integer = Some(is_valid);
        self
    }

    #[must_use]
    pub fn with_string_expression(mut self, site: OmpExpressionSite, is_valid: bool) -> Self {
        self.omp_expressions.entry(site).or_default().string = Some(is_valid);
        self
    }

    /// Supply whether an expression has the OpenMP logical type.
    ///
    /// This is intentionally distinct from [`Self::with_logical_evaluation`]:
    /// runtime conditions need a type fact but do not have a compile-time value.
    #[must_use]
    pub fn with_logical_expression(mut self, site: OmpExpressionSite, is_valid: bool) -> Self {
        self.omp_expressions.entry(site).or_default().logical = Some(is_valid);
        self
    }

    #[must_use]
    pub fn with_logical_evaluation(
        mut self,
        site: OmpExpressionSite,
        evaluation: LogicalEvaluation,
    ) -> Self {
        self.omp_expressions
            .entry(site)
            .or_default()
            .logical_evaluation = Some(evaluation);
        self
    }

    /// Supply whether an expression is invariant throughout the associated
    /// OpenMP region.
    #[must_use]
    pub fn with_region_invariant_expression(
        mut self,
        site: OmpExpressionSite,
        is_valid: bool,
    ) -> Self {
        self.omp_expressions
            .entry(site)
            .or_default()
            .region_invariant = Some(is_valid);
        self
    }

    /// Supply whether an expression satisfies the OpenMP `ultimate` property.
    #[must_use]
    pub fn with_ultimate_expression(mut self, site: OmpExpressionSite, is_valid: bool) -> Self {
        self.omp_expressions.entry(site).or_default().ultimate = Some(is_valid);
        self
    }

    /// Supply the compiler-owned restrictions for a non-literal `linear` step.
    ///
    /// Besides integer typing, this covers region invariance and, for an older
    /// `declare simd`, the requirement that a nonconstant step be an
    /// integer-typed parameter named in a `uniform` clause.
    #[must_use]
    pub fn with_linear_step(mut self, site: OmpExpressionSite, is_valid: bool) -> Self {
        self.omp_expressions.entry(site).or_default().linear_step = Some(is_valid);
        self
    }

    /// Supply whether an induction step is scalar, region invariant, and type
    /// compatible with its induction identifier and list items.
    #[must_use]
    pub fn with_induction_step(mut self, site: OmpExpressionSite, is_valid: bool) -> Self {
        self.omp_expressions.entry(site).or_default().induction_step = Some(is_valid);
        self
    }

    /// Supply whether a `collector` argument has collector-expression type for
    /// the declared induction types.
    #[must_use]
    pub fn with_collector_expression(mut self, site: OmpExpressionSite, is_valid: bool) -> Self {
        self.omp_expressions
            .entry(site)
            .or_default()
            .collector_expression = Some(is_valid);
        self
    }

    /// Supply whether an `inductor` argument has inductor-expression type for
    /// the declared induction types.
    #[must_use]
    pub fn with_inductor_expression(mut self, site: OmpExpressionSite, is_valid: bool) -> Self {
        self.omp_expressions
            .entry(site)
            .or_default()
            .inductor_expression = Some(is_valid);
        self
    }

    /// Supply whether an expression evaluates to a valid OpenMP
    /// synchronization hint.
    #[must_use]
    pub fn with_synchronization_hint(mut self, site: OmpExpressionSite, is_valid: bool) -> Self {
        self.omp_expressions
            .entry(site)
            .or_default()
            .synchronization_hint = Some(is_valid);
        self
    }

    /// Supply whether a `safesync` width is safesync-compatible in its
    /// enclosing parallel context.
    #[must_use]
    pub fn with_safesync_compatible(mut self, site: OmpExpressionSite, is_valid: bool) -> Self {
        self.omp_expressions
            .entry(site)
            .or_default()
            .safesync_compatible = Some(is_valid);
        self
    }

    /// Supply whether an expression has the OpenMP `impex` type.
    #[must_use]
    pub fn with_impex_expression(mut self, site: OmpExpressionSite, is_valid: bool) -> Self {
        self.omp_expressions.entry(site).or_default().impex = Some(is_valid);
        self
    }

    /// Supply whether an expression has OpenMP allocator-handle type.
    #[must_use]
    pub fn with_allocator_handle_expression(
        mut self,
        site: OmpExpressionSite,
        is_valid: bool,
    ) -> Self {
        self.omp_expressions
            .entry(site)
            .or_default()
            .allocator_handle = Some(is_valid);
        self
    }

    /// Supply whether a runtime expression evaluates to the same value for
    /// every thread/task in the clause's binding set.
    #[must_use]
    pub fn with_binding_set_invariant_expression(
        mut self,
        site: OmpExpressionSite,
        is_valid: bool,
    ) -> Self {
        self.omp_expressions
            .entry(site)
            .or_default()
            .binding_set_invariant = Some(is_valid);
        self
    }

    #[must_use]
    pub fn with_allocatable_item(mut self, site: OmpClauseItemSite, is_valid: bool) -> Self {
        self.allocatable_items.insert(site, is_valid);
        self
    }

    #[must_use]
    pub fn with_procedure_parameter(mut self, site: OmpClauseItemSite, is_valid: bool) -> Self {
        self.procedure_parameters.insert(site, is_valid);
        self
    }

    /// Supply whether a `linear` list item satisfies all host-language type,
    /// attribute, storage-association, and modifier restrictions.
    #[must_use]
    pub fn with_linear_item(mut self, site: OmpClauseItemSite, is_valid: bool) -> Self {
        self.linear_items.insert(site, is_valid);
        self
    }

    /// Supply whether an `induction` list item is definable and type/rank
    /// compatible with the selected induction identifier and step.
    #[must_use]
    pub fn with_induction_item(mut self, site: OmpClauseItemSite, is_valid: bool) -> Self {
        self.induction_items.insert(site, is_valid);
        self
    }

    /// Supply whether a `uses_allocators` traits variable satisfies the host
    /// language's constant-array, constant-value, scope, type, and rank rules.
    #[must_use]
    pub fn with_allocator_traits(mut self, site: OmpClauseItemSite, is_valid: bool) -> Self {
        self.allocator_traits.insert(site, is_valid);
        self
    }

    #[must_use]
    pub fn with_depend_object(mut self, site: OmpClauseItemSite, state: DependObjectState) -> Self {
        self.depend_objects.insert(site, state);
        self
    }

    /// Supply the type and initialization state of an interoperability object
    /// named by an `init`, `use`, or `destroy` action clause.
    #[must_use]
    pub fn with_interop_object(
        mut self,
        site: OmpClauseItemSite,
        state: InteropObjectState,
    ) -> Self {
        self.interop_objects.insert(site, state);
        self
    }

    /// Supply the complete compiler-owned classification of a detach event.
    #[must_use]
    pub fn with_detach_event(mut self, site: OmpClauseItemSite, status: DetachEventStatus) -> Self {
        self.detach_events.insert(site, status);
        self
    }

    /// Supply whether an action-clause variable can be modified.
    #[must_use]
    pub fn with_modifiable_item(mut self, site: OmpClauseItemSite, is_modifiable: bool) -> Self {
        self.modifiable_items.insert(site, is_modifiable);
        self
    }

    /// Supply whether an interoperability object was initialized with the
    /// `targetsync` interoperability type.
    #[must_use]
    pub fn with_interop_targetsync(mut self, site: OmpClauseSite, is_present: bool) -> Self {
        self.interop_targetsync.insert(site, is_present);
        self
    }

    #[must_use]
    pub fn with_lvalue_locator(mut self, site: OmpLocatorSite, is_valid: bool) -> Self {
        self.lvalue_locators.insert(site, is_valid);
        self
    }

    #[must_use]
    pub fn with_ordered_bounds(mut self, site: OmpClauseSite, is_valid: bool) -> Self {
        self.ordered_bounds.insert(site, is_valid);
        self
    }

    #[must_use]
    pub const fn with_atomic_update_form(mut self, form: AtomicUpdateForm) -> Self {
        self.atomic_update_form = Some(form);
        self
    }

    #[must_use]
    pub const fn with_associated_ordered_parameter(mut self, dimensions: usize) -> Self {
        self.associated_ordered_parameter = Some(dimensions);
        self
    }

    #[must_use]
    pub const fn declaration_position(&self) -> Option<bool> {
        self.declaration_position
    }

    #[must_use]
    pub const fn inside_target_region(&self) -> Option<bool> {
        self.inside_target_region
    }

    #[must_use]
    pub const fn dynamic_allocators_requirement(&self) -> Option<bool> {
        self.dynamic_allocators_requirement
    }

    #[must_use]
    pub const fn encountering_final_task(&self) -> Option<bool> {
        self.encountering_final_task
    }

    #[must_use]
    pub fn association(&self, association: AssociationKind) -> Option<bool> {
        self.associations.get(&association).copied()
    }

    #[must_use]
    pub fn constant_expression(&self, site: OmpExpressionSite) -> Option<bool> {
        self.omp_expressions
            .get(&site)
            .and_then(|facts| facts.constant)
    }

    #[must_use]
    pub fn integer_evaluation(&self, site: OmpExpressionSite) -> Option<IntegerEvaluation> {
        self.omp_expressions
            .get(&site)
            .and_then(|facts| facts.integer_evaluation)
    }

    #[must_use]
    pub fn acc_integer_evaluation(&self, site: AccExpressionSite) -> Option<IntegerEvaluation> {
        self.acc_integer_evaluations.get(&site).copied()
    }

    #[must_use]
    pub fn integer_expression(&self, site: OmpExpressionSite) -> Option<bool> {
        self.omp_expressions
            .get(&site)
            .and_then(|facts| facts.integer)
    }

    #[must_use]
    pub fn conforming_device_number(&self, site: OmpExpressionSite) -> Option<bool> {
        self.omp_expressions
            .get(&site)
            .and_then(|facts| facts.conforming_device_number)
    }

    #[must_use]
    pub fn nonnegative_integer_expression(&self, site: OmpExpressionSite) -> Option<bool> {
        self.omp_expressions
            .get(&site)
            .and_then(|facts| facts.nonnegative_integer)
    }

    #[must_use]
    pub fn positive_integer_expression(&self, site: OmpExpressionSite) -> Option<bool> {
        self.omp_expressions
            .get(&site)
            .and_then(|facts| facts.positive_integer)
    }

    #[must_use]
    pub fn string_expression(&self, site: OmpExpressionSite) -> Option<bool> {
        self.omp_expressions
            .get(&site)
            .and_then(|facts| facts.string)
    }

    #[must_use]
    pub fn logical_expression(&self, site: OmpExpressionSite) -> Option<bool> {
        self.omp_expressions
            .get(&site)
            .and_then(|facts| facts.logical)
    }

    #[must_use]
    pub fn logical_evaluation(&self, site: OmpExpressionSite) -> Option<LogicalEvaluation> {
        self.omp_expressions
            .get(&site)
            .and_then(|facts| facts.logical_evaluation)
    }

    #[must_use]
    pub fn region_invariant_expression(&self, site: OmpExpressionSite) -> Option<bool> {
        self.omp_expressions
            .get(&site)
            .and_then(|facts| facts.region_invariant)
    }

    #[must_use]
    pub fn ultimate_expression(&self, site: OmpExpressionSite) -> Option<bool> {
        self.omp_expressions
            .get(&site)
            .and_then(|facts| facts.ultimate)
    }

    #[must_use]
    pub fn linear_step(&self, site: OmpExpressionSite) -> Option<bool> {
        self.omp_expressions
            .get(&site)
            .and_then(|facts| facts.linear_step)
    }

    #[must_use]
    pub fn induction_step(&self, site: OmpExpressionSite) -> Option<bool> {
        self.omp_expressions
            .get(&site)
            .and_then(|facts| facts.induction_step)
    }

    #[must_use]
    pub fn collector_expression(&self, site: OmpExpressionSite) -> Option<bool> {
        self.omp_expressions
            .get(&site)
            .and_then(|facts| facts.collector_expression)
    }

    #[must_use]
    pub fn inductor_expression(&self, site: OmpExpressionSite) -> Option<bool> {
        self.omp_expressions
            .get(&site)
            .and_then(|facts| facts.inductor_expression)
    }

    #[must_use]
    pub fn synchronization_hint(&self, site: OmpExpressionSite) -> Option<bool> {
        self.omp_expressions
            .get(&site)
            .and_then(|facts| facts.synchronization_hint)
    }

    #[must_use]
    pub fn safesync_compatible(&self, site: OmpExpressionSite) -> Option<bool> {
        self.omp_expressions
            .get(&site)
            .and_then(|facts| facts.safesync_compatible)
    }

    #[must_use]
    pub fn impex_expression(&self, site: OmpExpressionSite) -> Option<bool> {
        self.omp_expressions
            .get(&site)
            .and_then(|facts| facts.impex)
    }

    #[must_use]
    pub fn allocator_handle_expression(&self, site: OmpExpressionSite) -> Option<bool> {
        self.omp_expressions
            .get(&site)
            .and_then(|facts| facts.allocator_handle)
    }

    #[must_use]
    pub fn binding_set_invariant_expression(&self, site: OmpExpressionSite) -> Option<bool> {
        self.omp_expressions
            .get(&site)
            .and_then(|facts| facts.binding_set_invariant)
    }

    #[must_use]
    pub fn allocatable_item(&self, site: OmpClauseItemSite) -> Option<bool> {
        self.allocatable_items.get(&site).copied()
    }

    #[must_use]
    pub fn procedure_parameter(&self, site: OmpClauseItemSite) -> Option<bool> {
        self.procedure_parameters.get(&site).copied()
    }

    #[must_use]
    pub fn linear_item(&self, site: OmpClauseItemSite) -> Option<bool> {
        self.linear_items.get(&site).copied()
    }

    #[must_use]
    pub fn induction_item(&self, site: OmpClauseItemSite) -> Option<bool> {
        self.induction_items.get(&site).copied()
    }

    #[must_use]
    pub fn allocator_traits(&self, site: OmpClauseItemSite) -> Option<bool> {
        self.allocator_traits.get(&site).copied()
    }

    #[must_use]
    pub fn depend_object(&self, site: OmpClauseItemSite) -> Option<DependObjectState> {
        self.depend_objects.get(&site).copied()
    }

    #[must_use]
    pub fn interop_object(&self, site: OmpClauseItemSite) -> Option<InteropObjectState> {
        self.interop_objects.get(&site).copied()
    }

    #[must_use]
    pub fn detach_event(&self, site: OmpClauseItemSite) -> Option<DetachEventStatus> {
        self.detach_events.get(&site).copied()
    }

    #[must_use]
    pub fn modifiable_item(&self, site: OmpClauseItemSite) -> Option<bool> {
        self.modifiable_items.get(&site).copied()
    }

    #[must_use]
    pub fn interop_targetsync(&self, site: OmpClauseSite) -> Option<bool> {
        self.interop_targetsync.get(&site).copied()
    }

    #[must_use]
    pub fn lvalue_locator(&self, site: OmpLocatorSite) -> Option<bool> {
        self.lvalue_locators.get(&site).copied()
    }

    #[must_use]
    pub fn ordered_bounds(&self, site: OmpClauseSite) -> Option<bool> {
        self.ordered_bounds.get(&site).copied()
    }

    #[must_use]
    pub const fn atomic_update_form(&self) -> Option<AtomicUpdateForm> {
        self.atomic_update_form
    }

    #[must_use]
    pub const fn associated_ordered_parameter(&self) -> Option<usize> {
        self.associated_ordered_parameter
    }
}

/// Validate all context-independent rules for one typed OpenMP directive.
///
/// This is the validation level available to standalone parsers. It does not
/// infer declaration placement, enclosing-construct association, or host
/// constant-expression semantics.
pub fn validate_openmp(
    directive: &OmpDirective,
    _policy: VersionPolicy<OpenMpVersion>,
    span: Span,
) -> Result<(), Diagnostic> {
    for (index, clause) in directive.clauses().iter().enumerate() {
        if let Some(modifier) = clause.directive_name_modifier() {
            if !omp_modifier_names_directive_or_constituent(directive.kind(), modifier) {
                return Err(Diagnostic::new(
                    DiagnosticCode::InvalidClause,
                    span,
                    format!(
                        "OpenMP directive-name modifier {} does not name {:?} or one of its constituents",
                        modifier.as_str(),
                        directive.kind()
                    ),
                ));
            }
            if !omp_clause_applies_to_named_constituent(directive.kind(), modifier, clause.kind()) {
                return Err(Diagnostic::new(
                    DiagnosticCode::ClauseNotAllowed,
                    span,
                    format!(
                        "OpenMP clause {:?} does not apply to named constituent {} of {:?}",
                        clause.kind(),
                        modifier.as_str(),
                        directive.kind()
                    ),
                ));
            }
        }
        let Some(allowed) = openmp_clause_allowed(directive.kind(), clause.kind()) else {
            return Err(Diagnostic::new(
                DiagnosticCode::InvalidClause,
                span,
                format!(
                    "OpenMP legality catalog has no entry for clause {:?} on {:?}",
                    clause.kind(),
                    directive.kind()
                ),
            ));
        };
        if !allowed {
            return Err(Diagnostic::new(
                DiagnosticCode::ClauseNotAllowed,
                span,
                format!(
                    "OpenMP clause {:?} is not allowed on directive {:?}",
                    clause.kind(),
                    directive.kind()
                ),
            ));
        }

        validate_openmp_directive_specific_clause(directive, clause, _policy, span)?;

        if clause.kind() == OmpClauseKind::Destroy
            && matches!(clause.payload(), ClauseData::Destroy { variable: None })
            && directive.kind() != OmpDirectiveKind::Depobj
        {
            return Err(Diagnostic::new(
                DiagnosticCode::InvalidClause,
                span,
                "only the historical depobj destroy clause may omit its argument",
            ));
        }

        validate_openmp_obvious_scalar_constraints(clause, span)?;

        if openmp_clause_is_unique(clause.kind())
            && directive.clauses()[..index]
                .iter()
                .any(|previous| previous.kind() == clause.kind())
        {
            return Err(Diagnostic::new(
                DiagnosticCode::DuplicateClause,
                span,
                format!(
                    "OpenMP clause {:?} may not be repeated on {:?}",
                    clause.kind(),
                    directive.kind()
                ),
            ));
        }

        if matches!(clause.payload(), ClauseData::If { .. }) {
            let modifier = clause.directive_name_modifier();
            let overlapping_modifier = directive.clauses()[..index]
                .iter()
                .filter(|previous| matches!(previous.payload(), ClauseData::If { .. }))
                .map(|previous| previous.directive_name_modifier())
                .any(|previous| omp_if_targets_overlap(directive.kind(), previous, modifier));
            if overlapping_modifier {
                return Err(Diagnostic::new(
                    DiagnosticCode::DuplicateClause,
                    span,
                    match modifier {
                        Some(modifier) => format!(
                            "OpenMP if clause modifier {} overlaps another if clause on {:?}",
                            modifier.as_str(),
                            directive.kind()
                        ),
                        None => format!(
                            "unmodified OpenMP if clause overlaps another if clause on {:?}",
                            directive.kind()
                        ),
                    },
                ));
            }
        }

        if let ClauseData::MetadirectiveSelector { selector } = clause.payload() {
            if let Some(nested) = selector.nested_directive() {
                validate_openmp(nested, _policy, nested.span())?;
            }
            for entry in selector.entries() {
                let OmpSelectorEntry::Construct { constructs } = entry else {
                    continue;
                };
                for construct in constructs {
                    let nested = construct.directive();
                    if !nested.clauses().is_empty() {
                        validate_openmp(nested, _policy, nested.span())?;
                    }
                }
            }
        }
    }

    validate_openmp_obvious_relations(directive, span)?;
    validate_openmp_obvious_apply_index_sets(directive, span)?;
    validate_openmp_conflicts(directive, span)?;
    validate_openmp_required_clauses(directive, span)
}

fn validate_openmp_directive_specific_clause(
    directive: &OmpDirective,
    clause: &crate::ast::OmpClause,
    policy: VersionPolicy<OpenMpVersion>,
    span: Span,
) -> Result<(), Diagnostic> {
    if matches!(
        clause.payload(),
        ClauseData::Default {
            category: Some(_),
            ..
        }
    ) && !matches!(
        directive.kind(),
        OmpDirectiveKind::Target | OmpDirectiveKind::TargetData
    ) {
        return Err(Diagnostic::new(
            DiagnosticCode::ClauseNotAllowed,
            span,
            "categorized default clauses are only valid on target and target_data",
        ));
    }
    match clause.payload() {
        ClauseData::Map {
            map_type,
            map_type_spelling,
            modifiers,
            ..
        } => {
            let invalid = match directive.kind() {
                OmpDirectiveKind::TargetEnterData => {
                    matches!(map_type, Some(crate::ir::MapType::From))
                        || matches!(
                            map_type_spelling,
                            crate::ir::MapTypeSpelling::Release
                                | crate::ir::MapTypeSpelling::Delete
                        )
                        || modifiers.contains(&crate::ir::MapModifier::Delete)
                }
                OmpDirectiveKind::TargetExitData => {
                    matches!(map_type, Some(crate::ir::MapType::To))
                        || *map_type_spelling == crate::ir::MapTypeSpelling::Alloc
                }
                _ => false,
            };
            if invalid {
                Err(Diagnostic::new(
                    DiagnosticCode::InvalidClause,
                    span,
                    format!(
                        "map clause direction is incompatible with directive {:?}",
                        directive.kind()
                    ),
                ))
            } else {
                Ok(())
            }
        }
        ClauseData::Apply {
            loop_modifier,
            applied_directives,
        } => validate_openmp_apply_clause(
            directive,
            loop_modifier.as_ref(),
            applied_directives,
            policy,
            span,
        ),
        ClauseData::AdjustArgs { parameters, .. } => {
            validate_adjust_args_obvious_overlaps(parameters, span)
        }
        ClauseData::Linear {
            modifier, items, ..
        } => {
            if matches!(
                modifier,
                Some(crate::ir::LinearModifier::Ref | crate::ir::LinearModifier::Uval)
            ) && directive.kind() != OmpDirectiveKind::DeclareSimd
            {
                return Err(Diagnostic::new(
                    DiagnosticCode::InvalidModifier,
                    span,
                    "linear ref and uval modifiers are only valid on declare_simd",
                ));
            }
            if items
                .iter()
                .any(|item| matches!(item, crate::ir::ClauseItem::FortranCommonBlock(_)))
            {
                return Err(Diagnostic::new(
                    DiagnosticCode::InvalidClause,
                    span,
                    "a Fortran common block may not appear in a linear clause",
                ));
            }
            Ok(())
        }
        ClauseData::UsesAllocators { allocators } if matches!(policy, VersionPolicy::Exact(version) if version <= OpenMpVersion::V5_1) => {
            if allocators.iter().any(|allocator| {
                matches!(
                    allocator.allocator(),
                    crate::ir::UsesAllocatorKind::Custom(_)
                ) && allocator.traits().is_none()
            }) {
                Err(Diagnostic::new(
                    DiagnosticCode::MissingRequiredClause,
                    span,
                    "OpenMP 5.0 and 5.1 require traits for every non-predefined allocator",
                ))
            } else {
                Ok(())
            }
        }
        ClauseData::Depend { dependence, .. } => {
            if let crate::ir::OmpDependence::Locators { kind, locators } = dependence
                && locators.contains(&OmpLocator::AllMemory)
                && (!matches!(
                    kind,
                    crate::ir::DependType::Out | crate::ir::DependType::Inout
                ) || locators.len() != 1)
            {
                return Err(Diagnostic::new(
                    DiagnosticCode::InvalidClause,
                    span,
                    "omp_all_memory must be the only locator and use out or inout dependence",
                ));
            }
            Ok(())
        }
        ClauseData::Doacross { kind, iteration } => {
            let valid = matches!(
                (kind, iteration),
                (
                    crate::ir::DoacrossType::Source,
                    crate::ir::OmpDoacrossIteration::Current
                ) | (
                    crate::ir::DoacrossType::Sink,
                    crate::ir::OmpDoacrossIteration::PreviousCurrent
                        | crate::ir::OmpDoacrossIteration::Vector(_)
                )
            );
            if valid {
                Ok(())
            } else {
                Err(Diagnostic::new(
                    DiagnosticCode::InvalidClause,
                    span,
                    "doacross source requires the current iteration and sink requires a previous-current or vector iteration",
                ))
            }
        }
        _ => Ok(()),
    }
}

fn validate_adjust_args_obvious_overlaps(
    parameters: &[OmpParameterListItem],
    span: Span,
) -> Result<(), Diagnostic> {
    let mut names = HashSet::new();
    let mut positions = HashSet::new();
    let mut ranges = Vec::<(u128, Option<u128>)>::new();
    for parameter in parameters {
        match parameter {
            OmpParameterListItem::Named(name) => {
                if !names.insert(name) {
                    return Err(Diagnostic::new(
                        DiagnosticCode::DuplicateClause,
                        span,
                        "adjust_args names each parameter at most once",
                    ));
                }
            }
            OmpParameterListItem::Position(position) => {
                if !positions.insert(u128::from(*position)) {
                    return Err(Diagnostic::new(
                        DiagnosticCode::DuplicateClause,
                        span,
                        "adjust_args positions each parameter at most once",
                    ));
                }
            }
            OmpParameterListItem::Range(range) => {
                let lower = match range.lower() {
                    Some(lower) => {
                        let Some(lower) =
                            reject_obvious_nonpositive(lower, span, "adjust_args range bound")?
                        else {
                            continue;
                        };
                        lower
                    }
                    None => 1,
                };
                let upper = match range.upper() {
                    Some(upper) => {
                        let Some(upper) =
                            reject_obvious_nonpositive(upper, span, "adjust_args range bound")?
                        else {
                            continue;
                        };
                        Some(upper)
                    }
                    None => None,
                };
                if upper.is_some_and(|upper| lower > upper) {
                    return Err(Diagnostic::new(
                        DiagnosticCode::InvalidClause,
                        span,
                        "adjust_args range lower bound must not exceed its upper bound",
                    ));
                }
                ranges.push((lower, upper));
            }
        }
    }
    reject_adjust_args_interval_overlaps(&positions, &ranges, span)
}

fn reject_adjust_args_interval_overlaps(
    positions: &HashSet<u128>,
    ranges: &[(u128, Option<u128>)],
    span: Span,
) -> Result<(), Diagnostic> {
    for position in positions {
        if ranges
            .iter()
            .any(|(lower, upper)| position >= lower && upper.is_none_or(|upper| position <= &upper))
        {
            return Err(Diagnostic::new(
                DiagnosticCode::ConflictingClauses,
                span,
                "adjust_args position and range items overlap",
            ));
        }
    }
    for (index, (first_lower, first_upper)) in ranges.iter().enumerate() {
        for (second_lower, second_upper) in &ranges[index + 1..] {
            let first_before_second = first_upper.is_some_and(|upper| upper < *second_lower);
            let second_before_first = second_upper.is_some_and(|upper| upper < *first_lower);
            if !first_before_second && !second_before_first {
                return Err(Diagnostic::new(
                    DiagnosticCode::ConflictingClauses,
                    span,
                    "adjust_args range items overlap",
                ));
            }
        }
    }
    Ok(())
}

fn validate_openmp_apply_clause(
    directive: &OmpDirective,
    modifier: Option<&crate::ir::OmpApplyLoopModifier>,
    applied_directives: &[OmpDirective],
    policy: VersionPolicy<OpenMpVersion>,
    span: Span,
) -> Result<(), Diagnostic> {
    let allowed_modifier = match directive.kind() {
        OmpDirectiveKind::Fuse => Some((OmpApplyLoopKind::Fused, true)),
        OmpDirectiveKind::Interchange => Some((OmpApplyLoopKind::Interchanged, true)),
        OmpDirectiveKind::Nothing => Some((OmpApplyLoopKind::Identity, true)),
        OmpDirectiveKind::Reverse => Some((OmpApplyLoopKind::Reversed, true)),
        OmpDirectiveKind::Split => Some((OmpApplyLoopKind::Split, false)),
        OmpDirectiveKind::Stripe => modifier
            .filter(|modifier| {
                matches!(
                    modifier.kind,
                    OmpApplyLoopKind::Offsets | OmpApplyLoopKind::Grid
                )
            })
            .map(|modifier| (modifier.kind, false)),
        OmpDirectiveKind::Tile => modifier
            .filter(|modifier| {
                matches!(
                    modifier.kind,
                    OmpApplyLoopKind::Grid | OmpApplyLoopKind::Intratile
                )
            })
            .map(|modifier| (modifier.kind, false)),
        OmpDirectiveKind::Unroll => Some((OmpApplyLoopKind::Unrolled, true)),
        _ => None,
    };
    let Some((expected, has_default)) = allowed_modifier else {
        return Err(Diagnostic::new(
            DiagnosticCode::InvalidModifier,
            span,
            format!("invalid apply loop modifier on {:?}", directive.kind()),
        ));
    };
    match modifier {
        Some(modifier) if modifier.kind != expected => {
            return Err(Diagnostic::new(
                DiagnosticCode::InvalidModifier,
                span,
                format!(
                    "apply modifier {:?} is not valid on {:?}",
                    modifier.kind,
                    directive.kind()
                ),
            ));
        }
        None if !has_default => {
            return Err(Diagnostic::new(
                DiagnosticCode::MissingRequiredClause,
                span,
                format!("apply on {:?} requires a loop modifier", directive.kind()),
            ));
        }
        Some(_) | None => {}
    }

    let generally_composable = matches!(
        directive.kind(),
        OmpDirectiveKind::Reverse | OmpDirectiveKind::Split | OmpDirectiveKind::Unroll
    );
    for applied in applied_directives {
        if !is_loop_nest_associated_apply_item(applied.kind()) {
            return Err(Diagnostic::new(
                DiagnosticCode::InvalidDirective,
                applied.span(),
                "apply list items must be nothing or loop-nest-associated directives",
            ));
        }
        if !generally_composable && !is_loop_transforming_directive(applied.kind()) {
            return Err(Diagnostic::new(
                DiagnosticCode::InvalidDirective,
                applied.span(),
                format!(
                    "apply on {:?} accepts only loop-transforming directives",
                    directive.kind()
                ),
            ));
        }
        validate_openmp(applied, policy, applied.span())?;
        validate_nested_apply_is_transform_only(applied)?;
    }

    let generated_loops = openmp_apply_generated_loop_count(directive).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::MissingRequiredClause,
            span,
            format!(
                "cannot determine generated-loop count for apply on {:?}",
                directive.kind()
            ),
        )
    })?;
    let index_count = modifier.map_or(0, |modifier| modifier.indices.len());
    let selected_count = if index_count == 0 {
        generated_loops
    } else {
        index_count
    };
    if applied_directives.len() != selected_count {
        return Err(Diagnostic::new(
            DiagnosticCode::InvalidClause,
            span,
            format!(
                "apply has {} directives but its loop modifier selects {selected_count} loops",
                applied_directives.len()
            ),
        ));
    }
    if let Some(modifier) = modifier {
        let mut previous = 0u128;
        for index in &modifier.indices {
            if let Some(value) = reject_obvious_nonpositive(index, span, "apply index")? {
                if value > generated_loops as u128 || value <= previous {
                    return Err(Diagnostic::new(
                        DiagnosticCode::InvalidClause,
                        span,
                        "apply indices must be unique, ascending, and within the generated-loop range",
                    ));
                }
                previous = value;
            }
        }
    }
    Ok(())
}

fn is_loop_transforming_directive(kind: OmpDirectiveKind) -> bool {
    matches!(
        kind,
        OmpDirectiveKind::Fuse
            | OmpDirectiveKind::Interchange
            | OmpDirectiveKind::Nothing
            | OmpDirectiveKind::Reverse
            | OmpDirectiveKind::Split
            | OmpDirectiveKind::Stripe
            | OmpDirectiveKind::Tile
            | OmpDirectiveKind::Unroll
    )
}

fn is_loop_nest_associated_apply_item(kind: OmpDirectiveKind) -> bool {
    kind == OmpDirectiveKind::Nothing || is_loop(kind) || is_loop_transforming_directive(kind)
}

fn validate_nested_apply_is_transform_only(directive: &OmpDirective) -> Result<(), Diagnostic> {
    if !is_loop_transforming_directive(directive.kind()) {
        return Ok(());
    }
    for clause in directive.clauses() {
        let ClauseData::Apply {
            applied_directives, ..
        } = clause.payload()
        else {
            continue;
        };
        if applied_directives
            .iter()
            .any(|applied| !is_loop_transforming_directive(applied.kind()))
        {
            return Err(Diagnostic::new(
                DiagnosticCode::InvalidDirective,
                clause.span(),
                "a nested loop-transforming apply clause accepts only loop-transforming directives",
            ));
        }
    }
    Ok(())
}

fn openmp_apply_generated_loop_count(directive: &OmpDirective) -> Option<usize> {
    match directive.kind() {
        OmpDirectiveKind::Fuse
        | OmpDirectiveKind::Nothing
        | OmpDirectiveKind::Reverse
        | OmpDirectiveKind::Unroll => Some(1),
        OmpDirectiveKind::Interchange => directive
            .clauses()
            .iter()
            .find_map(|clause| match clause.payload() {
                ClauseData::Permutation { positions } => Some(positions.len()),
                _ => None,
            })
            .or(Some(2)),
        OmpDirectiveKind::Split => {
            directive
                .clauses()
                .iter()
                .find_map(|clause| match clause.payload() {
                    ClauseData::Counts { counts } => Some(counts.len()),
                    _ => None,
                })
        }
        OmpDirectiveKind::Stripe | OmpDirectiveKind::Tile => {
            directive
                .clauses()
                .iter()
                .find_map(|clause| match clause.payload() {
                    ClauseData::Sizes { sizes } => Some(sizes.len()),
                    _ => None,
                })
        }
        _ => None,
    }
}

fn effective_apply_loop_kind(
    directive: OmpDirectiveKind,
    modifier: Option<&crate::ir::OmpApplyLoopModifier>,
) -> Option<OmpApplyLoopKind> {
    modifier.map(|modifier| modifier.kind).or(match directive {
        OmpDirectiveKind::Fuse => Some(OmpApplyLoopKind::Fused),
        OmpDirectiveKind::Interchange => Some(OmpApplyLoopKind::Interchanged),
        OmpDirectiveKind::Nothing => Some(OmpApplyLoopKind::Identity),
        OmpDirectiveKind::Reverse => Some(OmpApplyLoopKind::Reversed),
        OmpDirectiveKind::Unroll => Some(OmpApplyLoopKind::Unrolled),
        OmpDirectiveKind::Split | OmpDirectiveKind::Stripe | OmpDirectiveKind::Tile => None,
        _ => None,
    })
}

fn validate_openmp_obvious_apply_index_sets(
    directive: &OmpDirective,
    span: Span,
) -> Result<(), Diagnostic> {
    let Some(generated_loops) = openmp_apply_generated_loop_count(directive) else {
        return Ok(());
    };
    let mut selected_by_kind: HashMap<OmpApplyLoopKind, HashSet<u128>> = HashMap::new();
    for clause in directive.clauses() {
        let ClauseData::Apply { loop_modifier, .. } = clause.payload() else {
            continue;
        };
        let Some(kind) = effective_apply_loop_kind(directive.kind(), loop_modifier.as_ref()) else {
            continue;
        };
        let values = match loop_modifier {
            Some(modifier) if !modifier.indices.is_empty() => {
                let Some(evaluations) = modifier
                    .indices
                    .iter()
                    .map(obvious_integer_evaluation)
                    .collect::<Option<Vec<_>>>()
                else {
                    continue;
                };
                evaluations
                    .into_iter()
                    .map(|value| match value {
                        IntegerEvaluation::NonNegative(value) if value > 0 => Ok(value),
                        IntegerEvaluation::NotInteger => Err(Diagnostic::new(
                            DiagnosticCode::InvalidExpressionType,
                            span,
                            "apply indices require integer expressions",
                        )),
                        IntegerEvaluation::Negative | IntegerEvaluation::NonNegative(0) => {
                            Err(Diagnostic::new(
                                DiagnosticCode::InvalidClause,
                                span,
                                "apply indices require positive values",
                            ))
                        }
                        IntegerEvaluation::NonNegative(_) => {
                            unreachable!("positive value matched above")
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?
            }
            Some(_) | None => (1..=generated_loops as u128).collect(),
        };
        let selected = selected_by_kind.entry(kind).or_default();
        for value in values {
            if !selected.insert(value) {
                return Err(Diagnostic::new(
                    DiagnosticCode::InvalidClause,
                    span,
                    "an apply index may appear at most once across like-modified apply clauses",
                ));
            }
        }
    }
    Ok(())
}

/// Validate context-independent OpenMP rules and every semantic fact required
/// by this directive.
///
/// Missing facts are hard errors. Callers that do not own compiler context
/// should use [`validate_openmp`] and must not claim semantic validation.
pub fn validate_openmp_with_facts(
    directive: &OmpDirective,
    policy: VersionPolicy<OpenMpVersion>,
    span: Span,
    facts: &SemanticFacts,
) -> Result<(), Diagnostic> {
    validate_openmp(directive, policy, span)?;
    require_openmp_semantic_facts(directive, policy, span, facts)
}

/// Validate all context-independent rules for one typed OpenACC directive.
pub fn validate_openacc(
    directive: &AccDirective,
    _policy: VersionPolicy<OpenAccVersion>,
    span: Span,
) -> Result<(), Diagnostic> {
    for clause in directive.clauses() {
        let Some(allowed) = openacc_clause_allowed(directive.kind(), clause.kind()) else {
            return Err(Diagnostic::new(
                DiagnosticCode::InvalidClause,
                span,
                format!(
                    "OpenACC legality catalog has no entry for clause {:?} on {:?}",
                    clause.kind(),
                    directive.kind()
                ),
            ));
        };
        if !allowed {
            return Err(Diagnostic::new(
                DiagnosticCode::ClauseNotAllowed,
                span,
                format!(
                    "OpenACC clause {:?} is not allowed on directive {:?}",
                    clause.kind(),
                    directive.kind()
                ),
            ));
        }
        validate_openacc_directive_specific_shape(directive.kind(), clause)?;
    }

    validate_openacc_clause_sets(directive, span)?;
    validate_openacc_device_type_segments(directive, span)?;
    validate_openacc_required_clauses(directive, span)
}

fn validate_openacc_directive_specific_shape(
    directive: AccDirectiveKind,
    clause: &AccClause,
) -> Result<(), Diagnostic> {
    if directive != AccDirectiveKind::Routine {
        return Ok(());
    }

    let valid = match clause.payload() {
        AccClausePayload::Gang(gang) => matches!(gang.arguments(), [] | [AccGangArgument::Dim(_)]),
        AccClausePayload::Worker(worker) => matches!(worker, AccWorkerClause::Bare),
        AccClausePayload::Vector(vector) => matches!(vector, AccVectorClause::Bare),
        _ => true,
    };
    if valid {
        Ok(())
    } else {
        Err(Diagnostic::new(
            DiagnosticCode::InvalidClause,
            clause.span(),
            format!(
                "OpenACC clause {:?} has a payload that is not standardized on routine",
                clause.kind()
            ),
        ))
    }
}

/// Validate context-independent OpenACC rules and every semantic fact required
/// by this directive.
pub fn validate_openacc_with_facts(
    directive: &AccDirective,
    policy: VersionPolicy<OpenAccVersion>,
    span: Span,
    facts: &SemanticFacts,
) -> Result<(), Diagnostic> {
    validate_openacc(directive, policy, span)?;
    if matches!(
        directive.kind(),
        AccDirectiveKind::Declare | AccDirectiveKind::Routine
    ) {
        require_declaration_position(facts, span, "OpenACC declaration directive")?;
    }
    require_openacc_semantic_facts(directive, span, facts)
}

fn require_openmp_semantic_facts(
    directive: &OmpDirective,
    policy: VersionPolicy<OpenMpVersion>,
    span: Span,
    facts: &SemanticFacts,
) -> Result<(), Diagnostic> {
    validate_omp_expression_fact_consistency(facts, span)?;

    if openmp_requires_declaration_position(directive.kind()) {
        require_declaration_position(facts, span, "OpenMP declaration directive")?;
    }

    let association = match directive.kind() {
        OmpDirectiveKind::Section => Some(AssociationKind::SectionRegion),
        OmpDirectiveKind::Scan => Some(AssociationKind::ScanWithInscanLoop),
        OmpDirectiveKind::Ordered if has_omp_clause(directive, OmpClauseKind::Doacross) => {
            Some(AssociationKind::DoacrossLoop)
        }
        _ => None,
    };
    if let Some(association) = association {
        match facts.association(association) {
            None => {
                return Err(Diagnostic::new(
                    DiagnosticCode::MissingContext,
                    span,
                    format!("missing required association fact: {association:?}"),
                ));
            }
            Some(false) => {
                return Err(Diagnostic::new(
                    DiagnosticCode::InvalidAssociation,
                    span,
                    format!("invalid directive association: {association:?}"),
                ));
            }
            Some(true) => {}
        }
    }

    let mut occurrences = HashMap::new();
    for clause in directive.clauses() {
        let occurrence = occurrences.entry(clause.kind()).or_insert(0usize);
        let site = OmpClauseSite::new(clause.kind(), *occurrence);
        *occurrence += 1;
        require_openmp_clause_semantic_facts(directive, clause, site, policy, span, facts)?;
    }
    validate_interop_depend_semantic_context(directive, span, facts)?;
    validate_openmp_atomic_semantic_facts(directive, span, facts)?;
    validate_openmp_apply_semantic_facts(directive, span, facts)?;
    validate_adjust_args_semantic_overlaps(directive, span, facts)?;
    validate_allocate_semantic_context(directive, span, facts)?;
    validate_openmp_integer_relations(directive, span, facts)
}

fn validate_interop_depend_semantic_context(
    directive: &OmpDirective,
    span: Span,
    facts: &SemanticFacts,
) -> Result<(), Diagnostic> {
    if directive.kind() != OmpDirectiveKind::Interop
        || !has_omp_clause(directive, OmpClauseKind::Depend)
    {
        return Ok(());
    }
    let mut occurrences = HashMap::new();
    let mut includes_targetsync = false;
    for clause in directive.clauses() {
        let occurrence = occurrences.entry(clause.kind()).or_insert(0usize);
        let site = OmpClauseSite::new(clause.kind(), *occurrence);
        *occurrence += 1;
        match clause.payload() {
            ClauseData::InitInterop { interop_types, .. } => {
                includes_targetsync |=
                    interop_types.contains(&crate::ir::OmpInteropType::Targetsync);
            }
            ClauseData::Use { .. } | ClauseData::Destroy { .. } => {
                includes_targetsync |= facts.interop_targetsync(site).ok_or_else(|| {
                    Diagnostic::new(
                        DiagnosticCode::MissingSemanticFact,
                        span,
                        format!(
                            "{site:?} requires a targetsync-initialization fact when interop has depend"
                        ),
                    )
                })?;
            }
            _ => {}
        }
    }
    if includes_targetsync {
        Ok(())
    } else {
        Err(Diagnostic::new(
            DiagnosticCode::ClauseNotAllowed,
            span,
            "an interop depend clause requires a targetsync interoperability type",
        ))
    }
}

fn validate_allocate_semantic_context(
    directive: &OmpDirective,
    span: Span,
    facts: &SemanticFacts,
) -> Result<(), Diagnostic> {
    let has_unqualified_allocate = directive.clauses().iter().any(|clause| {
        matches!(
            clause.payload(),
            ClauseData::Allocate {
                allocator: None,
                ..
            }
        )
    });
    if !has_unqualified_allocate || directive.kind() == OmpDirectiveKind::Allocators {
        return Ok(());
    }
    let target_context = if directive.kind() == OmpDirectiveKind::Target {
        true
    } else {
        facts.inside_target_region().ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::MissingSemanticFact,
                span,
                "an unqualified allocate clause requires an inside-target-region fact",
            )
        })?
    };
    if !target_context {
        return Ok(());
    }
    match facts.dynamic_allocators_requirement() {
        None => Err(Diagnostic::new(
            DiagnosticCode::MissingSemanticFact,
            span,
            "an unqualified allocate clause in a target context requires a dynamic_allocators requirement fact",
        )),
        Some(false) => Err(Diagnostic::new(
            DiagnosticCode::InvalidClause,
            span,
            "an allocate clause in a target context must specify an allocator unless requires dynamic_allocators is present",
        )),
        Some(true) => Ok(()),
    }
}

fn validate_adjust_args_semantic_overlaps(
    directive: &OmpDirective,
    span: Span,
    facts: &SemanticFacts,
) -> Result<(), Diagnostic> {
    let mut occurrence = 0usize;
    for clause in directive.clauses() {
        let ClauseData::AdjustArgs { parameters, .. } = clause.payload() else {
            continue;
        };
        let clause_site = OmpClauseSite::new(OmpClauseKind::AdjustArgs, occurrence);
        occurrence += 1;
        let mut expression_index = 0usize;
        let mut positions = HashSet::new();
        let mut ranges = Vec::new();
        for parameter in parameters {
            match parameter {
                OmpParameterListItem::Named(_) => {}
                OmpParameterListItem::Position(position) => {
                    positions.insert(u128::from(*position));
                }
                OmpParameterListItem::Range(range) => {
                    let lower = match range.lower() {
                        Some(lower) => {
                            let site = OmpExpressionSite::new(clause_site, expression_index);
                            expression_index += 1;
                            require_positive_constant_integer(lower, site, span, facts)?
                        }
                        None => 1,
                    };
                    let upper = match range.upper() {
                        Some(upper) => {
                            let site = OmpExpressionSite::new(clause_site, expression_index);
                            expression_index += 1;
                            Some(require_positive_constant_integer(upper, site, span, facts)?)
                        }
                        None => None,
                    };
                    if upper.is_some_and(|upper| lower > upper) {
                        return Err(Diagnostic::new(
                            DiagnosticCode::InvalidClause,
                            span,
                            "adjust_args range lower bound must not exceed its upper bound",
                        ));
                    }
                    ranges.push((lower, upper));
                }
            }
        }
        reject_adjust_args_interval_overlaps(&positions, &ranges, span)?;
    }
    Ok(())
}

fn require_foreign_runtime_preference_facts(
    preferences: &[crate::ir::OmpPreferenceSpecification],
    clause_site: OmpClauseSite,
    span: Span,
    facts: &SemanticFacts,
) -> Result<(), Diagnostic> {
    let mut expression_index = 0usize;
    require_foreign_runtime_preference_facts_at(
        preferences,
        clause_site,
        &mut expression_index,
        span,
        facts,
    )
}

fn require_foreign_runtime_preference_facts_at(
    preferences: &[crate::ir::OmpPreferenceSpecification],
    clause_site: OmpClauseSite,
    expression_index: &mut usize,
    span: Span,
    facts: &SemanticFacts,
) -> Result<(), Diagnostic> {
    let mut require_identifier =
        |identifier: &crate::ir::OmpForeignRuntimeIdentifier| -> Result<(), Diagnostic> {
            let crate::ir::OmpForeignRuntimeIdentifier::ConstantExpression(expression) = identifier
            else {
                return Ok(());
            };
            let site = OmpExpressionSite::new(clause_site, *expression_index);
            *expression_index += 1;
            match require_constant_integer(expression, site, span, facts)? {
                IntegerEvaluation::NotInteger => Err(Diagnostic::new(
                    DiagnosticCode::InvalidExpressionType,
                    span,
                    format!("{site:?} requires a constant integer foreign-runtime identifier"),
                )),
                IntegerEvaluation::Negative | IntegerEvaluation::NonNegative(_) => Ok(()),
            }
        };
    for preference in preferences {
        match preference {
            crate::ir::OmpPreferenceSpecification::ForeignRuntime(identifier) => {
                require_identifier(identifier)?;
            }
            crate::ir::OmpPreferenceSpecification::Selectors(selectors) => {
                for selector in selectors {
                    if let crate::ir::OmpPreferenceSelector::ForeignRuntime(identifier) = selector {
                        require_identifier(identifier)?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn require_selector_semantic_facts(
    selector: &OmpSelector,
    clause_site: OmpClauseSite,
    policy: VersionPolicy<OpenMpVersion>,
    span: Span,
    facts: &SemanticFacts,
) -> Result<(), Diagnostic> {
    let mut expression_index = 0usize;
    for entry in selector.entries() {
        match entry {
            OmpSelectorEntry::Device { traits } | OmpSelectorEntry::TargetDevice { traits } => {
                for trait_ in traits {
                    match trait_ {
                        OmpSelectorDeviceTrait::DeviceNum(expression) => {
                            let site = OmpExpressionSite::new(clause_site, expression_index);
                            expression_index += 1;
                            require_integer_expression(expression, site, span, facts)?;
                            require_expression_fact(
                                facts.conforming_device_number(site),
                                site,
                                span,
                                "conforming target device number",
                            )?;
                        }
                        OmpSelectorDeviceTrait::Extension(extension) => {
                            for property in extension.properties() {
                                require_selector_extension_property_facts(
                                    property,
                                    clause_site,
                                    &mut expression_index,
                                    span,
                                    facts,
                                )?;
                            }
                        }
                        OmpSelectorDeviceTrait::NameList(_) | OmpSelectorDeviceTrait::Uid(_) => {}
                    }
                }
            }
            OmpSelectorEntry::Implementation { traits } => {
                for trait_ in traits {
                    if let Some(score) = trait_.score() {
                        let site = OmpExpressionSite::new(clause_site, expression_index);
                        expression_index += 1;
                        require_nonnegative_constant_integer(score, site, span, facts)?;
                    }
                    match trait_.kind() {
                        OmpSelectorImplementationTraitKind::Requires(requirements) => {
                            for requirement in requirements {
                                if let Some(required) = requirement.required() {
                                    let site =
                                        OmpExpressionSite::new(clause_site, expression_index);
                                    expression_index += 1;
                                    require_constant_logical(required, site, span, facts)?;
                                }
                            }
                        }
                        OmpSelectorImplementationTraitKind::Extension(extension) => {
                            for property in extension.properties() {
                                require_selector_extension_property_facts(
                                    property,
                                    clause_site,
                                    &mut expression_index,
                                    span,
                                    facts,
                                )?;
                            }
                        }
                        OmpSelectorImplementationTraitKind::NameList(_)
                        | OmpSelectorImplementationTraitKind::AtomicDefaultMemOrder(_)
                        | OmpSelectorImplementationTraitKind::Requirement(_) => {}
                    }
                }
            }
            OmpSelectorEntry::User { score, condition } => {
                if let Some(score) = score {
                    let site = OmpExpressionSite::new(clause_site, expression_index);
                    expression_index += 1;
                    require_nonnegative_constant_integer(score, site, span, facts)?;
                }
                let site = OmpExpressionSite::new(clause_site, expression_index);
                require_logical_expression(condition, site, span, facts)?;
                if matches!(policy, VersionPolicy::Exact(OpenMpVersion::V5_0)) {
                    require_constant_expression(condition, site, span, facts)?;
                }
            }
            OmpSelectorEntry::Construct { .. } => {}
        }
    }
    Ok(())
}

fn require_selector_extension_property_facts(
    property: &OmpSelectorExtensionProperty,
    clause_site: OmpClauseSite,
    expression_index: &mut usize,
    span: Span,
    facts: &SemanticFacts,
) -> Result<(), Diagnostic> {
    match property {
        OmpSelectorExtensionProperty::Name(_) => Ok(()),
        OmpSelectorExtensionProperty::Call { properties, .. } => {
            for property in properties {
                require_selector_extension_property_facts(
                    property,
                    clause_site,
                    expression_index,
                    span,
                    facts,
                )?;
            }
            Ok(())
        }
        OmpSelectorExtensionProperty::ConstantInteger(expression) => {
            let site = OmpExpressionSite::new(clause_site, *expression_index);
            *expression_index += 1;
            require_constant_integer(expression, site, span, facts).map(|_| ())
        }
    }
}

fn require_potential_locator_fact(
    locator: &OmpLocator,
    clause_site: OmpClauseSite,
    locator_index: usize,
    span: Span,
    facts: &SemanticFacts,
) -> Result<(), Diagnostic> {
    if !matches!(locator, OmpLocator::PotentialLValue(_)) {
        return Ok(());
    }
    let locator_site = OmpLocatorSite::new(clause_site, locator_index);
    match facts.lvalue_locator(locator_site) {
        None => Err(Diagnostic::new(
            DiagnosticCode::MissingSemanticFact,
            span,
            format!("locator {locator_index} requires an lvalue-category fact"),
        )),
        Some(false) => Err(Diagnostic::new(
            DiagnosticCode::InvalidLocator,
            span,
            format!("locator {locator_index} is not an lvalue"),
        )),
        Some(true) => Ok(()),
    }
}

fn validate_openmp_apply_semantic_facts(
    directive: &OmpDirective,
    span: Span,
    facts: &SemanticFacts,
) -> Result<(), Diagnostic> {
    let Some(generated_loops) = openmp_apply_generated_loop_count(directive) else {
        return Ok(());
    };
    let mut occurrences = 0usize;
    let mut selected_by_kind: HashMap<OmpApplyLoopKind, HashSet<u128>> = HashMap::new();
    for clause in directive.clauses() {
        let ClauseData::Apply { loop_modifier, .. } = clause.payload() else {
            continue;
        };
        let kind = effective_apply_loop_kind(directive.kind(), loop_modifier.as_ref()).ok_or_else(
            || {
                Diagnostic::new(
                    DiagnosticCode::InvalidModifier,
                    span,
                    "apply is missing a valid effective loop modifier",
                )
            },
        )?;
        let values = if let Some(modifier) = loop_modifier {
            if modifier.indices.is_empty() {
                (1..=generated_loops as u128).collect::<Vec<_>>()
            } else {
                let clause_site = OmpClauseSite::new(OmpClauseKind::Apply, occurrences);
                let mut values = Vec::with_capacity(modifier.indices.len());
                for (index, expression) in modifier.indices.iter().enumerate() {
                    values.push(require_positive_constant_integer(
                        expression,
                        OmpExpressionSite::new(clause_site, index),
                        span,
                        facts,
                    )?);
                }
                values
            }
        } else {
            (1..=generated_loops as u128).collect::<Vec<_>>()
        };
        let mut previous = 0u128;
        let selected = selected_by_kind.entry(kind).or_default();
        for value in values {
            if value > generated_loops as u128 || value <= previous || !selected.insert(value) {
                return Err(Diagnostic::new(
                    DiagnosticCode::InvalidClause,
                    span,
                    "apply indices must be unique, ascending, and within the generated-loop range across all like-modified apply clauses",
                ));
            }
            previous = value;
        }
        occurrences += 1;
    }
    Ok(())
}

fn validate_openmp_atomic_semantic_facts(
    directive: &OmpDirective,
    span: Span,
    facts: &SemanticFacts,
) -> Result<(), Diagnostic> {
    if directive.kind() != OmpDirectiveKind::Atomic {
        return Ok(());
    }

    let weak = has_omp_clause(directive, OmpClauseKind::Weak);
    let fail = has_omp_clause(directive, OmpClauseKind::Fail);
    if weak || fail {
        let form = facts.atomic_update_form().ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::MissingSemanticFact,
                span,
                "atomic weak or fail requires an associated atomic-update-form fact",
            )
        })?;
        if weak && form != AtomicUpdateForm::EqualityConditional {
            return Err(Diagnostic::new(
                DiagnosticCode::InvalidClause,
                span,
                "atomic weak requires an equality conditional update",
            ));
        }
        if fail && form == AtomicUpdateForm::Unconditional {
            return Err(Diagnostic::new(
                DiagnosticCode::InvalidClause,
                span,
                "atomic fail requires a conditional update or the Fortran MAX/MIN form",
            ));
        }
    }

    let effective_extended = |kind: OmpClauseKind,
                              extended_kind: crate::ir::ExtendedAtomicKind|
     -> Result<Option<bool>, Diagnostic> {
        let Some(clause) = directive
            .clauses()
            .iter()
            .find(|clause| clause.kind() == kind)
        else {
            return Ok(None);
        };
        let ClauseData::ExtendedAtomic {
            kind: actual_kind,
            use_semantics,
        } = clause.payload()
        else {
            return Err(Diagnostic::new(
                DiagnosticCode::InvalidClause,
                span,
                format!("atomic clause {kind:?} has the wrong typed payload"),
            ));
        };
        if *actual_kind != extended_kind {
            return Err(Diagnostic::new(
                DiagnosticCode::InvalidClause,
                span,
                format!("atomic clause {kind:?} has a mismatched typed payload"),
            ));
        }
        use_semantics
            .as_ref()
            .map(|condition| {
                require_constant_logical(
                    condition,
                    OmpExpressionSite::new(OmpClauseSite::new(kind, 0), 0),
                    span,
                    facts,
                )
            })
            .transpose()
            .map(|value| Some(value.unwrap_or(true)))
    };

    let weak_effective =
        effective_extended(OmpClauseKind::Weak, crate::ir::ExtendedAtomicKind::Weak)?;
    let compare_effective = effective_extended(
        OmpClauseKind::Compare,
        crate::ir::ExtendedAtomicKind::Compare,
    )?;
    if weak_effective == Some(true) && compare_effective == Some(false) {
        return Err(Diagnostic::new(
            DiagnosticCode::ConflictingClauses,
            span,
            "atomic weak(true) is incompatible with compare(false)",
        ));
    }
    Ok(())
}

fn validate_omp_expression_fact_consistency(
    facts: &SemanticFacts,
    span: Span,
) -> Result<(), Diagnostic> {
    for (site, record) in &facts.omp_expressions {
        let contradictory = record.integer == Some(false)
            && (record.positive_integer == Some(true) || record.nonnegative_integer == Some(true))
            || record.nonnegative_integer == Some(false) && record.positive_integer == Some(true)
            || record.string == Some(true)
                && (record.integer == Some(true)
                    || record.positive_integer == Some(true)
                    || record.nonnegative_integer == Some(true));
        let evaluation_contradicts = match record.integer_evaluation {
            None => false,
            Some(IntegerEvaluation::NotInteger) => {
                record.constant == Some(false)
                    || record.integer == Some(true)
                    || record.positive_integer == Some(true)
                    || record.nonnegative_integer == Some(true)
            }
            Some(IntegerEvaluation::Negative) => {
                record.constant == Some(false)
                    || record.integer == Some(false)
                    || record.positive_integer == Some(true)
                    || record.nonnegative_integer == Some(true)
                    || record.string == Some(true)
            }
            Some(IntegerEvaluation::NonNegative(value)) => {
                record.constant == Some(false)
                    || record.integer == Some(false)
                    || record.nonnegative_integer == Some(false)
                    || record.positive_integer == Some(value == 0)
                    || record.string == Some(true)
            }
        };
        let logical_contradicts = record.logical_evaluation.is_some()
            && record.constant == Some(false)
            || record.logical == Some(false)
                && matches!(
                    record.logical_evaluation,
                    Some(LogicalEvaluation::False | LogicalEvaluation::True)
                )
            || record.logical == Some(true)
                && record.logical_evaluation == Some(LogicalEvaluation::NotLogical);
        let property_contradicts = record.constant == Some(true)
            && (record.region_invariant == Some(false)
                || record.ultimate == Some(false)
                || record.binding_set_invariant == Some(false))
            || matches!(
                record.integer_evaluation,
                Some(IntegerEvaluation::Negative | IntegerEvaluation::NonNegative(_))
            ) && record.linear_step == Some(false);
        if contradictory || evaluation_contradicts || logical_contradicts || property_contradicts {
            return Err(Diagnostic::new(
                DiagnosticCode::InvalidConfiguration,
                span,
                format!("contradictory semantic facts were supplied for {site:?}"),
            ));
        }
    }
    Ok(())
}

fn validate_openmp_integer_relations(
    directive: &OmpDirective,
    span: Span,
    facts: &SemanticFacts,
) -> Result<(), Diagnostic> {
    let payload = |kind| {
        directive
            .clauses()
            .iter()
            .find(|clause| clause.kind() == kind)
            .map(crate::ast::OmpClause::payload)
    };

    if let (
        Some(ClauseData::Safelen { length: safe }),
        Some(ClauseData::Simdlen { length: simd }),
    ) = (
        payload(OmpClauseKind::Safelen),
        payload(OmpClauseKind::Simdlen),
    ) {
        let safe = require_positive_constant_integer(
            safe,
            OmpExpressionSite::new(OmpClauseSite::new(OmpClauseKind::Safelen, 0), 0),
            span,
            facts,
        )?;
        let simd = require_positive_constant_integer(
            simd,
            OmpExpressionSite::new(OmpClauseSite::new(OmpClauseKind::Simdlen, 0), 0),
            span,
            facts,
        )?;
        if simd > safe {
            return Err(Diagnostic::new(
                DiagnosticCode::InvalidClause,
                span,
                "simdlen must be less than or equal to safelen",
            ));
        }
    }

    if let (
        Some(ClauseData::Collapse { n: collapsed }),
        Some(ClauseData::Ordered { n: Some(ordered) }),
    ) = (
        payload(OmpClauseKind::Collapse),
        payload(OmpClauseKind::Ordered),
    ) {
        let collapsed = require_positive_constant_integer(
            collapsed,
            OmpExpressionSite::new(OmpClauseSite::new(OmpClauseKind::Collapse, 0), 0),
            span,
            facts,
        )?;
        let ordered = require_positive_constant_integer(
            ordered,
            OmpExpressionSite::new(OmpClauseSite::new(OmpClauseKind::Ordered, 0), 0),
            span,
            facts,
        )?;
        if ordered < collapsed {
            return Err(Diagnostic::new(
                DiagnosticCode::InvalidClause,
                span,
                "ordered(n) must be greater than or equal to collapse(n)",
            ));
        }
    }

    Ok(())
}

fn require_declaration_position(
    facts: &SemanticFacts,
    span: Span,
    subject: &str,
) -> Result<(), Diagnostic> {
    match facts.declaration_position() {
        None => Err(Diagnostic::new(
            DiagnosticCode::MissingSemanticFact,
            span,
            format!("{subject} requires a caller-supplied declaration-position fact"),
        )),
        Some(false) => Err(Diagnostic::new(
            DiagnosticCode::InvalidDeclarationPosition,
            span,
            format!("{subject} is not valid at this declaration position"),
        )),
        Some(true) => Ok(()),
    }
}

fn openmp_requires_declaration_position(kind: OmpDirectiveKind) -> bool {
    matches!(
        kind,
        OmpDirectiveKind::DeclareInduction
            | OmpDirectiveKind::DeclareMapper
            | OmpDirectiveKind::DeclareReduction
            | OmpDirectiveKind::DeclareSimd
            | OmpDirectiveKind::DeclareTarget
            | OmpDirectiveKind::DeclareVariant
            | OmpDirectiveKind::BeginDeclareTarget
            | OmpDirectiveKind::BeginDeclareVariant
            | OmpDirectiveKind::Requires
            | OmpDirectiveKind::Threadprivate
            | OmpDirectiveKind::Groupprivate
    )
}

fn validate_openmp_obvious_scalar_constraints(
    clause: &crate::ast::OmpClause,
    span: Span,
) -> Result<(), Diagnostic> {
    match clause.payload() {
        ClauseData::Collapse { n } => {
            reject_obvious_nonpositive(n, span, "collapse")?;
        }
        ClauseData::Ordered { n: Some(n) } => {
            reject_obvious_nonpositive(n, span, "ordered")?;
        }
        ClauseData::Safelen { length } => {
            reject_obvious_nonpositive(length, span, "safelen")?;
        }
        ClauseData::Simdlen { length } => {
            reject_obvious_nonpositive(length, span, "simdlen")?;
        }
        ClauseData::Aligned {
            alignment: Some(alignment),
            ..
        } => {
            reject_obvious_nonpositive(alignment, span, "aligned")?;
        }
        ClauseData::Linear {
            step: Some(step), ..
        } => {
            reject_obvious_noninteger(step, span, "linear step")?;
        }
        ClauseData::Sizes { sizes } => {
            for size in sizes {
                reject_obvious_nonpositive(size, span, "sizes")?;
            }
        }
        ClauseData::Permutation { positions } => {
            for position in positions {
                reject_obvious_nonpositive(position, span, "permutation")?;
            }
        }
        ClauseData::Counts { counts } => {
            for count in counts {
                if let OmpCount::Expression(expression) = count {
                    reject_obvious_negative(expression, span, "counts")?;
                }
            }
        }
        ClauseData::Align { alignment } => {
            if let Some(value) = reject_obvious_nonpositive(alignment, span, "align")?
                && !value.is_power_of_two()
            {
                return Err(Diagnostic::new(
                    DiagnosticCode::InvalidClause,
                    span,
                    "align requires a power-of-two constant integer value",
                ));
            }
        }
        ClauseData::Allocate {
            alignment: Some(alignment),
            ..
        } => {
            if let Some(value) = reject_obvious_nonpositive(alignment, span, "allocate alignment")?
                && !value.is_power_of_two()
            {
                return Err(Diagnostic::new(
                    DiagnosticCode::InvalidClause,
                    span,
                    "allocate alignment requires a power-of-two constant integer value",
                ));
            }
        }
        ClauseData::Looprange { first, count } => {
            reject_obvious_nonpositive(first, span, "looprange first")?;
            reject_obvious_nonpositive(count, span, "looprange count")?;
        }
        ClauseData::Partial {
            unroll_factor: Some(factor),
        } => {
            reject_obvious_nonpositive(factor, span, "partial")?;
        }
        ClauseData::Nowait {
            do_not_synchronize: Some(condition),
        }
        | ClauseData::Nogroup {
            do_not_synchronize: Some(condition),
        }
        | ClauseData::If { condition }
        | ClauseData::Final { condition }
        | ClauseData::Holds { condition }
        | ClauseData::Nocontext { condition }
        | ClauseData::Novariants { condition }
        | ClauseData::GraphReset {
            condition: Some(condition),
        }
        | ClauseData::InitComplete {
            create_init_phase: Some(condition),
        }
        | ClauseData::Branch {
            condition: Some(condition),
        }
        | ClauseData::Full {
            fully_unroll: Some(condition),
        }
        | ClauseData::Mergeable {
            can_merge: Some(condition),
        }
        | ClauseData::Untied {
            can_change_threads: Some(condition),
        }
        | ClauseData::Simd {
            apply_to_simd: Some(condition),
        }
        | ClauseData::Threads {
            apply_to_threads: Some(condition),
        }
        | ClauseData::Assumption {
            can_assume: Some(condition),
        }
        | ClauseData::Indirect {
            invoked_by_fptr: Some(condition),
        }
        | ClauseData::Replayable {
            replayable_expression: Some(condition),
        }
        | ClauseData::Requirement {
            required: Some(condition),
            ..
        }
        | ClauseData::MemoryOrder {
            use_semantics: Some(condition),
            ..
        }
        | ClauseData::AtomicOperation {
            use_semantics: Some(condition),
            ..
        }
        | ClauseData::ExtendedAtomic {
            use_semantics: Some(condition),
            ..
        } => {
            reject_obvious_nonlogical(condition, span, "OpenMP condition")?;
        }
        ClauseData::Schedule {
            chunk_size: Some(chunk),
            ..
        } => {
            reject_obvious_nonpositive(chunk, span, "schedule chunk size")?;
        }
        ClauseData::DistSchedule {
            chunk_size: Some(chunk),
            ..
        } => {
            reject_obvious_nonpositive(chunk, span, "dist_schedule chunk size")?;
        }
        ClauseData::Grainsize { grain, .. } => {
            reject_obvious_nonpositive(grain, span, "grainsize")?;
        }
        ClauseData::NumTasks { num, .. } => {
            reject_obvious_nonpositive(num, span, "num_tasks")?;
        }
        ClauseData::NumThreads { nthreads, .. } => {
            for nthreads in nthreads {
                reject_obvious_nonpositive(nthreads, span, "num_threads")?;
            }
        }
        ClauseData::NumTeams {
            lower_bound,
            upper_bound,
        } => {
            let lower = lower_bound
                .as_ref()
                .map(|lower| reject_obvious_nonpositive(lower, span, "num_teams lower bound"))
                .transpose()?
                .flatten();
            let upper = reject_obvious_nonpositive(upper_bound, span, "num_teams upper bound")?;
            if let (Some(lower), Some(upper)) = (lower, upper)
                && lower > upper
            {
                return Err(Diagnostic::new(
                    DiagnosticCode::InvalidClause,
                    span,
                    "num_teams lower bound must not exceed its upper bound",
                ));
            }
        }
        ClauseData::ThreadLimit { limit } => {
            reject_obvious_nonpositive(limit, span, "thread_limit")?;
        }
        ClauseData::Priority { priority } => {
            reject_obvious_negative(priority, span, "priority")?;
        }
        ClauseData::Safesync { width: Some(width) } => {
            reject_obvious_nonpositive(width, span, "safesync")?;
        }
        ClauseData::Filter { thread_num } => {
            reject_obvious_noninteger(thread_num, span, "filter")?;
        }
        ClauseData::GraphId { value } => {
            reject_obvious_noninteger(value, span, "graph_id")?;
        }
        ClauseData::Hint { value } => {
            reject_obvious_noninteger(value, span, "hint")?;
        }
        ClauseData::Device {
            modifier: Some(crate::ir::DeviceModifier::Ancestor),
            device_num,
        } => {
            if let Some(evaluation) = obvious_integer_evaluation(device_num) {
                if evaluation == IntegerEvaluation::NotInteger {
                    return Err(Diagnostic::new(
                        DiagnosticCode::InvalidExpressionType,
                        span,
                        "device requires an integer expression",
                    ));
                }
                if evaluation != IntegerEvaluation::NonNegative(1) {
                    return Err(Diagnostic::new(
                        DiagnosticCode::InvalidClause,
                        span,
                        "device(ancestor: ...) requires a constant value of exactly one",
                    ));
                }
            }
        }
        ClauseData::Device { device_num, .. } => {
            reject_obvious_noninteger(device_num, span, "device")?;
        }
        ClauseData::Doacross {
            iteration: crate::ir::OmpDoacrossIteration::Vector(vector),
            ..
        } => {
            for item in vector {
                if let Some(offset) = &item.offset {
                    let expression = match offset {
                        crate::ir::OmpDoacrossOffset::Add(expression)
                        | crate::ir::OmpDoacrossOffset::Subtract(expression) => expression,
                    };
                    reject_obvious_negative(expression, span, "doacross offset")?;
                }
            }
        }
        ClauseData::MetadirectiveSelector { selector } => {
            validate_selector_obvious_constraints(selector, span)?;
        }
        _ => {}
    }
    Ok(())
}

fn validate_selector_obvious_constraints(
    selector: &OmpSelector,
    span: Span,
) -> Result<(), Diagnostic> {
    for entry in selector.entries() {
        match entry {
            OmpSelectorEntry::Device { traits } | OmpSelectorEntry::TargetDevice { traits } => {
                for trait_ in traits {
                    match trait_ {
                        OmpSelectorDeviceTrait::DeviceNum(expression) => {
                            reject_obvious_negative(expression, span, "target_device device_num")?;
                        }
                        OmpSelectorDeviceTrait::Extension(extension) => {
                            for property in extension.properties() {
                                validate_selector_extension_property_obvious(property, span)?;
                            }
                        }
                        OmpSelectorDeviceTrait::NameList(_) | OmpSelectorDeviceTrait::Uid(_) => {}
                    }
                }
            }
            OmpSelectorEntry::Implementation { traits } => {
                for trait_ in traits {
                    if let Some(score) = trait_.score() {
                        reject_obvious_negative(score, span, "selector score")?;
                    }
                    match trait_.kind() {
                        OmpSelectorImplementationTraitKind::Requires(requirements) => {
                            for requirement in requirements {
                                if let Some(required) = requirement.required() {
                                    reject_obvious_nonlogical(
                                        required,
                                        span,
                                        "requires selector condition",
                                    )?;
                                }
                            }
                        }
                        OmpSelectorImplementationTraitKind::Extension(extension) => {
                            for property in extension.properties() {
                                validate_selector_extension_property_obvious(property, span)?;
                            }
                        }
                        OmpSelectorImplementationTraitKind::NameList(_)
                        | OmpSelectorImplementationTraitKind::AtomicDefaultMemOrder(_)
                        | OmpSelectorImplementationTraitKind::Requirement(_) => {}
                    }
                }
            }
            OmpSelectorEntry::User { score, condition } => {
                if let Some(score) = score {
                    reject_obvious_negative(score, span, "selector score")?;
                }
                reject_obvious_nonlogical(condition, span, "user selector condition")?;
            }
            OmpSelectorEntry::Construct { .. } => {}
        }
    }
    Ok(())
}

fn validate_selector_extension_property_obvious(
    property: &OmpSelectorExtensionProperty,
    span: Span,
) -> Result<(), Diagnostic> {
    match property {
        OmpSelectorExtensionProperty::Name(_) => Ok(()),
        OmpSelectorExtensionProperty::Call { properties, .. } => {
            for property in properties {
                validate_selector_extension_property_obvious(property, span)?;
            }
            Ok(())
        }
        OmpSelectorExtensionProperty::ConstantInteger(expression) => {
            reject_obvious_noninteger(expression, span, "selector extension property")
        }
    }
}

fn reject_obvious_nonpositive(
    expression: &Expression,
    span: Span,
    subject: &str,
) -> Result<Option<u128>, Diagnostic> {
    match obvious_integer_evaluation(expression) {
        None => Ok(None),
        Some(IntegerEvaluation::NonNegative(value)) if value > 0 => Ok(Some(value)),
        Some(IntegerEvaluation::NotInteger) => Err(Diagnostic::new(
            DiagnosticCode::InvalidExpressionType,
            span,
            format!("{subject} requires an integer expression"),
        )),
        Some(IntegerEvaluation::Negative | IntegerEvaluation::NonNegative(0)) => {
            Err(Diagnostic::new(
                DiagnosticCode::InvalidClause,
                span,
                format!("{subject} requires a positive integer value"),
            ))
        }
        Some(IntegerEvaluation::NonNegative(_)) => unreachable!("positive value matched above"),
    }
}

fn reject_obvious_negative(
    expression: &Expression,
    span: Span,
    subject: &str,
) -> Result<Option<u128>, Diagnostic> {
    match obvious_integer_evaluation(expression) {
        None => Ok(None),
        Some(IntegerEvaluation::NonNegative(value)) => Ok(Some(value)),
        Some(IntegerEvaluation::NotInteger) => Err(Diagnostic::new(
            DiagnosticCode::InvalidExpressionType,
            span,
            format!("{subject} requires an integer expression"),
        )),
        Some(IntegerEvaluation::Negative) => Err(Diagnostic::new(
            DiagnosticCode::InvalidClause,
            span,
            format!("{subject} requires a non-negative integer value"),
        )),
    }
}

fn reject_obvious_noninteger(
    expression: &Expression,
    span: Span,
    subject: &str,
) -> Result<(), Diagnostic> {
    if obvious_integer_evaluation(expression) == Some(IntegerEvaluation::NotInteger) {
        Err(Diagnostic::new(
            DiagnosticCode::InvalidExpressionType,
            span,
            format!("{subject} requires an integer expression"),
        ))
    } else {
        Ok(())
    }
}

fn reject_obvious_nonlogical(
    expression: &Expression,
    span: Span,
    subject: &str,
) -> Result<(), Diagnostic> {
    if obvious_logical_evaluation(expression) == Some(LogicalEvaluation::NotLogical) {
        Err(Diagnostic::new(
            DiagnosticCode::InvalidExpressionType,
            span,
            format!("{subject} requires an OpenMP logical expression"),
        ))
    } else {
        Ok(())
    }
}

fn validate_openmp_obvious_relations(
    directive: &OmpDirective,
    span: Span,
) -> Result<(), Diagnostic> {
    let payload = |kind| {
        directive
            .clauses()
            .iter()
            .find(|clause| clause.kind() == kind)
            .map(crate::ast::OmpClause::payload)
    };
    if let (Some(ClauseData::Safelen { length: safe }), Some(ClauseData::Simdlen { length: simd })) = (
        payload(OmpClauseKind::Safelen),
        payload(OmpClauseKind::Simdlen),
    ) && let (
        Some(IntegerEvaluation::NonNegative(safe)),
        Some(IntegerEvaluation::NonNegative(simd)),
    ) = (
        obvious_integer_evaluation(safe),
        obvious_integer_evaluation(simd),
    ) && simd > safe
    {
        return Err(Diagnostic::new(
            DiagnosticCode::InvalidClause,
            span,
            "simdlen must be less than or equal to safelen",
        ));
    }
    if let (
        Some(ClauseData::Collapse { n: collapsed }),
        Some(ClauseData::Ordered { n: Some(ordered) }),
    ) = (
        payload(OmpClauseKind::Collapse),
        payload(OmpClauseKind::Ordered),
    ) && let (
        Some(IntegerEvaluation::NonNegative(collapsed)),
        Some(IntegerEvaluation::NonNegative(ordered)),
    ) = (
        obvious_integer_evaluation(collapsed),
        obvious_integer_evaluation(ordered),
    ) && ordered < collapsed
    {
        return Err(Diagnostic::new(
            DiagnosticCode::InvalidClause,
            span,
            "ordered(n) must be greater than or equal to collapse(n)",
        ));
    }
    Ok(())
}

fn require_openmp_clause_semantic_facts(
    directive: &OmpDirective,
    clause: &crate::ast::OmpClause,
    clause_site: OmpClauseSite,
    policy: VersionPolicy<OpenMpVersion>,
    span: Span,
    facts: &SemanticFacts,
) -> Result<(), Diagnostic> {
    let directive_kind = directive.kind();
    let expression_site = |index| OmpExpressionSite::new(clause_site, index);
    match clause.payload() {
        ClauseData::Collapse { n } => {
            require_positive_constant_integer(n, expression_site(0), span, facts)?;
        }
        ClauseData::Ordered { n: Some(n) } => {
            require_positive_constant_integer(n, expression_site(0), span, facts)?;
        }
        ClauseData::Safelen { length } | ClauseData::Simdlen { length } => {
            require_positive_constant_integer(length, expression_site(0), span, facts)?;
        }
        ClauseData::Aligned { items, alignment } => {
            if let Some(alignment) = alignment {
                let site = expression_site(0);
                if matches!(policy, VersionPolicy::Exact(version) if version <= OpenMpVersion::V5_1)
                {
                    require_positive_constant_integer(alignment, site, span, facts)?;
                } else {
                    require_positive_integer_expression(alignment, site, span, facts)?;
                    require_expression_property_unless_constant(
                        alignment,
                        site,
                        span,
                        facts,
                        facts.region_invariant_expression(site),
                        "aligned alignment requires a region-invariant expression",
                    )?;
                    require_expression_property_unless_constant(
                        alignment,
                        site,
                        span,
                        facts,
                        facts.ultimate_expression(site),
                        "aligned alignment requires an ultimate expression",
                    )?;
                }
            }
            if directive_kind == OmpDirectiveKind::DeclareSimd {
                require_procedure_parameter_items(clause_site, items.len(), span, facts)?;
            }
        }
        ClauseData::Linear { items, step, .. } => {
            for index in 0..items.len() {
                let item_site = OmpClauseItemSite::new(clause_site, index);
                require_item_fact(
                    facts.linear_item(item_site),
                    item_site,
                    span,
                    "linear list-item semantic",
                )?;
            }
            if directive_kind == OmpDirectiveKind::DeclareSimd {
                require_procedure_parameter_items(clause_site, items.len(), span, facts)?;
            }
            if let Some(step) = step {
                let site = expression_site(0);
                require_integer_expression(step, site, span, facts)?;
                require_expression_property_unless_constant(
                    step,
                    site,
                    span,
                    facts,
                    facts.linear_step(site),
                    "linear step does not satisfy the region-invariance and declare_simd restrictions",
                )?;
            }
        }
        ClauseData::Induction { items, .. } => {
            let step_site = expression_site(0);
            require_expression_fact(
                facts.induction_step(step_site),
                step_site,
                span,
                "induction-step semantic",
            )?;
            for index in 0..items.len() {
                let item_site = OmpClauseItemSite::new(clause_site, index);
                require_item_fact(
                    facts.induction_item(item_site),
                    item_site,
                    span,
                    "induction list-item semantic",
                )?;
            }
        }
        ClauseData::Collector { .. } => {
            let site = expression_site(0);
            require_expression_type_fact(
                facts.collector_expression(site),
                site,
                span,
                "OpenMP collector expression",
            )?;
        }
        ClauseData::Inductor { .. } => {
            let site = expression_site(0);
            require_expression_type_fact(
                facts.inductor_expression(site),
                site,
                span,
                "OpenMP inductor expression",
            )?;
        }
        ClauseData::UsesAllocators { allocators } => {
            for (index, allocator) in allocators.iter().enumerate() {
                if allocator.traits().is_none() {
                    continue;
                }
                let item_site = OmpClauseItemSite::new(clause_site, index);
                require_item_fact(
                    facts.allocator_traits(item_site),
                    item_site,
                    span,
                    "uses_allocators traits-array semantic",
                )?;
            }
        }
        ClauseData::Uniform { parameters } => {
            require_procedure_parameter_items(clause_site, parameters.len(), span, facts)?;
        }
        ClauseData::AdjustArgs { parameters, .. } => {
            let mut expression_index = 0usize;
            for (item_index, parameter) in parameters.iter().enumerate() {
                match parameter {
                    OmpParameterListItem::Named(_) => {
                        require_procedure_parameter_item(
                            OmpClauseItemSite::new(clause_site, item_index),
                            span,
                            facts,
                        )?;
                    }
                    OmpParameterListItem::Position(_) => {}
                    OmpParameterListItem::Range(range) => {
                        let lower_value = range
                            .lower()
                            .map(|lower| {
                                let site = expression_site(expression_index);
                                expression_index += 1;
                                require_positive_constant_integer(lower, site, span, facts)
                            })
                            .transpose()?;
                        let upper_value = range
                            .upper()
                            .map(|upper| {
                                let site = expression_site(expression_index);
                                expression_index += 1;
                                require_positive_constant_integer(upper, site, span, facts)
                            })
                            .transpose()?;
                        if let (Some(lower), Some(upper)) = (lower_value, upper_value)
                            && lower > upper
                        {
                            return Err(Diagnostic::new(
                                DiagnosticCode::InvalidClause,
                                span,
                                "adjust_args range lower bound must not exceed its upper bound",
                            ));
                        }
                    }
                }
            }
        }
        ClauseData::InitInterop { preferences, .. } => {
            require_foreign_runtime_preference_facts(preferences, clause_site, span, facts)?;
            let item_site = OmpClauseItemSite::new(clause_site, 0);
            require_interop_object(item_site, false, span, facts)?;
            require_modifiable_item(item_site, span, facts, "interop init variable")?;
        }
        ClauseData::AppendArgs { operations } => {
            let mut expression_index = 0usize;
            for operation in operations {
                let crate::ir::OmpAppendOperation::Interop(modifiers) = operation;
                require_foreign_runtime_preference_facts_at(
                    &modifiers.preferences,
                    clause_site,
                    &mut expression_index,
                    span,
                    facts,
                )?;
            }
        }
        ClauseData::MetadirectiveSelector { selector } => {
            require_selector_semantic_facts(selector, clause_site, policy, span, facts)?;
        }
        ClauseData::InitDepobj { locator, .. } => {
            require_potential_locator_fact(locator, clause_site, 0, span, facts)?;
            let item_site = OmpClauseItemSite::new(clause_site, 0);
            require_depend_object(item_site, DependObjectState::Uninitialized, span, facts)?;
            require_modifiable_item(item_site, span, facts, "depobj init variable")?;
        }
        ClauseData::Depend { dependence, .. } => match dependence {
            crate::ir::OmpDependence::Locators { locators, .. } => {
                for (index, locator) in locators.iter().enumerate() {
                    require_potential_locator_fact(locator, clause_site, index, span, facts)?;
                }
                if directive_kind == OmpDirectiveKind::Depobj {
                    let item_site = OmpClauseItemSite::new(clause_site, 0);
                    require_depend_object(
                        item_site,
                        DependObjectState::Uninitialized,
                        span,
                        facts,
                    )?;
                    require_modifiable_item(item_site, span, facts, "depobj target")?;
                }
            }
            crate::ir::OmpDependence::Depobjs { objects } => {
                for index in 0..objects.len() {
                    let item_site = OmpClauseItemSite::new(clause_site, index);
                    require_depend_object(item_site, DependObjectState::Initialized, span, facts)?;
                }
            }
        },
        ClauseData::DepobjUpdate { .. } if directive_kind == OmpDirectiveKind::Depobj => {
            let item_site = OmpClauseItemSite::new(clause_site, 0);
            require_depend_object(item_site, DependObjectState::Initialized, span, facts)?;
        }
        ClauseData::Destroy { .. } if directive_kind == OmpDirectiveKind::Depobj => {
            let item_site = OmpClauseItemSite::new(clause_site, 0);
            require_depend_object(item_site, DependObjectState::Initialized, span, facts)?;
            require_modifiable_item(item_site, span, facts, "depobj destroy variable")?;
        }
        ClauseData::Use { .. } if directive_kind == OmpDirectiveKind::Interop => {
            let item_site = OmpClauseItemSite::new(clause_site, 0);
            require_interop_object(item_site, true, span, facts)?;
        }
        ClauseData::Destroy { .. } if directive_kind == OmpDirectiveKind::Interop => {
            let item_site = OmpClauseItemSite::new(clause_site, 0);
            require_interop_object(item_site, true, span, facts)?;
            require_modifiable_item(item_site, span, facts, "interop destroy variable")?;
        }
        ClauseData::Detach { .. } => {
            let item_site = OmpClauseItemSite::new(clause_site, 0);
            require_detach_event(item_site, span, facts)?;
            match facts.encountering_final_task() {
                None => {
                    return Err(Diagnostic::new(
                        DiagnosticCode::MissingSemanticFact,
                        span,
                        "detach requires an encountering-final-task fact",
                    ));
                }
                Some(true) => {
                    return Err(Diagnostic::new(
                        DiagnosticCode::InvalidAssociation,
                        span,
                        "a detach clause may not be encountered by a final task",
                    ));
                }
                Some(false) => {}
            }
        }
        ClauseData::Affinity { locators, .. } => {
            for (index, locator) in locators.iter().enumerate() {
                require_potential_locator_fact(locator, clause_site, index, span, facts)?;
            }
        }
        ClauseData::Doacross {
            iteration: crate::ir::OmpDoacrossIteration::Vector(vector),
            ..
        } => {
            match facts.associated_ordered_parameter() {
                None => {
                    return Err(Diagnostic::new(
                        DiagnosticCode::MissingSemanticFact,
                        span,
                        "a doacross vector requires the associated ordered(n) parameter fact",
                    ));
                }
                Some(dimensions) if dimensions != vector.len() => {
                    return Err(Diagnostic::new(
                        DiagnosticCode::InvalidAssociation,
                        span,
                        format!(
                            "doacross vector has {} dimensions but associated ordered has {dimensions}",
                            vector.len()
                        ),
                    ));
                }
                Some(_) => {}
            }
            let mut expression_index = 0usize;
            for item in vector {
                let Some(offset) = &item.offset else {
                    continue;
                };
                let expression = match offset {
                    crate::ir::OmpDoacrossOffset::Add(expression)
                    | crate::ir::OmpDoacrossOffset::Subtract(expression) => expression,
                };
                require_nonnegative_constant_integer(
                    expression,
                    expression_site(expression_index),
                    span,
                    facts,
                )?;
                expression_index += 1;
            }
        }
        ClauseData::Sizes { sizes } => {
            for (index, size) in sizes.iter().enumerate() {
                if matches!(policy, VersionPolicy::Exact(version) if version < OpenMpVersion::V6_0)
                {
                    require_positive_constant_integer(size, expression_site(index), span, facts)?;
                } else {
                    require_positive_integer_expression(size, expression_site(index), span, facts)?;
                }
            }
        }
        ClauseData::Permutation { positions } => {
            let mut seen = HashSet::new();
            for (index, position) in positions.iter().enumerate() {
                let value = require_positive_constant_integer(
                    position,
                    expression_site(index),
                    span,
                    facts,
                )?;
                if value > positions.len() as u128 || !seen.insert(value) {
                    return Err(Diagnostic::new(
                        DiagnosticCode::InvalidClause,
                        span,
                        "permutation values must contain every integer in 1..=n exactly once",
                    ));
                }
            }
            if seen.len() != positions.len() {
                return Err(Diagnostic::new(
                    DiagnosticCode::InvalidClause,
                    span,
                    "permutation values must contain every integer in 1..=n exactly once",
                ));
            }
        }
        ClauseData::Counts { counts } => {
            for (index, count) in counts.iter().enumerate() {
                if let OmpCount::Expression(expression) = count {
                    require_nonnegative_constant_integer(
                        expression,
                        expression_site(index),
                        span,
                        facts,
                    )?;
                }
            }
        }
        ClauseData::Align { alignment } => {
            let value =
                require_positive_constant_integer(alignment, expression_site(0), span, facts)?;
            if !value.is_power_of_two() {
                return Err(Diagnostic::new(
                    DiagnosticCode::InvalidClause,
                    span,
                    "align requires a power-of-two constant integer value",
                ));
            }
        }
        ClauseData::Allocate {
            allocator,
            alignment,
            ..
        } => {
            let mut index = 0usize;
            if allocator.is_some() {
                let site = expression_site(index);
                require_expression_type_fact(
                    facts.allocator_handle_expression(site),
                    site,
                    span,
                    "OpenMP allocator-handle expression",
                )?;
                index += 1;
            }
            if let Some(alignment) = alignment {
                let site = expression_site(index);
                let value = require_positive_constant_integer(alignment, site, span, facts)?;
                if !value.is_power_of_two() {
                    return Err(Diagnostic::new(
                        DiagnosticCode::InvalidClause,
                        span,
                        "allocate alignment requires a power-of-two constant integer value",
                    ));
                }
            }
        }
        ClauseData::Allocator { .. } => {
            let site = expression_site(0);
            require_expression_type_fact(
                facts.allocator_handle_expression(site),
                site,
                span,
                "OpenMP allocator-handle expression",
            )?;
        }
        ClauseData::Looprange { first, count } => {
            require_positive_constant_integer(first, expression_site(0), span, facts)?;
            require_positive_constant_integer(count, expression_site(1), span, facts)?;
        }
        ClauseData::Partial {
            unroll_factor: Some(factor),
        } => {
            require_positive_constant_integer(factor, expression_site(0), span, facts)?;
        }
        ClauseData::Nowait {
            do_not_synchronize: Some(condition),
        } => {
            let site = expression_site(0);
            require_logical_expression(condition, site, span, facts)?;
            require_expression_property_unless_constant(
                condition,
                site,
                span,
                facts,
                facts.binding_set_invariant_expression(site),
                "nowait argument must be invariant across its binding thread/task set",
            )?;
        }
        ClauseData::Nogroup {
            do_not_synchronize: Some(condition),
        }
        | ClauseData::If { condition }
        | ClauseData::Final { condition }
        | ClauseData::Holds { condition }
        | ClauseData::Nocontext { condition }
        | ClauseData::Novariants { condition }
        | ClauseData::GraphReset {
            condition: Some(condition),
        } => {
            require_logical_expression(condition, expression_site(0), span, facts)?;
        }
        ClauseData::InitComplete {
            create_init_phase: Some(condition),
        }
        | ClauseData::Branch {
            condition: Some(condition),
        }
        | ClauseData::Full {
            fully_unroll: Some(condition),
        }
        | ClauseData::Mergeable {
            can_merge: Some(condition),
        }
        | ClauseData::Untied {
            can_change_threads: Some(condition),
        }
        | ClauseData::Simd {
            apply_to_simd: Some(condition),
        }
        | ClauseData::Threads {
            apply_to_threads: Some(condition),
        }
        | ClauseData::Assumption {
            can_assume: Some(condition),
        }
        | ClauseData::Replayable {
            replayable_expression: Some(condition),
        }
        | ClauseData::Requirement {
            required: Some(condition),
            ..
        }
        | ClauseData::MemoryOrder {
            use_semantics: Some(condition),
            ..
        }
        | ClauseData::AtomicOperation {
            use_semantics: Some(condition),
            ..
        }
        | ClauseData::ExtendedAtomic {
            use_semantics: Some(condition),
            ..
        } => {
            require_constant_logical(condition, expression_site(0), span, facts)?;
        }
        ClauseData::Indirect {
            invoked_by_fptr: Some(condition),
        } => {
            let enabled = require_constant_logical(condition, expression_site(0), span, facts)?;
            if enabled
                && directive.clauses().iter().any(|clause| {
                    matches!(
                        clause.payload(),
                        ClauseData::DeviceType(device_type)
                            if *device_type != crate::ir::DeviceType::Any
                    )
                })
            {
                return Err(Diagnostic::new(
                    DiagnosticCode::ConflictingClauses,
                    span,
                    "indirect(true) only permits device_type(any)",
                ));
            }
        }
        ClauseData::Schedule {
            chunk_size: Some(chunk),
            ..
        }
        | ClauseData::DistSchedule {
            chunk_size: Some(chunk),
            ..
        } => {
            require_positive_integer_expression(chunk, expression_site(0), span, facts)?;
        }
        ClauseData::Grainsize { grain, .. } => {
            require_positive_integer_expression(grain, expression_site(0), span, facts)?;
        }
        ClauseData::NumTasks { num, .. } | ClauseData::ThreadLimit { limit: num } => {
            require_positive_integer_expression(num, expression_site(0), span, facts)?;
        }
        ClauseData::NumThreads { nthreads, .. } => {
            for (index, nthreads) in nthreads.iter().enumerate() {
                require_positive_integer_expression(nthreads, expression_site(index), span, facts)?;
            }
        }
        ClauseData::NumTeams {
            lower_bound,
            upper_bound,
        } => {
            let upper_index = usize::from(lower_bound.is_some());
            if let Some(lower_bound) = lower_bound {
                require_positive_integer_expression(lower_bound, expression_site(0), span, facts)?;
            }
            require_positive_integer_expression(
                upper_bound,
                expression_site(upper_index),
                span,
                facts,
            )?;
            if let Some(lower_bound) = lower_bound {
                require_expression_property_unless_constant(
                    lower_bound,
                    expression_site(0),
                    span,
                    facts,
                    facts.ultimate_expression(expression_site(0)),
                    "num_teams lower bound requires an ultimate expression",
                )?;
                let lower = obvious_integer_evaluation(lower_bound)
                    .or_else(|| facts.integer_evaluation(expression_site(0)));
                let upper = obvious_integer_evaluation(upper_bound)
                    .or_else(|| facts.integer_evaluation(expression_site(upper_index)));
                let supplied_order = facts.ordered_bounds(clause_site);
                if let (
                    Some(IntegerEvaluation::NonNegative(lower)),
                    Some(IntegerEvaluation::NonNegative(upper)),
                ) = (lower, upper)
                {
                    let actual_order = lower <= upper;
                    if supplied_order.is_some_and(|supplied| supplied != actual_order) {
                        return Err(Diagnostic::new(
                            DiagnosticCode::InvalidConfiguration,
                            span,
                            "num_teams bound-order fact contradicts evaluated bounds",
                        ));
                    }
                    if !actual_order {
                        return Err(Diagnostic::new(
                            DiagnosticCode::InvalidClause,
                            span,
                            "num_teams lower bound must not exceed its upper bound",
                        ));
                    }
                } else {
                    match supplied_order {
                        None => {
                            return Err(Diagnostic::new(
                                DiagnosticCode::MissingSemanticFact,
                                span,
                                "num_teams bounds require a lower-not-greater-than-upper fact",
                            ));
                        }
                        Some(false) => {
                            return Err(Diagnostic::new(
                                DiagnosticCode::InvalidClause,
                                span,
                                "num_teams lower bound must not exceed its upper bound",
                            ));
                        }
                        Some(true) => {}
                    }
                }
            }
        }
        ClauseData::Priority { priority } => {
            require_nonnegative_constant_integer(priority, expression_site(0), span, facts)?;
        }
        ClauseData::Filter { thread_num } | ClauseData::GraphId { value: thread_num } => {
            require_integer_expression(thread_num, expression_site(0), span, facts)?;
        }
        ClauseData::Device {
            modifier: Some(crate::ir::DeviceModifier::Ancestor),
            device_num,
        } => {
            let value = require_constant_integer(device_num, expression_site(0), span, facts)?;
            if value != IntegerEvaluation::NonNegative(1) {
                return Err(Diagnostic::new(
                    DiagnosticCode::InvalidClause,
                    span,
                    "device(ancestor: ...) requires a constant value of exactly one",
                ));
            }
        }
        ClauseData::Device { device_num, .. } => {
            require_integer_expression(device_num, expression_site(0), span, facts)?;
        }
        ClauseData::Safesync { width: Some(width) } => {
            let site = expression_site(0);
            require_positive_integer_expression(width, site, span, facts)?;
            require_expression_fact(
                facts.safesync_compatible(site),
                site,
                span,
                "safesync-compatible expression",
            )?;
        }
        ClauseData::Hint { .. } => {
            let site = expression_site(0);
            require_expression_fact(
                facts.synchronization_hint(site),
                site,
                span,
                "valid synchronization-hint expression",
            )?;
        }
        ClauseData::Transparent {
            impex_type: Some(_),
        } => {
            let site = expression_site(0);
            require_expression_type_fact(
                facts.impex_expression(site),
                site,
                span,
                "OpenMP impex-type expression",
            )?;
        }
        ClauseData::Apply {
            loop_modifier: Some(modifier),
            ..
        } => {
            for (index, expression) in modifier.indices.iter().enumerate() {
                require_positive_constant_integer(expression, expression_site(index), span, facts)?;
            }
        }
        ClauseData::Message { value } => {
            let site = expression_site(0);
            if !expression_is_string_literal(value) {
                match facts.string_expression(site) {
                    None => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::MissingSemanticFact,
                            span,
                            "message requires a string-expression type fact",
                        ));
                    }
                    Some(false) => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::InvalidExpressionType,
                            span,
                            "message requires an expression of string OpenMP type",
                        ));
                    }
                    Some(true) => {}
                }
            }
            let compilation_time = directive_kind == OmpDirectiveKind::Error
                && !directive.clauses().iter().any(|clause| {
                    matches!(
                        clause.payload(),
                        ClauseData::At(crate::ir::AtKind::Execution)
                    )
                });
            if compilation_time {
                require_constant_expression(value, site, span, facts)?;
            }
        }
        ClauseData::Enter {
            automap: true,
            items,
        } => {
            for index in 0..items.len() {
                let item_site = OmpClauseItemSite::new(clause_site, index);
                match facts.allocatable_item(item_site) {
                    None => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::MissingSemanticFact,
                            span,
                            format!(
                                "declare_target enter(automap: ...) item {index} requires an allocatable-item fact"
                            ),
                        ));
                    }
                    Some(false) => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::InvalidClause,
                            span,
                            format!(
                                "declare_target enter(automap: ...) item {index} is not allocatable"
                            ),
                        ));
                    }
                    Some(true) => {}
                }
            }
        }
        ClauseData::Map { locators, .. }
        | ClauseData::To { locators, .. }
        | ClauseData::From { locators, .. } => {
            for (index, locator) in locators.iter().enumerate() {
                if !matches!(locator, OmpLocator::PotentialLValue(_)) {
                    continue;
                }
                let locator_site = OmpLocatorSite::new(clause_site, index);
                match facts.lvalue_locator(locator_site) {
                    None => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::MissingSemanticFact,
                            span,
                            format!("data-motion locator {index} requires an lvalue-category fact"),
                        ));
                    }
                    Some(false) => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::InvalidLocator,
                            span,
                            format!("data-motion locator {index} is not an lvalue"),
                        ));
                    }
                    Some(true) => {}
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn require_expression_property_unless_constant(
    expression: &Expression,
    site: OmpExpressionSite,
    span: Span,
    facts: &SemanticFacts,
    property: Option<bool>,
    description: &str,
) -> Result<(), Diagnostic> {
    let proven_constant = matches!(
        obvious_integer_evaluation(expression),
        Some(IntegerEvaluation::Negative | IntegerEvaluation::NonNegative(_))
    ) || matches!(
        obvious_logical_evaluation(expression),
        Some(LogicalEvaluation::False | LogicalEvaluation::True)
    ) || matches!(
        facts.integer_evaluation(site),
        Some(IntegerEvaluation::Negative | IntegerEvaluation::NonNegative(_))
    ) || facts.constant_expression(site) == Some(true);
    if proven_constant {
        return Ok(());
    }
    require_expression_fact(property, site, span, description)
}

fn require_expression_fact(
    fact: Option<bool>,
    site: OmpExpressionSite,
    span: Span,
    description: &str,
) -> Result<(), Diagnostic> {
    match fact {
        None => Err(Diagnostic::new(
            DiagnosticCode::MissingSemanticFact,
            span,
            format!("{site:?} requires a {description} fact"),
        )),
        Some(false) => Err(Diagnostic::new(
            DiagnosticCode::InvalidClause,
            span,
            format!("{site:?}: {description} restriction is not satisfied"),
        )),
        Some(true) => Ok(()),
    }
}

fn require_expression_type_fact(
    fact: Option<bool>,
    site: OmpExpressionSite,
    span: Span,
    description: &str,
) -> Result<(), Diagnostic> {
    match fact {
        None => Err(Diagnostic::new(
            DiagnosticCode::MissingSemanticFact,
            span,
            format!("{site:?} requires a {description} fact"),
        )),
        Some(false) => Err(Diagnostic::new(
            DiagnosticCode::InvalidExpressionType,
            span,
            format!("{site:?} is not a {description}"),
        )),
        Some(true) => Ok(()),
    }
}

fn require_item_fact(
    fact: Option<bool>,
    site: OmpClauseItemSite,
    span: Span,
    description: &str,
) -> Result<(), Diagnostic> {
    match fact {
        None => Err(Diagnostic::new(
            DiagnosticCode::MissingSemanticFact,
            span,
            format!("{site:?} requires a {description} fact"),
        )),
        Some(false) => Err(Diagnostic::new(
            DiagnosticCode::InvalidClause,
            span,
            format!("{site:?}: {description} restriction is not satisfied"),
        )),
        Some(true) => Ok(()),
    }
}

fn require_depend_object(
    site: OmpClauseItemSite,
    required: DependObjectState,
    span: Span,
    facts: &SemanticFacts,
) -> Result<(), Diagnostic> {
    match facts.depend_object(site) {
        None => Err(Diagnostic::new(
            DiagnosticCode::MissingSemanticFact,
            span,
            format!("{site:?} requires a depend-object state fact"),
        )),
        Some(DependObjectState::WrongType) => Err(Diagnostic::new(
            DiagnosticCode::InvalidExpressionType,
            span,
            format!("{site:?} is not an OpenMP depend object"),
        )),
        Some(actual) if actual != required => Err(Diagnostic::new(
            DiagnosticCode::InvalidClause,
            span,
            format!("{site:?} is a {actual:?} depend object but this action requires {required:?}"),
        )),
        Some(_) => Ok(()),
    }
}

fn require_interop_object(
    site: OmpClauseItemSite,
    must_be_initialized: bool,
    span: Span,
    facts: &SemanticFacts,
) -> Result<(), Diagnostic> {
    match facts.interop_object(site) {
        None => Err(Diagnostic::new(
            DiagnosticCode::MissingSemanticFact,
            span,
            format!("{site:?} requires an interoperability-object state fact"),
        )),
        Some(InteropObjectState::WrongType) => Err(Diagnostic::new(
            DiagnosticCode::InvalidExpressionType,
            span,
            format!("{site:?} is not an OpenMP interoperability object"),
        )),
        Some(InteropObjectState::Uninitialized) if must_be_initialized => Err(Diagnostic::new(
            DiagnosticCode::InvalidClause,
            span,
            format!("{site:?} is an uninitialized interoperability object"),
        )),
        Some(InteropObjectState::Uninitialized | InteropObjectState::Initialized) => Ok(()),
    }
}

fn require_detach_event(
    site: OmpClauseItemSite,
    span: Span,
    facts: &SemanticFacts,
) -> Result<(), Diagnostic> {
    match facts.detach_event(site) {
        None => Err(Diagnostic::new(
            DiagnosticCode::MissingSemanticFact,
            span,
            format!("{site:?} requires a detach-event classification fact"),
        )),
        Some(DetachEventStatus::WrongType) => Err(Diagnostic::new(
            DiagnosticCode::InvalidExpressionType,
            span,
            format!("{site:?} is not an OpenMP event_handle variable"),
        )),
        Some(DetachEventStatus::Invalid) => Err(Diagnostic::new(
            DiagnosticCode::InvalidClause,
            span,
            format!("{site:?} violates a detach event-handle restriction"),
        )),
        Some(DetachEventStatus::Valid) => Ok(()),
    }
}

fn require_modifiable_item(
    site: OmpClauseItemSite,
    span: Span,
    facts: &SemanticFacts,
    subject: &str,
) -> Result<(), Diagnostic> {
    match facts.modifiable_item(site) {
        None => Err(Diagnostic::new(
            DiagnosticCode::MissingSemanticFact,
            span,
            format!("{site:?} requires a modifiable-variable fact for {subject}"),
        )),
        Some(false) => Err(Diagnostic::new(
            DiagnosticCode::InvalidClause,
            span,
            format!("{subject} must not be constant"),
        )),
        Some(true) => Ok(()),
    }
}

fn require_procedure_parameter_items(
    clause_site: OmpClauseSite,
    item_count: usize,
    span: Span,
    facts: &SemanticFacts,
) -> Result<(), Diagnostic> {
    for index in 0..item_count {
        let item_site = OmpClauseItemSite::new(clause_site, index);
        require_procedure_parameter_item(item_site, span, facts)?;
    }
    Ok(())
}

fn require_procedure_parameter_item(
    item_site: OmpClauseItemSite,
    span: Span,
    facts: &SemanticFacts,
) -> Result<(), Diagnostic> {
    match facts.procedure_parameter(item_site) {
        None => Err(Diagnostic::new(
            DiagnosticCode::MissingSemanticFact,
            span,
            format!("{item_site:?} requires an associated-procedure parameter fact"),
        )),
        Some(false) => Err(Diagnostic::new(
            DiagnosticCode::InvalidClause,
            span,
            format!("{item_site:?} does not name an associated-procedure parameter"),
        )),
        Some(true) => Ok(()),
    }
}

fn require_constant_logical(
    expression: &Expression,
    site: OmpExpressionSite,
    span: Span,
    facts: &SemanticFacts,
) -> Result<bool, Diagnostic> {
    if matches!(facts.constant_expression(site), Some(false)) {
        return Err(Diagnostic::new(
            DiagnosticCode::ConstantExpressionRequired,
            span,
            format!("{site:?} requires a constant logical expression"),
        ));
    }
    let evaluation = obvious_logical_evaluation(expression)
        .or_else(|| facts.logical_evaluation(site))
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::MissingSemanticFact,
                span,
                format!("{site:?} requires an evaluated constant-logical fact"),
            )
        })?;
    match evaluation {
        LogicalEvaluation::NotLogical => Err(Diagnostic::new(
            DiagnosticCode::InvalidExpressionType,
            span,
            format!("{site:?} requires a logical expression"),
        )),
        LogicalEvaluation::False => Ok(false),
        LogicalEvaluation::True => Ok(true),
    }
}

fn require_logical_expression(
    expression: &Expression,
    site: OmpExpressionSite,
    span: Span,
    facts: &SemanticFacts,
) -> Result<(), Diagnostic> {
    match obvious_logical_evaluation(expression) {
        Some(LogicalEvaluation::NotLogical) => Err(Diagnostic::new(
            DiagnosticCode::InvalidExpressionType,
            span,
            format!("{site:?} requires an OpenMP logical expression"),
        )),
        Some(LogicalEvaluation::False | LogicalEvaluation::True) => Ok(()),
        None => match facts.logical_expression(site) {
            None => Err(Diagnostic::new(
                DiagnosticCode::MissingSemanticFact,
                span,
                format!("{site:?} requires a logical-expression type fact"),
            )),
            Some(false) => Err(Diagnostic::new(
                DiagnosticCode::InvalidExpressionType,
                span,
                format!("{site:?} is not an OpenMP logical expression"),
            )),
            Some(true) => Ok(()),
        },
    }
}

fn require_constant_expression(
    expression: &Expression,
    site: OmpExpressionSite,
    span: Span,
    facts: &SemanticFacts,
) -> Result<(), Diagnostic> {
    let syntactic_literal = matches!(expression.ast().kind, crate::host::ExprKind::Literal(_));
    if syntactic_literal || facts.constant_expression(site) == Some(true) {
        return Ok(());
    }
    match facts.constant_expression(site) {
        None => Err(Diagnostic::new(
            DiagnosticCode::MissingSemanticFact,
            span,
            format!("{site:?} requires a constant-expression fact"),
        )),
        Some(false) => Err(Diagnostic::new(
            DiagnosticCode::ConstantExpressionRequired,
            span,
            format!("{site:?} requires a constant expression"),
        )),
        Some(true) => unreachable!("handled above"),
    }
}

fn require_positive_constant_integer(
    expression: &Expression,
    site: OmpExpressionSite,
    span: Span,
    facts: &SemanticFacts,
) -> Result<u128, Diagnostic> {
    let value = require_constant_integer(expression, site, span, facts)?;
    match value {
        IntegerEvaluation::NonNegative(value) if value > 0 => Ok(value),
        IntegerEvaluation::NotInteger => Err(Diagnostic::new(
            DiagnosticCode::InvalidExpressionType,
            span,
            format!("{site:?} requires an integer expression"),
        )),
        IntegerEvaluation::Negative | IntegerEvaluation::NonNegative(0) => Err(Diagnostic::new(
            DiagnosticCode::InvalidClause,
            span,
            format!("{site:?} requires a positive integer value"),
        )),
        IntegerEvaluation::NonNegative(_) => unreachable!("positive value matched above"),
    }
}

fn require_nonnegative_constant_integer(
    expression: &Expression,
    site: OmpExpressionSite,
    span: Span,
    facts: &SemanticFacts,
) -> Result<u128, Diagnostic> {
    match require_constant_integer(expression, site, span, facts)? {
        IntegerEvaluation::NonNegative(value) => Ok(value),
        IntegerEvaluation::NotInteger => Err(Diagnostic::new(
            DiagnosticCode::InvalidExpressionType,
            span,
            format!("{site:?} requires an integer expression"),
        )),
        IntegerEvaluation::Negative => Err(Diagnostic::new(
            DiagnosticCode::InvalidClause,
            span,
            format!("{site:?} requires a non-negative integer value"),
        )),
    }
}

fn require_constant_integer(
    expression: &Expression,
    site: OmpExpressionSite,
    span: Span,
    facts: &SemanticFacts,
) -> Result<IntegerEvaluation, Diagnostic> {
    if matches!(facts.constant_expression(site), Some(false)) {
        return Err(Diagnostic::new(
            DiagnosticCode::ConstantExpressionRequired,
            span,
            format!("{site:?} requires a constant expression"),
        ));
    }
    if let Some(value) = obvious_integer_evaluation(expression) {
        return Ok(value);
    }
    facts.integer_evaluation(site).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::MissingSemanticFact,
            span,
            format!("{site:?} requires an evaluated constant-integer fact"),
        )
    })
}

fn require_positive_integer_expression(
    expression: &Expression,
    site: OmpExpressionSite,
    span: Span,
    facts: &SemanticFacts,
) -> Result<(), Diagnostic> {
    if let Some(evaluation) =
        obvious_integer_evaluation(expression).or_else(|| facts.integer_evaluation(site))
    {
        return match evaluation {
            IntegerEvaluation::NonNegative(value) if value > 0 => Ok(()),
            IntegerEvaluation::NotInteger => Err(Diagnostic::new(
                DiagnosticCode::InvalidExpressionType,
                span,
                format!("{site:?} requires an integer expression"),
            )),
            IntegerEvaluation::Negative | IntegerEvaluation::NonNegative(0) => {
                Err(Diagnostic::new(
                    DiagnosticCode::InvalidClause,
                    span,
                    format!("{site:?} requires a positive integer value"),
                ))
            }
            IntegerEvaluation::NonNegative(_) => unreachable!("positive value matched above"),
        };
    }
    match facts.positive_integer_expression(site) {
        None => Err(Diagnostic::new(
            DiagnosticCode::MissingSemanticFact,
            span,
            format!("{site:?} requires a positive-integer expression fact"),
        )),
        Some(false) => Err(Diagnostic::new(
            DiagnosticCode::InvalidExpressionType,
            span,
            format!("{site:?} is not a positive integer expression"),
        )),
        Some(true) => Ok(()),
    }
}

fn require_integer_expression(
    expression: &Expression,
    site: OmpExpressionSite,
    span: Span,
    facts: &SemanticFacts,
) -> Result<(), Diagnostic> {
    if let Some(evaluation) =
        obvious_integer_evaluation(expression).or_else(|| facts.integer_evaluation(site))
    {
        return match evaluation {
            IntegerEvaluation::NotInteger => Err(Diagnostic::new(
                DiagnosticCode::InvalidExpressionType,
                span,
                format!("{site:?} requires an integer expression"),
            )),
            IntegerEvaluation::Negative | IntegerEvaluation::NonNegative(_) => Ok(()),
        };
    }
    match facts.integer_expression(site) {
        None => Err(Diagnostic::new(
            DiagnosticCode::MissingSemanticFact,
            span,
            format!("{site:?} requires an integer-expression fact"),
        )),
        Some(false) => Err(Diagnostic::new(
            DiagnosticCode::InvalidExpressionType,
            span,
            format!("{site:?} is not an integer expression"),
        )),
        Some(true) => Ok(()),
    }
}

fn obvious_integer_evaluation(expression: &Expression) -> Option<IntegerEvaluation> {
    use crate::host::{ExprKind, Literal, UnaryOp};

    fn classify(expression: &crate::host::Expr) -> Option<IntegerEvaluation> {
        match &expression.kind {
            ExprKind::Parenthesized(inner) => classify(inner),
            ExprKind::Literal(Literal::Integer(value)) => {
                Some(IntegerEvaluation::NonNegative(value.value))
            }
            ExprKind::Literal(_) => Some(IntegerEvaluation::NotInteger),
            ExprKind::Unary {
                op: UnaryOp::Plus,
                operand,
            } => classify(operand),
            ExprKind::Unary {
                op: UnaryOp::Minus,
                operand,
            } => match classify(operand) {
                Some(IntegerEvaluation::NonNegative(0)) => Some(IntegerEvaluation::NonNegative(0)),
                Some(IntegerEvaluation::NonNegative(_)) => Some(IntegerEvaluation::Negative),
                Some(IntegerEvaluation::NotInteger) => Some(IntegerEvaluation::NotInteger),
                Some(IntegerEvaluation::Negative) | None => None,
            },
            _ => None,
        }
    }

    classify(expression.ast())
}

fn obvious_logical_evaluation(expression: &Expression) -> Option<LogicalEvaluation> {
    use crate::host::{ExprKind, HostLanguage, Literal};

    fn classify(
        expression: &crate::host::Expr,
        language: HostLanguage,
    ) -> Option<LogicalEvaluation> {
        match &expression.kind {
            ExprKind::Parenthesized(inner) => classify(inner, language),
            ExprKind::Literal(Literal::Boolean(value)) => Some(if *value {
                LogicalEvaluation::True
            } else {
                LogicalEvaluation::False
            }),
            ExprKind::Literal(Literal::Integer(value))
                if matches!(language, HostLanguage::C | HostLanguage::Cpp) =>
            {
                Some(if value.value == 0 {
                    LogicalEvaluation::False
                } else {
                    LogicalEvaluation::True
                })
            }
            ExprKind::Literal(Literal::Real(value))
                if matches!(language, HostLanguage::C | HostLanguage::Cpp) =>
            {
                Some(if value.coefficient == 0 {
                    LogicalEvaluation::False
                } else {
                    LogicalEvaluation::True
                })
            }
            ExprKind::Literal(Literal::Character(value))
                if matches!(language, HostLanguage::C | HostLanguage::Cpp) =>
            {
                Some(if value.value == '\0' {
                    LogicalEvaluation::False
                } else {
                    LogicalEvaluation::True
                })
            }
            ExprKind::Literal(Literal::NullPointer)
                if matches!(language, HostLanguage::C | HostLanguage::Cpp) =>
            {
                Some(LogicalEvaluation::False)
            }
            ExprKind::Literal(Literal::String(_))
                if matches!(language, HostLanguage::C | HostLanguage::Cpp) =>
            {
                Some(LogicalEvaluation::True)
            }
            ExprKind::Literal(_) => Some(LogicalEvaluation::NotLogical),
            _ => None,
        }
    }

    classify(expression.ast(), expression.language())
}

fn expression_is_string_literal(expression: &Expression) -> bool {
    use crate::host::{ExprKind, Literal};

    matches!(
        &expression.ast().kind,
        ExprKind::Literal(Literal::String(_))
    )
}

fn require_openacc_semantic_facts(
    directive: &AccDirective,
    span: Span,
    facts: &SemanticFacts,
) -> Result<(), Diagnostic> {
    let mut occurrences = HashMap::new();
    for clause in directive.clauses() {
        let occurrence = occurrences.entry(clause.kind()).or_insert(0usize);
        let clause_site = AccClauseSite::new(clause.kind(), *occurrence);
        *occurrence += 1;
        if let AccClausePayload::Tile(sizes) = clause.payload() {
            for (index, size) in sizes.iter().enumerate() {
                let Some(expression) = size.expression() else {
                    continue;
                };
                let site = AccExpressionSite::new(clause_site, index);
                require_acc_positive_constant_integer(expression, site, span, facts)?;
            }
        }
    }
    Ok(())
}

fn require_acc_positive_constant_integer(
    expression: &Expression,
    site: AccExpressionSite,
    span: Span,
    facts: &SemanticFacts,
) -> Result<u128, Diagnostic> {
    let evaluation = obvious_integer_evaluation(expression)
        .or_else(|| facts.acc_integer_evaluation(site))
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::MissingSemanticFact,
                span,
                format!("{site:?} requires an evaluated constant-integer fact"),
            )
        })?;
    match evaluation {
        IntegerEvaluation::NonNegative(value) if value > 0 => Ok(value),
        IntegerEvaluation::NotInteger => Err(Diagnostic::new(
            DiagnosticCode::InvalidExpressionType,
            span,
            format!("{site:?} requires an integer expression"),
        )),
        IntegerEvaluation::Negative | IntegerEvaluation::NonNegative(0) => Err(Diagnostic::new(
            DiagnosticCode::InvalidClause,
            span,
            format!("{site:?} requires a positive constant integer value"),
        )),
        IntegerEvaluation::NonNegative(_) => unreachable!("positive value matched above"),
    }
}

fn omp_standalone_multiword_directive(kind: OmpDirectiveKind) -> bool {
    use OmpDirectiveKind as D;
    matches!(
        kind,
        D::BeginAssumes
            | D::BeginDeclareTarget
            | D::BeginDeclareVariant
            | D::BeginMetadirective
            | D::CancellationPoint
            | D::DeclareInduction
            | D::DeclareMapper
            | D::DeclareReduction
            | D::DeclareSimd
            | D::DeclareTarget
            | D::DeclareVariant
            | D::EndDeclareTarget
            | D::EndDeclareVariant
            | D::EndMetadirective
            | D::EndTargetData
            | D::TargetData
            | D::TargetEnterData
            | D::TargetExitData
            | D::TargetUpdate
            | D::TaskIteration
    )
}

pub(crate) fn omp_modifier_names_directive_or_constituent(
    directive: OmpDirectiveKind,
    modifier: OmpDirectiveKind,
) -> bool {
    if directive == modifier {
        return true;
    }
    if omp_standalone_multiword_directive(directive) {
        return false;
    }

    let directive_name = directive
        .as_str()
        .strip_prefix("end ")
        .unwrap_or(directive.as_str());
    let directive_words = directive_name.split_whitespace().collect::<Vec<_>>();
    let modifier_words = modifier.as_str().split_whitespace().collect::<Vec<_>>();
    !modifier_words.is_empty()
        && modifier_words.len() <= directive_words.len()
        && directive_words
            .windows(modifier_words.len())
            .any(|window| window == modifier_words)
}

pub(crate) fn omp_clause_applies_to_named_constituent(
    enclosing: OmpDirectiveKind,
    modifier: OmpDirectiveKind,
    clause: OmpClauseKind,
) -> bool {
    if !omp_modifier_names_directive_or_constituent(enclosing, modifier) {
        return false;
    }

    // On target compound constructs, private data belongs to the inner
    // constituent rather than the target-generating constituent.
    if clause == OmpClauseKind::Private
        && modifier == OmpDirectiveKind::Target
        && enclosing != OmpDirectiveKind::Target
        && enclosing.as_str().starts_with("target ")
    {
        return false;
    }

    openmp_clause_allowed(modifier, clause) == Some(true)
}

fn openmp_clause_allowed(directive: OmpDirectiveKind, clause: OmpClauseKind) -> Option<bool> {
    use OmpClauseKind as C;
    Some(match clause {
        C::NumThreads | C::ProcBind | C::CopyIn => is_parallel(directive),
        C::If => allows_if(directive),
        C::Hint => is_atomic(directive) || directive == OmpDirectiveKind::Critical,
        C::Schedule => is_for_do_loop(directive),
        C::DistSchedule => is_distribute(directive),
        C::Collapse => is_loop(directive),
        C::Ordered => is_for_do_loop(directive),
        C::Order => allows_order(directive),
        // On a combined or composite directive, a clause that belongs to the
        // `loop` constituent remains valid.  Keep this distinct from
        // `taskloop`, whose name happens to contain the same word but which
        // has different clause semantics.
        C::Bind => is_generic_loop(directive),
        C::Linear => is_simd(directive) || is_for_do_loop(directive),
        C::Aligned | C::Simdlen => is_simd(directive),
        C::Safelen | C::Nontemporal => {
            is_simd(directive) && directive != OmpDirectiveKind::DeclareSimd
        }
        C::Uniform | C::Inbranch | C::Notinbranch => directive == OmpDirectiveKind::DeclareSimd,
        C::NumTeams => is_teams(directive),
        C::ThreadLimit => is_teams(directive) || is_target_compute(directive),
        C::Map => {
            is_target_compute(directive)
                || matches!(
                    directive,
                    OmpDirectiveKind::TargetData
                        | OmpDirectiveKind::TargetEnterData
                        | OmpDirectiveKind::TargetExitData
                        | OmpDirectiveKind::DeclareMapper
                )
        }
        C::Defaultmap => is_target_compute(directive),
        C::Device => {
            is_target(directive)
                || matches!(
                    directive,
                    OmpDirectiveKind::Dispatch | OmpDirectiveKind::Interop
                )
        }
        C::IsDevicePtr | C::HasDeviceAddr => {
            is_target_compute(directive) || directive == OmpDirectiveKind::Dispatch
        }
        C::UseDevicePtr | C::UseDeviceAddr => directive == OmpDirectiveKind::TargetData,
        C::Nowait => allows_nowait(directive),
        C::Depend => allows_depend(directive),
        C::Reduction => allows_reduction(directive),
        C::InReduction => {
            is_task(directive)
                || is_target_compute(directive)
                || directive == OmpDirectiveKind::TargetData
        }
        C::TaskReduction => directive == OmpDirectiveKind::Taskgroup,
        C::Default => {
            is_parallel(directive)
                || is_task(directive)
                || is_teams(directive)
                || is_target_compute(directive)
                || directive == OmpDirectiveKind::TargetData
        }
        C::Private => allows_private(directive),
        C::Firstprivate => allows_firstprivate(directive),
        C::Lastprivate => allows_lastprivate(directive),
        C::Shared => allows_shared(directive),
        C::Allocate => allows_allocate(directive),
        C::Copyprivate => matches!(
            directive,
            OmpDirectiveKind::Single | OmpDirectiveKind::EndSingle
        ),
        C::Grainsize | C::NumTasks => is_taskloop(directive),
        C::Nogroup => {
            is_taskloop(directive)
                || matches!(
                    directive,
                    OmpDirectiveKind::TargetData | OmpDirectiveKind::Taskgraph
                )
        }
        C::Final | C::Untied => is_task(directive),
        C::Mergeable => is_task(directive) || directive == OmpDirectiveKind::TargetData,
        C::Affinity => matches!(
            directive,
            OmpDirectiveKind::TargetData | OmpDirectiveKind::Task | OmpDirectiveKind::TaskIteration
        ),
        C::Detach => matches!(
            directive,
            OmpDirectiveKind::TargetData | OmpDirectiveKind::Task
        ),
        C::Priority => {
            is_task(directive) || is_target(directive) || directive == OmpDirectiveKind::Taskgraph
        }
        C::Filter => matches!(
            directive,
            OmpDirectiveKind::Masked
                | OmpDirectiveKind::ParallelMasked
                | OmpDirectiveKind::MaskedTaskloop
                | OmpDirectiveKind::MaskedTaskloopSimd
                | OmpDirectiveKind::ParallelMaskedTaskloop
                | OmpDirectiveKind::ParallelMaskedTaskloopSimd
        ),
        C::When | C::Otherwise => matches!(
            directive,
            OmpDirectiveKind::Metadirective | OmpDirectiveKind::BeginMetadirective
        ),
        C::Match | C::AdjustArgs | C::AppendArgs => directive == OmpDirectiveKind::DeclareVariant,
        C::ReverseOffload
        | C::UnifiedAddress
        | C::UnifiedSharedMemory
        | C::AtomicDefaultMemOrder
        | C::DynamicAllocators
        | C::SelfMaps
        | C::ExtImplementationDefinedRequirement
        | C::DeviceSafesync => directive == OmpDirectiveKind::Requires,
        C::Read | C::Write | C::Update | C::Capture | C::Compare | C::Fail | C::Weak => {
            is_atomic(directive)
        }
        C::SeqCst | C::AcqRel | C::Acquire | C::Release | C::Relaxed => {
            is_atomic(directive) || directive == OmpDirectiveKind::Flush
        }
        C::At => directive == OmpDirectiveKind::Error,
        C::Severity | C::Message => directive == OmpDirectiveKind::Error || is_parallel(directive),
        C::UsesAllocators => is_target_compute(directive),
        C::DepobjUpdate => directive == OmpDirectiveKind::Depobj,
        C::Absent
        | C::Contains
        | C::Holds
        | C::NoOpenmp
        | C::NoOpenmpConstructs
        | C::NoOpenmpRoutines
        | C::NoParallelism => matches!(
            directive,
            OmpDirectiveKind::Assume | OmpDirectiveKind::Assumes | OmpDirectiveKind::BeginAssumes
        ),
        C::Apply => matches!(
            directive,
            OmpDirectiveKind::Tile
                | OmpDirectiveKind::Unroll
                | OmpDirectiveKind::Interchange
                | OmpDirectiveKind::Reverse
                | OmpDirectiveKind::Stripe
                | OmpDirectiveKind::Fuse
                | OmpDirectiveKind::Split
                | OmpDirectiveKind::Nothing
        ),
        C::Full | C::Partial => directive == OmpDirectiveKind::Unroll,
        C::Induction => allows_induction(directive),
        C::Sizes => matches!(directive, OmpDirectiveKind::Tile | OmpDirectiveKind::Stripe),
        C::Collector | C::Inductor => directive == OmpDirectiveKind::DeclareInduction,
        C::Combiner | C::Initializer => directive == OmpDirectiveKind::DeclareReduction,
        C::To => matches!(
            directive,
            OmpDirectiveKind::DeclareTarget
                | OmpDirectiveKind::BeginDeclareTarget
                | OmpDirectiveKind::TargetUpdate
        ),
        C::Link | C::Enter | C::Indirect => matches!(
            directive,
            OmpDirectiveKind::DeclareTarget | OmpDirectiveKind::BeginDeclareTarget
        ),
        C::DeviceType => {
            is_target_compute(directive)
                || matches!(
                    directive,
                    OmpDirectiveKind::DeclareTarget
                        | OmpDirectiveKind::BeginDeclareTarget
                        | OmpDirectiveKind::Groupprivate
                )
        }
        C::Inclusive | C::Exclusive => directive == OmpDirectiveKind::Scan,
        C::Init => matches!(
            directive,
            OmpDirectiveKind::Interop | OmpDirectiveKind::Depobj
        ),
        C::Use => directive == OmpDirectiveKind::Interop,
        C::Interop => directive == OmpDirectiveKind::Dispatch,
        C::Destroy => matches!(
            directive,
            OmpDirectiveKind::Depobj | OmpDirectiveKind::Interop
        ),
        C::Doacross | C::Threads | C::Simd => directive == OmpDirectiveKind::Ordered,
        C::GraphId | C::GraphReset => directive == OmpDirectiveKind::Taskgraph,
        C::Replayable => {
            is_task(directive)
                || is_target_compute(directive)
                || matches!(
                    directive,
                    OmpDirectiveKind::TargetEnterData
                        | OmpDirectiveKind::TargetExitData
                        | OmpDirectiveKind::TargetUpdate
                        | OmpDirectiveKind::Taskwait
                )
        }
        C::Transparent => is_task(directive) || directive == OmpDirectiveKind::TargetData,
        C::Threadset => is_task(directive),
        C::Nocontext | C::Novariants => directive == OmpDirectiveKind::Dispatch,
        C::Looprange => directive == OmpDirectiveKind::Fuse,
        C::Permutation => directive == OmpDirectiveKind::Interchange,
        C::Counts => directive == OmpDirectiveKind::Split,
        C::Local => matches!(
            directive,
            OmpDirectiveKind::DeclareTarget | OmpDirectiveKind::BeginDeclareTarget
        ),
        C::Memscope => is_atomic(directive) || directive == OmpDirectiveKind::Flush,
        C::From => directive == OmpDirectiveKind::TargetUpdate,
        C::Allocator | C::Align => directive == OmpDirectiveKind::Allocate,
        C::Parallel | C::Sections | C::For | C::Do | C::Taskgroup => matches!(
            directive,
            OmpDirectiveKind::Cancel | OmpDirectiveKind::CancellationPoint
        ),
        C::Safesync => is_parallel(directive),
        C::InitComplete => directive == OmpDirectiveKind::Scan,
    })
}

/// OpenMP clauses are repeatable by default (OpenMP 6.0, Table 5.1).
///
/// Keep only clauses with an explicit `unique` property here. Clause-set
/// uniqueness and the post-modified `defaultmap` categories are checked
/// separately because they are not equivalent to global clause-name
/// uniqueness.
fn openmp_clause_is_unique(kind: OmpClauseKind) -> bool {
    matches!(
        kind,
        OmpClauseKind::Absent
            | OmpClauseKind::AcqRel
            | OmpClauseKind::Acquire
            | OmpClauseKind::Align
            | OmpClauseKind::Allocator
            | OmpClauseKind::AppendArgs
            | OmpClauseKind::At
            | OmpClauseKind::AtomicDefaultMemOrder
            | OmpClauseKind::Bind
            | OmpClauseKind::Capture
            | OmpClauseKind::Collapse
            | OmpClauseKind::Collector
            | OmpClauseKind::Combiner
            | OmpClauseKind::Compare
            | OmpClauseKind::Contains
            | OmpClauseKind::Counts
            | OmpClauseKind::DepobjUpdate
            | OmpClauseKind::Detach
            | OmpClauseKind::Device
            | OmpClauseKind::DeviceSafesync
            | OmpClauseKind::DeviceType
            | OmpClauseKind::DistSchedule
            | OmpClauseKind::Do
            | OmpClauseKind::DynamicAllocators
            | OmpClauseKind::ExtImplementationDefinedRequirement
            | OmpClauseKind::Exclusive
            | OmpClauseKind::Fail
            | OmpClauseKind::Final
            | OmpClauseKind::Filter
            | OmpClauseKind::For
            | OmpClauseKind::Full
            | OmpClauseKind::GraphId
            | OmpClauseKind::GraphReset
            | OmpClauseKind::Grainsize
            | OmpClauseKind::Hint
            | OmpClauseKind::Holds
            | OmpClauseKind::Inbranch
            | OmpClauseKind::Inclusive
            | OmpClauseKind::Indirect
            | OmpClauseKind::Inductor
            | OmpClauseKind::InitComplete
            | OmpClauseKind::Initializer
            | OmpClauseKind::Interop
            | OmpClauseKind::Looprange
            | OmpClauseKind::Match
            | OmpClauseKind::Memscope
            | OmpClauseKind::Mergeable
            | OmpClauseKind::Message
            | OmpClauseKind::Nocontext
            | OmpClauseKind::Nogroup
            | OmpClauseKind::NoOpenmp
            | OmpClauseKind::NoOpenmpConstructs
            | OmpClauseKind::NoOpenmpRoutines
            | OmpClauseKind::NoParallelism
            | OmpClauseKind::Notinbranch
            | OmpClauseKind::Novariants
            | OmpClauseKind::Nowait
            | OmpClauseKind::NumTasks
            | OmpClauseKind::NumTeams
            | OmpClauseKind::NumThreads
            | OmpClauseKind::Order
            | OmpClauseKind::Ordered
            | OmpClauseKind::Otherwise
            | OmpClauseKind::Parallel
            | OmpClauseKind::Partial
            | OmpClauseKind::Permutation
            | OmpClauseKind::Priority
            | OmpClauseKind::ProcBind
            | OmpClauseKind::Read
            | OmpClauseKind::Relaxed
            | OmpClauseKind::Release
            | OmpClauseKind::ReverseOffload
            | OmpClauseKind::Safelen
            | OmpClauseKind::Safesync
            | OmpClauseKind::Schedule
            | OmpClauseKind::Sections
            | OmpClauseKind::SelfMaps
            | OmpClauseKind::SeqCst
            | OmpClauseKind::Severity
            | OmpClauseKind::Simd
            | OmpClauseKind::Simdlen
            | OmpClauseKind::Sizes
            | OmpClauseKind::Taskgroup
            | OmpClauseKind::ThreadLimit
            | OmpClauseKind::Threads
            | OmpClauseKind::Threadset
            | OmpClauseKind::Transparent
            | OmpClauseKind::UnifiedAddress
            | OmpClauseKind::UnifiedSharedMemory
            | OmpClauseKind::Untied
            | OmpClauseKind::Update
            | OmpClauseKind::Weak
            | OmpClauseKind::Write
    )
}

fn omp_if_targets_overlap(
    enclosing: OmpDirectiveKind,
    first: Option<OmpDirectiveKind>,
    second: Option<OmpDirectiveKind>,
) -> bool {
    let first = first.unwrap_or(enclosing);
    let second = second.unwrap_or(enclosing);
    first == second
        || omp_modifier_names_directive_or_constituent(first, second)
        || omp_modifier_names_directive_or_constituent(second, first)
}

fn validate_openmp_conflicts(directive: &OmpDirective, span: Span) -> Result<(), Diagnostic> {
    if directive.kind() == OmpDirectiveKind::Flush
        && matches!(
            directive.parameter(),
            Some(OmpDirectiveParameter::FlushList(_))
        )
        && directive.clauses().iter().any(|clause| {
            matches!(
                clause.kind(),
                OmpClauseKind::AcqRel
                    | OmpClauseKind::Acquire
                    | OmpClauseKind::Relaxed
                    | OmpClauseKind::Release
                    | OmpClauseKind::SeqCst
            )
        })
    {
        return Err(Diagnostic::new(
            DiagnosticCode::ConflictingClauses,
            span,
            "a flush list and a memory-order clause may not appear together",
        ));
    }
    if has_omp_clause(directive, OmpClauseKind::Inbranch)
        && has_omp_clause(directive, OmpClauseKind::Notinbranch)
    {
        return Err(Diagnostic::new(
            DiagnosticCode::ConflictingClauses,
            span,
            "inbranch and notinbranch clauses conflict",
        ));
    }
    if has_omp_clause(directive, OmpClauseKind::Inclusive)
        && has_omp_clause(directive, OmpClauseKind::Exclusive)
    {
        return Err(Diagnostic::new(
            DiagnosticCode::ConflictingClauses,
            span,
            "inclusive and exclusive clauses conflict",
        ));
    }
    if has_omp_clause(directive, OmpClauseKind::Copyprivate)
        && has_omp_clause(directive, OmpClauseKind::Nowait)
    {
        return Err(Diagnostic::new(
            DiagnosticCode::ConflictingClauses,
            span,
            "copyprivate and nowait clauses conflict",
        ));
    }

    reject_omp_exclusive_set(
        directive,
        &[
            OmpClauseKind::AcqRel,
            OmpClauseKind::Acquire,
            OmpClauseKind::Relaxed,
            OmpClauseKind::Release,
            OmpClauseKind::SeqCst,
        ],
        span,
        "memory-order clauses are mutually exclusive",
    )?;
    if directive.kind() == OmpDirectiveKind::Atomic {
        reject_omp_exclusive_set(
            directive,
            &[
                OmpClauseKind::Read,
                OmpClauseKind::Update,
                OmpClauseKind::Write,
            ],
            span,
            "atomic read, update, and write clauses are mutually exclusive",
        )?;
        let effective_update = has_omp_clause(directive, OmpClauseKind::Update)
            || !has_any_omp_clause(directive, &[OmpClauseKind::Read, OmpClauseKind::Write]);
        if has_any_omp_clause(directive, &[OmpClauseKind::Capture, OmpClauseKind::Compare])
            && !effective_update
        {
            return Err(Diagnostic::new(
                DiagnosticCode::ConflictingClauses,
                span,
                "atomic capture and compare require effective update semantics",
            ));
        }
        if has_omp_clause(directive, OmpClauseKind::Weak)
            && !has_omp_clause(directive, OmpClauseKind::Compare)
        {
            return Err(Diagnostic::new(
                DiagnosticCode::MissingRequiredClause,
                span,
                "atomic weak requires the compare clause",
            ));
        }
    }
    if matches!(
        directive.kind(),
        OmpDirectiveKind::Task | OmpDirectiveKind::TargetData
    ) {
        reject_omp_exclusive_set(
            directive,
            &[OmpClauseKind::Detach, OmpClauseKind::Mergeable],
            span,
            "detach and mergeable clauses are mutually exclusive",
        )?;
    }
    if is_taskloop(directive.kind()) {
        reject_omp_exclusive_set(
            directive,
            &[OmpClauseKind::Nogroup, OmpClauseKind::Reduction],
            span,
            "nogroup and reduction clauses are mutually exclusive on taskloop",
        )?;
        reject_omp_exclusive_set(
            directive,
            &[OmpClauseKind::Grainsize, OmpClauseKind::NumTasks],
            span,
            "grainsize and num_tasks clauses are mutually exclusive on taskloop",
        )?;
    }
    if directive.kind() == OmpDirectiveKind::Unroll {
        reject_omp_exclusive_set(
            directive,
            &[OmpClauseKind::Full, OmpClauseKind::Partial],
            span,
            "full and partial clauses are mutually exclusive on unroll",
        )?;
        if has_omp_clause(directive, OmpClauseKind::Apply)
            && !has_omp_clause(directive, OmpClauseKind::Partial)
        {
            return Err(Diagnostic::new(
                DiagnosticCode::ConflictingClauses,
                span,
                "an apply clause on unroll requires the partial clause",
            ));
        }
    }

    validate_default_categories(directive, span)?;
    validate_defaultmap_categories(directive, span)?;
    validate_allocate_item_relationships(directive, span)?;
    validate_detach_item_relationships(directive, span)?;
    validate_interop_action_variables(directive, span)?;

    let ordered = has_omp_clause(directive, OmpClauseKind::Ordered);
    let auto_or_runtime = directive.clauses().iter().any(|clause| {
        matches!(
            clause.payload(),
            ClauseData::Schedule {
                kind: ScheduleKind::Auto | ScheduleKind::Runtime,
                ..
            }
        )
    });
    if ordered && auto_or_runtime {
        return Err(Diagnostic::new(
            DiagnosticCode::ConflictingClauses,
            span,
            "ordered conflicts with schedule(auto) and schedule(runtime)",
        ));
    }
    Ok(())
}

fn validate_detach_item_relationships(
    directive: &OmpDirective,
    span: Span,
) -> Result<(), Diagnostic> {
    let Some(event) = directive.clauses().iter().find_map(|clause| {
        let ClauseData::Detach { event } = clause.payload() else {
            return None;
        };
        Some(event)
    }) else {
        return Ok(());
    };
    let conflicts = directive.clauses().iter().any(|clause| {
        let items = match clause.payload() {
            ClauseData::Private { items }
            | ClauseData::Firstprivate { items, .. }
            | ClauseData::Lastprivate { items, .. }
            | ClauseData::Shared { items }
            | ClauseData::Linear { items, .. }
            | ClauseData::Induction { items, .. }
            | ClauseData::Reduction { items, .. } => items.as_slice(),
            _ => &[],
        };
        items
            .iter()
            .any(|item| clause_item_names_variable(item, event))
    });
    if conflicts {
        Err(Diagnostic::new(
            DiagnosticCode::ConflictingClauses,
            span,
            "a detach event may not also appear in a data-environment attribute clause",
        ))
    } else {
        Ok(())
    }
}

fn clause_item_names_variable(
    item: &crate::ir::ClauseItem,
    variable: &crate::ir::Variable,
) -> bool {
    match item {
        crate::ir::ClauseItem::Identifier(identifier) => {
            variable.simple_identifier() == Some(identifier)
        }
        crate::ir::ClauseItem::Variable(item_variable) => item_variable == variable,
        crate::ir::ClauseItem::Expression(expression) => expression == variable.expression(),
        crate::ir::ClauseItem::FortranCommonBlock(_) => false,
    }
}

fn validate_interop_action_variables(
    directive: &OmpDirective,
    span: Span,
) -> Result<(), Diagnostic> {
    if directive.kind() != OmpDirectiveKind::Interop {
        return Ok(());
    }
    let mut variables = Vec::new();
    for clause in directive.clauses() {
        let variable = match clause.payload() {
            ClauseData::InitInterop { variable, .. } => Some(variable),
            ClauseData::Use { interop_var } => Some(interop_var),
            ClauseData::Destroy {
                variable: Some(variable),
            } => Some(variable),
            _ => None,
        };
        if let Some(variable) = variable {
            if variables.contains(&variable) {
                return Err(Diagnostic::new(
                    DiagnosticCode::ConflictingClauses,
                    span,
                    "an interoperability object may appear in only one action clause",
                ));
            }
            variables.push(variable);
        }
    }
    Ok(())
}

fn reject_omp_exclusive_set(
    directive: &OmpDirective,
    kinds: &[OmpClauseKind],
    span: Span,
    message: &str,
) -> Result<(), Diagnostic> {
    let count = directive
        .clauses()
        .iter()
        .filter(|clause| kinds.contains(&clause.kind()))
        .count();
    if count > 1 {
        Err(Diagnostic::new(
            DiagnosticCode::ConflictingClauses,
            span,
            message,
        ))
    } else {
        Ok(())
    }
}

fn validate_allocate_item_relationships(
    directive: &OmpDirective,
    span: Span,
) -> Result<(), Diagnostic> {
    if directive.kind() == OmpDirectiveKind::Allocators {
        return Ok(());
    }
    let privatized = directive
        .clauses()
        .iter()
        .flat_map(|clause| match clause.payload() {
            ClauseData::Private { items }
            | ClauseData::Firstprivate { items, .. }
            | ClauseData::Lastprivate { items, .. }
            | ClauseData::Linear { items, .. }
            | ClauseData::Reduction { items, .. } => items.as_slice(),
            _ => &[],
        })
        .collect::<Vec<_>>();
    for clause in directive.clauses() {
        let ClauseData::Allocate { items, .. } = clause.payload() else {
            continue;
        };
        if let Some(item) = items.iter().find(|item| !privatized.contains(item)) {
            return Err(Diagnostic::new(
                DiagnosticCode::InvalidClause,
                span,
                format!(
                    "allocate list item {item} must also appear in a privatizing data-sharing clause on the directive"
                ),
            ));
        }
    }
    Ok(())
}

fn validate_default_categories(directive: &OmpDirective, span: Span) -> Result<(), Diagnostic> {
    let mut seen = Vec::new();
    for clause in directive.clauses() {
        let ClauseData::Default { category, .. } = clause.payload() else {
            continue;
        };
        let category = category.unwrap_or(DefaultmapCategory::All);
        if seen.contains(&category) {
            return Err(Diagnostic::new(
                DiagnosticCode::DuplicateClause,
                span,
                format!("default category {category} may be specified at most once"),
            ));
        }
        if (category == DefaultmapCategory::All || seen.contains(&DefaultmapCategory::All))
            && !seen.is_empty()
        {
            return Err(Diagnostic::new(
                DiagnosticCode::ConflictingClauses,
                span,
                "default(all) conflicts with every other default clause",
            ));
        }
        seen.push(category);
    }
    Ok(())
}

fn validate_defaultmap_categories(directive: &OmpDirective, span: Span) -> Result<(), Diagnostic> {
    let mut seen = Vec::new();
    for clause in directive.clauses() {
        let ClauseData::Defaultmap { category, .. } = clause.payload() else {
            continue;
        };
        let category = category.unwrap_or(DefaultmapCategory::All);
        if seen.contains(&category) {
            return Err(Diagnostic::new(
                DiagnosticCode::DuplicateClause,
                span,
                format!("defaultmap category {category} may be specified at most once"),
            ));
        }
        if (category == DefaultmapCategory::All || seen.contains(&DefaultmapCategory::All))
            && !seen.is_empty()
        {
            return Err(Diagnostic::new(
                DiagnosticCode::ConflictingClauses,
                span,
                "defaultmap(all) conflicts with every other defaultmap clause",
            ));
        }
        seen.push(category);
    }
    Ok(())
}

fn validate_openmp_required_clauses(
    directive: &OmpDirective,
    span: Span,
) -> Result<(), Diagnostic> {
    if directive.kind() == OmpDirectiveKind::DeclareMapper {
        let map_clauses = directive
            .clauses()
            .iter()
            .filter_map(|clause| match clause.payload() {
                ClauseData::Map { locators, .. } => Some(locators.as_slice()),
                _ => None,
            })
            .collect::<Vec<_>>();
        if map_clauses.is_empty() {
            return Err(Diagnostic::new(
                DiagnosticCode::MissingRequiredClause,
                span,
                "declare mapper requires at least one map clause",
            ));
        }

        let Some(OmpDirectiveParameter::DeclareMapper(mapper)) = directive.parameter() else {
            return Err(Diagnostic::new(
                DiagnosticCode::InvalidDirective,
                span,
                "declare mapper is missing its typed mapper signature",
            ));
        };
        if !map_clauses
            .iter()
            .flat_map(|locators| locators.iter())
            .any(|locator| omp_locator_has_designator_root(locator, mapper.variable()))
        {
            return Err(Diagnostic::new(
                DiagnosticCode::InvalidClause,
                span,
                "a declare mapper map clause must map its declared variable or an element of it",
            ));
        }
    }
    if matches!(
        directive.kind(),
        OmpDirectiveKind::TargetEnterData | OmpDirectiveKind::TargetExitData
    ) && !has_omp_clause(directive, OmpClauseKind::Map)
    {
        return Err(Diagnostic::new(
            DiagnosticCode::MissingRequiredClause,
            span,
            format!("{:?} requires at least one map clause", directive.kind()),
        ));
    }
    if directive.kind() == OmpDirectiveKind::TargetData
        && !has_any_omp_clause(
            directive,
            &[
                OmpClauseKind::Map,
                OmpClauseKind::UseDeviceAddr,
                OmpClauseKind::UseDevicePtr,
            ],
        )
    {
        return Err(Diagnostic::new(
            DiagnosticCode::MissingRequiredClause,
            span,
            "target_data requires map, use_device_addr, or use_device_ptr",
        ));
    }
    if directive.kind() == OmpDirectiveKind::TargetUpdate
        && !has_any_omp_clause(directive, &[OmpClauseKind::From, OmpClauseKind::To])
    {
        return Err(Diagnostic::new(
            DiagnosticCode::MissingRequiredClause,
            span,
            "target_update requires at least one from or to clause",
        ));
    }
    if directive.kind() == OmpDirectiveKind::DeclareInduction
        && (!has_omp_clause(directive, OmpClauseKind::Collector)
            || !has_omp_clause(directive, OmpClauseKind::Inductor))
    {
        return Err(Diagnostic::new(
            DiagnosticCode::MissingRequiredClause,
            span,
            "declare induction requires both collector and inductor clauses",
        ));
    }
    if directive.kind() == OmpDirectiveKind::Scan {
        let count = [
            OmpClauseKind::Exclusive,
            OmpClauseKind::Inclusive,
            OmpClauseKind::InitComplete,
        ]
        .into_iter()
        .filter(|kind| has_omp_clause(directive, *kind))
        .count();
        if count != 1 {
            return Err(Diagnostic::new(
                DiagnosticCode::MissingRequiredClause,
                span,
                "scan requires exactly one of exclusive, inclusive, or init_complete",
            ));
        }
    }
    if matches!(
        directive.kind(),
        OmpDirectiveKind::Tile | OmpDirectiveKind::Stripe
    ) && !has_omp_clause(directive, OmpClauseKind::Sizes)
    {
        return Err(Diagnostic::new(
            DiagnosticCode::MissingRequiredClause,
            span,
            "tile and stripe constructs require the sizes clause",
        ));
    }
    if directive.kind() == OmpDirectiveKind::Split
        && !has_omp_clause(directive, OmpClauseKind::Counts)
    {
        return Err(Diagnostic::new(
            DiagnosticCode::MissingRequiredClause,
            span,
            "split requires the counts clause",
        ));
    }
    if matches!(
        directive.kind(),
        OmpDirectiveKind::Cancel | OmpDirectiveKind::CancellationPoint
    ) {
        let parameter_count = usize::from(matches!(
            directive.parameter(),
            Some(OmpDirectiveParameter::Construct(_))
        ));
        let count = parameter_count
            + omp_clause_count(
                directive,
                &[
                    OmpClauseKind::Do,
                    OmpClauseKind::For,
                    OmpClauseKind::Parallel,
                    OmpClauseKind::Sections,
                    OmpClauseKind::Taskgroup,
                ],
            );
        if count == 0 {
            return Err(Diagnostic::new(
                DiagnosticCode::MissingRequiredClause,
                span,
                "cancel and cancellation_point require a cancellation directive name",
            ));
        }
        if count > 1 {
            return Err(Diagnostic::new(
                DiagnosticCode::ConflictingClauses,
                span,
                "cancellation directive-name clauses are mutually exclusive",
            ));
        }
    }
    if directive.kind() == OmpDirectiveKind::Requires
        && !has_any_omp_clause(
            directive,
            &[
                OmpClauseKind::AtomicDefaultMemOrder,
                OmpClauseKind::DeviceSafesync,
                OmpClauseKind::DynamicAllocators,
                OmpClauseKind::ExtImplementationDefinedRequirement,
                OmpClauseKind::ReverseOffload,
                OmpClauseKind::SelfMaps,
                OmpClauseKind::UnifiedAddress,
                OmpClauseKind::UnifiedSharedMemory,
            ],
        )
    {
        return Err(Diagnostic::new(
            DiagnosticCode::MissingRequiredClause,
            span,
            "requires needs at least one requirement clause",
        ));
    }
    if matches!(
        directive.kind(),
        OmpDirectiveKind::Assume | OmpDirectiveKind::Assumes | OmpDirectiveKind::BeginAssumes
    ) && !has_any_omp_clause(
        directive,
        &[
            OmpClauseKind::Absent,
            OmpClauseKind::Contains,
            OmpClauseKind::Holds,
            OmpClauseKind::NoOpenmp,
            OmpClauseKind::NoOpenmpConstructs,
            OmpClauseKind::NoOpenmpRoutines,
            OmpClauseKind::NoParallelism,
        ],
    ) {
        return Err(Diagnostic::new(
            DiagnosticCode::MissingRequiredClause,
            span,
            "an assumption directive requires at least one assumption clause",
        ));
    }
    if directive.kind() == OmpDirectiveKind::DeclareVariant
        && !has_omp_clause(directive, OmpClauseKind::Match)
    {
        return Err(Diagnostic::new(
            DiagnosticCode::MissingRequiredClause,
            span,
            "declare_variant requires the match clause",
        ));
    }
    if directive.kind() == OmpDirectiveKind::Interop
        && !has_any_omp_clause(
            directive,
            &[
                OmpClauseKind::Destroy,
                OmpClauseKind::Init,
                OmpClauseKind::Use,
            ],
        )
    {
        return Err(Diagnostic::new(
            DiagnosticCode::MissingRequiredClause,
            span,
            "interop requires at least one destroy, init, or use action clause",
        ));
    }
    if directive.kind() == OmpDirectiveKind::Depobj {
        validate_depobj_action_form(directive, span)?;
    }
    Ok(())
}

fn validate_depobj_action_form(directive: &OmpDirective, span: Span) -> Result<(), Diagnostic> {
    let action_count = omp_clause_count(
        directive,
        &[
            OmpClauseKind::Depend,
            OmpClauseKind::DepobjUpdate,
            OmpClauseKind::Destroy,
            OmpClauseKind::Init,
        ],
    );
    let Some(parameter) = directive.parameter() else {
        if has_omp_clause(directive, OmpClauseKind::Depend) {
            return Err(Diagnostic::new(
                DiagnosticCode::ClauseNotAllowed,
                span,
                "the parameterless OpenMP 6 depobj form does not accept depend",
            ));
        }
        if action_count == 0 {
            return Err(Diagnostic::new(
                DiagnosticCode::MissingRequiredClause,
                span,
                "the parameterless depobj form requires init, update, or destroy",
            ));
        }
        for clause in directive.clauses() {
            let missing_variable = matches!(
                clause.payload(),
                ClauseData::DepobjUpdate { variable: None, .. }
                    | ClauseData::Destroy { variable: None }
            );
            if missing_variable {
                return Err(Diagnostic::new(
                    DiagnosticCode::MissingRequiredClause,
                    span,
                    "parameterless depobj update and destroy actions require an explicit variable",
                ));
            }
        }
        return Ok(());
    };

    let OmpDirectiveParameter::Depobj(target) = parameter else {
        return Err(Diagnostic::new(
            DiagnosticCode::InvalidDirective,
            span,
            "depobj has a non-depobj directive parameter",
        ));
    };
    if has_omp_clause(directive, OmpClauseKind::Init) {
        return Err(Diagnostic::new(
            DiagnosticCode::ClauseNotAllowed,
            span,
            "the historical depobj(depend-object) form does not accept init",
        ));
    }
    if action_count != 1 {
        return Err(Diagnostic::new(
            if action_count == 0 {
                DiagnosticCode::MissingRequiredClause
            } else {
                DiagnosticCode::ConflictingClauses
            },
            span,
            "the historical depobj(depend-object) form requires exactly one depend, update, or destroy action",
        ));
    }
    for clause in directive.clauses() {
        let explicit_variable = match clause.payload() {
            ClauseData::DepobjUpdate {
                variable: Some(variable),
                ..
            }
            | ClauseData::Destroy {
                variable: Some(variable),
            } => Some(variable),
            _ => None,
        };
        if explicit_variable.is_some_and(|variable| target.expression() != variable.expression()) {
            return Err(Diagnostic::new(
                DiagnosticCode::InvalidClause,
                span,
                "an explicit depobj action variable must match the historical directive argument",
            ));
        }
    }
    Ok(())
}

fn omp_locator_has_designator_root(
    locator: &OmpLocator,
    expected: &crate::host::Identifier,
) -> bool {
    match locator {
        OmpLocator::LValue(lvalue) => {
            crate::ir::Variable::from_expression(lvalue.expression().clone())
                .is_ok_and(|variable| variable.root_identifier() == Some(expected))
        }
        OmpLocator::AllMemory
        | OmpLocator::FortranCommonBlock(_)
        | OmpLocator::PotentialLValue(_) => false,
    }
}

fn has_omp_clause(directive: &OmpDirective, kind: OmpClauseKind) -> bool {
    directive
        .clauses()
        .iter()
        .any(|clause| clause.kind() == kind)
}

fn has_any_omp_clause(directive: &OmpDirective, kinds: &[OmpClauseKind]) -> bool {
    directive
        .clauses()
        .iter()
        .any(|clause| kinds.contains(&clause.kind()))
}

fn omp_clause_count(directive: &OmpDirective, kinds: &[OmpClauseKind]) -> usize {
    directive
        .clauses()
        .iter()
        .filter(|clause| kinds.contains(&clause.kind()))
        .count()
}

fn openacc_clause_allowed(directive: AccDirectiveKind, clause: AccClauseKind) -> Option<bool> {
    use AccClauseKind as C;
    let compute = matches!(
        directive,
        AccDirectiveKind::Parallel
            | AccDirectiveKind::ParallelLoop
            | AccDirectiveKind::Kernels
            | AccDirectiveKind::KernelsLoop
            | AccDirectiveKind::Serial
            | AccDirectiveKind::SerialLoop
    );
    let parallel_or_serial_compute = matches!(
        directive,
        AccDirectiveKind::Parallel
            | AccDirectiveKind::ParallelLoop
            | AccDirectiveKind::Serial
            | AccDirectiveKind::SerialLoop
    );
    let loop_directive = matches!(
        directive,
        AccDirectiveKind::Loop
            | AccDirectiveKind::ParallelLoop
            | AccDirectiveKind::KernelsLoop
            | AccDirectiveKind::SerialLoop
    );
    let structured_data = matches!(
        directive,
        AccDirectiveKind::Data
            | AccDirectiveKind::Parallel
            | AccDirectiveKind::ParallelLoop
            | AccDirectiveKind::Kernels
            | AccDirectiveKind::KernelsLoop
            | AccDirectiveKind::Serial
            | AccDirectiveKind::SerialLoop
    );

    Some(match clause {
        C::Async | C::Wait => {
            compute
                || matches!(
                    directive,
                    AccDirectiveKind::Data
                        | AccDirectiveKind::EnterData
                        | AccDirectiveKind::ExitData
                        | AccDirectiveKind::Update
                        | AccDirectiveKind::Wait
                )
        }
        C::If => {
            compute
                || matches!(
                    directive,
                    AccDirectiveKind::Data
                        | AccDirectiveKind::EnterData
                        | AccDirectiveKind::ExitData
                        | AccDirectiveKind::HostData
                        | AccDirectiveKind::Init
                        | AccDirectiveKind::Set
                        | AccDirectiveKind::Shutdown
                        | AccDirectiveKind::Update
                        | AccDirectiveKind::Wait
                        | AccDirectiveKind::Atomic
                )
        }
        C::Copy => structured_data || directive == AccDirectiveKind::Declare,
        C::CopyIn | C::Create => {
            structured_data
                || matches!(
                    directive,
                    AccDirectiveKind::EnterData | AccDirectiveKind::Declare
                )
        }
        C::CopyOut => {
            structured_data
                || matches!(
                    directive,
                    AccDirectiveKind::ExitData | AccDirectiveKind::Declare
                )
        }
        C::Present | C::DevicePtr => structured_data || directive == AccDirectiveKind::Declare,
        C::NoCreate | C::Default => structured_data,
        C::Attach => structured_data || directive == AccDirectiveKind::EnterData,
        C::Delete | C::Detach | C::Finalize => directive == AccDirectiveKind::ExitData,
        C::UseDevice => directive == AccDirectiveKind::HostData,
        C::Device => directive == AccDirectiveKind::Update,
        C::SelfClause => compute || directive == AccDirectiveKind::Update,
        C::IfPresent => matches!(
            directive,
            AccDirectiveKind::HostData | AccDirectiveKind::Update
        ),
        C::Private | C::Reduction => parallel_or_serial_compute || loop_directive,
        C::Firstprivate => parallel_or_serial_compute,
        C::Collapse | C::Independent | C::Auto | C::Tile => loop_directive,
        C::Gang | C::Worker | C::Vector | C::Seq => {
            loop_directive || directive == AccDirectiveKind::Routine
        }
        C::NumGangs | C::NumWorkers | C::VectorLength => matches!(
            directive,
            AccDirectiveKind::Parallel
                | AccDirectiveKind::ParallelLoop
                | AccDirectiveKind::Kernels
                | AccDirectiveKind::KernelsLoop
        ),
        C::Bind | C::NoHost => directive == AccDirectiveKind::Routine,
        C::DeviceType => {
            compute
                || loop_directive
                || matches!(
                    directive,
                    AccDirectiveKind::Data
                        | AccDirectiveKind::Routine
                        | AccDirectiveKind::Init
                        | AccDirectiveKind::Shutdown
                        | AccDirectiveKind::Set
                        | AccDirectiveKind::Update
                )
        }
        C::DeviceNum => matches!(
            directive,
            AccDirectiveKind::Init | AccDirectiveKind::Shutdown | AccDirectiveKind::Set
        ),
        C::DefaultAsync => directive == AccDirectiveKind::Set,
        C::Capture | C::Read | C::Update | C::Write => directive == AccDirectiveKind::Atomic,
        C::DeviceResident | C::Link => directive == AccDirectiveKind::Declare,
    })
}

fn validate_openacc_clause_sets(directive: &AccDirective, span: Span) -> Result<(), Diagnostic> {
    let if_is_unique = matches!(
        directive.kind(),
        AccDirectiveKind::Parallel
            | AccDirectiveKind::ParallelLoop
            | AccDirectiveKind::Kernels
            | AccDirectiveKind::KernelsLoop
            | AccDirectiveKind::Serial
            | AccDirectiveKind::SerialLoop
            | AccDirectiveKind::Data
            | AccDirectiveKind::EnterData
            | AccDirectiveKind::ExitData
            | AccDirectiveKind::HostData
            | AccDirectiveKind::Atomic
            | AccDirectiveKind::Update
    );
    if if_is_unique && acc_clause_count(directive, &[AccClauseKind::If]) > 1 {
        return Err(Diagnostic::new(
            DiagnosticCode::DuplicateClause,
            span,
            "OpenACC permits at most one if clause on this directive",
        ));
    }

    let default_is_unique = matches!(
        directive.kind(),
        AccDirectiveKind::Parallel
            | AccDirectiveKind::ParallelLoop
            | AccDirectiveKind::Kernels
            | AccDirectiveKind::KernelsLoop
            | AccDirectiveKind::Serial
            | AccDirectiveKind::SerialLoop
            | AccDirectiveKind::Data
    );
    if default_is_unique && acc_clause_count(directive, &[AccClauseKind::Default]) > 1 {
        return Err(Diagnostic::new(
            DiagnosticCode::DuplicateClause,
            span,
            "OpenACC permits at most one default clause on this construct",
        ));
    }

    if directive.kind() == AccDirectiveKind::Atomic
        && acc_clause_count(
            directive,
            &[
                AccClauseKind::Capture,
                AccClauseKind::Read,
                AccClauseKind::Update,
                AccClauseKind::Write,
            ],
        ) > 1
    {
        return Err(Diagnostic::new(
            DiagnosticCode::ConflictingClauses,
            span,
            "OpenACC atomic action clauses are mutually exclusive",
        ));
    }

    Ok(())
}

fn validate_openacc_device_type_segments(
    directive: &AccDirective,
    span: Span,
) -> Result<(), Diagnostic> {
    if !openacc_uses_device_specific_segments(directive.kind()) {
        return Ok(());
    }

    let mut defaults = Vec::new();
    let mut segments: Vec<Vec<&AccClause>> = Vec::new();
    let mut current_segment = None;
    let mut seen_device_types = Vec::<AccDeviceType>::new();
    for clause in directive.clauses() {
        if clause.kind() == AccClauseKind::DeviceType {
            let AccClausePayload::DeviceType(device_types) = clause.payload() else {
                return Err(Diagnostic::new(
                    DiagnosticCode::InvalidClause,
                    clause.span(),
                    "device_type must carry a typed device-type list",
                ));
            };
            for device_type in device_types {
                if seen_device_types.contains(device_type) {
                    return Err(Diagnostic::new(
                        DiagnosticCode::DuplicateClause,
                        clause.span(),
                        format!(
                            "OpenACC device type {device_type:?} is assigned by more than one device_type segment"
                        ),
                    ));
                }
                seen_device_types.push(device_type.clone());
            }
            segments.push(Vec::new());
            current_segment = Some(segments.len() - 1);
            continue;
        }
        if let Some(index) = current_segment {
            if !openacc_clause_may_follow_device_type(directive.kind(), clause.kind()) {
                return Err(Diagnostic::new(
                    DiagnosticCode::ClauseNotAllowed,
                    clause.span(),
                    format!(
                        "OpenACC clause {:?} may not follow device_type on {:?}",
                        clause.kind(),
                        directive.kind()
                    ),
                ));
            }
            segments[index].push(clause);
        } else {
            defaults.push(clause);
        }
    }

    if directive.kind() == AccDirectiveKind::Routine {
        validate_openacc_routine_parallelism(&defaults, span)?;
        for segment in &segments {
            validate_openacc_effective_routine_parallelism(&defaults, segment, span)?;
        }
    }
    Ok(())
}

fn openacc_uses_device_specific_segments(kind: AccDirectiveKind) -> bool {
    matches!(
        kind,
        AccDirectiveKind::Parallel
            | AccDirectiveKind::ParallelLoop
            | AccDirectiveKind::Kernels
            | AccDirectiveKind::KernelsLoop
            | AccDirectiveKind::Serial
            | AccDirectiveKind::SerialLoop
            | AccDirectiveKind::Data
            | AccDirectiveKind::Loop
            | AccDirectiveKind::Routine
            | AccDirectiveKind::Update
    )
}

fn openacc_clause_may_follow_device_type(
    directive: AccDirectiveKind,
    clause: AccClauseKind,
) -> bool {
    use AccClauseKind as C;
    let compute_follower = matches!(
        clause,
        C::Async | C::Wait | C::NumGangs | C::NumWorkers | C::VectorLength
    );
    let loop_follower = matches!(
        clause,
        C::Collapse | C::Gang | C::Worker | C::Vector | C::Seq | C::Independent | C::Auto | C::Tile
    );
    match directive {
        AccDirectiveKind::Parallel | AccDirectiveKind::Kernels | AccDirectiveKind::Serial => {
            compute_follower
        }
        AccDirectiveKind::Loop => loop_follower,
        AccDirectiveKind::ParallelLoop
        | AccDirectiveKind::KernelsLoop
        | AccDirectiveKind::SerialLoop => compute_follower || loop_follower,
        AccDirectiveKind::Data | AccDirectiveKind::Update => {
            matches!(clause, C::Async | C::Wait)
        }
        AccDirectiveKind::Routine => {
            matches!(clause, C::Gang | C::Worker | C::Vector | C::Seq | C::Bind)
        }
        _ => false,
    }
}

fn validate_openacc_routine_parallelism(
    clauses: &[&AccClause],
    span: Span,
) -> Result<(), Diagnostic> {
    let kinds = [
        AccClauseKind::Gang,
        AccClauseKind::Worker,
        AccClauseKind::Vector,
        AccClauseKind::Seq,
    ];
    let count = clauses
        .iter()
        .filter(|clause| kinds.contains(&clause.kind()))
        .count();
    if count > 1 {
        Err(Diagnostic::new(
            DiagnosticCode::ConflictingClauses,
            span,
            "OpenACC routine permits only one of gang, worker, vector, or seq per device type",
        ))
    } else {
        Ok(())
    }
}

fn validate_openacc_effective_routine_parallelism(
    defaults: &[&AccClause],
    specific: &[&AccClause],
    span: Span,
) -> Result<(), Diagnostic> {
    let kinds = [
        AccClauseKind::Gang,
        AccClauseKind::Worker,
        AccClauseKind::Vector,
        AccClauseKind::Seq,
    ];
    let effective = kinds.into_iter().fold(0usize, |count, kind| {
        let specific_count = specific
            .iter()
            .filter(|clause| clause.kind() == kind)
            .count();
        if specific_count == 0 {
            count
                + defaults
                    .iter()
                    .filter(|clause| clause.kind() == kind)
                    .count()
        } else {
            count + specific_count
        }
    });
    if effective > 1 {
        Err(Diagnostic::new(
            DiagnosticCode::ConflictingClauses,
            span,
            "OpenACC routine has conflicting effective parallelism clauses for a device type",
        ))
    } else {
        Ok(())
    }
}

fn acc_clause_count(directive: &AccDirective, kinds: &[AccClauseKind]) -> usize {
    directive
        .clauses()
        .iter()
        .filter(|clause| kinds.contains(&clause.kind()))
        .count()
}

fn validate_openacc_required_clauses(
    directive: &AccDirective,
    span: Span,
) -> Result<(), Diagnostic> {
    let has_any = |kinds: &[AccClauseKind]| {
        directive
            .clauses()
            .iter()
            .any(|clause| kinds.contains(&clause.kind()))
    };
    // Cumulative acceptance deliberately retains bare `data`, `host_data`,
    // and `routine`: their earlier standardized grammars made the relevant
    // clause list optional. The directives below never had a bare form.
    if directive.kind() == AccDirectiveKind::Declare && directive.clauses().is_empty() {
        return Err(Diagnostic::new(
            DiagnosticCode::MissingRequiredClause,
            span,
            "OpenACC declare requires at least one clause",
        ));
    }
    if matches!(
        directive.kind(),
        AccDirectiveKind::EnterData | AccDirectiveKind::ExitData
    ) && directive.clauses().is_empty()
    {
        return Err(Diagnostic::new(
            DiagnosticCode::MissingRequiredClause,
            span,
            "OpenACC enter data and exit data require a nonempty clause list",
        ));
    }
    if directive.kind() == AccDirectiveKind::Update
        && !has_any(&[AccClauseKind::Device, AccClauseKind::SelfClause])
    {
        return Err(Diagnostic::new(
            DiagnosticCode::MissingRequiredClause,
            span,
            "OpenACC update requires at least one self, host, or device action clause",
        ));
    }
    if directive.kind() == AccDirectiveKind::Set
        && !has_any(&[
            AccClauseKind::DefaultAsync,
            AccClauseKind::DeviceNum,
            AccClauseKind::DeviceType,
        ])
    {
        return Err(Diagnostic::new(
            DiagnosticCode::MissingRequiredClause,
            span,
            "OpenACC set requires default_async, device_num, or device_type",
        ));
    }
    Ok(())
}

fn is_parallel(kind: OmpDirectiveKind) -> bool {
    use OmpDirectiveKind as D;
    matches!(
        kind,
        D::Parallel
            | D::ParallelFor
            | D::ParallelForSimd
            | D::ParallelDo
            | D::ParallelDoSimd
            | D::ParallelSections
            | D::ParallelSingle
            | D::ParallelWorkshare
            | D::ParallelLoop
            | D::ParallelLoopSimd
            | D::ParallelMasked
            | D::ParallelMaskedTaskloop
            | D::ParallelMaskedTaskloopSimd
            | D::ParallelMaster
            | D::ParallelMasterTaskloop
            | D::ParallelMasterTaskloopSimd
            | D::TargetParallel
            | D::TargetParallelFor
            | D::TargetParallelForSimd
            | D::TargetParallelDo
            | D::TargetParallelDoSimd
            | D::TargetParallelLoop
            | D::TargetParallelLoopSimd
            | D::DistributeParallelFor
            | D::DistributeParallelForSimd
            | D::DistributeParallelDo
            | D::DistributeParallelDoSimd
            | D::DistributeParallelLoop
            | D::DistributeParallelLoopSimd
            | D::TeamsDistributeParallelFor
            | D::TeamsDistributeParallelForSimd
            | D::TeamsDistributeParallelDo
            | D::TeamsDistributeParallelDoSimd
            | D::TeamsDistributeParallelLoop
            | D::TeamsDistributeParallelLoopSimd
            | D::TargetTeamsDistributeParallelFor
            | D::TargetTeamsDistributeParallelForSimd
            | D::TargetTeamsDistributeParallelDo
            | D::TargetTeamsDistributeParallelDoSimd
            | D::TargetTeamsDistributeParallelLoop
            | D::TargetTeamsDistributeParallelLoopSimd
    )
}

fn is_for_do_loop(kind: OmpDirectiveKind) -> bool {
    use OmpDirectiveKind as D;
    matches!(
        kind,
        D::For
            | D::ForSimd
            | D::Do
            | D::DoSimd
            | D::ParallelFor
            | D::ParallelForSimd
            | D::ParallelDo
            | D::ParallelDoSimd
            | D::DistributeParallelFor
            | D::DistributeParallelForSimd
            | D::DistributeParallelDo
            | D::DistributeParallelDoSimd
            | D::TeamsDistributeParallelFor
            | D::TeamsDistributeParallelForSimd
            | D::TeamsDistributeParallelDo
            | D::TeamsDistributeParallelDoSimd
            | D::TargetParallelFor
            | D::TargetParallelForSimd
            | D::TargetParallelDo
            | D::TargetParallelDoSimd
            | D::TargetTeamsDistributeParallelFor
            | D::TargetTeamsDistributeParallelForSimd
            | D::TargetTeamsDistributeParallelDo
            | D::TargetTeamsDistributeParallelDoSimd
    )
}

fn is_loop(kind: OmpDirectiveKind) -> bool {
    use OmpDirectiveKind as D;
    matches!(
        kind,
        D::For
            | D::ForSimd
            | D::Do
            | D::DoSimd
            | D::Loop
            | D::Simd
            | D::ParallelFor
            | D::ParallelForSimd
            | D::ParallelDo
            | D::ParallelDoSimd
            | D::ParallelLoop
            | D::ParallelLoopSimd
            | D::Taskloop
            | D::TaskloopSimd
            | D::MaskedTaskloop
            | D::MaskedTaskloopSimd
            | D::ParallelMaskedTaskloop
            | D::ParallelMaskedTaskloopSimd
            | D::MasterTaskloop
            | D::MasterTaskloopSimd
            | D::ParallelMasterTaskloop
            | D::ParallelMasterTaskloopSimd
            | D::Distribute
            | D::DistributeSimd
            | D::DistributeParallelFor
            | D::DistributeParallelForSimd
            | D::DistributeParallelDo
            | D::DistributeParallelDoSimd
            | D::DistributeParallelLoop
            | D::DistributeParallelLoopSimd
            | D::TeamsDistribute
            | D::TeamsDistributeSimd
            | D::TeamsDistributeParallelFor
            | D::TeamsDistributeParallelForSimd
            | D::TeamsDistributeParallelDo
            | D::TeamsDistributeParallelDoSimd
            | D::TeamsDistributeParallelLoop
            | D::TeamsDistributeParallelLoopSimd
            | D::TeamsLoop
            | D::TeamsLoopSimd
            | D::TargetLoop
            | D::TargetLoopSimd
            | D::TargetParallelFor
            | D::TargetParallelForSimd
            | D::TargetParallelDo
            | D::TargetParallelDoSimd
            | D::TargetParallelLoop
            | D::TargetParallelLoopSimd
            | D::TargetTeamsDistribute
            | D::TargetTeamsDistributeSimd
            | D::TargetTeamsDistributeParallelFor
            | D::TargetTeamsDistributeParallelForSimd
            | D::TargetTeamsDistributeParallelDo
            | D::TargetTeamsDistributeParallelDoSimd
            | D::TargetTeamsDistributeParallelLoop
            | D::TargetTeamsDistributeParallelLoopSimd
            | D::TargetTeamsLoop
            | D::TargetTeamsLoopSimd
            | D::Workdistribute
    )
}

/// Whether the directive contains the generic OpenMP `loop` construct.
///
/// This deliberately excludes taskloop and the language-specific for/do loop
/// constructs.  It is used for clauses, such as `bind`, whose semantics are
/// inherited specifically from a `loop` constituent of a combined construct.
fn is_generic_loop(kind: OmpDirectiveKind) -> bool {
    use OmpDirectiveKind as D;
    matches!(
        kind,
        D::Loop
            | D::ParallelLoop
            | D::ParallelLoopSimd
            | D::TeamsLoop
            | D::TeamsLoopSimd
            | D::TargetLoop
            | D::TargetLoopSimd
            | D::TargetParallelLoop
            | D::TargetParallelLoopSimd
            | D::TargetTeamsLoop
            | D::TargetTeamsLoopSimd
            | D::DistributeParallelLoop
            | D::DistributeParallelLoopSimd
            | D::TeamsDistributeParallelLoop
            | D::TeamsDistributeParallelLoopSimd
            | D::TargetTeamsDistributeParallelLoop
            | D::TargetTeamsDistributeParallelLoopSimd
    )
}

fn is_simd(kind: OmpDirectiveKind) -> bool {
    use OmpDirectiveKind as D;
    matches!(
        kind,
        D::Simd
            | D::DeclareSimd
            | D::ForSimd
            | D::DoSimd
            | D::ParallelForSimd
            | D::ParallelDoSimd
            | D::ParallelLoopSimd
            | D::TaskloopSimd
            | D::MasterTaskloopSimd
            | D::MaskedTaskloopSimd
            | D::ParallelMaskedTaskloopSimd
            | D::ParallelMasterTaskloopSimd
            | D::DistributeSimd
            | D::DistributeParallelForSimd
            | D::DistributeParallelDoSimd
            | D::DistributeParallelLoopSimd
            | D::TeamsDistributeSimd
            | D::TeamsDistributeParallelForSimd
            | D::TeamsDistributeParallelDoSimd
            | D::TeamsDistributeParallelLoopSimd
            | D::TeamsLoopSimd
            | D::TargetSimd
            | D::TargetLoopSimd
            | D::TargetParallelForSimd
            | D::TargetParallelDoSimd
            | D::TargetParallelLoopSimd
            | D::TargetTeamsDistributeSimd
            | D::TargetTeamsDistributeParallelForSimd
            | D::TargetTeamsDistributeParallelDoSimd
            | D::TargetTeamsDistributeParallelLoopSimd
            | D::TargetTeamsLoopSimd
    )
}

fn is_distribute(kind: OmpDirectiveKind) -> bool {
    use OmpDirectiveKind as D;
    matches!(
        kind,
        D::Distribute
            | D::DistributeSimd
            | D::DistributeParallelFor
            | D::DistributeParallelForSimd
            | D::DistributeParallelDo
            | D::DistributeParallelDoSimd
            | D::DistributeParallelLoop
            | D::DistributeParallelLoopSimd
            | D::TeamsDistribute
            | D::TeamsDistributeSimd
            | D::TeamsDistributeParallelFor
            | D::TeamsDistributeParallelForSimd
            | D::TeamsDistributeParallelDo
            | D::TeamsDistributeParallelDoSimd
            | D::TeamsDistributeParallelLoop
            | D::TeamsDistributeParallelLoopSimd
            | D::TargetTeamsDistribute
            | D::TargetTeamsDistributeSimd
            | D::TargetTeamsDistributeParallelFor
            | D::TargetTeamsDistributeParallelForSimd
            | D::TargetTeamsDistributeParallelDo
            | D::TargetTeamsDistributeParallelDoSimd
            | D::TargetTeamsDistributeParallelLoop
            | D::TargetTeamsDistributeParallelLoopSimd
    )
}

fn is_taskloop(kind: OmpDirectiveKind) -> bool {
    use OmpDirectiveKind as D;
    matches!(
        kind,
        D::Taskloop
            | D::TaskloopSimd
            | D::MasterTaskloop
            | D::MasterTaskloopSimd
            | D::MaskedTaskloop
            | D::MaskedTaskloopSimd
            | D::ParallelMaskedTaskloop
            | D::ParallelMaskedTaskloopSimd
            | D::ParallelMasterTaskloop
            | D::ParallelMasterTaskloopSimd
    )
}

fn is_task(kind: OmpDirectiveKind) -> bool {
    kind == OmpDirectiveKind::Task || is_taskloop(kind)
}

fn is_target(kind: OmpDirectiveKind) -> bool {
    use OmpDirectiveKind as D;
    matches!(
        kind,
        D::Target
            | D::TargetData
            | D::TargetEnterData
            | D::TargetExitData
            | D::TargetUpdate
            | D::TargetLoop
            | D::TargetLoopSimd
            | D::TargetParallel
            | D::TargetParallelFor
            | D::TargetParallelForSimd
            | D::TargetParallelDo
            | D::TargetParallelDoSimd
            | D::TargetParallelLoop
            | D::TargetParallelLoopSimd
            | D::TargetSimd
            | D::TargetTeams
            | D::TargetTeamsDistribute
            | D::TargetTeamsDistributeSimd
            | D::TargetTeamsDistributeParallelFor
            | D::TargetTeamsDistributeParallelForSimd
            | D::TargetTeamsDistributeParallelDo
            | D::TargetTeamsDistributeParallelDoSimd
            | D::TargetTeamsDistributeParallelLoop
            | D::TargetTeamsDistributeParallelLoopSimd
            | D::TargetTeamsLoop
            | D::TargetTeamsLoopSimd
            | D::TargetTeamsWorkdistribute
    )
}

fn is_target_compute(kind: OmpDirectiveKind) -> bool {
    is_target(kind)
        && !matches!(
            kind,
            OmpDirectiveKind::TargetData
                | OmpDirectiveKind::TargetEnterData
                | OmpDirectiveKind::TargetExitData
                | OmpDirectiveKind::TargetUpdate
        )
}

fn is_teams(kind: OmpDirectiveKind) -> bool {
    use OmpDirectiveKind as D;
    matches!(
        kind,
        D::Teams
            | D::TeamsDistribute
            | D::TeamsDistributeSimd
            | D::TeamsDistributeParallelFor
            | D::TeamsDistributeParallelForSimd
            | D::TeamsDistributeParallelDo
            | D::TeamsDistributeParallelDoSimd
            | D::TeamsDistributeParallelLoop
            | D::TeamsDistributeParallelLoopSimd
            | D::TeamsLoop
            | D::TeamsLoopSimd
            | D::TargetTeams
            | D::TargetTeamsDistribute
            | D::TargetTeamsDistributeSimd
            | D::TargetTeamsDistributeParallelFor
            | D::TargetTeamsDistributeParallelForSimd
            | D::TargetTeamsDistributeParallelDo
            | D::TargetTeamsDistributeParallelDoSimd
            | D::TargetTeamsDistributeParallelLoop
            | D::TargetTeamsDistributeParallelLoopSimd
            | D::TargetTeamsLoop
            | D::TargetTeamsLoopSimd
            | D::TargetTeamsWorkdistribute
    )
}

fn is_atomic(kind: OmpDirectiveKind) -> bool {
    matches!(kind, OmpDirectiveKind::Atomic)
}

fn allows_private(kind: OmpDirectiveKind) -> bool {
    is_distribute(kind)
        || is_for_do_loop(kind)
        || is_generic_loop(kind)
        || is_parallel(kind)
        || is_simd(kind)
        || is_task(kind)
        || is_target_compute(kind)
        || is_teams(kind)
        || matches!(
            kind,
            OmpDirectiveKind::Scope
                | OmpDirectiveKind::Sections
                | OmpDirectiveKind::Single
                | OmpDirectiveKind::TargetData
        )
}

fn allows_firstprivate(kind: OmpDirectiveKind) -> bool {
    is_distribute(kind)
        || is_for_do_loop(kind)
        || is_parallel(kind)
        || is_task(kind)
        || is_target_compute(kind)
        || is_teams(kind)
        || matches!(
            kind,
            OmpDirectiveKind::Scope
                | OmpDirectiveKind::Sections
                | OmpDirectiveKind::Single
                | OmpDirectiveKind::TargetData
        )
}

fn allows_lastprivate(kind: OmpDirectiveKind) -> bool {
    is_distribute(kind)
        || is_for_do_loop(kind)
        || is_generic_loop(kind)
        || is_simd(kind)
        || is_taskloop(kind)
        || kind == OmpDirectiveKind::Sections
}

fn allows_shared(kind: OmpDirectiveKind) -> bool {
    is_parallel(kind) || is_task(kind) || is_teams(kind) || kind == OmpDirectiveKind::TargetData
}

fn allows_allocate(kind: OmpDirectiveKind) -> bool {
    is_distribute(kind)
        || is_for_do_loop(kind)
        || is_parallel(kind)
        || is_task(kind)
        || is_target_compute(kind)
        || is_teams(kind)
        || matches!(
            kind,
            OmpDirectiveKind::Allocators
                | OmpDirectiveKind::Scope
                | OmpDirectiveKind::Sections
                | OmpDirectiveKind::Single
                | OmpDirectiveKind::TargetData
                | OmpDirectiveKind::Taskgroup
        )
}

fn allows_reduction(kind: OmpDirectiveKind) -> bool {
    is_for_do_loop(kind)
        || is_generic_loop(kind)
        || is_parallel(kind)
        || is_simd(kind)
        || is_taskloop(kind)
        || is_teams(kind)
        || matches!(kind, OmpDirectiveKind::Scope | OmpDirectiveKind::Sections)
}

fn allows_induction(kind: OmpDirectiveKind) -> bool {
    is_distribute(kind) || is_for_do_loop(kind) || is_simd(kind) || is_taskloop(kind)
}

fn allows_order(kind: OmpDirectiveKind) -> bool {
    is_distribute(kind) || is_for_do_loop(kind) || is_generic_loop(kind) || is_simd(kind)
}

fn allows_depend(kind: OmpDirectiveKind) -> bool {
    is_task(kind)
        || is_target(kind)
        || matches!(
            kind,
            OmpDirectiveKind::Depobj
                | OmpDirectiveKind::Taskwait
                | OmpDirectiveKind::TaskIteration
                | OmpDirectiveKind::Dispatch
                | OmpDirectiveKind::Interop
        )
}

fn allows_nowait(kind: OmpDirectiveKind) -> bool {
    is_for_do_loop(kind)
        || is_distribute(kind)
        || matches!(
            kind,
            OmpDirectiveKind::Sections
                | OmpDirectiveKind::Single
                | OmpDirectiveKind::Workshare
                | OmpDirectiveKind::Scope
                | OmpDirectiveKind::EndDo
                | OmpDirectiveKind::EndDoSimd
                | OmpDirectiveKind::EndSections
                | OmpDirectiveKind::EndSingle
                | OmpDirectiveKind::EndWorkshare
                | OmpDirectiveKind::EndScope
                | OmpDirectiveKind::Dispatch
                | OmpDirectiveKind::Interop
                | OmpDirectiveKind::Taskwait
                | OmpDirectiveKind::Target
                | OmpDirectiveKind::TargetData
                | OmpDirectiveKind::TargetEnterData
                | OmpDirectiveKind::TargetExitData
                | OmpDirectiveKind::TargetUpdate
        )
}

fn allows_if(kind: OmpDirectiveKind) -> bool {
    is_parallel(kind)
        || is_simd(kind)
        || is_task(kind)
        || is_target(kind)
        || is_teams(kind)
        || matches!(
            kind,
            OmpDirectiveKind::Cancel
                | OmpDirectiveKind::Taskwait
                | OmpDirectiveKind::Taskgraph
                | OmpDirectiveKind::TaskIteration
                | OmpDirectiveKind::Dispatch
        )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OpenRegion {
    OpenMp(OmpDirectiveKind),
    OpenAcc(AccDirectiveKind),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RegionFrame {
    region: OpenRegion,
    span: Span,
}

/// Stateful validator for explicit begin/end regions.
#[derive(Debug, Default)]
pub struct ContextValidator {
    stack: Vec<RegionFrame>,
}

impl ContextValidator {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    pub fn begin_openmp(&mut self, kind: OmpDirectiveKind, span: Span) -> Result<(), Diagnostic> {
        if expected_openmp_end(kind).is_none() {
            return Err(Diagnostic::new(
                DiagnosticCode::InvalidDirective,
                span,
                format!("{kind:?} is not an explicit OpenMP region opener"),
            ));
        }
        self.stack.push(RegionFrame {
            region: OpenRegion::OpenMp(kind),
            span,
        });
        Ok(())
    }

    pub fn end_openmp(&mut self, end: OmpDirectiveKind, span: Span) -> Result<(), Diagnostic> {
        let expected_open = openmp_opener_for_end(end).ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::InvalidDirective,
                span,
                format!("{end:?} is not an explicit OpenMP end directive"),
            )
        })?;
        self.close(OpenRegion::OpenMp(expected_open), span, end.as_str())
    }

    pub fn begin_openacc(&mut self, kind: AccDirectiveKind, span: Span) -> Result<(), Diagnostic> {
        if !openacc_is_block(kind) {
            return Err(Diagnostic::new(
                DiagnosticCode::InvalidDirective,
                span,
                format!("{kind:?} is not an explicit OpenACC block opener"),
            ));
        }
        self.stack.push(RegionFrame {
            region: OpenRegion::OpenAcc(kind),
            span,
        });
        Ok(())
    }

    pub fn end_openacc(
        &mut self,
        ended_kind: AccDirectiveKind,
        span: Span,
    ) -> Result<(), Diagnostic> {
        if !openacc_is_block(ended_kind) {
            return Err(Diagnostic::new(
                DiagnosticCode::InvalidDirective,
                span,
                format!("{ended_kind:?} is not an OpenACC block directive"),
            ));
        }
        self.close(OpenRegion::OpenAcc(ended_kind), span, ended_kind.as_str())
    }

    pub fn finish(&self, end_of_input: Span) -> Result<(), Diagnostic> {
        let Some(frame) = self.stack.last() else {
            return Ok(());
        };
        Err(Diagnostic::new(
            DiagnosticCode::MissingContext,
            end_of_input,
            "input ended before the explicit directive region was closed",
        )
        .with_related(frame.span, "unclosed region begins here"))
    }

    fn close(
        &mut self,
        expected: OpenRegion,
        span: Span,
        end_name: &str,
    ) -> Result<(), Diagnostic> {
        let Some(frame) = self.stack.last().copied() else {
            return Err(Diagnostic::new(
                DiagnosticCode::MissingContext,
                span,
                format!("end directive {end_name:?} has no open region"),
            ));
        };
        if frame.region != expected {
            return Err(Diagnostic::new(
                DiagnosticCode::MismatchedEndDirective,
                span,
                format!("end directive {end_name:?} does not close the innermost region"),
            )
            .with_related(frame.span, "innermost open region begins here"));
        }
        self.stack.pop();
        Ok(())
    }
}

fn expected_openmp_end(kind: OmpDirectiveKind) -> Option<OmpDirectiveKind> {
    use OmpDirectiveKind as D;
    Some(match kind {
        D::BeginAssumes => D::EndAssumes,
        D::BeginDeclareTarget => D::EndDeclareTarget,
        D::BeginDeclareVariant => D::EndDeclareVariant,
        D::BeginMetadirective => D::EndMetadirective,
        D::Allocators => D::EndAllocators,
        D::Dispatch => D::EndDispatch,
        D::Parallel => D::EndParallel,
        D::Do => D::EndDo,
        D::DoSimd => D::EndDoSimd,
        D::Sections => D::EndSections,
        D::Single => D::EndSingle,
        D::Workshare => D::EndWorkshare,
        D::Ordered => D::EndOrdered,
        D::Critical => D::EndCritical,
        D::Atomic => D::EndAtomic,
        D::Teams => D::EndTeams,
        D::Task => D::EndTask,
        D::Taskgroup => D::EndTaskgroup,
        D::Taskloop => D::EndTaskloop,
        D::TaskloopSimd => D::EndTaskloopSimd,
        D::Target => D::EndTarget,
        D::TargetData => D::EndTargetData,
        D::Scope => D::EndScope,
        _ => return None,
    })
}

fn openmp_opener_for_end(end: OmpDirectiveKind) -> Option<OmpDirectiveKind> {
    OmpDirectiveKind::ALL
        .iter()
        .copied()
        .find(|candidate| expected_openmp_end(*candidate) == Some(end))
}

fn openacc_is_block(kind: AccDirectiveKind) -> bool {
    matches!(
        kind,
        AccDirectiveKind::Atomic
            | AccDirectiveKind::Data
            | AccDirectiveKind::HostData
            | AccDirectiveKind::Kernels
            | AccDirectiveKind::KernelsLoop
            | AccDirectiveKind::Loop
            | AccDirectiveKind::Parallel
            | AccDirectiveKind::ParallelLoop
            | AccDirectiveKind::Serial
            | AccDirectiveKind::SerialLoop
    )
}
