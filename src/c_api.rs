//! # Minimal Unsafe C API for OpenMP Parser
//!
//! This module provides a direct pointer-based C FFI with minimal unsafe code.
//!
//! ## Design Philosophy: Direct Pointers vs Handles
//!
//! We use **raw C pointers** to Rust structs instead of opaque handles for:
//! - **Idiomatic C API**: Standard malloc/free pattern familiar to C programmers
//! - **Simple memory model**: No global registry, no handle bookkeeping
//! - **Easy integration**: Works naturally with C/C++ code
//! - **Minimal code**: 632 lines vs 4000+ lines of handle management
//!
//! ## Safety Analysis: 18 Unsafe Blocks (~60 lines)
//!
//! All unsafe blocks are:
//! - **NULL-checked** before dereferencing
//! - **Documented** with explicit safety requirements
//! - **Isolated** only at FFI boundary, never in business logic
//! - **Auditable**: ~0.9% of file (60 unsafe lines / 632 total)
//!
//! ## Case-Insensitive Matching: String Allocation Tradeoff
//!
//! Functions `directive_name_to_kind()` and `clause_name_to_kind_for_constants()` allocate a String
//! for case-insensitive matching (Fortran uses uppercase, C uses lowercase).
//!
//! **Why not optimize with `eq_ignore_ascii_case()`?**
//! - Constants generator (`src/constants_gen.rs`) requires `match` expressions
//! - Parser uses syn crate to extract directive/clause mappings from AST
//! - Cannot parse if-else chains → must use `match normalized_name.as_str()`
//! - String allocation is necessary for match arm patterns
//!
//! **Is this a performance issue?**
//! - No: These functions are called once per directive/clause at API boundary
//! - Typical usage: Parse a few dozen directives in an entire program
//! - String allocation cost is negligible compared to parsing overhead
//!
//! **Future optimization path**: Update `constants_gen.rs` to parse if-else chains,
//! then use `eq_ignore_ascii_case()` without allocations.
//!
//! ## Learning Rust: Why Unsafe is Needed at FFI Boundary
//!
//! 1. **C strings** → Rust strings: `CStr::from_ptr()` requires unsafe
//! 2. **Memory ownership** transfer: `Box::into_raw()` / `Box::from_raw()`
//! 3. **Raw pointer dereferencing**: C passes pointers, we must dereference
//!
//! Safe Rust cannot verify C's guarantees, so we explicitly document them.
//!
//! ## C Caller Responsibilities
//!
//! C callers MUST:
//! - ✅ Check for NULL returns before use
//! - ✅ Call `_free()` functions to prevent memory leaks
//! - ✅ Never use pointers after calling `_free()`
//! - ✅ Pass only valid null-terminated strings
//! - ✅ Not modify strings returned by this API

// Clippy configuration for FFI module
// We intentionally wrap unsafe operations in safe public functions for the C API.
// The C callers are responsible for upholding safety invariants (documented above).
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::borrow::Cow;
use std::cell::Cell;
use std::ffi::{CStr, CString};
use std::mem::ManuallyDrop;
use std::os::raw::c_char;
use std::ptr;

use crate::ast::{
    ClauseNormalizationMode, DirectiveBody, OmpClauseKind, OmpConstructType, OmpDirectiveParameter,
    OmpScanMode, ReductionOperatorToken,
};
use crate::ir::{
    convert_directive, AffinityModifier, AtomicOp, ClauseData as IrClauseData, ClauseItem,
    DefaultKind, DefaultmapBehavior, DefaultmapCategory, DependIterator, DependType,
    DepobjUpdateDependence, DeviceModifier, GrainsizeModifier, Identifier, Language as IrLanguage,
    LastprivateModifier, LinearModifier, MapModifier, MapType, MemoryOrder, NumTasksModifier,
    OrderModifier, ParserConfig, ReductionModifier, ReductionOperator, RequireModifier,
    ScheduleKind as IrScheduleKind, ScheduleModifier, SourceLocation, UsesAllocatorBuiltin,
    UsesAllocatorKind, UsesAllocatorSpec, Variable,
};
use crate::lexer::Language;
use crate::parser::directive_kind::{lookup_directive_name, DirectiveName};
use crate::parser::lookup_clause_name;
use crate::parser::{openmp, Clause, ClauseKind, ClauseName};

mod openacc;
pub use openacc::*;

// Mapping used only by constants_gen/header generation; runtime uses AST converters.
fn clause_name_to_kind_for_constants(name: ClauseName) -> i32 {
    match name {
        ClauseName::If => CLAUSE_KIND_IF,
        ClauseName::NumThreads => CLAUSE_KIND_NUM_THREADS,
        ClauseName::Default => CLAUSE_KIND_DEFAULT,
        ClauseName::Private => CLAUSE_KIND_PRIVATE,
        ClauseName::Firstprivate => CLAUSE_KIND_FIRSTPRIVATE,
        ClauseName::Shared => CLAUSE_KIND_SHARED,
        ClauseName::CopyIn => CLAUSE_KIND_COPYIN,
        ClauseName::Align => CLAUSE_KIND_ALIGNED,
        ClauseName::Reduction => CLAUSE_KIND_REDUCTION,
        ClauseName::ProcBind => CLAUSE_KIND_PROC_BIND,
        ClauseName::NumTeams => CLAUSE_KIND_NUM_TEAMS,
        ClauseName::ThreadLimit => CLAUSE_KIND_THREAD_LIMIT,
        ClauseName::Lastprivate => CLAUSE_KIND_LASTPRIVATE,
        ClauseName::Collapse => CLAUSE_KIND_COLLAPSE,
        ClauseName::Ordered => CLAUSE_KIND_ORDERED,
        ClauseName::Nowait => CLAUSE_KIND_NOWAIT,
        ClauseName::Schedule => CLAUSE_KIND_SCHEDULE,
        ClauseName::DistSchedule => CLAUSE_KIND_DIST_SCHEDULE,
        ClauseName::InReduction => CLAUSE_KIND_IN_REDUCTION,
        ClauseName::Depend => CLAUSE_KIND_DEPEND,
        ClauseName::UsesAllocators => CLAUSE_KIND_USES_ALLOCATORS,
        ClauseName::TaskReduction => CLAUSE_KIND_TASK_REDUCTION,
        ClauseName::Device => CLAUSE_KIND_DEVICE,
        ClauseName::Map => CLAUSE_KIND_MAP,
        ClauseName::Defaultmap => CLAUSE_KIND_DEFAULTMAP,
        ClauseName::Copyprivate => CLAUSE_KIND_COPYPRIVATE,
        ClauseName::Affinity => CLAUSE_KIND_AFFINITY,
        ClauseName::Priority => CLAUSE_KIND_PRIORITY,
        ClauseName::Grainsize => CLAUSE_KIND_GRAINSIZE,
        ClauseName::NumTasks => CLAUSE_KIND_NUM_TASKS,
        ClauseName::Order => CLAUSE_KIND_ORDER,
        ClauseName::AtomicDefaultMemOrder => CLAUSE_KIND_ATOMIC_DEFAULT_MEM_ORDER,
        ClauseName::UseDevicePtr => CLAUSE_KIND_USE_DEVICE_PTR,
        ClauseName::UseDeviceAddr => CLAUSE_KIND_USE_DEVICE_ADDR,
        ClauseName::IsDevicePtr => CLAUSE_KIND_IS_DEVICE_PTR,
        ClauseName::HasDeviceAddr => CLAUSE_KIND_HAS_DEVICE_ADDR,
        ClauseName::DeviceType => CLAUSE_KIND_DEVICE_TYPE,
        ClauseName::DepobjUpdate => CLAUSE_KIND_DEPOBJ_UPDATE,
        ClauseName::Nontemporal => CLAUSE_KIND_NONTEMPORAL,
        ClauseName::Uniform => CLAUSE_KIND_UNIFORM,
        ClauseName::Inbranch => CLAUSE_KIND_INBRANCH,
        ClauseName::Notinbranch => CLAUSE_KIND_NOTINBRANCH,
        ClauseName::Inclusive => CLAUSE_KIND_INCLUSIVE,
        ClauseName::Exclusive => CLAUSE_KIND_EXCLUSIVE,
        ClauseName::Compare => CLAUSE_KIND_COMPARE,
        ClauseName::CompareCapture => CLAUSE_KIND_COMPARE_CAPTURE,
        ClauseName::Allocator => CLAUSE_KIND_ALLOCATOR,
        ClauseName::Allocate => CLAUSE_KIND_ALLOCATE,
        ClauseName::Copy => CLAUSE_KIND_MAP, // alias if needed
        ClauseName::CopyOut => CLAUSE_KIND_MAP, // alias if needed
        // OpenACC-only clauses are not part of the OpenMP kind space; return UNKNOWN_KIND.
        ClauseName::Async
        | ClauseName::Wait
        | ClauseName::NumGangs
        | ClauseName::NumWorkers
        | ClauseName::VectorLength
        | ClauseName::Gang
        | ClauseName::Worker
        | ClauseName::Vector
        | ClauseName::Seq
        | ClauseName::Independent
        | ClauseName::Auto
        | ClauseName::Bind
        | ClauseName::DefaultAsync
        | ClauseName::Link
        | ClauseName::NoCreate
        | ClauseName::NoHost
        | ClauseName::Read
        | ClauseName::SelfClause
        | ClauseName::Tile
        | ClauseName::Update
        | ClauseName::Delete
        | ClauseName::DevicePtr
        | ClauseName::DeviceNum
        | ClauseName::DeviceResident
        | ClauseName::Host
        | ClauseName::IfPresent
        | ClauseName::Capture
        | ClauseName::Write
        | ClauseName::Detach => UNKNOWN_KIND,
        other => clause_name_enum_to_kind(other),
    }
}

// ============================================================================
// Language Constants for Fortran Support
// ============================================================================

/// C language (default) - uses #pragma omp
pub const ROUP_LANG_C: i32 = 0;

/// Fortran free-form - uses !$OMP sentinel
pub const ROUP_LANG_FORTRAN_FREE: i32 = 1;

/// Fortran fixed-form - uses !$OMP or C$OMP in columns 1-6
pub const ROUP_LANG_FORTRAN_FIXED: i32 = 2;

// ============================================================================
// Constants Documentation
// ============================================================================
//
// SINGLE SOURCE OF TRUTH: This file defines all directive and clause kind codes.
//
// The constants are defined in:
// - directive_name_to_kind() function (directive codes 0-16)
// - clause_name_to_kind_for_constants() function (clause codes)
//
// For C/C++ usage:
// - build.rs auto-generates src/roup_constants.h with #define macros
// - The header provides compile-time constants for switch/case statements
// - Never modify roup_constants.h directly - edit this file instead
//
// Maintenance: When adding new directives/clauses:
// 1. Update directive_name_to_kind() or clause_name_to_kind_for_constants() in this file
// 2. Run `cargo build` to regenerate roup_constants.h
// 3. The header will automatically include your new constants

// ============================================================================
// C-Compatible Types
// ============================================================================
//
// Learning Rust: FFI Type Safety
// ===============================
// The `#[repr(C)]` attribute ensures these types have the same memory layout
// as C structs. This is crucial for FFI safety:
//
// - Rust's default layout is undefined and may reorder fields
// - C expects specific field ordering and sizes
// - `#[repr(C)]` guarantees C-compatible layout
//
// Without `#[repr(C)]`, passing these to C would cause undefined behavior!

/// Opaque directive type (C-compatible)
///
/// Represents a parsed OpenMP directive with its clauses.
/// C sees this as an opaque pointer - internal structure is hidden.
#[repr(C)]
pub struct OmpDirective {
    name: *const c_char,      // Directive name (e.g., "parallel")
    parameter: *const c_char, // Directive parameter (e.g., "(a,b,c)" for allocate/threadprivate)
    parameter_data: DirectiveParameterData,
    clauses: Vec<OmpClause>, // Associated clauses
}

/// Opaque clause type (C-compatible)
///
/// Represents a single clause within a directive.
/// Uses tagged union pattern for clause-specific data.
#[repr(C)]
pub struct OmpClause {
    kind: i32,                // Clause type (num_threads=0, schedule=7, etc.)
    arguments: *const c_char, // Raw clause arguments (NULL for bare clauses)
    data: ClauseData,         // Clause-specific data (union)
}

/// Clause-specific data stored in a C union
///
/// Learning Rust: Why ManuallyDrop?
/// =================================
/// Unions in Rust don't know which variant is active, so they can't
/// automatically call destructors. ManuallyDrop prevents automatic drops
/// and lets us manually free resources when we know which variant is active.
#[repr(C)]
union ClauseData {
    schedule: ManuallyDrop<ScheduleData>,
    dist_schedule: ManuallyDrop<DistScheduleData>,
    reduction: ManuallyDrop<ReductionData>,
    lastprivate: ManuallyDrop<LastprivateData>,
    order: ManuallyDrop<OrderData>,
    default: i32,
    variables: *mut OmpStringList,
    defaultmap: ManuallyDrop<DefaultmapData>,
    uses_allocators: *mut UsesAllocatorsData,
    requires: *mut RequiresData,
    device: *mut DeviceClauseData,
    map: *mut MapData,
    linear: *mut LinearData,
    depend: *mut DependData,
    allocate: *mut AllocateData,
    affinity: *mut AffinityData,
    aligned: *mut AlignedData,
}

/// Structured directive parameter information to avoid string parsing in compat.
#[repr(C)]
#[derive(Clone, Copy)]
struct DirectiveParameterData {
    kind: i32,
    scan_mode: i32,
    construct_type: i32,
    identifiers: *mut OmpStringList,
}

/// Schedule clause data (static, dynamic, guided, etc.)
#[repr(C)]
struct ScheduleData {
    kind: i32, // 0=static, 1=dynamic, 2=guided, 3=auto, 4=runtime
    modifiers: u32,
    chunk: *const c_char,
}

#[repr(C)]
struct DistScheduleData {
    kind: i32,
    chunk: *const c_char,
}

/// Reduction clause data (operator, modifiers, and variables)
#[repr(C)]
struct ReductionData {
    operator: i32,      // 0=+, 1=-, 2=*, 6=&&, 7=||, 8=min, 9=max, -1=user-defined
    modifier_mask: u32, // bitmask of ReductionModifier values
    modifiers_text: *const c_char,
    user_identifier: *const c_char,
    variables: *mut OmpStringList,
    space_after_colon: bool,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct DefaultmapData {
    behavior: i32,
    category: i32,
}

#[repr(C)]
struct UsesAllocatorEntryData {
    kind: i32,
    user_name: *const c_char,
    traits: *const c_char,
}

#[repr(C)]
struct UsesAllocatorsData {
    entries: Vec<UsesAllocatorEntryData>,
}

#[repr(C)]
struct LastprivateData {
    modifier: i32,
    variables: *mut OmpStringList,
}

#[repr(C)]
struct RequireEntryData {
    code: i32,
    name: *const c_char,
}

#[repr(C)]
struct RequiresData {
    entries: Vec<RequireEntryData>,
}

#[repr(C)]
struct DeviceClauseData {
    modifier: i32,
    expression: *const c_char,
}

#[repr(C)]
struct MapData {
    map_type: i32,
    modifiers: u32,
    mapper: *const c_char,
    variables: *mut OmpStringList,
    iterators: Vec<DependIteratorData>,
}

#[repr(C)]
struct LinearData {
    modifier: i32,
    step: *const c_char,
    variables: *mut OmpStringList,
}

#[repr(C)]
struct DependData {
    depend_type: i32,
    variables: *mut OmpStringList,
    iterators: Vec<DependIteratorData>,
}

#[repr(C)]
struct DependIteratorData {
    type_name: *const c_char,
    name: *const c_char,
    start: *const c_char,
    end: *const c_char,
    step: *const c_char,
}

#[repr(C)]
struct OrderData {
    modifier: i32,
    kind: i32,
}

#[repr(C)]
struct AllocateData {
    kind: i32,
    allocator: *const c_char,
    variables: *mut OmpStringList,
}

#[repr(C)]
struct AffinityData {
    modifier: i32,
    variables: *mut OmpStringList,
    iterators: Vec<DependIteratorData>,
}

#[repr(C)]
struct AlignedData {
    variables: *mut OmpStringList,
    alignment: *const c_char,
}

const REQUIRE_MOD_REVERSE_OFFLOAD: i32 = 0;
const REQUIRE_MOD_UNIFIED_ADDRESS: i32 = 1;
const REQUIRE_MOD_UNIFIED_SHARED_MEMORY: i32 = 2;
const REQUIRE_MOD_DYNAMIC_ALLOCATORS: i32 = 3;
const REQUIRE_MOD_SELF_MAPS: i32 = 4;
const REQUIRE_MOD_ATOMIC_SEQ_CST: i32 = 5;
const REQUIRE_MOD_ATOMIC_ACQ_REL: i32 = 6;
const REQUIRE_MOD_ATOMIC_RELEASE: i32 = 7;
const REQUIRE_MOD_ATOMIC_ACQUIRE: i32 = 8;
const REQUIRE_MOD_ATOMIC_RELAXED: i32 = 9;
const REQUIRE_MOD_EXT_IMPL_DEFINED: i32 = 10;
const MAP_MODIFIER_ALWAYS: u32 = 1 << 0;
const MAP_MODIFIER_CLOSE: u32 = 1 << 1;
const MAP_MODIFIER_PRESENT: u32 = 1 << 2;
const MAP_MODIFIER_SELF: u32 = 1 << 3;
const MAP_MODIFIER_OMPX_HOLD: u32 = 1 << 4;
const MAP_TYPE_UNSPECIFIED: i32 = -1;
const REQUIRE_MOD_NAMES: [&[u8]; 11] = [
    b"reverse_offload\0",
    b"unified_address\0",
    b"unified_shared_memory\0",
    b"dynamic_allocators\0",
    b"self_maps\0",
    b"atomic_default_mem_order(seq_cst)\0",
    b"atomic_default_mem_order(acq_rel)\0",
    b"atomic_default_mem_order(release)\0",
    b"atomic_default_mem_order(acquire)\0",
    b"atomic_default_mem_order(relaxed)\0",
    b"ext_implementation_defined_requirement\0",
];

const REDUCTION_MODIFIER_TASK: u32 = 1 << 0;
const REDUCTION_MODIFIER_INSCAN: u32 = 1 << 1;
const REDUCTION_MODIFIER_DEFAULT: u32 = 1 << 2;
const SCHEDULE_MODIFIER_MONOTONIC: u32 = 1 << 0;
const SCHEDULE_MODIFIER_NONMONOTONIC: u32 = 1 << 1;
const SCHEDULE_MODIFIER_SIMD: u32 = 1 << 2;
const UNKNOWN_KIND: i32 = -1;
// OpenMP clause kind codes (match compat/ompparser OpenMPClauseKind order)
const CLAUSE_KIND_IF: i32 = 1;
const CLAUSE_KIND_NUM_THREADS: i32 = 2;
const CLAUSE_KIND_DEFAULT: i32 = 3;
const CLAUSE_KIND_PRIVATE: i32 = 4;
const CLAUSE_KIND_FIRSTPRIVATE: i32 = 5;
const CLAUSE_KIND_SHARED: i32 = 6;
const CLAUSE_KIND_COPYIN: i32 = 7;
const CLAUSE_KIND_ALIGN: i32 = 8;
const CLAUSE_KIND_REDUCTION: i32 = 9;
const CLAUSE_KIND_PROC_BIND: i32 = 10;
const CLAUSE_KIND_ALLOCATE: i32 = 11;
const CLAUSE_KIND_NUM_TEAMS: i32 = 12;
const CLAUSE_KIND_THREAD_LIMIT: i32 = 13;
const CLAUSE_KIND_LASTPRIVATE: i32 = 14;
const CLAUSE_KIND_COLLAPSE: i32 = 15;
const CLAUSE_KIND_ORDERED: i32 = 16;
const CLAUSE_KIND_PARTIAL: i32 = 17;
const CLAUSE_KIND_NOWAIT: i32 = 18;
const CLAUSE_KIND_FULL: i32 = 19;
const CLAUSE_KIND_ORDER: i32 = 20;
const CLAUSE_KIND_LINEAR: i32 = 21;
const CLAUSE_KIND_SCHEDULE: i32 = 22;
const CLAUSE_KIND_SAFELEN: i32 = 23;
const CLAUSE_KIND_SIMDLEN: i32 = 24;
const CLAUSE_KIND_ALIGNED: i32 = 25;
const CLAUSE_KIND_NONTEMPORAL: i32 = 26;
const CLAUSE_KIND_UNIFORM: i32 = 27;
const CLAUSE_KIND_INBRANCH: i32 = 28;
const CLAUSE_KIND_NOTINBRANCH: i32 = 29;
const CLAUSE_KIND_DIST_SCHEDULE: i32 = 30;
const CLAUSE_KIND_BIND: i32 = 31;
const CLAUSE_KIND_INCLUSIVE: i32 = 32;
const CLAUSE_KIND_EXCLUSIVE: i32 = 33;
const CLAUSE_KIND_COPYPRIVATE: i32 = 34;
const CLAUSE_KIND_PARALLEL: i32 = 35;
const CLAUSE_KIND_SECTIONS: i32 = 36;
const CLAUSE_KIND_FOR: i32 = 37;
const CLAUSE_KIND_DO: i32 = 38;
const CLAUSE_KIND_TASKGROUP: i32 = 39;
const CLAUSE_KIND_ALLOCATOR: i32 = 40;
const CLAUSE_KIND_INITIALIZER: i32 = 41;
const CLAUSE_KIND_FINAL: i32 = 42;
const CLAUSE_KIND_UNTIED: i32 = 43;
const CLAUSE_KIND_REQUIRES: i32 = 44;
const CLAUSE_KIND_MERGEABLE: i32 = 45;
const CLAUSE_KIND_IN_REDUCTION: i32 = 46;
const CLAUSE_KIND_DEPEND: i32 = 47;
const CLAUSE_KIND_PRIORITY: i32 = 48;
const CLAUSE_KIND_AFFINITY: i32 = 49;
const CLAUSE_KIND_DETACH: i32 = 50;
const CLAUSE_KIND_GRAINSIZE: i32 = 51;
const CLAUSE_KIND_NUM_TASKS: i32 = 52;
const CLAUSE_KIND_NOGROUP: i32 = 53;
const CLAUSE_KIND_REVERSE_OFFLOAD: i32 = 54;
const CLAUSE_KIND_UNIFIED_ADDRESS: i32 = 55;
const CLAUSE_KIND_UNIFIED_SHARED_MEMORY: i32 = 56;
const CLAUSE_KIND_ATOMIC_DEFAULT_MEM_ORDER: i32 = 57;
const CLAUSE_KIND_DYNAMIC_ALLOCATORS: i32 = 58;
const CLAUSE_KIND_SELF: i32 = 59;
const CLAUSE_KIND_EXT_IMPLEMENTATION_DEFINED_REQUIREMENT: i32 = 60;
const CLAUSE_KIND_DEVICE: i32 = 61;
const CLAUSE_KIND_MAP: i32 = 62;
const CLAUSE_KIND_USE_DEVICE_PTR: i32 = 63;
const CLAUSE_KIND_SIZES: i32 = 64;
const CLAUSE_KIND_USE_DEVICE_ADDR: i32 = 65;
const CLAUSE_KIND_IS_DEVICE_PTR: i32 = 66;
const CLAUSE_KIND_HAS_DEVICE_ADDR: i32 = 67;
const CLAUSE_KIND_DEFAULTMAP: i32 = 68;
const CLAUSE_KIND_TO: i32 = 69;
const CLAUSE_KIND_FROM: i32 = 70;
const CLAUSE_KIND_USES_ALLOCATORS: i32 = 71;
const CLAUSE_KIND_WHEN: i32 = 72;
const CLAUSE_KIND_MATCH: i32 = 73;
const CLAUSE_KIND_LINK: i32 = 74;
const CLAUSE_KIND_DEVICE_TYPE: i32 = 75;
const CLAUSE_KIND_TASK_REDUCTION: i32 = 76;
const CLAUSE_KIND_ACQ_REL: i32 = 77;
const CLAUSE_KIND_RELEASE: i32 = 78;
const CLAUSE_KIND_ACQUIRE: i32 = 79;
const CLAUSE_KIND_ATOMIC_READ: i32 = 80;
const CLAUSE_KIND_ATOMIC_WRITE: i32 = 81;
const CLAUSE_KIND_ATOMIC_UPDATE: i32 = 82;
const CLAUSE_KIND_ATOMIC_CAPTURE: i32 = 83;
const CLAUSE_KIND_SEQ_CST: i32 = 84;
const CLAUSE_KIND_RELAXED: i32 = 85;
const CLAUSE_KIND_HINT: i32 = 86;
const CLAUSE_KIND_DESTROY: i32 = 87;
const CLAUSE_KIND_DEPOBJ_UPDATE: i32 = 88;
const CLAUSE_KIND_THREADS: i32 = 89;
const CLAUSE_KIND_SIMD: i32 = 90;
const CLAUSE_KIND_FILTER: i32 = 91;
#[allow(dead_code)] // still used by header generation; runtime uses AST constants
const CLAUSE_KIND_COMPARE: i32 = 92;
#[allow(dead_code)] // still used by header generation; runtime uses AST constants
const CLAUSE_KIND_COMPARE_CAPTURE: i32 = 92;
const CLAUSE_KIND_OTHERWISE: i32 = 102;
const CLAUSE_KIND_FAIL: i32 = 103;
const CLAUSE_KIND_WEAK: i32 = 104;
const CLAUSE_KIND_AT: i32 = 105;
const CLAUSE_KIND_SEVERITY: i32 = 106;
const CLAUSE_KIND_MESSAGE: i32 = 107;
const CLAUSE_KIND_DOACROSS: i32 = 108;
const CLAUSE_KIND_ABSENT: i32 = 109;
const CLAUSE_KIND_CONTAINS: i32 = 110;
const CLAUSE_KIND_HOLDS: i32 = 111;
const CLAUSE_KIND_GRAPH_ID: i32 = 112;
const CLAUSE_KIND_GRAPH_RESET: i32 = 113;
const CLAUSE_KIND_TRANSPARENT: i32 = 114;
const CLAUSE_KIND_REPLAYABLE: i32 = 115;
const CLAUSE_KIND_THREADSET: i32 = 116;
const CLAUSE_KIND_INDIRECT: i32 = 117;
const CLAUSE_KIND_LOCAL: i32 = 118;
const CLAUSE_KIND_INIT: i32 = 119;
const CLAUSE_KIND_INIT_COMPLETE: i32 = 120;
const CLAUSE_KIND_SAFESYNC: i32 = 121;
const CLAUSE_KIND_DEVICE_SAFESYNC: i32 = 122;
const CLAUSE_KIND_MEMSCOPE: i32 = 123;
const CLAUSE_KIND_LOOPRANGE: i32 = 124;
const CLAUSE_KIND_PERMUTATION: i32 = 125;
const CLAUSE_KIND_COUNTS: i32 = 126;
const CLAUSE_KIND_INDUCTION: i32 = 127;
const CLAUSE_KIND_INDUCTOR: i32 = 128;
const CLAUSE_KIND_COLLECTOR: i32 = 129;
const CLAUSE_KIND_COMBINER: i32 = 130;
const CLAUSE_KIND_ADJUST_ARGS: i32 = 131;
const CLAUSE_KIND_APPEND_ARGS: i32 = 132;
const CLAUSE_KIND_APPLY: i32 = 133;
const CLAUSE_KIND_NOOPENMP: i32 = 134;
const CLAUSE_KIND_NOOPENMP_CONSTRUCTS: i32 = 135;
const CLAUSE_KIND_NOOPENMP_ROUTINES: i32 = 136;
const CLAUSE_KIND_NOPARALLELISM: i32 = 137;
const CLAUSE_KIND_NOCONTEXT: i32 = 138;
const CLAUSE_KIND_NOVARIANTS: i32 = 139;
const CLAUSE_KIND_ENTER: i32 = 140;
const CLAUSE_KIND_USE: i32 = 141;
const ROUP_OMPA_USES_ALLOCATOR_DEFAULT: i32 = 0;
const ROUP_OMPA_USES_ALLOCATOR_LARGE_CAP: i32 = 1;
const ROUP_OMPA_USES_ALLOCATOR_CONST: i32 = 2;
const ROUP_OMPA_USES_ALLOCATOR_HIGH_BW: i32 = 3;
const ROUP_OMPA_USES_ALLOCATOR_LOW_LAT: i32 = 4;
const ROUP_OMPA_USES_ALLOCATOR_CGROUP: i32 = 5;
const ROUP_OMPA_USES_ALLOCATOR_PTEAM: i32 = 6;
const ROUP_OMPA_USES_ALLOCATOR_THREAD: i32 = 7;
const ROUP_OMPA_USES_ALLOCATOR_USER: i32 = 8;

// If-clause directive-name modifier codes
const ROUP_IF_MODIFIER_UNSPECIFIED: i32 = -1;
const ROUP_IF_MODIFIER_PARALLEL: i32 = 0;
const ROUP_IF_MODIFIER_TASK: i32 = 1;
const ROUP_IF_MODIFIER_TASKLOOP: i32 = 2;
const ROUP_IF_MODIFIER_TARGET: i32 = 3;
const ROUP_IF_MODIFIER_TARGET_DATA: i32 = 4;
const ROUP_IF_MODIFIER_TARGET_ENTER_DATA: i32 = 5;
const ROUP_IF_MODIFIER_TARGET_EXIT_DATA: i32 = 6;
const ROUP_IF_MODIFIER_TARGET_UPDATE: i32 = 7;
const ROUP_IF_MODIFIER_SIMD: i32 = 8;
const ROUP_IF_MODIFIER_CANCEL: i32 = 9;
const ROUP_IF_MODIFIER_USER: i32 = 10;

// Directive parameter classifications (for enum-based access from compat)
const ROUP_DIRECTIVE_PARAM_NONE: i32 = 0;
const ROUP_DIRECTIVE_PARAM_IDENTIFIER_LIST: i32 = 1;
const ROUP_DIRECTIVE_PARAM_IDENTIFIER: i32 = 2;
const ROUP_DIRECTIVE_PARAM_MAPPER: i32 = 3;
const ROUP_DIRECTIVE_PARAM_VARIANT_FUNCTION: i32 = 4;
const ROUP_DIRECTIVE_PARAM_DEPOBJ: i32 = 5;
const ROUP_DIRECTIVE_PARAM_SCAN: i32 = 6;
const ROUP_DIRECTIVE_PARAM_CONSTRUCT: i32 = 7;
const ROUP_DIRECTIVE_PARAM_CRITICAL: i32 = 8;
const ROUP_DIRECTIVE_PARAM_DECLARE_REDUCTION: i32 = 10;
const ROUP_DIRECTIVE_PARAM_DECLARE_SIMD: i32 = 11;

// Scan directive modes
const ROUP_SCAN_MODE_UNSPECIFIED: i32 = -1;
const ROUP_SCAN_MODE_EXCLUSIVE: i32 = 0;
const ROUP_SCAN_MODE_INCLUSIVE: i32 = 1;

// Cancel / cancellation-point construct parameter codes
const ROUP_CONSTRUCT_TYPE_UNKNOWN: i32 = -1;
const ROUP_CONSTRUCT_TYPE_PARALLEL: i32 = 0;
const ROUP_CONSTRUCT_TYPE_SECTIONS: i32 = 1;
const ROUP_CONSTRUCT_TYPE_FOR: i32 = 2;
const ROUP_CONSTRUCT_TYPE_TASKGROUP: i32 = 3;
const ROUP_CONSTRUCT_TYPE_OTHER: i32 = 4;

/// Iterator over clauses
///
/// Provides sequential access to directive's clauses.
/// Holds raw pointers to avoid ownership issues at FFI boundary.
#[repr(C)]
pub struct OmpClauseIterator {
    clauses: Vec<*const OmpClause>, // Pointers to clauses
    index: usize,                   // Current position
}

/// List of strings (for variable names in clauses)
///
/// Used for private, shared, reduction variable lists.
#[repr(C)]
pub struct OmpStringList {
    items: Vec<*const c_char>, // NULL-terminated C strings
}

thread_local! {
    static CURRENT_CLAUSE_LANGUAGE: Cell<Language> = const { Cell::new(Language::C) };
}

fn with_clause_language<T>(language: Language, f: impl FnOnce() -> T) -> T {
    CURRENT_CLAUSE_LANGUAGE.with(|cell| {
        let previous = cell.replace(language);
        let result = f();
        cell.set(previous);
        result
    })
}

fn current_clause_language() -> Language {
    CURRENT_CLAUSE_LANGUAGE.with(|cell| cell.get())
}

// ============================================================================
// Parse Function (UNSAFE BLOCK 1-2)
// ============================================================================

/// Parse an OpenMP directive from a C string.
///
/// ## Parameters
/// - `input`: Null-terminated C string containing the directive
///
/// ## Returns
/// - Pointer to `OmpDirective` on success
/// - NULL on parse failure or NULL input
///
/// ## Safety
/// Caller must:
/// - Pass valid null-terminated C string or NULL
/// - Call `roup_directive_free()` on the returned pointer
///
/// ## Example
/// ```c
/// OmpDirective* dir = roup_parse("#pragma omp parallel");
/// if (dir) {
///     // use directive
///     roup_directive_free(dir);
/// }
/// ```
#[no_mangle]
pub extern "C" fn roup_parse(input: *const c_char) -> *mut OmpDirective {
    // NULL check
    if input.is_null() {
        return ptr::null_mut();
    }

    // UNSAFE BLOCK 1: Convert C string to Rust &str
    // Safety: Caller guarantees valid null-terminated C string
    let c_str = unsafe { CStr::from_ptr(input) };

    let rust_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(), // Invalid UTF-8
    };

