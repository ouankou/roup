use std::borrow::Cow;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

use bitflags::bitflags;

use crate::lexer::Language;
use crate::parser::{
    openacc as openacc_parser, CacheDirectiveData as ParserCacheDirectiveData, Clause, ClauseKind,
    CopyinModifier, CopyoutModifier, CreateModifier, Directive, GangModifier, ReductionOperator,
    VectorModifier, WaitDirectiveData as ParserWaitDirectiveData, WorkerModifier,
};

use super::{ROUP_LANG_C, ROUP_LANG_FORTRAN_FIXED, ROUP_LANG_FORTRAN_FREE};

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

        let converted = build_acc_directive(directive, language);
        Box::into_raw(Box::new(converted))
    }
}

fn build_acc_directive(parsed: Directive<'_>, language: Language) -> AccDirective {
    let mut result = AccDirective {
        name: make_c_string(parsed.name.as_ref()),
        language: language_code(language),
        clauses: parsed.clauses.iter().map(convert_acc_clause).collect(),
        cache_data: None,
        wait_data: None,
        routine_name: None,
        end_paired_kind: None,
    };

    let name = parsed.name.as_ref();

    if let Some(cache) = parsed.cache_data.as_ref() {
        result.cache_data = Some(convert_cache_directive_data(cache));
    }

    if let Some(wait_data) = parsed.wait_data.as_ref() {
        result.wait_data = Some(convert_wait_directive_data(wait_data));
    }

    // Use parameter field directly for routine name (set by parse_routine_directive)
    // and for end directive paired kind (set by parse_end_directive)
    if let Some(param) = parsed.parameter.as_ref() {
        if name.eq_ignore_ascii_case("routine") {
            result.routine_name = Some(make_c_string(param.as_ref()));
        } else if name.eq_ignore_ascii_case("end") {
            // For "end" directives, parameter contains the directive being ended (e.g., "atomic")
            let kind = acc_directive_name_to_kind(param.as_ref());
            result.end_paired_kind = Some(kind);
        }
    }

    result
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
        acc_directive_name_to_kind(name)
    }
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

