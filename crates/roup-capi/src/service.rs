//! Safe state and parser service behind the C memory boundary.

use crate::boundary::BoundaryError;
use crate::handle::{GenerationalArena, Handle, HandleError};
use crate::{
    RoupClauseKind, RoupDirectiveHandle, RoupDirectiveKind, RoupErrorHandle, RoupFieldInfo,
    RoupNodeHandle, RoupNodeKind, RoupParameterKind, RoupParserHandle, RoupParserOptions, RoupSpan,
    RoupStatus, ROUP_ABI_VERSION, ROUP_DIAGNOSTIC_BUFFER_TOO_SMALL,
    ROUP_DIAGNOSTIC_INDEX_OUT_OF_RANGE, ROUP_DIAGNOSTIC_INTERNAL_ERROR,
    ROUP_DIAGNOSTIC_INVALID_HANDLE, ROUP_DIAGNOSTIC_INVALID_POINTER, ROUP_DIAGNOSTIC_INVALID_UTF8,
    ROUP_DIALECT_OPENACC, ROUP_DIALECT_OPENMP, ROUP_HOST_C, ROUP_HOST_CPP, ROUP_HOST_FORTRAN,
    ROUP_SOURCE_FORTRAN_FIXED, ROUP_SOURCE_FORTRAN_FREE, ROUP_SOURCE_PRAGMA, ROUP_VERSION_ANY,
    ROUP_VERSION_EXACT,
};
use roup::api::{OpenAccConfig, OpenAccParser, OpenMpConfig, OpenMpParser};
use roup::ast::{
    AccBindTarget, AccCacheItem, AccClause, AccClausePayload, AccDataModifier, AccDefaultKind,
    AccDeviceType, AccDirective, AccEndKind, AccGangArgument, AccReductionOperator,
    AccSizeExpression, AccVectorModifier, AccWorkerModifier, OmpClause, OmpDirective,
    RoupDirective,
};
use roup::diagnostic::{Diagnostic, DiagnosticCode};
use roup::version::{
    CStandard, CppStandard, DirectiveVersion, FortranStandard, HostLanguageProfile, OpenAccVersion,
    OpenMpVersion, SourceForm, VersionSet,
};
use std::sync::{Mutex, MutexGuard, OnceLock};

trait AbiStringLeaf {
    fn render_abi_leaf(&self) -> String;
}

impl AbiStringLeaf for str {
    fn render_abi_leaf(&self) -> String {
        self.to_owned()
    }
}

impl<T: AbiStringLeaf + ?Sized> AbiStringLeaf for Box<T> {
    fn render_abi_leaf(&self) -> String {
        self.as_ref().render_abi_leaf()
    }
}

macro_rules! impl_display_abi_leaf {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl AbiStringLeaf for $ty {
                fn render_abi_leaf(&self) -> String {
                    self.to_string()
                }
            }
        )+
    };
}

impl_display_abi_leaf!(
    roup::ir::Expression,
    roup::ir::Identifier,
    roup::ir::Variable,
    roup::ir::LValue,
    roup::host::TypeName,
    roup::ast::OmpCppTemplateId,
);

fn qualified_name_leaf(name: &roup::host::QualifiedName) -> String {
    let mut result = if name.global {
        "::".to_string()
    } else {
        String::new()
    };
    for (index, segment) in name.segments.iter().enumerate() {
        if index != 0 {
            result.push_str("::");
        }
        result.push_str(segment.as_str());
    }
    result
}

#[derive(Clone, Copy, Debug)]
enum ParserRecord {
    OpenMp(OpenMpParser),
    OpenAcc(OpenAccParser),
}

#[derive(Debug)]
struct DirectiveRecord {
    directive: RoupDirective,
    compatible_versions: u64,
}

#[derive(Clone, Debug)]
struct ErrorRecord {
    code: u32,
    span: RoupSpan,
    message: String,
}

#[derive(Clone, Debug)]
struct NodeRecord {
    kind: RoupNodeKind,
    fields: Vec<ClauseField>,
}

#[derive(Clone, Debug)]
enum FieldValue {
    Bool(bool),
    U32(u32),
    U64(u64),
    String(String),
    Strings(Vec<String>),
    U32s(Vec<u32>),
    Node(NodeRecord),
    Nodes(Vec<NodeRecord>),
}

#[derive(Clone, Debug)]
struct ClauseField {
    id: u32,
    name: &'static str,
    value: FieldValue,
}

impl ClauseField {
    fn u32(id: u32, name: &'static str, value: u32) -> Self {
        Self {
            id,
            name,
            value: FieldValue::U32(value),
        }
    }

    fn u32s(id: u32, name: &'static str, values: Vec<u32>) -> Self {
        Self {
            id,
            name,
            value: FieldValue::U32s(values),
        }
    }

    fn u64(id: u32, name: &'static str, value: u64) -> Self {
        Self {
            id,
            name,
            value: FieldValue::U64(value),
        }
    }

    fn string<T: AbiStringLeaf + ?Sized>(id: u32, name: &'static str, value: &T) -> Self {
        Self {
            id,
            name,
            value: FieldValue::String(value.render_abi_leaf()),
        }
    }

    fn strings<T: AbiStringLeaf>(id: u32, name: &'static str, values: &[T]) -> Self {
        Self {
            id,
            name,
            value: FieldValue::Strings(values.iter().map(AbiStringLeaf::render_abi_leaf).collect()),
        }
    }

    fn boolean(id: u32, name: &'static str, value: bool) -> Self {
        Self {
            id,
            name,
            value: FieldValue::Bool(value),
        }
    }

    fn node(id: u32, name: &'static str, value: NodeRecord) -> Self {
        Self {
            id,
            name,
            value: FieldValue::Node(value),
        }
    }

    fn nodes(id: u32, name: &'static str, values: Vec<NodeRecord>) -> Self {
        Self {
            id,
            name,
            value: FieldValue::Nodes(values),
        }
    }

    fn info(&self) -> RoupFieldInfo {
        let (value_kind, count) = match &self.value {
            FieldValue::Bool(_) => (crate::ROUP_FIELD_VALUE_BOOL, 1),
            FieldValue::U32(_) => (crate::ROUP_FIELD_VALUE_U32, 1),
            FieldValue::U64(_) => (crate::ROUP_FIELD_VALUE_U64, 1),
            FieldValue::String(_) => (crate::ROUP_FIELD_VALUE_STRING, 1),
            FieldValue::Strings(values) => (crate::ROUP_FIELD_VALUE_STRING_LIST, values.len()),
            FieldValue::U32s(values) => (crate::ROUP_FIELD_VALUE_U32_LIST, values.len()),
            FieldValue::Node(_) => (crate::ROUP_FIELD_VALUE_NODE, 1),
            FieldValue::Nodes(values) => (crate::ROUP_FIELD_VALUE_NODE_LIST, values.len()),
        };
        RoupFieldInfo {
            id: self.id,
            value_kind,
            count,
        }
    }
}

fn omp_directive_kind_ordinal(kind: roup::ast::OmpDirectiveKind) -> u32 {
    match kind {
        roup::ast::OmpDirectiveKind::Allocate => crate::ROUP_OMP_DIRECTIVE_ALLOCATE,
        roup::ast::OmpDirectiveKind::Allocators => crate::ROUP_OMP_DIRECTIVE_ALLOCATORS,
        roup::ast::OmpDirectiveKind::Assume => crate::ROUP_OMP_DIRECTIVE_ASSUME,
        roup::ast::OmpDirectiveKind::Assumes => crate::ROUP_OMP_DIRECTIVE_ASSUMES,
        roup::ast::OmpDirectiveKind::Atomic => crate::ROUP_OMP_DIRECTIVE_ATOMIC,
        roup::ast::OmpDirectiveKind::Barrier => crate::ROUP_OMP_DIRECTIVE_BARRIER,
        roup::ast::OmpDirectiveKind::BeginAssumes => crate::ROUP_OMP_DIRECTIVE_BEGIN_ASSUMES,
        roup::ast::OmpDirectiveKind::BeginDeclareTarget => {
            crate::ROUP_OMP_DIRECTIVE_BEGIN_DECLARE_TARGET
        }
        roup::ast::OmpDirectiveKind::BeginDeclareVariant => {
            crate::ROUP_OMP_DIRECTIVE_BEGIN_DECLARE_VARIANT
        }
        roup::ast::OmpDirectiveKind::Cancel => crate::ROUP_OMP_DIRECTIVE_CANCEL,
        roup::ast::OmpDirectiveKind::CancellationPoint => {
            crate::ROUP_OMP_DIRECTIVE_CANCELLATION_POINT
        }
        roup::ast::OmpDirectiveKind::Critical => crate::ROUP_OMP_DIRECTIVE_CRITICAL,
        roup::ast::OmpDirectiveKind::DeclareInduction => {
            crate::ROUP_OMP_DIRECTIVE_DECLARE_INDUCTION
        }
        roup::ast::OmpDirectiveKind::DeclareMapper => crate::ROUP_OMP_DIRECTIVE_DECLARE_MAPPER,
        roup::ast::OmpDirectiveKind::DeclareReduction => {
            crate::ROUP_OMP_DIRECTIVE_DECLARE_REDUCTION
        }
        roup::ast::OmpDirectiveKind::DeclareSimd => crate::ROUP_OMP_DIRECTIVE_DECLARE_SIMD,
        roup::ast::OmpDirectiveKind::DeclareTarget => crate::ROUP_OMP_DIRECTIVE_DECLARE_TARGET,
        roup::ast::OmpDirectiveKind::DeclareVariant => crate::ROUP_OMP_DIRECTIVE_DECLARE_VARIANT,
        roup::ast::OmpDirectiveKind::Depobj => crate::ROUP_OMP_DIRECTIVE_DEPOBJ,
        roup::ast::OmpDirectiveKind::Dispatch => crate::ROUP_OMP_DIRECTIVE_DISPATCH,
        roup::ast::OmpDirectiveKind::Distribute => crate::ROUP_OMP_DIRECTIVE_DISTRIBUTE,
        roup::ast::OmpDirectiveKind::DistributeParallelDo => {
            crate::ROUP_OMP_DIRECTIVE_DISTRIBUTE_PARALLEL_DO
        }
        roup::ast::OmpDirectiveKind::DistributeParallelDoSimd => {
            crate::ROUP_OMP_DIRECTIVE_DISTRIBUTE_PARALLEL_DO_SIMD
        }
        roup::ast::OmpDirectiveKind::DistributeParallelFor => {
            crate::ROUP_OMP_DIRECTIVE_DISTRIBUTE_PARALLEL_FOR
        }
        roup::ast::OmpDirectiveKind::DistributeParallelForSimd => {
            crate::ROUP_OMP_DIRECTIVE_DISTRIBUTE_PARALLEL_FOR_SIMD
        }
        roup::ast::OmpDirectiveKind::DistributeParallelLoop => {
            crate::ROUP_OMP_DIRECTIVE_DISTRIBUTE_PARALLEL_LOOP
        }
        roup::ast::OmpDirectiveKind::DistributeParallelLoopSimd => {
            crate::ROUP_OMP_DIRECTIVE_DISTRIBUTE_PARALLEL_LOOP_SIMD
        }
        roup::ast::OmpDirectiveKind::DistributeSimd => crate::ROUP_OMP_DIRECTIVE_DISTRIBUTE_SIMD,
        roup::ast::OmpDirectiveKind::Do => crate::ROUP_OMP_DIRECTIVE_DO,
        roup::ast::OmpDirectiveKind::DoSimd => crate::ROUP_OMP_DIRECTIVE_DO_SIMD,
        roup::ast::OmpDirectiveKind::EndAssume => crate::ROUP_OMP_DIRECTIVE_END_ASSUME,
        roup::ast::OmpDirectiveKind::EndAssumes => crate::ROUP_OMP_DIRECTIVE_END_ASSUMES,
        roup::ast::OmpDirectiveKind::EndAllocators => crate::ROUP_OMP_DIRECTIVE_END_ALLOCATORS,
        roup::ast::OmpDirectiveKind::EndDeclareTarget => {
            crate::ROUP_OMP_DIRECTIVE_END_DECLARE_TARGET
        }
        roup::ast::OmpDirectiveKind::EndDeclareVariant => {
            crate::ROUP_OMP_DIRECTIVE_END_DECLARE_VARIANT
        }
        roup::ast::OmpDirectiveKind::EndDispatch => crate::ROUP_OMP_DIRECTIVE_END_DISPATCH,
        roup::ast::OmpDirectiveKind::EndParallel => crate::ROUP_OMP_DIRECTIVE_END_PARALLEL,
        roup::ast::OmpDirectiveKind::EndDo => crate::ROUP_OMP_DIRECTIVE_END_DO,
        roup::ast::OmpDirectiveKind::EndSimd => crate::ROUP_OMP_DIRECTIVE_END_SIMD,
        roup::ast::OmpDirectiveKind::EndSections => crate::ROUP_OMP_DIRECTIVE_END_SECTIONS,
        roup::ast::OmpDirectiveKind::EndSingle => crate::ROUP_OMP_DIRECTIVE_END_SINGLE,
        roup::ast::OmpDirectiveKind::EndWorkshare => crate::ROUP_OMP_DIRECTIVE_END_WORKSHARE,
        roup::ast::OmpDirectiveKind::EndOrdered => crate::ROUP_OMP_DIRECTIVE_END_ORDERED,
        roup::ast::OmpDirectiveKind::EndLoop => crate::ROUP_OMP_DIRECTIVE_END_LOOP,
        roup::ast::OmpDirectiveKind::EndDistribute => crate::ROUP_OMP_DIRECTIVE_END_DISTRIBUTE,
        roup::ast::OmpDirectiveKind::EndTeams => crate::ROUP_OMP_DIRECTIVE_END_TEAMS,
        roup::ast::OmpDirectiveKind::EndTaskloop => crate::ROUP_OMP_DIRECTIVE_END_TASKLOOP,
        roup::ast::OmpDirectiveKind::EndTask => crate::ROUP_OMP_DIRECTIVE_END_TASK,
        roup::ast::OmpDirectiveKind::EndTaskgroup => crate::ROUP_OMP_DIRECTIVE_END_TASKGROUP,
        roup::ast::OmpDirectiveKind::EndMaster => crate::ROUP_OMP_DIRECTIVE_END_MASTER,
        roup::ast::OmpDirectiveKind::EndMasked => crate::ROUP_OMP_DIRECTIVE_END_MASKED,
        roup::ast::OmpDirectiveKind::EndUnroll => crate::ROUP_OMP_DIRECTIVE_END_UNROLL,
        roup::ast::OmpDirectiveKind::EndCritical => crate::ROUP_OMP_DIRECTIVE_END_CRITICAL,
        roup::ast::OmpDirectiveKind::EndAtomic => crate::ROUP_OMP_DIRECTIVE_END_ATOMIC,
        roup::ast::OmpDirectiveKind::EndParallelDo => crate::ROUP_OMP_DIRECTIVE_END_PARALLEL_DO,
        roup::ast::OmpDirectiveKind::EndParallelSections => {
            crate::ROUP_OMP_DIRECTIVE_END_PARALLEL_SECTIONS
        }
        roup::ast::OmpDirectiveKind::EndParallelWorkshare => {
            crate::ROUP_OMP_DIRECTIVE_END_PARALLEL_WORKSHARE
        }
        roup::ast::OmpDirectiveKind::EndParallelMaster => {
            crate::ROUP_OMP_DIRECTIVE_END_PARALLEL_MASTER
        }
        roup::ast::OmpDirectiveKind::EndParallelSingle => {
            crate::ROUP_OMP_DIRECTIVE_END_PARALLEL_SINGLE
        }
        roup::ast::OmpDirectiveKind::EndParallelMasterTaskloop => {
            crate::ROUP_OMP_DIRECTIVE_END_PARALLEL_MASTER_TASKLOOP
        }
        roup::ast::OmpDirectiveKind::EndParallelMasterTaskloopSimd => {
            crate::ROUP_OMP_DIRECTIVE_END_PARALLEL_MASTER_TASKLOOP_SIMD
        }
        roup::ast::OmpDirectiveKind::EndDoSimd => crate::ROUP_OMP_DIRECTIVE_END_DO_SIMD,
        roup::ast::OmpDirectiveKind::EndParallelDoSimd => {
            crate::ROUP_OMP_DIRECTIVE_END_PARALLEL_DO_SIMD
        }
        roup::ast::OmpDirectiveKind::EndDistributeSimd => {
            crate::ROUP_OMP_DIRECTIVE_END_DISTRIBUTE_SIMD
        }
        roup::ast::OmpDirectiveKind::EndDistributeParallelDo => {
            crate::ROUP_OMP_DIRECTIVE_END_DISTRIBUTE_PARALLEL_DO
        }
        roup::ast::OmpDirectiveKind::EndDistributeParallelDoSimd => {
            crate::ROUP_OMP_DIRECTIVE_END_DISTRIBUTE_PARALLEL_DO_SIMD
        }
        roup::ast::OmpDirectiveKind::EndTargetParallel => {
            crate::ROUP_OMP_DIRECTIVE_END_TARGET_PARALLEL
        }
        roup::ast::OmpDirectiveKind::EndTargetParallelDo => {
            crate::ROUP_OMP_DIRECTIVE_END_TARGET_PARALLEL_DO
        }
        roup::ast::OmpDirectiveKind::EndTargetParallelDoSimd => {
            crate::ROUP_OMP_DIRECTIVE_END_TARGET_PARALLEL_DO_SIMD
        }
        roup::ast::OmpDirectiveKind::EndTargetParallelLoop => {
            crate::ROUP_OMP_DIRECTIVE_END_TARGET_PARALLEL_LOOP
        }
        roup::ast::OmpDirectiveKind::EndTargetSimd => crate::ROUP_OMP_DIRECTIVE_END_TARGET_SIMD,
        roup::ast::OmpDirectiveKind::EndTargetTeams => crate::ROUP_OMP_DIRECTIVE_END_TARGET_TEAMS,
        roup::ast::OmpDirectiveKind::EndTargetTeamsDistribute => {
            crate::ROUP_OMP_DIRECTIVE_END_TARGET_TEAMS_DISTRIBUTE
        }
        roup::ast::OmpDirectiveKind::EndTargetTeamsDistributeParallelDo => {
            crate::ROUP_OMP_DIRECTIVE_END_TARGET_TEAMS_DISTRIBUTE_PARALLEL_DO
        }
        roup::ast::OmpDirectiveKind::EndTargetTeamsDistributeParallelDoSimd => {
            crate::ROUP_OMP_DIRECTIVE_END_TARGET_TEAMS_DISTRIBUTE_PARALLEL_DO_SIMD
        }
        roup::ast::OmpDirectiveKind::EndTargetTeamsDistributeSimd => {
            crate::ROUP_OMP_DIRECTIVE_END_TARGET_TEAMS_DISTRIBUTE_SIMD
        }
        roup::ast::OmpDirectiveKind::EndTargetTeamsLoop => {
            crate::ROUP_OMP_DIRECTIVE_END_TARGET_TEAMS_LOOP
        }
        roup::ast::OmpDirectiveKind::EndTargetTeamsWorkdistribute => {
            crate::ROUP_OMP_DIRECTIVE_END_TARGET_TEAMS_WORKDISTRIBUTE
        }
        roup::ast::OmpDirectiveKind::EndTeamsDistribute => {
            crate::ROUP_OMP_DIRECTIVE_END_TEAMS_DISTRIBUTE
        }
        roup::ast::OmpDirectiveKind::EndTeamsDistributeParallelDo => {
            crate::ROUP_OMP_DIRECTIVE_END_TEAMS_DISTRIBUTE_PARALLEL_DO
        }
        roup::ast::OmpDirectiveKind::EndTeamsDistributeParallelDoSimd => {
            crate::ROUP_OMP_DIRECTIVE_END_TEAMS_DISTRIBUTE_PARALLEL_DO_SIMD
        }
        roup::ast::OmpDirectiveKind::EndTeamsDistributeSimd => {
            crate::ROUP_OMP_DIRECTIVE_END_TEAMS_DISTRIBUTE_SIMD
        }
        roup::ast::OmpDirectiveKind::EndTeamsLoop => crate::ROUP_OMP_DIRECTIVE_END_TEAMS_LOOP,
        roup::ast::OmpDirectiveKind::EndTaskloopSimd => crate::ROUP_OMP_DIRECTIVE_END_TASKLOOP_SIMD,
        roup::ast::OmpDirectiveKind::EndMasterTaskloop => {
            crate::ROUP_OMP_DIRECTIVE_END_MASTER_TASKLOOP
        }
        roup::ast::OmpDirectiveKind::EndMasterTaskloopSimd => {
            crate::ROUP_OMP_DIRECTIVE_END_MASTER_TASKLOOP_SIMD
        }
        roup::ast::OmpDirectiveKind::EndMaskedTaskloop => {
            crate::ROUP_OMP_DIRECTIVE_END_MASKED_TASKLOOP
        }
        roup::ast::OmpDirectiveKind::EndMaskedTaskloopSimd => {
            crate::ROUP_OMP_DIRECTIVE_END_MASKED_TASKLOOP_SIMD
        }
        roup::ast::OmpDirectiveKind::EndParallelMasked => {
            crate::ROUP_OMP_DIRECTIVE_END_PARALLEL_MASKED
        }
        roup::ast::OmpDirectiveKind::EndParallelMaskedTaskloop => {
            crate::ROUP_OMP_DIRECTIVE_END_PARALLEL_MASKED_TASKLOOP
        }
        roup::ast::OmpDirectiveKind::EndParallelMaskedTaskloopSimd => {
            crate::ROUP_OMP_DIRECTIVE_END_PARALLEL_MASKED_TASKLOOP_SIMD
        }
        roup::ast::OmpDirectiveKind::EndParallelLoop => crate::ROUP_OMP_DIRECTIVE_END_PARALLEL_LOOP,
        roup::ast::OmpDirectiveKind::EndTargetLoop => crate::ROUP_OMP_DIRECTIVE_END_TARGET_LOOP,
        roup::ast::OmpDirectiveKind::EndTile => crate::ROUP_OMP_DIRECTIVE_END_TILE,
        roup::ast::OmpDirectiveKind::Error => crate::ROUP_OMP_DIRECTIVE_ERROR,
        roup::ast::OmpDirectiveKind::Flush => crate::ROUP_OMP_DIRECTIVE_FLUSH,
        roup::ast::OmpDirectiveKind::Fuse => crate::ROUP_OMP_DIRECTIVE_FUSE,
        roup::ast::OmpDirectiveKind::Groupprivate => crate::ROUP_OMP_DIRECTIVE_GROUPPRIVATE,
        roup::ast::OmpDirectiveKind::For => crate::ROUP_OMP_DIRECTIVE_FOR,
        roup::ast::OmpDirectiveKind::ForSimd => crate::ROUP_OMP_DIRECTIVE_FOR_SIMD,
        roup::ast::OmpDirectiveKind::Interchange => crate::ROUP_OMP_DIRECTIVE_INTERCHANGE,
        roup::ast::OmpDirectiveKind::Interop => crate::ROUP_OMP_DIRECTIVE_INTEROP,
        roup::ast::OmpDirectiveKind::Loop => crate::ROUP_OMP_DIRECTIVE_LOOP,
        roup::ast::OmpDirectiveKind::Reverse => crate::ROUP_OMP_DIRECTIVE_REVERSE,
        roup::ast::OmpDirectiveKind::Masked => crate::ROUP_OMP_DIRECTIVE_MASKED,
        roup::ast::OmpDirectiveKind::MaskedTaskloop => crate::ROUP_OMP_DIRECTIVE_MASKED_TASKLOOP,
        roup::ast::OmpDirectiveKind::MaskedTaskloopSimd => {
            crate::ROUP_OMP_DIRECTIVE_MASKED_TASKLOOP_SIMD
        }
        roup::ast::OmpDirectiveKind::Master => crate::ROUP_OMP_DIRECTIVE_MASTER,
        roup::ast::OmpDirectiveKind::MasterTaskloop => crate::ROUP_OMP_DIRECTIVE_MASTER_TASKLOOP,
        roup::ast::OmpDirectiveKind::MasterTaskloopSimd => {
            crate::ROUP_OMP_DIRECTIVE_MASTER_TASKLOOP_SIMD
        }
        roup::ast::OmpDirectiveKind::Metadirective => crate::ROUP_OMP_DIRECTIVE_METADIRECTIVE,
        roup::ast::OmpDirectiveKind::BeginMetadirective => {
            crate::ROUP_OMP_DIRECTIVE_BEGIN_METADIRECTIVE
        }
        roup::ast::OmpDirectiveKind::EndMetadirective => {
            crate::ROUP_OMP_DIRECTIVE_END_METADIRECTIVE
        }
        roup::ast::OmpDirectiveKind::Nothing => crate::ROUP_OMP_DIRECTIVE_NOTHING,
        roup::ast::OmpDirectiveKind::Ordered => crate::ROUP_OMP_DIRECTIVE_ORDERED,
        roup::ast::OmpDirectiveKind::Parallel => crate::ROUP_OMP_DIRECTIVE_PARALLEL,
        roup::ast::OmpDirectiveKind::ParallelDo => crate::ROUP_OMP_DIRECTIVE_PARALLEL_DO,
        roup::ast::OmpDirectiveKind::ParallelDoSimd => crate::ROUP_OMP_DIRECTIVE_PARALLEL_DO_SIMD,
        roup::ast::OmpDirectiveKind::ParallelFor => crate::ROUP_OMP_DIRECTIVE_PARALLEL_FOR,
        roup::ast::OmpDirectiveKind::ParallelForSimd => crate::ROUP_OMP_DIRECTIVE_PARALLEL_FOR_SIMD,
        roup::ast::OmpDirectiveKind::ParallelLoop => crate::ROUP_OMP_DIRECTIVE_PARALLEL_LOOP,
        roup::ast::OmpDirectiveKind::ParallelLoopSimd => {
            crate::ROUP_OMP_DIRECTIVE_PARALLEL_LOOP_SIMD
        }
        roup::ast::OmpDirectiveKind::ParallelMasked => crate::ROUP_OMP_DIRECTIVE_PARALLEL_MASKED,
        roup::ast::OmpDirectiveKind::ParallelMaskedTaskloop => {
            crate::ROUP_OMP_DIRECTIVE_PARALLEL_MASKED_TASKLOOP
        }
        roup::ast::OmpDirectiveKind::ParallelMaskedTaskloopSimd => {
            crate::ROUP_OMP_DIRECTIVE_PARALLEL_MASKED_TASKLOOP_SIMD
        }
        roup::ast::OmpDirectiveKind::ParallelMaster => crate::ROUP_OMP_DIRECTIVE_PARALLEL_MASTER,
        roup::ast::OmpDirectiveKind::ParallelMasterTaskloop => {
            crate::ROUP_OMP_DIRECTIVE_PARALLEL_MASTER_TASKLOOP
        }
        roup::ast::OmpDirectiveKind::ParallelMasterTaskloopSimd => {
            crate::ROUP_OMP_DIRECTIVE_PARALLEL_MASTER_TASKLOOP_SIMD
        }
        roup::ast::OmpDirectiveKind::ParallelSections => {
            crate::ROUP_OMP_DIRECTIVE_PARALLEL_SECTIONS
        }
        roup::ast::OmpDirectiveKind::ParallelSingle => crate::ROUP_OMP_DIRECTIVE_PARALLEL_SINGLE,
        roup::ast::OmpDirectiveKind::ParallelWorkshare => {
            crate::ROUP_OMP_DIRECTIVE_PARALLEL_WORKSHARE
        }
        roup::ast::OmpDirectiveKind::Requires => crate::ROUP_OMP_DIRECTIVE_REQUIRES,
        roup::ast::OmpDirectiveKind::Scope => crate::ROUP_OMP_DIRECTIVE_SCOPE,
        roup::ast::OmpDirectiveKind::EndScope => crate::ROUP_OMP_DIRECTIVE_END_SCOPE,
        roup::ast::OmpDirectiveKind::Scan => crate::ROUP_OMP_DIRECTIVE_SCAN,
        roup::ast::OmpDirectiveKind::Section => crate::ROUP_OMP_DIRECTIVE_SECTION,
        roup::ast::OmpDirectiveKind::Sections => crate::ROUP_OMP_DIRECTIVE_SECTIONS,
        roup::ast::OmpDirectiveKind::Simd => crate::ROUP_OMP_DIRECTIVE_SIMD,
        roup::ast::OmpDirectiveKind::Single => crate::ROUP_OMP_DIRECTIVE_SINGLE,
        roup::ast::OmpDirectiveKind::Split => crate::ROUP_OMP_DIRECTIVE_SPLIT,
        roup::ast::OmpDirectiveKind::Stripe => crate::ROUP_OMP_DIRECTIVE_STRIPE,
        roup::ast::OmpDirectiveKind::Target => crate::ROUP_OMP_DIRECTIVE_TARGET,
        roup::ast::OmpDirectiveKind::TargetData => crate::ROUP_OMP_DIRECTIVE_TARGET_DATA,
        roup::ast::OmpDirectiveKind::TargetEnterData => crate::ROUP_OMP_DIRECTIVE_TARGET_ENTER_DATA,
        roup::ast::OmpDirectiveKind::TargetExitData => crate::ROUP_OMP_DIRECTIVE_TARGET_EXIT_DATA,
        roup::ast::OmpDirectiveKind::EndTarget => crate::ROUP_OMP_DIRECTIVE_END_TARGET,
        roup::ast::OmpDirectiveKind::EndTargetData => crate::ROUP_OMP_DIRECTIVE_END_TARGET_DATA,
        roup::ast::OmpDirectiveKind::TargetLoop => crate::ROUP_OMP_DIRECTIVE_TARGET_LOOP,
        roup::ast::OmpDirectiveKind::TargetLoopSimd => crate::ROUP_OMP_DIRECTIVE_TARGET_LOOP_SIMD,
        roup::ast::OmpDirectiveKind::TargetParallel => crate::ROUP_OMP_DIRECTIVE_TARGET_PARALLEL,
        roup::ast::OmpDirectiveKind::TargetParallelDo => {
            crate::ROUP_OMP_DIRECTIVE_TARGET_PARALLEL_DO
        }
        roup::ast::OmpDirectiveKind::TargetParallelDoSimd => {
            crate::ROUP_OMP_DIRECTIVE_TARGET_PARALLEL_DO_SIMD
        }
        roup::ast::OmpDirectiveKind::TargetParallelFor => {
            crate::ROUP_OMP_DIRECTIVE_TARGET_PARALLEL_FOR
        }
        roup::ast::OmpDirectiveKind::TargetParallelForSimd => {
            crate::ROUP_OMP_DIRECTIVE_TARGET_PARALLEL_FOR_SIMD
        }
        roup::ast::OmpDirectiveKind::TargetParallelLoop => {
            crate::ROUP_OMP_DIRECTIVE_TARGET_PARALLEL_LOOP
        }
        roup::ast::OmpDirectiveKind::TargetParallelLoopSimd => {
            crate::ROUP_OMP_DIRECTIVE_TARGET_PARALLEL_LOOP_SIMD
        }
        roup::ast::OmpDirectiveKind::TargetSimd => crate::ROUP_OMP_DIRECTIVE_TARGET_SIMD,
        roup::ast::OmpDirectiveKind::TargetTeams => crate::ROUP_OMP_DIRECTIVE_TARGET_TEAMS,
        roup::ast::OmpDirectiveKind::TargetTeamsDistribute => {
            crate::ROUP_OMP_DIRECTIVE_TARGET_TEAMS_DISTRIBUTE
        }
        roup::ast::OmpDirectiveKind::TargetTeamsDistributeParallelDo => {
            crate::ROUP_OMP_DIRECTIVE_TARGET_TEAMS_DISTRIBUTE_PARALLEL_DO
        }
        roup::ast::OmpDirectiveKind::TargetTeamsDistributeParallelDoSimd => {
            crate::ROUP_OMP_DIRECTIVE_TARGET_TEAMS_DISTRIBUTE_PARALLEL_DO_SIMD
        }
        roup::ast::OmpDirectiveKind::TargetTeamsDistributeParallelFor => {
            crate::ROUP_OMP_DIRECTIVE_TARGET_TEAMS_DISTRIBUTE_PARALLEL_FOR
        }
        roup::ast::OmpDirectiveKind::TargetTeamsDistributeParallelForSimd => {
            crate::ROUP_OMP_DIRECTIVE_TARGET_TEAMS_DISTRIBUTE_PARALLEL_FOR_SIMD
        }
        roup::ast::OmpDirectiveKind::TargetTeamsDistributeParallelLoop => {
            crate::ROUP_OMP_DIRECTIVE_TARGET_TEAMS_DISTRIBUTE_PARALLEL_LOOP
        }
        roup::ast::OmpDirectiveKind::TargetTeamsDistributeParallelLoopSimd => {
            crate::ROUP_OMP_DIRECTIVE_TARGET_TEAMS_DISTRIBUTE_PARALLEL_LOOP_SIMD
        }
        roup::ast::OmpDirectiveKind::TargetTeamsDistributeSimd => {
            crate::ROUP_OMP_DIRECTIVE_TARGET_TEAMS_DISTRIBUTE_SIMD
        }
        roup::ast::OmpDirectiveKind::TargetTeamsLoop => crate::ROUP_OMP_DIRECTIVE_TARGET_TEAMS_LOOP,
        roup::ast::OmpDirectiveKind::TargetTeamsLoopSimd => {
            crate::ROUP_OMP_DIRECTIVE_TARGET_TEAMS_LOOP_SIMD
        }
        roup::ast::OmpDirectiveKind::TargetTeamsWorkdistribute => {
            crate::ROUP_OMP_DIRECTIVE_TARGET_TEAMS_WORKDISTRIBUTE
        }
        roup::ast::OmpDirectiveKind::TargetUpdate => crate::ROUP_OMP_DIRECTIVE_TARGET_UPDATE,
        roup::ast::OmpDirectiveKind::Task => crate::ROUP_OMP_DIRECTIVE_TASK,
        roup::ast::OmpDirectiveKind::TaskIteration => crate::ROUP_OMP_DIRECTIVE_TASK_ITERATION,
        roup::ast::OmpDirectiveKind::Taskgroup => crate::ROUP_OMP_DIRECTIVE_TASKGROUP,
        roup::ast::OmpDirectiveKind::Taskgraph => crate::ROUP_OMP_DIRECTIVE_TASKGRAPH,
        roup::ast::OmpDirectiveKind::Taskloop => crate::ROUP_OMP_DIRECTIVE_TASKLOOP,
        roup::ast::OmpDirectiveKind::TaskloopSimd => crate::ROUP_OMP_DIRECTIVE_TASKLOOP_SIMD,
        roup::ast::OmpDirectiveKind::Taskwait => crate::ROUP_OMP_DIRECTIVE_TASKWAIT,
        roup::ast::OmpDirectiveKind::Taskyield => crate::ROUP_OMP_DIRECTIVE_TASKYIELD,
        roup::ast::OmpDirectiveKind::Teams => crate::ROUP_OMP_DIRECTIVE_TEAMS,
        roup::ast::OmpDirectiveKind::TeamsDistribute => crate::ROUP_OMP_DIRECTIVE_TEAMS_DISTRIBUTE,
        roup::ast::OmpDirectiveKind::TeamsDistributeParallelDo => {
            crate::ROUP_OMP_DIRECTIVE_TEAMS_DISTRIBUTE_PARALLEL_DO
        }
        roup::ast::OmpDirectiveKind::TeamsDistributeParallelDoSimd => {
            crate::ROUP_OMP_DIRECTIVE_TEAMS_DISTRIBUTE_PARALLEL_DO_SIMD
        }
        roup::ast::OmpDirectiveKind::TeamsDistributeParallelFor => {
            crate::ROUP_OMP_DIRECTIVE_TEAMS_DISTRIBUTE_PARALLEL_FOR
        }
        roup::ast::OmpDirectiveKind::TeamsDistributeParallelForSimd => {
            crate::ROUP_OMP_DIRECTIVE_TEAMS_DISTRIBUTE_PARALLEL_FOR_SIMD
        }
        roup::ast::OmpDirectiveKind::TeamsDistributeParallelLoop => {
            crate::ROUP_OMP_DIRECTIVE_TEAMS_DISTRIBUTE_PARALLEL_LOOP
        }
        roup::ast::OmpDirectiveKind::TeamsDistributeParallelLoopSimd => {
            crate::ROUP_OMP_DIRECTIVE_TEAMS_DISTRIBUTE_PARALLEL_LOOP_SIMD
        }
        roup::ast::OmpDirectiveKind::TeamsDistributeSimd => {
            crate::ROUP_OMP_DIRECTIVE_TEAMS_DISTRIBUTE_SIMD
        }
        roup::ast::OmpDirectiveKind::TeamsLoop => crate::ROUP_OMP_DIRECTIVE_TEAMS_LOOP,
        roup::ast::OmpDirectiveKind::TeamsLoopSimd => crate::ROUP_OMP_DIRECTIVE_TEAMS_LOOP_SIMD,
        roup::ast::OmpDirectiveKind::Threadprivate => crate::ROUP_OMP_DIRECTIVE_THREADPRIVATE,
        roup::ast::OmpDirectiveKind::Tile => crate::ROUP_OMP_DIRECTIVE_TILE,
        roup::ast::OmpDirectiveKind::Unroll => crate::ROUP_OMP_DIRECTIVE_UNROLL,
        roup::ast::OmpDirectiveKind::Workdistribute => crate::ROUP_OMP_DIRECTIVE_WORKDISTRIBUTE,
        roup::ast::OmpDirectiveKind::Workshare => crate::ROUP_OMP_DIRECTIVE_WORKSHARE,
    }
}

