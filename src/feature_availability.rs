//! Typed clause and syntax-feature availability.
//!
//! Introduction versions are sourced from the official OpenMP 6.0 feature
//! history (Appendix B) and the OpenACC 3.4 change history (Sections
//! 1.9--1.18). Parser acceptance is cumulative: a standardized feature remains
//! accepted by every later exact-version policy, including after deprecation or
//! removal. Nonstandard parser vocabulary is classified explicitly and is
//! never assigned a guessed version.

use crate::ast::{
    AccClause, AccClauseKind, AccClausePayload, AccClauseSourceAlias, AccDataModifier,
    AccDefaultKind, AccDirective, AccDirectiveKind, AccDirectiveParameter, AccEndKind,
    AccGangArgument, OmpClause, OmpClauseKind, OmpClauseSourceAlias, OmpDeclareReductionSyntax,
    OmpDirective, OmpDirectiveKind, OmpDirectiveParameter, OmpDirectiveSourceAlias,
    OmpReductionIdentifier, OmpSelectorDeviceTrait, OmpSelectorEntry,
    OmpSelectorImplementationTraitKind,
};
use crate::availability::openmp_directive_availability;
use crate::ir::{
    AdjustArgsModifier, AllocateSourceSyntax, ClauseData, ClauseItem, DefaultKind,
    DefaultmapBehavior, DefaultmapCategory, DependType, DeviceModifier, LastprivateModifier,
    LinearSourceSyntax, MapModifier, MapType, MapTypeSpelling, MemoryOrder, OmpAppendOperation,
    OmpDependence, OmpLocator, OmpParameterListItem, ProcBind, ReductionModifier, RequireModifier,
    ScheduleKind, UsesAllocatorSourceSyntax,
};
use crate::version::{DirectiveVersion, HostLanguage, OpenAccVersion, OpenMpVersion, VersionSet};

/// Availability classification for one typed syntax feature.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FeatureAvailability<V> {
    /// Standardized beginning in the given specification version.
    Standardized { introduced: V },
    /// Recognized by the low-level parser but absent from the official specs.
    Nonstandard { reason: &'static str },
}

impl<V: DirectiveVersion> FeatureAvailability<V> {
    /// Returns the introduction version for standardized syntax.
    #[must_use]
    pub const fn introduced(self) -> Option<V> {
        match self {
            Self::Standardized { introduced } => Some(introduced),
            Self::Nonstandard { .. } => None,
        }
    }

    /// Returns cumulative parser-compatible versions, or `None` for
    /// explicitly nonstandard syntax.
    #[must_use]
    pub fn compatible_versions(self) -> Option<VersionSet<V>> {
        let Self::Standardized { introduced } = self else {
            return None;
        };
        Some(
            V::ALL
                .iter()
                .copied()
                .filter(|version| *version >= introduced)
                .collect(),
        )
    }

    /// Explains why syntax is classified as nonstandard.
    #[must_use]
    pub const fn nonstandard_reason(self) -> Option<&'static str> {
        match self {
            Self::Standardized { .. } => None,
            Self::Nonstandard { reason } => Some(reason),
        }
    }

    /// Combines two requirements, retaining the later introduction point.
    /// Nonstandard syntax dominates so no later standardized feature can mask
    /// it.
    #[must_use]
    pub fn intersect(self, other: Self) -> Self {
        match (self, other) {
            (Self::Nonstandard { reason }, _) | (_, Self::Nonstandard { reason }) => {
                Self::Nonstandard { reason }
            }
            (Self::Standardized { introduced: left }, Self::Standardized { introduced: right }) => {
                Self::Standardized {
                    introduced: std::cmp::max(left, right),
                }
            }
        }
    }
}

const fn omp(introduced: OpenMpVersion) -> FeatureAvailability<OpenMpVersion> {
    FeatureAvailability::Standardized { introduced }
}

const fn acc(introduced: OpenAccVersion) -> FeatureAvailability<OpenAccVersion> {
    FeatureAvailability::Standardized { introduced }
}

