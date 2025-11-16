use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

use bitflags::bitflags;

use crate::ast::{
    AccClause as AstAccClause, AccClauseKind as AstAccClauseKind,
    AccClausePayload as AstAccClausePayload, AccCopyKind, AccCopyModifier, AccCreateModifier,
    AccDefaultKind, AccDirective as AstAccDirective, AccDirectiveParameter, AccGangClause,
    ClauseNormalizationMode, DirectiveBody,
};
use crate::ir::ParserConfig;
use crate::lexer::Language;
use crate::parser::{
    ast_builder::{build_roup_directive, AstBuildError},
    openacc as openacc_parser, CacheDirectiveData as ParserCacheDirectiveData, Directive,
    WaitDirectiveData as ParserWaitDirectiveData,
};

use super::{ROUP_LANG_C, ROUP_LANG_FORTRAN_FIXED, ROUP_LANG_FORTRAN_FREE};

// Use the parser's canonical directive lookup and the shared enum->int helper
use crate::parser::directive_kind::lookup_directive_name;

bitflags! {
    struct AccClauseFlags: u32 {
        const WAIT_HAS_QUEUES = 0b0001;
        const WAIT_HAS_DEVNUM = 0b0010;
    }
}

pub struct AccDirective {
    name: CString,
    language: i32,
    clauses: Vec<AccClause>,
    cache_data: Option<CacheData>,
    wait_data: Option<WaitDirectiveData>,
    routine_name: Option<CString>,
    end_paired_kind: Option<i32>,
}

#[derive(Default)]
struct CacheData {
    modifier: i32,
    expressions: Vec<CString>,
}

#[derive(Default)]
struct WaitDirectiveData {
    devnum: Option<CString>,
    queues: bool,
    expressions: Vec<CString>,
}

pub struct AccClause {
    kind: i32,
    modifier: i32,
    original_keyword: Option<CString>,
    expressions: Vec<CString>,
    wait_devnum: Option<CString>,
    flags: AccClauseFlags,
}

pub struct AccClauseIterator {
    clauses: Vec<*const AccClause>,
    index: usize,
}

const ACC_CACHE_MODIFIER_UNSPECIFIED: i32 = 0;
const ACC_CACHE_MODIFIER_READONLY: i32 = 1;

const ACC_COPYIN_MODIFIER_UNSPECIFIED: i32 = 0;
const ACC_COPYIN_MODIFIER_READONLY: i32 = 1;

const ACC_COPYOUT_MODIFIER_UNSPECIFIED: i32 = 0;
const ACC_COPYOUT_MODIFIER_ZERO: i32 = 1;

const ACC_CREATE_MODIFIER_UNSPECIFIED: i32 = 0;
const ACC_CREATE_MODIFIER_ZERO: i32 = 1;

const ACC_DEFAULT_KIND_UNSPECIFIED: i32 = 0;
const ACC_DEFAULT_KIND_NONE: i32 = 1;
const ACC_DEFAULT_KIND_PRESENT: i32 = 2;

const ACC_VECTOR_MODIFIER_UNSPECIFIED: i32 = 0;
const ACC_VECTOR_MODIFIER_LENGTH: i32 = 1;

const ACC_WORKER_MODIFIER_UNSPECIFIED: i32 = 0;
const ACC_WORKER_MODIFIER_NUM: i32 = 1;

const ACC_REDUCTION_OP_UNSPECIFIED: i32 = 0;
const ACC_REDUCTION_OP_READONLY: i32 = 1;
const ACC_REDUCTION_OP_ADD: i32 = 2;
const ACC_REDUCTION_OP_SUB: i32 = 3;
const ACC_REDUCTION_OP_MUL: i32 = 4;
const ACC_REDUCTION_OP_MAX: i32 = 5;
const ACC_REDUCTION_OP_MIN: i32 = 6;
const ACC_REDUCTION_OP_BITAND: i32 = 7;
const ACC_REDUCTION_OP_BITOR: i32 = 8;
const ACC_REDUCTION_OP_BITXOR: i32 = 9;
const ACC_REDUCTION_OP_LOGAND: i32 = 10;
const ACC_REDUCTION_OP_LOGOR: i32 = 11;
const ACC_REDUCTION_OP_FORT_AND: i32 = 12;
const ACC_REDUCTION_OP_FORT_OR: i32 = 13;
const ACC_REDUCTION_OP_FORT_EQV: i32 = 14;
const ACC_REDUCTION_OP_FORT_NEQV: i32 = 15;
const ACC_REDUCTION_OP_FORT_IAND: i32 = 16;
const ACC_REDUCTION_OP_FORT_IOR: i32 = 17;
const ACC_REDUCTION_OP_FORT_IEOR: i32 = 18;

