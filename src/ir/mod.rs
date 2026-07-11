//! Typed directive data shared by the OpenMP and OpenACC parsers.
#![forbid(unsafe_code)]

// Re-export main types
pub use crate::ast::{
    OmpSelector, OmpSelectorConstruct, OmpSelectorDeviceTrait, OmpSelectorEntry,
    OmpSelectorExtensionProperty, OmpSelectorExtensionTrait, OmpSelectorImplementationTrait,
    OmpSelectorImplementationTraitKind, OmpSelectorNameListKind, OmpSelectorNameListTrait,
    OmpSelectorRequirement, OmpSelectorTraitValue,
};
pub(crate) use clause::UsesAllocatorSourceSyntax;
pub use clause::{
    AdjustArgsModifier, AllocateSourceSyntax, AtKind, AtomicOp, BindModifier, ClauseData,
    ClauseItem, DefaultKind, DefaultmapBehavior, DefaultmapCategory, DependIterator, DependType,
    DepobjUpdateDependence, DeviceModifier, DeviceType, DoacrossType, ExtendedAtomicKind,
    FirstprivateModifier, GrainsizeModifier, LastprivateModifier, LinearModifier,
    LinearSourceSyntax, MapModifier, MapRefKind, MapType, MapTypeSpelling, MemoryOrder,
    MemscopeKind, NumTasksModifier, OmpAppendOperation, OmpApplyLoopKind, OmpApplyLoopModifier,
    OmpCount, OmpDependence, OmpDoacrossIteration, OmpDoacrossOffset, OmpDoacrossVectorItem,
    OmpForeignRuntimeIdentifier, OmpInductionModifier, OmpInteropInitModifiers, OmpInteropType,
    OmpLocator, OmpMemorySpace, OmpParameterListItem, OmpParameterRange, OmpPreferenceSelector,
    OmpPreferenceSpecification, OrderKind, OrderModifier, OriginalSharing, ProcBind,
    ReductionModifier, RequireModifier, ScanClauseMode, ScheduleKind, ScheduleModifier,
    SeverityKind, ThreadsetKind, UsesAllocatorBuiltin, UsesAllocatorKind, UsesAllocatorSpec,
};
pub use error::ConversionError;
pub use expression::{
    BinaryOperator, Expression, ExpressionAst, ExpressionError, ExpressionKind,
    MAX_STRUCTURAL_NESTING_DEPTH, ParserConfig, UnaryOperator,
};
pub use variable::{Identifier, IdentifierError, LValue, LValueError, Variable, VariableError};

mod clause;
mod error;
mod expression;
pub(crate) mod lang;
mod variable;