/// Classifies every typed OpenMP clause kind.
///
/// This exhaustive match is intentionally kept on the enum rather than clause
/// spelling strings. Adding a new `OmpClauseKind` therefore requires an
/// explicit availability decision before the crate can compile.
#[must_use]
pub const fn openmp_clause_availability(kind: OmpClauseKind) -> FeatureAvailability<OpenMpVersion> {
    use OmpClauseKind as C;
    use OpenMpVersion as V;

    match kind {
        // OpenMP C/C++ 1.0 and Fortran 1.0 already define these clauses.
        C::CopyIn
        | C::Default
        | C::Firstprivate
        | C::If
        | C::Lastprivate
        | C::Nowait
        | C::NumThreads
        | C::Ordered
        | C::Private
        | C::Reduction
        | C::Schedule
        | C::Shared => omp(V::V1_0),
        C::Copyprivate => omp(V::V2_0),

        // OpenMP 3.0 introduced explicit tasks and collapsed loop nests.
        C::Collapse | C::Untied => omp(V::V3_0),

        // OpenMP 3.1 extended task and atomic syntax.
        C::Capture | C::Final | C::Mergeable | C::Read | C::Update | C::Write => omp(V::V3_1),

        // OpenMP 4.0 introduced cancellation, SIMD, device offload, teams,
        // distribute, task dependences, and user-defined reductions.
        C::Aligned
        | C::Depend
        | C::Device
        | C::DistSchedule
        | C::Do
        | C::For
        | C::From
        | C::Inbranch
        | C::Initializer
        | C::Linear
        | C::Link
        | C::Map
        | C::Notinbranch
        | C::NumTeams
        | C::Parallel
        | C::ProcBind
        | C::Safelen
        | C::Sections
        | C::SeqCst
        | C::Taskgroup
        | C::ThreadLimit
        | C::To
        | C::Uniform => omp(V::V4_0),

        // OpenMP 4.5 added unstructured mapping, taskloop, richer SIMD and
        // doacross support, and synchronization hints.
        C::Defaultmap
        | C::Grainsize
        | C::Hint
        | C::IsDevicePtr
        | C::Nogroup
        | C::NumTasks
        | C::Priority
        | C::Simd
        | C::Simdlen
        | C::Threads
        | C::UseDevicePtr => omp(V::V4_5),

        // OpenMP 5.0 introduced allocators, metadirectives, task reductions,
        // scanning, loop binding/order, weak memory orders, and depobj.
        C::AcqRel
        | C::Acquire
        | C::Affinity
        | C::Allocate
        | C::Allocator
        | C::AtomicDefaultMemOrder
        | C::Bind
        | C::DepobjUpdate
        | C::Destroy
        | C::Detach
        | C::DeviceType
        | C::DynamicAllocators
        | C::Exclusive
        | C::ExtImplementationDefinedRequirement
        | C::InReduction
        | C::Inclusive
        | C::Match
        | C::Nontemporal
        | C::Order
        | C::Relaxed
        | C::Release
        | C::ReverseOffload
        | C::TaskReduction
        | C::UnifiedAddress
        | C::UnifiedSharedMemory
        | C::UseDeviceAddr
        | C::UsesAllocators
        | C::When => omp(V::V5_0),

        // OpenMP 5.1 introduced assumptions, dispatch/error/interop, masked
        // constructs, loop transformations, and compare-and-swap syntax.
        C::Absent
        | C::AdjustArgs
        | C::Align
        | C::AppendArgs
        | C::At
        | C::Compare
        | C::Contains
        | C::Fail
        | C::Filter
        | C::Full
        | C::HasDeviceAddr
        | C::Holds
        | C::Indirect
        | C::Init
        | C::Message
        | C::Nocontext
        | C::NoOpenmp
        | C::NoOpenmpRoutines
        | C::NoParallelism
        | C::Novariants
        | C::Partial
        | C::Severity
        | C::Sizes
        | C::Use
        | C::Weak => omp(V::V5_1),

        // OpenMP 5.2 standardized clearer aliases for historical spellings.
        C::Doacross | C::Enter | C::Otherwise => omp(V::V5_2),

        // OpenMP 6.0 additions represented by the current typed AST.
        C::Apply
        | C::Collector
        | C::Combiner
        | C::Counts
        | C::DeviceSafesync
        | C::GraphId
        | C::GraphReset
        | C::Induction
        | C::Inductor
        | C::InitComplete
        | C::Interop
        | C::Local
        | C::Looprange
        | C::Memscope
        | C::NoOpenmpConstructs
        | C::Permutation
        | C::Replayable
        | C::Safesync
        | C::SelfMaps
        | C::Threadset
        | C::Transparent => omp(V::V6_0),
    }
}

/// Classifies the exact standardized spelling that produced a typed OpenMP
/// directive. Semantic directive kinds retain the historical spaced spelling;
/// private typed provenance prevents 6.0 underscore syntax from being
/// accepted under an older exact-version policy after canonicalization. The
/// host language also selects the correct historical floor for parameter
/// grammars that differed between C/C++ and Fortran.
#[must_use]
pub fn openmp_directive_spelling_availability(
    directive: &OmpDirective,
    host_language: HostLanguage,
) -> FeatureAvailability<OpenMpVersion> {
    use OpenMpVersion as V;

    let Some(canonical) = openmp_directive_availability(directive.kind().as_str()) else {
        return FeatureAvailability::Nonstandard {
            reason: "directive has no standardized OpenMP availability entry",
        };
    };
    let base = omp(canonical.introduced());
    let spelling = match directive.source_alias() {
        Some(OmpDirectiveSourceAlias::OpenMp60Underscore) => base.intersect(omp(V::V6_0)),
        Some(OmpDirectiveSourceAlias::FortranCompact) | None => base,
        Some(OmpDirectiveSourceAlias::FortranRedundantOmp) => FeatureAvailability::Nonstandard {
            reason: "a redundant omp token after the Fortran sentinel is nonstandard",
        },
    };
    match directive.parameter() {
        Some(OmpDirectiveParameter::DeclareReduction(reduction))
            if reduction.source_syntax() == OmpDeclareReductionSyntax::CombinerClause =>
        {
            spelling.intersect(omp(V::V6_0))
        }
        Some(OmpDirectiveParameter::DeclareVariant(target)) if target.base().is_some() => spelling
            .intersect(omp(match host_language {
                HostLanguage::Fortran => V::V5_0,
                HostLanguage::C | HostLanguage::Cpp => V::V5_2,
            })),
        None if directive.kind() == OmpDirectiveKind::Depobj => spelling.intersect(omp(V::V6_0)),
        _ => spelling,
    }
}

