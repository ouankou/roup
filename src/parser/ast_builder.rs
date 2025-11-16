use crate::ast::{
    AccCacheDirective, AccClause, AccClauseKind, AccClausePayload, AccCopyClause, AccCopyKind,
    AccCopyModifier, AccCreateClause, AccCreateKind, AccCreateModifier, AccDataClause, AccDataKind,
    AccDirective, AccDirectiveKind, AccDirectiveParameter, AccGangClause, AccReductionClause,
    AccRoutineDirective, AccWaitClause, AccWaitDirective, ClauseNormalizationMode, DirectiveBody,
    OmpClause, OmpClauseKind, OmpDirective, OmpDirectiveKind, RoupDirective, RoupLanguage,
};
use crate::ir::{
    convert::parse_clause_data, Expression, Identifier, Language, ParserConfig, SourceLocation,
};
use std::borrow::Cow;

use super::clause::{
    lookup_clause_name, Clause, ClauseKind, ClauseName, CopyinModifier, CopyoutModifier,
    CreateModifier, GangModifier, ReductionOperator as ParserReductionOperator, VectorModifier,
    WorkerModifier,
};
use super::directive::Directive;
use super::Dialect;
use crate::parser::directive_kind::lookup_directive_name;

/// Error raised during AST materialization from parser structures.
#[derive(Debug)]
pub enum AstBuildError {
    UnsupportedDirective(String),
    UnsupportedClause(String),
    ClauseConversion(String),
    ParseFailure(String),
}

impl std::fmt::Display for AstBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AstBuildError::UnsupportedDirective(name) => {
                write!(f, "directive kind not supported in current dialect: {name}")
            }
            AstBuildError::UnsupportedClause(name) => {
                write!(f, "clause kind not supported in current dialect: {name}")
            }
            AstBuildError::ClauseConversion(msg) => write!(f, "clause conversion failed: {msg}"),
            AstBuildError::ParseFailure(msg) => write!(f, "parse failed: {msg}"),
        }
    }
}

impl std::error::Error for AstBuildError {}

/// Convert a parsed directive into the enum-based ROUP AST.
pub fn build_roup_directive(
    directive: &Directive<'_>,
    dialect: Dialect,
    normalization: ClauseNormalizationMode,
    parser_config: &ParserConfig,
    host_language: Language,
) -> Result<RoupDirective, AstBuildError> {
    let normalized = normalize_directive(directive, normalization);
    let directive = normalized.as_ref();

    let language_tag = match dialect {
        Dialect::OpenMp => RoupLanguage::OpenMp,
        Dialect::OpenAcc => RoupLanguage::OpenAcc,
    };

    let body = match dialect {
        Dialect::OpenMp => DirectiveBody::OpenMp(build_omp_directive(
            directive,
            parser_config,
            host_language,
        )?),
        Dialect::OpenAcc => DirectiveBody::OpenAcc(build_acc_directive(
            directive,
            parser_config,
            host_language,
        )?),
    };

    // Normalization plumbing will hook into clause lists in later steps.
    let _ = normalization;

    Ok(RoupDirective {
        language: language_tag,
        source: SourceLocation::default(),
        body,
    })
}

fn build_omp_directive(
    directive: &Directive<'_>,
    parser_config: &ParserConfig,
    host_language: Language,
) -> Result<OmpDirective, AstBuildError> {
    let kind = OmpDirectiveKind::try_from(directive.name.clone()).map_err(|name| {
        AstBuildError::UnsupportedDirective(format!("{name:?} not supported for OpenMP"))
    })?;

    let clause_config = parser_config.for_language(host_language);
    let clauses = directive
        .clauses
        .iter()
        .map(|clause| convert_clause_to_omp(clause, &clause_config))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(OmpDirective {
        kind,
        parameter: None, // Directive-specific payloads wired in a follow-up change.
        clauses,
    })
}

fn build_acc_directive(
    directive: &Directive<'_>,
    parser_config: &ParserConfig,
    host_language: Language,
) -> Result<AccDirective, AstBuildError> {
    let kind = AccDirectiveKind::try_from(directive.name.clone()).map_err(|name| {
        AstBuildError::UnsupportedDirective(format!("{name:?} not supported for OpenACC"))
    })?;

    let clause_config = parser_config.for_language(host_language);
    let clauses = directive
        .clauses
        .iter()
        .map(|clause| convert_clause_to_acc(clause, &clause_config))
        .collect::<Result<Vec<_>, _>>()?;

    let parameter = build_acc_directive_parameter(directive, kind, &clause_config)?;

    Ok(AccDirective {
        kind,
        parameter,
        clauses,
    })
}