#[no_mangle]
pub extern "C" fn acc_parse(input: *const c_char) -> *mut AccDirective {
    parse_openacc_internal(input, Language::C)
}

#[no_mangle]
pub extern "C" fn acc_parse_with_language(
    input: *const c_char,
    language: i32,
) -> *mut AccDirective {
    let lang = match language {
        ROUP_LANG_C => Language::C,
        ROUP_LANG_FORTRAN_FREE => Language::FortranFree,
        ROUP_LANG_FORTRAN_FIXED => Language::FortranFixed,
        _ => return ptr::null_mut(),
    };

    parse_openacc_internal(input, lang)
}

fn parse_openacc_internal(input: *const c_char, language: Language) -> *mut AccDirective {
    if input.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let c_str = CStr::from_ptr(input);
        let rust_str = match c_str.to_str() {
            Ok(value) => value,
            Err(_) => return ptr::null_mut(),
        };

        let parser = openacc_parser::parser().with_language(language);
        let directive = match parser.parse(rust_str) {
            Ok((_, dir)) => dir,
            Err(_) => return ptr::null_mut(),
        };

        let ast = match build_openacc_ast(&directive, language) {
            Ok(ast) => ast,
            Err(_) => return ptr::null_mut(),
        };

        let converted = build_acc_directive(directive, &ast, language);
        Box::into_raw(Box::new(converted))
    }
}

fn build_openacc_ast(
    directive: &Directive<'_>,
    language: Language,
) -> Result<AstAccDirective, AstBuildError> {
    let ir_language = match language {
        Language::C => crate::ir::Language::C,
        Language::FortranFree | Language::FortranFixed => crate::ir::Language::Fortran,
    };

    let parser_config = ParserConfig::default();
    let roup = build_roup_directive(
        directive,
        crate::parser::Dialect::OpenAcc,
        ClauseNormalizationMode::ParserParity,
        &parser_config,
        ir_language,
    )?;

    match roup.body {
        DirectiveBody::OpenAcc(acc) => Ok(acc),
        _ => Err(AstBuildError::UnsupportedDirective(
            "expected an OpenACC directive body".to_string(),
        )),
    }
}

fn build_acc_directive(
    parsed: Directive<'_>,
    ast: &AstAccDirective,
    language: Language,
) -> AccDirective {
    let clauses = build_clauses_from_ast(ast);
    let ast_parameter = ast.parameter.clone();

    let mut result = AccDirective {
        name: make_c_string(parsed.name.as_ref()),
        language: language_code(language),
        clauses,
        cache_data: None,
        wait_data: None,
        routine_name: None,
        end_paired_kind: None,
    };

    if ast_parameter.is_some() {
        apply_ast_parameters(&mut result, ast_parameter);
    }

    let name = parsed.name.as_ref();

    if result.cache_data.is_none() {
        if let Some(cache) = parsed.cache_data.as_ref() {
            result.cache_data = Some(convert_cache_directive_data(cache));
        }
    }

    if result.wait_data.is_none() {
        if let Some(wait_data) = parsed.wait_data.as_ref() {
            result.wait_data = Some(convert_wait_directive_data(wait_data));
        }
    }

    // Use parameter field directly for routine name (set by parse_routine_directive)
    // and for end directive paired kind (set by parse_end_directive)
    if result.routine_name.is_none() || result.end_paired_kind.is_none() {
        if let Some(param) = parsed.parameter.as_ref() {
            if name.eq_ignore_ascii_case("routine") && result.routine_name.is_none() {
                let routine_name = param.as_ref().trim();
                let routine_name = routine_name
                    .strip_prefix('(')
                    .and_then(|s| s.strip_suffix(')'))
                    .unwrap_or(routine_name);
                result.routine_name = Some(make_c_string(routine_name));
            } else if name.eq_ignore_ascii_case("end") && result.end_paired_kind.is_none() {
                let dname = lookup_directive_name(param.as_ref());
                let kind = acc_directive_name_to_kind(dname);
                result.end_paired_kind = Some(kind);
            }
        }
    }

    result
}