fn convert_acc_clause(clause: &Clause) -> AccClause {
    let normalized_name = clause.name.to_ascii_lowercase();
    let original_keyword = if clause.name.as_ref().eq_ignore_ascii_case(&normalized_name) {
        None
    } else {
        Some(make_c_string(clause.name.as_ref()))
    };

    let clause_kind = clause_name_to_kind(&normalized_name);
    let clause_content = clause_content_from_kind(&clause.kind);
    let clause_content_str = clause_content.as_deref();

    let mut modifier = 0;
    let mut wait_devnum = None;
    let mut flags = AccClauseFlags::empty();
    let mut expressions: Vec<String> = Vec::new();

    match normalized_name.as_str() {
        "copyin" | "pcopyin" | "present_or_copyin" => {
            let (mod_value, exprs) = parse_prefixed_values(
                clause_content_str,
                "readonly",
                ACC_COPYIN_MODIFIER_READONLY,
                ACC_COPYIN_MODIFIER_UNSPECIFIED,
            );
            modifier = mod_value;
            expressions = exprs;
        }
        "copyout" | "pcopyout" | "present_or_copyout" => {
            let (mod_value, exprs) = parse_prefixed_values(
                clause_content_str,
                "zero",
                ACC_COPYOUT_MODIFIER_ZERO,
                ACC_COPYOUT_MODIFIER_UNSPECIFIED,
            );
            modifier = mod_value;
            expressions = exprs;
        }
        "copy" | "pcopy" | "present_or_copy" | "present" | "attach" | "detach" | "use_device"
        | "link" | "device_resident" | "host" | "device" | "deviceptr" | "delete" | "private"
        | "firstprivate" | "device_type" | "dtype" => {
            if let Some(clause_text) = clause_content_str {
                expressions = split_arguments(clause_text);
            }
        }
        "create" | "pcreate" | "present_or_create" => {
            let (mod_value, exprs) = parse_prefixed_values(
                clause_content_str,
                "zero",
                ACC_CREATE_MODIFIER_ZERO,
                ACC_CREATE_MODIFIER_UNSPECIFIED,
            );
            modifier = mod_value;
            expressions = exprs;
        }
        "default" => {
            modifier = parse_default_modifier(clause_content_str);
        }
        "reduction" => {
            let (op, exprs) = parse_reduction_clause(clause_content_str);
            modifier = op;
            expressions = exprs;
        }
        "vector" => {
            let (mod_value, exprs) = parse_prefixed_values(
                clause_content_str,
                "length",
                ACC_VECTOR_MODIFIER_LENGTH,
                ACC_VECTOR_MODIFIER_UNSPECIFIED,
            );
            modifier = mod_value;
            expressions = exprs;
        }
        "worker" => {
            let (mod_value, exprs) = parse_prefixed_values(
                clause_content_str,
                "num",
                ACC_WORKER_MODIFIER_NUM,
                ACC_WORKER_MODIFIER_UNSPECIFIED,
            );
            modifier = mod_value;
            expressions = exprs;
        }
        "wait" => {
            let (devnum_value, has_queues, exprs) = parse_wait_clause(clause_content_str);
            if let Some(devnum) = devnum_value {
                wait_devnum = Some(devnum);
                flags.insert(AccClauseFlags::WAIT_HAS_DEVNUM);
            }
            if has_queues {
                flags.insert(AccClauseFlags::WAIT_HAS_QUEUES);
            }
            expressions = exprs;
        }
        _ => {
            if let Some(value) = clause_content_str {
                expressions = split_arguments(value);
            }
        }
    }

    let expression_strings = expressions
        .into_iter()
        .map(|expr| make_c_string(&expr))
        .collect();

    AccClause {
        kind: clause_kind,
        modifier,
        original_keyword,
        expressions: expression_strings,
        wait_devnum,
        flags,
    }
}

fn parse_wait_clause(content: Option<&str>) -> (Option<CString>, bool, Vec<String>) {
    let Some(raw) = content else {
        return (None, false, Vec::new());
    };

    let (devnum, has_queues, exprs, parsed) = parse_wait_components(raw);
    if parsed {
        (devnum.map(|value| make_c_string(&value)), has_queues, exprs)
    } else {
        (None, false, split_arguments(raw))
    }
}

fn parse_wait_components(input: &str) -> (Option<String>, bool, Vec<String>, bool) {
    let mut rest = input.trim();
    let mut devnum = None;
    let mut has_queues = false;
    let mut parsed = false;

    if let Some((value, remaining)) = strip_named_section(rest, "devnum") {
        devnum = Some(value.trim().to_string());
        rest = remaining;
        parsed = true;
    }

    if let Some(after_queues) = strip_named_section_simple(rest, "queues") {
        has_queues = true;
        rest = after_queues;
        parsed = true;
    }

    let expressions = split_arguments(rest);
    (devnum, has_queues, expressions, parsed)
}

fn strip_named_section<'a>(input: &'a str, keyword: &str) -> Option<(&'a str, &'a str)> {
    let trimmed = input.trim_start();
    if !trimmed.to_ascii_lowercase().starts_with(keyword) {
        return None;
    }

    let mut rest = &trimmed[keyword.len()..];
    rest = rest.trim_start();
    if !rest.starts_with(':') {
        return None;
    }
    rest = rest[1..].trim_start();
    if rest.starts_with(':') {
        // Fortran-style double colon: treat as literal content
        return None;
    }

    let (value, remaining) = split_once_outside_double_colon(rest, ':').unwrap_or((rest, ""));
    Some((value.trim(), remaining.trim_start()))
}