/// Classifies the spelling that produced an OpenMP clause.
///
/// Several historical standardized spellings are canonicalized in the typed
/// AST. Their provenance remains typed and private to clause construction so
/// exact-version checks can use the spelling's real introduction version
/// without making formatting trivia part of AST equality.
#[must_use]
pub const fn openmp_clause_spelling_availability(
    clause: &OmpClause,
) -> FeatureAvailability<OpenMpVersion> {
    use OpenMpVersion as V;

    match clause.source_alias() {
        // `depend(source|sink: ...)` was introduced with doacross loops in
        // 4.5; the canonical `doacross(...)` clause name arrived in 5.2.
        Some(OmpClauseSourceAlias::DependSource | OmpClauseSourceAlias::DependSink) => omp(V::V4_5),
        Some(
            OmpClauseSourceAlias::DependSourceCurrent
            | OmpClauseSourceAlias::DependSinkPreviousCurrent,
        ) => omp(V::V5_2),
        // OpenMP 5.0 called the metadirective fallback clause `default`.
        // Version 5.2 added `otherwise` as its preferred synonym.
        Some(OmpClauseSourceAlias::MetadirectiveDefault) => omp(V::V5_0),
        // `to` on declare target dates to 4.0. Version 5.2 renamed the
        // canonical clause spelling to `enter`.
        Some(OmpClauseSourceAlias::DeclareTargetTo) => omp(V::V4_0),
        Some(OmpClauseSourceAlias::ProcBindMaster) => omp(V::V4_0),
        Some(OmpClauseSourceAlias::DoacrossSourceEmpty) => omp(V::V5_2),
        Some(OmpClauseSourceAlias::ReductionOriginalPositional) => omp(V::V6_0),
        None => openmp_clause_availability(clause.kind()),
    }
}