    let parser = openmp::parser();
    let ast = match parser.parse_ast(
        rust_str,
        ClauseNormalizationMode::Disabled,
        &ParserConfig::default(),
    ) {
        Ok(dir) => dir,
        Err(err) => {
            if std::env::var_os("ROUP_DEBUG_PARSE").is_some() {
                eprintln!("ROUP parse failed: {err}");
            }
            return ptr::null_mut();
        }
    };

    let omp_ast = match ast.body {
        DirectiveBody::OpenMp(d) => d,
        _ => return ptr::null_mut(),
    };

    let c_directive = build_c_api_directive_from_ast(&omp_ast, Language::C);

    Box::into_raw(Box::new(c_directive))
}

/// Free a directive allocated by `roup_parse()`.
///
/// ## Safety
/// - Must only be called once per directive
/// - Pointer must be from `roup_parse()`
/// - Do not use pointer after calling this function
#[no_mangle]
pub extern "C" fn roup_directive_free(directive: *mut OmpDirective) {
    if directive.is_null() {
        return;
    }

    // UNSAFE BLOCK 3: Convert raw pointer back to Box for deallocation
    // Safety: Pointer came from Box::into_raw in roup_parse
    unsafe {
        let boxed = Box::from_raw(directive);

        // Free the name string (was allocated with CString::into_raw)
        if !boxed.name.is_null() {
            drop(CString::from_raw(boxed.name as *mut c_char));
        }

        // Free the parameter string if present
        if !boxed.parameter.is_null() {
            drop(CString::from_raw(boxed.parameter as *mut c_char));
        }

        if !boxed.parameter_data.identifiers.is_null() {
            roup_string_list_free(boxed.parameter_data.identifiers);
        }

        // Free clause data
        for clause in &boxed.clauses {
            free_clause_data(clause);
        }

        // Box is dropped here, freeing memory
    }
}

/// Parse an OpenMP directive with explicit language specification.
///
/// ## Parameters
/// - `input`: Null-terminated string containing the directive
/// - `language`: Language format (ROUP_LANG_C, ROUP_LANG_FORTRAN_FREE, ROUP_LANG_FORTRAN_FIXED)
///
/// ## Returns
/// - Pointer to `OmpDirective` on success
/// - `NULL` on error:
///   - `input` is NULL
///   - `language` is not a valid ROUP_LANG_* constant
///   - `input` contains invalid UTF-8
///   - Parse error (invalid OpenMP directive syntax)
///
/// ## Error Handling
/// This function returns `NULL` for all error conditions without detailed error codes.
/// There is no way to distinguish between different error types (invalid language,
/// NULL input, UTF-8 error, or parse failure) from the return value alone.
///
/// Callers should:
/// - Validate `language` parameter before calling (use only ROUP_LANG_* constants)
/// - Ensure `input` is non-NULL and valid UTF-8
/// - Verify directive syntax is correct
/// - For debugging, enable logging or use a separate validation layer
///
/// For a version with detailed error codes, consider using the Rust API directly.
///
/// ## Example (Fortran free-form)
/// ```c
/// OmpDirective* dir = roup_parse_with_language("!$OMP PARALLEL PRIVATE(A)", ROUP_LANG_FORTRAN_FREE);
/// if (dir) {
///     // Use directive
///     roup_directive_free(dir);
/// } else {
///     // Handle error: NULL, invalid language, invalid UTF-8, or parse failure
///     fprintf(stderr, "Failed to parse directive\n");
/// }
/// ```
#[no_mangle]
pub extern "C" fn roup_parse_with_language(
    input: *const c_char,
    language: i32,
) -> *mut OmpDirective {
    // NULL check
    if input.is_null() {
        return ptr::null_mut();
    }

    // Convert language code to Language enum using explicit constants
    // Return NULL for invalid language values
    let lang = match language {
        ROUP_LANG_C => Language::C,
        ROUP_LANG_FORTRAN_FREE => Language::FortranFree,
        ROUP_LANG_FORTRAN_FIXED => Language::FortranFixed,
        _ => return ptr::null_mut(), // Invalid language value
    };

    // UNSAFE BLOCK: Convert C string to Rust &str
    let c_str = unsafe { CStr::from_ptr(input) };

    let rust_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let parser = openmp::parser().with_language(lang);
    let ast = match parser.parse_ast(
        rust_str,
        ClauseNormalizationMode::Disabled,
        &ParserConfig::default(),
    ) {
        Ok(dir) => dir,
        Err(err) => {
            if std::env::var_os("ROUP_DEBUG_PARSE").is_some() {
                eprintln!("ROUP parse failed: {err}");
            }
            return ptr::null_mut();
        }
    };

    let omp_ast = match ast.body {
        DirectiveBody::OpenMp(d) => d,
        _ => return ptr::null_mut(),
    };

    let c_directive = build_c_api_directive_from_ast(&omp_ast, lang);

    Box::into_raw(Box::new(c_directive))
}

// ============================================================================
// Translation Functions (C/C++ ↔ Fortran)
// ============================================================================

/// Convert an OpenMP directive from one language to another.
///
/// This function translates OpenMP directives between C/C++ and Fortran syntax:
/// - Sentinel conversion: `#pragma omp` ↔ `!$omp`
/// - Loop directive names: `for` ↔ `do`, `parallel for` ↔ `parallel do`, etc.
/// - Clause preservation: All clauses are preserved as-is
///
/// ## Parameters
/// - `input`: Null-terminated string containing the directive
/// - `from_language`: Source language (ROUP_LANG_C, ROUP_LANG_FORTRAN_FREE, ROUP_LANG_FORTRAN_FIXED)
/// - `to_language`: Target language (ROUP_LANG_C, ROUP_LANG_FORTRAN_FREE)
///
/// ## Returns
/// - Pointer to null-terminated C string with translated directive on success
/// - `NULL` on error:
///   - `input` is NULL
///   - `from_language` or `to_language` is invalid
///   - `input` contains invalid UTF-8
///   - Parse error (invalid OpenMP directive syntax)
///
/// ## Memory Management
/// The returned string is heap-allocated and must be freed by calling `roup_string_free()`.
///
/// ## Limitations
/// - **Expression translation**: Expressions within clauses are NOT translated
///   (e.g., C syntax like `arr[i]` remains unchanged)
/// - **Fixed-form output**: Only free-form `!$omp` output is supported (ROUP_LANG_FORTRAN_FIXED not supported as target)
/// - **Surrounding code**: Only directive lines are translated, not actual source code
///
/// ## Example (C to Fortran)
/// ```c
/// const char* c_input = "#pragma omp parallel for private(i) schedule(static, 4)";
/// char* fortran = roup_convert_language(c_input, ROUP_LANG_C, ROUP_LANG_FORTRAN_FREE);
/// if (fortran) {
///     printf("%s\n", fortran);  // Output: !$omp parallel do private(i) schedule(static, 4)
///     roup_string_free(fortran);
/// }
/// ```
///
/// ## Example (Fortran to C)
/// ```c
/// const char* fortran_input = "!$omp parallel do private(i)";
/// char* c_output = roup_convert_language(fortran_input, ROUP_LANG_FORTRAN_FREE, ROUP_LANG_C);
/// if (c_output) {
///     printf("%s\n", c_output);  // Output: #pragma omp parallel for private(i)
///     roup_string_free(c_output);
/// }
/// ```
#[no_mangle]
pub extern "C" fn roup_convert_language(
    input: *const c_char,
    from_language: i32,
    to_language: i32,
) -> *mut c_char {
    // NULL check
    if input.is_null() {
        return ptr::null_mut();
    }

    // Convert language codes to IR Language enum
    let from_lang = match language_code_to_ir_language(from_language) {
        Some(lang) => lang,
        None => return ptr::null_mut(), // Invalid from_language
    };

    let to_lang = match language_code_to_ir_language(to_language) {
        Some(lang) => lang,
        None => return ptr::null_mut(), // Invalid to_language
    };

    // Convert C string to Rust &str
    let c_str = unsafe { CStr::from_ptr(input) };

    let rust_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(), // Invalid UTF-8
    };

    // Return early if input is empty
    if rust_str.trim().is_empty() {
        return ptr::null_mut();
    }

    // Convert from_language code to lexer::Language for parser
    // IMPORTANT: Map language code directly to preserve fixed-form vs free-form distinction
    let lexer_lang = match from_language {
        ROUP_LANG_C => Language::C,
        ROUP_LANG_FORTRAN_FREE => Language::FortranFree,
        ROUP_LANG_FORTRAN_FIXED => Language::FortranFixed,
        _ => return ptr::null_mut(), // Invalid from_language
    };

    // Parse the directive with the source language
    let parser = openmp::parser().with_language(lexer_lang);
    let (rest, directive) = match parser.parse(rust_str) {
        Ok(result) => result,
        Err(_) => return ptr::null_mut(), // Parse error
    };

    // Check for unparsed trailing input
    if !rest.trim().is_empty() {
        return ptr::null_mut();
    }

    // Convert to IR with source language context
    let config = ParserConfig::with_parsing(from_lang);
    let ir = match convert_directive(&directive, SourceLocation::start(), from_lang, &config) {
        Ok(ir) => ir,
        Err(_) => return ptr::null_mut(), // Conversion error
    };

    // Translate to target language
    // Note: We don't modify the IR's language field, just render in target language
    let output_str = ir.to_string_for_language(to_lang);

    // Allocate C string for return
    match CString::new(output_str) {
        Ok(c_string) => c_string.into_raw(),
        Err(_) => ptr::null_mut(), // String contains null bytes (shouldn't happen)
    }
}

/// Free a string allocated by `roup_convert_language()`.
///
/// ## Parameters
/// - `ptr`: Pointer to string returned by `roup_convert_language()`
///
/// ## Safety
/// - Must only be called once per string
/// - Pointer must be from `roup_convert_language()`
/// - Do not use pointer after calling this function
///
/// ## Example
/// ```c
/// char* output = roup_convert_language(input, ROUP_LANG_C, ROUP_LANG_FORTRAN_FREE);
/// if (output) {
///     printf("%s\n", output);
///     roup_string_free(output);
/// }
/// ```
#[no_mangle]
pub extern "C" fn roup_string_free(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }

    // UNSAFE: Convert raw pointer back to CString for deallocation
    // Safety: Pointer came from CString::into_raw in roup_convert_language
    unsafe {
        drop(CString::from_raw(ptr));
    }
}

/// Free a clause.
#[no_mangle]
pub extern "C" fn roup_clause_free(clause: *mut OmpClause) {
    if clause.is_null() {
        return;
    }

    unsafe {
        let boxed = Box::from_raw(clause);
        free_clause_data(&boxed);
    }
}

// ============================================================================
// Directive Query Functions (All Safe)
// ============================================================================

/// Get directive kind.
///
/// Returns -1 if directive is NULL.
#[no_mangle]
pub extern "C" fn roup_directive_kind(directive: *const OmpDirective) -> i32 {
    if directive.is_null() {
        return -1;
    }

    // UNSAFE BLOCK 4: Dereference pointer
    // Safety: Caller guarantees valid pointer from roup_parse
    unsafe {
        let dir = &*directive;

        // Use the canonical lookup to map the stored directive name into
        // a `DirectiveName` enum, then map that enum to the C integer code.
        if dir.name.is_null() {
            return -1;
        }

        let c_str = CStr::from_ptr(dir.name);
        let name_str = match c_str.to_str() {
            Ok(s) => s,
            Err(_) => return -1,
        };

        let dname = lookup_directive_name(name_str);
        directive_name_enum_to_kind(dname)
    }
}

// See `directive_name_enum_to_kind` below for the canonical mapping of
// `DirectiveName` -> integer codes. Unknown/unhandled directives return -1.

/// Get directive name as a C string.
///
/// Returns NULL if directive is NULL.
/// Returned pointer is valid until directive is freed.
#[no_mangle]
pub extern "C" fn roup_directive_name(directive: *const OmpDirective) -> *const c_char {
    if directive.is_null() {
        return ptr::null();
    }

    unsafe {
        let dir = &*directive;
        dir.name
    }
}

/// Get directive parameter as a C string (e.g., "(a,b,c)" for allocate/threadprivate).
///
/// Returns NULL if directive is NULL or has no parameter.
/// Returned pointer is valid until directive is freed.
#[no_mangle]
pub extern "C" fn roup_directive_parameter(directive: *const OmpDirective) -> *const c_char {
    if directive.is_null() {
        return ptr::null();
    }

    unsafe {
        let dir = &*directive;
        dir.parameter
    }
}

/// Get structured parameter kind for a directive (enum-based).
#[no_mangle]
pub extern "C" fn roup_directive_parameter_kind(directive: *const OmpDirective) -> i32 {
    if directive.is_null() {
        return ROUP_DIRECTIVE_PARAM_NONE;
    }

    unsafe { (&*directive).parameter_data.kind }
}

/// Scan directive mode (inclusive/exclusive) when parameter_kind == SCAN.
#[no_mangle]
pub extern "C" fn roup_directive_parameter_scan_mode(directive: *const OmpDirective) -> i32 {
    if directive.is_null() {
        return ROUP_SCAN_MODE_UNSPECIFIED;
    }

    unsafe { (&*directive).parameter_data.scan_mode }
}

/// Cancel/cancellation-point construct type when parameter_kind == CONSTRUCT.
#[no_mangle]
pub extern "C" fn roup_directive_parameter_construct_type(directive: *const OmpDirective) -> i32 {
    if directive.is_null() {
        return ROUP_CONSTRUCT_TYPE_UNKNOWN;
    }

    unsafe { (&*directive).parameter_data.construct_type }
}

/// Clone directive parameter identifier list (caller frees with `roup_string_list_free`).
#[no_mangle]
pub extern "C" fn roup_directive_parameter_identifiers(
    directive: *const OmpDirective,
) -> *mut OmpStringList {
    if directive.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let dir = &*directive;
        if dir.parameter_data.identifiers.is_null() {
            ptr::null_mut()
        } else {
            clone_string_list(dir.parameter_data.identifiers)
        }
    }
}

/// Get number of clauses in a directive.
///
/// Returns 0 if directive is NULL.
#[no_mangle]
pub extern "C" fn roup_directive_clause_count(directive: *const OmpDirective) -> i32 {
    if directive.is_null() {
        return 0;
    }

    unsafe {
        let dir = &*directive;
        dir.clauses.len() as i32
    }
}

/// Create an iterator over directive clauses.
///
/// Returns NULL if directive is NULL.
/// Caller must call `roup_clause_iterator_free()`.
#[no_mangle]
pub extern "C" fn roup_directive_clauses_iter(
    directive: *const OmpDirective,
) -> *mut OmpClauseIterator {
    if directive.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let dir = &*directive;
        let iter = OmpClauseIterator {
            clauses: dir.clauses.iter().map(|c| c as *const OmpClause).collect(),
            index: 0,
        };
        Box::into_raw(Box::new(iter))
    }
}

// ============================================================================
// Iterator Functions (UNSAFE BLOCKS 5-6)
// ============================================================================