fn strip_named_section_simple<'a>(input: &'a str, keyword: &str) -> Option<&'a str> {
    let trimmed = input.trim_start();
    if !trimmed.to_ascii_lowercase().starts_with(keyword) {
        return None;
    }

    let mut rest = &trimmed[keyword.len()..];
    rest = rest.trim_start();
    if !rest.starts_with(':') {
        return None;
    }
    rest = rest[1..].trim_start();
    if rest.starts_with(':') {
        // Fortran syntax - treat as literal
        return None;
    }

    Some(rest)
}

fn clause_content_from_kind<'a>(kind: &'a ClauseKind<'a>) -> Option<Cow<'a, str>> {
    match kind {
        ClauseKind::Parenthesized(value) => Some(Cow::Borrowed(value.as_ref())),
        ClauseKind::VariableList(values) => Some(Cow::Owned(join_variable_list(values))),
        ClauseKind::GangClause {
            modifier,
            variables,
        } => Some(Cow::Owned(format_gang_clause(*modifier, variables))),
        ClauseKind::WorkerClause {
            modifier,
            variables,
        } => Some(Cow::Owned(format_worker_clause(*modifier, variables))),
        ClauseKind::VectorClause {
            modifier,
            variables,
        } => Some(Cow::Owned(format_vector_clause(*modifier, variables))),
        ClauseKind::CopyinClause {
            modifier,
            variables,
        } => Some(Cow::Owned(format_copyin_clause(*modifier, variables))),
        ClauseKind::CopyoutClause {
            modifier,
            variables,
        } => Some(Cow::Owned(format_copyout_clause(*modifier, variables))),
        ClauseKind::CreateClause {
            modifier,
            variables,
        } => Some(Cow::Owned(format_create_clause(*modifier, variables))),
        ClauseKind::ReductionClause {
            operator,
            variables,
        } => Some(Cow::Owned(format_reduction_clause(*operator, variables))),
        ClauseKind::Bare => None,
    }
}