/// Classifies the complete typed syntax of an OpenMP clause, including
/// modifiers, argument keywords, directive-specific additions, aliases, and
/// recursively nested metadirective variants.
#[must_use]
pub fn openmp_clause_syntax_availability(
    directive_kind: OmpDirectiveKind,
    clause: &OmpClause,
    host_language: HostLanguage,
) -> FeatureAvailability<OpenMpVersion> {
    use OpenMpVersion as V;

    let mut result = openmp_clause_spelling_availability(clause);
    let mut nested_requirements = Vec::new();

    {
        let mut require = |version| result = result.intersect(omp(version));
        if clause.directive_name_modifier().is_some() {
            require(if clause.kind() == OmpClauseKind::If {
                V::V4_5
            } else {
                V::V6_0
            });
        }
        match &clause.payload() {
            ClauseData::Default { category, kind } => {
                if category.is_some() {
                    require(V::V6_0);
                }
                match (host_language, kind) {
                    (_, DefaultKind::Shared | DefaultKind::None) => {}
                    (HostLanguage::Fortran, DefaultKind::Private) => {}
                    (HostLanguage::Fortran, DefaultKind::Firstprivate) => require(V::V3_0),
                    (
                        HostLanguage::C | HostLanguage::Cpp,
                        DefaultKind::Private | DefaultKind::Firstprivate,
                    ) => require(V::V5_1),
                }
            }
            ClauseData::Defaultmap { behavior, category } => {
                let original_form = *behavior == DefaultmapBehavior::Tofrom
                    && *category == Some(DefaultmapCategory::Scalar);
                if !original_form {
                    require(V::V5_0);
                }
                if *behavior == DefaultmapBehavior::Present {
                    require(V::V5_1);
                }
                if matches!(
                    behavior,
                    DefaultmapBehavior::Private
                        | DefaultmapBehavior::SelfMap
                        | DefaultmapBehavior::Storage
                ) {
                    require(V::V6_0);
                }
                if *category == Some(DefaultmapCategory::All) {
                    require(V::V5_2);
                }
            }
            ClauseData::Reduction {
                modifiers,
                operator,
                ..
            } => {
                match operator {
                    OmpReductionIdentifier::Name(name)
                        if !matches!(host_language, HostLanguage::Fortran)
                            && name.qualified_name().is_some_and(|name| {
                                !name.global
                                    && name.segments.len() == 1
                                    && matches!(name.segments[0].as_str(), "min" | "max")
                            }) =>
                    {
                        require(V::V3_1)
                    }
                    OmpReductionIdentifier::Name(_)
                    | OmpReductionIdentifier::FortranDefinedOperator(_) => require(V::V4_0),
                    _ => {}
                }
                for modifier in modifiers {
                    match modifier {
                        ReductionModifier::Task
                        | ReductionModifier::Inscan
                        | ReductionModifier::Default => require(V::V5_0),
                        ReductionModifier::Original(_) => require(V::V6_0),
                    }
                }
            }
            ClauseData::Map {
                map_type,
                map_type_spelling,
                modifiers,
                mapper,
                iterators,
                ..
            } => {
                if *map_type == Some(MapType::Storage) {
                    require(match map_type_spelling {
                        MapTypeSpelling::Canonical => V::V6_0,
                        MapTypeSpelling::Alloc
                        | MapTypeSpelling::Release
                        | MapTypeSpelling::Delete => V::V4_5,
                    });
                }
                if mapper.is_some() {
                    require(V::V5_0);
                }
                if !iterators.is_empty() {
                    require(V::V5_1);
                }
                for modifier in modifiers {
                    match modifier {
                        MapModifier::Always => require(V::V4_5),
                        MapModifier::Close => require(V::V5_0),
                        MapModifier::Present | MapModifier::Iterator => require(V::V5_1),
                        MapModifier::Delete => {
                            require(if *map_type_spelling == MapTypeSpelling::Delete {
                                V::V4_5
                            } else {
                                V::V6_0
                            })
                        }
                        MapModifier::SelfMap | MapModifier::Ref(_) => require(V::V6_0),
                    }
                }
            }
            ClauseData::To {
                present,
                mapper,
                iterators,
                ..
            }
            | ClauseData::From {
                present,
                mapper,
                iterators,
                ..
            } => {
                if mapper.is_some() {
                    require(V::V5_0);
                }
                if *present || !iterators.is_empty() {
                    require(V::V5_1);
                }
            }
            ClauseData::Enter { automap: true, .. } => require(V::V6_0),
            ClauseData::Destroy { variable: Some(_) }
                if directive_kind == OmpDirectiveKind::Depobj =>
            {
                require(V::V5_2)
            }
            ClauseData::Depend {
                dependence,
                iterators,
            } => {
                match dependence {
                    OmpDependence::Locators { kind, locators } => {
                        match kind {
                            DependType::In | DependType::Out | DependType::Inout => {}
                            DependType::Mutexinoutset => require(V::V5_0),
                            DependType::Inoutset => require(V::V5_1),
                        }
                        if locators
                            .iter()
                            .any(|locator| matches!(locator, OmpLocator::AllMemory))
                        {
                            require(V::V5_1);
                        }
                    }
                    OmpDependence::Depobjs { .. } => require(V::V5_0),
                }
                if !iterators.is_empty() {
                    require(V::V5_0);
                }
            }
            ClauseData::Schedule {
                kind, modifiers, ..
            } => {
                if *kind == ScheduleKind::Auto {
                    require(V::V3_0);
                }
                if !modifiers.is_empty() {
                    require(V::V4_5);
                }
            }
            ClauseData::DistSchedule { kind, .. } if *kind == ScheduleKind::Auto => {
                require(V::V3_0)
            }
            ClauseData::Ordered { n: Some(_) } => require(V::V4_5),
            ClauseData::Linear { source_syntax, .. } => match source_syntax {
                LinearSourceSyntax::Historical => {}
                LinearSourceSyntax::ModifierPrefix => require(V::V4_5),
                LinearSourceSyntax::CanonicalModifiers => require(V::V5_2),
            },
            ClauseData::Allocate {
                source_syntax: AllocateSourceSyntax::Modifiers,
                ..
            } => require(V::V5_1),
            ClauseData::NumThreads {
                strict, nthreads, ..
            } if *strict || nthreads.len() > 1 => require(V::V6_0),
            ClauseData::NumTeams {
                lower_bound: Some(_),
                ..
            } => require(V::V5_1),
            ClauseData::Lastprivate {
                modifier: Some(LastprivateModifier::Conditional),
                ..
            } => require(V::V5_0),
            ClauseData::Firstprivate {
                modifier: Some(_), ..
            } => require(V::V6_0),
            ClauseData::UsesAllocators { allocators } => {
                if allocators
                    .iter()
                    .any(|entry| entry.source_syntax() == UsesAllocatorSourceSyntax::Modifier)
                {
                    require(V::V5_2);
                }
                if allocators.len() > 1
                    && allocators
                        .iter()
                        .all(|entry| entry.source_syntax() == UsesAllocatorSourceSyntax::Modifier)
                {
                    require(V::V6_0);
                }
            }
            ClauseData::ProcBind(ProcBind::Primary)
                if clause.source_alias() != Some(OmpClauseSourceAlias::ProcBindMaster) =>
            {
                require(V::V5_1)
            }
            ClauseData::Device {
                modifier: Some(DeviceModifier::Ancestor),
                ..
            } => require(V::V5_0),
            ClauseData::Device {
                modifier: Some(DeviceModifier::DeviceNum),
                ..
            } => require(V::V6_0),
            ClauseData::Fail { order } => require(memory_order_introduction(*order)),
            ClauseData::MemoryOrder {
                order,
                use_semantics,
            } => {
                require(memory_order_introduction(*order));
                if use_semantics.is_some() {
                    require(V::V6_0);
                }
            }
            ClauseData::AtomicOperation {
                use_semantics: Some(_),
                ..
            }
            | ClauseData::ExtendedAtomic {
                use_semantics: Some(_),
                ..
            }
            | ClauseData::Nowait {
                do_not_synchronize: Some(_),
            }
            | ClauseData::Nogroup {
                do_not_synchronize: Some(_),
            }
            | ClauseData::Branch {
                condition: Some(_), ..
            }
            | ClauseData::Full {
                fully_unroll: Some(_),
            }
            | ClauseData::Mergeable { can_merge: Some(_) }
            | ClauseData::Untied {
                can_change_threads: Some(_),
            }
            | ClauseData::Simd {
                apply_to_simd: Some(_),
            }
            | ClauseData::Threads {
                apply_to_threads: Some(_),
            }
            | ClauseData::Assumption {
                can_assume: Some(_),
                ..
            } => require(V::V6_0),
            ClauseData::Order {
                modifier: Some(_), ..
            } => require(V::V5_1),
            ClauseData::Grainsize {
                modifier: Some(_), ..
            }
            | ClauseData::NumTasks {
                modifier: Some(_), ..
            } => require(V::V5_1),
            ClauseData::AdjustArgs {
                operation,
                parameters,
            } if *operation == AdjustArgsModifier::NeedDeviceAddr
                || parameters
                    .iter()
                    .any(|parameter| !matches!(parameter, OmpParameterListItem::Named(_))) =>
            {
                require(V::V6_0);
            }
            ClauseData::AppendArgs { operations }
                if operations.iter().any(|operation| {
                    matches!(
                        operation,
                        OmpAppendOperation::Interop(modifiers)
                            if !modifiers.preferences.is_empty()
                    )
                }) =>
            {
                require(V::V6_0);
            }
            ClauseData::Apply {
                applied_directives, ..
            } => {
                for directive in applied_directives {
                    nested_requirements.push(openmp_nested_directive_availability(
                        directive,
                        host_language,
                    ));
                }
            }
            ClauseData::InitDepobj { .. } => require(V::V6_0),
            ClauseData::DepobjUpdate {
                variable: Some(_), ..
            } => require(V::V6_0),
            ClauseData::InitInterop { preferences, .. }
                if preferences.iter().any(|preference| {
                    matches!(
                        preference,
                        crate::ir::OmpPreferenceSpecification::Selectors(_)
                    )
                }) =>
            {
                require(V::V6_0);
            }
            ClauseData::Requirement {
                requirement,
                required,
            } => {
                match requirement {
                    RequireModifier::SelfMaps | RequireModifier::DeviceSafesync => require(V::V6_0),
                    RequireModifier::AtomicDefaultMemOrder(order) => {
                        require(atomic_default_memory_order_introduction(*order))
                    }
                    _ => {}
                }
                if required.is_some() {
                    require(V::V6_0);
                }
            }
            ClauseData::MetadirectiveSelector { selector, .. } => {
                if let Some(nested) = selector.nested_directive() {
                    nested_requirements
                        .push(openmp_nested_directive_availability(nested, host_language));
                }
                for entry in selector.entries() {
                    match entry {
                        OmpSelectorEntry::Device { .. } => {}
                        OmpSelectorEntry::TargetDevice { traits } => {
                            require(V::V5_1);
                            if traits
                                .iter()
                                .any(|trait_| matches!(trait_, OmpSelectorDeviceTrait::Uid(_)))
                            {
                                require(V::V6_0);
                            }
                        }
                        OmpSelectorEntry::Construct { constructs } => {
                            for construct in constructs {
                                nested_requirements.push(openmp_nested_directive_availability(
                                    construct.directive(),
                                    host_language,
                                ));
                            }
                        }
                        OmpSelectorEntry::Implementation { traits } => {
                            for trait_ in traits {
                                match trait_.kind() {
                                    OmpSelectorImplementationTraitKind::AtomicDefaultMemOrder(
                                        order,
                                    ) => {
                                        require(V::V5_1);
                                        require(atomic_default_memory_order_introduction(*order));
                                    }
                                    OmpSelectorImplementationTraitKind::Requirement(
                                        requirement,
                                    ) => {
                                        if matches!(
                                            requirement,
                                            RequireModifier::SelfMaps
                                                | RequireModifier::DeviceSafesync
                                        ) {
                                            require(V::V6_0);
                                        }
                                    }
                                    OmpSelectorImplementationTraitKind::Requires(requirements) => {
                                        require(V::V5_1);
                                        for property in requirements {
                                            match property.requirement() {
                                                RequireModifier::SelfMaps
                                                | RequireModifier::DeviceSafesync => {
                                                    require(V::V6_0)
                                                }
                                                RequireModifier::AtomicDefaultMemOrder(order) => {
                                                    require(
                                                        atomic_default_memory_order_introduction(
                                                            *order,
                                                        ),
                                                    )
                                                }
                                                _ => {}
                                            }
                                            if property.required().is_some() {
                                                require(V::V6_0);
                                            }
                                        }
                                    }
                                    OmpSelectorImplementationTraitKind::NameList(_)
                                    | OmpSelectorImplementationTraitKind::Extension(_) => {}
                                }
                            }
                        }
                        OmpSelectorEntry::User { .. } => {}
                    }
                }
            }
            _ => {}
        }
    }

    for requirement in nested_requirements {
        result = result.intersect(requirement);
    }

    result.intersect(openmp_clause_usage_availability(
        directive_kind,
        clause.kind(),
    ))
}