fn build_acc_directive_parameter(
    directive: &Directive<'_>,
    kind: AccDirectiveKind,
    parser_config: &ParserConfig,
) -> Result<Option<AccDirectiveParameter>, AstBuildError> {
    if let Some(cache) = directive.cache_data.as_ref() {
        let variables = cache
            .variables
            .iter()
            .map(|name| Identifier::new(name.as_ref()))
            .collect();
        return Ok(Some(AccDirectiveParameter::Cache(AccCacheDirective {
            readonly: cache.readonly,
            variables,
        })));
    }

    if let Some(wait) = directive.wait_data.as_ref() {
        let devnum = wait
            .devnum
            .as_ref()
            .map(|expr| Expression::new(expr.as_ref(), parser_config));
        let queues = wait
            .queue_exprs
            .iter()
            .map(|expr| Expression::new(expr.as_ref(), parser_config))
            .collect();
        return Ok(Some(AccDirectiveParameter::Wait(AccWaitDirective {
            devnum,
            queues,
            explicit_queues: wait.has_queues,
        })));
    }

    if kind == AccDirectiveKind::Routine {
        if let Some(param) = directive.parameter.as_ref() {
            let inner = param
                .trim()
                .trim_start_matches('(')
                .trim_end_matches(')')
                .trim();
            let ident = if inner.is_empty() {
                None
            } else {
                Some(Identifier::new(inner))
            };
            return Ok(Some(AccDirectiveParameter::Routine(AccRoutineDirective {
                name: ident,
            })));
        }
    }

    if kind == AccDirectiveKind::End {
        if let Some(param) = directive.parameter.as_ref() {
            let canonical = lookup_directive_name(param.as_ref());
            if let Ok(acc_kind) = AccDirectiveKind::try_from(canonical.clone()) {
                return Ok(Some(AccDirectiveParameter::End(acc_kind)));
            }
        }
    }

    Ok(None)
}

fn convert_clause_to_omp(
    clause: &Clause<'_>,
    parser_config: &ParserConfig,
) -> Result<OmpClause, AstBuildError> {
    let clause_name = lookup_clause_name(clause.name.as_ref());
    let kind = OmpClauseKind::try_from(clause_name.clone())
        .map_err(|_| AstBuildError::UnsupportedClause(format!("{}", clause.name.as_ref())))?;

    let payload = parse_clause_data(clause, parser_config)
        .map_err(|err| AstBuildError::ClauseConversion(err.to_string()))?;

    Ok(OmpClause { kind, payload })
}

fn convert_clause_to_acc(
    clause: &Clause<'_>,
    parser_config: &ParserConfig,
) -> Result<AccClause, AstBuildError> {
    let clause_name = lookup_clause_name(clause.name.as_ref());
    let kind = AccClauseKind::try_from(clause_name.clone())
        .map_err(|_| AstBuildError::UnsupportedClause(format!("{}", clause.name.as_ref())))?;

    Ok(AccClause {
        kind,
        payload: build_acc_clause_payload(clause, clause_name, parser_config)?,
    })
}

fn build_acc_clause_payload(
    clause: &Clause<'_>,
    clause_name: ClauseName,
    parser_config: &ParserConfig,
) -> Result<AccClausePayload, AstBuildError> {
    use ClauseName::*;
    match clause_name {
        Copy | CopyIn | CopyOut => build_acc_copy_clause(clause, parser_config),
        Create => build_acc_create_clause(clause, parser_config),
        Reduction => build_acc_reduction_clause(clause, parser_config),
        Wait => build_acc_wait_clause(clause, parser_config),
        Vector => build_acc_vector_clause(clause, parser_config),
        Worker => build_acc_worker_clause(clause, parser_config),
        Attach => build_acc_data_clause(clause, AccDataKind::Attach),
        Detach => build_acc_data_clause(clause, AccDataKind::Detach),
        UseDevice => build_acc_data_clause(clause, AccDataKind::UseDevice),
        Link => build_acc_data_clause(clause, AccDataKind::Link),
        DeviceResident => build_acc_data_clause(clause, AccDataKind::DeviceResident),
        Host => build_acc_data_clause(clause, AccDataKind::Host),
        Device => build_acc_data_clause(clause, AccDataKind::Device),
        Delete => build_acc_data_clause(clause, AccDataKind::Delete),
        DeviceType => Ok(build_acc_device_type_clause(clause)),
        Async | Bind | Collapse | NumGangs | NumWorkers | VectorLength | Gang | Seq
        | Independent | Auto | DefaultAsync | NoCreate | NoHost | SelfClause | Tile | Finalize
        | IfPresent | DevicePtr | DeviceNum => Ok(build_identifier_list_payload(clause)),
        _ => Ok(build_fallback_clause_payload(clause, parser_config)),
    }
}