/// Get next clause from iterator.
///
/// ## Parameters
/// - `iter`: Iterator from `roup_directive_clauses_iter()`
/// - `out`: Output pointer to write clause pointer
///
/// ## Returns
/// - 1 if clause available (written to `out`)
/// - 0 if no more clauses or NULL inputs
#[no_mangle]
pub extern "C" fn roup_clause_iterator_next(
    iter: *mut OmpClauseIterator,
    out: *mut *const OmpClause,
) -> i32 {
    // NULL checks
    if iter.is_null() || out.is_null() {
        return 0;
    }

    // UNSAFE BLOCK 5: Dereference iterator
    // Safety: Caller guarantees valid iterator pointer
    unsafe {
        let iterator = &mut *iter;

        if iterator.index >= iterator.clauses.len() {
            return 0; // No more items
        }

        let clause_ptr = iterator.clauses[iterator.index];
        iterator.index += 1;

        // UNSAFE BLOCK 6: Write to output pointer
        // Safety: Caller guarantees valid output pointer
        *out = clause_ptr;
        1
    }
}

/// Free clause iterator.
#[no_mangle]
pub extern "C" fn roup_clause_iterator_free(iter: *mut OmpClauseIterator) {
    if iter.is_null() {
        return;
    }

    unsafe {
        drop(Box::from_raw(iter));
    }
}

// ============================================================================
// Clause Query Functions (UNSAFE BLOCKS 7-8)
// ============================================================================

/// Get clause kind.
///
/// Returns -1 if clause is NULL.
#[no_mangle]
pub extern "C" fn roup_clause_kind(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return -1;
    }

    // UNSAFE BLOCK 7: Dereference clause
    // Safety: Caller guarantees valid clause pointer
    unsafe {
        let c = &*clause;
        c.kind
    }
}

/// Get schedule kind from schedule clause.
///
/// Returns -1 if clause is NULL or not a schedule clause.
#[no_mangle]
pub extern "C" fn roup_clause_schedule_kind(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return -1;
    }

    unsafe {
        let c = &*clause;
        if c.kind != CLAUSE_KIND_SCHEDULE {
            // Not a schedule clause
            return -1;
        }
        c.data.schedule.kind
    }
}

/// Get schedule modifier mask (monotonic/nonmonotonic/simd) from schedule clause.
#[no_mangle]
pub extern "C" fn roup_clause_schedule_modifier_mask(clause: *const OmpClause) -> u32 {
    if clause.is_null() {
        return 0;
    }

    unsafe {
        let c = &*clause;
        if c.kind != CLAUSE_KIND_SCHEDULE {
            return 0;
        }
        c.data.schedule.modifiers
    }
}

/// Get schedule chunk expression as a string (NULL if not present).
#[no_mangle]
pub extern "C" fn roup_clause_schedule_chunk(clause: *const OmpClause) -> *const c_char {
    if clause.is_null() {
        return ptr::null();
    }

    unsafe {
        let c = &*clause;
        if c.kind != CLAUSE_KIND_SCHEDULE {
            return ptr::null();
        }
        c.data.schedule.chunk
    }
}

/// Get dist_schedule kind (static/user).
///
/// Returns -1 if clause is NULL or not a dist_schedule clause.
#[no_mangle]
pub extern "C" fn roup_clause_dist_schedule_kind(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return -1;
    }

    unsafe {
        let c = &*clause;
        if c.kind != CLAUSE_KIND_DIST_SCHEDULE {
            return -1;
        }
        c.data.dist_schedule.kind
    }
}