fn memory_order_introduction(order: MemoryOrder) -> OpenMpVersion {
    match order {
        MemoryOrder::SeqCst => OpenMpVersion::V4_0,
        MemoryOrder::AcqRel
        | MemoryOrder::Release
        | MemoryOrder::Acquire
        | MemoryOrder::Relaxed => OpenMpVersion::V5_0,
    }
}

fn atomic_default_memory_order_introduction(order: MemoryOrder) -> OpenMpVersion {
    match order {
        MemoryOrder::SeqCst | MemoryOrder::AcqRel | MemoryOrder::Relaxed => OpenMpVersion::V5_0,
        MemoryOrder::Release | MemoryOrder::Acquire => OpenMpVersion::V6_0,
    }
}

fn openmp_nested_directive_availability(
    directive: &OmpDirective,
    host_language: HostLanguage,
) -> FeatureAvailability<OpenMpVersion> {
    let Some(availability) = openmp_directive_availability(directive.kind().as_str()) else {
        return FeatureAvailability::Nonstandard {
            reason: "nested directive has no standardized OpenMP availability entry",
        };
    };
    if !availability.languages().supports(host_language) {
        return FeatureAvailability::Nonstandard {
            reason: "nested directive is not standardized for the configured host language",
        };
    }
    let mut result = openmp_directive_spelling_availability(directive, host_language);
    for clause in directive.clauses() {
        result = result.intersect(openmp_clause_syntax_availability(
            directive.kind(),
            clause,
            host_language,
        ));
    }
    result
}