fn join_variable_list(values: &[Cow<'_, str>]) -> String {
    let mut result = String::new();
    for value in values {
        let trimmed = value.as_ref().trim();
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

fn format_with_optional_prefix(
    prefix: &str,
    has_prefix: bool,
    variables: &[Cow<'_, str>],
) -> String {
    let joined = join_variable_list(variables);
    if has_prefix {
        if joined.is_empty() {
            prefix.to_string()
        } else {
            format!("{prefix}: {joined}")
        }
    } else {
        joined
    }
}

fn format_gang_clause(modifier: Option<GangModifier>, variables: &[Cow<'_, str>]) -> String {
    match modifier {
        Some(GangModifier::Num) => format_with_optional_prefix("num", true, variables),
        Some(GangModifier::Static) => format_with_optional_prefix("static", true, variables),
        None => join_variable_list(variables),
    }
}

fn format_worker_clause(modifier: Option<WorkerModifier>, variables: &[Cow<'_, str>]) -> String {
    let has_prefix = matches!(modifier, Some(WorkerModifier::Num));
    format_with_optional_prefix("num", has_prefix, variables)
}

fn format_vector_clause(modifier: Option<VectorModifier>, variables: &[Cow<'_, str>]) -> String {
    let has_prefix = matches!(modifier, Some(VectorModifier::Length));
    format_with_optional_prefix("length", has_prefix, variables)
}

fn format_copyin_clause(modifier: Option<CopyinModifier>, variables: &[Cow<'_, str>]) -> String {
    let has_prefix = matches!(modifier, Some(CopyinModifier::Readonly));
    format_with_optional_prefix("readonly", has_prefix, variables)
}

fn format_copyout_clause(modifier: Option<CopyoutModifier>, variables: &[Cow<'_, str>]) -> String {
    let has_prefix = matches!(modifier, Some(CopyoutModifier::Zero));
    format_with_optional_prefix("zero", has_prefix, variables)
}

fn format_create_clause(modifier: Option<CreateModifier>, variables: &[Cow<'_, str>]) -> String {
    let has_prefix = matches!(modifier, Some(CreateModifier::Zero));
    format_with_optional_prefix("zero", has_prefix, variables)
}

fn format_reduction_clause(operator: ReductionOperator, variables: &[Cow<'_, str>]) -> String {
    let token = reduction_operator_token(operator);
    let joined = join_variable_list(variables);
    if token.is_empty() {
        joined
    } else if joined.is_empty() {
        token.to_string()
    } else {
        format!("{token}: {joined}")
    }
}

fn reduction_operator_token(operator: ReductionOperator) -> &'static str {
    match operator {
        ReductionOperator::Add => "+",
        ReductionOperator::Sub => "-",
        ReductionOperator::Mul => "*",
        ReductionOperator::Max => "max",
        ReductionOperator::Min => "min",
        ReductionOperator::BitAnd => "&",
        ReductionOperator::BitOr => "|",
        ReductionOperator::BitXor => "^",
        ReductionOperator::LogAnd => "&&",
        ReductionOperator::LogOr => "||",
        ReductionOperator::FortAnd => "and",
        ReductionOperator::FortOr => "or",
        ReductionOperator::FortEqv => "eqv",
        ReductionOperator::FortNeqv => "neqv",
        ReductionOperator::FortIand => "iand",
        ReductionOperator::FortIor => "ior",
        ReductionOperator::FortIeor => "ieor",
    }
}

fn parse_prefixed_values(
    content: Option<&str>,
    modifier_keyword: &str,
    modifier_value: i32,
    unspecified_value: i32,
) -> (i32, Vec<String>) {
    let Some(raw_content) = content else {
        return (unspecified_value, Vec::new());
    };

    if raw_content.is_empty() {
        return (unspecified_value, Vec::new());
    }

    if let Some(rest) = strip_modifier_prefix(raw_content, modifier_keyword) {
        return (modifier_value, split_arguments(rest));
    }

    (unspecified_value, split_arguments(raw_content))
}

fn strip_modifier_prefix<'a>(content: &'a str, keyword: &str) -> Option<&'a str> {
    let trimmed = content.trim_start();
    if trimmed.len() < keyword.len() {
        return None;
    }

    if !trimmed[..keyword.len()].eq_ignore_ascii_case(keyword) {
        return None;
    }

    let mut rest = &trimmed[keyword.len()..];
    rest = rest.trim_start();
    if rest.starts_with(':') {
        rest = rest[1..].trim_start();
    } else {
        return None;
    }

    Some(rest)
}

fn parse_default_modifier(content: Option<&str>) -> i32 {
    let Some(clause_content) = content else {
        return ACC_DEFAULT_KIND_UNSPECIFIED;
    };

    let value = clause_content.trim().to_ascii_lowercase();
    match value.as_str() {
        "none" => ACC_DEFAULT_KIND_NONE,
        "present" => ACC_DEFAULT_KIND_PRESENT,
        _ => ACC_DEFAULT_KIND_UNSPECIFIED,
    }
}

fn parse_reduction_clause(content: Option<&str>) -> (i32, Vec<String>) {
    let Some(raw_content) = content else {
        return (ACC_REDUCTION_OP_UNSPECIFIED, Vec::new());
    };

    let mut operator = ACC_REDUCTION_OP_UNSPECIFIED;
    let mut rest = raw_content.trim();
    if let Some((op, remaining)) = split_once_outside_double_colon(rest, ':') {
        operator = match op.trim().to_ascii_lowercase().as_str() {
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
            "and" => ACC_REDUCTION_OP_FORT_AND,
            "or" => ACC_REDUCTION_OP_FORT_OR,
            "eqv" => ACC_REDUCTION_OP_FORT_EQV,
            "neqv" => ACC_REDUCTION_OP_FORT_NEQV,
            "iand" => ACC_REDUCTION_OP_FORT_IAND,
            "ior" => ACC_REDUCTION_OP_FORT_IOR,
            "ieor" => ACC_REDUCTION_OP_FORT_IEOR,
            _ => ACC_REDUCTION_OP_UNSPECIFIED,
        };
        rest = remaining.trim();
    }

    (operator, split_arguments(rest))
}

fn split_arguments(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut paren_depth = 0;
    let mut bracket_depth = 0;

    for ch in input.chars() {
        match ch {
            '(' => {
                paren_depth += 1;
                current.push(ch);
            }
            ')' => {
                if paren_depth > 0 {
                    paren_depth -= 1;
                }
                current.push(ch);
            }
            '[' => {
                bracket_depth += 1;
                current.push(ch);
            }
            ']' => {
                if bracket_depth > 0 {
                    bracket_depth -= 1;
                }
                current.push(ch);
            }
            ',' if paren_depth == 0 && bracket_depth == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    args.push(trimmed.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    let trimmed = current.trim();
    if !trimmed.is_empty() {
        args.push(trimmed.to_string());
    }

    args
}

fn split_once_outside_double_colon(input: &str, needle: char) -> Option<(&str, &str)> {
    let mut idx = 0usize;
    let chars: Vec<char> = input.chars().collect();
    while idx < chars.len() {
        if chars[idx] == needle {
            let next = chars.get(idx + 1);
            if next == Some(&':') {
                idx += 2;
                continue;
            }
            let left = &input[..idx];
            let right = &input[idx + 1..];
            return Some((left, right));
        }
        idx += 1;
    }
    None
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

const ACC_UNKNOWN_KIND: i32 = 999;

fn acc_directive_name_to_kind(name: &str) -> i32 {
    let normalized = name.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "parallel" => 0,
        "loop" => 1,
        "kernels" => 2,
        "data" => 3,
        "enter data" => 4,
        "exit data" => 5,
        "host_data" => 6,
        "atomic" => 7,
        "declare" => 8,
        "wait" => 9,
        "end" => 10,
        "host data" => 11,
        "update" => 12,
        "cache" => 23,
        "kernels loop" => 14,
        "parallel loop" => 15,
        "serial loop" => 16,
        "serial" => 17,
        "routine" => 18,
        "set" => 19,
        "init" => 20,
        "shutdown" => 21,
        "enter_data" => 24,
        "exit_data" => 25,
        _ if normalized.starts_with("cache(") => 23,
        _ if normalized.starts_with("wait(") => 26,
        _ if normalized.starts_with("end ") => 10,
        _ => ACC_UNKNOWN_KIND,
    }
}

fn clause_name_to_kind(name: &str) -> i32 {
    match name {
        "async" => 0,
        "wait" => 1,
        "num_gangs" => 2,
        "num_workers" => 3,
        "vector_length" => 4,
        "gang" => 5,
        "worker" => 6,
        "vector" => 7,
        "seq" => 8,
        "independent" => 9,
        "auto" => 10,
        "collapse" => 11,
        "device_type" | "dtype" => 12,
        "bind" => 13,
        "if" => 14,
        "default" => 15,
        "firstprivate" => 16,
        "default_async" => 17,
        "link" => 18,
        "no_create" => 19,
        "nohost" => 20,
        "present" => 21,
        "private" => 22,
        "reduction" => 23,
        "read" => 24,
        "self" => 25,
        "tile" => 26,
        "use_device" => 27,
        "attach" => 28,
        "detach" => 29,
        "finalize" => 30,
        "if_present" => 31,
        "capture" => 32,
        "write" => 33,
        "update" => 34,
        "copy" | "pcopy" | "present_or_copy" => 35,
        "copyin" | "pcopyin" | "present_or_copyin" => 36,
        "copyout" | "pcopyout" | "present_or_copyout" => 37,
        "create" | "pcreate" | "present_or_create" => 38,
        "delete" => 39,
        "device" => 40,
        "deviceptr" => 41,
        "device_num" => 42,
        "device_resident" => 43,
        "host" => 44,
        _ => 999,
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