fn apply_ast_parameters(result: &mut AccDirective, parameter: Option<AccDirectiveParameter>) {
    use AccDirectiveParameter::*;
    if let Some(param) = parameter {
        match param {
            Cache(cache) => {
                result.cache_data = Some(CacheData {
                    modifier: if cache.readonly {
                        ACC_CACHE_MODIFIER_READONLY
                    } else {
                        ACC_CACHE_MODIFIER_UNSPECIFIED
                    },
                    expressions: cache
                        .variables
                        .iter()
                        .map(|ident| make_c_string(ident.as_str()))
                        .collect(),
                });
            }
            Wait(wait) => {
                result.wait_data = Some(WaitDirectiveData {
                    devnum: wait
                        .devnum
                        .as_ref()
                        .map(|expr| make_c_string(&expr.to_string())),
                    queues: wait.explicit_queues,
                    expressions: wait
                        .queues
                        .iter()
                        .map(|expr| make_c_string(&expr.to_string()))
                        .collect(),
                });
            }
            Routine(routine) => {
                if let Some(name) = routine.name.as_ref() {
                    result.routine_name = Some(make_c_string(name.as_str()));
                }
            }
            End(kind) => {
                result.end_paired_kind = Some(acc_directive_name_to_kind(
                    crate::parser::directive_kind::DirectiveName::from(kind),
                ));
            }
        }
    }
}

fn build_clauses_from_ast(ast: &AstAccDirective) -> Vec<AccClause> {
    ast.clauses
        .iter()
        .map(convert_acc_clause_from_ast)
        .collect()
}

fn convert_cache_directive_data(data: &ParserCacheDirectiveData<'_>) -> CacheData {
    let modifier = if data.readonly {
        ACC_CACHE_MODIFIER_READONLY
    } else {
        ACC_CACHE_MODIFIER_UNSPECIFIED
    };

    let expressions = data
        .variables
        .iter()
        .map(|value| make_c_string(value.as_ref()))
        .collect();

    CacheData {
        modifier,
        expressions,
    }
}

fn convert_wait_directive_data(data: &ParserWaitDirectiveData<'_>) -> WaitDirectiveData {
    let devnum = data
        .devnum
        .as_ref()
        .map(|value| make_c_string(value.as_ref()));
    let expressions = data
        .queue_exprs
        .iter()
        .map(|expr| make_c_string(expr.as_ref()))
        .collect();

    WaitDirectiveData {
        devnum,
        queues: data.has_queues,
        expressions,
    }
}

#[no_mangle]
pub extern "C" fn acc_directive_free(directive: *mut AccDirective) {
    if directive.is_null() {
        return;
    }

    unsafe {
        drop(Box::from_raw(directive));
    }
}

#[no_mangle]
pub extern "C" fn acc_directive_kind(directive: *const AccDirective) -> i32 {
    if directive.is_null() {
        return -1;
    }
    unsafe {
        let name = (*directive).name.as_c_str().to_str().unwrap_or("");
        let dname = lookup_directive_name(name);
        // Prefer an OpenACC-specific mapping when available so we can
        // preserve directive codes expected by compatibility layers.
        // The internal OpenACC mapping uses an ACC_DIRECTIVE_BASE offset
        // The internal OpenACC mapping puts OpenACC directive numeric codes
        // into their own numeric range (ACC_DIRECTIVE_BASE + raw). The C API
        // must expose these canonical OpenACC numeric values directly so
        // consumers (including compatibility layers) receive the authoritative
        // mapping generated at build time. Do NOT normalize back to reduced
        // 0..N values here — that leakage is the root cause of runtime
        // mismatches and must be fixed at the producer.
        acc_directive_name_to_kind(dname)
    }
}

/// OpenACC-specific mapping from `DirectiveName` -> integer kind code.
///
/// This function mirrors the old `acc_directive_name_to_kind` helper and is
/// intentionally enum-based so `src/constants_gen.rs` can extract its
/// match arms at build time (AST-only). The numeric codes align with the
/// compatibility mapping used by `compat/accparser`.
fn acc_directive_name_to_kind(name: crate::parser::directive_kind::DirectiveName) -> i32 {
    use crate::parser::directive_kind::DirectiveName::*;
    // Put OpenACC directive numeric codes into their own numeric range so
    // OpenMP and OpenACC codes never overlap. Use a large base offset.
    const ACC_DIRECTIVE_BASE: i32 = 10000;

    let raw = match name {
        // Parallel family -> 0
        Parallel | ParallelFor | ParallelDo | ParallelForSimd | ParallelDoSimd => 0,

        // Loop / For -> 1
        For | Do | ForSimd | DoSimd | Loop => 1,

        // Kernels -> 2
        Kernels => 2,

        // Sections are OpenMP constructs; do not include them in the OpenACC mapping

        // Data family
        Data => 4,
        // Distinguish space vs underscore forms explicitly so the
        // auto-generated header contains stable macros for the canonical
        // variants (space-separated where applicable). Underscore-form
        // enum variants must not be present in the AST.
        EnterData => 5,
        ExitData => 6,
        HostData => 7,
        /* underscore-form variants removed: enter_data/exit_data underscore forms
        are not valid OpenACC directives in accparser; only space-separated
        canonical names are supported. */
        // Atomic / declare / wait / end
        Atomic => 11,
        Declare => 12,
        Wait => 13,
        End => 14,

        // Update
        Update => 15,

        // Kernel/Loop family (unique values to avoid duplicates)
        KernelsLoop => 16,
        ParallelLoop => 17,
        SerialLoop => 18,
        Serial => 19,

        // Misc
        Routine => 20,
        Set => 21,
        Init => 22,
        Shutdown => 23,
        Cache => 24,

        // Only include directives that accparser's OpenACCKinds.h understands.
        // The accparser header contains: atomic, cache, data, declare, end,
        // enter_data, exit_data, host_data, init, kernels, kernels_loop,
        // loop, parallel, parallel_loop, routine, serial, serial_loop,
        // set, shutdown, update, wait.

        // Default: unknown in OpenACC mapping — enforce strict separation.
        _ => return -1,
    };

    ACC_DIRECTIVE_BASE + raw
}