fn openmp_clause_usage_availability(
    directive: OmpDirectiveKind,
    clause: OmpClauseKind,
) -> FeatureAvailability<OpenMpVersion> {
    use OmpClauseKind as C;
    use OmpDirectiveKind as D;
    use OpenMpVersion as V;

    let introduced = match (directive, clause) {
        (D::Target, C::Private | C::Firstprivate | C::Nowait | C::Depend) => V::V4_5,
        (D::For | D::Do | D::ParallelFor | D::ParallelDo, C::Linear) => V::V4_5,
        (directive, C::If) if is_simd_directive(directive) => V::V5_0,
        (directive, C::Hint) if is_atomic_directive(directive) => V::V5_0,
        (D::Taskwait, C::Depend) => V::V5_0,
        (D::Target, C::ThreadLimit) => V::V5_1,
        (D::Taskwait, C::Nowait) | (D::Flush, C::SeqCst) => V::V5_1,
        (directive, C::If) if is_teams_directive(directive) => V::V5_2,
        (D::Scope, C::Allocate | C::Firstprivate) => V::V5_2,
        (D::Target | D::TargetData, C::Default) => V::V6_0,
        (directive, C::Message | C::Severity) if directive != D::Error => V::V6_0,
        (directive, C::Priority) if is_target_directive(directive) => V::V6_0,
        (D::Target, C::DeviceType) => V::V6_0,
        _ => V::V1_0,
    };
    omp(introduced)
}

fn is_atomic_directive(kind: OmpDirectiveKind) -> bool {
    use OmpDirectiveKind as D;
    matches!(kind, D::Atomic)
}

fn is_simd_directive(kind: OmpDirectiveKind) -> bool {
    kind.as_str()
        .split_ascii_whitespace()
        .any(|word| word == "simd")
}

fn is_teams_directive(kind: OmpDirectiveKind) -> bool {
    kind.as_str()
        .split_ascii_whitespace()
        .any(|word| word == "teams")
}

fn is_target_directive(kind: OmpDirectiveKind) -> bool {
    kind.as_str()
        .split_ascii_whitespace()
        .next()
        .is_some_and(|word| word == "target")
}