fn build_acc_copy_clause(
    clause: &Clause<'_>,
    _parser_config: &ParserConfig,
) -> Result<AccClausePayload, AstBuildError> {
    let keyword = clause.name.as_ref().to_ascii_lowercase();
    let kind = match keyword.as_str() {
        "copy" => AccCopyKind::Copy,
        "pcopy" => AccCopyKind::PCopy,
        "present_or_copy" => AccCopyKind::PresentOrCopy,
        "copyin" => AccCopyKind::CopyIn,
        "pcopyin" => AccCopyKind::PCopyIn,
        "present_or_copyin" => AccCopyKind::PresentOrCopyIn,
        "copyout" => AccCopyKind::CopyOut,
        "pcopyout" => AccCopyKind::PCopyOut,
        "present_or_copyout" => AccCopyKind::PresentOrCopyOut,
        other => {
            return Err(AstBuildError::UnsupportedClause(format!(
                "unknown OpenACC copy clause keyword: {other}"
            )))
        }
    };

    let (modifier, variables) = match &clause.kind {
        ClauseKind::CopyinClause {
            modifier,
            variables,
        } => (
            modifier.and_then(|m| {
                if matches!(m, CopyinModifier::Readonly) {
                    Some(AccCopyModifier::Readonly)
                } else {
                    None
                }
            }),
            variables
                .iter()
                .map(|item| Identifier::new(item.as_ref()))
                .collect(),
        ),
        ClauseKind::CopyoutClause {
            modifier,
            variables,
        } => (
            modifier.and_then(|m| {
                if matches!(m, CopyoutModifier::Zero) {
                    Some(AccCopyModifier::Zero)
                } else {
                    None
                }
            }),
            variables
                .iter()
                .map(|item| Identifier::new(item.as_ref()))
                .collect(),
        ),
        ClauseKind::VariableList(items) => (
            None,
            items
                .iter()
                .map(|item| Identifier::new(item.as_ref()))
                .collect(),
        ),
        _ => {
            return Err(AstBuildError::UnsupportedClause(
                "copy clause requires a variable list".to_string(),
            ))
        }
    };

    Ok(AccClausePayload::Copy(AccCopyClause {
        kind,
        modifier,
        variables,
    }))
}

fn build_acc_create_clause(
    clause: &Clause<'_>,
    _parser_config: &ParserConfig,
) -> Result<AccClausePayload, AstBuildError> {
    let keyword = clause.name.as_ref().to_ascii_lowercase();
    let kind = match keyword.as_str() {
        "create" => AccCreateKind::Create,
        "pcreate" => AccCreateKind::PCreate,
        "present_or_create" => AccCreateKind::PresentOrCreate,
        other => {
            return Err(AstBuildError::UnsupportedClause(format!(
                "unknown OpenACC create clause keyword: {other}"
            )))
        }
    };

    let (modifier, variables) = match &clause.kind {
        ClauseKind::CreateClause {
            modifier,
            variables,
        } => (
            modifier.and_then(|m| {
                if matches!(m, CreateModifier::Zero) {
                    Some(AccCreateModifier::Zero)
                } else {
                    None
                }
            }),
            variables
                .iter()
                .map(|item| Identifier::new(item.as_ref()))
                .collect(),
        ),
        ClauseKind::VariableList(items) => (
            None,
            items
                .iter()
                .map(|item| Identifier::new(item.as_ref()))
                .collect(),
        ),
        _ => {
            return Err(AstBuildError::UnsupportedClause(
                "create clause requires a variable list".to_string(),
            ))
        }
    };

    Ok(AccClausePayload::Create(AccCreateClause {
        kind,
        modifier,
        variables,
    }))
}