#[no_mangle]
pub extern "C" fn acc_directive_language(directive: *const AccDirective) -> i32 {
    if directive.is_null() {
        return ROUP_LANG_C;
    }

    unsafe { (*directive).language }
}

#[no_mangle]
pub extern "C" fn acc_directive_name(directive: *const AccDirective) -> *const c_char {
    if directive.is_null() {
        return ptr::null();
    }

    unsafe { (*directive).name.as_ptr() }
}

#[no_mangle]
pub extern "C" fn acc_directive_clause_count(directive: *const AccDirective) -> i32 {
    if directive.is_null() {
        return 0;
    }

    unsafe { (*directive).clauses.len() as i32 }
}

#[no_mangle]
pub extern "C" fn acc_directive_clauses_iter(
    directive: *const AccDirective,
) -> *mut AccClauseIterator {
    if directive.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let dir = &*directive;
        let clauses = dir.clauses.iter().map(|c| c as *const AccClause).collect();
        Box::into_raw(Box::new(AccClauseIterator { clauses, index: 0 }))
    }
}

#[no_mangle]
pub extern "C" fn acc_clause_iterator_next(
    iter: *mut AccClauseIterator,
    out: *mut *const AccClause,
) -> i32 {
    if iter.is_null() || out.is_null() {
        return 0;
    }

    unsafe {
        let iterator = &mut *iter;
        if iterator.index >= iterator.clauses.len() {
            *out = ptr::null();
            return 0;
        }

        *out = iterator.clauses[iterator.index];
        iterator.index += 1;
        1
    }
}

#[no_mangle]
pub extern "C" fn acc_clause_iterator_free(iter: *mut AccClauseIterator) {
    if iter.is_null() {
        return;
    }

    unsafe {
        drop(Box::from_raw(iter));
    }
}

#[no_mangle]
pub extern "C" fn acc_clause_kind(clause: *const AccClause) -> i32 {
    if clause.is_null() {
        return -1;
    }

    unsafe { (*clause).kind }
}

#[no_mangle]
pub extern "C" fn acc_clause_modifier(clause: *const AccClause) -> i32 {
    if clause.is_null() {
        return 0;
    }

    unsafe { (*clause).modifier }
}

#[no_mangle]
pub extern "C" fn acc_clause_original_keyword(clause: *const AccClause) -> *const c_char {
    if clause.is_null() {
        return ptr::null();
    }

    unsafe {
        (*clause)
            .original_keyword
            .as_ref()
            .map(|kw| kw.as_ptr())
            .unwrap_or(ptr::null())
    }
}

#[no_mangle]
pub extern "C" fn acc_clause_expressions_count(clause: *const AccClause) -> i32 {
    if clause.is_null() {
        return 0;
    }

    unsafe { (*clause).expressions.len() as i32 }
}

#[no_mangle]
pub extern "C" fn acc_clause_expression_at(clause: *const AccClause, index: i32) -> *const c_char {
    if clause.is_null() || index < 0 {
        return ptr::null();
    }

    unsafe {
        let clause_ref = &*clause;
        let idx = index as usize;
        clause_ref
            .expressions
            .get(idx)
            .map(|expr| expr.as_ptr())
            .unwrap_or(ptr::null())
    }
}

#[no_mangle]
pub extern "C" fn acc_clause_wait_devnum(clause: *const AccClause) -> *const c_char {
    if clause.is_null() {
        return ptr::null();
    }

    unsafe {
        (*clause)
            .wait_devnum
            .as_ref()
            .map(|value| value.as_ptr())
            .unwrap_or(ptr::null())
    }
}