/// Classifies syntax carried by an OpenACC directive parameter.
///
/// In particular, the closed [`AccEndKind`] set prevents arbitrary directive
/// kinds from entering an `end` node, while this function preserves the
/// historical introduction point of each standardized paired form.
#[must_use]
pub const fn openacc_directive_parameter_availability(
    directive: &AccDirective,
) -> FeatureAvailability<OpenAccVersion> {
    use OpenAccVersion as V;

    match directive.parameter() {
        Some(AccDirectiveParameter::End(AccEndKind::Atomic)) => acc(V::V2_0),
        Some(AccDirectiveParameter::End(AccEndKind::Serial | AccEndKind::SerialLoop)) => {
            acc(V::V2_6)
        }
        Some(AccDirectiveParameter::Cache(cache)) if cache.readonly() => acc(V::V2_7),
        Some(AccDirectiveParameter::Wait(wait)) if wait.devnum().is_some() => acc(V::V3_0),
        Some(AccDirectiveParameter::End(
            AccEndKind::Data
            | AccEndKind::HostData
            | AccEndKind::Kernels
            | AccEndKind::KernelsLoop
            | AccEndKind::Loop
            | AccEndKind::Parallel
            | AccEndKind::ParallelLoop,
        ))
        | Some(
            AccDirectiveParameter::Cache(_)
            | AccDirectiveParameter::Wait(_)
            | AccDirectiveParameter::Routine(_),
        )
        | None => acc(V::V1_0),
    }
}

/// Classifies every typed OpenACC clause kind.
///
/// Introduction points follow the cumulative change list in the official
/// OpenACC 3.4 specification.
#[must_use]
pub const fn openacc_clause_availability(
    kind: AccClauseKind,
) -> FeatureAvailability<OpenAccVersion> {
    use AccClauseKind as C;
    use OpenAccVersion as V;

    match kind {
        // OpenACC 1.0 baseline.
        C::Async
        | C::Collapse
        | C::Copy
        | C::CopyIn
        | C::CopyOut
        | C::Create
        | C::Device
        | C::DevicePtr
        | C::DeviceResident
        | C::Firstprivate
        | C::Gang
        | C::If
        | C::Independent
        | C::NumGangs
        | C::NumWorkers
        | C::Present
        | C::Private
        | C::Reduction
        | C::Seq
        | C::UseDevice
        | C::Vector
        | C::VectorLength
        | C::Worker => acc(V::V1_0),

        // OpenACC 2.0 added data lifetime directives, routine/device-specific
        // compilation, atomics, auto/tile loops and wait clauses on computes.
        C::Auto
        | C::Bind
        | C::Capture
        | C::Default
        | C::Delete
        | C::DeviceType
        | C::Link
        | C::NoHost
        | C::Read
        | C::SelfClause
        | C::Tile
        | C::Update
        | C::Wait
        | C::Write => acc(V::V2_0),

        // OpenACC 2.5 added set/default-async selection and finalize.
        C::DefaultAsync | C::DeviceNum | C::Finalize => acc(V::V2_5),

        C::Indirect => FeatureAvailability::Nonstandard {
            reason: "indirect is an accparser extension, not standardized OpenACC syntax",
        },

        // OpenACC 2.5 added if_present to update. OpenACC 2.7 then added
        // no_create and attach/detach (and extended if_present to host_data).
        C::IfPresent => acc(V::V2_5),
        C::Attach | C::Detach | C::NoCreate => acc(V::V2_7),
    }
}

/// Classifies the spelling that produced an OpenACC clause.
///
/// The `p*` and `present_or_*` data-clause aliases were part of OpenACC 1.0
/// and remain accepted cumulatively even though the AST canonicalizes them to
/// their modern data-clause kinds.
#[must_use]
pub const fn openacc_clause_spelling_availability(
    clause: &AccClause,
) -> FeatureAvailability<OpenAccVersion> {
    use OpenAccVersion as V;

    match clause.source_alias() {
        Some(
            AccClauseSourceAlias::PCopy
            | AccClauseSourceAlias::PresentOrCopy
            | AccClauseSourceAlias::PCopyIn
            | AccClauseSourceAlias::PresentOrCopyIn
            | AccClauseSourceAlias::PCopyOut
            | AccClauseSourceAlias::PresentOrCopyOut
            | AccClauseSourceAlias::PCreate
            | AccClauseSourceAlias::PresentOrCreate,
        ) => acc(V::V1_0),
        Some(AccClauseSourceAlias::UpdateHost) => acc(V::V1_0),
        None => openacc_clause_availability(clause.kind()),
    }
}

/// Classifies the complete typed syntax of an OpenACC clause, including
/// modifier, argument-shape, and directive-specific introduction points.
#[must_use]
pub fn openacc_clause_syntax_availability(
    directive_kind: AccDirectiveKind,
    clause: &AccClause,
) -> FeatureAvailability<OpenAccVersion> {
    use OpenAccVersion as V;

    let mut result = openacc_clause_spelling_availability(clause);

    {
        let mut require = |version| result = result.intersect(acc(version));
        if acc_clause_contains_common_block(clause.payload()) {
            require(V::V2_0);
        }
        match &clause.payload() {
            AccClausePayload::Default(AccDefaultKind::Present) => require(V::V2_5),
            AccClausePayload::Copy(copy) => {
                for modifier in copy.modifiers() {
                    require(acc_data_modifier_introduction(*modifier));
                }
            }
            AccClausePayload::Create(create) => {
                for modifier in create.modifiers() {
                    require(acc_data_modifier_introduction(*modifier));
                }
            }
            AccClausePayload::Collapse(collapse) if collapse.force() => require(V::V3_4),
            AccClausePayload::Gang(gang)
                if gang
                    .arguments()
                    .iter()
                    .any(|argument| matches!(argument, AccGangArgument::Dim(_))) =>
            {
                require(V::V3_3)
            }
            AccClausePayload::Wait(wait) if wait.devnum().is_some() => require(V::V3_0),
            AccClausePayload::NumGangs(values) if values.len() > 1 => require(V::V3_3),
            _ => {}
        }
    }

    result.intersect(openacc_clause_usage_availability(
        directive_kind,
        clause.kind(),
    ))
}