fn omp_clause_kind_ordinal(kind: roup::ast::OmpClauseKind) -> u32 {
    match kind {
        roup::ast::OmpClauseKind::Absent => crate::ROUP_OMP_CLAUSE_ABSENT,
        roup::ast::OmpClauseKind::AcqRel => crate::ROUP_OMP_CLAUSE_ACQ_REL,
        roup::ast::OmpClauseKind::Acquire => crate::ROUP_OMP_CLAUSE_ACQUIRE,
        roup::ast::OmpClauseKind::AdjustArgs => crate::ROUP_OMP_CLAUSE_ADJUST_ARGS,
        roup::ast::OmpClauseKind::Affinity => crate::ROUP_OMP_CLAUSE_AFFINITY,
        roup::ast::OmpClauseKind::Align => crate::ROUP_OMP_CLAUSE_ALIGN,
        roup::ast::OmpClauseKind::Aligned => crate::ROUP_OMP_CLAUSE_ALIGNED,
        roup::ast::OmpClauseKind::Allocate => crate::ROUP_OMP_CLAUSE_ALLOCATE,
        roup::ast::OmpClauseKind::Allocator => crate::ROUP_OMP_CLAUSE_ALLOCATOR,
        roup::ast::OmpClauseKind::AppendArgs => crate::ROUP_OMP_CLAUSE_APPEND_ARGS,
        roup::ast::OmpClauseKind::Apply => crate::ROUP_OMP_CLAUSE_APPLY,
        roup::ast::OmpClauseKind::At => crate::ROUP_OMP_CLAUSE_AT,
        roup::ast::OmpClauseKind::AtomicDefaultMemOrder => {
            crate::ROUP_OMP_CLAUSE_ATOMIC_DEFAULT_MEM_ORDER
        }
        roup::ast::OmpClauseKind::Bind => crate::ROUP_OMP_CLAUSE_BIND,
        roup::ast::OmpClauseKind::Capture => crate::ROUP_OMP_CLAUSE_CAPTURE,
        roup::ast::OmpClauseKind::Collapse => crate::ROUP_OMP_CLAUSE_COLLAPSE,
        roup::ast::OmpClauseKind::Collector => crate::ROUP_OMP_CLAUSE_COLLECTOR,
        roup::ast::OmpClauseKind::Combiner => crate::ROUP_OMP_CLAUSE_COMBINER,
        roup::ast::OmpClauseKind::Compare => crate::ROUP_OMP_CLAUSE_COMPARE,
        roup::ast::OmpClauseKind::Contains => crate::ROUP_OMP_CLAUSE_CONTAINS,
        roup::ast::OmpClauseKind::CopyIn => crate::ROUP_OMP_CLAUSE_COPY_IN,
        roup::ast::OmpClauseKind::Copyprivate => crate::ROUP_OMP_CLAUSE_COPYPRIVATE,
        roup::ast::OmpClauseKind::Parallel => crate::ROUP_OMP_CLAUSE_PARALLEL,
        roup::ast::OmpClauseKind::Sections => crate::ROUP_OMP_CLAUSE_SECTIONS,
        roup::ast::OmpClauseKind::For => crate::ROUP_OMP_CLAUSE_FOR,
        roup::ast::OmpClauseKind::Do => crate::ROUP_OMP_CLAUSE_DO,
        roup::ast::OmpClauseKind::Taskgroup => crate::ROUP_OMP_CLAUSE_TASKGROUP,
        roup::ast::OmpClauseKind::Counts => crate::ROUP_OMP_CLAUSE_COUNTS,
        roup::ast::OmpClauseKind::Default => crate::ROUP_OMP_CLAUSE_DEFAULT,
        roup::ast::OmpClauseKind::Defaultmap => crate::ROUP_OMP_CLAUSE_DEFAULTMAP,
        roup::ast::OmpClauseKind::Depend => crate::ROUP_OMP_CLAUSE_DEPEND,
        roup::ast::OmpClauseKind::DepobjUpdate => crate::ROUP_OMP_CLAUSE_DEPOBJ_UPDATE,
        roup::ast::OmpClauseKind::Destroy => crate::ROUP_OMP_CLAUSE_DESTROY,
        roup::ast::OmpClauseKind::Detach => crate::ROUP_OMP_CLAUSE_DETACH,
        roup::ast::OmpClauseKind::Device => crate::ROUP_OMP_CLAUSE_DEVICE,
        roup::ast::OmpClauseKind::DeviceSafesync => crate::ROUP_OMP_CLAUSE_DEVICE_SAFESYNC,
        roup::ast::OmpClauseKind::DeviceType => crate::ROUP_OMP_CLAUSE_DEVICE_TYPE,
        roup::ast::OmpClauseKind::DistSchedule => crate::ROUP_OMP_CLAUSE_DIST_SCHEDULE,
        roup::ast::OmpClauseKind::Doacross => crate::ROUP_OMP_CLAUSE_DOACROSS,
        roup::ast::OmpClauseKind::DynamicAllocators => crate::ROUP_OMP_CLAUSE_DYNAMIC_ALLOCATORS,
        roup::ast::OmpClauseKind::ExtImplementationDefinedRequirement => {
            crate::ROUP_OMP_CLAUSE_EXT_IMPLEMENTATION_DEFINED_REQUIREMENT
        }
        roup::ast::OmpClauseKind::Enter => crate::ROUP_OMP_CLAUSE_ENTER,
        roup::ast::OmpClauseKind::Exclusive => crate::ROUP_OMP_CLAUSE_EXCLUSIVE,
        roup::ast::OmpClauseKind::Fail => crate::ROUP_OMP_CLAUSE_FAIL,
        roup::ast::OmpClauseKind::Final => crate::ROUP_OMP_CLAUSE_FINAL,
        roup::ast::OmpClauseKind::Filter => crate::ROUP_OMP_CLAUSE_FILTER,
        roup::ast::OmpClauseKind::Firstprivate => crate::ROUP_OMP_CLAUSE_FIRSTPRIVATE,
        roup::ast::OmpClauseKind::From => crate::ROUP_OMP_CLAUSE_FROM,
        roup::ast::OmpClauseKind::Full => crate::ROUP_OMP_CLAUSE_FULL,
        roup::ast::OmpClauseKind::Grainsize => crate::ROUP_OMP_CLAUSE_GRAINSIZE,
        roup::ast::OmpClauseKind::GraphId => crate::ROUP_OMP_CLAUSE_GRAPH_ID,
        roup::ast::OmpClauseKind::GraphReset => crate::ROUP_OMP_CLAUSE_GRAPH_RESET,
        roup::ast::OmpClauseKind::HasDeviceAddr => crate::ROUP_OMP_CLAUSE_HAS_DEVICE_ADDR,
        roup::ast::OmpClauseKind::Hint => crate::ROUP_OMP_CLAUSE_HINT,
        roup::ast::OmpClauseKind::Holds => crate::ROUP_OMP_CLAUSE_HOLDS,
        roup::ast::OmpClauseKind::If => crate::ROUP_OMP_CLAUSE_IF,
        roup::ast::OmpClauseKind::InReduction => crate::ROUP_OMP_CLAUSE_IN_REDUCTION,
        roup::ast::OmpClauseKind::Induction => crate::ROUP_OMP_CLAUSE_INDUCTION,
        roup::ast::OmpClauseKind::Inductor => crate::ROUP_OMP_CLAUSE_INDUCTOR,
        roup::ast::OmpClauseKind::Inbranch => crate::ROUP_OMP_CLAUSE_INBRANCH,
        roup::ast::OmpClauseKind::Inclusive => crate::ROUP_OMP_CLAUSE_INCLUSIVE,
        roup::ast::OmpClauseKind::Init => crate::ROUP_OMP_CLAUSE_INIT,
        roup::ast::OmpClauseKind::InitComplete => crate::ROUP_OMP_CLAUSE_INIT_COMPLETE,
        roup::ast::OmpClauseKind::Initializer => crate::ROUP_OMP_CLAUSE_INITIALIZER,
        roup::ast::OmpClauseKind::Indirect => crate::ROUP_OMP_CLAUSE_INDIRECT,
        roup::ast::OmpClauseKind::IsDevicePtr => crate::ROUP_OMP_CLAUSE_IS_DEVICE_PTR,
        roup::ast::OmpClauseKind::Lastprivate => crate::ROUP_OMP_CLAUSE_LASTPRIVATE,
        roup::ast::OmpClauseKind::Linear => crate::ROUP_OMP_CLAUSE_LINEAR,
        roup::ast::OmpClauseKind::Link => crate::ROUP_OMP_CLAUSE_LINK,
        roup::ast::OmpClauseKind::Local => crate::ROUP_OMP_CLAUSE_LOCAL,
        roup::ast::OmpClauseKind::Looprange => crate::ROUP_OMP_CLAUSE_LOOPRANGE,
        roup::ast::OmpClauseKind::Map => crate::ROUP_OMP_CLAUSE_MAP,
        roup::ast::OmpClauseKind::Match => crate::ROUP_OMP_CLAUSE_MATCH,
        roup::ast::OmpClauseKind::Message => crate::ROUP_OMP_CLAUSE_MESSAGE,
        roup::ast::OmpClauseKind::Memscope => crate::ROUP_OMP_CLAUSE_MEMSCOPE,
        roup::ast::OmpClauseKind::Mergeable => crate::ROUP_OMP_CLAUSE_MERGEABLE,
        roup::ast::OmpClauseKind::Nocontext => crate::ROUP_OMP_CLAUSE_NOCONTEXT,
        roup::ast::OmpClauseKind::Nogroup => crate::ROUP_OMP_CLAUSE_NOGROUP,
        roup::ast::OmpClauseKind::NoOpenmp => crate::ROUP_OMP_CLAUSE_NO_OPENMP,
        roup::ast::OmpClauseKind::NoOpenmpConstructs => crate::ROUP_OMP_CLAUSE_NO_OPENMP_CONSTRUCTS,
        roup::ast::OmpClauseKind::NoOpenmpRoutines => crate::ROUP_OMP_CLAUSE_NO_OPENMP_ROUTINES,
        roup::ast::OmpClauseKind::NoParallelism => crate::ROUP_OMP_CLAUSE_NO_PARALLELISM,
        roup::ast::OmpClauseKind::Nontemporal => crate::ROUP_OMP_CLAUSE_NONTEMPORAL,
        roup::ast::OmpClauseKind::Notinbranch => crate::ROUP_OMP_CLAUSE_NOTINBRANCH,
        roup::ast::OmpClauseKind::Novariants => crate::ROUP_OMP_CLAUSE_NOVARIANTS,
        roup::ast::OmpClauseKind::Interop => crate::ROUP_OMP_CLAUSE_INTEROP,
        roup::ast::OmpClauseKind::Nowait => crate::ROUP_OMP_CLAUSE_NOWAIT,
        roup::ast::OmpClauseKind::NumTasks => crate::ROUP_OMP_CLAUSE_NUM_TASKS,
        roup::ast::OmpClauseKind::NumTeams => crate::ROUP_OMP_CLAUSE_NUM_TEAMS,
        roup::ast::OmpClauseKind::NumThreads => crate::ROUP_OMP_CLAUSE_NUM_THREADS,
        roup::ast::OmpClauseKind::Order => crate::ROUP_OMP_CLAUSE_ORDER,
        roup::ast::OmpClauseKind::Ordered => crate::ROUP_OMP_CLAUSE_ORDERED,
        roup::ast::OmpClauseKind::Otherwise => crate::ROUP_OMP_CLAUSE_OTHERWISE,
        roup::ast::OmpClauseKind::Partial => crate::ROUP_OMP_CLAUSE_PARTIAL,
        roup::ast::OmpClauseKind::Permutation => crate::ROUP_OMP_CLAUSE_PERMUTATION,
        roup::ast::OmpClauseKind::Priority => crate::ROUP_OMP_CLAUSE_PRIORITY,
        roup::ast::OmpClauseKind::Private => crate::ROUP_OMP_CLAUSE_PRIVATE,
        roup::ast::OmpClauseKind::ProcBind => crate::ROUP_OMP_CLAUSE_PROC_BIND,
        roup::ast::OmpClauseKind::Read => crate::ROUP_OMP_CLAUSE_READ,
        roup::ast::OmpClauseKind::Reduction => crate::ROUP_OMP_CLAUSE_REDUCTION,
        roup::ast::OmpClauseKind::Release => crate::ROUP_OMP_CLAUSE_RELEASE,
        roup::ast::OmpClauseKind::Relaxed => crate::ROUP_OMP_CLAUSE_RELAXED,
        roup::ast::OmpClauseKind::Replayable => crate::ROUP_OMP_CLAUSE_REPLAYABLE,
        roup::ast::OmpClauseKind::ReverseOffload => crate::ROUP_OMP_CLAUSE_REVERSE_OFFLOAD,
        roup::ast::OmpClauseKind::Safelen => crate::ROUP_OMP_CLAUSE_SAFELEN,
        roup::ast::OmpClauseKind::Safesync => crate::ROUP_OMP_CLAUSE_SAFESYNC,
        roup::ast::OmpClauseKind::Schedule => crate::ROUP_OMP_CLAUSE_SCHEDULE,
        roup::ast::OmpClauseKind::SelfMaps => crate::ROUP_OMP_CLAUSE_SELF_MAPS,
        roup::ast::OmpClauseKind::SeqCst => crate::ROUP_OMP_CLAUSE_SEQ_CST,
        roup::ast::OmpClauseKind::Severity => crate::ROUP_OMP_CLAUSE_SEVERITY,
        roup::ast::OmpClauseKind::Shared => crate::ROUP_OMP_CLAUSE_SHARED,
        roup::ast::OmpClauseKind::Simd => crate::ROUP_OMP_CLAUSE_SIMD,
        roup::ast::OmpClauseKind::Simdlen => crate::ROUP_OMP_CLAUSE_SIMDLEN,
        roup::ast::OmpClauseKind::Sizes => crate::ROUP_OMP_CLAUSE_SIZES,
        roup::ast::OmpClauseKind::TaskReduction => crate::ROUP_OMP_CLAUSE_TASK_REDUCTION,
        roup::ast::OmpClauseKind::ThreadLimit => crate::ROUP_OMP_CLAUSE_THREAD_LIMIT,
        roup::ast::OmpClauseKind::Threads => crate::ROUP_OMP_CLAUSE_THREADS,
        roup::ast::OmpClauseKind::Threadset => crate::ROUP_OMP_CLAUSE_THREADSET,
        roup::ast::OmpClauseKind::To => crate::ROUP_OMP_CLAUSE_TO,
        roup::ast::OmpClauseKind::Transparent => crate::ROUP_OMP_CLAUSE_TRANSPARENT,
        roup::ast::OmpClauseKind::UnifiedAddress => crate::ROUP_OMP_CLAUSE_UNIFIED_ADDRESS,
        roup::ast::OmpClauseKind::UnifiedSharedMemory => {
            crate::ROUP_OMP_CLAUSE_UNIFIED_SHARED_MEMORY
        }
        roup::ast::OmpClauseKind::Uniform => crate::ROUP_OMP_CLAUSE_UNIFORM,
        roup::ast::OmpClauseKind::Untied => crate::ROUP_OMP_CLAUSE_UNTIED,
        roup::ast::OmpClauseKind::Update => crate::ROUP_OMP_CLAUSE_UPDATE,
        roup::ast::OmpClauseKind::Use => crate::ROUP_OMP_CLAUSE_USE,
        roup::ast::OmpClauseKind::UseDeviceAddr => crate::ROUP_OMP_CLAUSE_USE_DEVICE_ADDR,
        roup::ast::OmpClauseKind::UseDevicePtr => crate::ROUP_OMP_CLAUSE_USE_DEVICE_PTR,
        roup::ast::OmpClauseKind::UsesAllocators => crate::ROUP_OMP_CLAUSE_USES_ALLOCATORS,
        roup::ast::OmpClauseKind::Weak => crate::ROUP_OMP_CLAUSE_WEAK,
        roup::ast::OmpClauseKind::When => crate::ROUP_OMP_CLAUSE_WHEN,
        roup::ast::OmpClauseKind::Write => crate::ROUP_OMP_CLAUSE_WRITE,
    }
}

fn acc_directive_kind_ordinal(kind: roup::ast::AccDirectiveKind) -> u32 {
    match kind {
        roup::ast::AccDirectiveKind::Atomic => crate::ROUP_ACC_DIRECTIVE_ATOMIC,
        roup::ast::AccDirectiveKind::Cache => crate::ROUP_ACC_DIRECTIVE_CACHE,
        roup::ast::AccDirectiveKind::Data => crate::ROUP_ACC_DIRECTIVE_DATA,
        roup::ast::AccDirectiveKind::Declare => crate::ROUP_ACC_DIRECTIVE_DECLARE,
        roup::ast::AccDirectiveKind::End => crate::ROUP_ACC_DIRECTIVE_END,
        roup::ast::AccDirectiveKind::EnterData => crate::ROUP_ACC_DIRECTIVE_ENTER_DATA,
        roup::ast::AccDirectiveKind::ExitData => crate::ROUP_ACC_DIRECTIVE_EXIT_DATA,
        roup::ast::AccDirectiveKind::HostData => crate::ROUP_ACC_DIRECTIVE_HOST_DATA,
        roup::ast::AccDirectiveKind::Init => crate::ROUP_ACC_DIRECTIVE_INIT,
        roup::ast::AccDirectiveKind::Kernels => crate::ROUP_ACC_DIRECTIVE_KERNELS,
        roup::ast::AccDirectiveKind::KernelsLoop => crate::ROUP_ACC_DIRECTIVE_KERNELS_LOOP,
        roup::ast::AccDirectiveKind::Loop => crate::ROUP_ACC_DIRECTIVE_LOOP,
        roup::ast::AccDirectiveKind::Parallel => crate::ROUP_ACC_DIRECTIVE_PARALLEL,
        roup::ast::AccDirectiveKind::ParallelLoop => crate::ROUP_ACC_DIRECTIVE_PARALLEL_LOOP,
        roup::ast::AccDirectiveKind::Routine => crate::ROUP_ACC_DIRECTIVE_ROUTINE,
        roup::ast::AccDirectiveKind::Serial => crate::ROUP_ACC_DIRECTIVE_SERIAL,
        roup::ast::AccDirectiveKind::SerialLoop => crate::ROUP_ACC_DIRECTIVE_SERIAL_LOOP,
        roup::ast::AccDirectiveKind::Set => crate::ROUP_ACC_DIRECTIVE_SET,
        roup::ast::AccDirectiveKind::Shutdown => crate::ROUP_ACC_DIRECTIVE_SHUTDOWN,
        roup::ast::AccDirectiveKind::Update => crate::ROUP_ACC_DIRECTIVE_UPDATE,
        roup::ast::AccDirectiveKind::Wait => crate::ROUP_ACC_DIRECTIVE_WAIT,
    }
}

fn acc_clause_kind_ordinal(kind: roup::ast::AccClauseKind) -> u32 {
    match kind {
        roup::ast::AccClauseKind::Async => crate::ROUP_ACC_CLAUSE_ASYNC,
        roup::ast::AccClauseKind::Attach => crate::ROUP_ACC_CLAUSE_ATTACH,
        roup::ast::AccClauseKind::Auto => crate::ROUP_ACC_CLAUSE_AUTO,
        roup::ast::AccClauseKind::Bind => crate::ROUP_ACC_CLAUSE_BIND,
        roup::ast::AccClauseKind::Capture => crate::ROUP_ACC_CLAUSE_CAPTURE,
        roup::ast::AccClauseKind::Collapse => crate::ROUP_ACC_CLAUSE_COLLAPSE,
        roup::ast::AccClauseKind::Copy => crate::ROUP_ACC_CLAUSE_COPY,
        roup::ast::AccClauseKind::CopyIn => crate::ROUP_ACC_CLAUSE_COPY_IN,
        roup::ast::AccClauseKind::CopyOut => crate::ROUP_ACC_CLAUSE_COPY_OUT,
        roup::ast::AccClauseKind::Create => crate::ROUP_ACC_CLAUSE_CREATE,
        roup::ast::AccClauseKind::Default => crate::ROUP_ACC_CLAUSE_DEFAULT,
        roup::ast::AccClauseKind::DefaultAsync => crate::ROUP_ACC_CLAUSE_DEFAULT_ASYNC,
        roup::ast::AccClauseKind::Delete => crate::ROUP_ACC_CLAUSE_DELETE,
        roup::ast::AccClauseKind::Detach => crate::ROUP_ACC_CLAUSE_DETACH,
        roup::ast::AccClauseKind::Device => crate::ROUP_ACC_CLAUSE_DEVICE,
        roup::ast::AccClauseKind::DeviceNum => crate::ROUP_ACC_CLAUSE_DEVICE_NUM,
        roup::ast::AccClauseKind::DeviceResident => crate::ROUP_ACC_CLAUSE_DEVICE_RESIDENT,
        roup::ast::AccClauseKind::DeviceType => crate::ROUP_ACC_CLAUSE_DEVICE_TYPE,
        roup::ast::AccClauseKind::DevicePtr => crate::ROUP_ACC_CLAUSE_DEVICE_PTR,
        roup::ast::AccClauseKind::Finalize => crate::ROUP_ACC_CLAUSE_FINALIZE,
        roup::ast::AccClauseKind::Firstprivate => crate::ROUP_ACC_CLAUSE_FIRSTPRIVATE,
        roup::ast::AccClauseKind::Gang => crate::ROUP_ACC_CLAUSE_GANG,
        roup::ast::AccClauseKind::If => crate::ROUP_ACC_CLAUSE_IF,
        roup::ast::AccClauseKind::IfPresent => crate::ROUP_ACC_CLAUSE_IF_PRESENT,
        roup::ast::AccClauseKind::Independent => crate::ROUP_ACC_CLAUSE_INDEPENDENT,
        roup::ast::AccClauseKind::Link => crate::ROUP_ACC_CLAUSE_LINK,
        roup::ast::AccClauseKind::NoCreate => crate::ROUP_ACC_CLAUSE_NO_CREATE,
        roup::ast::AccClauseKind::NoHost => crate::ROUP_ACC_CLAUSE_NO_HOST,
        roup::ast::AccClauseKind::NumGangs => crate::ROUP_ACC_CLAUSE_NUM_GANGS,
        roup::ast::AccClauseKind::NumWorkers => crate::ROUP_ACC_CLAUSE_NUM_WORKERS,
        roup::ast::AccClauseKind::Present => crate::ROUP_ACC_CLAUSE_PRESENT,
        roup::ast::AccClauseKind::Private => crate::ROUP_ACC_CLAUSE_PRIVATE,
        roup::ast::AccClauseKind::Reduction => crate::ROUP_ACC_CLAUSE_REDUCTION,
        roup::ast::AccClauseKind::Read => crate::ROUP_ACC_CLAUSE_READ,
        roup::ast::AccClauseKind::SelfClause => crate::ROUP_ACC_CLAUSE_SELF_CLAUSE,
        roup::ast::AccClauseKind::Seq => crate::ROUP_ACC_CLAUSE_SEQ,
        roup::ast::AccClauseKind::Tile => crate::ROUP_ACC_CLAUSE_TILE,
        roup::ast::AccClauseKind::Update => crate::ROUP_ACC_CLAUSE_UPDATE,
        roup::ast::AccClauseKind::UseDevice => crate::ROUP_ACC_CLAUSE_USE_DEVICE,
        roup::ast::AccClauseKind::Vector => crate::ROUP_ACC_CLAUSE_VECTOR,
        roup::ast::AccClauseKind::VectorLength => crate::ROUP_ACC_CLAUSE_VECTOR_LENGTH,
        roup::ast::AccClauseKind::Wait => crate::ROUP_ACC_CLAUSE_WAIT,
        roup::ast::AccClauseKind::Worker => crate::ROUP_ACC_CLAUSE_WORKER,
        roup::ast::AccClauseKind::Write => crate::ROUP_ACC_CLAUSE_WRITE,
    }
}

fn abi_span(span: roup::source::Span) -> RoupSpan {
    RoupSpan {
        start_byte: span.start_byte(),
        end_byte: span.end_byte(),
        start_line: span.start().line(),
        start_column: span.start().column(),
        end_line: span.end().line(),
        end_column: span.end().column(),
    }
}

impl ErrorRecord {
    fn from_diagnostic(diagnostic: Diagnostic) -> Self {
        let span = diagnostic.primary_span();
        Self {
            code: u32::from(diagnostic.code().number()),
            span: abi_span(span),
            message: diagnostic.message().to_owned(),
        }
    }