fn build_acc_reduction_clause(
    clause: &Clause<'_>,
    _parser_config: &ParserConfig,
) -> Result<AccClausePayload, AstBuildError> {
    if let ClauseKind::ReductionClause {
        operator,
        user_defined_identifier,
        variables,
        ..
    } = &clause.kind
    {
        let operator_text = match operator {
            ParserReductionOperator::Add => "+",
            ParserReductionOperator::Sub => "-",
            ParserReductionOperator::Mul => "*",
            ParserReductionOperator::Max => "max",
            ParserReductionOperator::Min => "min",
            ParserReductionOperator::BitAnd => "&",
            ParserReductionOperator::BitOr => "|",
            ParserReductionOperator::BitXor => "^",
            ParserReductionOperator::LogAnd => "&&",
            ParserReductionOperator::LogOr => "||",
            ParserReductionOperator::FortAnd => ".and.",
            ParserReductionOperator::FortOr => ".or.",
            ParserReductionOperator::FortEqv => ".eqv.",
            ParserReductionOperator::FortNeqv => ".neqv.",
            ParserReductionOperator::FortIand => "iand",
            ParserReductionOperator::FortIor => "ior",
            ParserReductionOperator::FortIeor => "ieor",
            ParserReductionOperator::UserDefined => {
                user_defined_identifier.as_deref().unwrap_or("user")
            }
        };

        let variables = variables
            .iter()
            .map(|item| Identifier::new(item.as_ref()))
            .collect();

        return Ok(AccClausePayload::Reduction(AccReductionClause {
            operator: operator_text.to_string(),
            variables,
        }));
    }

    Err(AstBuildError::UnsupportedClause(
        "reduction clause missing structured payload".to_string(),
    ))
}

fn build_acc_wait_clause(
    clause: &Clause<'_>,
    parser_config: &ParserConfig,
) -> Result<AccClausePayload, AstBuildError> {
    let content = match &clause.kind {
        ClauseKind::Parenthesized(text) => text.as_ref().to_string(),
        ClauseKind::VariableList(items) => join_variable_list(items),
        ClauseKind::Bare => String::new(),
        other => clause_content_from_kind(other)
            .unwrap_or_default()
            .into_owned(),
    };

    let (devnum, has_queues, expressions, parsed) = parse_wait_components(&content);
    let devnum_expr = devnum.map(|value| Expression::new(value.trim(), parser_config));
    let queue_exprs = expressions
        .into_iter()
        .map(|expr| Expression::new(expr.trim(), parser_config))
        .collect();

    if parsed {
        Ok(AccClausePayload::Wait(AccWaitClause {
            devnum: devnum_expr,
            queues: queue_exprs,
            explicit_queues: has_queues,
        }))
    } else {
        // Fallback: treat clause as a simple identifier list when parsing fails
        Ok(AccClausePayload::IdentifierList(clause_variable_list(
            &clause.kind,
        )))
    }
}

fn build_acc_vector_clause(
    clause: &Clause<'_>,
    parser_config: &ParserConfig,
) -> Result<AccClausePayload, AstBuildError> {
    if let ClauseKind::VectorClause {
        modifier,
        variables,
    } = &clause.kind
    {
        let label = modifier.map(|m| match m {
            VectorModifier::Length => "length".to_string(),
        });
        let values = variables
            .iter()
            .map(|value| Expression::new(value.as_ref(), parser_config))
            .collect();
        return Ok(AccClausePayload::Vector(AccGangClause {
            modifier: label,
            values,
        }));
    }

    Ok(AccClausePayload::IdentifierList(clause_variable_list(
        &clause.kind,
    )))
}

fn build_acc_worker_clause(
    clause: &Clause<'_>,
    parser_config: &ParserConfig,
) -> Result<AccClausePayload, AstBuildError> {
    if let ClauseKind::WorkerClause {
        modifier,
        variables,
    } = &clause.kind
    {
        let label = modifier.map(|m| match m {
            WorkerModifier::Num => "num".to_string(),
        });
        let values = variables
            .iter()
            .map(|value| Expression::new(value.as_ref(), parser_config))
            .collect();
        return Ok(AccClausePayload::Worker(AccGangClause {
            modifier: label,
            values,
        }));
    }

    Ok(AccClausePayload::IdentifierList(clause_variable_list(
        &clause.kind,
    )))
}

fn build_acc_data_clause(
    clause: &Clause<'_>,
    kind: AccDataKind,
) -> Result<AccClausePayload, AstBuildError> {
    Ok(AccClausePayload::Data(AccDataClause {
        kind,
        variables: clause_variable_list(&clause.kind),
    }))
}

fn build_acc_device_type_clause(clause: &Clause<'_>) -> AccClausePayload {
    let values = clause_variable_strings(&clause.kind);
    AccClausePayload::DeviceType(values)
}