fn acc_clause_contains_common_block(payload: &AccClausePayload) -> bool {
    let items = match payload {
        AccClausePayload::ItemList { items, .. } => Some(items.as_slice()),
        AccClausePayload::Copy(copy) => Some(copy.variables()),
        AccClausePayload::Create(create) => Some(create.variables()),
        AccClausePayload::Data(data) => Some(data.variables()),
        AccClausePayload::Reduction(reduction) => Some(reduction.variables()),
        AccClausePayload::Bare { .. }
        | AccClausePayload::Expression { .. }
        | AccClausePayload::NumGangs(_)
        | AccClausePayload::Tile(_)
        | AccClausePayload::Bind(_)
        | AccClausePayload::Indirect(_)
        | AccClausePayload::Collapse(_)
        | AccClausePayload::Default(_)
        | AccClausePayload::DeviceType(_)
        | AccClausePayload::Gang(_)
        | AccClausePayload::Worker(_)
        | AccClausePayload::Vector(_)
        | AccClausePayload::Wait(_) => None,
    };
    items.is_some_and(|items| {
        items
            .iter()
            .any(|item| matches!(item, ClauseItem::FortranCommonBlock(_)))
    })
}

fn acc_data_modifier_introduction(modifier: AccDataModifier) -> OpenAccVersion {
    match modifier {
        AccDataModifier::Readonly => OpenAccVersion::V2_7,
        AccDataModifier::Zero => OpenAccVersion::V3_0,
        AccDataModifier::Always
        | AccDataModifier::AlwaysIn
        | AccDataModifier::AlwaysOut
        | AccDataModifier::Capture => OpenAccVersion::V3_4,
    }
}

fn openacc_clause_usage_availability(
    directive: AccDirectiveKind,
    clause: AccClauseKind,
) -> FeatureAvailability<OpenAccVersion> {
    use AccClauseKind as C;
    use AccDirectiveKind as D;
    use OpenAccVersion as V;

    let introduced = match (directive, clause) {
        (D::Kernels | D::KernelsLoop, C::NumGangs | C::NumWorkers | C::VectorLength) => V::V2_5,
        (D::HostData, C::If | C::IfPresent) => V::V2_7,
        (D::Data, C::Default) => V::V2_7,
        (directive, C::SelfClause) if is_acc_compute(directive) => V::V2_7,
        (D::Init | D::Shutdown | D::Set | D::Wait, C::If) => V::V3_0,
        (D::Data, C::Async | C::Wait | C::DeviceType) => V::V3_2,
        (D::Atomic, C::If) => V::V3_4,
        _ => V::V1_0,
    };
    acc(introduced)
}

fn is_acc_compute(kind: AccDirectiveKind) -> bool {
    use AccDirectiveKind as D;
    matches!(
        kind,
        D::Parallel | D::ParallelLoop | D::Kernels | D::KernelsLoop | D::Serial | D::SerialLoop
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_openmp_clause_kind_has_an_explicit_classification() {
        let nonstandard = OmpClauseKind::ALL
            .iter()
            .copied()
            .filter(|kind| {
                matches!(
                    openmp_clause_availability(*kind),
                    FeatureAvailability::Nonstandard { .. }
                )
            })
            .collect::<Vec<_>>();

        assert!(nonstandard.is_empty());
    }

    #[test]
    fn every_openacc_clause_kind_has_an_explicit_classification() {
        let nonstandard = AccClauseKind::ALL
            .iter()
            .copied()
            .filter(|kind| {
                matches!(
                    openacc_clause_availability(*kind),
                    FeatureAvailability::Nonstandard { .. }
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(nonstandard, vec![AccClauseKind::Indirect]);
    }

    #[test]
    fn feature_versions_are_cumulative() {
        let omp_versions = openmp_clause_availability(OmpClauseKind::ProcBind)
            .compatible_versions()
            .unwrap();
        assert!(!omp_versions.contains(OpenMpVersion::V3_1));
        assert!(omp_versions.contains(OpenMpVersion::V4_0));
        assert!(omp_versions.contains(OpenMpVersion::V6_0));

        let acc_versions = openacc_clause_availability(AccClauseKind::NoCreate)
            .compatible_versions()
            .unwrap();
        assert!(!acc_versions.contains(OpenAccVersion::V2_6));
        assert!(acc_versions.contains(OpenAccVersion::V2_7));
        assert!(acc_versions.contains(OpenAccVersion::V3_4));
    }
}