#[no_mangle]
pub extern "C" fn acc_clause_wait_has_queues(clause: *const AccClause) -> i32 {
    if clause.is_null() {
        return 0;
    }

    unsafe {
        if (*clause).flags.contains(AccClauseFlags::WAIT_HAS_QUEUES) {
            1
        } else {
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn acc_cache_directive_modifier(directive: *const AccDirective) -> i32 {
    if directive.is_null() {
        return ACC_CACHE_MODIFIER_UNSPECIFIED;
    }

    unsafe {
        (*directive)
            .cache_data
            .as_ref()
            .map(|data| data.modifier)
            .unwrap_or(ACC_CACHE_MODIFIER_UNSPECIFIED)
    }
}

#[no_mangle]
pub extern "C" fn acc_cache_directive_var_count(directive: *const AccDirective) -> i32 {
    if directive.is_null() {
        return 0;
    }

    unsafe {
        (*directive)
            .cache_data
            .as_ref()
            .map(|data| data.expressions.len() as i32)
            .unwrap_or(0)
    }
}

#[no_mangle]
pub extern "C" fn acc_cache_directive_var_at(
    directive: *const AccDirective,
    index: i32,
) -> *const c_char {
    if directive.is_null() || index < 0 {
        return ptr::null();
    }

    unsafe {
        (*directive)
            .cache_data
            .as_ref()
            .and_then(|data| data.expressions.get(index as usize))
            .map(|value| value.as_ptr())
            .unwrap_or(ptr::null())
    }
}

#[no_mangle]
pub extern "C" fn acc_directive_wait_expression_count(directive: *const AccDirective) -> i32 {
    if directive.is_null() {
        return 0;
    }

    unsafe {
        (*directive)
            .wait_data
            .as_ref()
            .map(|data| data.expressions.len() as i32)
            .unwrap_or(0)
    }
}

#[no_mangle]
pub extern "C" fn acc_directive_wait_expression_at(
    directive: *const AccDirective,
    index: i32,
) -> *const c_char {
    if directive.is_null() || index < 0 {
        return ptr::null();
    }

    unsafe {
        (*directive)
            .wait_data
            .as_ref()
            .and_then(|data| data.expressions.get(index as usize))
            .map(|value| value.as_ptr())
            .unwrap_or(ptr::null())
    }
}

#[no_mangle]
pub extern "C" fn acc_directive_wait_devnum(directive: *const AccDirective) -> *const c_char {
    if directive.is_null() {
        return ptr::null();
    }

    unsafe {
        (*directive)
            .wait_data
            .as_ref()
            .and_then(|data| data.devnum.as_ref())
            .map(|value| value.as_ptr())
            .unwrap_or(ptr::null())
    }
}

#[no_mangle]
pub extern "C" fn acc_directive_wait_has_queues(directive: *const AccDirective) -> i32 {
    if directive.is_null() {
        return 0;
    }

    unsafe {
        (*directive)
            .wait_data
            .as_ref()
            .map(|data| if data.queues { 1 } else { 0 })
            .unwrap_or(0)
    }
}

#[no_mangle]
pub extern "C" fn acc_directive_routine_name(directive: *const AccDirective) -> *const c_char {
    if directive.is_null() {
        return ptr::null();
    }

    unsafe {
        (*directive)
            .routine_name
            .as_ref()
            .map(|value| value.as_ptr())
            .unwrap_or(ptr::null())
    }
}

#[no_mangle]
pub extern "C" fn acc_directive_end_paired_kind(directive: *const AccDirective) -> i32 {
    if directive.is_null() {
        return -1;
    }

    unsafe { (*directive).end_paired_kind.unwrap_or(-1) }
}

fn convert_acc_clause_from_ast(ast_clause: &AstAccClause) -> AccClause {
    let kind_code = clause_name_to_kind(ast_clause.kind.as_str());
    let mut clause = AccClause {
        kind: kind_code,
        modifier: 0,
        original_keyword: None,
        expressions: Vec::new(),
        wait_devnum: None,
        flags: AccClauseFlags::empty(),
    };

    use AstAccClausePayload::*;
    match &ast_clause.payload {
        Bare => {}
        Expression(expr) => {
            clause.expressions.push(make_c_string(&expr.to_string()));
        }
        IdentifierList(items) => {
            clause.expressions = identifiers_to_cstrings(items);
        }
        Copy(copy) => {
            clause.expressions = identifiers_to_cstrings(&copy.variables);
            clause.modifier = match copy.kind {
                AccCopyKind::CopyIn
                | AccCopyKind::PCopyIn
                | AccCopyKind::PresentOrCopyIn
                | AccCopyKind::Copy => match copy.modifier {
                    Some(AccCopyModifier::Readonly) => ACC_COPYIN_MODIFIER_READONLY,
                    _ => ACC_COPYIN_MODIFIER_UNSPECIFIED,
                },
                AccCopyKind::CopyOut | AccCopyKind::PCopyOut | AccCopyKind::PresentOrCopyOut => {
                    match copy.modifier {
                        Some(AccCopyModifier::Zero) => ACC_COPYOUT_MODIFIER_ZERO,
                        _ => ACC_COPYOUT_MODIFIER_UNSPECIFIED,
                    }
                }
                _ => ACC_COPYIN_MODIFIER_UNSPECIFIED,
            };
        }
        Create(create) => {
            clause.expressions = identifiers_to_cstrings(&create.variables);
            clause.modifier = match create.modifier {
                Some(AccCreateModifier::Zero) => ACC_CREATE_MODIFIER_ZERO,
                _ => ACC_CREATE_MODIFIER_UNSPECIFIED,
            };
        }
        Reduction(reduction) => {
            clause.expressions = identifiers_to_cstrings(&reduction.variables);
            clause.modifier = acc_reduction_operator_code(&reduction.operator);
        }
        Data(data) => {
            clause.expressions = identifiers_to_cstrings(&data.variables);
        }
        DeviceType(values) => {
            clause.expressions = values.iter().map(|v| make_c_string(v)).collect();
        }
        Default(kind) => {
            clause.modifier = match kind {
                AccDefaultKind::Unspecified => ACC_DEFAULT_KIND_UNSPECIFIED,
                AccDefaultKind::None => ACC_DEFAULT_KIND_NONE,
                AccDefaultKind::Present => ACC_DEFAULT_KIND_PRESENT,
            };
        }
        Wait(wait) => {
            clause.wait_devnum = wait
                .devnum
                .as_ref()
                .map(|expr| make_c_string(&expr.to_string()));
            clause.expressions = wait
                .queues
                .iter()
                .map(|expr| make_c_string(&expr.to_string()))
                .collect();
            if wait.explicit_queues {
                clause.flags.insert(AccClauseFlags::WAIT_HAS_QUEUES);
            }
            if clause.wait_devnum.is_some() {
                clause.flags.insert(AccClauseFlags::WAIT_HAS_DEVNUM);
            }
        }
        Vector(data) => {
            set_vector_worker_payload(&mut clause, data, AstAccClauseKind::Vector);
        }
        Worker(data) => {
            set_vector_worker_payload(&mut clause, data, AstAccClauseKind::Worker);
        }
        Gang(data) => {
            clause.expressions = gang_clause_expressions(data);
        }
    }

    clause
}

fn identifiers_to_cstrings(items: &[crate::ir::Identifier]) -> Vec<CString> {
    items
        .iter()
        .map(|ident| make_c_string(ident.as_str()))
        .collect()
}

fn set_vector_worker_payload(clause: &mut AccClause, data: &AccGangClause, kind: AstAccClauseKind) {
    clause.expressions = clause_expressions_with_modifier(&data.values, data.modifier.as_deref());

    match kind {
        AstAccClauseKind::Vector => {
            clause.modifier = match data.modifier.as_deref() {
                Some("length") => ACC_VECTOR_MODIFIER_LENGTH,
                _ => ACC_VECTOR_MODIFIER_UNSPECIFIED,
            };
        }
        AstAccClauseKind::Worker => {
            clause.modifier = match data.modifier.as_deref() {
                Some("num") => ACC_WORKER_MODIFIER_NUM,
                _ => ACC_WORKER_MODIFIER_UNSPECIFIED,
            };
        }
        _ => {}
    }
}

fn gang_clause_expressions(data: &AccGangClause) -> Vec<CString> {
    clause_expressions_with_modifier(&data.values, data.modifier.as_deref())
}

fn clause_expressions_with_modifier(
    values: &[crate::ir::Expression],
    modifier: Option<&str>,
) -> Vec<CString> {
    let rendered_values: Vec<String> = values.iter().map(|expr| expr.to_string()).collect();

    if let Some(prefix) = modifier {
        let joined = join_expression_list(&rendered_values);
        let formatted = if joined.is_empty() {
            prefix.to_string()
        } else {
            format!("{prefix}: {joined}")
        };
        vec![make_c_string(&formatted)]
    } else {
        rendered_values
            .into_iter()
            .filter_map(|value| {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(make_c_string(trimmed))
                }
            })
            .collect()
    }
}

fn join_expression_list(values: &[String]) -> String {
    let mut result = String::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !result.is_empty() {
            result.push_str(", ");
        }
        result.push_str(trimmed);
    }
    result
}

fn acc_reduction_operator_code(token: &str) -> i32 {
    match token.to_ascii_lowercase().as_str() {
        "readonly" => ACC_REDUCTION_OP_READONLY,
        "+" => ACC_REDUCTION_OP_ADD,
        "-" => ACC_REDUCTION_OP_SUB,
        "*" => ACC_REDUCTION_OP_MUL,
        "max" => ACC_REDUCTION_OP_MAX,
        "min" => ACC_REDUCTION_OP_MIN,
        "&" => ACC_REDUCTION_OP_BITAND,
        "|" => ACC_REDUCTION_OP_BITOR,
        "^" => ACC_REDUCTION_OP_BITXOR,
        "&&" => ACC_REDUCTION_OP_LOGAND,
        "||" => ACC_REDUCTION_OP_LOGOR,
        "and" | ".and." => ACC_REDUCTION_OP_FORT_AND,
        "or" | ".or." => ACC_REDUCTION_OP_FORT_OR,
        "eqv" | ".eqv." => ACC_REDUCTION_OP_FORT_EQV,
        "neqv" | ".neqv." => ACC_REDUCTION_OP_FORT_NEQV,
        "iand" => ACC_REDUCTION_OP_FORT_IAND,
        "ior" => ACC_REDUCTION_OP_FORT_IOR,
        "ieor" => ACC_REDUCTION_OP_FORT_IEOR,
        _ => ACC_REDUCTION_OP_UNSPECIFIED,
    }
}

fn make_c_string(value: &str) -> CString {
    if value.contains('\0') {
        let sanitized = value.replace('\0', " ");
        CString::new(sanitized).unwrap()
    } else {
        CString::new(value).unwrap()
    }
}

fn language_code(language: Language) -> i32 {
    match language {
        Language::C => ROUP_LANG_C,
        Language::FortranFree => ROUP_LANG_FORTRAN_FREE,
        Language::FortranFixed => ROUP_LANG_FORTRAN_FIXED,
    }
}

// acc_directive_name_to_kind is removed: we now use the canonical
// `DirectiveName` lookup and the shared `directive_name_enum_to_kind`
// helper in the parent module directly. Unknown directives return -1.

fn clause_name_to_kind(name: &str) -> i32 {
    let cname = crate::parser::lookup_clause_name(name);
    use crate::parser::ClauseName;

    match cname {
        ClauseName::Copy => 35,
        ClauseName::CopyIn => 36,
        ClauseName::CopyOut => 37,
        ClauseName::Create => 38,
        ClauseName::Present => 39,
        // OpenACC-specific clause kind codes (match generated header expectations)
        ClauseName::Async => 2000,
        ClauseName::Wait => 2001,
        ClauseName::NumGangs => 2002,
        ClauseName::NumWorkers => 2003,
        ClauseName::VectorLength => 2004,
        ClauseName::Gang => 2005,
        ClauseName::Worker => 2006,
        ClauseName::Vector => 2007,
        ClauseName::Seq => 2008,
        ClauseName::Independent => 2009,
        ClauseName::Auto => 2010,
        ClauseName::DeviceType => 2011,
        ClauseName::Bind => 2012,
        ClauseName::DefaultAsync => 2013,
        ClauseName::Link => 2014,
        ClauseName::NoCreate => 2015,
        ClauseName::NoHost => 2016,
        ClauseName::Read => 2017,
        ClauseName::SelfClause => 2018,
        ClauseName::Tile => 2019,
        ClauseName::UseDevice => 2020,
        ClauseName::Attach => 2021,
        ClauseName::Detach => 2022,
        ClauseName::Finalize => 2023,
        ClauseName::IfPresent => 2024,
        ClauseName::Capture => 2025,
        ClauseName::Write => 2026,
        ClauseName::Update => 2027,
        ClauseName::Delete => 2028,
        ClauseName::Device => 2029,
        ClauseName::DevicePtr => 2030,
        ClauseName::DeviceNum => 2031,
        ClauseName::DeviceResident => 2032,
        ClauseName::Host => 2033,
        ClauseName::Other(ref s) => panic!("Unknown OpenACC clause: {}", s),
        ClauseName::NumThreads => 0,
        ClauseName::If => 14,
        ClauseName::Private => 22,
        ClauseName::Shared => 21, // shared mapping
        ClauseName::Firstprivate => 16,
        ClauseName::Lastprivate => -1, // not present in OpenACC table above
        ClauseName::Reduction => 23,
        ClauseName::Schedule => -1, // not in OpenACC table
        ClauseName::Collapse => 11,
        ClauseName::Ordered => -1,
        ClauseName::Nowait => -1,
        ClauseName::Default => 15,
        // OpenMP-specific atomic/map clauses (not present in OpenACC)
        ClauseName::Hint => -1,
        ClauseName::SeqCst => -1,
        ClauseName::Release => -1,
        ClauseName::Acquire => -1,
        ClauseName::Relaxed => -1,
        ClauseName::AcqRel => -1,
        ClauseName::Map => -1,
        ClauseName::Allocator => -1,
        ClauseName::Align => -1,
        // Additional OpenMP-only clauses (not in OpenACC)
        ClauseName::InReduction => -1,
        ClauseName::IsDevicePtr => -1,
        ClauseName::Defaultmap => -1,
        ClauseName::Depend => -1,
        ClauseName::UsesAllocators => -1,
        ClauseName::NumTeams => -1,
        ClauseName::ThreadLimit => -1,
        ClauseName::DistSchedule => -1,
        // Additional OpenMP-only clauses (not in OpenACC)
        ClauseName::ProcBind => -1,
        ClauseName::Allocate => -1,
        ClauseName::Linear => -1,
        ClauseName::Safelen => -1,
        ClauseName::Simdlen => -1,
        ClauseName::Aligned => -1,
        ClauseName::Nontemporal => -1,
        ClauseName::Uniform => -1,
        ClauseName::Inbranch => -1,
        ClauseName::Notinbranch => -1,
        ClauseName::Inclusive => -1,
        ClauseName::Exclusive => -1,
        ClauseName::Copyprivate => -1,
        ClauseName::Parallel => -1,
        ClauseName::Sections => -1,
        ClauseName::For => -1,
        ClauseName::Do => -1,
        ClauseName::Taskgroup => -1,
        ClauseName::Initializer => -1,
        ClauseName::Final => -1,
        ClauseName::Untied => -1,
        ClauseName::Requires => -1,
        ClauseName::Mergeable => -1,
        ClauseName::Priority => -1,
        ClauseName::Affinity => -1,
        ClauseName::Grainsize => -1,
        ClauseName::NumTasks => -1,
        ClauseName::Nogroup => -1,
        ClauseName::ReverseOffload => -1,
        ClauseName::UnifiedAddress => -1,
        ClauseName::UnifiedSharedMemory => -1,
        ClauseName::AtomicDefaultMemOrder => -1,
        ClauseName::DynamicAllocators => -1,
        ClauseName::SelfMaps => -1,
        ClauseName::ExtImplementationDefinedRequirement => -1,
        ClauseName::UseDevicePtr => -1,
        ClauseName::Sizes => -1,
        ClauseName::UseDeviceAddr => -1,
        ClauseName::HasDeviceAddr => -1,
        ClauseName::To => -1,
        ClauseName::From => -1,
        ClauseName::When => -1,
        ClauseName::Match => -1,
        ClauseName::TaskReduction => -1,
        ClauseName::Destroy => -1,
        ClauseName::DepobjUpdate => -1,
        ClauseName::Compare => -1,
        ClauseName::CompareCapture => -1,
        ClauseName::Partial => -1,
        ClauseName::Full => -1,
        ClauseName::Order => -1,
        // Additional OpenMP-only clauses added for ompparser compatibility (not in OpenACC)
        ClauseName::Threads => -1,
        ClauseName::Simd => -1,
        ClauseName::Filter => -1,
        ClauseName::Fail => -1,
        ClauseName::Weak => -1,
        ClauseName::At => -1,
        ClauseName::Severity => -1,
        ClauseName::Message => -1,
        ClauseName::Doacross => -1,
        ClauseName::Absent => -1,
        ClauseName::Contains => -1,
        ClauseName::Holds => -1,
        ClauseName::Otherwise => -1,
        ClauseName::GraphId => -1,
        ClauseName::GraphReset => -1,
        ClauseName::Transparent => -1,
        ClauseName::Replayable => -1,
        ClauseName::Threadset => -1,
        ClauseName::Indirect => -1,
        ClauseName::Local => -1,
        ClauseName::Init => -1,
        ClauseName::InitComplete => -1,
        ClauseName::Safesync => -1,
        ClauseName::DeviceSafesync => -1,
        ClauseName::Memscope => -1,
        ClauseName::Looprange => -1,
        ClauseName::Permutation => -1,
        ClauseName::Counts => -1,
        ClauseName::Induction => -1,
        ClauseName::Inductor => -1,
        ClauseName::Collector => -1,
        ClauseName::Combiner => -1,
        ClauseName::AdjustArgs => -1,
        ClauseName::AppendArgs => -1,
        ClauseName::Apply => -1,
        ClauseName::NoOpenmp => -1,
        ClauseName::NoOpenmpConstructs => -1,
        ClauseName::NoOpenmpRoutines => -1,
        ClauseName::NoParallelism => -1,
        ClauseName::Nocontext => -1,
        ClauseName::Novariants => -1,
        ClauseName::Enter => -1,
        ClauseName::Use => -1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn parse_plain_acc_string() {
        let input = CString::new("#pragma acc parallel").unwrap();
        let directive = acc_parse(input.as_ptr());
        assert!(!directive.is_null());
        acc_directive_free(directive);
    }
}