/// Get dist_schedule chunk expression (NULL if absent).
#[no_mangle]
pub extern "C" fn roup_clause_dist_schedule_chunk(clause: *const OmpClause) -> *const c_char {
    if clause.is_null() {
        return ptr::null();
    }

    unsafe {
        let c = &*clause;
        if c.kind != CLAUSE_KIND_DIST_SCHEDULE {
            return ptr::null();
        }
        c.data.dist_schedule.chunk
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_order_modifier(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return -1;
    }
    unsafe {
        get_order_data(&*clause)
            .map(|data| data.modifier)
            .unwrap_or(-1)
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_order_kind(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return -1;
    }
    unsafe { get_order_data(&*clause).map(|data| data.kind).unwrap_or(-1) }
}

/// Get if-clause directive-name modifier code.
///
/// Returns -1 if clause is NULL or not an if clause.
#[no_mangle]
pub extern "C" fn roup_clause_if_modifier(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return -1;
    }
    unsafe {
        let c = &*clause;
        if c.kind != CLAUSE_KIND_IF {
            return -1;
        }
        c.data.default
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_map_type(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return MAP_TYPE_UNSPECIFIED;
    }
    unsafe {
        get_map_data(&*clause)
            .map(|data| data.map_type)
            .unwrap_or(MAP_TYPE_UNSPECIFIED)
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_map_modifier_mask(clause: *const OmpClause) -> u32 {
    if clause.is_null() {
        return 0;
    }
    unsafe {
        get_map_data(&*clause)
            .map(|data| data.modifiers)
            .unwrap_or(0)
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_map_mapper(clause: *const OmpClause) -> *const c_char {
    if clause.is_null() {
        return ptr::null();
    }
    unsafe {
        get_map_data(&*clause)
            .map(|data| data.mapper)
            .unwrap_or(ptr::null())
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_map_iterator_count(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return 0;
    }
    unsafe {
        get_map_data(&*clause)
            .map(|data| data.iterators.len() as i32)
            .unwrap_or(0)
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_map_iterator_type(
    clause: *const OmpClause,
    index: i32,
) -> *const c_char {
    if clause.is_null() || index < 0 {
        return ptr::null();
    }
    unsafe {
        get_map_data(&*clause)
            .and_then(|data| data.iterators.get(index as usize))
            .map(|it| it.type_name)
            .unwrap_or(ptr::null())
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_map_iterator_name(
    clause: *const OmpClause,
    index: i32,
) -> *const c_char {
    if clause.is_null() || index < 0 {
        return ptr::null();
    }
    unsafe {
        get_map_data(&*clause)
            .and_then(|data| data.iterators.get(index as usize))
            .map(|it| it.name)
            .unwrap_or(ptr::null())
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_map_iterator_start(
    clause: *const OmpClause,
    index: i32,
) -> *const c_char {
    if clause.is_null() || index < 0 {
        return ptr::null();
    }
    unsafe {
        get_map_data(&*clause)
            .and_then(|data| data.iterators.get(index as usize))
            .map(|it| it.start)
            .unwrap_or(ptr::null())
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_map_iterator_end(
    clause: *const OmpClause,
    index: i32,
) -> *const c_char {
    if clause.is_null() || index < 0 {
        return ptr::null();
    }
    unsafe {
        get_map_data(&*clause)
            .and_then(|data| data.iterators.get(index as usize))
            .map(|it| it.end)
            .unwrap_or(ptr::null())
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_map_iterator_step(
    clause: *const OmpClause,
    index: i32,
) -> *const c_char {
    if clause.is_null() || index < 0 {
        return ptr::null();
    }
    unsafe {
        get_map_data(&*clause)
            .and_then(|data| data.iterators.get(index as usize))
            .map(|it| it.step)
            .unwrap_or(ptr::null())
    }
}

/// Get reduction operator from reduction clause.
///
/// Returns -1 if clause is NULL or not a reduction clause.
#[no_mangle]
pub extern "C" fn roup_clause_reduction_operator(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return -1;
    }

    unsafe {
        let c = &*clause;
        if let Some(data) = get_reduction_data(c) {
            data.operator
        } else {
            -1
        }
    }
}

/// Get reduction modifier mask (bitfield of task/inscan/default).
#[no_mangle]
pub extern "C" fn roup_clause_reduction_modifier_mask(clause: *const OmpClause) -> u32 {
    if clause.is_null() {
        return 0;
    }

    unsafe {
        let c = &*clause;
        if let Some(data) = get_reduction_data(c) {
            data.modifier_mask
        } else {
            0
        }
    }
}

/// Get user-defined identifier for reduction operator (if any).
#[no_mangle]
pub extern "C" fn roup_clause_reduction_user_identifier(clause: *const OmpClause) -> *const c_char {
    if clause.is_null() {
        return ptr::null();
    }

    unsafe {
        let c = &*clause;
        if let Some(data) = get_reduction_data(c) {
            data.user_identifier
        } else {
            ptr::null()
        }
    }
}

/// Get comma-separated modifier text for reduction clause (if any).
#[no_mangle]
pub extern "C" fn roup_clause_reduction_modifiers_text(clause: *const OmpClause) -> *const c_char {
    if clause.is_null() {
        return ptr::null();
    }

    unsafe {
        let c = &*clause;
        if let Some(data) = get_reduction_data(c) {
            data.modifiers_text
        } else {
            ptr::null()
        }
    }
}

/// Whether a space existed after the colon in the reduction clause.
#[no_mangle]
pub extern "C" fn roup_clause_reduction_space_after_colon(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return 0;
    }

    unsafe {
        let c = &*clause;
        if let Some(data) = get_reduction_data(c) {
            if data.space_after_colon {
                1
            } else {
                0
            }
        } else {
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_linear_modifier(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return -1;
    }

    unsafe {
        get_linear_data(&*clause)
            .map(|data| data.modifier)
            .unwrap_or(-1)
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_linear_step(clause: *const OmpClause) -> *const c_char {
    if clause.is_null() {
        return ptr::null();
    }

    unsafe {
        get_linear_data(&*clause)
            .map(|data| data.step)
            .unwrap_or(ptr::null())
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_grainsize_modifier(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return -1;
    }
    unsafe {
        let c = &*clause;
        if c.kind == CLAUSE_KIND_GRAINSIZE {
            c.data.default
        } else {
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_num_tasks_modifier(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return -1;
    }
    unsafe {
        let c = &*clause;
        if c.kind == CLAUSE_KIND_NUM_TASKS {
            c.data.default
        } else {
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_depend_type(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return -1;
    }

    unsafe {
        get_depend_data(&*clause)
            .map(|data| data.depend_type)
            .unwrap_or(-1)
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_depend_has_iterators(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return 0;
    }

    unsafe {
        get_depend_data(&*clause)
            .map(|data| (!data.iterators.is_empty()) as i32)
            .unwrap_or(0)
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_depend_iterators(clause: *const OmpClause) -> *mut OmpStringList {
    if clause.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        get_depend_data(&*clause).map_or(ptr::null_mut(), |data| {
            if data.iterators.is_empty() {
                return ptr::null_mut();
            }
            let defs: Vec<Cow<'_, str>> = data
                .iterators
                .iter()
                .filter_map(format_depend_iterator)
                .collect();
            build_string_list(&defs)
        })
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_depend_iterator_count(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return 0;
    }
    unsafe {
        get_depend_data(&*clause)
            .map(|data| data.iterators.len() as i32)
            .unwrap_or(0)
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_depend_iterator_type(
    clause: *const OmpClause,
    index: i32,
) -> *const c_char {
    if clause.is_null() || index < 0 {
        return ptr::null();
    }
    unsafe {
        get_depend_data(&*clause)
            .and_then(|data| data.iterators.get(index as usize))
            .map(|it| it.type_name)
            .unwrap_or(ptr::null())
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_depend_iterator_name(
    clause: *const OmpClause,
    index: i32,
) -> *const c_char {
    if clause.is_null() || index < 0 {
        return ptr::null();
    }
    unsafe {
        get_depend_data(&*clause)
            .and_then(|data| data.iterators.get(index as usize))
            .map(|it| it.name)
            .unwrap_or(ptr::null())
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_depend_iterator_start(
    clause: *const OmpClause,
    index: i32,
) -> *const c_char {
    if clause.is_null() || index < 0 {
        return ptr::null();
    }
    unsafe {
        get_depend_data(&*clause)
            .and_then(|data| data.iterators.get(index as usize))
            .map(|it| it.start)
            .unwrap_or(ptr::null())
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_depend_iterator_end(
    clause: *const OmpClause,
    index: i32,
) -> *const c_char {
    if clause.is_null() || index < 0 {
        return ptr::null();
    }
    unsafe {
        get_depend_data(&*clause)
            .and_then(|data| data.iterators.get(index as usize))
            .map(|it| it.end)
            .unwrap_or(ptr::null())
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_depend_iterator_step(
    clause: *const OmpClause,
    index: i32,
) -> *const c_char {
    if clause.is_null() || index < 0 {
        return ptr::null();
    }
    unsafe {
        get_depend_data(&*clause)
            .and_then(|data| data.iterators.get(index as usize))
            .map(|it| it.step)
            .unwrap_or(ptr::null())
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_allocate_allocator(clause: *const OmpClause) -> *const c_char {
    if clause.is_null() {
        return ptr::null();
    }
    unsafe {
        get_allocate_data(&*clause)
            .map(|data| data.allocator)
            .unwrap_or(ptr::null())
    }
}

/// Get allocate clause allocator classification code.
#[no_mangle]
pub extern "C" fn roup_clause_allocate_kind(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return -1;
    }
    unsafe {
        get_allocate_data(&*clause)
            .map(|data| data.kind)
            .unwrap_or(-1)
    }
}

/// Get aligned clause alignment expression (NULL when absent).
#[no_mangle]
pub extern "C" fn roup_clause_aligned_alignment(clause: *const OmpClause) -> *const c_char {
    if clause.is_null() {
        return ptr::null();
    }
    unsafe {
        if let Some(data) = get_aligned_data(&*clause) {
            data.alignment
        } else {
            ptr::null()
        }
    }
}

/// Get allocator clause classification code.
#[no_mangle]
pub extern "C" fn roup_clause_allocator_kind(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return -1;
    }
    unsafe {
        let c = &*clause;
        if c.kind != CLAUSE_KIND_ALLOCATOR {
            return -1;
        }
        c.data.default
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_affinity_modifier(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return -1;
    }
    unsafe {
        get_affinity_data(&*clause)
            .map(|data| data.modifier)
            .unwrap_or(-1)
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_affinity_iterator_count(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return 0;
    }
    unsafe {
        get_affinity_data(&*clause)
            .map(|data| data.iterators.len() as i32)
            .unwrap_or(0)
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_affinity_iterator_type(
    clause: *const OmpClause,
    index: i32,
) -> *const c_char {
    if clause.is_null() || index < 0 {
        return ptr::null();
    }
    unsafe {
        get_affinity_data(&*clause)
            .and_then(|data| data.iterators.get(index as usize))
            .map(|it| it.type_name)
            .unwrap_or(ptr::null())
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_affinity_iterator_name(
    clause: *const OmpClause,
    index: i32,
) -> *const c_char {
    if clause.is_null() || index < 0 {
        return ptr::null();
    }
    unsafe {
        get_affinity_data(&*clause)
            .and_then(|data| data.iterators.get(index as usize))
            .map(|it| it.name)
            .unwrap_or(ptr::null())
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_affinity_iterator_start(
    clause: *const OmpClause,
    index: i32,
) -> *const c_char {
    if clause.is_null() || index < 0 {
        return ptr::null();
    }
    unsafe {
        get_affinity_data(&*clause)
            .and_then(|data| data.iterators.get(index as usize))
            .map(|it| it.start)
            .unwrap_or(ptr::null())
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_affinity_iterator_end(
    clause: *const OmpClause,
    index: i32,
) -> *const c_char {
    if clause.is_null() || index < 0 {
        return ptr::null();
    }
    unsafe {
        get_affinity_data(&*clause)
            .and_then(|data| data.iterators.get(index as usize))
            .map(|it| it.end)
            .unwrap_or(ptr::null())
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_affinity_iterator_step(
    clause: *const OmpClause,
    index: i32,
) -> *const c_char {
    if clause.is_null() || index < 0 {
        return ptr::null();
    }
    unsafe {
        get_affinity_data(&*clause)
            .and_then(|data| data.iterators.get(index as usize))
            .map(|it| it.step)
            .unwrap_or(ptr::null())
    }
}

/// Get default data sharing from default clause.
///
/// Returns -1 if clause is NULL or not a default clause.
#[no_mangle]
pub extern "C" fn roup_clause_default_data_sharing(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return -1;
    }

    unsafe {
        let c = &*clause;
        if c.kind != CLAUSE_KIND_DEFAULT {
            // Not a default clause
            return -1;
        }
        c.data.default
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_defaultmap_behavior(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return -1;
    }

    unsafe {
        if let Some(data) = get_defaultmap_data(&*clause) {
            data.behavior
        } else {
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_defaultmap_category(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return -1;
    }

    unsafe {
        if let Some(data) = get_defaultmap_data(&*clause) {
            data.category
        } else {
            -1
        }
    }
}

/// Get proc_bind policy code.
#[no_mangle]
pub extern "C" fn roup_clause_proc_bind_policy(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return -1;
    }
    unsafe {
        let c = &*clause;
        if c.kind != CLAUSE_KIND_PROC_BIND {
            return -1;
        }
        c.data.default
    }
}

/// Get device_type value.
#[no_mangle]
pub extern "C" fn roup_clause_device_type_kind(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return -1;
    }
    unsafe {
        let c = &*clause;
        if c.kind != CLAUSE_KIND_DEVICE_TYPE {
            return -1;
        }
        c.data.default
    }
}

/// Get atomic_default_mem_order value.
#[no_mangle]
pub extern "C" fn roup_clause_atomic_default_mem_order(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return -1;
    }
    unsafe {
        let c = &*clause;
        if c.kind != CLAUSE_KIND_ATOMIC_DEFAULT_MEM_ORDER {
            return -1;
        }
        c.data.default
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_uses_allocators_count(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return 0;
    }

    unsafe {
        if let Some(data) = get_uses_allocators_data(&*clause) {
            data.entries.len() as i32
        } else {
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_requires_count(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return 0;
    }

    unsafe {
        if let Some(data) = get_requires_data(&*clause) {
            data.entries.len() as i32
        } else {
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_requires_modifier(clause: *const OmpClause, index: i32) -> i32 {
    if clause.is_null() || index < 0 {
        return REQUIRE_MOD_EXT_IMPL_DEFINED;
    }

    unsafe {
        if let Some(data) = get_requires_data(&*clause) {
            data.entries
                .get(index as usize)
                .map(|entry| entry.code)
                .unwrap_or(REQUIRE_MOD_EXT_IMPL_DEFINED)
        } else {
            REQUIRE_MOD_EXT_IMPL_DEFINED
        }
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_requires_name(clause: *const OmpClause, index: i32) -> *const c_char {
    if clause.is_null() || index < 0 {
        return ptr::null();
    }

    unsafe {
        if let Some(data) = get_requires_data(&*clause) {
            data.entries
                .get(index as usize)
                .map(|entry| entry.name)
                .unwrap_or(ptr::null())
        } else {
            ptr::null()
        }
    }
}

#[no_mangle]
pub extern "C" fn roup_requires_modifier_name(code: i32) -> *const c_char {
    REQUIRE_MOD_NAMES
        .get(code as usize)
        .map(|bytes| bytes.as_ptr() as *const c_char)
        .unwrap_or(ptr::null())
}

#[no_mangle]
pub extern "C" fn roup_clause_uses_allocator_kind(clause: *const OmpClause, index: i32) -> i32 {
    if clause.is_null() || index < 0 {
        return -1;
    }

    unsafe {
        if let Some(data) = get_uses_allocators_data(&*clause) {
            data.entries
                .get(index as usize)
                .map(|entry| entry.kind)
                .unwrap_or(-1)
        } else {
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_uses_allocator_user(
    clause: *const OmpClause,
    index: i32,
) -> *const c_char {
    if clause.is_null() || index < 0 {
        return ptr::null();
    }

    unsafe {
        if let Some(data) = get_uses_allocators_data(&*clause) {
            data.entries
                .get(index as usize)
                .map(|entry| entry.user_name)
                .unwrap_or(ptr::null())
        } else {
            ptr::null()
        }
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_uses_allocator_traits(
    clause: *const OmpClause,
    index: i32,
) -> *const c_char {
    if clause.is_null() || index < 0 {
        return ptr::null();
    }

    unsafe {
        if let Some(data) = get_uses_allocators_data(&*clause) {
            data.entries
                .get(index as usize)
                .map(|entry| entry.traits)
                .unwrap_or(ptr::null())
        } else {
            ptr::null()
        }
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_device_modifier(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return -1;
    }
    unsafe { (*clause).data.default }
}

#[no_mangle]
pub extern "C" fn roup_clause_device_expression(clause: *const OmpClause) -> *const c_char {
    if clause.is_null() {
        return ptr::null();
    }
    unsafe { (*clause).arguments }
}

#[no_mangle]
pub extern "C" fn roup_clause_depobj_update_dependence(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return -1;
    }
    unsafe {
        if is_depobj_update_clause_kind((*clause).kind) {
            (*clause).data.default
        } else {
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_lastprivate_modifier(clause: *const OmpClause) -> i32 {
    if clause.is_null() {
        return -1;
    }
    unsafe {
        if let Some(data) = get_lastprivate_data(&*clause) {
            data.modifier
        } else {
            -1
        }
    }
}

/// Get clause arguments as a string.
///
/// Returns NULL if clause is NULL or has no arguments.
/// Returned pointer is valid until clause is freed.
///
/// For bare clauses (nowait, ordered, etc.), returns NULL.
/// For parenthesized clauses, returns the content inside parentheses.
/// For example: "private(a, b)" returns "a, b"
#[no_mangle]
pub extern "C" fn roup_clause_arguments(clause: *const OmpClause) -> *const c_char {
    if clause.is_null() {
        return ptr::null();
    }

    unsafe {
        let c = &*clause;
        c.arguments
    }
}

// ============================================================================
// Variable List Functions (UNSAFE BLOCKS 9-11)
// ============================================================================

/// Get variable list from clause (private, shared, reduction, etc.).
///
/// Returns NULL if clause is NULL or has no variables.
/// Caller must call `roup_string_list_free()`.
#[no_mangle]
pub extern "C" fn roup_clause_variables(clause: *const OmpClause) -> *mut OmpStringList {
    if clause.is_null() {
        return ptr::null_mut();
    }

    // UNSAFE BLOCK 8: Dereference clause for variable check
    // Safety: Caller guarantees valid clause pointer
    unsafe {
        let c = &*clause;

        // Check if this clause type has variables
        // Kinds 3,4,5,13 are private/firstprivate/shared/lastprivate
        if is_reduction_clause_kind(c.kind) {
            if let Some(data) = get_reduction_data(c) {
                return clone_string_list(data.variables);
            }
        }

        if is_map_clause_kind(c.kind) {
            if let Some(data) = get_map_data(c) {
                if data.variables.is_null() {
                    return ptr::null_mut();
                }
                return clone_string_list(data.variables);
            }
        }

        if is_linear_clause_kind(c.kind) {
            if let Some(data) = get_linear_data(c) {
                if data.variables.is_null() {
                    return ptr::null_mut();
                }
                return clone_string_list(data.variables);
            }
        }

        if is_depend_clause_kind(c.kind) {
            if let Some(data) = get_depend_data(c) {
                if data.variables.is_null() {
                    return ptr::null_mut();
                }
                return clone_string_list(data.variables);
            }
        }

        if is_allocate_clause_kind(c.kind) {
            if let Some(data) = get_allocate_data(c) {
                if data.variables.is_null() {
                    return ptr::null_mut();
                }
                return clone_string_list(data.variables);
            }
        }

        if is_affinity_clause_kind(c.kind) {
            if let Some(data) = get_affinity_data(c) {
                if data.variables.is_null() {
                    return ptr::null_mut();
                }
                return clone_string_list(data.variables);
            }
        }

        if c.kind == CLAUSE_KIND_ALIGNED {
            if let Some(data) = get_aligned_data(c) {
                if data.variables.is_null() {
                    return ptr::null_mut();
                }
                return clone_string_list(data.variables);
            }
        }

        if is_lastprivate_clause_kind(c.kind) {
            if let Some(data) = get_lastprivate_data(c) {
                let vars_ptr = data.variables;
                if vars_ptr.is_null() {
                    return ptr::null_mut();
                }
                return clone_string_list(vars_ptr);
            }
        }

        if !clause_kind_uses_variable_list(c.kind) {
            return ptr::null_mut();
        }

        let vars_ptr = c.data.variables;
        if vars_ptr.is_null() {
            return ptr::null_mut();
        }

        clone_string_list(vars_ptr)
    }
}

/// Get length of string list.
///
/// Returns 0 if list is NULL.
#[no_mangle]
pub extern "C" fn roup_string_list_len(list: *const OmpStringList) -> i32 {
    if list.is_null() {
        return 0;
    }

    unsafe {
        let l = &*list;
        l.items.len() as i32
    }
}

/// Get string at index from list.
///
/// Returns NULL if list is NULL or index out of bounds.
/// Returned pointer is valid until list is freed.
#[no_mangle]
pub extern "C" fn roup_string_list_get(list: *const OmpStringList, index: i32) -> *const c_char {
    if list.is_null() || index < 0 {
        return ptr::null();
    }

    // UNSAFE BLOCK 9: Dereference list
    // Safety: Caller guarantees valid list pointer
    unsafe {
        let l = &*list;
        let idx = index as usize;

        if idx >= l.items.len() {
            return ptr::null();
        }

        l.items[idx]
    }
}

/// Free string list.
#[no_mangle]
pub extern "C" fn roup_string_list_free(list: *mut OmpStringList) {
    if list.is_null() {
        return;
    }

    // UNSAFE BLOCK 10: Free list and strings
    // Safety: Pointer from roup_clause_variables
    unsafe {
        let boxed = Box::from_raw(list);

        // Free each C string (was allocated with CString::into_raw)
        for item_ptr in &boxed.items {
            if !item_ptr.is_null() {
                drop(CString::from_raw(*item_ptr as *mut c_char));
            }
        }

        // Box dropped here
    }
}

// ============================================================================
// Helper Functions (Internal - Not Exported to C)
// ============================================================================
//
// These functions handle conversion between Rust and C representations.
// They're not exported because C doesn't need to call them directly.

/// Convert language code to IR Language enum.
///
/// Maps the C API language constants to the ir::Language enum used for
/// semantic representation and translation.
///
/// ## Language Code Mapping:
/// - 0 (ROUP_LANG_C) → Language::C (use this for both C and C++)
/// - 1 (ROUP_LANG_FORTRAN_FREE) → Language::Fortran
/// - 2 (ROUP_LANG_FORTRAN_FIXED) → Language::Fortran
///
/// Note: Both Fortran variants map to Language::Fortran in IR, as the
/// distinction between free-form and fixed-form is a lexical concern,
/// not a semantic one. There is no separate constant for C++; use ROUP_LANG_C for both C and C++.
fn language_code_to_ir_language(code: i32) -> Option<IrLanguage> {
    match code {
        ROUP_LANG_C => Some(IrLanguage::C), // C/C++ use same OpenMP syntax
        ROUP_LANG_FORTRAN_FREE => Some(IrLanguage::Fortran),
        ROUP_LANG_FORTRAN_FIXED => Some(IrLanguage::Fortran),
        _ => None, // Invalid language code
    }
}

/// Allocate a C string from Rust &str.
///
/// Creates a heap-allocated, null-terminated C string from Rust string.
/// The returned pointer must be freed with CString::from_raw() later.
///
/// ## How it works:
/// 1. CString::new() creates null-terminated copy
/// 2. into_raw() gives us ownership of the pointer
/// 3. Caller (or roup_directive_free) must eventually free it
fn allocate_c_string(s: &str) -> *const c_char {
    let c_string = std::ffi::CString::new(s).unwrap();
    c_string.into_raw() as *const c_char
}

/// Find a top-level colon (:) not nested inside parentheses. Returns (left, right)
#[allow(dead_code)] // retained for legacy parser paths used by generators
fn split_once_top_level_colon(input: &str) -> Option<(&str, &str)> {
    let mut depth: isize = 0;
    for (i, ch) in input.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ':' => {
                if depth == 0 {
                    let left = &input[..i];
                    let right = &input[i + 1..];
                    return Some((left, right));
                }
            }
            _ => {}
        }
    }
    None
}

fn build_string_list(items: &[std::borrow::Cow<'_, str>]) -> *mut OmpStringList {
    if items.is_empty() {
        return ptr::null_mut();
    }

    let mut list = OmpStringList { items: Vec::new() };
    for item in items {
        let c_string = match CString::new(item.as_ref()) {
            Ok(value) => value,
            Err(_) => continue,
        };
        list.items.push(c_string.into_raw());
    }

    Box::into_raw(Box::new(list))
}

unsafe fn clone_string_list(src: *mut OmpStringList) -> *mut OmpStringList {
    if src.is_null() {
        return ptr::null_mut();
    }

    let src_ref = &*src;
    if src_ref.items.is_empty() {
        return ptr::null_mut();
    }

    let mut list = OmpStringList { items: Vec::new() };
    for &ptr in &src_ref.items {
        if ptr.is_null() {
            continue;
        }
        let c_str = std::ffi::CStr::from_ptr(ptr);
        if let Ok(cloned) = CString::new(c_str.to_bytes()) {
            list.items.push(cloned.into_raw());
        }
    }

    Box::into_raw(Box::new(list))
}

#[allow(dead_code)] // Kept for constants/header generation; runtime uses enum-based path
fn is_reduction_clause_kind(kind: i32) -> bool {
    matches!(
        kind,
        CLAUSE_KIND_REDUCTION | CLAUSE_KIND_IN_REDUCTION | CLAUSE_KIND_TASK_REDUCTION
    )
}

unsafe fn get_reduction_data(clause: &OmpClause) -> Option<&ReductionData> {
    if is_reduction_clause_kind(clause.kind) {
        Some(&*clause.data.reduction)
    } else {
        None
    }
}

fn is_lastprivate_clause_kind(kind: i32) -> bool {
    kind == CLAUSE_KIND_LASTPRIVATE
}

unsafe fn get_lastprivate_data(clause: &OmpClause) -> Option<&LastprivateData> {
    if is_lastprivate_clause_kind(clause.kind) {
        Some(&*clause.data.lastprivate)
    } else {
        None
    }
}

fn is_defaultmap_clause_kind(kind: i32) -> bool {
    kind == CLAUSE_KIND_DEFAULTMAP
}

unsafe fn get_defaultmap_data(clause: &OmpClause) -> Option<&DefaultmapData> {
    if is_defaultmap_clause_kind(clause.kind) {
        Some(&*clause.data.defaultmap)
    } else {
        None
    }
}

fn is_uses_allocators_clause_kind(kind: i32) -> bool {
    kind == CLAUSE_KIND_USES_ALLOCATORS
}

fn is_depobj_update_clause_kind(kind: i32) -> bool {
    kind == CLAUSE_KIND_DEPOBJ_UPDATE
}

fn depobj_update_dependence_code(dep: DepobjUpdateDependence) -> i32 {
    match dep {
        DepobjUpdateDependence::In => 0,
        DepobjUpdateDependence::Out => 1,
        DepobjUpdateDependence::Inout => 2,
        DepobjUpdateDependence::Inoutset => 3,
        DepobjUpdateDependence::Mutexinoutset => 4,
        DepobjUpdateDependence::Depobj => 5,
        DepobjUpdateDependence::Sink => 6,
        DepobjUpdateDependence::Source => 7,
        DepobjUpdateDependence::Unknown => 8,
    }
}

unsafe fn get_uses_allocators_data(clause: &OmpClause) -> Option<&UsesAllocatorsData> {
    if is_uses_allocators_clause_kind(clause.kind) {
        let ptr = clause.data.uses_allocators;
        if ptr.is_null() {
            None
        } else {
            Some(&*ptr)
        }
    } else {
        None
    }
}

fn is_requires_clause_kind(kind: i32) -> bool {
    kind == CLAUSE_KIND_REQUIRES
}

fn clause_kind_uses_variable_list(kind: i32) -> bool {
    matches!(
        kind,
        CLAUSE_KIND_PRIVATE
            | CLAUSE_KIND_FIRSTPRIVATE
            | CLAUSE_KIND_SHARED
            | CLAUSE_KIND_USE_DEVICE_PTR
            | CLAUSE_KIND_USE_DEVICE_ADDR
            | CLAUSE_KIND_IS_DEVICE_PTR
            | CLAUSE_KIND_HAS_DEVICE_ADDR
            | CLAUSE_KIND_COPYIN
            | CLAUSE_KIND_COPYPRIVATE
            | CLAUSE_KIND_NONTEMPORAL
            | CLAUSE_KIND_UNIFORM
            | CLAUSE_KIND_SIZES
            | CLAUSE_KIND_TO
            | CLAUSE_KIND_FROM
            | CLAUSE_KIND_LINK
    )
}

fn is_map_clause_kind(kind: i32) -> bool {
    kind == CLAUSE_KIND_MAP
}

fn is_linear_clause_kind(kind: i32) -> bool {
    kind == CLAUSE_KIND_LINEAR
}

fn is_depend_clause_kind(kind: i32) -> bool {
    kind == CLAUSE_KIND_DEPEND
}

fn is_allocate_clause_kind(kind: i32) -> bool {
    kind == CLAUSE_KIND_ALLOCATE
}

fn is_affinity_clause_kind(kind: i32) -> bool {
    kind == CLAUSE_KIND_AFFINITY
}

fn is_order_clause_kind(kind: i32) -> bool {
    kind == CLAUSE_KIND_ORDER
}

unsafe fn get_requires_data(clause: &OmpClause) -> Option<&RequiresData> {
    if is_requires_clause_kind(clause.kind) {
        let ptr = clause.data.requires;
        if ptr.is_null() {
            None
        } else {
            Some(&*ptr)
        }
    } else {
        None
    }
}

unsafe fn get_map_data(clause: &OmpClause) -> Option<&MapData> {
    if is_map_clause_kind(clause.kind) {
        let ptr = clause.data.map;
        if ptr.is_null() {
            None
        } else {
            Some(&*ptr)
        }
    } else {
        None
    }
}

unsafe fn get_linear_data(clause: &OmpClause) -> Option<&LinearData> {
    if is_linear_clause_kind(clause.kind) {
        let ptr = clause.data.linear;
        if ptr.is_null() {
            None
        } else {
            Some(&*ptr)
        }
    } else {
        None
    }
}

unsafe fn get_depend_data(clause: &OmpClause) -> Option<&DependData> {
    if is_depend_clause_kind(clause.kind) {
        let ptr = clause.data.depend;
        if ptr.is_null() {
            None
        } else {
            Some(&*ptr)
        }
    } else {
        None
    }
}

unsafe fn get_allocate_data(clause: &OmpClause) -> Option<&AllocateData> {
    if is_allocate_clause_kind(clause.kind) {
        let ptr = clause.data.allocate;
        if ptr.is_null() {
            None
        } else {
            Some(&*ptr)
        }
    } else {
        None
    }
}

unsafe fn get_affinity_data(clause: &OmpClause) -> Option<&AffinityData> {
    if is_affinity_clause_kind(clause.kind) {
        let ptr = clause.data.affinity;
        if ptr.is_null() {
            None
        } else {
            Some(&*ptr)
        }
    } else {
        None
    }
}

unsafe fn get_aligned_data(clause: &OmpClause) -> Option<&AlignedData> {
    if clause.kind == CLAUSE_KIND_ALIGNED {
        let ptr = clause.data.aligned;
        if ptr.is_null() {
            None
        } else {
            Some(&*ptr)
        }
    } else {
        None
    }
}

unsafe fn get_order_data(clause: &OmpClause) -> Option<&OrderData> {
    if is_order_clause_kind(clause.kind) {
        Some(&*clause.data.order)
    } else {
        None
    }
}

fn build_c_api_directive_from_ast(
    directive: &crate::ast::OmpDirective,
    language: Language,
) -> OmpDirective {
    with_clause_language(language, || {
        let directive_name: DirectiveName = directive.kind.into();
        let (name, extra_clause) = atomic_directive_info(directive_name);

        let (parameter_text, parameter_data) =
            directive_parameter_data_from_ast(directive.parameter.as_ref(), language);

        let mut clauses: Vec<OmpClause> = directive
            .clauses
            .iter()
            .map(|clause| convert_clause_from_ast(clause.kind, &clause.payload))
            .collect();

        if let Some(clause_name) = extra_clause {
            clauses.insert(0, convert_atomic_suffix_clause(clause_name));
        }

        OmpDirective {
            name: allocate_c_string(&name),
            parameter: parameter_text
                .as_ref()
                .and_then(|p| {
                    if p.is_empty() {
                        None
                    } else {
                        Some(allocate_c_string(p))
                    }
                })
                .unwrap_or(ptr::null()),
            parameter_data,
            clauses,
        }
    })
}

fn atomic_directive_info(kind: DirectiveName) -> (String, Option<&'static str>) {
    match kind {
        DirectiveName::AtomicRead => ("atomic".to_string(), Some("read")),
        DirectiveName::AtomicWrite => ("atomic".to_string(), Some("write")),
        DirectiveName::AtomicUpdate => ("atomic".to_string(), Some("update")),
        DirectiveName::AtomicCapture => ("atomic".to_string(), Some("capture")),
        DirectiveName::AtomicCompareCapture => ("atomic".to_string(), Some("compare capture")),
        _ => (kind.as_ref().to_string(), None),
    }
}

fn convert_atomic_suffix_clause(name: &str) -> OmpClause {
    let kind = match name {
        "read" => CLAUSE_KIND_ATOMIC_READ,
        "write" => CLAUSE_KIND_ATOMIC_WRITE,
        "update" => CLAUSE_KIND_ATOMIC_UPDATE,
        "capture" => CLAUSE_KIND_ATOMIC_CAPTURE,
        "compare capture" => CLAUSE_KIND_COMPARE_CAPTURE,
        _ => panic!("unexpected atomic suffix clause: {name}"),
    };

    OmpClause {
        kind,
        arguments: ptr::null(),
        data: ClauseData { default: 0 },
    }
}

fn format_directive_parameter(param: &OmpDirectiveParameter) -> String {
    use OmpDirectiveParameter::*;
    fn join_identifiers(list: &[crate::ir::Identifier]) -> String {
        list.iter()
            .map(|id| id.name().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn format_reduction_operator(token: &ReductionOperatorToken) -> String {
        match token {
            ReductionOperatorToken::Builtin(op) => op.to_string(),
            ReductionOperatorToken::Identifier(id) => id.name().to_string(),
        }
    }

    match param {
        IdentifierList(list) => format!("({})", join_identifiers(list)),
        Identifier(name) => name.name().to_string(),
        Mapper(id) | VariantFunction(id) | Depobj(id) | CriticalSection(id) => {
            format!("({})", id.name())
        }
        Scan(scan) => {
            let vars = join_identifiers(&scan.variables);
            let prefix = match scan.mode {
                OmpScanMode::Exclusive => "exclusive",
                OmpScanMode::Inclusive => "inclusive",
            };
            format!("{prefix}({vars})")
        }
        Construct(kind) => match kind {
            OmpConstructType::Parallel => "parallel".to_string(),
            OmpConstructType::Sections => "sections".to_string(),
            OmpConstructType::For => "for".to_string(),
            OmpConstructType::Taskgroup => "taskgroup".to_string(),
            OmpConstructType::Other(v) => v.clone(),
        },
        FlushList(list) => format!("({})", join_identifiers(list)),
        DeclareReduction(dr) => {
            let types = dr.type_names.join(", ");
            let mut out = format!(
                "({} : {} : {})",
                format_reduction_operator(&dr.operator),
                types,
                dr.combiner
            );
            if let Some(init) = &dr.initializer {
                out.push_str(" initializer(");
                out.push_str(init);
                out.push(')');
            }
            out
        }
        DeclareSimd(target) => target
            .function
            .as_ref()
            .map(|f| format!("({})", f.name()))
            .unwrap_or_default(),
    }
}

fn directive_parameter_data_from_ast(
    parameter: Option<&OmpDirectiveParameter>,
    language: Language,
) -> (Option<String>, DirectiveParameterData) {
    let text = parameter.map(format_directive_parameter);
    let mut data = DirectiveParameterData {
        kind: ROUP_DIRECTIVE_PARAM_NONE,
        scan_mode: ROUP_SCAN_MODE_UNSPECIFIED,
        construct_type: ROUP_CONSTRUCT_TYPE_UNKNOWN,
        identifiers: ptr::null_mut(),
    };

    if let Some(param) = parameter {
        match param {
            OmpDirectiveParameter::IdentifierList(list)
            | OmpDirectiveParameter::FlushList(list) => {
                data.kind = ROUP_DIRECTIVE_PARAM_IDENTIFIER_LIST;
                data.identifiers = build_string_list_from_identifiers(list, language);
            }
            OmpDirectiveParameter::Identifier(id)
            | OmpDirectiveParameter::Mapper(id)
            | OmpDirectiveParameter::VariantFunction(id)
            | OmpDirectiveParameter::Depobj(id)
            | OmpDirectiveParameter::CriticalSection(id) => {
                data.kind = match param {
                    OmpDirectiveParameter::Identifier(_) => ROUP_DIRECTIVE_PARAM_IDENTIFIER,
                    OmpDirectiveParameter::Mapper(_) => ROUP_DIRECTIVE_PARAM_MAPPER,
                    OmpDirectiveParameter::VariantFunction(_) => {
                        ROUP_DIRECTIVE_PARAM_VARIANT_FUNCTION
                    }
                    OmpDirectiveParameter::Depobj(_) => ROUP_DIRECTIVE_PARAM_DEPOBJ,
                    _ => ROUP_DIRECTIVE_PARAM_CRITICAL,
                };
                data.identifiers =
                    build_string_list_from_identifiers(std::slice::from_ref(id), language);
            }
            OmpDirectiveParameter::Scan(scan) => {
                data.kind = ROUP_DIRECTIVE_PARAM_SCAN;
                data.scan_mode = match scan.mode {
                    OmpScanMode::Exclusive => ROUP_SCAN_MODE_EXCLUSIVE,
                    OmpScanMode::Inclusive => ROUP_SCAN_MODE_INCLUSIVE,
                };
                data.identifiers = build_string_list_from_identifiers(&scan.variables, language);
            }
            OmpDirectiveParameter::Construct(kind) => {
                data.kind = ROUP_DIRECTIVE_PARAM_CONSTRUCT;
                data.construct_type = match kind {
                    OmpConstructType::Parallel => ROUP_CONSTRUCT_TYPE_PARALLEL,
                    OmpConstructType::Sections => ROUP_CONSTRUCT_TYPE_SECTIONS,
                    OmpConstructType::For => ROUP_CONSTRUCT_TYPE_FOR,
                    OmpConstructType::Taskgroup => ROUP_CONSTRUCT_TYPE_TASKGROUP,
                    OmpConstructType::Other(_) => ROUP_CONSTRUCT_TYPE_OTHER,
                };
            }
            OmpDirectiveParameter::DeclareReduction(_) => {
                data.kind = ROUP_DIRECTIVE_PARAM_DECLARE_REDUCTION;
            }
            OmpDirectiveParameter::DeclareSimd(_) => {
                data.kind = ROUP_DIRECTIVE_PARAM_DECLARE_SIMD;
            }
        }
    }

    (text, data)
}

fn convert_clause_from_ast(kind: OmpClauseKind, payload: &IrClauseData) -> OmpClause {
    use crate::parser::ClauseName::*;
    let clause_name: ClauseName = kind.into();
    match clause_name {
        Default => expect_clause(convert_default_clause_from_ast(payload), "default"),
        Defaultmap => expect_clause(convert_defaultmap_clause_from_ast(payload), "defaultmap"),
        If => expect_clause(convert_if_clause_from_ast(payload), "if"),
        NumThreads => expect_clause(convert_num_threads_clause_from_ast(payload), "num_threads"),
        Private => expect_clause(convert_private_clause_from_ast(payload), "private"),
        Firstprivate => expect_clause(
            convert_firstprivate_clause_from_ast(payload),
            "firstprivate",
        ),
        Shared => expect_clause(convert_shared_clause_from_ast(payload), "shared"),
        Lastprivate => expect_clause(convert_lastprivate_clause_from_ast(payload), "lastprivate"),
        CopyIn => expect_clause(convert_copyin_clause_from_ast(payload), "copyin"),
        Nowait => expect_clause(
            convert_bare_clause_from_ast(payload, CLAUSE_KIND_NOWAIT),
            "nowait",
        ),
        Nogroup => expect_clause(
            convert_bare_clause_from_ast(payload, CLAUSE_KIND_NOGROUP),
            "nogroup",
        ),
        Untied => expect_clause(
            convert_bare_clause_from_ast(payload, CLAUSE_KIND_UNTIED),
            "untied",
        ),
        Mergeable => expect_clause(
            convert_bare_clause_from_ast(payload, CLAUSE_KIND_MERGEABLE),
            "mergeable",
        ),
        SeqCst => expect_clause(
            convert_bare_clause_from_ast(payload, CLAUSE_KIND_SEQ_CST),
            "seq_cst",
        ),
        Relaxed => expect_clause(
            convert_bare_clause_from_ast(payload, CLAUSE_KIND_RELAXED),
            "relaxed",
        ),
        Release => expect_clause(
            convert_bare_clause_from_ast(payload, CLAUSE_KIND_RELEASE),
            "release",
        ),
        Acquire => expect_clause(
            convert_bare_clause_from_ast(payload, CLAUSE_KIND_ACQUIRE),
            "acquire",
        ),
        AcqRel => expect_clause(
            convert_bare_clause_from_ast(payload, CLAUSE_KIND_ACQ_REL),
            "acq_rel",
        ),
        ProcBind => expect_clause(convert_proc_bind_clause_from_ast(payload), "proc_bind"),
        NumTeams => expect_clause(convert_num_teams_clause_from_ast(payload), "num_teams"),
        ThreadLimit => expect_clause(
            convert_thread_limit_clause_from_ast(payload),
            "thread_limit",
        ),
        Collapse => expect_clause(convert_collapse_clause_from_ast(payload), "collapse"),
        Ordered => expect_clause(convert_ordered_clause_from_ast(payload), "ordered"),
        Linear => expect_clause(convert_linear_clause_from_ast(payload), "linear"),
        Safelen => expect_clause(convert_safelen_clause_from_ast(payload), "safelen"),
        Simdlen => expect_clause(convert_simdlen_clause_from_ast(payload), "simdlen"),
        Aligned => expect_clause(convert_aligned_clause_from_ast(payload), "aligned"),
        Reduction => expect_clause(
            convert_reduction_clause_from_ast(CLAUSE_KIND_REDUCTION, payload),
            "reduction",
        ),
        InReduction => expect_clause(
            convert_reduction_clause_from_ast(CLAUSE_KIND_IN_REDUCTION, payload),
            "in_reduction",
        ),
        TaskReduction => expect_clause(
            convert_reduction_clause_from_ast(CLAUSE_KIND_TASK_REDUCTION, payload),
            "task_reduction",
        ),
        Requires => expect_clause(convert_requires_clause_from_ast(payload), "requires"),
        Schedule => expect_clause(convert_schedule_clause_from_ast(payload), "schedule"),
        DistSchedule => expect_clause(
            convert_dist_schedule_clause_from_ast(payload),
            "dist_schedule",
        ),
        Map => expect_clause(convert_map_clause_from_ast(payload), "map"),
        UseDevicePtr => expect_clause(
            convert_use_device_ptr_clause_from_ast(payload),
            "use_device_ptr",
        ),
        UseDeviceAddr => expect_clause(
            convert_use_device_addr_clause_from_ast(payload),
            "use_device_addr",
        ),
        IsDevicePtr => expect_clause(
            convert_is_device_ptr_clause_from_ast(payload),
            "is_device_ptr",
        ),
        HasDeviceAddr => expect_clause(
            convert_has_device_addr_clause_from_ast(payload),
            "has_device_addr",
        ),
        Sizes => expect_clause(
            Some(convert_generic_clause_from_ast(clause_name, payload)),
            "sizes",
        ),
        UsesAllocators => expect_clause(
            convert_uses_allocators_clause_from_ast(payload),
            "uses_allocators",
        ),
        Depend => expect_clause(convert_depend_clause_from_ast(payload), "depend"),
        Device => expect_clause(convert_device_clause_from_ast(payload), "device"),
        DeviceType => expect_clause(convert_device_type_clause_from_ast(payload), "device_type"),
        DepobjUpdate => expect_clause(
            convert_depobj_update_clause_from_ast(payload),
            "depobj_update",
        ),
        Allocate => expect_clause(convert_allocate_clause_from_ast(payload), "allocate"),
        Allocator => expect_clause(convert_allocator_clause_from_ast(payload), "allocator"),
        Copyprivate => expect_clause(convert_copyprivate_clause_from_ast(payload), "copyprivate"),
        Affinity => expect_clause(convert_affinity_clause_from_ast(payload), "affinity"),
        Priority => expect_clause(convert_priority_clause_from_ast(payload), "priority"),
        Grainsize => expect_clause(convert_grainsize_clause_from_ast(payload), "grainsize"),
        NumTasks => expect_clause(convert_num_tasks_clause_from_ast(payload), "num_tasks"),
        Filter => expect_clause(convert_filter_clause_from_ast(payload), "filter"),
        Order => expect_clause(convert_order_clause_from_ast(payload), "order"),
        AtomicDefaultMemOrder => expect_clause(
            convert_atomic_default_mem_order_clause_from_ast(payload),
            "atomic_default_mem_order",
        ),
        Read => expect_clause(
            convert_atomic_operation_clause_from_ast(CLAUSE_KIND_ATOMIC_READ, payload),
            "read",
        ),
        Write => expect_clause(
            convert_atomic_operation_clause_from_ast(CLAUSE_KIND_ATOMIC_WRITE, payload),
            "write",
        ),
        Update => expect_clause(convert_update_clause_from_ast(payload), "update"),
        Capture => expect_clause(
            convert_atomic_operation_clause_from_ast(CLAUSE_KIND_ATOMIC_CAPTURE, payload),
            "capture",
        ),
        Nontemporal => expect_clause(
            convert_item_list_clause_from_ast(CLAUSE_KIND_NONTEMPORAL, payload),
            "nontemporal",
        ),
        Uniform => expect_clause(
            convert_item_list_clause_from_ast(CLAUSE_KIND_UNIFORM, payload),
            "uniform",
        ),
        Inbranch => expect_clause(
            convert_bare_clause_from_ast(payload, CLAUSE_KIND_INBRANCH),
            "inbranch",
        ),
        Notinbranch => expect_clause(
            convert_bare_clause_from_ast(payload, CLAUSE_KIND_NOTINBRANCH),
            "notinbranch",
        ),
        Inclusive => expect_clause(
            convert_bare_clause_from_ast(payload, CLAUSE_KIND_INCLUSIVE),
            "inclusive",
        ),
        Exclusive => expect_clause(
            convert_bare_clause_from_ast(payload, CLAUSE_KIND_EXCLUSIVE),
            "exclusive",
        ),
        ReverseOffload => {
            expect_clause(convert_requires_clause_from_ast(payload), "reverse_offload")
        }
        UnifiedAddress => {
            expect_clause(convert_requires_clause_from_ast(payload), "unified_address")
        }
        UnifiedSharedMemory => expect_clause(
            convert_requires_clause_from_ast(payload),
            "unified_shared_memory",
        ),
        DynamicAllocators => expect_clause(
            convert_requires_clause_from_ast(payload),
            "dynamic_allocators",
        ),
        ExtImplementationDefinedRequirement => expect_clause(
            convert_requires_clause_from_ast(payload),
            "ext_implementation_defined_requirement",
        ),
        _ => convert_generic_clause_from_ast(clause_name, payload),
    }
}

fn convert_generic_clause_from_ast(clause_name: ClauseName, payload: &IrClauseData) -> OmpClause {
    let kind = clause_name_enum_to_kind(clause_name);
    let (arguments, data) = match payload {
        IrClauseData::ItemList(items) => {
            let args_ptr = format_clause_items(items)
                .map(|text| allocate_c_string(&text))
                .unwrap_or(ptr::null());
            let vars = build_string_list_from_items(items);
            (args_ptr, ClauseData { variables: vars })
        }
        _ => {
            let arguments = render_arguments_from_payload(payload)
                .map(|text| allocate_c_string(&text))
                .unwrap_or(ptr::null());
            (arguments, ClauseData { default: 0 })
        }
    };

    OmpClause {
        kind,
        arguments,
        data,
    }
}

fn render_arguments_from_payload(payload: &IrClauseData) -> Option<String> {
    match payload {
        IrClauseData::Bare(_) => None,
        IrClauseData::Expression(expr) => Some(expr.to_string()),
        IrClauseData::ItemList(items) => format_clause_items(items),
        IrClauseData::Private { items }
        | IrClauseData::Firstprivate { items }
        | IrClauseData::Shared { items }
        | IrClauseData::Copyin { items }
        | IrClauseData::Copyprivate { items }
        | IrClauseData::UseDevicePtr { items }
        | IrClauseData::UseDeviceAddr { items }
        | IrClauseData::IsDevicePtr { items }
        | IrClauseData::HasDeviceAddr { items }
        | IrClauseData::Allocate { items, .. } => format_clause_items(items),
        IrClauseData::Map {
            map_type,
            modifiers,
            mapper,
            iterators,
            items,
        } => {
            let mut parts = Vec::new();
            if !iterators.is_empty() {
                let defs = iterators
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                parts.push(format!("iterator ( {defs} )"));
            }
            if let Some(mapper_id) = mapper {
                parts.push(format!("mapper({mapper_id})"));
            }
            if !modifiers.is_empty() {
                parts.push(
                    modifiers
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", "),
                );
            }
            if let Some(mt) = map_type {
                parts.push(format!("{mt}:"));
            } else if !parts.is_empty() {
                parts.push(":".to_string());
            }
            if let Some(list) = format_clause_items(items) {
                parts.push(list);
            }
            Some(parts.join(" "))
        }
        IrClauseData::Depend {
            depend_type,
            items,
            iterators,
        } => {
            let mut parts = Vec::new();
            if !iterators.is_empty() {
                let defs = iterators
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                parts.push(format!("iterator ( {defs} )"));
            }

            let mut args = depend_type.to_string();
            if let Some(list) = format_clause_items(items) {
                if !list.is_empty() {
                    args.push_str(": ");
                    args.push_str(&list);
                }
            }
            parts.push(args);

            Some(parts.join(", "))
        }
        IrClauseData::Lastprivate { modifier, items } => {
            let mut args = String::new();
            if let Some(m) = modifier {
                args.push_str(&m.to_string());
                args.push_str(": ");
            }
            if let Some(list) = format_clause_items(items) {
                args.push_str(&list);
            }
            Some(args)
        }
        IrClauseData::Default(kind) => Some(kind.to_string()),
        IrClauseData::Defaultmap { behavior, category } => {
            Some(format_defaultmap_arguments(*behavior, *category))
        }
        IrClauseData::Reduction {
            modifiers,
            operator,
            user_identifier,
            items,
            space_after_colon,
        } => Some(format_reduction_argument_text(
            modifiers,
            operator,
            user_identifier.as_ref(),
            items,
            *space_after_colon,
        )),
        IrClauseData::Schedule {
            kind,
            chunk_size,
            modifiers,
        } => Some(format_schedule_arguments(
            *kind,
            modifiers,
            chunk_size.as_ref(),
        )),
        IrClauseData::Aligned { items, alignment } => {
            let mut args = format_clause_items(items).unwrap_or_default();
            if let Some(expr) = alignment {
                if !args.is_empty() {
                    args.push_str(": ");
                }
                args.push_str(&expr.to_string());
            }
            Some(args)
        }
        IrClauseData::DistSchedule { kind, chunk_size } => {
            let mut args = kind.to_string();
            if let Some(expr) = chunk_size {
                args.push_str(", ");
                args.push_str(&expr.to_string());
            }
            Some(args)
        }
        IrClauseData::Linear {
            modifier,
            items,
            step,
        } => {
            let mut args = String::new();
            if let Some(m) = modifier {
                args.push_str(&m.to_string());
                args.push_str(": ");
            }
            if let Some(list) = format_clause_items(items) {
                args.push_str(&list);
            }
            if let Some(expr) = step {
                args.push_str(": ");
                args.push_str(&expr.to_string());
            }
            Some(args)
        }
        IrClauseData::Safelen { length } => Some(length.to_string()),
        IrClauseData::Simdlen { length } => Some(length.to_string()),
        IrClauseData::NumThreads { num } => Some(num.to_string()),
        IrClauseData::Collapse { n } => Some(n.to_string()),
        IrClauseData::Ordered { n } => n.as_ref().map(|expr| expr.to_string()),
        IrClauseData::If {
            directive_name,
            condition,
        } => {
            let mut args = String::new();
            if let Some(name) = directive_name {
                args.push_str(&name.to_string());
                args.push_str(": ");
            }
            args.push_str(&condition.to_string());
            Some(args)
        }
        IrClauseData::NumTeams { num } => Some(num.to_string()),
        IrClauseData::ThreadLimit { limit } => Some(limit.to_string()),
        IrClauseData::Device {
            modifier,
            device_num,
        } => {
            let mut args = String::new();
            if *modifier != DeviceModifier::Unspecified {
                args.push_str(&modifier.to_string());
                args.push_str(": ");
            }
            args.push_str(&device_num.to_string());
            Some(args)
        }
        IrClauseData::DeviceType(device_type) => Some(device_type.to_string()),
        IrClauseData::AtomicDefaultMemOrder(order) => Some(order.to_string()),
        IrClauseData::AtomicOperation { memory_order, .. } => {
            memory_order.as_ref().map(|mo| mo.to_string())
        }
        IrClauseData::ProcBind(policy) => Some(policy.to_string()),
        IrClauseData::Order { modifier, kind } => {
            let mut text = String::new();
            if *modifier != OrderModifier::Unspecified {
                text.push_str(&modifier.to_string());
                text.push_str(": ");
            }
            text.push_str(&kind.to_string());
            Some(text)
        }
        IrClauseData::Priority { priority } => Some(priority.to_string()),
        IrClauseData::Grainsize { modifier, grain } => {
            let mut text = String::new();
            if *modifier != GrainsizeModifier::Unspecified {
                text.push_str(&modifier.to_string());
                text.push_str(": ");
            }
            text.push_str(&grain.to_string());
            Some(text)
        }
        IrClauseData::NumTasks { modifier, num } => {
            let mut text = String::new();
            if *modifier != NumTasksModifier::Unspecified {
                text.push_str(&modifier.to_string());
                text.push_str(": ");
            }
            text.push_str(&num.to_string());
            Some(text)
        }
        IrClauseData::Affinity {
            modifier,
            iterators,
            items,
        } => {
            let mut args = String::new();
            if *modifier == AffinityModifier::Iterator && !iterators.is_empty() {
                let defs = iterators
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                args.push_str(&format!("iterator ( {defs} )"));
                if let Some(list) = format_clause_items(items) {
                    args.push_str(" : ");
                    args.push_str(&list);
                }
                Some(args)
            } else {
                if *modifier != AffinityModifier::Unspecified {
                    args.push_str(&modifier.to_string());
                    if !items.is_empty() {
                        args.push_str(": ");
                    }
                }
                if let Some(list) = format_clause_items(items) {
                    args.push_str(&list);
                }
                if args.is_empty() {
                    None
                } else {
                    Some(args)
                }
            }
        }
        IrClauseData::Filter { thread_num } => Some(thread_num.to_string()),
        IrClauseData::Allocator { allocator } => Some(allocator.to_string()),
        IrClauseData::UsesAllocators { allocators } => {
            Some(format_uses_allocators_arguments(allocators))
        }
        IrClauseData::Requires { requirements } => format_requires_arguments(requirements),
        IrClauseData::Generic { data, .. } => data.clone(),
        IrClauseData::DepobjUpdate { dependence } => Some(dependence.to_string()),
    }
}

fn convert_bare_clause_from_ast(payload: &IrClauseData, kind: i32) -> Option<OmpClause> {
    match payload {
        IrClauseData::Bare(_) => Some(OmpClause {
            kind,
            arguments: ptr::null(),
            data: ClauseData { default: 0 },
        }),
        IrClauseData::Expression(expr) => Some(OmpClause {
            kind,
            arguments: allocate_c_string(&expr.to_string()),
            data: ClauseData { default: 0 },
        }),
        _ => None,
    }
}

fn expect_clause(value: Option<OmpClause>, clause: &'static str) -> OmpClause {
    value.unwrap_or_else(|| panic!("AST payload mismatch for clause '{clause}'"))
}

// Legacy clause converter retained for header generation (constants_gen.rs).
// Not used at runtime; all data flows through the AST-based converters.
#[allow(dead_code)]
fn convert_clause(clause: &Clause) -> OmpClause {
    // Normalize clause name to lowercase for case-insensitive matching
    // (Fortran clauses are uppercase, C clauses are lowercase)
    // Note: One String allocation per clause is acceptable at C API boundary.
    // Alternative (build-time constant map) requires updating constants_gen.rs
    // to parse if-else chains instead of match expressions.
    let normalized_name = clause.name.to_ascii_lowercase();

    let clause_enum = lookup_clause_name(&normalized_name);
    let (kind, data) = match clause_enum {
        // Generated from compat/ompparser/ompparser/src/OpenMPKinds.h enum OpenMPClauseKind
        // The enum starts at index 0, incrementing by 1 for each OPENMP_CLAUSE entry
        // Only clauses that exist in ROUP's ClauseName enum (src/parser/clause.rs) are mapped
        crate::parser::ClauseName::If => (0, ClauseData { default: 0 }),
        crate::parser::ClauseName::NumThreads => (1, ClauseData { default: 0 }),
        crate::parser::ClauseName::Default => {
            let default_kind = parse_default_kind(clause);
            (
                2,
                ClauseData {
                    default: default_kind,
                },
            )
        }
        crate::parser::ClauseName::Private => (
            3,
            ClauseData {
                variables: ptr::null_mut(),
            },
        ),
        crate::parser::ClauseName::Firstprivate => (
            4,
            ClauseData {
                variables: ptr::null_mut(),
            },
        ),
        crate::parser::ClauseName::Shared => (
            5,
            ClauseData {
                variables: ptr::null_mut(),
            },
        ),
        // copyin is 6 in both OpenMP and OpenACC, but with different semantics
        crate::parser::ClauseName::CopyIn => (6, ClauseData { default: 0 }),
        crate::parser::ClauseName::Align => (7, ClauseData { default: 0 }),
        crate::parser::ClauseName::Reduction => (8, ClauseData { default: 0 }),
        // proc_bind = 9 (not in ROUP ClauseName yet, mapped via Other below)
        // allocate = 10 (not in ROUP ClauseName yet, mapped via Other below)
        crate::parser::ClauseName::NumTeams => (11, ClauseData { default: 0 }),
        crate::parser::ClauseName::ThreadLimit => (12, ClauseData { default: 0 }),
        crate::parser::ClauseName::Lastprivate => (
            13,
            ClauseData {
                variables: ptr::null_mut(),
            },
        ),
        crate::parser::ClauseName::Collapse => (14, ClauseData { default: 0 }),
        crate::parser::ClauseName::Ordered => (15, ClauseData { default: 0 }),
        // partial = 16 (not in ROUP ClauseName yet, mapped via Other below)
        crate::parser::ClauseName::Nowait => (17, ClauseData { default: 0 }),
        // full = 18 (not in ROUP ClauseName yet, mapped via Other below)
        // order = 19 (not in ROUP ClauseName yet, mapped via Other below)
        // linear = 20 (not in ROUP ClauseName yet, mapped via Other below)
        crate::parser::ClauseName::Schedule => {
            let schedule_kind = parse_schedule_kind(clause);
            (
                21,
                ClauseData {
                    schedule: ManuallyDrop::new(ScheduleData {
                        kind: schedule_kind,
                        modifiers: 0,
                        chunk: ptr::null(),
                    }),
                },
            )
        }
        // safelen to bind = 22-30
        crate::parser::ClauseName::DistSchedule => (29, ClauseData { default: 0 }),
        // 31-44 (not in ROUP ClauseName yet, mapped via Other below)
        crate::parser::ClauseName::InReduction => (45, ClauseData { default: 0 }),
        crate::parser::ClauseName::Depend => (
            46,
            ClauseData {
                variables: ptr::null_mut(),
            },
        ),
        crate::parser::ClauseName::UsesAllocators => (71, ClauseData { default: 0 }),
        crate::parser::ClauseName::TaskReduction => (75, ClauseData { default: 0 }),
        crate::parser::ClauseName::Destroy => (88, ClauseData { default: 0 }),
        crate::parser::ClauseName::DepobjUpdate => (89, ClauseData { default: 0 }),
        crate::parser::ClauseName::Read => (79, ClauseData { default: 0 }),
        crate::parser::ClauseName::Write => (80, ClauseData { default: 0 }),
        crate::parser::ClauseName::Update => (81, ClauseData { default: 0 }),
        crate::parser::ClauseName::Capture => (82, ClauseData { default: 0 }),
        crate::parser::ClauseName::Compare => (86, ClauseData { default: 0 }),
        crate::parser::ClauseName::CompareCapture => (87, ClauseData { default: 0 }),
        crate::parser::ClauseName::Otherwise => (144, ClauseData { default: 0 }),
        crate::parser::ClauseName::Other(ref s) => panic!("Unknown OpenMP clause: {}", s),
        _ => panic!("convert_clause mapping incomplete for constants generation"),
    };

    // Extract clause arguments based on ClauseKind
    // For simplicity, we reconstruct the clause as a string using the Display trait
    // This avoids having to handle all the complex ClauseKind variants
    let arguments = match &clause.kind {
        ClauseKind::Bare => {
            // For extension clauses (kind == 59), pass the clause name as the argument
            // IMPORTANT: ompparser expects the name WITHOUT the "ext_" prefix
            // e.g., "ext_user_test" → "user_test" (ompparser adds "ext_" when printing)
            if kind == 59 {
                let name = clause.name.as_ref();
                if let Some(stripped) = name.strip_prefix("ext_") {
                    allocate_c_string(stripped) // Skip "ext_" prefix
                } else {
                    allocate_c_string(name)
                }
            } else {
                ptr::null()
            }
        }
        ClauseKind::Parenthesized(ref args) => allocate_c_string(args.as_ref()),
        _ => ptr::null(),
    };

    OmpClause {
        kind,
        arguments,
        data,
    }
}

fn convert_default_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    match payload {
        IrClauseData::Default(kind) => {
            let value = match kind {
                DefaultKind::Shared => 0,
                DefaultKind::None => 1,
                DefaultKind::Private => 2,
                DefaultKind::Firstprivate => 3,
            };
            Some(OmpClause {
                kind: CLAUSE_KIND_DEFAULT,
                arguments: allocate_c_string(&kind.to_string()),
                data: ClauseData { default: value },
            })
        }
        IrClauseData::Expression(expr) => Some(OmpClause {
            kind: CLAUSE_KIND_DEFAULT,
            arguments: allocate_c_string(&expr.to_string()),
            data: ClauseData { default: -1 },
        }),
        _ => None,
    }
}

fn convert_defaultmap_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::Defaultmap { behavior, category } = payload {
        let behavior_code = defaultmap_behavior_code(*behavior);
        let category_code = category
            .map(defaultmap_category_code)
            .unwrap_or(defaultmap_category_code(DefaultmapCategory::Unspecified));
        let args = format_defaultmap_arguments(*behavior, *category);
        return Some(OmpClause {
            kind: CLAUSE_KIND_DEFAULTMAP,
            arguments: if args.is_empty() {
                ptr::null()
            } else {
                allocate_c_string(&args)
            },
            data: ClauseData {
                defaultmap: ManuallyDrop::new(DefaultmapData {
                    behavior: behavior_code,
                    category: category_code,
                }),
            },
        });
    }
    None
}

fn convert_num_threads_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::NumThreads { num } = payload {
        return Some(OmpClause {
            kind: CLAUSE_KIND_NUM_THREADS,
            arguments: allocate_c_string(&num.to_string()),
            data: ClauseData { default: 0 },
        });
    }
    None
}

fn convert_if_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::If {
        directive_name,
        condition,
    } = payload
    {
        let modifier_code = directive_name
            .as_ref()
            .map(if_modifier_code)
            .unwrap_or(ROUP_IF_MODIFIER_UNSPECIFIED);
        let args = condition.to_string();
        return Some(OmpClause {
            kind: CLAUSE_KIND_IF,
            arguments: allocate_c_string(&args),
            data: ClauseData {
                default: modifier_code,
            },
        });
    }
    None
}

fn convert_private_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::Private { items } = payload {
        return Some(build_variable_clause(CLAUSE_KIND_PRIVATE, items));
    }
    None
}

fn convert_firstprivate_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::Firstprivate { items } = payload {
        return Some(build_variable_clause(CLAUSE_KIND_FIRSTPRIVATE, items));
    }
    None
}

fn convert_shared_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::Shared { items } = payload {
        return Some(build_variable_clause(CLAUSE_KIND_SHARED, items));
    }
    None
}

fn convert_lastprivate_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::Lastprivate { modifier, items } = payload {
        let modifier_code = match modifier {
            Some(LastprivateModifier::Conditional) => 1,
            None => 0,
        };

        let args = render_arguments_from_payload(payload)
            .unwrap_or_else(|| format_clause_items(items).unwrap_or_default());

        return Some(OmpClause {
            kind: CLAUSE_KIND_LASTPRIVATE,
            arguments: if args.is_empty() {
                ptr::null()
            } else {
                allocate_c_string(&args)
            },
            data: ClauseData {
                lastprivate: ManuallyDrop::new(LastprivateData {
                    modifier: modifier_code,
                    variables: build_string_list_from_items(items),
                }),
            },
        });
    }
    None
}

fn convert_collapse_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::Collapse { n } = payload {
        return Some(OmpClause {
            kind: CLAUSE_KIND_COLLAPSE,
            arguments: allocate_c_string(&n.to_string()),
            data: ClauseData { default: 0 },
        });
    }
    None
}

fn convert_ordered_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::Ordered { n } = payload {
        return Some(OmpClause {
            kind: CLAUSE_KIND_ORDERED,
            arguments: n
                .as_ref()
                .map(|expr| allocate_c_string(&expr.to_string()))
                .unwrap_or(ptr::null()),
            data: ClauseData { default: 0 },
        });
    }
    None
}

fn convert_reduction_clause_from_ast(
    clause_kind: i32,
    payload: &IrClauseData,
) -> Option<OmpClause> {
    if let IrClauseData::Reduction {
        modifiers,
        operator,
        user_identifier,
        items,
        space_after_colon,
    } = payload
    {
        let data = build_reduction_data_from_ast(
            modifiers,
            *operator,
            user_identifier.as_ref(),
            items,
            *space_after_colon,
        );
        return Some(OmpClause {
            kind: clause_kind,
            arguments: format_reduction_arguments(
                modifiers,
                operator,
                user_identifier.as_ref(),
                items,
                *space_after_colon,
            ),
            data: ClauseData {
                reduction: ManuallyDrop::new(data),
            },
        });
    }
    None
}

fn convert_schedule_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::Schedule {
        kind,
        chunk_size,
        modifiers,
    } = payload
    {
        let args = format_schedule_arguments(*kind, modifiers, chunk_size.as_ref());
        let chunk_text = chunk_size
            .as_ref()
            .map(|expr| allocate_c_string(&expr.to_string()))
            .unwrap_or(ptr::null());
        let data = ClauseData {
            schedule: ManuallyDrop::new(ScheduleData {
                kind: schedule_kind_code(*kind),
                modifiers: schedule_modifier_mask(modifiers),
                chunk: chunk_text,
            }),
        };
        return Some(OmpClause {
            kind: CLAUSE_KIND_SCHEDULE,
            arguments: allocate_c_string(&args),
            data,
        });
    }
    None
}

fn convert_dist_schedule_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::DistSchedule { kind, chunk_size } = payload {
        let mut args = kind.to_string();
        let chunk_ptr = match chunk_size {
            Some(expr) => {
                args.push_str(", ");
                args.push_str(&expr.to_string());
                allocate_c_string(&expr.to_string())
            }
            None => ptr::null(),
        };
        return Some(OmpClause {
            kind: CLAUSE_KIND_DIST_SCHEDULE,
            arguments: allocate_c_string(&args),
            data: ClauseData {
                dist_schedule: ManuallyDrop::new(DistScheduleData {
                    kind: schedule_kind_code(*kind),
                    chunk: chunk_ptr,
                }),
            },
        });
    }
    None
}

fn convert_iterator_data(
    iterators: &[DependIterator],
    empty_step_as_null: bool,
) -> Vec<DependIteratorData> {
    iterators
        .iter()
        .map(|it| {
            let step_ptr = match (empty_step_as_null, it.step.as_ref()) {
                (true, Some(step)) => allocate_c_string(&step.to_string()),
                (true, None) => ptr::null(),
                (false, Some(step)) => allocate_c_string(&step.to_string()),
                (false, None) => allocate_c_string(""),
            };
            DependIteratorData {
                type_name: allocate_c_string(it.type_name.as_deref().unwrap_or("")),
                name: allocate_c_string(it.name.name()),
                start: allocate_c_string(&it.start.to_string()),
                end: allocate_c_string(&it.end.to_string()),
                step: step_ptr,
            }
        })
        .collect()
}

fn convert_map_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::Map {
        map_type,
        modifiers,
        mapper,
        iterators,
        items,
    } = payload
    {
        let mut parts = Vec::new();
        if !iterators.is_empty() {
            let defs = iterators
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            parts.push(format!("iterator ( {defs} )"));
        }
        if let Some(mapper_id) = mapper {
            parts.push(format!("mapper({mapper_id})"));
        }
        if !modifiers.is_empty() {
            parts.push(
                modifiers
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", "),
            );
        }
        if let Some(mt) = map_type {
            parts.push(format!("{mt}:"));
        } else if !parts.is_empty() {
            parts.push(":".to_string());
        }
        if let Some(list) = format_clause_items(items) {
            parts.push(list);
        }
        let args = parts.join(" ");

        let mapper_text = mapper
            .as_ref()
            .map(|id| allocate_c_string(id.as_str()))
            .unwrap_or(ptr::null());
        let variables = build_string_list_from_items(items);
        let iterator_data = convert_iterator_data(iterators, false);
        let data_ptr = Box::into_raw(Box::new(MapData {
            map_type: map_type_code(*map_type),
            modifiers: map_modifier_mask(modifiers),
            mapper: mapper_text,
            variables,
            iterators: iterator_data,
        }));
        return Some(OmpClause {
            kind: CLAUSE_KIND_MAP,
            arguments: if args.is_empty() {
                ptr::null()
            } else {
                allocate_c_string(&args)
            },
            data: ClauseData { map: data_ptr },
        });
    }
    None
}

fn convert_depend_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::Depend {
        depend_type,
        items,
        iterators,
    } = payload
    {
        let mut parts = Vec::new();
        if !iterators.is_empty() {
            let defs = iterators
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            parts.push(format!("iterator ( {defs} )"));
        }

        let mut args = depend_type.to_string();
        if let Some(list) = format_clause_items(items) {
            args.push(':');
            args.push(' ');
            args.push_str(&list);
        }
        parts.push(args);

        let variables = build_string_list_from_items(items);
        let iterator_data = convert_iterator_data(iterators, true);
        let data_ptr = Box::into_raw(Box::new(DependData {
            depend_type: depend_type_code(*depend_type),
            variables,
            iterators: iterator_data,
        }));
        return Some(OmpClause {
            kind: CLAUSE_KIND_DEPEND,
            arguments: allocate_c_string(&parts.join(", ")),
            data: ClauseData { depend: data_ptr },
        });
    }
    None
}

fn convert_depobj_update_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::DepobjUpdate { dependence } = payload {
        return Some(OmpClause {
            kind: CLAUSE_KIND_DEPOBJ_UPDATE,
            arguments: allocate_c_string(&dependence.to_string()),
            data: ClauseData {
                default: depobj_update_dependence_code(*dependence),
            },
        });
    }
    None
}

fn convert_update_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let Some(depobj) = convert_depobj_update_clause_from_ast(payload) {
        return Some(depobj);
    }
    convert_atomic_operation_clause_from_ast(CLAUSE_KIND_ATOMIC_UPDATE, payload)
}

fn convert_copyin_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::Copyin { items } = payload {
        return Some(build_variable_clause(CLAUSE_KIND_COPYIN, items));
    }
    None
}

fn convert_proc_bind_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::ProcBind(policy) = payload {
        return Some(OmpClause {
            kind: CLAUSE_KIND_PROC_BIND,
            arguments: allocate_c_string(&policy.to_string()),
            data: ClauseData {
                default: *policy as i32,
            },
        });
    }
    None
}

fn convert_num_teams_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::NumTeams { num } = payload {
        return Some(OmpClause {
            kind: CLAUSE_KIND_NUM_TEAMS,
            arguments: allocate_c_string(&num.to_string()),
            data: ClauseData { default: 0 },
        });
    }
    None
}

fn convert_thread_limit_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::ThreadLimit { limit } = payload {
        return Some(OmpClause {
            kind: CLAUSE_KIND_THREAD_LIMIT,
            arguments: allocate_c_string(&limit.to_string()),
            data: ClauseData { default: 0 },
        });
    }
    None
}

fn convert_linear_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::Linear {
        modifier,
        items,
        step,
    } = payload
    {
        let language = current_clause_language();
        let formatted_items: Vec<String> = items
            .iter()
            .map(|item| format_clause_item_for_language(item, language))
            .collect();
        if std::env::var_os("ROUP_DEBUG_LINEAR_RS").is_some() {
            eprintln!(
                "linear args debug: modifier={modifier:?} items={formatted_items:?} step={step:?}"
            );
        }

        let mut args = String::new();
        if let Some(m) = modifier {
            args.push_str(&m.to_string());
            if !formatted_items.is_empty() {
                args.push('(');
                args.push_str(&formatted_items.join(", "));
                args.push(')');
            }
        } else if let Some(list) = format_clause_items(items) {
            args.push_str(&list);
        }
        if let Some(expr) = step {
            if !args.is_empty() {
                args.push_str(": ");
            }
            args.push_str(&expr.to_string());
        }

        if matches!(language, Language::FortranFree | Language::FortranFixed) && !args.is_empty() {
            args = normalize_fortran_variable_text(&args);
        }

        let variables = build_string_list_from_items(items);

        let step_text = step
            .as_ref()
            .map(|expr| allocate_c_string(&expr.to_string()))
            .unwrap_or(ptr::null());
        let data_ptr = Box::into_raw(Box::new(LinearData {
            modifier: linear_modifier_code(*modifier),
            step: step_text,
            variables,
        }));
        return Some(OmpClause {
            kind: CLAUSE_KIND_LINEAR,
            arguments: if args.is_empty() {
                ptr::null()
            } else {
                allocate_c_string(&args)
            },
            data: ClauseData { linear: data_ptr },
        });
    }
    None
}

fn convert_aligned_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::Aligned { items, alignment } = payload {
        let mut args = String::new();
        if let Some(list) = format_clause_items(items) {
            args.push_str(&list);
        }
        let alignment_ptr = if let Some(expr) = alignment {
            if !args.is_empty() {
                args.push(':');
            }
            args.push_str(&expr.to_string());
            allocate_c_string(&expr.to_string())
        } else {
            ptr::null()
        };
        let variables = build_string_list_from_items(items);
        let data_ptr = Box::into_raw(Box::new(AlignedData {
            variables,
            alignment: alignment_ptr,
        }));
        return Some(OmpClause {
            kind: CLAUSE_KIND_ALIGNED,
            arguments: if args.is_empty() {
                ptr::null()
            } else {
                allocate_c_string(&args)
            },
            data: ClauseData { aligned: data_ptr },
        });
    }
    None
}

fn convert_safelen_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::Safelen { length } = payload {
        return Some(OmpClause {
            kind: CLAUSE_KIND_SAFELEN,
            arguments: allocate_c_string(&length.to_string()),
            data: ClauseData { default: 0 },
        });
    }
    None
}

fn convert_simdlen_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::Simdlen { length } = payload {
        return Some(OmpClause {
            kind: CLAUSE_KIND_SIMDLEN,
            arguments: allocate_c_string(&length.to_string()),
            data: ClauseData { default: 0 },
        });
    }
    None
}

fn convert_use_device_ptr_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::UseDevicePtr { items } = payload {
        return Some(build_variable_clause(CLAUSE_KIND_USE_DEVICE_PTR, items));
    }
    None
}

fn convert_use_device_addr_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::UseDeviceAddr { items } = payload {
        return Some(build_variable_clause(CLAUSE_KIND_USE_DEVICE_ADDR, items));
    }
    None
}

fn convert_is_device_ptr_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::IsDevicePtr { items } = payload {
        return Some(build_variable_clause(CLAUSE_KIND_IS_DEVICE_PTR, items));
    }
    None
}

fn convert_has_device_addr_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::HasDeviceAddr { items } = payload {
        return Some(build_variable_clause(CLAUSE_KIND_HAS_DEVICE_ADDR, items));
    }
    None
}

fn convert_uses_allocators_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::UsesAllocators { allocators } = payload {
        let args = format_uses_allocators_arguments(allocators);
        let data_ptr = build_uses_allocators_data_from_ast(allocators);
        return Some(OmpClause {
            kind: CLAUSE_KIND_USES_ALLOCATORS,
            arguments: if args.is_empty() {
                ptr::null()
            } else {
                allocate_c_string(&args)
            },
            data: ClauseData {
                uses_allocators: data_ptr,
            },
        });
    }
    None
}

fn convert_requires_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::Requires { requirements } = payload {
        let args = format_requires_arguments(requirements);
        let data_ptr = build_requires_data_from_ast(requirements);
        return Some(OmpClause {
            kind: CLAUSE_KIND_REQUIRES,
            arguments: args.map_or(ptr::null(), |s| allocate_c_string(&s)),
            data: ClauseData { requires: data_ptr },
        });
    }
    None
}

fn convert_allocate_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::Allocate { allocator, items } = payload {
        let mut args = String::new();
        if let Some(alloc) = allocator {
            args.push_str(&alloc.to_string());
            if !items.is_empty() {
                args.push_str(": ");
            }
        }
        if let Some(list) = format_clause_items(items) {
            args.push_str(&list);
        }
        let (allocator_text, allocator_kind) =
            allocator
                .as_ref()
                .map_or((ptr::null(), ROUP_OMPA_USES_ALLOCATOR_USER), |id| {
                    let allocator_kind = allocator_kind_from_identifier(id);
                    let kind_code = uses_allocator_kind_code(&allocator_kind).0;
                    let text_ptr = allocate_c_string(id.name());
                    (text_ptr, kind_code)
                });
        let variables = build_string_list_from_items(items);
        let data_ptr = Box::into_raw(Box::new(AllocateData {
            kind: allocator_kind,
            allocator: allocator_text,
            variables,
        }));

        return Some(OmpClause {
            kind: CLAUSE_KIND_ALLOCATE,
            arguments: if args.is_empty() {
                ptr::null()
            } else {
                allocate_c_string(&args)
            },
            data: ClauseData { allocate: data_ptr },
        });
    }
    None
}

fn convert_allocator_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::Allocator { allocator } = payload {
        let (kind_code, _) = uses_allocator_kind_code(&allocator_kind_from_identifier(allocator));
        return Some(OmpClause {
            kind: CLAUSE_KIND_ALLOCATOR,
            arguments: allocate_c_string(&allocator.to_string()),
            data: ClauseData { default: kind_code },
        });
    }
    None
}

fn convert_copyprivate_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::Copyprivate { items } = payload {
        return Some(build_variable_clause(CLAUSE_KIND_COPYPRIVATE, items));
    }
    None
}

fn convert_affinity_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::Affinity {
        modifier,
        iterators,
        items,
    } = payload
    {
        let mut args = String::new();
        if *modifier == AffinityModifier::Iterator && !iterators.is_empty() {
            let defs = iterators
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            args.push_str(&format!("iterator ( {defs} )"));
            if let Some(list) = format_clause_items(items) {
                args.push_str(" : ");
                args.push_str(&list);
            }
        } else {
            if *modifier != AffinityModifier::Unspecified {
                args.push_str(&modifier.to_string());
                if !items.is_empty() {
                    args.push_str(": ");
                }
            }
            if let Some(list) = format_clause_items(items) {
                args.push_str(&list);
            }
        }

        let variables = build_string_list_from_items(items);
        let iterator_data = convert_iterator_data(iterators, false);
        let data_ptr = Box::into_raw(Box::new(AffinityData {
            modifier: *modifier as i32,
            variables,
            iterators: iterator_data,
        }));

        return Some(OmpClause {
            kind: CLAUSE_KIND_AFFINITY,
            arguments: if args.is_empty() {
                ptr::null()
            } else {
                allocate_c_string(&args)
            },
            data: ClauseData { affinity: data_ptr },
        });
    }
    None
}

fn convert_priority_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::Priority { priority } = payload {
        return Some(OmpClause {
            kind: CLAUSE_KIND_PRIORITY,
            arguments: allocate_c_string(&priority.to_string()),
            data: ClauseData { default: 0 },
        });
    }
    None
}

fn convert_grainsize_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::Grainsize { modifier, grain } = payload {
        return Some(OmpClause {
            kind: CLAUSE_KIND_GRAINSIZE,
            arguments: allocate_c_string(&grain.to_string()),
            data: ClauseData {
                default: *modifier as i32,
            },
        });
    }
    None
}

fn convert_num_tasks_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::NumTasks { modifier, num } = payload {
        return Some(OmpClause {
            kind: CLAUSE_KIND_NUM_TASKS,
            arguments: allocate_c_string(&num.to_string()),
            data: ClauseData {
                default: *modifier as i32,
            },
        });
    }
    None
}

fn convert_filter_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::Filter { thread_num } = payload {
        return Some(OmpClause {
            kind: CLAUSE_KIND_FILTER,
            arguments: allocate_c_string(&thread_num.to_string()),
            data: ClauseData { default: 0 },
        });
    }
    None
}

fn convert_device_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::Device {
        modifier,
        device_num,
    } = payload
    {
        let modifier_code = match modifier {
            DeviceModifier::Unspecified => 2,
            DeviceModifier::Ancestor => 0,
            DeviceModifier::DeviceNum => 1,
        };
        let expr_ptr = allocate_c_string(&device_num.to_string());
        return Some(OmpClause {
            kind: CLAUSE_KIND_DEVICE,
            arguments: expr_ptr,
            data: ClauseData {
                default: modifier_code,
            },
        });
    }
    None
}

fn convert_device_type_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::DeviceType(device_type) = payload {
        return Some(OmpClause {
            kind: CLAUSE_KIND_DEVICE_TYPE,
            arguments: allocate_c_string(&device_type.to_string()),
            data: ClauseData {
                default: *device_type as i32,
            },
        });
    }
    None
}

fn convert_order_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::Order { modifier, kind } = payload {
        let mut text = String::new();
        if *modifier != OrderModifier::Unspecified {
            text.push_str(&modifier.to_string());
            text.push_str(": ");
        }
        text.push_str(&kind.to_string());

        return Some(OmpClause {
            kind: CLAUSE_KIND_ORDER,
            arguments: allocate_c_string(&text),
            data: ClauseData {
                order: ManuallyDrop::new(OrderData {
                    modifier: *modifier as i32,
                    kind: *kind as i32,
                }),
            },
        });
    }
    None
}

fn convert_atomic_default_mem_order_clause_from_ast(payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::AtomicDefaultMemOrder(order) = payload {
        return Some(OmpClause {
            kind: CLAUSE_KIND_ATOMIC_DEFAULT_MEM_ORDER,
            arguments: allocate_c_string(&order.to_string()),
            data: ClauseData {
                default: *order as i32,
            },
        });
    }
    None
}

fn convert_atomic_operation_clause_from_ast(
    clause_kind: i32,
    payload: &IrClauseData,
) -> Option<OmpClause> {
    if let IrClauseData::AtomicOperation { op, memory_order } = payload {
        let matches_kind = match clause_kind {
            CLAUSE_KIND_ATOMIC_READ => *op == AtomicOp::Read,
            CLAUSE_KIND_ATOMIC_WRITE => *op == AtomicOp::Write,
            CLAUSE_KIND_ATOMIC_UPDATE => *op == AtomicOp::Update,
            CLAUSE_KIND_ATOMIC_CAPTURE => *op == AtomicOp::Capture,
            _ => true,
        };
        if !matches_kind {
            return None;
        }
        return Some(OmpClause {
            kind: clause_kind,
            arguments: memory_order
                .as_ref()
                .map(|order| allocate_c_string(&order.to_string()))
                .unwrap_or(ptr::null()),
            data: ClauseData { default: 0 },
        });
    }
    None
}

fn format_variable_for_language(variable: &Variable, language: Language) -> String {
    if matches!(language, Language::FortranFree | Language::FortranFixed) {
        if let Some(original) = variable.original() {
            return normalize_fortran_variable_text(original);
        }
    }
    variable.to_string()
}

fn normalize_fortran_variable_text(original: &str) -> String {
    let mut result = String::new();
    let mut chars = original.chars().peekable();

    while let Some(ch) = chars.next() {
        result.push(ch);

        if ch == '(' || ch == ',' {
            // Skip any existing spaces immediately after the delimiter to avoid duplicates
            while matches!(chars.peek(), Some(' ')) {
                chars.next();
            }
            // Do not append space before a closing paren or colon (e.g., (:))
            if let Some(next) = chars.peek() {
                if *next != ')' && *next != ':' {
                    result.push(' ');
                }
            }
        }
    }

    result
}

fn format_clause_item_for_language(item: &ClauseItem, language: Language) -> String {
    match item {
        ClauseItem::Identifier(id) => id.name().to_string(),
        ClauseItem::Variable(var) => format_variable_for_language(var, language),
        ClauseItem::Expression(expr) => expr.as_str().to_string(),
    }
}

fn format_depend_iterator(data: &DependIteratorData) -> Option<Cow<'_, str>> {
    unsafe {
        let name = if data.name.is_null() {
            None
        } else {
            Some(CStr::from_ptr(data.name).to_string_lossy().into_owned())
        }?;
        let start = if data.start.is_null() {
            None
        } else {
            Some(CStr::from_ptr(data.start).to_string_lossy().into_owned())
        }?;
        let end = if data.end.is_null() {
            None
        } else {
            Some(CStr::from_ptr(data.end).to_string_lossy().into_owned())
        }?;
        let type_name = if data.type_name.is_null() {
            None
        } else {
            Some(
                CStr::from_ptr(data.type_name)
                    .to_string_lossy()
                    .into_owned(),
            )
        };
        let step = if data.step.is_null() {
            None
        } else {
            Some(CStr::from_ptr(data.step).to_string_lossy().into_owned())
        };

        let mut text = String::new();
        if let Some(ty) = type_name {
            if !ty.is_empty() {
                text.push_str(&ty);
                text.push(' ');
            }
        }
        text.push_str(&name);
        text.push('=');
        text.push_str(&start);
        text.push(':');
        text.push_str(&end);
        if let Some(step_val) = step {
            if !step_val.is_empty() {
                text.push(':');
                text.push_str(&step_val);
            }
        }
        Some(Cow::Owned(text))
    }
}

fn format_clause_items(items: &[ClauseItem]) -> Option<String> {
    if items.is_empty() {
        return None;
    }
    let language = current_clause_language();
    let rendered: Vec<String> = items
        .iter()
        .map(|item| format_clause_item_for_language(item, language))
        .collect();
    Some(rendered.join(", "))
}

fn build_string_list_from_items(items: &[ClauseItem]) -> *mut OmpStringList {
    if items.is_empty() {
        return ptr::null_mut();
    }
    let language = current_clause_language();
    let cows: Vec<Cow<'_, str>> = items
        .iter()
        .map(|item| Cow::Owned(format_clause_item_for_language(item, language)))
        .collect();
    build_string_list(&cows)
}

fn build_string_list_from_identifiers(
    identifiers: &[Identifier],
    language: Language,
) -> *mut OmpStringList {
    if identifiers.is_empty() {
        return ptr::null_mut();
    }
    let _ = language; // Reserved for future language-specific formatting
    let cows: Vec<Cow<'_, str>> = identifiers
        .iter()
        .map(|id| Cow::Owned(id.name().to_string()))
        .collect();
    build_string_list(&cows)
}

fn convert_item_list_clause_from_ast(code: i32, payload: &IrClauseData) -> Option<OmpClause> {
    if let IrClauseData::ItemList(items) = payload {
        return Some(build_variable_clause(code, items));
    }
    None
}

fn build_variable_clause(code: i32, items: &[ClauseItem]) -> OmpClause {
    OmpClause {
        kind: code,
        arguments: format_clause_items(items)
            .map(|s| allocate_c_string(&s))
            .unwrap_or(ptr::null()),
        data: ClauseData {
            variables: build_string_list_from_items(items),
        },
    }
}

fn format_reduction_argument_text(
    modifiers: &[ReductionModifier],
    operator: &ReductionOperator,
    user_identifier: Option<&Identifier>,
    items: &[ClauseItem],
    space_after_colon: bool,
) -> String {
    let mut segments = Vec::new();
    if !modifiers.is_empty() {
        let mods: Vec<String> = modifiers.iter().map(|m| m.to_string()).collect();
        segments.push(mods.join(", "));
    }

    let op_text = match (operator, user_identifier) {
        (ReductionOperator::Custom, Some(id)) => id.name().to_string(),
        _ => operator.to_string(),
    };
    segments.push(op_text);

    let mut rendered = segments.join(", ");
    rendered.push_str(if space_after_colon { ": " } else { ":" });
    if let Some(vars) = format_clause_items(items) {
        rendered.push_str(&vars);
    }
    rendered
}

fn format_reduction_arguments(
    modifiers: &[ReductionModifier],
    operator: &ReductionOperator,
    user_identifier: Option<&Identifier>,
    items: &[ClauseItem],
    space_after_colon: bool,
) -> *const c_char {
    allocate_c_string(&format_reduction_argument_text(
        modifiers,
        operator,
        user_identifier,
        items,
        space_after_colon,
    ))
}

fn format_defaultmap_arguments(
    behavior: DefaultmapBehavior,
    category: Option<DefaultmapCategory>,
) -> String {
    if behavior == DefaultmapBehavior::Unspecified && category.is_none() {
        String::new()
    } else if let Some(cat) = category {
        format!("{behavior}: {cat}")
    } else {
        behavior.to_string()
    }
}

fn defaultmap_behavior_code(value: DefaultmapBehavior) -> i32 {
    value as i32
}

fn defaultmap_category_code(value: DefaultmapCategory) -> i32 {
    value as i32
}

fn format_schedule_arguments(
    kind: IrScheduleKind,
    modifiers: &[ScheduleModifier],
    chunk: Option<&crate::ir::Expression>,
) -> String {
    let mut parts = Vec::new();
    if !modifiers.is_empty() {
        let mods: Vec<String> = modifiers.iter().map(|m| m.to_string()).collect();
        parts.push(mods.join(", "));
    }

    let mut result = kind.to_string();
    if let Some(expr) = chunk {
        result.push_str(", ");
        result.push_str(&expr.to_string());
    }

    if parts.is_empty() {
        result
    } else {
        format!("{}: {}", parts.join(", "), result)
    }
}

fn build_reduction_data_from_ast(
    modifiers: &[ReductionModifier],
    operator: ReductionOperator,
    user_identifier: Option<&Identifier>,
    items: &[ClauseItem],
    space_after_colon: bool,
) -> ReductionData {
    let modifier_mask = modifiers.iter().fold(0, |acc, m| {
        acc | match m {
            ReductionModifier::Task => REDUCTION_MODIFIER_TASK,
            ReductionModifier::Inscan => REDUCTION_MODIFIER_INSCAN,
            ReductionModifier::Default => REDUCTION_MODIFIER_DEFAULT,
        }
    });

    let modifiers_text = if modifiers.is_empty() {
        ptr::null()
    } else {
        let joined = modifiers
            .iter()
            .map(|m| m.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        allocate_c_string(&joined)
    };

    let user_identifier_ptr = user_identifier
        .map(|id| allocate_c_string(id.name()))
        .unwrap_or(ptr::null());

    ReductionData {
        operator: reduction_operator_code_from_ir(operator),
        modifier_mask,
        modifiers_text,
        user_identifier: user_identifier_ptr,
        variables: build_string_list_from_items(items),
        space_after_colon,
    }
}

fn schedule_kind_code(kind: IrScheduleKind) -> i32 {
    match kind {
        IrScheduleKind::Static => 0,
        IrScheduleKind::Dynamic => 1,
        IrScheduleKind::Guided => 2,
        IrScheduleKind::Auto => 3,
        IrScheduleKind::Runtime => 4,
    }
}

fn schedule_modifier_mask(modifiers: &[ScheduleModifier]) -> u32 {
    modifiers.iter().fold(0, |acc, m| {
        acc | match m {
            ScheduleModifier::Monotonic => SCHEDULE_MODIFIER_MONOTONIC,
            ScheduleModifier::Nonmonotonic => SCHEDULE_MODIFIER_NONMONOTONIC,
            ScheduleModifier::Simd => SCHEDULE_MODIFIER_SIMD,
        }
    })
}

fn reduction_operator_code_from_ir(op: ReductionOperator) -> i32 {
    match op {
        ReductionOperator::Add => 0,
        ReductionOperator::Subtract => 1,
        ReductionOperator::Multiply => 2,
        ReductionOperator::BitwiseAnd => 3,
        ReductionOperator::BitwiseOr => 4,
        ReductionOperator::BitwiseXor => 5,
        ReductionOperator::LogicalAnd => 6,
        ReductionOperator::LogicalOr => 7,
        ReductionOperator::Min => 8,
        ReductionOperator::Max => 9,
        ReductionOperator::MinusEqual => 15,
        ReductionOperator::Custom => -1,
    }
}

fn format_uses_allocators_arguments(specs: &[UsesAllocatorSpec]) -> String {
    let mut parts = Vec::new();
    for spec in specs {
        let mut entry = spec.allocator.canonical_name().to_string();
        if let Some(traits) = spec.traits.as_ref() {
            entry.push('(');
            entry.push_str(&traits.to_string());
            entry.push(')');
        }
        parts.push(entry);
    }
    parts.join(", ")
}

fn format_requires_arguments(requirements: &[RequireModifier]) -> Option<String> {
    if requirements.is_empty() {
        return None;
    }

    let mut rendered = String::new();
    for (idx, req) in requirements.iter().enumerate() {
        if idx > 0 {
            rendered.push(' ');
        }
        match req {
            RequireModifier::ReverseOffload => rendered.push_str("reverse_offload"),
            RequireModifier::UnifiedAddress => rendered.push_str("unified_address"),
            RequireModifier::UnifiedSharedMemory => rendered.push_str("unified_shared_memory"),
            RequireModifier::DynamicAllocators => rendered.push_str("dynamic_allocators"),
            RequireModifier::SelfMaps => rendered.push_str("self_maps"),
            RequireModifier::AtomicDefaultMemOrder(order) => {
                rendered.push_str("atomic_default_mem_order(");
                rendered.push_str(&order.to_string());
                rendered.push(')');
            }
            RequireModifier::ExtImplementationDefinedRequirement(name) => {
                if let Some(id) = name {
                    rendered.push_str(id.as_str());
                } else {
                    rendered.push_str("ext_implementation_defined_requirement");
                }
            }
        }
    }

    Some(rendered)
}

fn build_requires_data_from_ast(requirements: &[RequireModifier]) -> *mut RequiresData {
    if requirements.is_empty() {
        return ptr::null_mut();
    }

    let mut entries = Vec::with_capacity(requirements.len());
    for req in requirements {
        let (code, name) = match req {
            RequireModifier::ReverseOffload => (REQUIRE_MOD_REVERSE_OFFLOAD, ptr::null()),
            RequireModifier::UnifiedAddress => (REQUIRE_MOD_UNIFIED_ADDRESS, ptr::null()),
            RequireModifier::UnifiedSharedMemory => {
                (REQUIRE_MOD_UNIFIED_SHARED_MEMORY, ptr::null())
            }
            RequireModifier::DynamicAllocators => (REQUIRE_MOD_DYNAMIC_ALLOCATORS, ptr::null()),
            RequireModifier::SelfMaps => (REQUIRE_MOD_SELF_MAPS, ptr::null()),
            RequireModifier::AtomicDefaultMemOrder(order) => {
                (map_memory_order_to_require_kind(*order), ptr::null())
            }
            RequireModifier::ExtImplementationDefinedRequirement(name) => {
                let name_ptr = name
                    .as_ref()
                    .map(|id| allocate_c_string(id.as_str()))
                    .unwrap_or(ptr::null());
                (REQUIRE_MOD_EXT_IMPL_DEFINED, name_ptr)
            }
        };
        entries.push(RequireEntryData { code, name });
    }

    Box::into_raw(Box::new(RequiresData { entries }))
}

fn map_memory_order_to_require_kind(order: MemoryOrder) -> i32 {
    match order {
        MemoryOrder::SeqCst => REQUIRE_MOD_ATOMIC_SEQ_CST,
        MemoryOrder::AcqRel => REQUIRE_MOD_ATOMIC_ACQ_REL,
        MemoryOrder::Release => REQUIRE_MOD_ATOMIC_RELEASE,
        MemoryOrder::Acquire => REQUIRE_MOD_ATOMIC_ACQUIRE,
        MemoryOrder::Relaxed => REQUIRE_MOD_ATOMIC_RELAXED,
    }
}

fn map_modifier_mask(modifiers: &[MapModifier]) -> u32 {
    let mut mask = 0;
    for modifier in modifiers {
        mask |= match modifier {
            MapModifier::Always => MAP_MODIFIER_ALWAYS,
            MapModifier::Close => MAP_MODIFIER_CLOSE,
            MapModifier::Present => MAP_MODIFIER_PRESENT,
            MapModifier::SelfMap => MAP_MODIFIER_SELF,
            MapModifier::OmpxHold => MAP_MODIFIER_OMPX_HOLD,
        };
    }
    mask
}

fn map_type_code(map_type: Option<MapType>) -> i32 {
    map_type.map(|mt| mt as i32).unwrap_or(MAP_TYPE_UNSPECIFIED)
}

fn linear_modifier_code(modifier: Option<LinearModifier>) -> i32 {
    modifier.map(|m| m as i32).unwrap_or(-1)
}

fn depend_type_code(depend_type: DependType) -> i32 {
    depend_type as i32
}

fn build_uses_allocators_data_from_ast(specs: &[UsesAllocatorSpec]) -> *mut UsesAllocatorsData {
    if specs.is_empty() {
        return ptr::null_mut();
    }

    let mut entries = Vec::with_capacity(specs.len());
    for spec in specs {
        let (kind_code, user_name) = uses_allocator_kind_code(&spec.allocator);
        let user_ptr = match user_name {
            Some(name) if !name.is_empty() => allocate_c_string(&name),
            _ => ptr::null(),
        };
        let traits_ptr = spec
            .traits
            .as_ref()
            .map(|expr| allocate_c_string(&expr.to_string()))
            .unwrap_or(ptr::null());
        entries.push(UsesAllocatorEntryData {
            kind: kind_code,
            user_name: user_ptr,
            traits: traits_ptr,
        });
    }

    Box::into_raw(Box::new(UsesAllocatorsData { entries }))
}

fn uses_allocator_kind_code(kind: &UsesAllocatorKind) -> (i32, Option<String>) {
    match kind {
        UsesAllocatorKind::Builtin(builtin) => (uses_allocator_builtin_code(*builtin), None),
        UsesAllocatorKind::Custom(identifier) => {
            (ROUP_OMPA_USES_ALLOCATOR_USER, Some(identifier.to_string()))
        }
    }
}

fn uses_allocator_builtin_code(kind: UsesAllocatorBuiltin) -> i32 {
    match kind {
        UsesAllocatorBuiltin::Default => ROUP_OMPA_USES_ALLOCATOR_DEFAULT,
        UsesAllocatorBuiltin::LargeCap => ROUP_OMPA_USES_ALLOCATOR_LARGE_CAP,
        UsesAllocatorBuiltin::Const => ROUP_OMPA_USES_ALLOCATOR_CONST,
        UsesAllocatorBuiltin::HighBw => ROUP_OMPA_USES_ALLOCATOR_HIGH_BW,
        UsesAllocatorBuiltin::LowLat => ROUP_OMPA_USES_ALLOCATOR_LOW_LAT,
        UsesAllocatorBuiltin::Cgroup => ROUP_OMPA_USES_ALLOCATOR_CGROUP,
        UsesAllocatorBuiltin::Pteam => ROUP_OMPA_USES_ALLOCATOR_PTEAM,
        UsesAllocatorBuiltin::Thread => ROUP_OMPA_USES_ALLOCATOR_THREAD,
    }
}

fn allocator_kind_from_identifier(id: &Identifier) -> UsesAllocatorKind {
    match id.as_str() {
        "omp_default_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::Default),
        "omp_large_cap_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::LargeCap),
        "omp_const_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::Const),
        "omp_high_bw_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::HighBw),
        "omp_low_lat_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::LowLat),
        "omp_cgroup_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::Cgroup),
        "omp_pteam_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::Pteam),
        "omp_thread_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::Thread),
        _ => UsesAllocatorKind::Custom(id.clone()),
    }
}

fn if_modifier_code(name: &Identifier) -> i32 {
    let normalized = name.as_str().to_ascii_lowercase();
    match normalized.as_str() {
        "parallel" => ROUP_IF_MODIFIER_PARALLEL,
        "task" => ROUP_IF_MODIFIER_TASK,
        "taskloop" => ROUP_IF_MODIFIER_TASKLOOP,
        "target" => ROUP_IF_MODIFIER_TARGET,
        "target data" => ROUP_IF_MODIFIER_TARGET_DATA,
        "target enter data" => ROUP_IF_MODIFIER_TARGET_ENTER_DATA,
        "target exit data" => ROUP_IF_MODIFIER_TARGET_EXIT_DATA,
        "target update" => ROUP_IF_MODIFIER_TARGET_UPDATE,
        "simd" => ROUP_IF_MODIFIER_SIMD,
        "cancel" => ROUP_IF_MODIFIER_CANCEL,
        "" => ROUP_IF_MODIFIER_UNSPECIFIED,
        _ => ROUP_IF_MODIFIER_USER,
    }
}

/// Parse schedule kind from clause arguments.
///
/// Extracts schedule type from clause like "schedule(dynamic, 4)".
/// Returns integer code for the schedule policy.
///
/// ## Schedule Codes:
/// - 0 = static   (default, divide iterations evenly)
/// - 1 = dynamic  (distribute at runtime)
/// - 2 = guided   (decreasing chunk sizes)
/// - 3 = auto     (compiler decides)
/// - 4 = runtime  (OMP_SCHEDULE environment variable)
#[allow(dead_code)] // Used by constants/header generation tooling
fn parse_schedule_kind(clause: &Clause) -> i32 {
    if let ClauseKind::Parenthesized(ref args) = clause.kind {
        let args = args.as_ref();
        // Use IR schedule parser to canonicalize modifiers and kind
        let config = ParserConfig::default();
        if let Ok(crate::ir::ClauseData::Schedule { kind, .. }) =
            crate::ir::convert::parse_schedule_clause(args, &config)
        {
            use crate::ir::ScheduleKind;
            return match kind {
                ScheduleKind::Static => 0,
                ScheduleKind::Dynamic => 1,
                ScheduleKind::Guided => 2,
                ScheduleKind::Auto => 3,
                ScheduleKind::Runtime => 4,
            };
        }
    }
    0 // Default to static
}

/// Parse default clause data-sharing attribute.
///
/// Extracts the default sharing from clause like "default(shared)".
/// Returns integer code for the default policy.
///
/// ## Default Codes:
/// - 0 = shared (all variables shared by default)
/// - 1 = none   (must explicitly declare all variables)
#[allow(dead_code)] // Used by constants/header generation tooling
fn parse_default_kind(clause: &Clause) -> i32 {
    if let ClauseKind::Parenthesized(ref args) = clause.kind {
        let args = args.as_ref();
        match args.trim().to_ascii_lowercase().as_str() {
            "shared" => return 0,
            "none" => return 1,
            _ => {}
        }
    }
    0 // Default to shared
}

/// Map a typed clause name to the ompparser clause kind code.
/// This is used by the constants generator to keep roup_constants.h in sync
/// with the enum-driven mappings instead of relying on string parsing.
#[allow(dead_code)] // Used by constants/header generation tooling
fn clause_name_enum_to_kind(name: ClauseName) -> i32 {
    use ClauseName::*;
    match name {
        Default => CLAUSE_KIND_DEFAULT,
        Defaultmap => CLAUSE_KIND_DEFAULTMAP,
        If => CLAUSE_KIND_IF,
        NumThreads => CLAUSE_KIND_NUM_THREADS,
        Private => CLAUSE_KIND_PRIVATE,
        Firstprivate => CLAUSE_KIND_FIRSTPRIVATE,
        Shared => CLAUSE_KIND_SHARED,
        Lastprivate => CLAUSE_KIND_LASTPRIVATE,
        CopyIn => CLAUSE_KIND_COPYIN,
        Align => CLAUSE_KIND_ALIGN,
        Nowait => CLAUSE_KIND_NOWAIT,
        Nogroup => CLAUSE_KIND_NOGROUP,
        Untied => CLAUSE_KIND_UNTIED,
        Mergeable => CLAUSE_KIND_MERGEABLE,
        SeqCst => CLAUSE_KIND_SEQ_CST,
        Relaxed => CLAUSE_KIND_RELAXED,
        Release => CLAUSE_KIND_RELEASE,
        Acquire => CLAUSE_KIND_ACQUIRE,
        AcqRel => CLAUSE_KIND_ACQ_REL,
        ProcBind => CLAUSE_KIND_PROC_BIND,
        NumTeams => CLAUSE_KIND_NUM_TEAMS,
        ThreadLimit => CLAUSE_KIND_THREAD_LIMIT,
        Collapse => CLAUSE_KIND_COLLAPSE,
        Ordered => CLAUSE_KIND_ORDERED,
        Linear => CLAUSE_KIND_LINEAR,
        Safelen => CLAUSE_KIND_SAFELEN,
        Simdlen => CLAUSE_KIND_SIMDLEN,
        Aligned => CLAUSE_KIND_ALIGNED,
        Bind => CLAUSE_KIND_BIND,
        Reduction => CLAUSE_KIND_REDUCTION,
        InReduction => CLAUSE_KIND_IN_REDUCTION,
        TaskReduction => CLAUSE_KIND_TASK_REDUCTION,
        Requires => CLAUSE_KIND_REQUIRES,
        Schedule => CLAUSE_KIND_SCHEDULE,
        DistSchedule => CLAUSE_KIND_DIST_SCHEDULE,
        Map => CLAUSE_KIND_MAP,
        Otherwise => CLAUSE_KIND_OTHERWISE,
        To => CLAUSE_KIND_TO,
        From => CLAUSE_KIND_FROM,
        UseDevicePtr => CLAUSE_KIND_USE_DEVICE_PTR,
        UseDeviceAddr => CLAUSE_KIND_USE_DEVICE_ADDR,
        IsDevicePtr => CLAUSE_KIND_IS_DEVICE_PTR,
        HasDeviceAddr => CLAUSE_KIND_HAS_DEVICE_ADDR,
        Sizes => CLAUSE_KIND_SIZES,
        UsesAllocators => CLAUSE_KIND_USES_ALLOCATORS,
        Depend => CLAUSE_KIND_DEPEND,
        Device => CLAUSE_KIND_DEVICE,
        DeviceType => CLAUSE_KIND_DEVICE_TYPE,
        DepobjUpdate => CLAUSE_KIND_DEPOBJ_UPDATE,
        Allocate => CLAUSE_KIND_ALLOCATE,
        Allocator => CLAUSE_KIND_ALLOCATOR,
        Copyprivate => CLAUSE_KIND_COPYPRIVATE,
        Affinity => CLAUSE_KIND_AFFINITY,
        Priority => CLAUSE_KIND_PRIORITY,
        Grainsize => CLAUSE_KIND_GRAINSIZE,
        NumTasks => CLAUSE_KIND_NUM_TASKS,
        Filter => CLAUSE_KIND_FILTER,
        Order => CLAUSE_KIND_ORDER,
        AtomicDefaultMemOrder => CLAUSE_KIND_ATOMIC_DEFAULT_MEM_ORDER,
        Read => CLAUSE_KIND_ATOMIC_READ,
        Write => CLAUSE_KIND_ATOMIC_WRITE,
        Update => CLAUSE_KIND_ATOMIC_UPDATE,
        Capture => CLAUSE_KIND_ATOMIC_CAPTURE,
        Nontemporal => CLAUSE_KIND_NONTEMPORAL,
        Uniform => CLAUSE_KIND_UNIFORM,
        Inbranch => CLAUSE_KIND_INBRANCH,
        Notinbranch => CLAUSE_KIND_NOTINBRANCH,
        Inclusive => CLAUSE_KIND_INCLUSIVE,
        Exclusive => CLAUSE_KIND_EXCLUSIVE,
        ReverseOffload => CLAUSE_KIND_REVERSE_OFFLOAD,
        UnifiedAddress => CLAUSE_KIND_UNIFIED_ADDRESS,
        UnifiedSharedMemory => CLAUSE_KIND_UNIFIED_SHARED_MEMORY,
        DynamicAllocators => CLAUSE_KIND_DYNAMIC_ALLOCATORS,
        SelfMaps => CLAUSE_KIND_SELF,
        ExtImplementationDefinedRequirement => CLAUSE_KIND_EXT_IMPLEMENTATION_DEFINED_REQUIREMENT,
        When => CLAUSE_KIND_WHEN,
        Match => CLAUSE_KIND_MATCH,
        Link => CLAUSE_KIND_LINK,
        Taskgroup => CLAUSE_KIND_TASKGROUP,
        Initializer => CLAUSE_KIND_INITIALIZER,
        Final => CLAUSE_KIND_FINAL,
        Parallel => CLAUSE_KIND_PARALLEL,
        Sections => CLAUSE_KIND_SECTIONS,
        For => CLAUSE_KIND_FOR,
        Do => CLAUSE_KIND_DO,
        Destroy => CLAUSE_KIND_DESTROY,
        Threads => CLAUSE_KIND_THREADS,
        Simd => CLAUSE_KIND_SIMD,
        Compare => CLAUSE_KIND_COMPARE,
        CompareCapture => CLAUSE_KIND_COMPARE_CAPTURE,
        Hint => CLAUSE_KIND_HINT,
        Full => CLAUSE_KIND_FULL,
        Partial => CLAUSE_KIND_PARTIAL,
        Detach => CLAUSE_KIND_DETACH,
        Fail => CLAUSE_KIND_FAIL,
        Weak => CLAUSE_KIND_WEAK,
        At => CLAUSE_KIND_AT,
        Severity => CLAUSE_KIND_SEVERITY,
        Message => CLAUSE_KIND_MESSAGE,
        Doacross => CLAUSE_KIND_DOACROSS,
        Absent => CLAUSE_KIND_ABSENT,
        Contains => CLAUSE_KIND_CONTAINS,
        Holds => CLAUSE_KIND_HOLDS,
        GraphId => CLAUSE_KIND_GRAPH_ID,
        GraphReset => CLAUSE_KIND_GRAPH_RESET,
        Transparent => CLAUSE_KIND_TRANSPARENT,
        Replayable => CLAUSE_KIND_REPLAYABLE,
        Threadset => CLAUSE_KIND_THREADSET,
        Indirect => CLAUSE_KIND_INDIRECT,
        Local => CLAUSE_KIND_LOCAL,
        Init => CLAUSE_KIND_INIT,
        InitComplete => CLAUSE_KIND_INIT_COMPLETE,
        Safesync => CLAUSE_KIND_SAFESYNC,
        DeviceSafesync => CLAUSE_KIND_DEVICE_SAFESYNC,
        Memscope => CLAUSE_KIND_MEMSCOPE,
        Looprange => CLAUSE_KIND_LOOPRANGE,
        Permutation => CLAUSE_KIND_PERMUTATION,
        Counts => CLAUSE_KIND_COUNTS,
        Induction => CLAUSE_KIND_INDUCTION,
        Inductor => CLAUSE_KIND_INDUCTOR,
        Collector => CLAUSE_KIND_COLLECTOR,
        Combiner => CLAUSE_KIND_COMBINER,
        AdjustArgs => CLAUSE_KIND_ADJUST_ARGS,
        AppendArgs => CLAUSE_KIND_APPEND_ARGS,
        Apply => CLAUSE_KIND_APPLY,
        NoOpenmp => CLAUSE_KIND_NOOPENMP,
        NoOpenmpConstructs => CLAUSE_KIND_NOOPENMP_CONSTRUCTS,
        NoOpenmpRoutines => CLAUSE_KIND_NOOPENMP_ROUTINES,
        NoParallelism => CLAUSE_KIND_NOPARALLELISM,
        Nocontext => CLAUSE_KIND_NOCONTEXT,
        Novariants => CLAUSE_KIND_NOVARIANTS,
        Enter => CLAUSE_KIND_ENTER,
        Use => CLAUSE_KIND_USE,
        _ => UNKNOWN_KIND,
    }
}

/// Convert directive name to kind enum code.
///
/// Maps directive names (parallel, for, task, etc.) to integer codes
/// so C code can use switch statements instead of string comparisons.
///
/// ## Directive Codes:
/// - 0 = parallel     - 5 = critical
/// - 1 = for          - 6 = atomic
/// - 2 = sections     - 7 = barrier
/// - 3 = single       - 8 = master
/// - 4 = task         - 9 = teams
/// - 10 = target      - 11 = distribute
/// - -1 = NULL/unknown
// Map a `DirectiveName` enum to the integer codes used by the C API.
// This function is the single source of truth for directive -> integer mapping
// at runtime. It uses the typed `DirectiveName` enum (no string matching).
//
// Unknown or unhandled directives are treated as an error and return `-1` so
// callers can detect a missing mapping and the maintainers are alerted to add
// the correct mapping.
fn directive_name_enum_to_kind(name: DirectiveName) -> i32 {
    use DirectiveName::*;

    // Map to ompparser's OpenMPDirectiveKind enum values
    // These must match the sequential order in OpenMPKinds.h exactly
    // Generated from compat/ompparser/ompparser/src/OpenMPKinds.h
    let result = match name {
        Parallel => 0,                    // OMPD_parallel
        For => 1,                         // OMPD_for
        Do => 2,                          // OMPD_do
        Simd => 3,                        // OMPD_simd
        ForSimd => 4,                     // OMPD_for_simd
        DoSimd => 5,                      // OMPD_do_simd
        ParallelForSimd => 6,             // OMPD_parallel_for_simd
        ParallelDoSimd => 7,              // OMPD_parallel_do_simd
        DeclareSimd => 8,                 // OMPD_declare_simd
        Distribute => 9,                  // OMPD_distribute
        DistributeSimd => 10,             // OMPD_distribute_simd
        DistributeParallelFor => 11,      // OMPD_distribute_parallel_for
        DistributeParallelDo => 12,       // OMPD_distribute_parallel_do
        DistributeParallelForSimd => 13,  // OMPD_distribute_parallel_for_simd
        DistributeParallelDoSimd => 14,   // OMPD_distribute_parallel_do_simd
        Loop => 15,                       // OMPD_loop
        Scan => 16,                       // OMPD_scan
        Sections => 17,                   // OMPD_sections
        Section => 18,                    // OMPD_section
        Single => 19,                     // OMPD_single
        Workshare => 20,                  // OMPD_workshare
        Cancel => 21,                     // OMPD_cancel
        CancellationPoint => 22,          // OMPD_cancellation_point
        Allocate => 23,                   // OMPD_allocate
        Threadprivate => 24,              // OMPD_threadprivate
        DeclareReduction => 25,           // OMPD_declare_reduction
        DeclareMapper => 26,              // OMPD_declare_mapper
        ParallelFor => 27,                // OMPD_parallel_for
        ParallelDo => 28,                 // OMPD_parallel_do
        ParallelLoop => 29,               // OMPD_parallel_loop
        ParallelSections => 30,           // OMPD_parallel_sections
        ParallelSingle => 31,             // OMPD_parallel_single
        ParallelWorkshare => 32,          // OMPD_parallel_workshare
        ParallelMaster => 33,             // OMPD_parallel_master
        MasterTaskloop => 34,             // OMPD_master_taskloop
        MasterTaskloopSimd => 35,         // OMPD_master_taskloop_simd
        ParallelMasterTaskloop => 36,     // OMPD_parallel_master_taskloop
        ParallelMasterTaskloopSimd => 37, // OMPD_parallel_master_taskloop_simd
        Teams => 38,                      // OMPD_teams
        Metadirective => 39,              // OMPD_metadirective
        DeclareVariant => 40,             // OMPD_declare_variant
        BeginDeclareVariant => 41,        // OMPD_begin_declare_variant
        EndDeclareVariant => 42,          // OMPD_end_declare_variant
        Task => 43,                       // OMPD_task
        Taskloop => 44,                   // OMPD_taskloop
        TaskloopSimd => 45,               // OMPD_taskloop_simd
        Taskyield => 46,                  // OMPD_taskyield
        Requires => 47,                   // OMPD_requires
        TargetData => 48,                 // OMPD_target_data
        TargetDataComposite => 49,        // OMPD_target_data_composite
        TargetEnterData => 50,            // OMPD_target_enter_data
        TargetUpdate => 51,               // OMPD_target_update
        TargetExitData => 52,             // OMPD_target_exit_data
        Target => 53,                     // OMPD_target
        DeclareTarget => 54,              // OMPD_declare_target
        BeginDeclareTarget => 55,         // OMPD_begin_declare_target
        EndDeclareTarget => 56,           // OMPD_end_declare_target
        Master => 57,                     // OMPD_master
        End => 58,                        // OMPD_end (bare "end" only)
        // End directives - each gets unique constant for enum-based compat layer
        // These map to OMPD_end in ompparser but need unique ROUP constants
        EndParallel => 131,
        EndDo => 132,
        EndSimd => 133,
        EndSections => 134,
        EndSingle => 135,
        EndWorkshare => 136,
        EndOrdered => 137,
        EndLoop => 138,
        EndDistribute => 139,
        EndTeams => 140,
        EndTaskloop => 141,
        EndTask => 142,
        EndTaskgroup => 143,
        EndMaster => 144,
        EndCritical => 145,
        EndAtomic => 146,
        EndParallelDo => 147,
        EndParallelFor => 148,
        EndParallelSections => 149,
        EndParallelWorkshare => 150,
        EndParallelMaster => 151,
        EndDoSimd => 152,
        EndForSimd => 153,
        EndParallelDoSimd => 154,
        EndParallelForSimd => 155,
        EndDistributeSimd => 156,
        EndDistributeParallelDo => 157,
        EndDistributeParallelFor => 158,
        EndDistributeParallelDoSimd => 159,
        EndDistributeParallelForSimd => 160,
        EndTargetParallel => 161,
        EndTargetParallelDo => 162,
        EndTargetParallelFor => 163,
        EndTargetParallelDoSimd => 164,
        EndTargetParallelForSimd => 165,
        EndTargetSimd => 166,
        EndTargetTeams => 167,
        EndTargetTeamsDistribute => 168,
        EndTargetTeamsDistributeParallelDo => 169,
        EndTargetTeamsDistributeParallelFor => 170,
        EndTargetTeamsDistributeParallelDoSimd => 171,
        EndTargetTeamsDistributeParallelForSimd => 172,
        EndTargetTeamsDistributeSimd => 173,
        EndTargetTeamsLoop => 174,
        EndTeamsDistribute => 175,
        EndTeamsDistributeParallelDo => 176,
        EndTeamsDistributeParallelFor => 177,
        EndTeamsDistributeParallelDoSimd => 178,
        EndTeamsDistributeParallelForSimd => 179,
        EndTeamsDistributeSimd => 180,
        EndTeamsLoop => 181,
        EndTaskloopSimd => 182,
        EndMasterTaskloop => 183,
        EndMasterTaskloopSimd => 184,
        EndParallelMasterTaskloop => 185,
        EndParallelMasterTaskloopSimd => 186,
        EndTargetParallelLoop => 187,
        EndParallelLoop => 188,
        EndTargetLoop => 189,
        EndSection => 190,
        EndUnroll => 196,
        EndTile => 197,
        EndMasked => 198,
        EndMaskedTaskloop => 199,
        EndMaskedTaskloopSimd => 200,
        EndParallelMasked => 201,
        EndParallelMaskedTaskloop => 202,
        EndParallelMaskedTaskloopSimd => 203,
        Barrier => 59,   // OMPD_barrier
        Taskwait => 60,  // OMPD_taskwait
        Unroll => 61,    // OMPD_unroll
        Tile => 62,      // OMPD_tile
        Taskgroup => 63, // OMPD_taskgroup
        Flush => 64,     // OMPD_flush
        Atomic => 65,    // OMPD_atomic
        // All atomic variants map to OMPD_atomic (read/write/update/capture are clauses)
        AtomicRead => 65,
        AtomicWrite => 65,
        AtomicUpdate => 65,
        AtomicCapture => 65,
        AtomicCompareCapture => 65,
        Critical => 66,                               // OMPD_critical
        Depobj => 67,                                 // OMPD_depobj
        Ordered => 68,                                // OMPD_ordered
        TeamsDistribute => 69,                        // OMPD_teams_distribute
        TeamsDistributeSimd => 70,                    // OMPD_teams_distribute_simd
        TeamsDistributeParallelFor => 71,             // OMPD_teams_distribute_parallel_for
        TeamsDistributeParallelForSimd => 72,         // OMPD_teams_distribute_parallel_for_simd
        TeamsLoop => 73,                              // OMPD_teams_loop
        TargetParallel => 74,                         // OMPD_target_parallel
        TargetParallelFor => 75,                      // OMPD_target_parallel_for
        TargetParallelForSimd => 76,                  // OMPD_target_parallel_for_simd
        TargetParallelLoop => 77,                     // OMPD_target_parallel_loop
        TargetSimd => 78,                             // OMPD_target_simd
        TargetTeams => 79,                            // OMPD_target_teams
        TargetTeamsDistribute => 80,                  // OMPD_target_teams_distribute
        TargetTeamsDistributeSimd => 81,              // OMPD_target_teams_distribute_simd
        TargetTeamsLoop => 82,                        // OMPD_target_teams_loop
        TargetTeamsDistributeParallelFor => 83,       // OMPD_target_teams_distribute_parallel_for
        TargetTeamsDistributeParallelForSimd => 84, // OMPD_target_teams_distribute_parallel_for_simd
        TeamsDistributeParallelDo => 85,            // OMPD_teams_distribute_parallel_do
        TeamsDistributeParallelDoSimd => 86,        // OMPD_teams_distribute_parallel_do_simd
        TargetParallelDo => 87,                     // OMPD_target_parallel_do
        TargetParallelDoSimd => 88,                 // OMPD_target_parallel_do_simd
        TargetTeamsDistributeParallelDo => 89,      // OMPD_target_teams_distribute_parallel_do
        TargetTeamsDistributeParallelDoSimd => 90,  // OMPD_target_teams_distribute_parallel_do_simd
        Error => 91,                                // OMPD_error
        Nothing => 92,                              // OMPD_nothing
        Masked => 93,                               // OMPD_masked
        Scope => 94,                                // OMPD_scope
        EndScope => 94,                             // Treat end scope as scope
        MaskedTaskloop => 95,                       // OMPD_masked_taskloop
        MaskedTaskloopSimd => 96,                   // OMPD_masked_taskloop_simd
        ParallelMasked => 97,                       // OMPD_parallel_masked
        ParallelMaskedTaskloop => 98,               // OMPD_parallel_masked_taskloop
        ParallelMaskedTaskloopSimd => 99,           // OMPD_parallel_masked_taskloop_simd
        Interop => 100,                             // OMPD_interop
        Assume => 101,                              // OMPD_assume
        EndAssume => 102,                           // OMPD_end_assume
        Assumes => 103,                             // OMPD_assumes
        BeginAssumes => 104,                        // OMPD_begin_assumes
        EndAssumes => 105,                          // OMPD_end_assumes
        Allocators => 106,                          // OMPD_allocators
        Taskgraph => 107,                           // OMPD_taskgraph
        TaskIteration => 108,                       // OMPD_task_iteration
        Dispatch => 109,                            // OMPD_dispatch
        Groupprivate => 110,                        // OMPD_groupprivate
        Workdistribute => 111,                      // OMPD_workdistribute
        Fuse => 112,                                // OMPD_fuse
        Interchange => 113,                         // OMPD_interchange
        Reverse => 114,                             // OMPD_reverse
        Split => 115,                               // OMPD_split
        Stripe => 116,                              // OMPD_stripe
        DeclareInduction => 117,                    // OMPD_declare_induction
        BeginMetadirective => 118,                  // OMPD_begin_metadirective
        ParallelLoopSimd => 119,                    // OMPD_parallel_loop_simd
        TeamsLoopSimd => 120,                       // OMPD_teams_loop_simd
        TargetLoop => 121,                          // OMPD_target_loop
        TargetLoopSimd => 122,                      // OMPD_target_loop_simd
        TargetParallelLoopSimd => 123,              // OMPD_target_parallel_loop_simd
        TargetTeamsLoopSimd => 124,                 // OMPD_target_teams_loop_simd
        DistributeParallelLoop => 125,              // OMPD_distribute_parallel_loop
        DistributeParallelLoopSimd => 126,          // OMPD_distribute_parallel_loop_simd
        TeamsDistributeParallelLoop => 127,         // OMPD_teams_distribute_parallel_loop
        TeamsDistributeParallelLoopSimd => 128,     // OMPD_teams_distribute_parallel_loop_simd
        TargetTeamsDistributeParallelLoop => 129,   // OMPD_target_teams_distribute_parallel_loop
        TargetTeamsDistributeParallelLoopSimd => 130, // OMPD_target_teams_distribute_parallel_loop_simd

        // OpenACC-specific directives: these are not part of the OpenMP C API
        Data | EnterData | ExitData | HostData | Kernels | KernelsLoop | Update | Serial
        | SerialLoop | Routine | Set | Init | Shutdown | Cache | Wait | Declare => -1,

        // EndTarget and related end directives - unique constants
        EndTarget => 191,          // Maps to OMPD_end in ompparser
        EndTargetData => 192,      // Maps to OMPD_end in ompparser
        EndTargetEnterData => 193, // Maps to OMPD_end in ompparser
        EndTargetExitData => 194,  // Maps to OMPD_end in ompparser
        EndTargetUpdate => 195,    // Maps to OMPD_end in ompparser

        // NothingKnown is a placeholder
        NothingKnown => -1,

        // Unknown / unhandled directive — treat as error so maintainers notice
        Other(s) => {
            eprintln!(
                "[c_api] unknown directive mapping requested: {}",
                s.as_ref()
            );
            -1
        }
    };
    result
}

/// Free clause-specific data.
/// Free clause-specific data (internal helper).
///
/// Handles cleanup for union types where different variants need different
/// cleanup strategies. Currently only variable lists need explicit freeing.
///
/// ## Design Note:
/// Most clause data (schedule, reduction, default) are Copy types that don't
/// need cleanup. Only heap-allocated variable lists need explicit freeing.
///
/// ## IMPORTANT: ClauseData is a C union!
/// The ClauseData union has 4 fields, but only ONE is active per clause:
///   - `variables: *mut OmpStringList` - used by kinds 2-5 (private/shared/firstprivate/lastprivate)
///   - `reduction: ReductionData` - used by kind 6 (reduction operator only, NO variables pointer)
///   - `schedule: ScheduleData` - used by kind 7 (schedule policy only)
///   - `default: i32` - used by other kinds
///   - `defaultmap: DefaultmapData` - used by defaultmap clauses
///   - `uses_allocators: UsesAllocatorsData*` - used by uses_allocators
///   - `requires: RequiresData*` - used by requires
///
/// Reduction clauses (kind 6) do NOT use the `variables` field. Trying to free
/// clause.data.variables on a reduction clause would read garbage memory from the
/// wrong union variant (the bytes of ReductionData::operator interpreted as a pointer).
fn free_clause_data(clause: &OmpClause) {
    unsafe {
        // Free arguments string if present
        if !clause.arguments.is_null() {
            drop(CString::from_raw(clause.arguments as *mut c_char));
        }

        // Free variable lists if present
        // Clause kinds with variable lists (see convert_clause):
        //   3 = private, 4 = firstprivate, 5 = shared (lastprivate handled separately)
        // Other kinds use different union fields:
        //   2 = default (uses .default field, NOT .variables)
        //   8 = reduction (uses .reduction field, NOT .variables)
        //   21 = schedule (uses .schedule field, NOT .variables)
        //   68 = defaultmap (.defaultmap field)
        //   71 = uses_allocators (uses_allocators pointer)
        //   60 = device (modifier in .default, expression in arguments)
        //   89 = depobj_update (code in .default)
        if is_reduction_clause_kind(clause.kind) {
            if let Some(data) = get_reduction_data(clause) {
                if !data.user_identifier.is_null() {
                    drop(CString::from_raw(data.user_identifier as *mut c_char));
                }
                if !data.modifiers_text.is_null() {
                    drop(CString::from_raw(data.modifiers_text as *mut c_char));
                }
                if !data.variables.is_null() {
                    roup_string_list_free(data.variables);
                }
            }
        } else if is_lastprivate_clause_kind(clause.kind) {
            if let Some(data) = get_lastprivate_data(clause) {
                if !data.variables.is_null() {
                    roup_string_list_free(data.variables);
                }
            }
        } else if is_map_clause_kind(clause.kind) {
            let ptr = clause.data.map;
            if !ptr.is_null() {
                let boxed = Box::from_raw(ptr);
                if !boxed.mapper.is_null() {
                    drop(CString::from_raw(boxed.mapper as *mut c_char));
                }
                if !boxed.variables.is_null() {
                    roup_string_list_free(boxed.variables);
                }
                for it in boxed.iterators {
                    if !it.type_name.is_null() {
                        drop(CString::from_raw(it.type_name as *mut c_char));
                    }
                    if !it.name.is_null() {
                        drop(CString::from_raw(it.name as *mut c_char));
                    }
                    if !it.start.is_null() {
                        drop(CString::from_raw(it.start as *mut c_char));
                    }
                    if !it.end.is_null() {
                        drop(CString::from_raw(it.end as *mut c_char));
                    }
                    if !it.step.is_null() {
                        drop(CString::from_raw(it.step as *mut c_char));
                    }
                }
            }
        } else if is_linear_clause_kind(clause.kind) {
            let ptr = clause.data.linear;
            if !ptr.is_null() {
                let boxed = Box::from_raw(ptr);
                if !boxed.step.is_null() {
                    drop(CString::from_raw(boxed.step as *mut c_char));
                }
                if !boxed.variables.is_null() {
                    roup_string_list_free(boxed.variables);
                }
            }
        } else if is_depend_clause_kind(clause.kind) {
            let ptr = clause.data.depend;
            if !ptr.is_null() {
                let boxed = Box::from_raw(ptr);
                if !boxed.variables.is_null() {
                    roup_string_list_free(boxed.variables);
                }
                for it in boxed.iterators {
                    if !it.type_name.is_null() {
                        drop(CString::from_raw(it.type_name as *mut c_char));
                    }
                    if !it.name.is_null() {
                        drop(CString::from_raw(it.name as *mut c_char));
                    }
                    if !it.start.is_null() {
                        drop(CString::from_raw(it.start as *mut c_char));
                    }
                    if !it.end.is_null() {
                        drop(CString::from_raw(it.end as *mut c_char));
                    }
                    if !it.step.is_null() {
                        drop(CString::from_raw(it.step as *mut c_char));
                    }
                }
            }
        } else if is_allocate_clause_kind(clause.kind) {
            let ptr = clause.data.allocate;
            if !ptr.is_null() {
                let boxed = Box::from_raw(ptr);
                if !boxed.allocator.is_null() {
                    drop(CString::from_raw(boxed.allocator as *mut c_char));
                }
                if !boxed.variables.is_null() {
                    roup_string_list_free(boxed.variables);
                }
            }
        } else if is_affinity_clause_kind(clause.kind) {
            let ptr = clause.data.affinity;
            if !ptr.is_null() {
                let boxed = Box::from_raw(ptr);
                if !boxed.variables.is_null() {
                    roup_string_list_free(boxed.variables);
                }
                for it in boxed.iterators {
                    if !it.type_name.is_null() {
                        drop(CString::from_raw(it.type_name as *mut c_char));
                    }
                    if !it.name.is_null() {
                        drop(CString::from_raw(it.name as *mut c_char));
                    }
                    if !it.start.is_null() {
                        drop(CString::from_raw(it.start as *mut c_char));
                    }
                    if !it.end.is_null() {
                        drop(CString::from_raw(it.end as *mut c_char));
                    }
                    if !it.step.is_null() {
                        drop(CString::from_raw(it.step as *mut c_char));
                    }
                }
            }
        } else if clause.kind == CLAUSE_KIND_ALIGNED {
            let ptr = clause.data.aligned;
            if !ptr.is_null() {
                let boxed = Box::from_raw(ptr);
                if !boxed.alignment.is_null() {
                    drop(CString::from_raw(boxed.alignment as *mut c_char));
                }
                if !boxed.variables.is_null() {
                    roup_string_list_free(boxed.variables);
                }
            }
        } else if clause.kind == CLAUSE_KIND_SCHEDULE {
            let sched = &*clause.data.schedule;
            if !sched.chunk.is_null() {
                drop(CString::from_raw(sched.chunk as *mut c_char));
            }
        } else if clause.kind == CLAUSE_KIND_DIST_SCHEDULE {
            let dist = &*clause.data.dist_schedule;
            if !dist.chunk.is_null() {
                drop(CString::from_raw(dist.chunk as *mut c_char));
            }
        } else if clause_kind_uses_variable_list(clause.kind) {
            let vars_ptr = clause.data.variables;
            if !vars_ptr.is_null() {
                roup_string_list_free(vars_ptr);
            }
        } else if is_requires_clause_kind(clause.kind) {
            let data_ptr = clause.data.requires;
            if !data_ptr.is_null() {
                let boxed = Box::from_raw(data_ptr);
                for entry in &boxed.entries {
                    if !entry.name.is_null() {
                        drop(CString::from_raw(entry.name as *mut c_char));
                    }
                }
            }
        } else if is_uses_allocators_clause_kind(clause.kind) {
            let data_ptr = clause.data.uses_allocators;
            if !data_ptr.is_null() {
                let boxed = Box::from_raw(data_ptr);
                for entry in &boxed.entries {
                    if !entry.user_name.is_null() {
                        drop(CString::from_raw(entry.user_name as *mut c_char));
                    }
                    if !entry.traits.is_null() {
                        drop(CString::from_raw(entry.traits as *mut c_char));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;
    use std::ffi::CStr;
    use std::ptr;

    #[test]
    fn roup_parse_accepts_hint_clause() {
        let input =
            std::ffi::CString::new("#pragma omp critical(test1) hint(test2)").expect("cstring");

        let directive = roup_parse(input.as_ptr());
        assert!(
            !directive.is_null(),
            "roup_parse should produce a directive for critical+hint"
        );

        roup_directive_free(directive);
    }

    #[test]
    fn roup_parse_reports_hint_clause_kind() {
        let input =
            std::ffi::CString::new("#pragma omp critical(test1) hint(test2)").expect("cstring");

        let directive = roup_parse(input.as_ptr());
        assert!(!directive.is_null());

        let count = roup_directive_clause_count(directive);
        assert_eq!(count, 1, "expected single hint clause");

        let iter = roup_directive_clauses_iter(directive);
        assert!(!iter.is_null(), "iterator should be created");
        let mut clause_ptr: *const OmpClause = ptr::null();
        let advanced = roup_clause_iterator_next(iter, &mut clause_ptr);
        assert_eq!(advanced, 1, "iterator should yield first clause");
        assert!(!clause_ptr.is_null(), "clause pointer should not be NULL");

        let kind = roup_clause_kind(clause_ptr);
        assert_eq!(kind, CLAUSE_KIND_HINT);

        roup_clause_iterator_free(iter);
        roup_directive_free(directive);
    }

    #[test]
    fn uniform_clause_preserves_arguments_and_variables() {
        let input =
            std::ffi::CString::new("#pragma omp declare simd uniform(*a,&b)").expect("cstring");
        let directive = roup_parse(input.as_ptr());
        assert!(!directive.is_null());

        let iter = roup_directive_clauses_iter(directive);
        assert!(!iter.is_null(), "iterator should be created");

        let mut clause_ptr: *const OmpClause = ptr::null();
        let mut found = false;
        while roup_clause_iterator_next(iter, &mut clause_ptr) == 1 {
            let kind = roup_clause_kind(clause_ptr);
            if kind == CLAUSE_KIND_UNIFORM {
                found = true;
                let args_ptr = roup_clause_arguments(clause_ptr);
                assert!(
                    !args_ptr.is_null(),
                    "uniform clause should carry argument list"
                );
                let args = unsafe { CStr::from_ptr(args_ptr) }.to_str().expect("utf8");
                assert_eq!(args, "*a, &b");

                let vars = roup_clause_variables(clause_ptr);
                assert!(!vars.is_null(), "variables list should exist");
                let len = roup_string_list_len(vars);
                assert_eq!(len, 2);
                roup_string_list_free(vars);
            }
        }

        assert!(found, "uniform clause should be present");

        roup_clause_iterator_free(iter);
        roup_directive_free(directive);
    }

    #[test]
    fn fortran_linear_clause_uses_fortran_variable_formatting() {
        let input = std::ffi::CString::new("!$omp do linear(val(a,b,c):2)").expect("cstring");
        let directive = roup_parse_with_language(input.as_ptr(), ROUP_LANG_FORTRAN_FREE);
        assert!(!directive.is_null());

        let iter = roup_directive_clauses_iter(directive);
        assert!(!iter.is_null(), "iterator should be created");

        let mut clause_ptr: *const OmpClause = ptr::null();
        let mut found = false;
        while roup_clause_iterator_next(iter, &mut clause_ptr) == 1 {
            if clause_ptr.is_null() {
                continue;
            }
            let kind = roup_clause_kind(clause_ptr);
            if kind == CLAUSE_KIND_LINEAR {
                found = true;
                let args_ptr = roup_clause_arguments(clause_ptr);
                assert!(!args_ptr.is_null(), "linear clause should expose arguments");
                let args = unsafe { CStr::from_ptr(args_ptr) }
                    .to_str()
                    .expect("utf8 args");
                assert_eq!(args, "val( a, b, c): 2");

                let vars = roup_clause_variables(clause_ptr);
                assert!(!vars.is_null(), "linear clause should expose variables");
                let len = roup_string_list_len(vars);
                assert_eq!(len, 3);
                let first = roup_string_list_get(vars, 0);
                assert!(!first.is_null(), "first variable should exist");
                let text = unsafe { CStr::from_ptr(first) }
                    .to_str()
                    .expect("utf8 variable text");
                assert_eq!(text, "a");
                roup_string_list_free(vars);
            }
        }

        assert!(found, "expected linear clause");

        roup_clause_iterator_free(iter);
        roup_directive_free(directive);
    }

    #[test]
    fn fortran_end_do_nowait_includes_clause() {
        let input = std::ffi::CString::new("!$omp end do nowait").expect("cstring");
        let directive = roup_parse_with_language(input.as_ptr(), ROUP_LANG_FORTRAN_FREE);
        assert!(!directive.is_null());

        let count = roup_directive_clause_count(directive);
        assert!(count >= 1, "expected at least one clause on end do nowait");

        let iter = roup_directive_clauses_iter(directive);
        assert!(!iter.is_null());

        let mut clause_ptr: *const OmpClause = ptr::null();
        let mut found = false;
        while roup_clause_iterator_next(iter, &mut clause_ptr) == 1 {
            if clause_ptr.is_null() {
                continue;
            }
            let kind = roup_clause_kind(clause_ptr);
            if kind == CLAUSE_KIND_NOWAIT {
                found = true;
                break;
            }
        }

        assert!(found, "nowait clause should be present");

        roup_clause_iterator_free(iter);
        roup_directive_free(directive);
    }

    #[test]
    fn unmapped_directive_returns_minus_one() {
        // Construct an Other variant and ensure the enum->int helper returns -1
        let other = DirectiveName::Other(Cow::Owned("__not_a_real_directive__".to_string()));
        let v = directive_name_enum_to_kind(other);
        assert_eq!(v, -1);
    }
}

// ============================================================================
// END OF C API IMPLEMENTATION
// ============================================================================
//
// Summary: This C API provides a minimal unsafe FFI layer over safe Rust code.
//
// Total Safety Profile:
// - 18 unsafe blocks (~60 lines = 0.9% of file)
// - All unsafe confined to FFI boundary
// - Core parsing logic remains 100% safe Rust
//
// Design Principles Applied:
// 1. ✅ Minimal unsafe: Only at C boundary, not in business logic
// 2. ✅ Direct pointers: Simple, predictable, C-friendly
// 3. ✅ Caller responsibility: C manages memory lifetime explicitly
// 4. ✅ Fail-fast: NULL returns on any error
// 5. ✅ No hidden state: Stateless API, no global variables
//
// Why This Approach Works:
// - C programmers understand manual memory management
// - Performance: No overhead from handle tables or reference counting
// - Simplicity: Direct mapping between Rust types and C expectations
// - Safety: Core parser is safe Rust, only FFI layer has unsafe
//
// Future Considerations for v1.0.0:
// - Thread safety annotations (Rust types are Send/Sync, C must ensure too)
// - Comprehensive error reporting (currently just NULL on error)
// - Semantic validation beyond parsing (requires deeper OpenMP knowledge)

// ============================================================================
// OpenACC C API is implemented in src/c_api/openacc.rs