fn build_identifier_list_payload(clause: &Clause<'_>) -> AccClausePayload {
    AccClausePayload::IdentifierList(clause_variable_list(&clause.kind))
}

fn build_fallback_clause_payload(
    clause: &Clause<'_>,
    parser_config: &ParserConfig,
) -> AccClausePayload {
    match &clause.kind {
        ClauseKind::Bare => AccClausePayload::Bare,
        ClauseKind::Parenthesized(content) => {
            AccClausePayload::Expression(Expression::new(content.as_ref().trim(), parser_config))
        }
        _ => build_identifier_list_payload(clause),
    }
}

fn clause_variable_list(kind: &ClauseKind<'_>) -> Vec<Identifier> {
    match kind {
        ClauseKind::VariableList(items)
        | ClauseKind::CopyinClause {
            variables: items, ..
        }
        | ClauseKind::CopyoutClause {
            variables: items, ..
        }
        | ClauseKind::CreateClause {
            variables: items, ..
        }
        | ClauseKind::GangClause {
            variables: items, ..
        }
        | ClauseKind::WorkerClause {
            variables: items, ..
        }
        | ClauseKind::VectorClause {
            variables: items, ..
        }
        | ClauseKind::ReductionClause {
            variables: items, ..
        } => items
            .iter()
            .map(|item| Identifier::new(item.as_ref()))
            .collect(),
        ClauseKind::Bare | ClauseKind::Parenthesized(_) => Vec::new(),
    }
}

fn clause_variable_strings(kind: &ClauseKind<'_>) -> Vec<String> {
    clause_variable_list(kind)
        .into_iter()
        .map(|identifier| identifier.to_string())
        .collect()
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
            ..
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

fn format_reduction_clause(
    operator: ParserReductionOperator,
    variables: &[Cow<'_, str>],
) -> String {
    let token = match operator {
        ParserReductionOperator::Add => "+",
        ParserReductionOperator::Sub => "-",
        ParserReductionOperator::Mul => "*",
        ParserReductionOperator::Max => "max",
        ParserReductionOperator::Min => "min",
        ParserReductionOperator::BitAnd => "&",
        ParserReductionOperator::BitOr => "|",
        ParserReductionOperator::BitXor => "^",
        ParserReductionOperator::LogAnd => "&&",
        ParserReductionOperator::LogOr => "||",
        ParserReductionOperator::FortAnd => "and",
        ParserReductionOperator::FortOr => "or",
        ParserReductionOperator::FortEqv => "eqv",
        ParserReductionOperator::FortNeqv => "neqv",
        ParserReductionOperator::FortIand => "iand",
        ParserReductionOperator::FortIor => "ior",
        ParserReductionOperator::FortIeor => "ieor",
        ParserReductionOperator::UserDefined => "user",
    };

    let joined = join_variable_list(variables);
    if token.is_empty() {
        joined
    } else if joined.is_empty() {
        token.to_string()
    } else {
        format!("{token}: {joined}")
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
        return None;
    }

    Some(rest)
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

fn normalize_directive<'a>(
    directive: &'a Directive<'a>,
    mode: ClauseNormalizationMode,
) -> Cow<'a, Directive<'a>> {
    match mode {
        ClauseNormalizationMode::Disabled => Cow::Borrowed(directive),
        ClauseNormalizationMode::MergeVariableLists | ClauseNormalizationMode::ParserParity => {
            let mut cloned = directive.clone();
            cloned.merge_clauses();
            Cow::Owned(cloned)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{
        clause::{Clause, ClauseKind},
        directive::Directive,
        directive_kind::DirectiveName,
        Dialect,
    };
    use std::borrow::Cow;

    #[test]
    fn builds_basic_openmp_ast() {
        let directive = Directive {
            name: DirectiveName::Parallel,
            parameter: None,
            clauses: vec![Clause {
                name: Cow::Borrowed("nowait"),
                kind: ClauseKind::Bare,
            }],
            wait_data: None,
            cache_data: None,
        };

        let config = ParserConfig::default().for_language(Language::C);
        let ast = build_roup_directive(
            &directive,
            Dialect::OpenMp,
            ClauseNormalizationMode::Disabled,
            &config,
            Language::C,
        )
        .expect("ast conversion should succeed");

        match ast.body {
            DirectiveBody::OpenMp(omp) => {
                assert!(matches!(omp.kind, OmpDirectiveKind::Parallel));
                assert_eq!(omp.clauses.len(), 1);
            }
            _ => panic!("expected OpenMP directive"),
        }
    }
}