    fn abi(code: u32, span: RoupSpan, message: impl Into<String>) -> Self {
        Self {
            code,
            span,
            message: message.into(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Failure {
    pub(crate) status: RoupStatus,
    pub(crate) error: RoupErrorHandle,
}

#[derive(Debug)]
struct UnrecordedFailure {
    status: RoupStatus,
    error: ErrorRecord,
}

impl UnrecordedFailure {
    fn diagnostic(status: RoupStatus, diagnostic: Diagnostic) -> Self {
        Self {
            status,
            error: ErrorRecord::from_diagnostic(diagnostic),
        }
    }

    fn abi(status: RoupStatus, code: u32, message: impl Into<String>) -> Self {
        Self {
            status,
            error: ErrorRecord::abi(code, RoupSpan::default(), message),
        }
    }
}

pub(crate) type ServiceResult<T> = Result<T, Failure>;
type UnrecordedResult<T> = Result<T, UnrecordedFailure>;

#[derive(Debug)]
enum StoredObject {
    Parser(ParserRecord),
    Directive(DirectiveRecord),
    Node(NodeRecord),
    Error(ErrorRecord),
}

impl StoredObject {
    const fn kind_name(&self) -> &'static str {
        match self {
            Self::Parser(_) => "parser",
            Self::Directive(_) => "directive",
            Self::Node(_) => "node",
            Self::Error(_) => "error",
        }
    }
}

#[derive(Debug, Default)]
struct State {
    objects: GenerationalArena<StoredObject>,
}

const EMERGENCY_ERROR_HANDLE: RoupErrorHandle = RoupErrorHandle {
    generation: 1,
    index: u32::MAX as u64,
};

const EMERGENCY_ERROR_CODE: u32 = ROUP_DIAGNOSTIC_INTERNAL_ERROR;
const EMERGENCY_ERROR_MESSAGE: &str = "C ABI internal state is unavailable";

impl State {
    fn record(&mut self, unrecorded: UnrecordedFailure) -> Failure {
        match self.objects.insert(StoredObject::Error(unrecorded.error)) {
            Ok(handle) => Failure {
                status: unrecorded.status,
                error: RoupErrorHandle::active(handle.index(), handle.generation()),
            },
            Err(_) => Failure {
                status: RoupStatus::INTERNAL_ERROR,
                error: EMERGENCY_ERROR_HANDLE,
            },
        }
    }
}

static STATE: OnceLock<Mutex<State>> = OnceLock::new();

fn state() -> &'static Mutex<State> {
    STATE.get_or_init(|| Mutex::new(State::default()))
}

fn lock_state() -> Result<MutexGuard<'static, State>, Failure> {
    match state().lock() {
        Ok(state) => Ok(state),
        Err(_) => Err(poisoned_state_failure()),
    }
}

fn with_state<T>(operation: impl FnOnce(&mut State) -> UnrecordedResult<T>) -> ServiceResult<T> {
    let mut state = lock_state()?;
    match operation(&mut state) {
        Ok(value) => Ok(value),
        Err(unrecorded) => Err(state.record(unrecorded)),
    }
}

pub(crate) fn record_internal(message: impl Into<String>) -> Failure {
    let unrecorded = UnrecordedFailure::abi(
        RoupStatus::INTERNAL_ERROR,
        ROUP_DIAGNOSTIC_INTERNAL_ERROR,
        message,
    );
    match state().lock() {
        Ok(mut state) => state.record(unrecorded),
        Err(_) => poisoned_state_failure(),
    }
}

fn record_unrecorded(unrecorded: UnrecordedFailure) -> Failure {
    match state().lock() {
        Ok(mut state) => state.record(unrecorded),
        Err(_) => poisoned_state_failure(),
    }
}

const fn poisoned_state_failure() -> Failure {
    Failure {
        status: RoupStatus::INTERNAL_ERROR,
        error: EMERGENCY_ERROR_HANDLE,
    }
}

pub(crate) fn boundary_failure(error: BoundaryError) -> Failure {
    let (status, code, span) = match error {
        BoundaryError::NullPointer | BoundaryError::LengthOverflow { .. } => (
            RoupStatus::INVALID_ARGUMENT,
            ROUP_DIAGNOSTIC_INVALID_POINTER,
            RoupSpan::default(),
        ),
        BoundaryError::InvalidUtf8 { valid_up_to, .. } => (
            RoupStatus::INVALID_UTF8,
            ROUP_DIAGNOSTIC_INVALID_UTF8,
            RoupSpan {
                start_byte: valid_up_to,
                end_byte: valid_up_to,
                ..RoupSpan::default()
            },
        ),
        BoundaryError::BufferTooSmall { .. } => (
            RoupStatus::BUFFER_TOO_SMALL,
            ROUP_DIAGNOSTIC_BUFFER_TOO_SMALL,
            RoupSpan::default(),
        ),
    };
    record_unrecorded(UnrecordedFailure {
        status,
        error: ErrorRecord::abi(code, span, error.to_string()),
    })
}

pub(crate) fn create_parser(options: RoupParserOptions) -> ServiceResult<RoupParserHandle> {
    let parser = validate_options(options).map_err(record_unrecorded)?;
    with_state(|state| {
        let handle = state
            .objects
            .insert(StoredObject::Parser(parser))
            .map_err(arena_internal)?;
        Ok(RoupParserHandle::active(
            handle.index(),
            handle.generation(),
        ))
    })
}

pub(crate) fn release_parser(handle: RoupParserHandle) -> ServiceResult<()> {
    with_state(|state| {
        let handle = parser_handle(handle)?;
        remove_stored_object(state, handle, StoredObjectKind::Parser)?;
        Ok(())
    })
}

pub(crate) fn parse(
    parser: RoupParserHandle,
    source: String,
) -> ServiceResult<RoupDirectiveHandle> {
    let parser = with_state(|state| {
        let handle = parser_handle(parser)?;
        match state.objects.get(handle).map_err(handle_failure)? {
            StoredObject::Parser(parser) => Ok(*parser),
            object => Err(stored_object_kind_failure("parser", object)),
        }
    })?;

    let directive = match parser {
        ParserRecord::OpenMp(parser) => parser
            .parse(&source)
            .map(|parsed| DirectiveRecord {
                compatible_versions: version_bits(parsed.compatible_versions()),
                directive: RoupDirective::OpenMp(Box::new(parsed.into_directive())),
            })
            .map_err(parse_failure)?,
        ParserRecord::OpenAcc(parser) => parser
            .parse(&source)
            .map(|parsed| DirectiveRecord {
                compatible_versions: version_bits(parsed.compatible_versions()),
                directive: RoupDirective::OpenAcc(Box::new(parsed.into_directive())),
            })
            .map_err(parse_failure)?,
    };

    with_state(|state| {
        let handle = state
            .objects
            .insert(StoredObject::Directive(directive))
            .map_err(arena_internal)?;
        Ok(RoupDirectiveHandle::active(
            handle.index(),
            handle.generation(),
        ))
    })
}

fn parse_failure(diagnostic: Diagnostic) -> Failure {
    record_unrecorded(UnrecordedFailure::diagnostic(
        RoupStatus::PARSE_ERROR,
        diagnostic,
    ))
}

pub(crate) fn release_directive(handle: RoupDirectiveHandle) -> ServiceResult<()> {
    with_state(|state| {
        let handle = directive_handle(handle)?;
        remove_stored_object(state, handle, StoredObjectKind::Directive)?;
        Ok(())
    })
}

pub(crate) fn directive_dialect(handle: RoupDirectiveHandle) -> ServiceResult<u32> {
    with_directive(handle, |record| {
        Ok(match record.directive {
            RoupDirective::OpenMp(_) => ROUP_DIALECT_OPENMP,
            RoupDirective::OpenAcc(_) => ROUP_DIALECT_OPENACC,
        })
    })
}

pub(crate) fn directive_kind(handle: RoupDirectiveHandle) -> ServiceResult<RoupDirectiveKind> {
    with_directive(handle, |record| match &record.directive {
        RoupDirective::OpenMp(directive) => Ok(RoupDirectiveKind {
            dialect: ROUP_DIALECT_OPENMP,
            ordinal: omp_directive_kind_ordinal(directive.kind()),
        }),
        RoupDirective::OpenAcc(directive) => Ok(RoupDirectiveKind {
            dialect: ROUP_DIALECT_OPENACC,
            ordinal: acc_directive_kind_ordinal(directive.kind()),
        }),
    })
}

pub(crate) fn directive_span(handle: RoupDirectiveHandle) -> ServiceResult<RoupSpan> {
    with_directive(handle, |record| {
        Ok(match &record.directive {
            RoupDirective::OpenMp(directive) => abi_span(directive.span()),
            RoupDirective::OpenAcc(directive) => abi_span(directive.span()),
        })
    })
}

pub(crate) fn directive_has_parameter(handle: RoupDirectiveHandle) -> ServiceResult<u32> {
    with_directive(handle, |record| {
        Ok(u32::from(match &record.directive {
            RoupDirective::OpenMp(directive) => directive.parameter().is_some(),
            RoupDirective::OpenAcc(directive) => directive.parameter().is_some(),
        }))
    })
}

pub(crate) fn directive_parameter_kind(
    handle: RoupDirectiveHandle,
) -> ServiceResult<RoupParameterKind> {
    with_directive(handle, |record| match &record.directive {
        RoupDirective::OpenMp(directive) => Ok(RoupParameterKind {
            dialect: ROUP_DIALECT_OPENMP,
            variant: omp_parameter_variant(directive.parameter().ok_or_else(no_parameter_failure)?),
        }),
        RoupDirective::OpenAcc(directive) => Ok(RoupParameterKind {
            dialect: ROUP_DIALECT_OPENACC,
            variant: match directive.parameter().ok_or_else(no_parameter_failure)? {
                roup::ast::AccDirectiveParameter::Cache(_) => crate::ROUP_ACC_PARAMETER_CACHE,
                roup::ast::AccDirectiveParameter::Wait(_) => crate::ROUP_ACC_PARAMETER_WAIT,
                roup::ast::AccDirectiveParameter::Routine(_) => crate::ROUP_ACC_PARAMETER_ROUTINE,
                roup::ast::AccDirectiveParameter::End(_) => crate::ROUP_ACC_PARAMETER_END,
            },
        }),
    })
}

pub(crate) fn directive_parameter_field_count(handle: RoupDirectiveHandle) -> ServiceResult<usize> {
    with_parameter_fields(handle, |fields| Ok(fields.len()))
}

pub(crate) fn directive_parameter_field_info(
    handle: RoupDirectiveHandle,
    field_index: usize,
) -> ServiceResult<RoupFieldInfo> {
    with_parameter_fields(handle, |fields| Ok(field_at(fields, field_index)?.info()))
}

pub(crate) fn directive_parameter_field_name(
    handle: RoupDirectiveHandle,
    field_index: usize,
) -> ServiceResult<String> {
    with_parameter_fields(handle, |fields| {
        Ok(field_at(fields, field_index)?.name.to_owned())
    })
}

pub(crate) fn directive_parameter_field_bool(
    handle: RoupDirectiveHandle,
    field_index: usize,
    value_index: usize,
) -> ServiceResult<u32> {
    with_parameter_fields(handle, |fields| {
        scalar_field_bool(field_at(fields, field_index)?, value_index)
    })
}

pub(crate) fn directive_parameter_field_u32(
    handle: RoupDirectiveHandle,
    field_index: usize,
    value_index: usize,
) -> ServiceResult<u32> {
    with_parameter_fields(handle, |fields| {
        scalar_field_u32(field_at(fields, field_index)?, value_index)
    })
}

pub(crate) fn directive_parameter_field_u64(
    handle: RoupDirectiveHandle,
    field_index: usize,
    value_index: usize,
) -> ServiceResult<u64> {
    with_parameter_fields(handle, |fields| {
        scalar_field_u64(field_at(fields, field_index)?, value_index)
    })
}

pub(crate) fn directive_parameter_field_string(
    handle: RoupDirectiveHandle,
    field_index: usize,
    value_index: usize,
) -> ServiceResult<String> {
    with_parameter_fields(handle, |fields| {
        scalar_field_string(field_at(fields, field_index)?, value_index)
    })
}

pub(crate) fn directive_parameter_field_node(
    handle: RoupDirectiveHandle,
    field_index: usize,
    value_index: usize,
) -> ServiceResult<RoupNodeHandle> {
    let node = with_parameter_fields(handle, |fields| {
        scalar_field_node(field_at(fields, field_index)?, value_index)
    })?;
    store_node(node)
}

pub(crate) fn directive_name(handle: RoupDirectiveHandle) -> ServiceResult<String> {
    with_directive(handle, |record| {
        Ok(match &record.directive {
            RoupDirective::OpenMp(directive) => directive.kind().as_str().to_owned(),
            RoupDirective::OpenAcc(directive) => directive.kind().as_str().to_owned(),
        })
    })
}

pub(crate) fn directive_clause_count(handle: RoupDirectiveHandle) -> ServiceResult<usize> {
    with_directive(handle, |record| {
        Ok(match &record.directive {
            RoupDirective::OpenMp(directive) => directive.clauses().len(),
            RoupDirective::OpenAcc(directive) => directive.clauses().len(),
        })
    })
}

pub(crate) fn directive_compatible_versions(handle: RoupDirectiveHandle) -> ServiceResult<u64> {
    with_directive(handle, |record| Ok(record.compatible_versions))
}

pub(crate) fn clause_kind(
    handle: RoupDirectiveHandle,
    index: usize,
) -> ServiceResult<RoupClauseKind> {
    with_directive(handle, |record| match &record.directive {
        RoupDirective::OpenMp(directive) => {
            let clause = omp_clause(directive, index)?;
            Ok(RoupClauseKind {
                dialect: ROUP_DIALECT_OPENMP,
                ordinal: omp_clause_kind_ordinal(clause.kind()),
            })
        }
        RoupDirective::OpenAcc(directive) => {
            let clause = acc_clause(directive, index)?;
            Ok(RoupClauseKind {
                dialect: ROUP_DIALECT_OPENACC,
                ordinal: acc_clause_kind_ordinal(clause.kind()),
            })
        }
    })
}

pub(crate) fn clause_span(handle: RoupDirectiveHandle, index: usize) -> ServiceResult<RoupSpan> {
    with_directive(handle, |record| match &record.directive {
        RoupDirective::OpenMp(directive) => Ok(abi_span(omp_clause(directive, index)?.span())),
        RoupDirective::OpenAcc(directive) => Ok(abi_span(acc_clause(directive, index)?.span())),
    })
}

pub(crate) fn clause_name(handle: RoupDirectiveHandle, index: usize) -> ServiceResult<String> {
    with_directive(handle, |record| match &record.directive {
        RoupDirective::OpenMp(directive) => {
            Ok(omp_clause(directive, index)?.kind().as_str().to_owned())
        }
        RoupDirective::OpenAcc(directive) => {
            Ok(acc_clause(directive, index)?.kind().as_str().to_owned())
        }
    })
}

pub(crate) fn clause_field_count(
    handle: RoupDirectiveHandle,
    clause_index: usize,
) -> ServiceResult<usize> {
    with_clause_fields(handle, clause_index, |fields| Ok(fields.len()))
}

pub(crate) fn clause_field_info(
    handle: RoupDirectiveHandle,
    clause_index: usize,
    field_index: usize,
) -> ServiceResult<RoupFieldInfo> {
    with_clause_fields(handle, clause_index, |fields| {
        Ok(field_at(fields, field_index)?.info())
    })
}

pub(crate) fn clause_field_name(
    handle: RoupDirectiveHandle,
    clause_index: usize,
    field_index: usize,
) -> ServiceResult<String> {
    with_clause_fields(handle, clause_index, |fields| {
        Ok(field_at(fields, field_index)?.name.to_owned())
    })
}

pub(crate) fn clause_field_bool(
    handle: RoupDirectiveHandle,
    clause_index: usize,
    field_index: usize,
    value_index: usize,
) -> ServiceResult<u32> {
    with_clause_fields(handle, clause_index, |fields| {
        scalar_field_bool(field_at(fields, field_index)?, value_index)
    })
}

pub(crate) fn clause_field_u32(
    handle: RoupDirectiveHandle,
    clause_index: usize,
    field_index: usize,
    value_index: usize,
) -> ServiceResult<u32> {
    with_clause_fields(handle, clause_index, |fields| {
        scalar_field_u32(field_at(fields, field_index)?, value_index)
    })
}

pub(crate) fn clause_field_u64(
    handle: RoupDirectiveHandle,
    clause_index: usize,
    field_index: usize,
    value_index: usize,
) -> ServiceResult<u64> {
    with_clause_fields(handle, clause_index, |fields| {
        scalar_field_u64(field_at(fields, field_index)?, value_index)
    })
}

pub(crate) fn clause_field_string(
    handle: RoupDirectiveHandle,
    clause_index: usize,
    field_index: usize,
    value_index: usize,
) -> ServiceResult<String> {
    with_clause_fields(handle, clause_index, |fields| {
        scalar_field_string(field_at(fields, field_index)?, value_index)
    })
}

pub(crate) fn clause_field_node(
    handle: RoupDirectiveHandle,
    clause_index: usize,
    field_index: usize,
    value_index: usize,
) -> ServiceResult<RoupNodeHandle> {
    let node = with_clause_fields(handle, clause_index, |fields| {
        scalar_field_node(field_at(fields, field_index)?, value_index)
    })?;
    store_node(node)
}

pub(crate) fn node_kind(handle: RoupNodeHandle) -> ServiceResult<RoupNodeKind> {
    with_node(handle, |node| Ok(node.kind))
}

pub(crate) fn node_field_count(handle: RoupNodeHandle) -> ServiceResult<usize> {
    with_node(handle, |node| Ok(node.fields.len()))
}

pub(crate) fn node_field_info(
    handle: RoupNodeHandle,
    field_index: usize,
) -> ServiceResult<RoupFieldInfo> {
    with_node_fields(handle, |fields| Ok(field_at(fields, field_index)?.info()))
}

pub(crate) fn node_field_name(handle: RoupNodeHandle, field_index: usize) -> ServiceResult<String> {
    with_node_fields(handle, |fields| {
        Ok(field_at(fields, field_index)?.name.to_owned())
    })
}

pub(crate) fn node_field_bool(
    handle: RoupNodeHandle,
    field_index: usize,
    value_index: usize,
) -> ServiceResult<u32> {
    with_node_fields(handle, |fields| {
        scalar_field_bool(field_at(fields, field_index)?, value_index)
    })
}

pub(crate) fn node_field_u32(
    handle: RoupNodeHandle,
    field_index: usize,
    value_index: usize,
) -> ServiceResult<u32> {
    with_node_fields(handle, |fields| {
        scalar_field_u32(field_at(fields, field_index)?, value_index)
    })
}

pub(crate) fn node_field_u64(
    handle: RoupNodeHandle,
    field_index: usize,
    value_index: usize,
) -> ServiceResult<u64> {
    with_node_fields(handle, |fields| {
        scalar_field_u64(field_at(fields, field_index)?, value_index)
    })
}

pub(crate) fn node_field_string(
    handle: RoupNodeHandle,
    field_index: usize,
    value_index: usize,
) -> ServiceResult<String> {
    with_node_fields(handle, |fields| {
        scalar_field_string(field_at(fields, field_index)?, value_index)
    })
}

pub(crate) fn node_field_node(
    handle: RoupNodeHandle,
    field_index: usize,
    value_index: usize,
) -> ServiceResult<RoupNodeHandle> {
    let node = with_node_fields(handle, |fields| {
        scalar_field_node(field_at(fields, field_index)?, value_index)
    })?;
    store_node(node)
}

pub(crate) fn release_node(handle: RoupNodeHandle) -> ServiceResult<()> {
    with_state(|state| {
        let handle = node_handle(handle)?;
        remove_stored_object(state, handle, StoredObjectKind::Node)?;
        Ok(())
    })
}

pub(crate) fn error_code(handle: RoupErrorHandle) -> ServiceResult<u32> {
    if handle == EMERGENCY_ERROR_HANDLE {
        return Ok(EMERGENCY_ERROR_CODE);
    }
    with_error(handle, |error| Ok(error.code))
}

pub(crate) fn error_span(handle: RoupErrorHandle) -> ServiceResult<RoupSpan> {
    if handle == EMERGENCY_ERROR_HANDLE {
        return Ok(RoupSpan::default());
    }
    with_error(handle, |error| Ok(error.span))
}

pub(crate) fn error_message(handle: RoupErrorHandle) -> ServiceResult<String> {
    if handle == EMERGENCY_ERROR_HANDLE {
        return Ok(EMERGENCY_ERROR_MESSAGE.to_owned());
    }
    with_error(handle, |error| Ok(error.message.clone()))
}

pub(crate) fn release_error(handle: RoupErrorHandle) -> ServiceResult<()> {
    if handle == EMERGENCY_ERROR_HANDLE {
        return Ok(());
    }
    with_state(|state| {
        let handle = error_handle(handle)?;
        remove_stored_object(state, handle, StoredObjectKind::Error)?;
        Ok(())
    })
}

fn with_directive<T>(
    handle: RoupDirectiveHandle,
    operation: impl FnOnce(&DirectiveRecord) -> UnrecordedResult<T>,
) -> ServiceResult<T> {
    with_state(|state| {
        let handle = directive_handle(handle)?;
        match state.objects.get(handle).map_err(handle_failure)? {
            StoredObject::Directive(directive) => operation(directive),
            object => Err(stored_object_kind_failure("directive", object)),
        }
    })
}

fn with_error<T>(
    handle: RoupErrorHandle,
    operation: impl FnOnce(&ErrorRecord) -> UnrecordedResult<T>,
) -> ServiceResult<T> {
    with_state(|state| {
        let handle = error_handle(handle)?;
        match state.objects.get(handle).map_err(handle_failure)? {
            StoredObject::Error(error) => operation(error),
            object => Err(stored_object_kind_failure("error", object)),
        }
    })
}

fn with_node<T>(
    handle: RoupNodeHandle,
    operation: impl FnOnce(&NodeRecord) -> UnrecordedResult<T>,
) -> ServiceResult<T> {
    with_state(|state| {
        let handle = node_handle(handle)?;
        match state.objects.get(handle).map_err(handle_failure)? {
            StoredObject::Node(node) => operation(node),
            object => Err(stored_object_kind_failure("node", object)),
        }
    })
}

fn with_node_fields<T>(
    handle: RoupNodeHandle,
    operation: impl FnOnce(&[ClauseField]) -> UnrecordedResult<T>,
) -> ServiceResult<T> {
    with_node(handle, |node| operation(&node.fields))
}

fn store_node(node: NodeRecord) -> ServiceResult<RoupNodeHandle> {
    with_state(|state| {
        let handle = state
            .objects
            .insert(StoredObject::Node(node))
            .map_err(arena_internal)?;
        Ok(RoupNodeHandle::active(handle.index(), handle.generation()))
    })
}

#[cfg(test)]
pub(crate) fn store_test_u32_node() -> ServiceResult<RoupNodeHandle> {
    store_node(NodeRecord {
        kind: RoupNodeKind {
            family: u32::MAX,
            variant: 0,
        },
        fields: vec![
            ClauseField::u32(crate::ROUP_FIELD_VALUE, "scalar", 17),
            ClauseField::u32s(crate::ROUP_FIELD_VALUES, "values", vec![3, 5, 8]),
        ],
    })
}

fn with_clause_fields<T>(
    handle: RoupDirectiveHandle,
    clause_index: usize,
    operation: impl FnOnce(&[ClauseField]) -> UnrecordedResult<T>,
) -> ServiceResult<T> {
    with_directive(handle, |record| {
        let fields = match &record.directive {
            RoupDirective::OpenMp(directive) => {
                omp_clause_fields(omp_clause(directive, clause_index)?)?
            }
            RoupDirective::OpenAcc(directive) => {
                acc_fields(acc_clause(directive, clause_index)?.payload())?
            }
        };
        operation(&fields)
    })
}

fn with_parameter_fields<T>(
    handle: RoupDirectiveHandle,
    operation: impl FnOnce(&[ClauseField]) -> UnrecordedResult<T>,
) -> ServiceResult<T> {
    with_directive(handle, |record| {
        let fields = parameter_fields(&record.directive)?;
        operation(&fields)
    })
}

fn field_at(fields: &[ClauseField], index: usize) -> UnrecordedResult<&ClauseField> {
    fields.get(index).ok_or_else(|| {
        UnrecordedFailure::abi(
            RoupStatus::INVALID_ARGUMENT,
            ROUP_DIAGNOSTIC_INDEX_OUT_OF_RANGE,
            format!(
                "clause field index {index} is out of range for {} fields",
                fields.len()
            ),
        )
    })
}

fn scalar_field_bool(field: &ClauseField, value_index: usize) -> UnrecordedResult<u32> {
    match &field.value {
        FieldValue::Bool(value) if value_index == 0 => Ok(u32::from(*value)),
        FieldValue::Bool(_) => Err(value_index_failure(field.name, value_index, 1)),
        FieldValue::U32(_)
        | FieldValue::U64(_)
        | FieldValue::U32s(_)
        | FieldValue::String(_)
        | FieldValue::Strings(_)
        | FieldValue::Node(_)
        | FieldValue::Nodes(_) => Err(field_type_failure(field.name, "a boolean")),
    }
}

fn scalar_field_u32(field: &ClauseField, value_index: usize) -> UnrecordedResult<u32> {
    match &field.value {
        FieldValue::U32(value) if value_index == 0 => Ok(*value),
        FieldValue::U32s(values) => values
            .get(value_index)
            .copied()
            .ok_or_else(|| value_index_failure(field.name, value_index, values.len())),
        FieldValue::U32(_) => Err(value_index_failure(field.name, value_index, 1)),
        FieldValue::Bool(_)
        | FieldValue::U64(_)
        | FieldValue::String(_)
        | FieldValue::Strings(_)
        | FieldValue::Node(_)
        | FieldValue::Nodes(_) => Err(field_type_failure(field.name, "an unsigned 32-bit integer")),
    }
}

fn scalar_field_u64(field: &ClauseField, value_index: usize) -> UnrecordedResult<u64> {
    match &field.value {
        FieldValue::U64(value) if value_index == 0 => Ok(*value),
        FieldValue::U64(_) => Err(value_index_failure(field.name, value_index, 1)),
        FieldValue::Bool(_)
        | FieldValue::U32(_)
        | FieldValue::U32s(_)
        | FieldValue::String(_)
        | FieldValue::Strings(_)
        | FieldValue::Node(_)
        | FieldValue::Nodes(_) => Err(field_type_failure(field.name, "an unsigned 64-bit integer")),
    }
}

fn scalar_field_string(field: &ClauseField, value_index: usize) -> UnrecordedResult<String> {
    match &field.value {
        FieldValue::String(value) if value_index == 0 => Ok(value.clone()),
        FieldValue::Strings(values) => values
            .get(value_index)
            .cloned()
            .ok_or_else(|| value_index_failure(field.name, value_index, values.len())),
        FieldValue::String(_) => Err(value_index_failure(field.name, value_index, 1)),
        FieldValue::Bool(_)
        | FieldValue::U32(_)
        | FieldValue::U64(_)
        | FieldValue::U32s(_)
        | FieldValue::Node(_)
        | FieldValue::Nodes(_) => Err(field_type_failure(field.name, "a string")),
    }
}

fn scalar_field_node(field: &ClauseField, value_index: usize) -> UnrecordedResult<NodeRecord> {
    match &field.value {
        FieldValue::Node(value) if value_index == 0 => Ok(value.clone()),
        FieldValue::Nodes(values) => values
            .get(value_index)
            .cloned()
            .ok_or_else(|| value_index_failure(field.name, value_index, values.len())),
        FieldValue::Node(_) => Err(value_index_failure(field.name, value_index, 1)),
        FieldValue::Bool(_)
        | FieldValue::U32(_)
        | FieldValue::U64(_)
        | FieldValue::U32s(_)
        | FieldValue::String(_)
        | FieldValue::Strings(_) => Err(field_type_failure(field.name, "a semantic node")),
    }
}

fn value_index_failure(name: &str, index: usize, count: usize) -> UnrecordedFailure {
    UnrecordedFailure::abi(
        RoupStatus::INVALID_ARGUMENT,
        ROUP_DIAGNOSTIC_INDEX_OUT_OF_RANGE,
        format!("value index {index} is out of range for field {name:?} with {count} values"),
    )
}

fn field_type_failure(name: &str, expected: &str) -> UnrecordedFailure {
    UnrecordedFailure::abi(
        RoupStatus::INVALID_ARGUMENT,
        ROUP_DIAGNOSTIC_INDEX_OUT_OF_RANGE,
        format!("field {name:?} is not {expected}"),
    )
}

fn no_parameter_failure() -> UnrecordedFailure {
    UnrecordedFailure::abi(
        RoupStatus::INVALID_ARGUMENT,
        ROUP_DIAGNOSTIC_INDEX_OUT_OF_RANGE,
        "directive has no parameter",
    )
}

fn parser_handle(raw: RoupParserHandle) -> UnrecordedResult<Handle<StoredObject>> {
    stored_handle(raw.index, raw.generation, "parser")
}

fn directive_handle(raw: RoupDirectiveHandle) -> UnrecordedResult<Handle<StoredObject>> {
    stored_handle(raw.index, raw.generation, "directive")
}

fn error_handle(raw: RoupErrorHandle) -> UnrecordedResult<Handle<StoredObject>> {
    stored_handle(raw.index, raw.generation, "error")
}

fn node_handle(raw: RoupNodeHandle) -> UnrecordedResult<Handle<StoredObject>> {
    stored_handle(raw.index, raw.generation, "node")
}

fn stored_handle(
    index: u64,
    generation: u64,
    name: &str,
) -> UnrecordedResult<Handle<StoredObject>> {
    let index = u32::try_from(index).map_err(|_| {
        UnrecordedFailure::abi(
            RoupStatus::INVALID_HANDLE,
            ROUP_DIAGNOSTIC_INVALID_HANDLE,
            format!("{name} handle index is outside the supported range"),
        )
    })?;
    Handle::from_raw_parts(index, generation).map_err(handle_failure)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StoredObjectKind {
    Parser,
    Directive,
    Node,
    Error,
}

impl StoredObjectKind {
    const fn name(self) -> &'static str {
        match self {
            Self::Parser => "parser",
            Self::Directive => "directive",
            Self::Node => "node",
            Self::Error => "error",
        }
    }

    const fn matches(self, object: &StoredObject) -> bool {
        matches!(
            (self, object),
            (Self::Parser, StoredObject::Parser(_))
                | (Self::Directive, StoredObject::Directive(_))
                | (Self::Node, StoredObject::Node(_))
                | (Self::Error, StoredObject::Error(_))
        )
    }
}

fn stored_object_kind_failure(expected: &str, actual: &StoredObject) -> UnrecordedFailure {
    UnrecordedFailure::abi(
        RoupStatus::INVALID_HANDLE,
        ROUP_DIAGNOSTIC_INVALID_HANDLE,
        format!(
            "{expected} handle identifies a stored {} object",
            actual.kind_name()
        ),
    )
}

fn remove_stored_object(
    state: &mut State,
    handle: Handle<StoredObject>,
    expected: StoredObjectKind,
) -> UnrecordedResult<()> {
    let object = state.objects.get(handle).map_err(handle_failure)?;
    if !expected.matches(object) {
        return Err(stored_object_kind_failure(expected.name(), object));
    }
    let removed = state.objects.remove(handle).map_err(handle_failure)?;
    if !expected.matches(&removed) {
        return Err(internal_failure(format!(
            "stored object changed kind while releasing a {} handle",
            expected.name()
        )));
    }
    Ok(())
}

fn handle_failure(error: HandleError) -> UnrecordedFailure {
    match error {
        HandleError::GenerationMismatch { .. } | HandleError::Vacant { .. } => {
            UnrecordedFailure::abi(
                RoupStatus::STALE_HANDLE,
                ROUP_DIAGNOSTIC_INVALID_HANDLE,
                error.to_string(),
            )
        }
        HandleError::ZeroGeneration | HandleError::IndexOutOfRange { .. } => {
            UnrecordedFailure::abi(
                RoupStatus::INVALID_HANDLE,
                ROUP_DIAGNOSTIC_INVALID_HANDLE,
                error.to_string(),
            )
        }
        HandleError::CapacityExhausted
        | HandleError::CorruptFreeList { .. }
        | HandleError::CorruptLiveCount => internal_failure(error.to_string()),
    }
}

fn arena_internal(error: HandleError) -> UnrecordedFailure {
    internal_failure(format!("internal handle allocation failed: {error}"))
}

fn internal_failure(message: impl Into<String>) -> UnrecordedFailure {
    UnrecordedFailure::abi(
        RoupStatus::INTERNAL_ERROR,
        ROUP_DIAGNOSTIC_INTERNAL_ERROR,
        message,
    )
}

fn index_failure(index: usize, count: usize) -> UnrecordedFailure {
    UnrecordedFailure::abi(
        RoupStatus::INVALID_ARGUMENT,
        ROUP_DIAGNOSTIC_INDEX_OUT_OF_RANGE,
        format!("clause index {index} is out of range for {count} clauses"),
    )
}

fn omp_clause(directive: &OmpDirective, index: usize) -> UnrecordedResult<&OmpClause> {
    directive
        .clauses()
        .get(index)
        .ok_or_else(|| index_failure(index, directive.clauses().len()))
}

fn acc_clause(directive: &AccDirective, index: usize) -> UnrecordedResult<&AccClause> {
    directive
        .clauses()
        .get(index)
        .ok_or_else(|| index_failure(index, directive.clauses().len()))
}

fn validate_options(options: RoupParserOptions) -> UnrecordedResult<ParserRecord> {
    if options.abi_version != ROUP_ABI_VERSION {
        return Err(config_error(format!(
            "ABI version {} is unsupported; expected {ROUP_ABI_VERSION}",
            options.abi_version
        )));
    }
    let expected_size = u32::try_from(core::mem::size_of::<RoupParserOptions>())
        .map_err(|_| internal_failure("parser option size exceeds u32"))?;
    if options.struct_size != expected_size {
        return Err(config_error(format!(
            "parser option size {} is invalid; expected {expected_size}",
            options.struct_size
        )));
    }

    let dialect = match options.dialect {
        ROUP_DIALECT_OPENMP => ROUP_DIALECT_OPENMP,
        ROUP_DIALECT_OPENACC => ROUP_DIALECT_OPENACC,
        value => return Err(config_error(format!("unknown dialect value {value}"))),
    };

    if !matches!(
        options.version_policy,
        ROUP_VERSION_ANY | ROUP_VERSION_EXACT
    ) {
        return Err(config_error(format!(
            "unknown version policy value {}",
            options.version_policy
        )));
    }
    if options.version_policy == ROUP_VERSION_ANY && options.version != 0 {
        return Err(config_error(
            "union-version policy requires the version field to be zero",
        ));
    }
    if options.version_policy == ROUP_VERSION_EXACT && options.version == 0 {
        return Err(config_error(
            "exact-version policy requires a nonzero version",
        ));
    }

    let host = match options.host_language {
        ROUP_HOST_C => HostLanguageProfile::C(match options.host_standard {
            89 => CStandard::C89,
            99 => CStandard::C99,
            11 => CStandard::C11,
            18 => CStandard::C18,
            23 => CStandard::C23,
            value => {
                return Err(config_error(format!(
                    "unknown C language standard value {value}"
                )))
            }
        }),
        ROUP_HOST_CPP => HostLanguageProfile::Cpp(match options.host_standard {
            98 => CppStandard::Cpp98,
            11 => CppStandard::Cpp11,
            14 => CppStandard::Cpp14,
            17 => CppStandard::Cpp17,
            20 => CppStandard::Cpp20,
            23 => CppStandard::Cpp23,
            value => {
                return Err(config_error(format!(
                    "unknown C++ language standard value {value}"
                )))
            }
        }),
        ROUP_HOST_FORTRAN => HostLanguageProfile::Fortran(match options.host_standard {
            77 => FortranStandard::Fortran77,
            90 => FortranStandard::Fortran90,
            95 => FortranStandard::Fortran95,
            2003 => FortranStandard::Fortran2003,
            2008 => FortranStandard::Fortran2008,
            2018 => FortranStandard::Fortran2018,
            2023 => FortranStandard::Fortran2023,
            value => {
                return Err(config_error(format!(
                    "unknown Fortran language standard value {value}"
                )))
            }
        }),
        value => return Err(config_error(format!("unknown host language value {value}"))),
    };

    let source_form = match options.source_form {
        ROUP_SOURCE_PRAGMA => SourceForm::Pragma,
        ROUP_SOURCE_FORTRAN_FREE => SourceForm::FortranFree,
        ROUP_SOURCE_FORTRAN_FIXED => SourceForm::FortranFixed,
        value => return Err(config_error(format!("unknown source form value {value}"))),
    };

    if options.flags != 0 {
        return Err(config_error(format!(
            "unsupported parser option flags {:#x}",
            options.flags
        )));
    }
    if let Some((index, value)) = options
        .reserved
        .iter()
        .copied()
        .enumerate()
        .find(|(_, value)| *value != 0)
    {
        return Err(config_error(format!(
            "reserved parser option field {index} must be zero, not {value}"
        )));
    }

    match dialect {
        ROUP_DIALECT_OPENMP => {
            let config = if options.version_policy == ROUP_VERSION_ANY {
                OpenMpConfig::new(host, source_form)
            } else {
                OpenMpConfig::exact(openmp_version(options.version)?, host, source_form)
            }
            .map_err(config_diagnostic)?;
            Ok(ParserRecord::OpenMp(config.parser()))
        }
        ROUP_DIALECT_OPENACC => {
            let config = if options.version_policy == ROUP_VERSION_ANY {
                OpenAccConfig::new(host, source_form)
            } else {
                OpenAccConfig::exact(openacc_version(options.version)?, host, source_form)
            }
            .map_err(config_diagnostic)?;
            Ok(ParserRecord::OpenAcc(config.parser()))
        }
        _ => Err(internal_failure("validated dialect became invalid")),
    }
}

fn openmp_version(value: u32) -> UnrecordedResult<OpenMpVersion> {
    match value {
        100 => Ok(OpenMpVersion::V1_0),
        110 => Ok(OpenMpVersion::V1_1),
        200 => Ok(OpenMpVersion::V2_0),
        250 => Ok(OpenMpVersion::V2_5),
        300 => Ok(OpenMpVersion::V3_0),
        310 => Ok(OpenMpVersion::V3_1),
        400 => Ok(OpenMpVersion::V4_0),
        450 => Ok(OpenMpVersion::V4_5),
        500 => Ok(OpenMpVersion::V5_0),
        510 => Ok(OpenMpVersion::V5_1),
        520 => Ok(OpenMpVersion::V5_2),
        600 => Ok(OpenMpVersion::V6_0),
        _ => Err(config_error(format!(
            "unknown OpenMP version encoding {value}"
        ))),
    }
}

fn openacc_version(value: u32) -> UnrecordedResult<OpenAccVersion> {
    match value {
        100 => Ok(OpenAccVersion::V1_0),
        200 => Ok(OpenAccVersion::V2_0),
        250 => Ok(OpenAccVersion::V2_5),
        260 => Ok(OpenAccVersion::V2_6),
        270 => Ok(OpenAccVersion::V2_7),
        300 => Ok(OpenAccVersion::V3_0),
        310 => Ok(OpenAccVersion::V3_1),
        320 => Ok(OpenAccVersion::V3_2),
        330 => Ok(OpenAccVersion::V3_3),
        340 => Ok(OpenAccVersion::V3_4),
        _ => Err(config_error(format!(
            "unknown OpenACC version encoding {value}"
        ))),
    }
}

fn config_error(message: impl Into<String>) -> UnrecordedFailure {
    let diagnostic = Diagnostic::new(
        DiagnosticCode::InvalidConfiguration,
        roup::source::Span::start_of(""),
        message.into(),
    );
    UnrecordedFailure::diagnostic(RoupStatus::INVALID_ARGUMENT, diagnostic)
}

fn config_diagnostic(diagnostic: Diagnostic) -> UnrecordedFailure {
    UnrecordedFailure::diagnostic(RoupStatus::INVALID_ARGUMENT, diagnostic)
}

fn version_bits<V: DirectiveVersion>(versions: VersionSet<V>) -> u64 {
    versions
        .iter()
        .fold(0_u64, |bits, version| bits | (1_u64 << version.ordinal()))
}

fn parameter_fields(body: &RoupDirective) -> UnrecordedResult<Vec<ClauseField>> {
    let mut fields = Vec::new();
    match body {
        RoupDirective::OpenMp(directive) => {
            let parameter = directive.parameter().ok_or_else(no_parameter_failure)?;
            match parameter {
                roup::ast::OmpDirectiveParameter::AllocateList(values)
                | roup::ast::OmpDirectiveParameter::ThreadprivateList(values)
                | roup::ast::OmpDirectiveParameter::GroupprivateList(values) => {
                    fields.push(ClauseField::nodes(
                        crate::ROUP_FIELD_ITEMS,
                        "items",
                        values.iter().map(omp_storage_item_node).collect(),
                    ))
                }
                roup::ast::OmpDirectiveParameter::DeclareTargetList(values) => {
                    fields.push(ClauseField::nodes(
                        crate::ROUP_FIELD_ITEMS,
                        "items",
                        values.iter().map(omp_declare_target_item_node).collect(),
                    ))
                }
                roup::ast::OmpDirectiveParameter::FlushList(values) => {
                    fields.push(ClauseField::nodes(
                        crate::ROUP_FIELD_ITEMS,
                        "items",
                        values.iter().map(omp_flush_item_node).collect(),
                    ))
                }
                roup::ast::OmpDirectiveParameter::Depobj(value) => {
                    fields.push(ClauseField::string(crate::ROUP_FIELD_VALUE, "value", value))
                }
                roup::ast::OmpDirectiveParameter::CriticalSection(value) => {
                    fields.push(ClauseField::string(crate::ROUP_FIELD_VALUE, "value", value))
                }
                roup::ast::OmpDirectiveParameter::DeclareMapper(mapper) => {
                    if let Some(identifier) = mapper.identifier() {
                        fields.push(ClauseField::node(
                            crate::ROUP_FIELD_MAPPER,
                            "mapper",
                            omp_mapper_id_node(identifier),
                        ));
                    }
                    fields.push(ClauseField::string(
                        crate::ROUP_FIELD_TYPE_NAME,
                        "type_name",
                        mapper.type_name(),
                    ));
                    fields.push(ClauseField::string(
                        crate::ROUP_FIELD_VARIABLE,
                        "variable",
                        mapper.variable(),
                    ));
                }
                roup::ast::OmpDirectiveParameter::DeclareVariant(target) => {
                    push_optional_string(
                        &mut fields,
                        crate::ROUP_FIELD_BASE,
                        "base",
                        target.base(),
                    );
                    fields.push(ClauseField::node(
                        crate::ROUP_FIELD_FUNCTION,
                        "function",
                        omp_function_name_node(target.variant()),
                    ));
                }
                roup::ast::OmpDirectiveParameter::Construct(construct) => {
                    fields.push(ClauseField::u32(
                        crate::ROUP_FIELD_KIND,
                        "kind",
                        match construct {
                            roup::ast::OmpConstructType::Parallel => {
                                crate::ROUP_OMP_CONSTRUCT_PARALLEL
                            }
                            roup::ast::OmpConstructType::Sections => {
                                crate::ROUP_OMP_CONSTRUCT_SECTIONS
                            }
                            roup::ast::OmpConstructType::For => crate::ROUP_OMP_CONSTRUCT_FOR,
                            roup::ast::OmpConstructType::Taskgroup => {
                                crate::ROUP_OMP_CONSTRUCT_TASKGROUP
                            }
                        },
                    ))
                }
                roup::ast::OmpDirectiveParameter::DeclareReduction(reduction) => {
                    fields.push(ClauseField::node(
                        crate::ROUP_FIELD_NAME,
                        "identifier",
                        omp_reduction_identifier_node(reduction.identifier()),
                    ));
                    fields.push(ClauseField::strings(
                        crate::ROUP_FIELD_VALUES,
                        "type_names",
                        reduction.type_names(),
                    ));
                    fields.push(ClauseField::node(
                        crate::ROUP_FIELD_COMBINER,
                        "combiner",
                        omp_reduction_combiner_node(reduction.combiner()),
                    ));
                    if let Some(initializer) = reduction.initializer() {
                        fields.push(ClauseField::node(
                            crate::ROUP_FIELD_INITIALIZER,
                            "initializer",
                            omp_reduction_initializer_node(initializer),
                        ));
                    }
                }
                roup::ast::OmpDirectiveParameter::DeclareInduction(induction) => {
                    fields.push(ClauseField::node(
                        crate::ROUP_FIELD_NAME,
                        "identifier",
                        omp_induction_identifier_node(induction.identifier()),
                    ));
                    fields.push(ClauseField::nodes(
                        crate::ROUP_FIELD_TYPE_SPECIFIERS,
                        "type_specifiers",
                        induction
                            .type_specifiers()
                            .iter()
                            .map(induction_type_specifier_node)
                            .collect(),
                    ));
                }
                roup::ast::OmpDirectiveParameter::DeclareSimd(target) => {
                    fields.push(ClauseField::string(
                        crate::ROUP_FIELD_FUNCTION,
                        "function",
                        target.function(),
                    ));
                }
            }
        }
        RoupDirective::OpenAcc(directive) => {
            let parameter = directive.parameter().ok_or_else(no_parameter_failure)?;
            match parameter {
                roup::ast::AccDirectiveParameter::Cache(cache) => {
                    fields.push(ClauseField::boolean(
                        crate::ROUP_FIELD_READONLY,
                        "readonly",
                        cache.readonly(),
                    ));
                    fields.push(ClauseField::nodes(
                        crate::ROUP_FIELD_ITEMS,
                        "items",
                        cache.items().iter().map(acc_cache_item_node).collect(),
                    ));
                }
                roup::ast::AccDirectiveParameter::Wait(wait) => {
                    push_optional_string(
                        &mut fields,
                        crate::ROUP_FIELD_DEVICE_NUM,
                        "device_num",
                        wait.devnum(),
                    );
                    fields.push(ClauseField::strings(
                        crate::ROUP_FIELD_VALUES,
                        "queues",
                        wait.queues(),
                    ));
                }
                roup::ast::AccDirectiveParameter::Routine(routine) => fields.push(
                    ClauseField::string(crate::ROUP_FIELD_FUNCTION, "function", routine.name()),
                ),
                roup::ast::AccDirectiveParameter::End(kind) => fields.push(ClauseField::node(
                    crate::ROUP_FIELD_KIND,
                    "kind",
                    acc_end_kind_node(*kind),
                )),
            }
        }
    }
    Ok(fields)
}

fn omp_fields(payload: &roup::ir::ClauseData) -> UnrecordedResult<Vec<ClauseField>> {
    use roup::ir::ClauseData;

    let mut fields = Vec::new();
    match payload {
        ClauseData::Bare => {}
        ClauseData::Nowait { do_not_synchronize } | ClauseData::Nogroup { do_not_synchronize } => {
            push_optional_string(
                &mut fields,
                crate::ROUP_FIELD_DO_NOT_SYNCHRONIZE,
                "do_not_synchronize",
                do_not_synchronize.as_ref(),
            )
        }
        ClauseData::Align { alignment } => fields.push(ClauseField::string(
            crate::ROUP_FIELD_ALIGNMENT,
            "alignment",
            alignment,
        )),
        ClauseData::Destroy { variable } => push_optional_string(
            &mut fields,
            crate::ROUP_FIELD_VARIABLE,
            "variable",
            variable.as_ref(),
        ),
        ClauseData::Final { condition }
        | ClauseData::Holds { condition }
        | ClauseData::Nocontext { condition }
        | ClauseData::Novariants { condition } => fields.push(ClauseField::string(
            crate::ROUP_FIELD_CONDITION,
            "condition",
            condition,
        )),
        ClauseData::GraphId { value }
        | ClauseData::Hint { value }
        | ClauseData::Message { value } => {
            fields.push(ClauseField::string(crate::ROUP_FIELD_VALUE, "value", value))
        }
        ClauseData::Threadset(kind) => fields.push(ClauseField::u32(
            crate::ROUP_FIELD_KIND,
            "threadset_kind",
            omp_threadset_kind(*kind),
        )),
        ClauseData::Memscope(kind) => fields.push(ClauseField::u32(
            crate::ROUP_FIELD_KIND,
            "memscope_kind",
            omp_memscope_kind(*kind),
        )),
        ClauseData::Looprange { first, count } => {
            fields.push(ClauseField::string(crate::ROUP_FIELD_FIRST, "first", first));
            fields.push(ClauseField::string(crate::ROUP_FIELD_COUNT, "count", count));
        }
        ClauseData::GraphReset { condition } => push_optional_string(
            &mut fields,
            crate::ROUP_FIELD_CONDITION,
            "condition",
            condition.as_ref(),
        ),
        ClauseData::ItemList(items) => fields.push(clause_items_field(items)),
        ClauseData::Sizes { sizes } => fields.push(ClauseField::strings(
            crate::ROUP_FIELD_VALUES,
            "sizes",
            sizes,
        )),
        ClauseData::Permutation { positions } => fields.push(ClauseField::strings(
            crate::ROUP_FIELD_VALUES,
            "positions",
            positions,
        )),
        ClauseData::Counts { counts } => fields.push(ClauseField::nodes(
            crate::ROUP_FIELD_VALUES,
            "counts",
            counts.iter().map(omp_count_node).collect(),
        )),
        ClauseData::Uniform { parameters } => fields.push(ClauseField::strings(
            crate::ROUP_FIELD_ARGUMENTS,
            "parameters",
            parameters,
        )),
        ClauseData::Use { interop_var } => fields.push(ClauseField::string(
            crate::ROUP_FIELD_VARIABLE,
            "interop_var",
            interop_var,
        )),
        ClauseData::Enter { automap, items } => {
            fields.push(ClauseField::u32s(
                crate::ROUP_FIELD_MODIFIERS,
                "modifiers",
                if *automap {
                    vec![crate::ROUP_OMP_ENTER_AUTOMAP]
                } else {
                    Vec::new()
                },
            ));
            fields.push(clause_items_field(items));
        }
        ClauseData::To {
            present,
            mapper,
            iterators,
            locators,
        }
        | ClauseData::From {
            present,
            mapper,
            iterators,
            locators,
        } => {
            fields.push(ClauseField::u32s(
                crate::ROUP_FIELD_MODIFIERS,
                "modifiers",
                if *present {
                    vec![crate::ROUP_OMP_DATA_MOTION_PRESENT]
                } else {
                    Vec::new()
                },
            ));
            if let Some(mapper) = mapper {
                fields.push(ClauseField::node(
                    crate::ROUP_FIELD_MAPPER,
                    "mapper",
                    omp_mapper_id_node(mapper),
                ));
            }
            fields.push(ClauseField::nodes(
                crate::ROUP_FIELD_ITERATORS,
                "iterators",
                iterators.iter().map(depend_iterator_node).collect(),
            ));
            fields.push(ClauseField::nodes(
                crate::ROUP_FIELD_ITEMS,
                "locators",
                locators.iter().map(omp_locator_node).collect(),
            ));
        }
        ClauseData::Scan { mode, items } => {
            fields.push(ClauseField::u32(
                crate::ROUP_FIELD_KIND,
                "kind",
                omp_scan_mode(*mode),
            ));
            fields.push(clause_items_field(items));
        }
        ClauseData::InitComplete { create_init_phase } => push_optional_string(
            &mut fields,
            crate::ROUP_FIELD_CONDITION,
            "create_init_phase",
            create_init_phase.as_ref(),
        ),
        ClauseData::Branch { condition } => push_optional_string(
            &mut fields,
            crate::ROUP_FIELD_CONDITION,
            "condition",
            condition.as_ref(),
        ),
        ClauseData::Full { fully_unroll } => push_optional_string(
            &mut fields,
            crate::ROUP_FIELD_FULLY_UNROLL,
            "fully_unroll",
            fully_unroll.as_ref(),
        ),
        ClauseData::Partial { unroll_factor } => push_optional_string(
            &mut fields,
            crate::ROUP_FIELD_UNROLL_FACTOR,
            "unroll_factor",
            unroll_factor.as_ref(),
        ),
        ClauseData::Mergeable { can_merge } => push_optional_string(
            &mut fields,
            crate::ROUP_FIELD_CAN_MERGE,
            "can_merge",
            can_merge.as_ref(),
        ),
        ClauseData::Untied { can_change_threads } => push_optional_string(
            &mut fields,
            crate::ROUP_FIELD_CAN_CHANGE_THREADS,
            "can_change_threads",
            can_change_threads.as_ref(),
        ),
        ClauseData::Simd { apply_to_simd } => push_optional_string(
            &mut fields,
            crate::ROUP_FIELD_APPLY_TO_SIMD,
            "apply_to_simd",
            apply_to_simd.as_ref(),
        ),
        ClauseData::Threads { apply_to_threads } => push_optional_string(
            &mut fields,
            crate::ROUP_FIELD_APPLY_TO_THREADS,
            "apply_to_threads",
            apply_to_threads.as_ref(),
        ),
        ClauseData::Assumption { can_assume } => push_optional_string(
            &mut fields,
            crate::ROUP_FIELD_CAN_ASSUME,
            "can_assume",
            can_assume.as_ref(),
        ),
        ClauseData::Indirect { invoked_by_fptr } => push_optional_string(
            &mut fields,
            crate::ROUP_FIELD_INVOKED_BY_FPTR,
            "invoked_by_fptr",
            invoked_by_fptr.as_ref(),
        ),
        ClauseData::Replayable {
            replayable_expression,
        } => push_optional_string(
            &mut fields,
            crate::ROUP_FIELD_REPLAYABLE_EXPRESSION,
            "replayable_expression",
            replayable_expression.as_ref(),
        ),
        ClauseData::Safesync { width } => push_optional_string(
            &mut fields,
            crate::ROUP_FIELD_WIDTH,
            "width",
            width.as_ref(),
        ),
        ClauseData::Transparent { impex_type } => push_optional_string(
            &mut fields,
            crate::ROUP_FIELD_IMPEX_TYPE,
            "impex_type",
            impex_type.as_ref(),
        ),
        ClauseData::Detach { event } => {
            fields.push(ClauseField::string(crate::ROUP_FIELD_EVENT, "event", event))
        }
        ClauseData::Absent { directives } | ClauseData::Contains { directives } => {
            fields.push(ClauseField::u32s(
                crate::ROUP_FIELD_DIRECTIVES,
                "directives",
                directives
                    .iter()
                    .copied()
                    .map(omp_directive_kind_ordinal)
                    .collect(),
            ));
        }
        ClauseData::AdjustArgs {
            operation,
            parameters,
        } => {
            fields.push(ClauseField::u32(
                crate::ROUP_FIELD_OPERATION,
                "operation",
                omp_adjust_args_modifier(*operation),
            ));
            fields.push(ClauseField::nodes(
                crate::ROUP_FIELD_PARAMETERS,
                "parameters",
                parameters
                    .iter()
                    .map(omp_parameter_list_item_node)
                    .collect(),
            ));
        }
        ClauseData::AppendArgs { operations } => fields.push(ClauseField::nodes(
            crate::ROUP_FIELD_OPERATIONS,
            "operations",
            operations.iter().map(omp_append_operation_node).collect(),
        )),
        ClauseData::Collector { expression } => {
            fields.push(ClauseField::string(
                crate::ROUP_FIELD_VALUE,
                "value",
                expression,
            ));
        }
        ClauseData::Inductor { expression } => {
            fields.push(ClauseField::node(
                crate::ROUP_FIELD_VALUE,
                "value",
                omp_inductor_expression_node(expression),
            ));
        }
        ClauseData::Apply {
            loop_modifier,
            applied_directives,
        } => {
            if let Some(modifier) = loop_modifier {
                fields.push(ClauseField::node(
                    crate::ROUP_FIELD_LOOP_MODIFIER,
                    "loop_modifier",
                    omp_apply_modifier_node(modifier),
                ));
            }
            fields.push(ClauseField::nodes(
                crate::ROUP_FIELD_APPLIED_DIRECTIVES,
                "applied_directives",
                applied_directives
                    .iter()
                    .map(omp_directive_node)
                    .collect::<UnrecordedResult<Vec<_>>>()?,
            ));
        }
        ClauseData::Induction {
            modifier,
            step,
            identifier,
            items,
        } => {
            if let Some(modifier) = modifier {
                fields.push(ClauseField::u32(
                    crate::ROUP_FIELD_MODIFIER,
                    "modifier",
                    match modifier {
                        roup::ir::OmpInductionModifier::Relaxed => {
                            crate::ROUP_OMP_INDUCTION_RELAXED
                        }
                        roup::ir::OmpInductionModifier::Strict => crate::ROUP_OMP_INDUCTION_STRICT,
                    },
                ));
            }
            fields.push(ClauseField::string(crate::ROUP_FIELD_STEP, "step", step));
            fields.push(ClauseField::node(
                crate::ROUP_FIELD_IDENTIFIER,
                "identifier",
                omp_induction_identifier_node(identifier),
            ));
            fields.push(clause_items_field(items));
        }
        ClauseData::Private { items }
        | ClauseData::Shared { items }
        | ClauseData::UseDevicePtr { items }
        | ClauseData::UseDeviceAddr { items }
        | ClauseData::IsDevicePtr { items }
        | ClauseData::HasDeviceAddr { items }
        | ClauseData::Copyin { items }
        | ClauseData::Copyprivate { items } => fields.push(clause_items_field(items)),
        ClauseData::Firstprivate { modifier, items } => {
            if let Some(modifier) = modifier {
                fields.push(ClauseField::u32(
                    crate::ROUP_FIELD_MODIFIER,
                    "modifier",
                    omp_firstprivate_modifier(*modifier),
                ));
            }
            fields.push(clause_items_field(items));
        }
        ClauseData::Lastprivate { modifier, items } => {
            if let Some(modifier) = modifier {
                fields.push(ClauseField::u32(
                    crate::ROUP_FIELD_MODIFIER,
                    "modifier",
                    omp_lastprivate_modifier(*modifier),
                ));
            }
            fields.push(clause_items_field(items));
        }
        ClauseData::Default { category, kind } => {
            fields.push(ClauseField::u32(
                crate::ROUP_FIELD_KIND,
                "kind",
                omp_default_kind(*kind),
            ));
            if let Some(category) = category {
                fields.push(ClauseField::u32(
                    crate::ROUP_FIELD_CATEGORY,
                    "category",
                    omp_defaultmap_category(*category),
                ));
            }
        }
        ClauseData::Defaultmap { behavior, category } => {
            fields.push(ClauseField::u32(
                crate::ROUP_FIELD_BEHAVIOR,
                "behavior",
                omp_defaultmap_behavior(*behavior),
            ));
            if let Some(category) = category {
                fields.push(ClauseField::u32(
                    crate::ROUP_FIELD_CATEGORY,
                    "category",
                    omp_defaultmap_category(*category),
                ));
            }
        }
        ClauseData::Reduction {
            modifiers,
            operator,
            items,
        } => {
            fields.push(ClauseField::nodes(
                crate::ROUP_FIELD_MODIFIERS,
                "modifiers",
                modifiers.iter().map(reduction_modifier_node).collect(),
            ));
            fields.push(ClauseField::node(
                crate::ROUP_FIELD_OPERATOR,
                "operator",
                omp_reduction_identifier_node(operator),
            ));
            fields.push(clause_items_field(items));
        }
        ClauseData::Map {
            map_type,
            map_type_spelling: _,
            modifiers,
            mapper,
            iterators,
            locators,
        } => {
            if let Some(map_type) = map_type {
                fields.push(ClauseField::u32(
                    crate::ROUP_FIELD_KIND,
                    "map_type",
                    omp_map_type(*map_type),
                ));
            }
            fields.push(ClauseField::u32s(
                crate::ROUP_FIELD_MODIFIERS,
                "modifiers",
                modifiers.iter().copied().map(omp_map_modifier).collect(),
            ));
            if let Some(mapper) = mapper {
                fields.push(ClauseField::node(
                    crate::ROUP_FIELD_MAPPER,
                    "mapper",
                    omp_mapper_id_node(mapper),
                ));
            }
            fields.push(ClauseField::nodes(
                crate::ROUP_FIELD_ITERATORS,
                "iterators",
                iterators.iter().map(depend_iterator_node).collect(),
            ));
            fields.push(ClauseField::nodes(
                crate::ROUP_FIELD_ITEMS,
                "locators",
                locators.iter().map(omp_locator_node).collect(),
            ));
        }
        ClauseData::Depend {
            dependence,
            iterators,
        } => {
            fields.push(ClauseField::nodes(
                crate::ROUP_FIELD_ITERATORS,
                "iterators",
                iterators.iter().map(depend_iterator_node).collect(),
            ));
            fields.push(ClauseField::node(
                crate::ROUP_FIELD_DEPENDENCE,
                "dependence",
                omp_dependence_node(dependence),
            ));
        }
        ClauseData::Doacross { kind, iteration } => {
            fields.push(ClauseField::u32(
                crate::ROUP_FIELD_KIND,
                "kind",
                omp_doacross_type(*kind),
            ));
            fields.push(ClauseField::node(
                crate::ROUP_FIELD_ITERATION,
                "iteration",
                omp_doacross_iteration_node(iteration),
            ));
        }
        ClauseData::Priority { priority } => fields.push(ClauseField::string(
            crate::ROUP_FIELD_VALUE,
            "priority",
            priority,
        )),
        ClauseData::Affinity {
            iterators,
            locators,
        } => {
            if !iterators.is_empty() {
                fields.push(ClauseField::u32(
                    crate::ROUP_FIELD_MODIFIER,
                    "modifier",
                    crate::ROUP_OMP_AFFINITY_ITERATOR,
                ));
                fields.push(ClauseField::nodes(
                    crate::ROUP_FIELD_ITERATORS,
                    "iterators",
                    iterators.iter().map(depend_iterator_node).collect(),
                ));
            }
            fields.push(ClauseField::nodes(
                crate::ROUP_FIELD_ITEMS,
                "locators",
                locators.iter().map(omp_locator_node).collect(),
            ));
        }
        ClauseData::Schedule {
            kind,
            modifiers,
            chunk_size,
        } => {
            fields.push(ClauseField::u32(
                crate::ROUP_FIELD_KIND,
                "kind",
                omp_schedule_kind(*kind),
            ));
            fields.push(ClauseField::u32s(
                crate::ROUP_FIELD_MODIFIERS,
                "modifiers",
                modifiers
                    .iter()
                    .copied()
                    .map(omp_schedule_modifier)
                    .collect(),
            ));
            push_optional_string(
                &mut fields,
                crate::ROUP_FIELD_CHUNK_SIZE,
                "chunk_size",
                chunk_size.as_ref(),
            );
        }
        ClauseData::Collapse { n } => {
            fields.push(ClauseField::string(crate::ROUP_FIELD_VALUE, "count", n))
        }
        ClauseData::Ordered { n } => {
            push_optional_string(&mut fields, crate::ROUP_FIELD_VALUE, "count", n.as_ref())
        }
        ClauseData::Linear {
            modifier,
            items,
            step,
            source_syntax,
        } => {
            if let Some(modifier) = modifier {
                fields.push(ClauseField::u32(
                    crate::ROUP_FIELD_MODIFIER,
                    "modifier",
                    omp_linear_modifier(*modifier),
                ));
            }
            fields.push(clause_items_field(items));
            push_optional_string(&mut fields, crate::ROUP_FIELD_STEP, "step", step.as_ref());
            fields.push(ClauseField::u32(
                crate::ROUP_FIELD_SOURCE_SYNTAX,
                "source_syntax",
                match source_syntax {
                    roup::ir::LinearSourceSyntax::Historical => {
                        crate::ROUP_OMP_LINEAR_SOURCE_HISTORICAL
                    }
                    roup::ir::LinearSourceSyntax::ModifierPrefix => {
                        crate::ROUP_OMP_LINEAR_SOURCE_MODIFIER_PREFIX
                    }
                    roup::ir::LinearSourceSyntax::CanonicalModifiers => {
                        crate::ROUP_OMP_LINEAR_SOURCE_CANONICAL_MODIFIERS
                    }
                },
            ));
        }
        ClauseData::Aligned { items, alignment } => {
            fields.push(clause_items_field(items));
            push_optional_string(
                &mut fields,
                crate::ROUP_FIELD_ALIGNMENT,
                "alignment",
                alignment.as_ref(),
            );
        }
        ClauseData::Safelen { length } | ClauseData::Simdlen { length } => fields.push(
            ClauseField::string(crate::ROUP_FIELD_VALUE, "length", length),
        ),
        ClauseData::If { condition } => {
            fields.push(ClauseField::string(
                crate::ROUP_FIELD_CONDITION,
                "condition",
                condition,
            ));
        }
        ClauseData::ProcBind(kind) => fields.push(ClauseField::u32(
            crate::ROUP_FIELD_KIND,
            "kind",
            omp_proc_bind(*kind),
        )),
        ClauseData::Bind(modifier) => fields.push(ClauseField::u32(
            crate::ROUP_FIELD_MODIFIER,
            "modifier",
            omp_bind_modifier(*modifier),
        )),
        ClauseData::NumThreads { strict, nthreads } => {
            fields.push(ClauseField::u32s(
                crate::ROUP_FIELD_MODIFIERS,
                "modifiers",
                if *strict {
                    vec![crate::ROUP_OMP_NUM_THREADS_STRICT]
                } else {
                    Vec::new()
                },
            ));
            fields.push(ClauseField::strings(
                crate::ROUP_FIELD_VALUES,
                "nthreads",
                nthreads,
            ));
        }
        ClauseData::NumTeams {
            lower_bound,
            upper_bound,
        } => {
            push_optional_string(
                &mut fields,
                crate::ROUP_FIELD_LOWER_BOUND,
                "lower_bound",
                lower_bound.as_ref(),
            );
            fields.push(ClauseField::string(
                crate::ROUP_FIELD_UPPER_BOUND,
                "upper_bound",
                upper_bound,
            ));
        }
        ClauseData::Device {
            modifier,
            device_num,
        } => {
            if let Some(modifier) = modifier {
                fields.push(ClauseField::u32(
                    crate::ROUP_FIELD_MODIFIER,
                    "modifier",
                    omp_device_modifier(*modifier),
                ));
            }
            fields.push(ClauseField::string(
                crate::ROUP_FIELD_DEVICE_NUM,
                "device_num",
                device_num,
            ));
        }
        ClauseData::DeviceType(kind) => fields.push(ClauseField::u32(
            crate::ROUP_FIELD_KIND,
            "kind",
            omp_device_type(*kind),
        )),
        ClauseData::At(kind) => fields.push(ClauseField::u32(
            crate::ROUP_FIELD_KIND,
            "kind",
            omp_at_kind(*kind),
        )),
        ClauseData::Severity(kind) => fields.push(ClauseField::u32(
            crate::ROUP_FIELD_KIND,
            "kind",
            omp_severity_kind(*kind),
        )),
        ClauseData::InitInterop {
            interop_types,
            preferences,
            variable,
        } => {
            fields.push(ClauseField::u32s(
                crate::ROUP_FIELD_INTEROP_TYPES,
                "interop_types",
                interop_types
                    .iter()
                    .copied()
                    .map(omp_interop_type)
                    .collect(),
            ));
            fields.push(ClauseField::nodes(
                crate::ROUP_FIELD_PREFERENCES,
                "preferences",
                preferences
                    .iter()
                    .map(omp_preference_specification_node)
                    .collect(),
            ));
            fields.push(ClauseField::string(
                crate::ROUP_FIELD_VARIABLE,
                "variable",
                variable,
            ));
        }
        ClauseData::InitDepobj {
            dependence,
            locator,
            variable,
        } => {
            fields.push(ClauseField::u32(
                crate::ROUP_FIELD_DEPEND_TYPE,
                "dependence",
                omp_depobj_update(*dependence),
            ));
            fields.push(ClauseField::node(
                crate::ROUP_FIELD_LOCATOR,
                "locator",
                omp_locator_node(locator),
            ));
            fields.push(ClauseField::string(
                crate::ROUP_FIELD_VARIABLE,
                "variable",
                variable,
            ));
        }
        ClauseData::Fail { order } => fields.push(ClauseField::u32(
            crate::ROUP_FIELD_MEMORY_ORDER,
            "memory_order",
            omp_memory_order(*order),
        )),
        ClauseData::MemoryOrder {
            order,
            use_semantics,
        } => {
            fields.push(ClauseField::u32(
                crate::ROUP_FIELD_MEMORY_ORDER,
                "memory_order",
                omp_memory_order(*order),
            ));
            push_optional_string(
                &mut fields,
                crate::ROUP_FIELD_USE_SEMANTICS,
                "use_semantics",
                use_semantics.as_ref(),
            );
        }
        ClauseData::AtomicOperation { op, use_semantics } => {
            fields.push(ClauseField::u32(
                crate::ROUP_FIELD_KIND,
                "operation",
                omp_atomic_operation(*op),
            ));
            push_optional_string(
                &mut fields,
                crate::ROUP_FIELD_USE_SEMANTICS,
                "use_semantics",
                use_semantics.as_ref(),
            );
        }
        ClauseData::ExtendedAtomic {
            kind,
            use_semantics,
        } => {
            fields.push(ClauseField::u32(
                crate::ROUP_FIELD_KIND,
                "extended_atomic_kind",
                match kind {
                    roup::ir::ExtendedAtomicKind::Capture => {
                        crate::ROUP_OMP_EXTENDED_ATOMIC_CAPTURE
                    }
                    roup::ir::ExtendedAtomicKind::Compare => {
                        crate::ROUP_OMP_EXTENDED_ATOMIC_COMPARE
                    }
                    roup::ir::ExtendedAtomicKind::Weak => crate::ROUP_OMP_EXTENDED_ATOMIC_WEAK,
                },
            ));
            push_optional_string(
                &mut fields,
                crate::ROUP_FIELD_USE_SEMANTICS,
                "use_semantics",
                use_semantics.as_ref(),
            );
        }
        ClauseData::Order { modifier, kind } => {
            if let Some(modifier) = modifier {
                fields.push(ClauseField::u32(
                    crate::ROUP_FIELD_MODIFIER,
                    "modifier",
                    omp_order_modifier(*modifier),
                ));
            }
            fields.push(ClauseField::u32(
                crate::ROUP_FIELD_KIND,
                "kind",
                omp_order_kind(*kind),
            ));
        }
        ClauseData::ThreadLimit { limit } => {
            fields.push(ClauseField::string(crate::ROUP_FIELD_VALUE, "limit", limit))
        }
        ClauseData::Allocate {
            allocator,
            alignment,
            items,
            source_syntax,
        } => {
            push_optional_string(
                &mut fields,
                crate::ROUP_FIELD_ALLOCATOR_EXPRESSION,
                "allocator_expression",
                allocator.as_ref(),
            );
            push_optional_string(
                &mut fields,
                crate::ROUP_FIELD_ALIGNMENT_EXPRESSION,
                "alignment_expression",
                alignment.as_ref(),
            );
            fields.push(clause_items_field(items));
            fields.push(ClauseField::u32(
                crate::ROUP_FIELD_ALLOCATE_SOURCE_SYNTAX,
                "source_syntax",
                omp_allocate_source_syntax(*source_syntax),
            ));
        }
        ClauseData::Allocator { allocator } => fields.push(ClauseField::string(
            crate::ROUP_FIELD_ALLOCATOR_EXPRESSION,
            "allocator_expression",
            allocator,
        )),
        ClauseData::DistSchedule { kind, chunk_size } => {
            fields.push(ClauseField::u32(
                crate::ROUP_FIELD_KIND,
                "kind",
                omp_schedule_kind(*kind),
            ));
            push_optional_string(
                &mut fields,
                crate::ROUP_FIELD_CHUNK_SIZE,
                "chunk_size",
                chunk_size.as_ref(),
            );
        }
        ClauseData::Grainsize { modifier, grain } => {
            if modifier.is_some() {
                fields.push(ClauseField::u32(
                    crate::ROUP_FIELD_MODIFIER,
                    "modifier",
                    crate::ROUP_OMP_GRAINSIZE_STRICT,
                ));
            }
            fields.push(ClauseField::string(crate::ROUP_FIELD_VALUE, "grain", grain));
        }
        ClauseData::NumTasks { modifier, num } => {
            if modifier.is_some() {
                fields.push(ClauseField::u32(
                    crate::ROUP_FIELD_MODIFIER,
                    "modifier",
                    crate::ROUP_OMP_NUM_TASKS_STRICT,
                ));
            }
            fields.push(ClauseField::string(crate::ROUP_FIELD_VALUE, "value", num));
        }
        ClauseData::Filter { thread_num } => fields.push(ClauseField::string(
            crate::ROUP_FIELD_VALUE,
            "thread_num",
            thread_num,
        )),
        ClauseData::UsesAllocators { allocators } => fields.push(ClauseField::nodes(
            crate::ROUP_FIELD_ALLOCATORS,
            "allocators",
            allocators
                .iter()
                .map(uses_allocator_node)
                .collect::<UnrecordedResult<Vec<_>>>()?,
        )),
        ClauseData::Requirement {
            requirement,
            required,
        } => {
            fields.push(ClauseField::node(
                crate::ROUP_FIELD_REQUIREMENTS,
                "requirement",
                require_modifier_node(requirement)?,
            ));
            push_optional_string(
                &mut fields,
                crate::ROUP_FIELD_REQUIRED,
                "required",
                required.as_ref(),
            );
        }
        ClauseData::DepobjUpdate {
            dependence,
            variable,
        } => {
            fields.push(ClauseField::u32(
                crate::ROUP_FIELD_DEPEND_TYPE,
                "dependence",
                omp_depobj_update(*dependence),
            ));
            push_optional_string(
                &mut fields,
                crate::ROUP_FIELD_VARIABLE,
                "variable",
                variable.as_ref(),
            );
        }
        ClauseData::MetadirectiveSelector { selector } => {
            fields.extend(selector_fields(selector)?);
        }
    }
    Ok(fields)
}

fn acc_fields(payload: &AccClausePayload) -> UnrecordedResult<Vec<ClauseField>> {
    let mut fields = Vec::new();
    match payload {
        AccClausePayload::Bare => {}
        AccClausePayload::Expression(value) => {
            fields.push(ClauseField::string(crate::ROUP_FIELD_VALUE, "value", value))
        }
        AccClausePayload::NumGangs(values) => fields.push(ClauseField::strings(
            crate::ROUP_FIELD_VALUES,
            "values",
            values,
        )),
        AccClausePayload::Tile(sizes) => fields.push(ClauseField::nodes(
            crate::ROUP_FIELD_VALUES,
            "sizes",
            sizes.iter().map(acc_size_expression_node).collect(),
        )),
        AccClausePayload::ItemList(items) => fields.push(clause_items_field(items)),
        AccClausePayload::Bind(target) => fields.push(ClauseField::node(
            crate::ROUP_FIELD_VALUE,
            "target",
            acc_bind_target_node(target),
        )),
        AccClausePayload::Collapse(collapse) => {
            fields.push(ClauseField::boolean(
                crate::ROUP_FIELD_FORCE,
                "force",
                collapse.force(),
            ));
            fields.push(ClauseField::string(
                crate::ROUP_FIELD_VALUE,
                "count",
                collapse.count(),
            ));
        }
        AccClausePayload::Default(kind) => fields.push(ClauseField::u32(
            crate::ROUP_FIELD_KIND,
            "kind",
            match kind {
                AccDefaultKind::None => crate::ROUP_ACC_DEFAULT_NONE,
                AccDefaultKind::Present => crate::ROUP_ACC_DEFAULT_PRESENT,
            },
        )),
        AccClausePayload::Copy(copy) => {
            fields.push(ClauseField::u32(
                crate::ROUP_FIELD_KIND,
                "kind",
                match copy.kind() {
                    roup::ast::AccCopyKind::Copy => crate::ROUP_ACC_COPY,
                    roup::ast::AccCopyKind::CopyIn => crate::ROUP_ACC_COPYIN,
                    roup::ast::AccCopyKind::CopyOut => crate::ROUP_ACC_COPYOUT,
                },
            ));
            fields.push(ClauseField::u32s(
                crate::ROUP_FIELD_MODIFIERS,
                "modifiers",
                copy.modifiers().iter().map(acc_data_modifier).collect(),
            ));
            fields.push(clause_items_field(copy.variables()));
        }
        AccClausePayload::Create(create) => {
            fields.push(ClauseField::u32(
                crate::ROUP_FIELD_KIND,
                "kind",
                match create.kind() {
                    roup::ast::AccCreateKind::Create => crate::ROUP_ACC_CREATE,
                },
            ));
            fields.push(ClauseField::u32s(
                crate::ROUP_FIELD_MODIFIERS,
                "modifiers",
                create.modifiers().iter().map(acc_data_modifier).collect(),
            ));
            fields.push(clause_items_field(create.variables()));
        }
        AccClausePayload::Data(data) => {
            fields.push(ClauseField::u32(
                crate::ROUP_FIELD_KIND,
                "kind",
                acc_data_kind(data.kind()),
            ));
            fields.push(clause_items_field(data.variables()));
        }
        AccClausePayload::DeviceType(values) => fields.push(ClauseField::nodes(
            crate::ROUP_FIELD_VALUES,
            "device_types",
            values.iter().map(acc_device_type_node).collect(),
        )),
        AccClausePayload::Gang(gang) => {
            fields.push(ClauseField::nodes(
                crate::ROUP_FIELD_ARGUMENTS,
                "arguments",
                gang.arguments()
                    .iter()
                    .map(acc_gang_argument_node)
                    .collect(),
            ));
        }
        AccClausePayload::Worker(worker) => {
            if let Some(modifier) = worker.modifier() {
                fields.push(ClauseField::u32(
                    crate::ROUP_FIELD_MODIFIER,
                    "modifier",
                    match modifier {
                        AccWorkerModifier::Num => crate::ROUP_ACC_WORKER_NUM,
                        AccWorkerModifier::ExprOnly => crate::ROUP_ACC_WORKER_EXPRESSION,
                    },
                ));
            }
            if let Some(expression) = worker.expression() {
                fields.push(ClauseField::string(
                    crate::ROUP_FIELD_VALUE,
                    "value",
                    expression,
                ));
            }
        }
        AccClausePayload::Vector(vector) => {
            if let Some(modifier) = vector.modifier() {
                fields.push(ClauseField::u32(
                    crate::ROUP_FIELD_MODIFIER,
                    "modifier",
                    match modifier {
                        AccVectorModifier::Length => crate::ROUP_ACC_VECTOR_LENGTH,
                        AccVectorModifier::ExprOnly => crate::ROUP_ACC_VECTOR_EXPRESSION,
                    },
                ));
            }
            if let Some(expression) = vector.expression() {
                fields.push(ClauseField::string(
                    crate::ROUP_FIELD_VALUE,
                    "value",
                    expression,
                ));
            }
        }
        AccClausePayload::Wait(wait) => {
            push_optional_string(
                &mut fields,
                crate::ROUP_FIELD_DEVICE_NUM,
                "device_num",
                wait.devnum(),
            );
            fields.push(ClauseField::strings(
                crate::ROUP_FIELD_VALUES,
                "queues",
                wait.queues(),
            ));
        }
        AccClausePayload::Reduction(reduction) => {
            fields.push(ClauseField::node(
                crate::ROUP_FIELD_OPERATOR,
                "operator",
                acc_reduction_operator_node(reduction.operator()),
            ));
            fields.push(clause_items_field(reduction.variables()));
        }
    }
    Ok(fields)
}

fn push_optional_string<T: AbiStringLeaf>(
    fields: &mut Vec<ClauseField>,
    id: u32,
    name: &'static str,
    value: Option<&T>,
) {
    if let Some(value) = value {
        fields.push(ClauseField::string(id, name, value));
    }
}

fn omp_scan_mode(mode: roup::ir::ScanClauseMode) -> u32 {
    match mode {
        roup::ir::ScanClauseMode::Inclusive => crate::ROUP_OMP_SCAN_INCLUSIVE,
        roup::ir::ScanClauseMode::Exclusive => crate::ROUP_OMP_SCAN_EXCLUSIVE,
    }
}

fn omp_adjust_args_modifier(modifier: roup::ir::AdjustArgsModifier) -> u32 {
    match modifier {
        roup::ir::AdjustArgsModifier::Nothing => crate::ROUP_OMP_ADJUST_ARGS_NOTHING,
        roup::ir::AdjustArgsModifier::NeedDevicePtr => crate::ROUP_OMP_ADJUST_ARGS_NEED_DEVICE_PTR,
        roup::ir::AdjustArgsModifier::NeedDeviceAddr => {
            crate::ROUP_OMP_ADJUST_ARGS_NEED_DEVICE_ADDR
        }
    }
}

fn omp_firstprivate_modifier(modifier: roup::ir::FirstprivateModifier) -> u32 {
    match modifier {
        roup::ir::FirstprivateModifier::Saved => crate::ROUP_OMP_FIRSTPRIVATE_SAVED,
    }
}

fn omp_lastprivate_modifier(modifier: roup::ir::LastprivateModifier) -> u32 {
    match modifier {
        roup::ir::LastprivateModifier::Conditional => crate::ROUP_OMP_LASTPRIVATE_CONDITIONAL,
    }
}

fn omp_original_sharing(sharing: roup::ir::OriginalSharing) -> u32 {
    match sharing {
        roup::ir::OriginalSharing::Default => crate::ROUP_OMP_ORIGINAL_SHARING_DEFAULT,
        roup::ir::OriginalSharing::Private => crate::ROUP_OMP_ORIGINAL_SHARING_PRIVATE,
        roup::ir::OriginalSharing::Shared => crate::ROUP_OMP_ORIGINAL_SHARING_SHARED,
    }
}

fn omp_default_kind(kind: roup::ir::DefaultKind) -> u32 {
    match kind {
        roup::ir::DefaultKind::Shared => crate::ROUP_OMP_DEFAULT_SHARED,
        roup::ir::DefaultKind::None => crate::ROUP_OMP_DEFAULT_NONE,
        roup::ir::DefaultKind::Private => crate::ROUP_OMP_DEFAULT_PRIVATE,
        roup::ir::DefaultKind::Firstprivate => crate::ROUP_OMP_DEFAULT_FIRSTPRIVATE,
    }
}

fn omp_defaultmap_behavior(behavior: roup::ir::DefaultmapBehavior) -> u32 {
    match behavior {
        roup::ir::DefaultmapBehavior::Alloc => crate::ROUP_OMP_DEFAULTMAP_ALLOC,
        roup::ir::DefaultmapBehavior::To => crate::ROUP_OMP_DEFAULTMAP_TO,
        roup::ir::DefaultmapBehavior::From => crate::ROUP_OMP_DEFAULTMAP_FROM,
        roup::ir::DefaultmapBehavior::Tofrom => crate::ROUP_OMP_DEFAULTMAP_TOFROM,
        roup::ir::DefaultmapBehavior::Firstprivate => crate::ROUP_OMP_DEFAULTMAP_FIRSTPRIVATE,
        roup::ir::DefaultmapBehavior::None => crate::ROUP_OMP_DEFAULTMAP_NONE,
        roup::ir::DefaultmapBehavior::Default => crate::ROUP_OMP_DEFAULTMAP_DEFAULT,
        roup::ir::DefaultmapBehavior::Present => crate::ROUP_OMP_DEFAULTMAP_PRESENT,
        roup::ir::DefaultmapBehavior::Private => crate::ROUP_OMP_DEFAULTMAP_PRIVATE,
        roup::ir::DefaultmapBehavior::SelfMap => crate::ROUP_OMP_DEFAULTMAP_SELF,
        roup::ir::DefaultmapBehavior::Storage => crate::ROUP_OMP_DEFAULTMAP_STORAGE,
    }
}

fn omp_defaultmap_category(category: roup::ir::DefaultmapCategory) -> u32 {
    match category {
        roup::ir::DefaultmapCategory::Scalar => crate::ROUP_OMP_DEFAULTMAP_CATEGORY_SCALAR,
        roup::ir::DefaultmapCategory::Aggregate => crate::ROUP_OMP_DEFAULTMAP_CATEGORY_AGGREGATE,
        roup::ir::DefaultmapCategory::Pointer => crate::ROUP_OMP_DEFAULTMAP_CATEGORY_POINTER,
        roup::ir::DefaultmapCategory::All => crate::ROUP_OMP_DEFAULTMAP_CATEGORY_ALL,
        roup::ir::DefaultmapCategory::Allocatable => {
            crate::ROUP_OMP_DEFAULTMAP_CATEGORY_ALLOCATABLE
        }
    }
}

fn omp_map_type(kind: roup::ir::MapType) -> u32 {
    match kind {
        roup::ir::MapType::To => crate::ROUP_OMP_MAP_TO,
        roup::ir::MapType::From => crate::ROUP_OMP_MAP_FROM,
        roup::ir::MapType::ToFrom => crate::ROUP_OMP_MAP_TOFROM,
        roup::ir::MapType::Storage => crate::ROUP_OMP_MAP_STORAGE,
    }
}

fn omp_map_modifier(modifier: roup::ir::MapModifier) -> u32 {
    match modifier {
        roup::ir::MapModifier::Always => crate::ROUP_OMP_MAP_MODIFIER_ALWAYS,
        roup::ir::MapModifier::Close => crate::ROUP_OMP_MAP_MODIFIER_CLOSE,
        roup::ir::MapModifier::Present => crate::ROUP_OMP_MAP_MODIFIER_PRESENT,
        roup::ir::MapModifier::SelfMap => crate::ROUP_OMP_MAP_MODIFIER_SELF,
        roup::ir::MapModifier::Iterator => crate::ROUP_OMP_MAP_MODIFIER_ITERATOR,
        roup::ir::MapModifier::Ref(roup::ir::MapRefKind::Pointee) => {
            crate::ROUP_OMP_MAP_MODIFIER_REF_POINTEE
        }
        roup::ir::MapModifier::Ref(roup::ir::MapRefKind::Pointer) => {
            crate::ROUP_OMP_MAP_MODIFIER_REF_POINTER
        }
        roup::ir::MapModifier::Ref(roup::ir::MapRefKind::PointerAndPointee) => {
            crate::ROUP_OMP_MAP_MODIFIER_REF_POINTER_AND_POINTEE
        }
        roup::ir::MapModifier::Delete => crate::ROUP_OMP_MAP_MODIFIER_DELETE,
    }
}

fn omp_depend_type(kind: roup::ir::DependType) -> u32 {
    match kind {
        roup::ir::DependType::In => crate::ROUP_OMP_DEPEND_IN,
        roup::ir::DependType::Out => crate::ROUP_OMP_DEPEND_OUT,
        roup::ir::DependType::Inout => crate::ROUP_OMP_DEPEND_INOUT,
        roup::ir::DependType::Inoutset => crate::ROUP_OMP_DEPEND_INOUTSET,
        roup::ir::DependType::Mutexinoutset => crate::ROUP_OMP_DEPEND_MUTEXINOUTSET,
    }
}

fn omp_doacross_type(kind: roup::ir::DoacrossType) -> u32 {
    match kind {
        roup::ir::DoacrossType::Source => crate::ROUP_OMP_DOACROSS_SOURCE,
        roup::ir::DoacrossType::Sink => crate::ROUP_OMP_DOACROSS_SINK,
    }
}

fn omp_schedule_kind(kind: roup::ir::ScheduleKind) -> u32 {
    match kind {
        roup::ir::ScheduleKind::Static => crate::ROUP_OMP_SCHEDULE_STATIC,
        roup::ir::ScheduleKind::Dynamic => crate::ROUP_OMP_SCHEDULE_DYNAMIC,
        roup::ir::ScheduleKind::Guided => crate::ROUP_OMP_SCHEDULE_GUIDED,
        roup::ir::ScheduleKind::Auto => crate::ROUP_OMP_SCHEDULE_AUTO,
        roup::ir::ScheduleKind::Runtime => crate::ROUP_OMP_SCHEDULE_RUNTIME,
    }
}

fn omp_schedule_modifier(modifier: roup::ir::ScheduleModifier) -> u32 {
    match modifier {
        roup::ir::ScheduleModifier::Monotonic => crate::ROUP_OMP_SCHEDULE_MODIFIER_MONOTONIC,
        roup::ir::ScheduleModifier::Nonmonotonic => crate::ROUP_OMP_SCHEDULE_MODIFIER_NONMONOTONIC,
        roup::ir::ScheduleModifier::Simd => crate::ROUP_OMP_SCHEDULE_MODIFIER_SIMD,
    }
}

fn omp_linear_modifier(modifier: roup::ir::LinearModifier) -> u32 {
    match modifier {
        roup::ir::LinearModifier::Val => crate::ROUP_OMP_LINEAR_VAL,
        roup::ir::LinearModifier::Ref => crate::ROUP_OMP_LINEAR_REF,
        roup::ir::LinearModifier::Uval => crate::ROUP_OMP_LINEAR_UVAL,
    }
}

fn omp_allocate_source_syntax(syntax: roup::ir::AllocateSourceSyntax) -> u32 {
    match syntax {
        roup::ir::AllocateSourceSyntax::Unmodified => crate::ROUP_OMP_ALLOCATE_SOURCE_UNMODIFIED,
        roup::ir::AllocateSourceSyntax::SimpleAllocator => {
            crate::ROUP_OMP_ALLOCATE_SOURCE_SIMPLE_ALLOCATOR
        }
        roup::ir::AllocateSourceSyntax::Modifiers => crate::ROUP_OMP_ALLOCATE_SOURCE_MODIFIERS,
    }
}

fn omp_threadset_kind(kind: roup::ir::ThreadsetKind) -> u32 {
    match kind {
        roup::ir::ThreadsetKind::OmpPool => crate::ROUP_OMP_THREADSET_OMP_POOL,
        roup::ir::ThreadsetKind::OmpTeam => crate::ROUP_OMP_THREADSET_OMP_TEAM,
    }
}

fn omp_memscope_kind(kind: roup::ir::MemscopeKind) -> u32 {
    match kind {
        roup::ir::MemscopeKind::All => crate::ROUP_OMP_MEMSCOPE_ALL,
        roup::ir::MemscopeKind::Cgroup => crate::ROUP_OMP_MEMSCOPE_CGROUP,
        roup::ir::MemscopeKind::Device => crate::ROUP_OMP_MEMSCOPE_DEVICE,
    }
}

fn omp_proc_bind(kind: roup::ir::ProcBind) -> u32 {
    match kind {
        roup::ir::ProcBind::Close => crate::ROUP_OMP_PROC_BIND_CLOSE,
        roup::ir::ProcBind::Spread => crate::ROUP_OMP_PROC_BIND_SPREAD,
        roup::ir::ProcBind::Primary => crate::ROUP_OMP_PROC_BIND_PRIMARY,
    }
}

fn omp_bind_modifier(modifier: roup::ir::BindModifier) -> u32 {
    match modifier {
        roup::ir::BindModifier::Teams => crate::ROUP_OMP_BIND_TEAMS,
        roup::ir::BindModifier::Parallel => crate::ROUP_OMP_BIND_PARALLEL,
        roup::ir::BindModifier::Thread => crate::ROUP_OMP_BIND_THREAD,
    }
}

fn omp_device_modifier(modifier: roup::ir::DeviceModifier) -> u32 {
    match modifier {
        roup::ir::DeviceModifier::Ancestor => crate::ROUP_OMP_DEVICE_ANCESTOR,
        roup::ir::DeviceModifier::DeviceNum => crate::ROUP_OMP_DEVICE_NUM,
    }
}

fn omp_device_type(kind: roup::ir::DeviceType) -> u32 {
    match kind {
        roup::ir::DeviceType::Host => crate::ROUP_OMP_DEVICE_TYPE_HOST,
        roup::ir::DeviceType::Nohost => crate::ROUP_OMP_DEVICE_TYPE_NOHOST,
        roup::ir::DeviceType::Any => crate::ROUP_OMP_DEVICE_TYPE_ANY,
    }
}

fn omp_at_kind(kind: roup::ir::AtKind) -> u32 {
    match kind {
        roup::ir::AtKind::Compilation => crate::ROUP_OMP_AT_COMPILATION,
        roup::ir::AtKind::Execution => crate::ROUP_OMP_AT_EXECUTION,
    }
}

fn omp_severity_kind(kind: roup::ir::SeverityKind) -> u32 {
    match kind {
        roup::ir::SeverityKind::Fatal => crate::ROUP_OMP_SEVERITY_FATAL,
        roup::ir::SeverityKind::Warning => crate::ROUP_OMP_SEVERITY_WARNING,
    }
}

fn omp_memory_order(order: roup::ir::MemoryOrder) -> u32 {
    match order {
        roup::ir::MemoryOrder::SeqCst => crate::ROUP_OMP_MEMORY_ORDER_SEQ_CST,
        roup::ir::MemoryOrder::AcqRel => crate::ROUP_OMP_MEMORY_ORDER_ACQ_REL,
        roup::ir::MemoryOrder::Release => crate::ROUP_OMP_MEMORY_ORDER_RELEASE,
        roup::ir::MemoryOrder::Acquire => crate::ROUP_OMP_MEMORY_ORDER_ACQUIRE,
        roup::ir::MemoryOrder::Relaxed => crate::ROUP_OMP_MEMORY_ORDER_RELAXED,
    }
}

fn omp_atomic_operation(operation: roup::ir::AtomicOp) -> u32 {
    match operation {
        roup::ir::AtomicOp::Read => crate::ROUP_OMP_ATOMIC_READ,
        roup::ir::AtomicOp::Write => crate::ROUP_OMP_ATOMIC_WRITE,
        roup::ir::AtomicOp::Update => crate::ROUP_OMP_ATOMIC_UPDATE,
    }
}

fn omp_order_modifier(modifier: roup::ir::OrderModifier) -> u32 {
    match modifier {
        roup::ir::OrderModifier::Reproducible => crate::ROUP_OMP_ORDER_REPRODUCIBLE,
        roup::ir::OrderModifier::Unconstrained => crate::ROUP_OMP_ORDER_UNCONSTRAINED,
    }
}

fn omp_order_kind(kind: roup::ir::OrderKind) -> u32 {
    match kind {
        roup::ir::OrderKind::Concurrent => crate::ROUP_OMP_ORDER_CONCURRENT,
    }
}

fn omp_depobj_update(dependence: roup::ir::DepobjUpdateDependence) -> u32 {
    match dependence {
        roup::ir::DepobjUpdateDependence::In => crate::ROUP_OMP_DEPOBJ_UPDATE_IN,
        roup::ir::DepobjUpdateDependence::Out => crate::ROUP_OMP_DEPOBJ_UPDATE_OUT,
        roup::ir::DepobjUpdateDependence::Inout => crate::ROUP_OMP_DEPOBJ_UPDATE_INOUT,
        roup::ir::DepobjUpdateDependence::Inoutset => crate::ROUP_OMP_DEPOBJ_UPDATE_INOUTSET,
        roup::ir::DepobjUpdateDependence::Mutexinoutset => {
            crate::ROUP_OMP_DEPOBJ_UPDATE_MUTEXINOUTSET
        }
    }
}

fn semantic_node(family: u32, variant: u32, fields: Vec<ClauseField>) -> NodeRecord {
    NodeRecord {
        kind: RoupNodeKind { family, variant },
        fields,
    }
}

fn omp_parameter_list_item_node(item: &roup::ir::OmpParameterListItem) -> NodeRecord {
    use roup::ir::OmpParameterListItem as I;
    match item {
        I::Named(name) => semantic_node(
            crate::ROUP_NODE_FAMILY_OMP_PARAMETER_LIST_ITEM,
            crate::ROUP_OMP_PARAMETER_NAMED,
            vec![ClauseField::string(crate::ROUP_FIELD_NAME, "name", name)],
        ),
        I::Position(position) => semantic_node(
            crate::ROUP_NODE_FAMILY_OMP_PARAMETER_LIST_ITEM,
            crate::ROUP_OMP_PARAMETER_POSITION,
            vec![ClauseField::u64(
                crate::ROUP_FIELD_VALUE,
                "position",
                *position,
            )],
        ),
        I::Range(range) => {
            let mut fields = Vec::new();
            push_optional_string(
                &mut fields,
                crate::ROUP_FIELD_LOWER_BOUND,
                "lower_bound",
                range.lower(),
            );
            push_optional_string(
                &mut fields,
                crate::ROUP_FIELD_UPPER_BOUND,
                "upper_bound",
                range.upper(),
            );
            semantic_node(
                crate::ROUP_NODE_FAMILY_OMP_PARAMETER_LIST_ITEM,
                crate::ROUP_OMP_PARAMETER_RANGE,
                fields,
            )
        }
    }
}

fn omp_interop_type(kind: roup::ir::OmpInteropType) -> u32 {
    match kind {
        roup::ir::OmpInteropType::Target => crate::ROUP_OMP_INTEROP_TARGET,
        roup::ir::OmpInteropType::Targetsync => crate::ROUP_OMP_INTEROP_TARGETSYNC,
    }
}

fn omp_append_operation_node(operation: &roup::ir::OmpAppendOperation) -> NodeRecord {
    match operation {
        roup::ir::OmpAppendOperation::Interop(modifiers) => semantic_node(
            crate::ROUP_NODE_FAMILY_OMP_APPEND_OPERATION,
            crate::ROUP_OMP_APPEND_INTEROP,
            vec![
                ClauseField::u32s(
                    crate::ROUP_FIELD_INTEROP_TYPES,
                    "interop_types",
                    modifiers
                        .interop_types
                        .iter()
                        .copied()
                        .map(omp_interop_type)
                        .collect(),
                ),
                ClauseField::nodes(
                    crate::ROUP_FIELD_PREFERENCES,
                    "preferences",
                    modifiers
                        .preferences
                        .iter()
                        .map(omp_preference_specification_node)
                        .collect(),
                ),
            ],
        ),
    }
}

fn variable_clause_item_node(variable: &roup::ir::Variable) -> NodeRecord {
    semantic_node(
        crate::ROUP_NODE_FAMILY_CLAUSE_ITEM,
        crate::ROUP_CLAUSE_ITEM_VARIABLE,
        vec![ClauseField::string(
            crate::ROUP_FIELD_VARIABLE,
            "variable",
            variable,
        )],
    )
}

fn omp_dependence_node(dependence: &roup::ir::OmpDependence) -> NodeRecord {
    match dependence {
        roup::ir::OmpDependence::Locators { kind, locators } => semantic_node(
            crate::ROUP_NODE_FAMILY_OMP_DEPENDENCE,
            crate::ROUP_OMP_DEPENDENCE_LOCATORS,
            vec![
                ClauseField::u32(
                    crate::ROUP_FIELD_DEPEND_TYPE,
                    "depend_type",
                    omp_depend_type(*kind),
                ),
                ClauseField::nodes(
                    crate::ROUP_FIELD_ITEMS,
                    "locators",
                    locators.iter().map(omp_locator_node).collect(),
                ),
            ],
        ),
        roup::ir::OmpDependence::Depobjs { objects } => semantic_node(
            crate::ROUP_NODE_FAMILY_OMP_DEPENDENCE,
            crate::ROUP_OMP_DEPENDENCE_DEPOBJS,
            vec![ClauseField::nodes(
                crate::ROUP_FIELD_OBJECTS,
                "objects",
                objects.iter().map(variable_clause_item_node).collect(),
            )],
        ),
    }
}

fn omp_doacross_iteration_node(iteration: &roup::ir::OmpDoacrossIteration) -> NodeRecord {
    match iteration {
        roup::ir::OmpDoacrossIteration::Current => semantic_node(
            crate::ROUP_NODE_FAMILY_OMP_DOACROSS_ITERATION,
            crate::ROUP_OMP_DOACROSS_CURRENT,
            Vec::new(),
        ),
        roup::ir::OmpDoacrossIteration::PreviousCurrent => semantic_node(
            crate::ROUP_NODE_FAMILY_OMP_DOACROSS_ITERATION,
            crate::ROUP_OMP_DOACROSS_PREVIOUS_CURRENT,
            Vec::new(),
        ),
        roup::ir::OmpDoacrossIteration::Vector(vector) => semantic_node(
            crate::ROUP_NODE_FAMILY_OMP_DOACROSS_ITERATION,
            crate::ROUP_OMP_DOACROSS_VECTOR,
            vec![ClauseField::nodes(
                crate::ROUP_FIELD_ITEMS,
                "vector",
                vector.iter().map(omp_doacross_vector_item_node).collect(),
            )],
        ),
    }
}

fn omp_doacross_vector_item_node(item: &roup::ir::OmpDoacrossVectorItem) -> NodeRecord {
    let mut fields = vec![ClauseField::string(
        crate::ROUP_FIELD_VARIABLE,
        "variable",
        &item.variable,
    )];
    if let Some(offset) = &item.offset {
        let (kind, expression) = match offset {
            roup::ir::OmpDoacrossOffset::Add(expression) => {
                (crate::ROUP_OMP_DOACROSS_OFFSET_ADD, expression)
            }
            roup::ir::OmpDoacrossOffset::Subtract(expression) => {
                (crate::ROUP_OMP_DOACROSS_OFFSET_SUBTRACT, expression)
            }
        };
        fields.push(ClauseField::u32(
            crate::ROUP_FIELD_KIND,
            "offset_kind",
            kind,
        ));
        fields.push(ClauseField::string(
            crate::ROUP_FIELD_OFFSET,
            "offset",
            expression,
        ));
    }
    semantic_node(
        crate::ROUP_NODE_FAMILY_OMP_DOACROSS_VECTOR_ITEM,
        crate::ROUP_OMP_DOACROSS_VECTOR_ITEM,
        fields,
    )
}

fn clause_items_field(items: &[roup::ir::ClauseItem]) -> ClauseField {
    ClauseField::nodes(
        crate::ROUP_FIELD_ITEMS,
        "items",
        items.iter().map(clause_item_node).collect(),
    )
}

fn clause_item_node(item: &roup::ir::ClauseItem) -> NodeRecord {
    let (variant, field) = match item {
        roup::ir::ClauseItem::Identifier(identifier) => (
            crate::ROUP_CLAUSE_ITEM_IDENTIFIER,
            ClauseField::string(crate::ROUP_FIELD_NAME, "name", identifier),
        ),
        roup::ir::ClauseItem::Variable(variable) => (
            crate::ROUP_CLAUSE_ITEM_VARIABLE,
            ClauseField::string(crate::ROUP_FIELD_VARIABLE, "variable", variable),
        ),
        roup::ir::ClauseItem::FortranCommonBlock(name) => (
            crate::ROUP_CLAUSE_ITEM_FORTRAN_COMMON_BLOCK,
            ClauseField::string(crate::ROUP_FIELD_NAME, "name", name),
        ),
        roup::ir::ClauseItem::Expression(expression) => (
            crate::ROUP_CLAUSE_ITEM_EXPRESSION,
            ClauseField::string(crate::ROUP_FIELD_VALUE, "expression", expression),
        ),
    };
    semantic_node(crate::ROUP_NODE_FAMILY_CLAUSE_ITEM, variant, vec![field])
}

fn omp_count_node(count: &roup::ir::OmpCount) -> NodeRecord {
    match count {
        roup::ir::OmpCount::Fill => semantic_node(
            crate::ROUP_NODE_FAMILY_OMP_COUNT,
            crate::ROUP_OMP_COUNT_FILL,
            Vec::new(),
        ),
        roup::ir::OmpCount::Expression(expression) => semantic_node(
            crate::ROUP_NODE_FAMILY_OMP_COUNT,
            crate::ROUP_OMP_COUNT_EXPRESSION,
            vec![ClauseField::string(
                crate::ROUP_FIELD_VALUE,
                "expression",
                expression,
            )],
        ),
    }
}

fn omp_locator_node(locator: &roup::ir::OmpLocator) -> NodeRecord {
    match locator {
        roup::ir::OmpLocator::AllMemory => semantic_node(
            crate::ROUP_NODE_FAMILY_OMP_LOCATOR,
            crate::ROUP_OMP_LOCATOR_ALL_MEMORY,
            Vec::new(),
        ),
        roup::ir::OmpLocator::LValue(value) => semantic_node(
            crate::ROUP_NODE_FAMILY_OMP_LOCATOR,
            crate::ROUP_OMP_LOCATOR_LVALUE,
            vec![ClauseField::string(
                crate::ROUP_FIELD_VALUE,
                "lvalue",
                value,
            )],
        ),
        roup::ir::OmpLocator::PotentialLValue(value) => semantic_node(
            crate::ROUP_NODE_FAMILY_OMP_LOCATOR,
            crate::ROUP_OMP_LOCATOR_POTENTIAL_LVALUE,
            vec![ClauseField::string(
                crate::ROUP_FIELD_VALUE,
                "potential_lvalue",
                value,
            )],
        ),
        roup::ir::OmpLocator::FortranCommonBlock(name) => semantic_node(
            crate::ROUP_NODE_FAMILY_OMP_LOCATOR,
            crate::ROUP_OMP_LOCATOR_FORTRAN_COMMON_BLOCK,
            vec![ClauseField::string(crate::ROUP_FIELD_NAME, "name", name)],
        ),
    }
}

fn omp_storage_item_node(item: &roup::ast::OmpStorageListItem) -> NodeRecord {
    let (variant, field) = match item {
        roup::ast::OmpStorageListItem::Name(name) => (
            crate::ROUP_OMP_STORAGE_ITEM_NAME,
            ClauseField::string(
                crate::ROUP_FIELD_NAME,
                "name",
                qualified_name_leaf(name).as_str(),
            ),
        ),
        roup::ast::OmpStorageListItem::FortranCommonBlock(name) => (
            crate::ROUP_OMP_STORAGE_ITEM_FORTRAN_COMMON_BLOCK,
            ClauseField::string(crate::ROUP_FIELD_NAME, "name", name),
        ),
    };
    semantic_node(
        crate::ROUP_NODE_FAMILY_OMP_STORAGE_ITEM,
        variant,
        vec![field],
    )
}

fn omp_declare_target_item_node(item: &roup::ast::OmpDeclareTargetListItem) -> NodeRecord {
    let (variant, field) = match item {
        roup::ast::OmpDeclareTargetListItem::Name(name) => (
            crate::ROUP_OMP_DECLARE_TARGET_ITEM_NAME,
            ClauseField::string(
                crate::ROUP_FIELD_NAME,
                "name",
                qualified_name_leaf(name).as_str(),
            ),
        ),
        roup::ast::OmpDeclareTargetListItem::FortranCommonBlock(name) => (
            crate::ROUP_OMP_DECLARE_TARGET_ITEM_FORTRAN_COMMON_BLOCK,
            ClauseField::string(crate::ROUP_FIELD_NAME, "name", name),
        ),
    };
    semantic_node(
        crate::ROUP_NODE_FAMILY_OMP_DECLARE_TARGET_ITEM,
        variant,
        vec![field],
    )
}

fn omp_flush_item_node(item: &roup::ast::OmpFlushListItem) -> NodeRecord {
    let (variant, field) = match item {
        roup::ast::OmpFlushListItem::Identifier(identifier) => (
            crate::ROUP_OMP_FLUSH_ITEM_IDENTIFIER,
            ClauseField::string(crate::ROUP_FIELD_NAME, "name", identifier),
        ),
        roup::ast::OmpFlushListItem::Variable(variable) => (
            crate::ROUP_OMP_FLUSH_ITEM_VARIABLE,
            ClauseField::string(crate::ROUP_FIELD_VARIABLE, "variable", variable),
        ),
        roup::ast::OmpFlushListItem::FortranCommonBlock(name) => (
            crate::ROUP_OMP_FLUSH_ITEM_FORTRAN_COMMON_BLOCK,
            ClauseField::string(crate::ROUP_FIELD_NAME, "name", name),
        ),
    };
    semantic_node(crate::ROUP_NODE_FAMILY_OMP_FLUSH_ITEM, variant, vec![field])
}

fn omp_mapper_id_node(identifier: &roup::ast::OmpMapperId) -> NodeRecord {
    let (variant, fields) = match identifier {
        roup::ast::OmpMapperId::Default => (crate::ROUP_OMP_MAPPER_ID_DEFAULT, Vec::new()),
        roup::ast::OmpMapperId::User(identifier) => (
            crate::ROUP_OMP_MAPPER_ID_USER,
            vec![ClauseField::string(
                crate::ROUP_FIELD_NAME,
                "name",
                identifier,
            )],
        ),
    };
    semantic_node(crate::ROUP_NODE_FAMILY_OMP_MAPPER_ID, variant, fields)
}

fn omp_function_name_node(name: &roup::ast::OmpFunctionName) -> NodeRecord {
    let (variant, spelling) = match name {
        roup::ast::OmpFunctionName::Name(name) => (
            crate::ROUP_OMP_ID_EXPRESSION_NAME,
            qualified_name_leaf(name),
        ),
        roup::ast::OmpFunctionName::CppTemplateId(template_id) => (
            crate::ROUP_OMP_ID_EXPRESSION_CPP_TEMPLATE_ID,
            template_id.render_abi_leaf(),
        ),
    };
    semantic_node(
        crate::ROUP_NODE_FAMILY_OMP_ID_EXPRESSION,
        variant,
        vec![ClauseField::string(
            crate::ROUP_FIELD_NAME,
            "spelling",
            spelling.as_str(),
        )],
    )
}

fn omp_id_expression_node(expression: &roup::ast::OmpIdExpression) -> NodeRecord {
    match expression {
        roup::ast::OmpIdExpression::Name(name) => semantic_node(
            crate::ROUP_NODE_FAMILY_OMP_ID_EXPRESSION,
            crate::ROUP_OMP_ID_EXPRESSION_NAME,
            vec![ClauseField::string(
                crate::ROUP_FIELD_NAME,
                "spelling",
                qualified_name_leaf(name).as_str(),
            )],
        ),
        roup::ast::OmpIdExpression::CppTemplateId(name) => semantic_node(
            crate::ROUP_NODE_FAMILY_OMP_ID_EXPRESSION,
            crate::ROUP_OMP_ID_EXPRESSION_CPP_TEMPLATE_ID,
            vec![ClauseField::string(
                crate::ROUP_FIELD_NAME,
                "spelling",
                name,
            )],
        ),
        roup::ast::OmpIdExpression::CppOperatorFunction(operator) => {
            let mut fields = vec![
                ClauseField::boolean(crate::ROUP_FIELD_GLOBAL, "global", operator.is_global()),
                ClauseField::u32(
                    crate::ROUP_FIELD_OPERATOR,
                    "operator",
                    omp_cpp_reduction_operator(operator.operator()),
                ),
            ];
            if let Some(qualifier) = operator.qualifier() {
                fields.push(ClauseField::node(
                    crate::ROUP_FIELD_QUALIFIER,
                    "qualifier",
                    omp_cpp_operator_qualifier_node(qualifier),
                ));
            }
            semantic_node(
                crate::ROUP_NODE_FAMILY_OMP_ID_EXPRESSION,
                crate::ROUP_OMP_ID_EXPRESSION_CPP_OPERATOR_FUNCTION,
                fields,
            )
        }
    }
}

fn omp_cpp_operator_qualifier_node(qualifier: &roup::ast::OmpCppOperatorQualifier) -> NodeRecord {
    let (variant, spelling) = match qualifier {
        roup::ast::OmpCppOperatorQualifier::Name(name) => (
            crate::ROUP_OMP_CPP_OPERATOR_QUALIFIER_NAME,
            qualified_name_leaf(name),
        ),
        roup::ast::OmpCppOperatorQualifier::TemplateId(template_id) => (
            crate::ROUP_OMP_CPP_OPERATOR_QUALIFIER_TEMPLATE_ID,
            template_id.render_abi_leaf(),
        ),
    };
    semantic_node(
        crate::ROUP_NODE_FAMILY_OMP_CPP_OPERATOR_QUALIFIER,
        variant,
        vec![ClauseField::string(
            crate::ROUP_FIELD_NAME,
            "spelling",
            spelling.as_str(),
        )],
    )
}

fn omp_cpp_reduction_operator(operator: roup::ast::OmpCppReductionOperator) -> u32 {
    match operator {
        roup::ast::OmpCppReductionOperator::Add => crate::ROUP_OMP_CPP_OPERATOR_ADD,
        roup::ast::OmpCppReductionOperator::Subtract => crate::ROUP_OMP_CPP_OPERATOR_SUBTRACT,
        roup::ast::OmpCppReductionOperator::Multiply => crate::ROUP_OMP_CPP_OPERATOR_MULTIPLY,
        roup::ast::OmpCppReductionOperator::BitwiseAnd => crate::ROUP_OMP_CPP_OPERATOR_BITWISE_AND,
        roup::ast::OmpCppReductionOperator::BitwiseOr => crate::ROUP_OMP_CPP_OPERATOR_BITWISE_OR,
        roup::ast::OmpCppReductionOperator::BitwiseXor => crate::ROUP_OMP_CPP_OPERATOR_BITWISE_XOR,
        roup::ast::OmpCppReductionOperator::LogicalAnd => crate::ROUP_OMP_CPP_OPERATOR_LOGICAL_AND,
        roup::ast::OmpCppReductionOperator::LogicalOr => crate::ROUP_OMP_CPP_OPERATOR_LOGICAL_OR,
    }
}

fn omp_reduction_identifier_node(identifier: &roup::ast::OmpReductionIdentifier) -> NodeRecord {
    use roup::ast::{OmpFortranReductionIntrinsic as F, OmpReductionIdentifier as I};
    let (variant, fields) = match identifier {
        I::Add => (crate::ROUP_OMP_IDENTIFIER_ADD, Vec::new()),
        I::Subtract => (crate::ROUP_OMP_IDENTIFIER_SUBTRACT, Vec::new()),
        I::Multiply => (crate::ROUP_OMP_IDENTIFIER_MULTIPLY, Vec::new()),
        I::BitwiseAnd => (crate::ROUP_OMP_IDENTIFIER_BITWISE_AND, Vec::new()),
        I::BitwiseOr => (crate::ROUP_OMP_IDENTIFIER_BITWISE_OR, Vec::new()),
        I::BitwiseXor => (crate::ROUP_OMP_IDENTIFIER_BITWISE_XOR, Vec::new()),
        I::LogicalAnd => (crate::ROUP_OMP_IDENTIFIER_LOGICAL_AND, Vec::new()),
        I::LogicalOr => (crate::ROUP_OMP_IDENTIFIER_LOGICAL_OR, Vec::new()),
        I::FortranLogicalAnd => (crate::ROUP_OMP_IDENTIFIER_FORTRAN_LOGICAL_AND, Vec::new()),
        I::FortranLogicalOr => (crate::ROUP_OMP_IDENTIFIER_FORTRAN_LOGICAL_OR, Vec::new()),
        I::FortranLogicalEqv => (crate::ROUP_OMP_IDENTIFIER_FORTRAN_LOGICAL_EQV, Vec::new()),
        I::FortranLogicalNeqv => (crate::ROUP_OMP_IDENTIFIER_FORTRAN_LOGICAL_NEQV, Vec::new()),
        I::Name(name) => (
            crate::ROUP_OMP_IDENTIFIER_NAME,
            vec![ClauseField::node(
                crate::ROUP_FIELD_VALUE,
                "id_expression",
                omp_id_expression_node(name),
            )],
        ),
        I::FortranIntrinsic(intrinsic) => (
            match intrinsic {
                F::Max => crate::ROUP_OMP_IDENTIFIER_FORTRAN_MAX,
                F::Min => crate::ROUP_OMP_IDENTIFIER_FORTRAN_MIN,
                F::Iand => crate::ROUP_OMP_IDENTIFIER_FORTRAN_IAND,
                F::Ior => crate::ROUP_OMP_IDENTIFIER_FORTRAN_IOR,
                F::Ieor => crate::ROUP_OMP_IDENTIFIER_FORTRAN_IEOR,
            },
            Vec::new(),
        ),
        I::FortranDefinedOperator(name) => (
            crate::ROUP_OMP_IDENTIFIER_FORTRAN_DEFINED_OPERATOR,
            vec![ClauseField::string(crate::ROUP_FIELD_NAME, "name", name)],
        ),
    };
    semantic_node(crate::ROUP_NODE_FAMILY_OMP_IDENTIFIER, variant, fields)
}

fn omp_induction_identifier_node(identifier: &roup::ast::OmpInductionIdentifier) -> NodeRecord {
    use roup::ast::OmpInductionIdentifier as I;
    let (variant, fields) = match identifier {
        I::Add => (crate::ROUP_OMP_IDENTIFIER_ADD, Vec::new()),
        I::Multiply => (crate::ROUP_OMP_IDENTIFIER_MULTIPLY, Vec::new()),
        I::Name(name) => (
            crate::ROUP_OMP_IDENTIFIER_NAME,
            vec![ClauseField::node(
                crate::ROUP_FIELD_VALUE,
                "id_expression",
                omp_id_expression_node(name),
            )],
        ),
        I::DefinedOperator(name) => (
            crate::ROUP_OMP_IDENTIFIER_FORTRAN_DEFINED_OPERATOR,
            vec![ClauseField::string(crate::ROUP_FIELD_NAME, "name", name)],
        ),
    };
    semantic_node(crate::ROUP_NODE_FAMILY_OMP_IDENTIFIER, variant, fields)
}

fn omp_reduction_combiner_node(combiner: &roup::ast::OmpReductionCombiner) -> NodeRecord {
    match combiner {
        roup::ast::OmpReductionCombiner::COrCppExpression(expression) => {
            omp_stylized_expression_node(
                crate::ROUP_OMP_STYLIZED_C_CPP_EXPRESSION,
                vec![ClauseField::string(
                    crate::ROUP_FIELD_VALUE,
                    "expression",
                    expression,
                )],
            )
        }
        roup::ast::OmpReductionCombiner::FortranAssignment(assignment) => {
            omp_stylized_expression_node(
                crate::ROUP_OMP_STYLIZED_FORTRAN_ASSIGNMENT,
                omp_fortran_assignment_fields(assignment),
            )
        }
        roup::ast::OmpReductionCombiner::FortranSubroutineCall(expression) => {
            omp_stylized_expression_node(
                crate::ROUP_OMP_STYLIZED_FORTRAN_SUBROUTINE_CALL,
                vec![ClauseField::string(
                    crate::ROUP_FIELD_VALUE,
                    "call",
                    expression,
                )],
            )
        }
    }
}

fn omp_inductor_expression_node(expression: &roup::ast::OmpInductorExpression) -> NodeRecord {
    match expression {
        roup::ast::OmpInductorExpression::COrCppExpression(expression) => {
            omp_stylized_expression_node(
                crate::ROUP_OMP_STYLIZED_C_CPP_EXPRESSION,
                vec![ClauseField::string(
                    crate::ROUP_FIELD_VALUE,
                    "expression",
                    expression,
                )],
            )
        }
        roup::ast::OmpInductorExpression::FortranAssignment(assignment) => {
            omp_stylized_expression_node(
                crate::ROUP_OMP_STYLIZED_FORTRAN_ASSIGNMENT,
                omp_fortran_assignment_fields(assignment),
            )
        }
        roup::ast::OmpInductorExpression::FortranSubroutineCall(expression) => {
            omp_stylized_expression_node(
                crate::ROUP_OMP_STYLIZED_FORTRAN_SUBROUTINE_CALL,
                vec![ClauseField::string(
                    crate::ROUP_FIELD_VALUE,
                    "call",
                    expression,
                )],
            )
        }
    }
}

fn omp_stylized_expression_node(variant: u32, fields: Vec<ClauseField>) -> NodeRecord {
    semantic_node(
        crate::ROUP_NODE_FAMILY_OMP_STYLIZED_EXPRESSION,
        variant,
        fields,
    )
}

fn omp_fortran_assignment_fields(assignment: &roup::ast::OmpFortranAssignment) -> Vec<ClauseField> {
    vec![
        ClauseField::string(crate::ROUP_FIELD_VARIABLE, "target", assignment.target()),
        ClauseField::string(crate::ROUP_FIELD_VALUE, "value", assignment.value()),
    ]
}

fn omp_reduction_initializer_node(initializer: &roup::ast::OmpReductionInitializer) -> NodeRecord {
    use roup::ast::OmpReductionInitializer as I;
    let (variant, fields) = match initializer {
        I::CAssignment(value) => (
            crate::ROUP_OMP_INITIALIZER_C_ASSIGNMENT,
            vec![ClauseField::node(
                crate::ROUP_FIELD_VALUE,
                "value",
                omp_initializer_value_node(value),
            )],
        ),
        I::CppCopy(value) => (
            crate::ROUP_OMP_INITIALIZER_CPP_COPY,
            vec![ClauseField::node(
                crate::ROUP_FIELD_VALUE,
                "value",
                omp_initializer_value_node(value),
            )],
        ),
        I::CppDirect(expression) => (
            crate::ROUP_OMP_INITIALIZER_CPP_DIRECT,
            vec![ClauseField::string(
                crate::ROUP_FIELD_VALUE,
                "expression",
                expression,
            )],
        ),
        I::CppList(initializer) => (
            crate::ROUP_OMP_INITIALIZER_CPP_LIST,
            vec![ClauseField::nodes(
                crate::ROUP_FIELD_VALUES,
                "elements",
                initializer
                    .elements()
                    .iter()
                    .map(omp_initializer_value_node)
                    .collect(),
            )],
        ),
        I::COrCppFunctionCall(expression) => (
            crate::ROUP_OMP_INITIALIZER_C_CPP_FUNCTION_CALL,
            vec![ClauseField::string(
                crate::ROUP_FIELD_VALUE,
                "call",
                expression,
            )],
        ),
        I::FortranAssignment(assignment) => (
            crate::ROUP_OMP_INITIALIZER_FORTRAN_ASSIGNMENT,
            omp_fortran_assignment_fields(assignment),
        ),
        I::FortranSubroutineCall(expression) => (
            crate::ROUP_OMP_INITIALIZER_FORTRAN_SUBROUTINE_CALL,
            vec![ClauseField::string(
                crate::ROUP_FIELD_VALUE,
                "call",
                expression,
            )],
        ),
    };
    semantic_node(
        crate::ROUP_NODE_FAMILY_OMP_REDUCTION_INITIALIZER,
        variant,
        fields,
    )
}

fn omp_initializer_value_node(value: &roup::ast::OmpInitializerValue) -> NodeRecord {
    match value {
        roup::ast::OmpInitializerValue::Expression(expression) => semantic_node(
            crate::ROUP_NODE_FAMILY_OMP_INITIALIZER_VALUE,
            crate::ROUP_OMP_INITIALIZER_VALUE_EXPRESSION,
            vec![ClauseField::string(
                crate::ROUP_FIELD_VALUE,
                "expression",
                expression,
            )],
        ),
        roup::ast::OmpInitializerValue::Braced(initializer) => semantic_node(
            crate::ROUP_NODE_FAMILY_OMP_INITIALIZER_VALUE,
            crate::ROUP_OMP_INITIALIZER_VALUE_BRACED,
            vec![ClauseField::nodes(
                crate::ROUP_FIELD_VALUES,
                "elements",
                initializer
                    .elements()
                    .iter()
                    .map(omp_initializer_value_node)
                    .collect(),
            )],
        ),
    }
}

fn omp_allocator_kind_node(allocator: &roup::ir::UsesAllocatorKind) -> NodeRecord {
    use roup::ir::{UsesAllocatorBuiltin as B, UsesAllocatorKind as K};
    let (variant, fields) = match allocator {
        K::Builtin(builtin) => (
            match builtin {
                B::Null => crate::ROUP_OMP_ALLOCATOR_NULL,
                B::Default => crate::ROUP_OMP_ALLOCATOR_DEFAULT,
                B::LargeCap => crate::ROUP_OMP_ALLOCATOR_LARGE_CAP,
                B::Const => crate::ROUP_OMP_ALLOCATOR_CONST,
                B::HighBw => crate::ROUP_OMP_ALLOCATOR_HIGH_BW,
                B::LowLat => crate::ROUP_OMP_ALLOCATOR_LOW_LAT,
                B::Cgroup => crate::ROUP_OMP_ALLOCATOR_CGROUP,
                B::Pteam => crate::ROUP_OMP_ALLOCATOR_PTEAM,
                B::Thread => crate::ROUP_OMP_ALLOCATOR_THREAD,
            },
            Vec::new(),
        ),
        K::Custom(identifier) => (
            crate::ROUP_OMP_ALLOCATOR_CUSTOM,
            vec![ClauseField::string(
                crate::ROUP_FIELD_NAME,
                "name",
                identifier,
            )],
        ),
    };
    semantic_node(crate::ROUP_NODE_FAMILY_OMP_ALLOCATOR_KIND, variant, fields)
}

fn acc_size_expression_node(size: &AccSizeExpression) -> NodeRecord {
    match size {
        AccSizeExpression::Automatic => semantic_node(
            crate::ROUP_NODE_FAMILY_ACC_SIZE_EXPRESSION,
            crate::ROUP_ACC_SIZE_AUTOMATIC,
            Vec::new(),
        ),
        AccSizeExpression::Expression(expression) => semantic_node(
            crate::ROUP_NODE_FAMILY_ACC_SIZE_EXPRESSION,
            crate::ROUP_ACC_SIZE_EXPRESSION,
            vec![ClauseField::string(
                crate::ROUP_FIELD_VALUE,
                "value",
                expression,
            )],
        ),
    }
}

fn acc_cache_item_node(item: &AccCacheItem) -> NodeRecord {
    let variant = match item {
        AccCacheItem::ArrayElement(_) => crate::ROUP_ACC_CACHE_ARRAY_ELEMENT,
        AccCacheItem::ContiguousSubarray(_) => crate::ROUP_ACC_CACHE_CONTIGUOUS_SUBARRAY,
    };
    semantic_node(
        crate::ROUP_NODE_FAMILY_ACC_CACHE_ITEM,
        variant,
        vec![ClauseField::string(
            crate::ROUP_FIELD_VARIABLE,
            "variable",
            item.variable(),
        )],
    )
}

fn acc_bind_target_node(target: &AccBindTarget) -> NodeRecord {
    let (variant, fields) = match target {
        AccBindTarget::Name(name) => (
            crate::ROUP_ACC_BIND_NAME,
            vec![ClauseField::string(
                crate::ROUP_FIELD_VALUE,
                "value",
                name.as_str(),
            )],
        ),
        AccBindTarget::StringLiteral(literal) => (
            crate::ROUP_ACC_BIND_STRING_LITERAL,
            vec![
                ClauseField::string(crate::ROUP_FIELD_VALUE, "value", literal.value.as_str()),
                ClauseField::u32(
                    crate::ROUP_FIELD_ENCODING,
                    "encoding",
                    character_encoding(literal.encoding),
                ),
            ],
        ),
    };
    semantic_node(crate::ROUP_NODE_FAMILY_ACC_BIND_TARGET, variant, fields)
}

fn acc_end_kind_node(kind: AccEndKind) -> NodeRecord {
    let variant = match kind {
        AccEndKind::Atomic => crate::ROUP_ACC_END_ATOMIC,
        AccEndKind::Data => crate::ROUP_ACC_END_DATA,
        AccEndKind::HostData => crate::ROUP_ACC_END_HOST_DATA,
        AccEndKind::Kernels => crate::ROUP_ACC_END_KERNELS,
        AccEndKind::KernelsLoop => crate::ROUP_ACC_END_KERNELS_LOOP,
        AccEndKind::Loop => crate::ROUP_ACC_END_LOOP,
        AccEndKind::Parallel => crate::ROUP_ACC_END_PARALLEL,
        AccEndKind::ParallelLoop => crate::ROUP_ACC_END_PARALLEL_LOOP,
        AccEndKind::Serial => crate::ROUP_ACC_END_SERIAL,
        AccEndKind::SerialLoop => crate::ROUP_ACC_END_SERIAL_LOOP,
    };
    semantic_node(crate::ROUP_NODE_FAMILY_ACC_END_KIND, variant, Vec::new())
}

fn acc_gang_argument_node(argument: &AccGangArgument) -> NodeRecord {
    let (variant, value) = match argument {
        AccGangArgument::Positional(expression) => (
            crate::ROUP_ACC_GANG_POSITIONAL,
            acc_expression_size_node(expression),
        ),
        AccGangArgument::Num(expression) => (
            crate::ROUP_ACC_GANG_NUM,
            acc_expression_size_node(expression),
        ),
        AccGangArgument::Dim(expression) => (
            crate::ROUP_ACC_GANG_DIM,
            acc_expression_size_node(expression),
        ),
        AccGangArgument::Static(size) => {
            (crate::ROUP_ACC_GANG_STATIC, acc_size_expression_node(size))
        }
    };
    semantic_node(
        crate::ROUP_NODE_FAMILY_ACC_GANG_ARGUMENT,
        variant,
        vec![ClauseField::node(crate::ROUP_FIELD_VALUE, "value", value)],
    )
}

fn acc_expression_size_node(expression: &roup::ir::Expression) -> NodeRecord {
    semantic_node(
        crate::ROUP_NODE_FAMILY_ACC_SIZE_EXPRESSION,
        crate::ROUP_ACC_SIZE_EXPRESSION,
        vec![ClauseField::string(
            crate::ROUP_FIELD_VALUE,
            "value",
            expression,
        )],
    )
}

fn omp_apply_modifier_node(modifier: &roup::ir::OmpApplyLoopModifier) -> NodeRecord {
    use roup::ir::OmpApplyLoopKind as K;
    let variant = match modifier.kind {
        K::Fused => crate::ROUP_OMP_APPLY_FUSED,
        K::Grid => crate::ROUP_OMP_APPLY_GRID,
        K::Identity => crate::ROUP_OMP_APPLY_IDENTITY,
        K::Interchanged => crate::ROUP_OMP_APPLY_INTERCHANGED,
        K::Intratile => crate::ROUP_OMP_APPLY_INTRATILE,
        K::Offsets => crate::ROUP_OMP_APPLY_OFFSETS,
        K::Reversed => crate::ROUP_OMP_APPLY_REVERSED,
        K::Split => crate::ROUP_OMP_APPLY_SPLIT,
        K::Unrolled => crate::ROUP_OMP_APPLY_UNROLLED,
    };
    semantic_node(
        crate::ROUP_NODE_FAMILY_OMP_APPLY_MODIFIER,
        variant,
        vec![ClauseField::strings(
            crate::ROUP_FIELD_INDICES,
            "indices",
            &modifier.indices,
        )],
    )
}

fn omp_preference_specification_node(
    specification: &roup::ir::OmpPreferenceSpecification,
) -> NodeRecord {
    match specification {
        roup::ir::OmpPreferenceSpecification::ForeignRuntime(identifier) => semantic_node(
            crate::ROUP_NODE_FAMILY_OMP_PREFERENCE_SPECIFICATION,
            crate::ROUP_OMP_PREFERENCE_FOREIGN_RUNTIME,
            vec![ClauseField::node(
                crate::ROUP_FIELD_VALUE,
                "foreign_runtime",
                omp_foreign_runtime_identifier_node(identifier),
            )],
        ),
        roup::ir::OmpPreferenceSpecification::Selectors(selectors) => semantic_node(
            crate::ROUP_NODE_FAMILY_OMP_PREFERENCE_SPECIFICATION,
            crate::ROUP_OMP_PREFERENCE_SELECTORS,
            vec![ClauseField::nodes(
                crate::ROUP_FIELD_SELECTORS,
                "selectors",
                selectors.iter().map(omp_preference_selector_node).collect(),
            )],
        ),
    }
}

fn omp_preference_selector_node(selector: &roup::ir::OmpPreferenceSelector) -> NodeRecord {
    match selector {
        roup::ir::OmpPreferenceSelector::ForeignRuntime(identifier) => semantic_node(
            crate::ROUP_NODE_FAMILY_OMP_PREFERENCE_SELECTOR,
            crate::ROUP_OMP_PREFERENCE_SELECTOR_FOREIGN_RUNTIME,
            vec![ClauseField::node(
                crate::ROUP_FIELD_VALUE,
                "foreign_runtime",
                omp_foreign_runtime_identifier_node(identifier),
            )],
        ),
        roup::ir::OmpPreferenceSelector::Attributes(attributes) => semantic_node(
            crate::ROUP_NODE_FAMILY_OMP_PREFERENCE_SELECTOR,
            crate::ROUP_OMP_PREFERENCE_SELECTOR_ATTRIBUTES,
            vec![ClauseField::nodes(
                crate::ROUP_FIELD_ATTRIBUTES,
                "attributes",
                attributes.iter().map(string_literal_node).collect(),
            )],
        ),
    }
}

fn omp_foreign_runtime_identifier_node(
    identifier: &roup::ir::OmpForeignRuntimeIdentifier,
) -> NodeRecord {
    match identifier {
        roup::ir::OmpForeignRuntimeIdentifier::StringLiteral(literal) => semantic_node(
            crate::ROUP_NODE_FAMILY_OMP_FOREIGN_RUNTIME_IDENTIFIER,
            crate::ROUP_OMP_FOREIGN_RUNTIME_STRING,
            vec![ClauseField::node(
                crate::ROUP_FIELD_VALUE,
                "literal",
                string_literal_node(literal),
            )],
        ),
        roup::ir::OmpForeignRuntimeIdentifier::ConstantExpression(expression) => semantic_node(
            crate::ROUP_NODE_FAMILY_OMP_FOREIGN_RUNTIME_IDENTIFIER,
            crate::ROUP_OMP_FOREIGN_RUNTIME_EXPRESSION,
            vec![ClauseField::string(
                crate::ROUP_FIELD_VALUE,
                "expression",
                expression,
            )],
        ),
    }
}

fn string_literal_node(literal: &roup::host::StringLiteral) -> NodeRecord {
    semantic_node(
        crate::ROUP_NODE_FAMILY_STRING_LITERAL,
        crate::ROUP_STRING_LITERAL,
        vec![
            ClauseField::string(crate::ROUP_FIELD_VALUE, "value", literal.value.as_str()),
            ClauseField::u32(
                crate::ROUP_FIELD_ENCODING,
                "encoding",
                character_encoding(literal.encoding),
            ),
        ],
    )
}

fn induction_type_specifier_node(specifier: &roup::ast::OmpInductionTypeSpecifier) -> NodeRecord {
    match specifier {
        roup::ast::OmpInductionTypeSpecifier::Same(type_name) => semantic_node(
            crate::ROUP_NODE_FAMILY_OMP_INDUCTION_TYPE,
            crate::ROUP_INDUCTION_TYPE_SAME,
            vec![ClauseField::string(
                crate::ROUP_FIELD_TYPE_NAME,
                "type_name",
                type_name,
            )],
        ),
        roup::ast::OmpInductionTypeSpecifier::Pair { variable, step } => semantic_node(
            crate::ROUP_NODE_FAMILY_OMP_INDUCTION_TYPE,
            crate::ROUP_INDUCTION_TYPE_PAIR,
            vec![
                ClauseField::string(crate::ROUP_FIELD_VARIABLE_TYPE, "variable_type", variable),
                ClauseField::string(crate::ROUP_FIELD_STEP_TYPE, "step_type", step),
            ],
        ),
    }
}

fn require_modifier_node(requirement: &roup::ir::RequireModifier) -> UnrecordedResult<NodeRecord> {
    let (variant, fields) = match requirement {
        roup::ir::RequireModifier::ReverseOffload => {
            (crate::ROUP_REQUIRE_REVERSE_OFFLOAD, Vec::new())
        }
        roup::ir::RequireModifier::UnifiedAddress => {
            (crate::ROUP_REQUIRE_UNIFIED_ADDRESS, Vec::new())
        }
        roup::ir::RequireModifier::UnifiedSharedMemory => {
            (crate::ROUP_REQUIRE_UNIFIED_SHARED_MEMORY, Vec::new())
        }
        roup::ir::RequireModifier::DynamicAllocators => {
            (crate::ROUP_REQUIRE_DYNAMIC_ALLOCATORS, Vec::new())
        }
        roup::ir::RequireModifier::SelfMaps => (crate::ROUP_REQUIRE_SELF_MAPS, Vec::new()),
        roup::ir::RequireModifier::DeviceSafesync => {
            (crate::ROUP_REQUIRE_DEVICE_SAFESYNC, Vec::new())
        }
        roup::ir::RequireModifier::AtomicDefaultMemOrder(order) => (
            crate::ROUP_REQUIRE_ATOMIC_DEFAULT_MEM_ORDER,
            vec![ClauseField::u32(
                crate::ROUP_FIELD_MEMORY_ORDER,
                "memory_order",
                omp_memory_order(*order),
            )],
        ),
        roup::ir::RequireModifier::ExtImplementationDefinedRequirement(Some(identifier)) => (
            crate::ROUP_REQUIRE_EXTENSION,
            vec![ClauseField::string(
                crate::ROUP_FIELD_VALUE,
                "identifier",
                identifier,
            )],
        ),
        roup::ir::RequireModifier::ExtImplementationDefinedRequirement(None) => {
            return Err(internal_failure(
                "strict OpenMP parser produced an extension requirement without an identifier",
            ));
        }
    };
    Ok(semantic_node(
        crate::ROUP_NODE_FAMILY_REQUIRE_MODIFIER,
        variant,
        fields,
    ))
}

fn reduction_modifier_node(modifier: &roup::ir::ReductionModifier) -> NodeRecord {
    let (variant, fields) = match modifier {
        roup::ir::ReductionModifier::Task => (crate::ROUP_REDUCTION_MODIFIER_TASK, Vec::new()),
        roup::ir::ReductionModifier::Inscan => (crate::ROUP_REDUCTION_MODIFIER_INSCAN, Vec::new()),
        roup::ir::ReductionModifier::Default => {
            (crate::ROUP_REDUCTION_MODIFIER_DEFAULT, Vec::new())
        }
        roup::ir::ReductionModifier::Original(sharing) => (
            crate::ROUP_REDUCTION_MODIFIER_ORIGINAL,
            vec![ClauseField::u32(
                crate::ROUP_FIELD_KIND,
                "sharing",
                omp_original_sharing(*sharing),
            )],
        ),
    };
    semantic_node(crate::ROUP_NODE_FAMILY_REDUCTION_MODIFIER, variant, fields)
}

fn depend_iterator_node(iterator: &roup::ir::DependIterator) -> NodeRecord {
    let mut fields = Vec::new();
    push_optional_string(
        &mut fields,
        crate::ROUP_FIELD_TYPE_NAME,
        "type_name",
        iterator.type_name(),
    );
    fields.push(ClauseField::string(
        crate::ROUP_FIELD_VARIABLE,
        "name",
        iterator.name(),
    ));
    fields.push(ClauseField::string(
        crate::ROUP_FIELD_START,
        "start",
        iterator.start(),
    ));
    fields.push(ClauseField::string(
        crate::ROUP_FIELD_END,
        "end",
        iterator.end(),
    ));
    push_optional_string(&mut fields, crate::ROUP_FIELD_STEP, "step", iterator.step());
    semantic_node(
        crate::ROUP_NODE_FAMILY_DEPEND_ITERATOR,
        crate::ROUP_NODE_RECORD,
        fields,
    )
}

fn uses_allocator_node(entry: &roup::ir::UsesAllocatorSpec) -> UnrecordedResult<NodeRecord> {
    let mut fields = vec![ClauseField::node(
        crate::ROUP_FIELD_ALLOCATOR,
        "allocator",
        omp_allocator_kind_node(entry.allocator()),
    )];
    push_optional_string(
        &mut fields,
        crate::ROUP_FIELD_TRAITS,
        "traits",
        entry.traits(),
    );
    if let Some(memspace) = entry.memspace() {
        fields.push(ClauseField::u32(
            crate::ROUP_FIELD_MEMSPACE,
            "memspace",
            match memspace {
                roup::ir::OmpMemorySpace::Default => crate::ROUP_OMP_MEMORY_SPACE_DEFAULT,
                roup::ir::OmpMemorySpace::LargeCap => crate::ROUP_OMP_MEMORY_SPACE_LARGE_CAP,
                roup::ir::OmpMemorySpace::Const => crate::ROUP_OMP_MEMORY_SPACE_CONST,
                roup::ir::OmpMemorySpace::HighBw => crate::ROUP_OMP_MEMORY_SPACE_HIGH_BW,
                roup::ir::OmpMemorySpace::LowLat => crate::ROUP_OMP_MEMORY_SPACE_LOW_LAT,
            },
        ));
    }
    Ok(semantic_node(
        crate::ROUP_NODE_FAMILY_USES_ALLOCATOR,
        crate::ROUP_NODE_RECORD,
        fields,
    ))
}

fn omp_directive_node(directive: &OmpDirective) -> UnrecordedResult<NodeRecord> {
    let variant = omp_directive_kind_ordinal(directive.kind());
    let mut fields = Vec::new();
    if directive.parameter().is_some() {
        fields.push(ClauseField::node(
            crate::ROUP_FIELD_PARAMETER,
            "parameter",
            omp_parameter_node(directive)?,
        ));
    }
    fields.push(ClauseField::nodes(
        crate::ROUP_FIELD_CLAUSES,
        "clauses",
        directive
            .clauses()
            .iter()
            .map(omp_clause_node)
            .collect::<UnrecordedResult<Vec<_>>>()?,
    ));
    Ok(semantic_node(
        crate::ROUP_NODE_FAMILY_OMP_DIRECTIVE,
        variant,
        fields,
    ))
}

fn omp_clause_node(clause: &OmpClause) -> UnrecordedResult<NodeRecord> {
    let variant = omp_clause_kind_ordinal(clause.kind());
    let fields = omp_clause_fields(clause)?;
    Ok(semantic_node(
        crate::ROUP_NODE_FAMILY_OMP_CLAUSE,
        variant,
        fields,
    ))
}

fn omp_clause_fields(clause: &OmpClause) -> UnrecordedResult<Vec<ClauseField>> {
    let mut fields = Vec::new();
    if let Some(modifier) = clause.directive_name_modifier() {
        fields.push(ClauseField::u32(
            crate::ROUP_FIELD_DIRECTIVE_NAME_MODIFIER,
            "directive_name_modifier",
            omp_directive_kind_ordinal(modifier),
        ));
    }
    fields.extend(omp_fields(clause.payload())?);
    Ok(fields)
}

fn omp_parameter_node(directive: &OmpDirective) -> UnrecordedResult<NodeRecord> {
    let parameter = directive.parameter().ok_or_else(no_parameter_failure)?;
    let variant = omp_parameter_variant(parameter);
    let wrapper = RoupDirective::OpenMp(Box::new(directive.clone()));
    Ok(semantic_node(
        crate::ROUP_NODE_FAMILY_OMP_PARAMETER,
        variant,
        parameter_fields(&wrapper)?,
    ))
}

fn omp_parameter_variant(parameter: &roup::ast::OmpDirectiveParameter) -> u32 {
    match parameter {
        roup::ast::OmpDirectiveParameter::AllocateList(_) => {
            crate::ROUP_OMP_PARAMETER_ALLOCATE_LIST
        }
        roup::ast::OmpDirectiveParameter::ThreadprivateList(_) => {
            crate::ROUP_OMP_PARAMETER_THREADPRIVATE_LIST
        }
        roup::ast::OmpDirectiveParameter::GroupprivateList(_) => {
            crate::ROUP_OMP_PARAMETER_GROUPPRIVATE_LIST
        }
        roup::ast::OmpDirectiveParameter::DeclareTargetList(_) => {
            crate::ROUP_OMP_PARAMETER_DECLARE_TARGET_LIST
        }
        roup::ast::OmpDirectiveParameter::DeclareMapper(_) => {
            crate::ROUP_OMP_PARAMETER_DECLARE_MAPPER
        }
        roup::ast::OmpDirectiveParameter::DeclareVariant(_) => {
            crate::ROUP_OMP_PARAMETER_DECLARE_VARIANT
        }
        roup::ast::OmpDirectiveParameter::Depobj(_) => crate::ROUP_OMP_PARAMETER_DEPOBJ,
        roup::ast::OmpDirectiveParameter::Construct(_) => crate::ROUP_OMP_PARAMETER_CONSTRUCT,
        roup::ast::OmpDirectiveParameter::CriticalSection(_) => {
            crate::ROUP_OMP_PARAMETER_CRITICAL_SECTION
        }
        roup::ast::OmpDirectiveParameter::FlushList(_) => crate::ROUP_OMP_PARAMETER_FLUSH_LIST,
        roup::ast::OmpDirectiveParameter::DeclareReduction(_) => {
            crate::ROUP_OMP_PARAMETER_DECLARE_REDUCTION
        }
        roup::ast::OmpDirectiveParameter::DeclareInduction(_) => {
            crate::ROUP_OMP_PARAMETER_DECLARE_INDUCTION
        }
        roup::ast::OmpDirectiveParameter::DeclareSimd(_) => crate::ROUP_OMP_PARAMETER_DECLARE_SIMD,
    }
}

fn selector_fields(selector: &roup::ast::OmpSelector) -> UnrecordedResult<Vec<ClauseField>> {
    let mut fields = vec![ClauseField::nodes(
        crate::ROUP_FIELD_ENTRIES,
        "selector_entries",
        selector
            .entries()
            .iter()
            .map(selector_entry_node)
            .collect::<UnrecordedResult<Vec<_>>>()?,
    )];
    if let Some(nested) = selector.nested_directive() {
        fields.push(ClauseField::node(
            crate::ROUP_FIELD_NESTED_DIRECTIVE,
            "nested_directive",
            omp_directive_node(nested)?,
        ));
    }
    Ok(fields)
}

fn selector_entry_node(entry: &roup::ast::OmpSelectorEntry) -> UnrecordedResult<NodeRecord> {
    let (variant, fields) = match entry {
        roup::ast::OmpSelectorEntry::Device { traits } => (
            crate::ROUP_SELECTOR_ENTRY_DEVICE,
            vec![ClauseField::nodes(
                crate::ROUP_FIELD_TRAITS,
                "traits",
                traits.iter().map(selector_device_trait_node).collect(),
            )],
        ),
        roup::ast::OmpSelectorEntry::TargetDevice { traits } => (
            crate::ROUP_SELECTOR_ENTRY_TARGET_DEVICE,
            vec![ClauseField::nodes(
                crate::ROUP_FIELD_TRAITS,
                "traits",
                traits.iter().map(selector_device_trait_node).collect(),
            )],
        ),
        roup::ast::OmpSelectorEntry::Implementation { traits } => (
            crate::ROUP_SELECTOR_ENTRY_IMPLEMENTATION,
            vec![ClauseField::nodes(
                crate::ROUP_FIELD_TRAITS,
                "traits",
                traits
                    .iter()
                    .map(selector_implementation_trait_node)
                    .collect::<UnrecordedResult<Vec<_>>>()?,
            )],
        ),
        roup::ast::OmpSelectorEntry::User { score, condition } => {
            let mut fields = Vec::with_capacity(2);
            if let Some(score) = score {
                fields.push(ClauseField::string(crate::ROUP_FIELD_SCORE, "score", score));
            }
            fields.push(ClauseField::string(
                crate::ROUP_FIELD_CONDITION,
                "condition",
                condition,
            ));
            (crate::ROUP_SELECTOR_ENTRY_USER, fields)
        }
        roup::ast::OmpSelectorEntry::Construct { constructs } => (
            crate::ROUP_SELECTOR_ENTRY_CONSTRUCT,
            vec![ClauseField::nodes(
                crate::ROUP_FIELD_ITEMS,
                "constructs",
                constructs
                    .iter()
                    .map(selector_construct_node)
                    .collect::<UnrecordedResult<Vec<_>>>()?,
            )],
        ),
    };
    Ok(semantic_node(
        crate::ROUP_NODE_FAMILY_OMP_SELECTOR_ENTRY,
        variant,
        fields,
    ))
}

fn selector_trait_value_node(value: &roup::ast::OmpSelectorTraitValue) -> NodeRecord {
    let (variant, fields) = match value {
        roup::ast::OmpSelectorTraitValue::Identifier(identifier) => (
            crate::ROUP_SELECTOR_TRAIT_IDENTIFIER,
            vec![ClauseField::string(
                crate::ROUP_FIELD_VALUE,
                "identifier",
                identifier,
            )],
        ),
        roup::ast::OmpSelectorTraitValue::StringLiteral(literal) => (
            crate::ROUP_SELECTOR_TRAIT_STRING_LITERAL,
            vec![
                ClauseField::string(crate::ROUP_FIELD_VALUE, "value", literal.value.as_str()),
                ClauseField::u32(
                    crate::ROUP_FIELD_ENCODING,
                    "encoding",
                    character_encoding(literal.encoding),
                ),
            ],
        ),
    };
    semantic_node(
        crate::ROUP_NODE_FAMILY_OMP_SELECTOR_TRAIT_VALUE,
        variant,
        fields,
    )
}

fn selector_device_trait_node(value: &roup::ast::OmpSelectorDeviceTrait) -> NodeRecord {
    let (variant, fields) = match value {
        roup::ast::OmpSelectorDeviceTrait::NameList(name_list) => (
            crate::ROUP_SELECTOR_DEVICE_NAME_LIST,
            vec![ClauseField::node(
                crate::ROUP_FIELD_TRAIT_NAME,
                "name_list_trait",
                selector_name_list_trait_node(name_list),
            )],
        ),
        roup::ast::OmpSelectorDeviceTrait::DeviceNum(expression) => (
            crate::ROUP_SELECTOR_DEVICE_NUM,
            vec![ClauseField::string(
                crate::ROUP_FIELD_VALUE,
                "device_num",
                expression,
            )],
        ),
        roup::ast::OmpSelectorDeviceTrait::Uid(uid) => (
            crate::ROUP_SELECTOR_DEVICE_UID,
            vec![ClauseField::node(
                crate::ROUP_FIELD_VALUE,
                "uid",
                selector_trait_value_node(uid),
            )],
        ),
        roup::ast::OmpSelectorDeviceTrait::Extension(extension) => (
            crate::ROUP_SELECTOR_DEVICE_EXTENSION,
            vec![ClauseField::node(
                crate::ROUP_FIELD_TRAIT_NAME,
                "extension_trait",
                selector_extension_trait_node(extension),
            )],
        ),
    };
    semantic_node(
        crate::ROUP_NODE_FAMILY_OMP_SELECTOR_DEVICE_TRAIT,
        variant,
        fields,
    )
}

fn selector_implementation_trait_node(
    value: &roup::ast::OmpSelectorImplementationTrait,
) -> UnrecordedResult<NodeRecord> {
    let mut fields = Vec::new();
    push_optional_string(&mut fields, crate::ROUP_FIELD_SCORE, "score", value.score());
    let variant = match value.kind() {
        roup::ast::OmpSelectorImplementationTraitKind::NameList(name_list) => {
            fields.push(ClauseField::node(
                crate::ROUP_FIELD_TRAIT_NAME,
                "name_list_trait",
                selector_name_list_trait_node(name_list),
            ));
            crate::ROUP_SELECTOR_IMPLEMENTATION_NAME_LIST
        }
        roup::ast::OmpSelectorImplementationTraitKind::AtomicDefaultMemOrder(order) => {
            fields.push(ClauseField::u32(
                crate::ROUP_FIELD_MEMORY_ORDER,
                "memory_order",
                omp_memory_order(*order),
            ));
            crate::ROUP_SELECTOR_IMPLEMENTATION_ATOMIC_DEFAULT_MEM_ORDER
        }
        roup::ast::OmpSelectorImplementationTraitKind::Requirement(requirement) => {
            fields.push(ClauseField::node(
                crate::ROUP_FIELD_REQUIREMENT,
                "requirement",
                require_modifier_node(requirement)?,
            ));
            crate::ROUP_SELECTOR_IMPLEMENTATION_REQUIREMENT
        }
        roup::ast::OmpSelectorImplementationTraitKind::Requires(requirements) => {
            fields.push(ClauseField::nodes(
                crate::ROUP_FIELD_PROPERTIES,
                "requirements",
                requirements
                    .iter()
                    .map(selector_requirement_node)
                    .collect::<UnrecordedResult<Vec<_>>>()?,
            ));
            crate::ROUP_SELECTOR_IMPLEMENTATION_REQUIRES
        }
        roup::ast::OmpSelectorImplementationTraitKind::Extension(extension) => {
            fields.push(ClauseField::node(
                crate::ROUP_FIELD_TRAIT_NAME,
                "extension_trait",
                selector_extension_trait_node(extension),
            ));
            crate::ROUP_SELECTOR_IMPLEMENTATION_EXTENSION
        }
    };
    Ok(semantic_node(
        crate::ROUP_NODE_FAMILY_OMP_SELECTOR_IMPLEMENTATION_TRAIT,
        variant,
        fields,
    ))
}

fn selector_name_list_trait_node(value: &roup::ast::OmpSelectorNameListTrait) -> NodeRecord {
    let variant = match value.kind() {
        roup::ast::OmpSelectorNameListKind::Kind => crate::ROUP_SELECTOR_NAME_LIST_KIND,
        roup::ast::OmpSelectorNameListKind::Isa => crate::ROUP_SELECTOR_NAME_LIST_ISA,
        roup::ast::OmpSelectorNameListKind::Arch => crate::ROUP_SELECTOR_NAME_LIST_ARCH,
        roup::ast::OmpSelectorNameListKind::Vendor => crate::ROUP_SELECTOR_NAME_LIST_VENDOR,
        roup::ast::OmpSelectorNameListKind::Extension => crate::ROUP_SELECTOR_NAME_LIST_EXTENSION,
    };
    semantic_node(
        crate::ROUP_NODE_FAMILY_OMP_SELECTOR_NAME_LIST_TRAIT,
        variant,
        vec![ClauseField::nodes(
            crate::ROUP_FIELD_PROPERTIES,
            "properties",
            value
                .properties()
                .iter()
                .map(selector_trait_value_node)
                .collect(),
        )],
    )
}

fn selector_extension_trait_node(value: &roup::ast::OmpSelectorExtensionTrait) -> NodeRecord {
    semantic_node(
        crate::ROUP_NODE_FAMILY_OMP_SELECTOR_EXTENSION_TRAIT,
        crate::ROUP_SELECTOR_EXTENSION_TRAIT,
        vec![
            ClauseField::string(crate::ROUP_FIELD_NAME, "name", value.name()),
            ClauseField::nodes(
                crate::ROUP_FIELD_PROPERTIES,
                "properties",
                value
                    .properties()
                    .iter()
                    .map(selector_extension_property_node)
                    .collect(),
            ),
        ],
    )
}

fn selector_extension_property_node(value: &roup::ast::OmpSelectorExtensionProperty) -> NodeRecord {
    let (variant, fields) = match value {
        roup::ast::OmpSelectorExtensionProperty::Name(name) => (
            crate::ROUP_SELECTOR_EXTENSION_PROPERTY_NAME,
            vec![ClauseField::node(
                crate::ROUP_FIELD_VALUE,
                "name",
                selector_trait_value_node(name),
            )],
        ),
        roup::ast::OmpSelectorExtensionProperty::Call { name, properties } => (
            crate::ROUP_SELECTOR_EXTENSION_PROPERTY_CALL,
            vec![
                ClauseField::string(crate::ROUP_FIELD_NAME, "name", name),
                ClauseField::nodes(
                    crate::ROUP_FIELD_PROPERTIES,
                    "properties",
                    properties
                        .iter()
                        .map(selector_extension_property_node)
                        .collect(),
                ),
            ],
        ),
        roup::ast::OmpSelectorExtensionProperty::ConstantInteger(expression) => (
            crate::ROUP_SELECTOR_EXTENSION_PROPERTY_CONSTANT_INTEGER,
            vec![ClauseField::string(
                crate::ROUP_FIELD_VALUE,
                "constant_integer",
                expression,
            )],
        ),
    };
    semantic_node(
        crate::ROUP_NODE_FAMILY_OMP_SELECTOR_EXTENSION_PROPERTY,
        variant,
        fields,
    )
}

fn selector_requirement_node(
    value: &roup::ast::OmpSelectorRequirement,
) -> UnrecordedResult<NodeRecord> {
    let mut fields = vec![ClauseField::node(
        crate::ROUP_FIELD_REQUIREMENT,
        "requirement",
        require_modifier_node(value.requirement())?,
    )];
    push_optional_string(
        &mut fields,
        crate::ROUP_FIELD_REQUIRED,
        "required",
        value.required(),
    );
    Ok(semantic_node(
        crate::ROUP_NODE_FAMILY_OMP_SELECTOR_REQUIREMENT,
        crate::ROUP_SELECTOR_REQUIREMENT,
        fields,
    ))
}

fn selector_construct_node(
    construct: &roup::ast::OmpSelectorConstruct,
) -> UnrecordedResult<NodeRecord> {
    let fields = vec![ClauseField::node(
        crate::ROUP_FIELD_NESTED_DIRECTIVE,
        "directive",
        omp_directive_node(construct.directive())?,
    )];
    Ok(semantic_node(
        crate::ROUP_NODE_FAMILY_OMP_SELECTOR_CONSTRUCT,
        crate::ROUP_SELECTOR_CONSTRUCT,
        fields,
    ))
}

fn acc_data_modifier(modifier: &AccDataModifier) -> u32 {
    match modifier {
        AccDataModifier::Always => crate::ROUP_ACC_DATA_MODIFIER_ALWAYS,
        AccDataModifier::AlwaysIn => crate::ROUP_ACC_DATA_MODIFIER_ALWAYS_IN,
        AccDataModifier::AlwaysOut => crate::ROUP_ACC_DATA_MODIFIER_ALWAYS_OUT,
        AccDataModifier::Capture => crate::ROUP_ACC_DATA_MODIFIER_CAPTURE,
        AccDataModifier::Readonly => crate::ROUP_ACC_DATA_MODIFIER_READONLY,
        AccDataModifier::Zero => crate::ROUP_ACC_DATA_MODIFIER_ZERO,
    }
}

fn acc_data_kind(kind: roup::ast::AccDataKind) -> u32 {
    match kind {
        roup::ast::AccDataKind::Attach => crate::ROUP_ACC_DATA_ATTACH,
        roup::ast::AccDataKind::Detach => crate::ROUP_ACC_DATA_DETACH,
        roup::ast::AccDataKind::UseDevice => crate::ROUP_ACC_DATA_USE_DEVICE,
        roup::ast::AccDataKind::Link => crate::ROUP_ACC_DATA_LINK,
        roup::ast::AccDataKind::DeviceResident => crate::ROUP_ACC_DATA_DEVICE_RESIDENT,
        roup::ast::AccDataKind::Device => crate::ROUP_ACC_DATA_DEVICE,
        roup::ast::AccDataKind::Delete => crate::ROUP_ACC_DATA_DELETE,
    }
}

fn acc_device_type_node(value: &AccDeviceType) -> NodeRecord {
    let (variant, fields) = match value {
        AccDeviceType::Host => (crate::ROUP_ACC_DEVICE_TYPE_HOST, Vec::new()),
        AccDeviceType::Wildcard => (crate::ROUP_ACC_DEVICE_TYPE_WILDCARD, Vec::new()),
        AccDeviceType::Multicore => (crate::ROUP_ACC_DEVICE_TYPE_MULTICORE, Vec::new()),
        AccDeviceType::Default => (crate::ROUP_ACC_DEVICE_TYPE_DEFAULT, Vec::new()),
        AccDeviceType::Named(identifier) => (
            crate::ROUP_ACC_DEVICE_TYPE_NAMED,
            vec![ClauseField::string(
                crate::ROUP_FIELD_NAME,
                "name",
                identifier,
            )],
        ),
    };
    semantic_node(crate::ROUP_NODE_FAMILY_ACC_DEVICE_TYPE, variant, fields)
}

fn acc_reduction_operator_node(operator: &AccReductionOperator) -> NodeRecord {
    let (variant, fields) = match operator {
        AccReductionOperator::Add => (crate::ROUP_ACC_REDUCTION_ADD, Vec::new()),
        AccReductionOperator::Mul => (crate::ROUP_ACC_REDUCTION_MUL, Vec::new()),
        AccReductionOperator::Max => (crate::ROUP_ACC_REDUCTION_MAX, Vec::new()),
        AccReductionOperator::Min => (crate::ROUP_ACC_REDUCTION_MIN, Vec::new()),
        AccReductionOperator::BitAnd => (crate::ROUP_ACC_REDUCTION_BIT_AND, Vec::new()),
        AccReductionOperator::BitOr => (crate::ROUP_ACC_REDUCTION_BIT_OR, Vec::new()),
        AccReductionOperator::BitXor => (crate::ROUP_ACC_REDUCTION_BIT_XOR, Vec::new()),
        AccReductionOperator::LogAnd => (crate::ROUP_ACC_REDUCTION_LOG_AND, Vec::new()),
        AccReductionOperator::LogOr => (crate::ROUP_ACC_REDUCTION_LOG_OR, Vec::new()),
        AccReductionOperator::FortAnd => (crate::ROUP_ACC_REDUCTION_FORTRAN_AND, Vec::new()),
        AccReductionOperator::FortOr => (crate::ROUP_ACC_REDUCTION_FORTRAN_OR, Vec::new()),
        AccReductionOperator::FortEqv => (crate::ROUP_ACC_REDUCTION_FORTRAN_EQV, Vec::new()),
        AccReductionOperator::FortNeqv => (crate::ROUP_ACC_REDUCTION_FORTRAN_NEQV, Vec::new()),
        AccReductionOperator::FortIand => (crate::ROUP_ACC_REDUCTION_FORTRAN_IAND, Vec::new()),
        AccReductionOperator::FortIor => (crate::ROUP_ACC_REDUCTION_FORTRAN_IOR, Vec::new()),
        AccReductionOperator::FortIeor => (crate::ROUP_ACC_REDUCTION_FORTRAN_IEOR, Vec::new()),
    };
    semantic_node(
        crate::ROUP_NODE_FAMILY_ACC_REDUCTION_OPERATOR,
        variant,
        fields,
    )
}

fn character_encoding(encoding: roup::host::CharacterEncoding) -> u32 {
    match encoding {
        roup::host::CharacterEncoding::Ordinary => crate::ROUP_CHARACTER_ENCODING_ORDINARY,
        roup::host::CharacterEncoding::Utf8 => crate::ROUP_CHARACTER_ENCODING_UTF8,
        roup::host::CharacterEncoding::Utf16 => crate::ROUP_CHARACTER_ENCODING_UTF16,
        roup::host::CharacterEncoding::Utf32 => crate::ROUP_CHARACTER_ENCODING_UTF32,
        roup::host::CharacterEncoding::Wide => crate::ROUP_CHARACTER_ENCODING_WIDE,
        roup::host::CharacterEncoding::Fortran => crate::ROUP_CHARACTER_ENCODING_FORTRAN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::fmt::Debug;

    fn openmp_options() -> RoupParserOptions {
        RoupParserOptions {
            abi_version: ROUP_ABI_VERSION,
            struct_size: core::mem::size_of::<RoupParserOptions>() as u32,
            dialect: ROUP_DIALECT_OPENMP,
            version_policy: ROUP_VERSION_ANY,
            version: 0,
            host_language: ROUP_HOST_C,
            host_standard: 23,
            source_form: ROUP_SOURCE_PRAGMA,
            flags: 0,
            reserved: [0; 3],
        }
    }

    fn openmp_fortran_options() -> RoupParserOptions {
        let mut options = openmp_options();
        options.host_language = ROUP_HOST_FORTRAN;
        options.host_standard = 2023;
        options.source_form = ROUP_SOURCE_FORTRAN_FREE;
        options
    }

    fn openacc_options() -> RoupParserOptions {
        let mut options = openmp_options();
        options.dialect = ROUP_DIALECT_OPENACC;
        options
    }

    fn only_field(fields: &[ClauseField], id: u32) -> &FieldValue {
        let matches = fields
            .iter()
            .filter(|field| field.id == id)
            .collect::<Vec<_>>();
        assert_eq!(matches.len(), 1, "expected one field with id {id}");
        &matches[0].value
    }

    fn openmp_parameter_projection(options: RoupParserOptions, source: &str) -> Vec<ClauseField> {
        let parser = create_parser(options).unwrap();
        let directive = parse(parser, source.to_owned()).unwrap();
        let fields =
            with_directive(directive, |record| parameter_fields(&record.directive)).unwrap();
        release_directive(directive).unwrap();
        release_parser(parser).unwrap();
        fields
    }

    fn projected_clause_fields(
        options: RoupParserOptions,
        source: &str,
    ) -> Vec<(u32, Vec<ClauseField>)> {
        let parser = create_parser(options).unwrap();
        let directive = parse(parser, source.to_owned()).unwrap();
        let fields = with_directive(directive, |record| match &record.directive {
            RoupDirective::OpenMp(directive) => directive
                .clauses()
                .iter()
                .map(|clause| {
                    Ok((
                        omp_clause_kind_ordinal(clause.kind()),
                        omp_clause_fields(clause)?,
                    ))
                })
                .collect::<UnrecordedResult<Vec<_>>>(),
            RoupDirective::OpenAcc(directive) => directive
                .clauses()
                .iter()
                .map(|clause| {
                    Ok((
                        acc_clause_kind_ordinal(clause.kind()),
                        acc_fields(clause.payload())?,
                    ))
                })
                .collect::<UnrecordedResult<Vec<_>>>(),
        })
        .unwrap();
        release_directive(directive).unwrap();
        release_parser(parser).unwrap();
        fields
    }

    #[test]
    fn invalid_configuration_returns_a_queryable_error() {
        let mut options = openmp_options();
        options.host_standard = 1234;
        let failure = create_parser(options).unwrap_err();

        assert_eq!(failure.status, RoupStatus::INVALID_ARGUMENT);
        assert_eq!(error_code(failure.error).unwrap(), 1000);
        assert!(error_message(failure.error)
            .unwrap()
            .contains("unknown C language standard"));
        release_error(failure.error).unwrap();
    }

    #[test]
    fn stale_parser_handles_are_hard_errors() {
        let parser = create_parser(openmp_options()).unwrap();
        release_parser(parser).unwrap();
        let failure = release_parser(parser).unwrap_err();

        assert_eq!(failure.status, RoupStatus::STALE_HANDLE);
        assert_eq!(
            error_code(failure.error).unwrap(),
            ROUP_DIAGNOSTIC_INVALID_HANDLE
        );
        release_error(failure.error).unwrap();
    }

    #[test]
    fn every_handle_family_has_server_owned_identity_and_rejects_retagging() {
        let parser = create_parser(openmp_options()).unwrap();
        let directive = parse(parser, "#pragma omp parallel".to_owned()).unwrap();
        let node = store_test_u32_node().unwrap();
        let mut invalid_options = openmp_options();
        invalid_options.host_standard = 1234;
        let diagnostic = create_parser(invalid_options).unwrap_err().error;

        let identities = [parser.index, directive.index, node.index, diagnostic.index];
        assert_eq!(identities.iter().copied().collect::<HashSet<_>>().len(), 4);

        fn assert_invalid<T>(result: ServiceResult<T>) {
            let failure = result.err().expect("retagged handle must fail");
            assert_eq!(failure.status, RoupStatus::INVALID_HANDLE);
            release_error(failure.error).unwrap();
        }

        let parser_as_directive = RoupDirectiveHandle {
            generation: parser.generation,
            index: parser.index,
        };
        let parser_as_node = RoupNodeHandle {
            generation: parser.generation,
            index: parser.index,
        };
        let parser_as_error = RoupErrorHandle {
            generation: parser.generation,
            index: parser.index,
        };
        assert_invalid(release_directive(parser_as_directive));
        assert_invalid(release_node(parser_as_node));
        assert_invalid(release_error(parser_as_error));

        let directive_as_parser = RoupParserHandle {
            generation: directive.generation,
            index: directive.index,
        };
        let directive_as_node = RoupNodeHandle {
            generation: directive.generation,
            index: directive.index,
        };
        let directive_as_error = RoupErrorHandle {
            generation: directive.generation,
            index: directive.index,
        };
        assert_invalid(release_parser(directive_as_parser));
        assert_invalid(release_node(directive_as_node));
        assert_invalid(release_error(directive_as_error));

        let node_as_parser = RoupParserHandle {
            generation: node.generation,
            index: node.index,
        };
        let node_as_directive = RoupDirectiveHandle {
            generation: node.generation,
            index: node.index,
        };
        let node_as_error = RoupErrorHandle {
            generation: node.generation,
            index: node.index,
        };
        assert_invalid(release_parser(node_as_parser));
        assert_invalid(release_directive(node_as_directive));
        assert_invalid(release_error(node_as_error));

        let error_as_parser = RoupParserHandle {
            generation: diagnostic.generation,
            index: diagnostic.index,
        };
        let error_as_directive = RoupDirectiveHandle {
            generation: diagnostic.generation,
            index: diagnostic.index,
        };
        let error_as_node = RoupNodeHandle {
            generation: diagnostic.generation,
            index: diagnostic.index,
        };
        assert_invalid(release_parser(error_as_parser));
        assert_invalid(release_directive(error_as_directive));
        assert_invalid(release_node(error_as_node));

        assert_eq!(directive_dialect(directive).unwrap(), ROUP_DIALECT_OPENMP);
        assert_eq!(node_kind(node).unwrap().family, u32::MAX);
        assert_eq!(error_code(diagnostic).unwrap(), 1000);

        let oversized = RoupParserHandle {
            generation: 1,
            index: u64::MAX,
        };
        assert_invalid(release_parser(oversized));

        release_error(diagnostic).unwrap();
        release_node(node).unwrap();
        release_directive(directive).unwrap();
        release_parser(parser).unwrap();
    }

    #[test]
    fn u32_scalar_and_list_fields_have_strict_metadata_and_access() {
        let scalar = ClauseField::u32(crate::ROUP_FIELD_VALUE, "scalar", 17);
        assert_eq!(scalar.info().value_kind, crate::ROUP_FIELD_VALUE_U32);
        assert_eq!(scalar.info().count, 1);
        assert_eq!(scalar_field_u32(&scalar, 0).unwrap(), 17);
        assert!(scalar_field_u32(&scalar, 1).is_err());
        assert!(scalar_field_bool(&scalar, 0).is_err());

        let boolean = ClauseField::boolean(crate::ROUP_FIELD_FORCE, "boolean", true);
        assert_eq!(boolean.info().value_kind, crate::ROUP_FIELD_VALUE_BOOL);
        assert_eq!(scalar_field_bool(&boolean, 0).unwrap(), 1);
        assert!(scalar_field_bool(&boolean, 1).is_err());
        assert!(scalar_field_u32(&boolean, 0).is_err());

        let values = ClauseField::u32s(crate::ROUP_FIELD_VALUES, "values", vec![3, 5, 8]);
        assert_eq!(values.info().value_kind, crate::ROUP_FIELD_VALUE_U32_LIST);
        assert_eq!(values.info().count, 3);
        assert_eq!(scalar_field_u32(&values, 0).unwrap(), 3);
        assert_eq!(scalar_field_u32(&values, 2).unwrap(), 8);
        assert!(scalar_field_u32(&values, 3).is_err());
        assert!(scalar_field_string(&values, 0).is_err());
        assert!(scalar_field_node(&values, 0).is_err());
    }

    fn screaming_snake(debug_name: &str) -> String {
        let chars = debug_name.chars().collect::<Vec<_>>();
        let mut result = String::new();
        for (index, ch) in chars.iter().copied().enumerate() {
            if ch.is_ascii_uppercase() && index != 0 {
                let previous = chars[index - 1];
                let next_is_lower = chars
                    .get(index + 1)
                    .is_some_and(|next| next.is_ascii_lowercase());
                if previous.is_ascii_lowercase()
                    || previous.is_ascii_digit()
                    || (previous.is_ascii_uppercase() && next_is_lower)
                {
                    result.push('_');
                }
            }
            result.push(ch.to_ascii_uppercase());
        }
        result
    }

    fn header_schema(prefix: &str) -> Vec<(String, u32)> {
        include_str!("../include/roup.h")
            .lines()
            .filter_map(|line| {
                let definition = line.strip_prefix("#define ")?;
                let mut parts = definition.split_whitespace();
                let name = parts.next()?;
                if !name.starts_with(prefix) {
                    return None;
                }
                let encoded = parts.next()?;
                if parts.next().is_some() {
                    panic!("unexpected tokens in schema definition {line}");
                }
                let ordinal = encoded
                    .strip_prefix("UINT32_C(")
                    .and_then(|value| value.strip_suffix(')'))
                    .unwrap_or_else(|| panic!("invalid schema ordinal in {line}"))
                    .parse::<u32>()
                    .unwrap_or_else(|_| panic!("non-numeric schema ordinal in {line}"));
                Some((name.to_owned(), ordinal))
            })
            .collect()
    }

    fn assert_named_schema<T: Copy + Debug>(prefix: &str, all: &[T], project: impl Fn(T) -> u32) {
        let definitions = header_schema(prefix);
        assert_eq!(definitions.len(), all.len(), "{prefix} definition count");
        let unique_names = definitions
            .iter()
            .map(|(name, _)| name)
            .collect::<HashSet<_>>();
        assert_eq!(unique_names.len(), all.len(), "duplicate {prefix} name");
        let unique_ordinals = definitions
            .iter()
            .map(|(_, ordinal)| ordinal)
            .collect::<HashSet<_>>();
        assert_eq!(
            unique_ordinals.len(),
            all.len(),
            "duplicate {prefix} ordinal"
        );

        for (expected, kind) in all.iter().copied().enumerate() {
            let expected = u32::try_from(expected).unwrap();
            assert_eq!(project(kind), expected, "Rust projection for {kind:?}");
            let expected_name = format!("{prefix}{}", screaming_snake(&format!("{kind:?}")));
            assert!(
                definitions
                    .iter()
                    .any(|(name, ordinal)| name == &expected_name && *ordinal == expected),
                "missing header definition {expected_name}={expected}"
            );
        }
    }

    #[test]
    fn directive_and_clause_ordinals_have_exhaustive_named_schemas() {
        assert_named_schema(
            "ROUP_OMP_DIRECTIVE_",
            roup::ast::OmpDirectiveKind::ALL,
            omp_directive_kind_ordinal,
        );
        assert_named_schema(
            "ROUP_OMP_CLAUSE_",
            roup::ast::OmpClauseKind::ALL,
            omp_clause_kind_ordinal,
        );
        assert_named_schema(
            "ROUP_ACC_DIRECTIVE_",
            roup::ast::AccDirectiveKind::ALL,
            acc_directive_kind_ordinal,
        );
        assert_named_schema(
            "ROUP_ACC_CLAUSE_",
            roup::ast::AccClauseKind::ALL,
            acc_clause_kind_ordinal,
        );
    }

    #[test]
    fn structured_openmp_parameters_never_flatten_to_strings() {
        let storage =
            openmp_parameter_projection(openmp_fortran_options(), "!$omp threadprivate(/state/)");
        let FieldValue::Nodes(items) = only_field(&storage, crate::ROUP_FIELD_ITEMS) else {
            panic!("threadprivate items were not tagged nodes");
        };
        assert!(matches!(
            items.as_slice(),
            [NodeRecord {
                kind: RoupNodeKind {
                    family: crate::ROUP_NODE_FAMILY_OMP_STORAGE_ITEM,
                    variant: crate::ROUP_OMP_STORAGE_ITEM_FORTRAN_COMMON_BLOCK,
                },
                ..
            }]
        ));

        let reduction = openmp_parameter_projection(
            openmp_options(),
            "#pragma omp declare reduction(sum : int : omp_out += omp_in) initializer(omp_priv = 0)",
        );
        let FieldValue::Node(identifier) = only_field(&reduction, crate::ROUP_FIELD_NAME) else {
            panic!("reduction identifier was not a tagged node");
        };
        assert_eq!(
            identifier.kind.family,
            crate::ROUP_NODE_FAMILY_OMP_IDENTIFIER
        );
        assert_eq!(identifier.kind.variant, crate::ROUP_OMP_IDENTIFIER_NAME);
        let FieldValue::Node(id_expression) =
            only_field(&identifier.fields, crate::ROUP_FIELD_VALUE)
        else {
            panic!("user reduction identifier lost its id-expression tag");
        };
        assert_eq!(
            id_expression.kind,
            RoupNodeKind {
                family: crate::ROUP_NODE_FAMILY_OMP_ID_EXPRESSION,
                variant: crate::ROUP_OMP_ID_EXPRESSION_NAME,
            }
        );
        let FieldValue::Node(combiner) = only_field(&reduction, crate::ROUP_FIELD_COMBINER) else {
            panic!("reduction combiner was not a tagged node");
        };
        assert_eq!(
            combiner.kind.variant,
            crate::ROUP_OMP_STYLIZED_C_CPP_EXPRESSION
        );
        let FieldValue::Node(initializer) = only_field(&reduction, crate::ROUP_FIELD_INITIALIZER)
        else {
            panic!("reduction initializer was not a tagged node");
        };
        assert_eq!(
            initializer.kind.variant,
            crate::ROUP_OMP_INITIALIZER_C_ASSIGNMENT
        );
    }

    #[test]
    fn closed_clause_values_and_allocator_classes_use_numeric_tags() {
        let clauses = projected_clause_fields(
            openmp_options(),
            "#pragma omp parallel for schedule(monotonic: dynamic, 4) default(none)",
        );
        let schedule = clauses
            .iter()
            .find(|(kind, _)| *kind == crate::ROUP_OMP_CLAUSE_SCHEDULE)
            .map(|(_, fields)| fields)
            .expect("schedule clause");
        assert!(matches!(
            only_field(schedule, crate::ROUP_FIELD_KIND),
            FieldValue::U32(crate::ROUP_OMP_SCHEDULE_DYNAMIC)
        ));
        assert!(matches!(
            only_field(schedule, crate::ROUP_FIELD_MODIFIERS),
            FieldValue::U32s(values)
                if values.as_slice() == [crate::ROUP_OMP_SCHEDULE_MODIFIER_MONOTONIC]
        ));
        let default = clauses
            .iter()
            .find(|(kind, _)| *kind == crate::ROUP_OMP_CLAUSE_DEFAULT)
            .map(|(_, fields)| fields)
            .expect("default clause");
        assert!(matches!(
            only_field(default, crate::ROUP_FIELD_KIND),
            FieldValue::U32(crate::ROUP_OMP_DEFAULT_NONE)
        ));

        let allocator_clauses = projected_clause_fields(
            openmp_options(),
            "#pragma omp target uses_allocators(omp_default_mem_alloc, custom_allocator(custom_traits))",
        );
        let fields = &allocator_clauses[0].1;
        let FieldValue::Nodes(entries) = only_field(fields, crate::ROUP_FIELD_ALLOCATORS) else {
            panic!("uses_allocators entries were not tagged nodes");
        };
        let FieldValue::Node(builtin) = only_field(&entries[0].fields, crate::ROUP_FIELD_ALLOCATOR)
        else {
            panic!("predefined allocator was not tagged");
        };
        assert_eq!(builtin.kind.variant, crate::ROUP_OMP_ALLOCATOR_DEFAULT);
        let FieldValue::Node(custom) = only_field(&entries[1].fields, crate::ROUP_FIELD_ALLOCATOR)
        else {
            panic!("custom allocator was not tagged");
        };
        assert_eq!(custom.kind.variant, crate::ROUP_OMP_ALLOCATOR_CUSTOM);
    }

    #[test]
    fn apply_induction_and_init_cross_the_abi_as_typed_nodes() {
        let apply_clauses = projected_clause_fields(
            openmp_options(),
            "#pragma omp tile sizes(8) apply(grid(1): reverse)",
        );
        let apply = apply_clauses
            .iter()
            .find(|(kind, _)| *kind == crate::ROUP_OMP_CLAUSE_APPLY)
            .map(|(_, fields)| fields)
            .expect("apply clause");
        let FieldValue::Node(modifier) = only_field(apply, crate::ROUP_FIELD_LOOP_MODIFIER) else {
            panic!("apply modifier must be a tagged node");
        };
        assert_eq!(
            modifier.kind,
            RoupNodeKind {
                family: crate::ROUP_NODE_FAMILY_OMP_APPLY_MODIFIER,
                variant: crate::ROUP_OMP_APPLY_GRID,
            }
        );
        assert!(matches!(
            only_field(&modifier.fields, crate::ROUP_FIELD_INDICES),
            FieldValue::Strings(indices) if indices.as_slice() == ["1"]
        ));
        let FieldValue::Nodes(applied) = only_field(apply, crate::ROUP_FIELD_APPLIED_DIRECTIVES)
        else {
            panic!("applied directives must be semantic nodes");
        };
        assert_eq!(
            applied[0].kind.family,
            crate::ROUP_NODE_FAMILY_OMP_DIRECTIVE
        );

        let induction_clauses = projected_clause_fields(
            openmp_options(),
            "#pragma omp parallel for induction(strict, step(delta), *: index)",
        );
        let induction = induction_clauses
            .iter()
            .find(|(kind, _)| *kind == crate::ROUP_OMP_CLAUSE_INDUCTION)
            .map(|(_, fields)| fields)
            .expect("induction clause");
        assert!(matches!(
            only_field(induction, crate::ROUP_FIELD_MODIFIER),
            FieldValue::U32(crate::ROUP_OMP_INDUCTION_STRICT)
        ));
        let FieldValue::Node(identifier) = only_field(induction, crate::ROUP_FIELD_IDENTIFIER)
        else {
            panic!("induction identifier must be a tagged identifier node");
        };
        assert_eq!(identifier.kind.variant, crate::ROUP_OMP_IDENTIFIER_MULTIPLY);

        let init_clauses = projected_clause_fields(
            openmp_options(),
            "#pragma omp interop init(prefer_type({fr(\"cuda\"), attr(\"ompx_fast\")}), target: object)",
        );
        let init = init_clauses
            .iter()
            .find(|(kind, _)| *kind == crate::ROUP_OMP_CLAUSE_INIT)
            .map(|(_, fields)| fields)
            .expect("init clause");
        assert!(matches!(
            only_field(init, crate::ROUP_FIELD_INTEROP_TYPES),
            FieldValue::U32s(types) if types.as_slice() == [crate::ROUP_OMP_INTEROP_TARGET]
        ));
        let FieldValue::Nodes(preferences) = only_field(init, crate::ROUP_FIELD_PREFERENCES) else {
            panic!("preference specifications must be tagged nodes");
        };
        assert_eq!(
            preferences[0].kind,
            RoupNodeKind {
                family: crate::ROUP_NODE_FAMILY_OMP_PREFERENCE_SPECIFICATION,
                variant: crate::ROUP_OMP_PREFERENCE_SELECTORS,
            }
        );

        let update_clauses = projected_clause_fields(
            openmp_options(),
            "#pragma omp depobj update(inout: dependence_object)",
        );
        let update = update_clauses
            .iter()
            .find(|(kind, _)| *kind == crate::ROUP_OMP_CLAUSE_DEPOBJ_UPDATE)
            .map(|(_, fields)| fields)
            .expect("depobj update clause");
        assert!(matches!(
            only_field(update, crate::ROUP_FIELD_DEPEND_TYPE),
            FieldValue::U32(crate::ROUP_OMP_DEPOBJ_UPDATE_INOUT)
        ));
        assert!(matches!(
            only_field(update, crate::ROUP_FIELD_VARIABLE),
            FieldValue::String(variable) if variable == "dependence_object"
        ));
    }

    #[test]
    fn argument_adjustment_and_append_operations_cross_the_abi_as_typed_nodes() {
        let clauses = projected_clause_fields(
            openmp_options(),
            "#pragma omp declare variant(fast) match(construct={parallel}) adjust_args(need_device_ptr: 1, 3:5, named) append_args(interop(target, targetsync))",
        );

        let adjust = clauses
            .iter()
            .find(|(kind, _)| *kind == crate::ROUP_OMP_CLAUSE_ADJUST_ARGS)
            .map(|(_, fields)| fields)
            .expect("adjust_args clause");
        assert!(matches!(
            only_field(adjust, crate::ROUP_FIELD_OPERATION),
            FieldValue::U32(crate::ROUP_OMP_ADJUST_ARGS_NEED_DEVICE_PTR)
        ));
        let FieldValue::Nodes(parameters) = only_field(adjust, crate::ROUP_FIELD_PARAMETERS) else {
            panic!("adjust_args parameters must be typed nodes");
        };
        assert_eq!(parameters.len(), 3);
        assert_eq!(
            parameters[0].kind,
            RoupNodeKind {
                family: crate::ROUP_NODE_FAMILY_OMP_PARAMETER_LIST_ITEM,
                variant: crate::ROUP_OMP_PARAMETER_POSITION,
            }
        );
        assert!(matches!(
            only_field(&parameters[0].fields, crate::ROUP_FIELD_VALUE),
            FieldValue::U64(1)
        ));
        assert_eq!(parameters[1].kind.variant, crate::ROUP_OMP_PARAMETER_RANGE);
        assert!(matches!(
            only_field(&parameters[1].fields, crate::ROUP_FIELD_LOWER_BOUND),
            FieldValue::String(lower) if lower == "3"
        ));
        assert!(matches!(
            only_field(&parameters[1].fields, crate::ROUP_FIELD_UPPER_BOUND),
            FieldValue::String(upper) if upper == "5"
        ));
        assert_eq!(parameters[2].kind.variant, crate::ROUP_OMP_PARAMETER_NAMED);

        let append = clauses
            .iter()
            .find(|(kind, _)| *kind == crate::ROUP_OMP_CLAUSE_APPEND_ARGS)
            .map(|(_, fields)| fields)
            .expect("append_args clause");
        let FieldValue::Nodes(operations) = only_field(append, crate::ROUP_FIELD_OPERATIONS) else {
            panic!("append_args operations must be typed nodes");
        };
        assert_eq!(operations.len(), 1);
        assert_eq!(
            operations[0].kind,
            RoupNodeKind {
                family: crate::ROUP_NODE_FAMILY_OMP_APPEND_OPERATION,
                variant: crate::ROUP_OMP_APPEND_INTEROP,
            }
        );
        assert!(matches!(
            only_field(&operations[0].fields, crate::ROUP_FIELD_INTEROP_TYPES),
            FieldValue::U32s(types)
                if types == &[crate::ROUP_OMP_INTEROP_TARGET, crate::ROUP_OMP_INTEROP_TARGETSYNC]
        ));
        assert!(matches!(
            only_field(&operations[0].fields, crate::ROUP_FIELD_PREFERENCES),
            FieldValue::Nodes(preferences) if preferences.is_empty()
        ));
    }

    #[test]
    fn context_selectors_cross_the_abi_as_distinct_recursive_nodes() {
        let clauses = projected_clause_fields(
            openmp_options(),
            "#pragma omp metadirective when(device={kind(cpu, gpu)}, target_device={device_num(dev), uid(\"gpu-0\")}, implementation={vendor(score(4): llvm, gnu), atomic_default_mem_order(acquire), requires(unified_address(flag), atomic_default_mem_order(relaxed)), vendor_trait(score(2): nested(prop, 4))}, user={condition(score(5): runtime_flag)}: parallel)",
        );
        let when = clauses
            .iter()
            .find(|(kind, _)| *kind == crate::ROUP_OMP_CLAUSE_WHEN)
            .map(|(_, fields)| fields)
            .expect("when clause");
        let FieldValue::Nodes(entries) = only_field(when, crate::ROUP_FIELD_ENTRIES) else {
            panic!("selector entries must be nodes");
        };
        assert_eq!(entries.len(), 4);
        assert_eq!(
            entries[0].kind,
            RoupNodeKind {
                family: crate::ROUP_NODE_FAMILY_OMP_SELECTOR_ENTRY,
                variant: crate::ROUP_SELECTOR_ENTRY_DEVICE,
            }
        );
        assert_eq!(
            entries[1].kind.variant,
            crate::ROUP_SELECTOR_ENTRY_TARGET_DEVICE
        );

        let FieldValue::Nodes(device_traits) =
            only_field(&entries[0].fields, crate::ROUP_FIELD_TRAITS)
        else {
            panic!("device traits must be nodes");
        };
        assert_eq!(
            device_traits[0].kind,
            RoupNodeKind {
                family: crate::ROUP_NODE_FAMILY_OMP_SELECTOR_DEVICE_TRAIT,
                variant: crate::ROUP_SELECTOR_DEVICE_NAME_LIST,
            }
        );
        let FieldValue::Node(kind_trait) =
            only_field(&device_traits[0].fields, crate::ROUP_FIELD_TRAIT_NAME)
        else {
            panic!("name-list trait must be a node");
        };
        assert_eq!(
            kind_trait.kind,
            RoupNodeKind {
                family: crate::ROUP_NODE_FAMILY_OMP_SELECTOR_NAME_LIST_TRAIT,
                variant: crate::ROUP_SELECTOR_NAME_LIST_KIND,
            }
        );
        let FieldValue::Nodes(kind_properties) =
            only_field(&kind_trait.fields, crate::ROUP_FIELD_PROPERTIES)
        else {
            panic!("name-list properties must be nodes");
        };
        assert_eq!(kind_properties.len(), 2);
        assert!(kind_properties.iter().all(|property| {
            property.kind.family == crate::ROUP_NODE_FAMILY_OMP_SELECTOR_TRAIT_VALUE
        }));

        let FieldValue::Nodes(target_traits) =
            only_field(&entries[1].fields, crate::ROUP_FIELD_TRAITS)
        else {
            panic!("target-device traits must be nodes");
        };
        assert_eq!(
            target_traits[0].kind.variant,
            crate::ROUP_SELECTOR_DEVICE_NUM
        );
        assert!(matches!(
            only_field(&target_traits[0].fields, crate::ROUP_FIELD_VALUE),
            FieldValue::String(value) if value == "dev"
        ));
        assert_eq!(
            target_traits[1].kind.variant,
            crate::ROUP_SELECTOR_DEVICE_UID
        );
        let FieldValue::Node(uid) = only_field(&target_traits[1].fields, crate::ROUP_FIELD_VALUE)
        else {
            panic!("uid must retain its typed string-literal node");
        };
        assert_eq!(
            uid.kind.family,
            crate::ROUP_NODE_FAMILY_OMP_SELECTOR_TRAIT_VALUE
        );
        assert_eq!(uid.kind.variant, crate::ROUP_SELECTOR_TRAIT_STRING_LITERAL);

        let FieldValue::Nodes(implementation_traits) =
            only_field(&entries[2].fields, crate::ROUP_FIELD_TRAITS)
        else {
            panic!("implementation traits must be nodes");
        };
        assert_eq!(implementation_traits.len(), 4);
        assert_eq!(
            implementation_traits[0].kind.variant,
            crate::ROUP_SELECTOR_IMPLEMENTATION_NAME_LIST
        );
        assert!(matches!(
            only_field(&implementation_traits[0].fields, crate::ROUP_FIELD_SCORE),
            FieldValue::String(score) if score == "4"
        ));
        assert_eq!(
            implementation_traits[1].kind.variant,
            crate::ROUP_SELECTOR_IMPLEMENTATION_ATOMIC_DEFAULT_MEM_ORDER
        );
        assert!(matches!(
            only_field(
                &implementation_traits[1].fields,
                crate::ROUP_FIELD_MEMORY_ORDER
            ),
            FieldValue::U32(crate::ROUP_OMP_MEMORY_ORDER_ACQUIRE)
        ));

        assert_eq!(
            implementation_traits[2].kind.variant,
            crate::ROUP_SELECTOR_IMPLEMENTATION_REQUIRES
        );
        let FieldValue::Nodes(requirements) = only_field(
            &implementation_traits[2].fields,
            crate::ROUP_FIELD_PROPERTIES,
        ) else {
            panic!("requires properties must be typed nodes");
        };
        assert_eq!(
            requirements[0].kind.family,
            crate::ROUP_NODE_FAMILY_OMP_SELECTOR_REQUIREMENT
        );
        let FieldValue::Node(requirement) =
            only_field(&requirements[0].fields, crate::ROUP_FIELD_REQUIREMENT)
        else {
            panic!("selector requirement must wrap a requirement node");
        };
        assert_eq!(
            requirement.kind.family,
            crate::ROUP_NODE_FAMILY_REQUIRE_MODIFIER
        );
        assert_eq!(
            requirement.kind.variant,
            crate::ROUP_REQUIRE_UNIFIED_ADDRESS
        );
        assert!(matches!(
            only_field(&requirements[0].fields, crate::ROUP_FIELD_REQUIRED),
            FieldValue::String(required) if required == "flag"
        ));

        assert_eq!(
            implementation_traits[3].kind.variant,
            crate::ROUP_SELECTOR_IMPLEMENTATION_EXTENSION
        );
        let FieldValue::Node(extension) = only_field(
            &implementation_traits[3].fields,
            crate::ROUP_FIELD_TRAIT_NAME,
        ) else {
            panic!("implementation extension must be a typed trait node");
        };
        assert_eq!(
            extension.kind.family,
            crate::ROUP_NODE_FAMILY_OMP_SELECTOR_EXTENSION_TRAIT
        );
        let FieldValue::Nodes(properties) =
            only_field(&extension.fields, crate::ROUP_FIELD_PROPERTIES)
        else {
            panic!("extension properties must be recursive nodes");
        };
        assert_eq!(
            properties[0].kind,
            RoupNodeKind {
                family: crate::ROUP_NODE_FAMILY_OMP_SELECTOR_EXTENSION_PROPERTY,
                variant: crate::ROUP_SELECTOR_EXTENSION_PROPERTY_CALL,
            }
        );
        assert_eq!(entries[3].kind.variant, crate::ROUP_SELECTOR_ENTRY_USER);
        assert!(matches!(
            only_field(&entries[3].fields, crate::ROUP_FIELD_SCORE),
            FieldValue::String(score) if score == "5"
        ));
        assert!(matches!(
            only_field(&entries[3].fields, crate::ROUP_FIELD_CONDITION),
            FieldValue::String(condition) if condition == "runtime_flag"
        ));
    }

    #[test]
    fn allocation_expressions_cross_the_abi_without_allocator_name_reclassification() {
        let historical = projected_clause_fields(
            openmp_options(),
            "#pragma omp parallel private(value) allocate(select_allocator(device): value)",
        );
        let fields = historical
            .iter()
            .find(|(kind, _)| *kind == crate::ROUP_OMP_CLAUSE_ALLOCATE)
            .map(|(_, fields)| fields)
            .expect("allocate clause");
        assert!(matches!(
            only_field(fields, crate::ROUP_FIELD_ALLOCATOR_EXPRESSION),
            FieldValue::String(expression) if expression == "select_allocator(device)"
        ));
        assert!(fields
            .iter()
            .all(|field| field.id != crate::ROUP_FIELD_ALIGNMENT_EXPRESSION));
        assert!(matches!(
            only_field(fields, crate::ROUP_FIELD_ALLOCATE_SOURCE_SYNTAX),
            FieldValue::U32(crate::ROUP_OMP_ALLOCATE_SOURCE_SIMPLE_ALLOCATOR)
        ));

        let modern = projected_clause_fields(
            openmp_options(),
            "#pragma omp parallel private(value) allocate(allocator(select_allocator(device)), align(64): value)",
        );
        let fields = modern
            .iter()
            .find(|(kind, _)| *kind == crate::ROUP_OMP_CLAUSE_ALLOCATE)
            .map(|(_, fields)| fields)
            .expect("allocate clause");
        assert!(matches!(
            only_field(fields, crate::ROUP_FIELD_ALLOCATOR_EXPRESSION),
            FieldValue::String(expression) if expression == "select_allocator(device)"
        ));
        assert!(matches!(
            only_field(fields, crate::ROUP_FIELD_ALIGNMENT_EXPRESSION),
            FieldValue::String(expression) if expression == "64"
        ));
        assert!(matches!(
            only_field(fields, crate::ROUP_FIELD_ALLOCATE_SOURCE_SYNTAX),
            FieldValue::U32(crate::ROUP_OMP_ALLOCATE_SOURCE_MODIFIERS)
        ));

        let allocator = projected_clause_fields(
            openmp_options(),
            "#pragma omp allocate(value) allocator(select_allocator(device))",
        );
        let fields = allocator
            .iter()
            .find(|(kind, _)| *kind == crate::ROUP_OMP_CLAUSE_ALLOCATOR)
            .map(|(_, fields)| fields)
            .expect("allocator clause");
        assert!(matches!(
            only_field(fields, crate::ROUP_FIELD_ALLOCATOR_EXPRESSION),
            FieldValue::String(expression) if expression == "select_allocator(device)"
        ));
    }

    #[test]
    fn openacc_closed_values_and_user_leaves_remain_separate() {
        let clauses = projected_clause_fields(
            openacc_options(),
            "#pragma acc parallel default(present) reduction(+: sum) device_type(host, gpu)",
        );
        let default = clauses
            .iter()
            .find(|(kind, _)| *kind == crate::ROUP_ACC_CLAUSE_DEFAULT)
            .map(|(_, fields)| fields)
            .expect("default clause");
        assert!(matches!(
            only_field(default, crate::ROUP_FIELD_KIND),
            FieldValue::U32(crate::ROUP_ACC_DEFAULT_PRESENT)
        ));
        let device_type = clauses
            .iter()
            .find(|(kind, _)| *kind == crate::ROUP_ACC_CLAUSE_DEVICE_TYPE)
            .map(|(_, fields)| fields)
            .expect("device_type clause");
        let FieldValue::Nodes(values) = only_field(device_type, crate::ROUP_FIELD_VALUES) else {
            panic!("device types were not tagged nodes");
        };
        assert_eq!(values[0].kind.variant, crate::ROUP_ACC_DEVICE_TYPE_HOST);
        assert_eq!(values[1].kind.variant, crate::ROUP_ACC_DEVICE_TYPE_NAMED);
        assert!(matches!(
            only_field(&values[1].fields, crate::ROUP_FIELD_NAME),
            FieldValue::String(name) if name == "gpu"
        ));
        let reduction = clauses
            .iter()
            .find(|(kind, _)| *kind == crate::ROUP_ACC_CLAUSE_REDUCTION)
            .map(|(_, fields)| fields)
            .expect("reduction clause");
        assert!(matches!(
            only_field(reduction, crate::ROUP_FIELD_OPERATOR),
            FieldValue::Node(NodeRecord {
                kind: RoupNodeKind {
                    family: crate::ROUP_NODE_FAMILY_ACC_REDUCTION_OPERATOR,
                    variant: crate::ROUP_ACC_REDUCTION_ADD,
                },
                ..
            })
        ));
    }

    #[test]
    fn global_state_supports_parallel_parser_lifetimes() {
        let workers = (0..8)
            .map(|_| {
                std::thread::spawn(|| {
                    let parser = create_parser(openmp_options()).unwrap();
                    let directive = parse(parser, "#pragma omp parallel".to_owned()).unwrap();
                    assert_eq!(directive_dialect(directive).unwrap(), ROUP_DIALECT_OPENMP);
                    release_directive(directive).unwrap();
                    release_parser(parser).unwrap();
                })
            })
            .collect::<Vec<_>>();

        for worker in workers {
            worker.join().unwrap();
        }
    }
}
