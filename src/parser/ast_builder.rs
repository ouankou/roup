use crate::ast::{
    AccCacheDirective, AccClause, AccClauseKind, AccClausePayload, AccCopyClause, AccCopyKind,
    AccCreateClause, AccCreateKind, AccDataClause, AccDataKind, AccDataModifier, AccDefaultKind,
    AccDeviceType, AccDirective, AccDirectiveKind, AccDirectiveParameter, AccGangClause,
    AccGangModifier, AccIndirectClause, AccReductionClause, AccReductionOperator,
    AccRoutineDirective, AccVectorClause, AccVectorModifier, AccWaitClause, AccWaitDirective,
    AccWorkerClause, AccWorkerModifier, ClauseNormalizationMode, DirectiveBody, OmpClause,
    OmpClauseKind, OmpConstructType, OmpDeclareMapper, OmpDeclareMapperId, OmpDeclareReduction,
    OmpDirective, OmpDirectiveKind, OmpDirectiveParameter, OmpSimdTarget, ReductionOperatorToken,
    RoupDirective, RoupLanguage,
};
use crate::ir::{
    convert::{parse_clause_data, parse_identifier_list},
    ClauseData, ClauseItem, Expression, Identifier, Language, ParserConfig, ReductionOperator,
    RequireModifier, SourceLocation,
};
use std::borrow::Cow;

use super::clause::{
    lookup_clause_name, parse_variable_list, Clause, ClauseKind, ClauseName, CopyinModifier,
    CopyoutModifier, CreateModifier, GangModifier, ReductionOperator as ParserReductionOperator,
    VectorModifier, WorkerModifier,
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

impl From<crate::ir::ConversionError> for AstBuildError {
    fn from(err: crate::ir::ConversionError) -> Self {
        AstBuildError::ParseFailure(err.to_string())
    }
}

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
    let directive_name = directive.name.clone();

    let kind = OmpDirectiveKind::try_from(directive_name).map_err(|name| {
        AstBuildError::UnsupportedDirective(format!("{name:?} not supported for OpenMP"))
    })?;

    if kind == OmpDirectiveKind::Requires {
        let requirements = build_requires_from_clauses(&directive.clauses, parser_config)?;
        let clause = OmpClause {
            kind: OmpClauseKind::Requires,
            payload: ClauseData::Requires { requirements },
            separator: crate::ast::OmpClauseSeparator::Space,
        };
        return Ok(OmpDirective {
            kind,
            parameter: None,
            clauses: vec![clause],
        });
    }

    let clause_config = parser_config.for_language(host_language);
    let clauses = directive
        .clauses
        .iter()
        .map(|clause| convert_clause_to_omp(clause, &clause_config))
        .collect::<Result<Vec<_>, _>>()?;

    validate_omp_directive(kind, &clauses, host_language)?;

    let parameter = build_omp_directive_parameter(directive, &clause_config)?;

    Ok(OmpDirective {
        kind,
        parameter,
        clauses,
    })
}

fn validate_omp_directive(
    kind: OmpDirectiveKind,
    clauses: &[OmpClause],
    host_language: Language,
) -> Result<(), AstBuildError> {
    if matches!(host_language, Language::Fortran) {
        if matches!(kind, OmpDirectiveKind::Do | OmpDirectiveKind::DoSimd)
            && clauses
                .iter()
                .any(|clause| matches!(clause.kind, OmpClauseKind::Nowait))
        {
            return Err(AstBuildError::ParseFailure(
                "Fortran DO directives accept NOWAIT only on the terminating directive".to_string(),
            ));
        }

        if matches!(kind, OmpDirectiveKind::EndDo | OmpDirectiveKind::EndDoSimd) {
            for clause in clauses {
                if !matches!(clause.kind, OmpClauseKind::Nowait) {
                    return Err(AstBuildError::ParseFailure(
                        "END DO only accepts a NOWAIT clause in Fortran".to_string(),
                    ));
                }
            }
        }
    }

    Ok(())
}

fn build_omp_directive_parameter(
    directive: &Directive<'_>,
    parser_config: &ParserConfig,
) -> Result<Option<OmpDirectiveParameter>, AstBuildError> {
    let param = match directive.parameter.as_ref() {
        Some(param) => param,
        None => return Ok(None),
    };
    let param_str = param.as_ref();
    use crate::parser::directive_kind::DirectiveName;

    match &directive.name {
        DirectiveName::Cancel | DirectiveName::CancellationPoint => {
            if std::env::var_os("ROUP_DEBUG_CONSTRUCT").is_some() {
                eprintln!(
                    "[ast] construct param directive={} value={param_str}",
                    directive.name.as_ref()
                );
            }
            let construct = match param_str.trim().to_ascii_lowercase().as_str() {
                "parallel" => OmpConstructType::Parallel,
                "sections" => OmpConstructType::Sections,
                "for" | "do" => OmpConstructType::For,
                "taskgroup" => OmpConstructType::Taskgroup,
                other => {
                    return Err(AstBuildError::ParseFailure(format!(
                        "unknown cancel construct: {other}"
                    )))
                }
            };
            return Ok(Some(OmpDirectiveParameter::Construct(construct)));
        }
        DirectiveName::Critical => {
            let cleaned = strip_wrapping_parens(param_str);
            if std::env::var_os("ROUP_DEBUG_CONSTRUCT").is_some() {
                eprintln!("[ast] critical param value={param_str} cleaned={cleaned}");
            }
            return Ok(Some(OmpDirectiveParameter::CriticalSection(
                Identifier::new(cleaned),
            )));
        }
        DirectiveName::Depobj => {
            let list = parse_identifier_list_parameter(param_str, parser_config)?;
            let first = list.first().ok_or_else(|| {
                AstBuildError::ParseFailure("depobj requires a target identifier".to_string())
            })?;
            return Ok(Some(OmpDirectiveParameter::Depobj(first.clone())));
        }
        DirectiveName::Flush => {
            if param_str.trim().is_empty() || strip_wrapping_parens(param_str).is_empty() {
                return Ok(None);
            }
            let list = parse_identifier_list_parameter(param_str, parser_config)?;
            return Ok(Some(OmpDirectiveParameter::FlushList(list)));
        }
        DirectiveName::DeclareSimd => {
            return Ok(Some(OmpDirectiveParameter::DeclareSimd(
                parse_declare_simd_target(param_str),
            )));
        }
        DirectiveName::DeclareMapper => {
            let mapper = parse_declare_mapper_param(param_str, parser_config).ok_or_else(|| {
                AstBuildError::ParseFailure("declare mapper parameter is invalid".to_string())
            })?;
            return Ok(Some(OmpDirectiveParameter::DeclareMapper(mapper)));
        }
        DirectiveName::DeclareReduction => {
            let reduction =
                parse_declare_reduction_param(param_str, parser_config).ok_or_else(|| {
                    AstBuildError::ParseFailure(
                        "declare reduction parameter is invalid".to_string(),
                    )
                })?;
            return Ok(Some(OmpDirectiveParameter::DeclareReduction(reduction)));
        }
        DirectiveName::DeclareVariant | DirectiveName::BeginDeclareVariant => {
            let cleaned = strip_wrapping_parens(param_str);
            if cleaned.is_empty() {
                return Err(AstBuildError::ParseFailure(
                    "declare variant requires a variant function".to_string(),
                ));
            }
            return Ok(Some(OmpDirectiveParameter::VariantFunction(
                Identifier::new(cleaned),
            )));
        }
        _ => {}
    }

    if directive_expects_identifier_list(&directive.name) {
        let list = parse_identifier_list_parameter(param_str, parser_config)?;
        return Ok(Some(OmpDirectiveParameter::IdentifierList(list)));
    }

    Ok(Some(OmpDirectiveParameter::Identifier(Identifier::new(
        param_str,
    ))))
}

fn directive_expects_identifier_list(name: &crate::parser::directive_kind::DirectiveName) -> bool {
    use crate::parser::directive_kind::DirectiveName;
    matches!(
        name,
        DirectiveName::Allocate | DirectiveName::Threadprivate | DirectiveName::Groupprivate
    )
}

fn parse_identifier_list_parameter(
    raw: &str,
    parser_config: &ParserConfig,
) -> Result<Vec<Identifier>, AstBuildError> {
    let trimmed = raw.trim();
    if !(trimmed.starts_with('(') && trimmed.ends_with(')')) {
        return Err(AstBuildError::ParseFailure(
            "expected a parenthesized identifier list".to_string(),
        ));
    }
    let content = &trimmed[1..trimmed.len() - 1];
    let items = parse_identifier_list(content, parser_config)?;
    if items.is_empty() {
        return Err(AstBuildError::ParseFailure(
            "identifier list cannot be empty".to_string(),
        ));
    }
    Ok(items
        .into_iter()
        .map(clause_item_to_identifier)
        .collect::<Vec<_>>())
}

fn strip_wrapping_parens(raw: &str) -> &str {
    let trimmed = raw.trim();
    if trimmed.starts_with('(') && trimmed.ends_with(')') && trimmed.len() > 1 {
        trimmed[1..trimmed.len() - 1].trim()
    } else {
        trimmed
    }
}

fn parse_declare_simd_target(raw: &str) -> OmpSimdTarget {
    let inner = strip_wrapping_parens(raw);

    let function = if inner.is_empty() {
        None
    } else {
        Some(Identifier::new(inner))
    };

    OmpSimdTarget { function }
}

fn parse_declare_mapper_param(raw: &str, parser_config: &ParserConfig) -> Option<OmpDeclareMapper> {
    let _ = parser_config;
    let inner = strip_wrapping_parens(raw);

    let mut mapper_id: Option<&str> = None;
    let mut rest: &str = inner;

    // Find a colon that is NOT part of a Fortran-style '::'
    if let Some(pos) = inner.find(':') {
        let bytes = inner.as_bytes();
        if bytes.get(pos + 1).copied() != Some(b':') {
            mapper_id = Some(inner[..pos].trim());
            rest = inner[pos + 1..].trim();
        }
    }

    // If we didn't parse a mapper id, everything is the type/variable portion
    if mapper_id.is_none_or(|id| id.is_empty()) {
        mapper_id = None;
        rest = inner.trim();
    }

    // Split remaining portion into type and variable
    let (type_part, var_part) = if let Some(pos) = rest.find("::") {
        (
            rest[..pos].trim().to_string(),
            rest[pos + 2..].trim().to_string(),
        )
    } else {
        let mut pieces = rest.split_whitespace().collect::<Vec<_>>();
        if pieces.is_empty() {
            return None;
        }
        let var = pieces.pop().unwrap().trim().to_string();
        let ty = pieces.join(" ");
        (ty.trim().to_string(), var)
    };

    if type_part.is_empty() || var_part.is_empty() {
        return None;
    }

    let identifier = match mapper_id {
        Some(id) if id.eq_ignore_ascii_case("default") || id.is_empty() => {
            OmpDeclareMapperId::Default
        }
        Some(id) => OmpDeclareMapperId::User(Identifier::new(id)),
        None => OmpDeclareMapperId::Unspecified,
    };

    Some(OmpDeclareMapper {
        identifier,
        type_name: type_part,
        variable: var_part,
    })
}

fn parse_declare_reduction_param(
    raw: &str,
    parser_config: &ParserConfig,
) -> Option<OmpDeclareReduction> {
    let _ = parser_config;
    let trimmed = raw.trim();
    let debug = std::env::var_os("ROUP_DEBUG_DECLARE_REDUCTION").is_some();
    if debug {
        eprintln!("[declare_reduction] raw=\"{trimmed}\"");
    }

    // Extract the parenthesized portion "(op : types [: combiner])"
    let open = trimmed.find('(')?;
    let mut depth: i32 = 0;
    let mut close: Option<usize> = None;
    for (idx, ch) in trimmed.char_indices().skip(open) {
        match ch {
            '(' => depth += 1,
            ')' => {
                if depth == 0 {
                    return None;
                }
                depth -= 1;
                if depth == 0 {
                    close = Some(idx);
                    break;
                }
            }
            _ => {}
        }
    }
    let close = close?;
    let inner = trimmed[open + 1..close].trim();
    let mut remainder = trimmed[close + 1..].trim_start().to_string();
    if debug {
        eprintln!("[declare_reduction] inner=\"{inner}\" remainder=\"{remainder}\"");
    }

    // Find separator colons that are not part of '::'
    let mut first_sep: Option<usize> = None;
    let mut second_sep: Option<usize> = None;
    let bytes = inner.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] != b':' {
            continue;
        }
        if (i > 0 && bytes[i - 1] == b':') || (i + 1 < bytes.len() && bytes[i + 1] == b':') {
            continue;
        }
        if first_sep.is_none() {
            first_sep = Some(i);
        } else {
            second_sep = Some(i);
            break;
        }
    }

    let (op_part, types_part, embedded_combiner) = match (first_sep, second_sep) {
        (Some(s1), Some(s2)) if s1 < s2 && s2 < inner.len() => {
            let op = inner[..s1].trim();
            let types = inner[s1 + 1..s2].trim();
            let comb = inner[s2 + 1..].trim();
            (op, types, if comb.is_empty() { None } else { Some(comb) })
        }
        (Some(s1), None) if s1 + 1 < inner.len() => {
            let op = inner[..s1].trim();
            let types = inner[s1 + 1..].trim();
            (op, types, None)
        }
        _ => return None,
    };

    if op_part.is_empty() || types_part.is_empty() {
        return None;
    }

    // Parse optional combiner/initializer keywords that may follow the parenthesized portion.
    let mut combiner: Option<String> = embedded_combiner.map(|c| c.to_string());
    let mut combiner_from_clause = false;
    let mut initializer: Option<String> = None;
    let extract_paren_payload = |text: &str| -> Option<(String, String)> {
        let trimmed = text.trim_start();
        let open = trimmed.find('(')?;
        let mut depth: i32 = 0;
        let mut close_idx: Option<usize> = None;
        for (offset, ch) in trimmed[open..].char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    if depth == 0 {
                        return None;
                    }
                    depth -= 1;
                    if depth == 0 {
                        close_idx = Some(open + offset);
                        break;
                    }
                }
                _ => {}
            }
        }
        let close = close_idx?;
        let payload = trimmed[open + 1..close].to_string();
        let rest = trimmed[close + 1..].trim_start().to_string();
        Some((payload, rest))
    };

    while !remainder.is_empty() {
        if let Some(rest) = remainder.strip_prefix("combiner") {
            if let Some((payload, after)) = extract_paren_payload(rest) {
                combiner = Some(payload.trim().to_string());
                combiner_from_clause = true;
                remainder = after;
                continue;
            } else {
                return None;
            }
        }
        if let Some(rest) = remainder.strip_prefix("initializer") {
            if let Some((payload, after)) = extract_paren_payload(rest) {
                initializer = Some(payload.trim().to_string());
                remainder = after;
                continue;
            } else {
                return None;
            }
        }
        break;
    }

    let operator = match op_part {
        "+" => ReductionOperatorToken::Builtin(ReductionOperator::Add),
        "-" => ReductionOperatorToken::Builtin(ReductionOperator::Subtract),
        "*" => ReductionOperatorToken::Builtin(ReductionOperator::Multiply),
        "&" => ReductionOperatorToken::Builtin(ReductionOperator::BitwiseAnd),
        "|" => ReductionOperatorToken::Builtin(ReductionOperator::BitwiseOr),
        "^" => ReductionOperatorToken::Builtin(ReductionOperator::BitwiseXor),
        "&&" => ReductionOperatorToken::Builtin(ReductionOperator::LogicalAnd),
        "||" => ReductionOperatorToken::Builtin(ReductionOperator::LogicalOr),
        "min" => ReductionOperatorToken::Builtin(ReductionOperator::Min),
        "max" => ReductionOperatorToken::Builtin(ReductionOperator::Max),
        other => ReductionOperatorToken::Identifier(Identifier::new(other)),
    };

    let type_names = types_part
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    if debug {
        eprintln!(
            "[declare_reduction] op={op_part} types={type_names:?} combiner={combiner:?} init={initializer:?}"
        );
    }

    if type_names.is_empty() || combiner.is_none() {
        return None;
    }

    Some(OmpDeclareReduction {
        operator,
        type_names,
        combiner: combiner.unwrap(),
        initializer,
        combiner_from_clause,
    })
}

fn clause_item_to_identifier(item: ClauseItem) -> Identifier {
    match item {
        ClauseItem::Identifier(id) => id,
        ClauseItem::Variable(var) => Identifier::new(var.to_string()),
        ClauseItem::Expression(expr) => Identifier::new(expr.as_str()),
    }
}

fn build_requires_from_clauses(
    clauses: &[Clause<'_>],
    parser_config: &ParserConfig,
) -> Result<Vec<RequireModifier>, AstBuildError> {
    let mut requirements = Vec::new();
    for clause in clauses {
        let clause_name = lookup_clause_name(clause.name.as_ref());
        if std::env::var_os("ROUP_DEBUG_REQ").is_some() {
            eprintln!(
                "requires clause token: {} -> {:?}",
                clause.name.as_ref(),
                clause_name
            );
        }
        match clause_name {
            ClauseName::Requires => {
                let payload = parse_clause_data(clause, parser_config)
                    .map_err(|err| AstBuildError::ClauseConversion(err.to_string()))?;
                if let ClauseData::Requires { requirements: reqs } = payload {
                    requirements.extend(reqs);
                }
            }
            ClauseName::ReverseOffload => requirements.push(RequireModifier::ReverseOffload),
            ClauseName::UnifiedAddress => requirements.push(RequireModifier::UnifiedAddress),
            ClauseName::UnifiedSharedMemory => {
                requirements.push(RequireModifier::UnifiedSharedMemory)
            }
            ClauseName::DynamicAllocators => requirements.push(RequireModifier::DynamicAllocators),
            ClauseName::SelfMaps => requirements.push(RequireModifier::SelfMaps),
            ClauseName::AtomicDefaultMemOrder => {
                let payload = parse_clause_data(clause, parser_config)
                    .map_err(|err| AstBuildError::ClauseConversion(err.to_string()))?;
                match payload {
                    ClauseData::AtomicDefaultMemOrder(order) => {
                        requirements.push(RequireModifier::AtomicDefaultMemOrder(order))
                    }
                    _ => {
                        return Err(AstBuildError::ClauseConversion(
                            "invalid atomic_default_mem_order payload".to_string(),
                        ))
                    }
                }
            }
            ClauseName::ExtImplementationDefinedRequirement => {
                let value = match &clause.kind {
                    ClauseKind::Parenthesized(content) => {
                        let trimmed = content.as_ref().trim();
                        (!trimmed.is_empty()).then(|| Identifier::new(trimmed))
                    }
                    ClauseKind::VariableList(items) => items
                        .first()
                        .map(|item| item.as_ref().trim())
                        .filter(|item| !item.is_empty())
                        .map(Identifier::new),
                    ClauseKind::Bare => None,
                    _ => {
                        return Err(AstBuildError::ClauseConversion(
                            "invalid ext_implementation_defined_requirement payload".to_string(),
                        ))
                    }
                };
                requirements.push(RequireModifier::ExtImplementationDefinedRequirement(value))
            }
            ClauseName::Other(name) => {
                requirements.push(RequireModifier::ExtImplementationDefinedRequirement(Some(
                    Identifier::new(name.as_ref()),
                )));
            }
            _ => {
                return Err(AstBuildError::UnsupportedClause(
                    clause.name.as_ref().to_string(),
                ))
            }
        }
    }

    if requirements.is_empty() {
        return Err(AstBuildError::ClauseConversion(
            "requires clause must specify at least one requirement".to_string(),
        ));
    }

    Ok(requirements)
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
        .map_err(|_| AstBuildError::UnsupportedClause(clause.name.as_ref().to_string()))?;

    let payload = parse_clause_data(clause, parser_config)
        .map_err(|err| AstBuildError::ClauseConversion(err.to_string()))?;

    Ok(OmpClause {
        kind,
        payload,
        separator: clause.separator,
    })
}

fn convert_clause_to_acc(
    clause: &Clause<'_>,
    parser_config: &ParserConfig,
) -> Result<AccClause, AstBuildError> {
    let clause_name = lookup_clause_name(clause.name.as_ref());
    let kind = AccClauseKind::try_from(clause_name.clone())
        .map_err(|_| AstBuildError::UnsupportedClause(clause.name.as_ref().to_string()))?;

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
        Default => build_acc_default_clause(clause),
        Wait => build_acc_wait_clause(clause, parser_config),
        Vector => build_acc_vector_clause(clause, parser_config),
        Worker => build_acc_worker_clause(clause, parser_config),
        Gang => build_acc_gang_clause(clause, parser_config),
        Attach => build_acc_data_clause(clause, AccDataKind::Attach),
        Detach => build_acc_data_clause(clause, AccDataKind::Detach),
        UseDevice => build_acc_data_clause(clause, AccDataKind::UseDevice),
        Link => build_acc_data_clause(clause, AccDataKind::Link),
        DeviceResident => build_acc_data_clause(clause, AccDataKind::DeviceResident),
        Host => build_acc_data_clause(clause, AccDataKind::Host),
        Device => build_acc_data_clause(clause, AccDataKind::Device),
        Delete => build_acc_data_clause(clause, AccDataKind::Delete),
        DeviceType => build_acc_device_type_clause(clause),
        Indirect => Ok(build_acc_indirect_clause(clause)),
        SelfClause => Ok(build_acc_self_clause(clause, parser_config)),
        Async | Bind | Collapse | NumGangs | NumWorkers | VectorLength | Seq | Independent
        | Auto | DefaultAsync | NoCreate | NoHost | Tile | Finalize | IfPresent | DevicePtr
        | DeviceNum => Ok(build_fallback_clause_payload(clause, parser_config)),
        _ => Ok(build_fallback_clause_payload(clause, parser_config)),
    }
}

fn build_acc_default_clause(clause: &Clause<'_>) -> Result<AccClausePayload, AstBuildError> {
    let value = match &clause.kind {
        ClauseKind::Parenthesized(content) => content.as_ref().trim(),
        ClauseKind::VariableList(items) => items
            .first()
            .map(|item| item.as_ref().trim())
            .unwrap_or_default(),
        ClauseKind::Bare => "",
        _ => "",
    };

    let kind = match value.to_ascii_lowercase().as_str() {
        "none" => AccDefaultKind::None,
        "present" => AccDefaultKind::Present,
        _ => AccDefaultKind::Unspecified,
    };

    Ok(AccClausePayload::Default(kind))
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

    let (modifiers, variables) = match &clause.kind {
        ClauseKind::CopyinClause {
            modifier,
            variables,
        } => (
            modifier
                .and_then(|m| {
                    matches!(m, CopyinModifier::Readonly).then_some(AccDataModifier::Readonly)
                })
                .into_iter()
                .collect(),
            variables
                .iter()
                .map(|item| Identifier::new(item.as_ref()))
                .collect(),
        ),
        ClauseKind::CopyoutClause {
            modifier,
            variables,
        } => (
            modifier
                .and_then(|m| matches!(m, CopyoutModifier::Zero).then_some(AccDataModifier::Zero))
                .into_iter()
                .collect(),
            variables
                .iter()
                .map(|item| Identifier::new(item.as_ref()))
                .collect(),
        ),
        ClauseKind::VariableList(items) => (
            Vec::new(),
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
        modifiers,
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

    let (modifiers, variables) = match &clause.kind {
        ClauseKind::CreateClause {
            modifier,
            variables,
        } => (
            modifier
                .and_then(|m| matches!(m, CreateModifier::Zero).then_some(AccDataModifier::Zero))
                .into_iter()
                .collect(),
            variables
                .iter()
                .map(|item| Identifier::new(item.as_ref()))
                .collect(),
        ),
        ClauseKind::VariableList(items) => (
            Vec::new(),
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
        modifiers,
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
        let op = match operator {
            ParserReductionOperator::Add => AccReductionOperator::Add,
            ParserReductionOperator::Sub => AccReductionOperator::Sub,
            ParserReductionOperator::Mul => AccReductionOperator::Mul,
            ParserReductionOperator::Max => AccReductionOperator::Max,
            ParserReductionOperator::Min => AccReductionOperator::Min,
            ParserReductionOperator::BitAnd => AccReductionOperator::BitAnd,
            ParserReductionOperator::BitOr => AccReductionOperator::BitOr,
            ParserReductionOperator::BitXor => AccReductionOperator::BitXor,
            ParserReductionOperator::LogAnd => AccReductionOperator::LogAnd,
            ParserReductionOperator::LogOr => AccReductionOperator::LogOr,
            ParserReductionOperator::FortAnd => AccReductionOperator::FortAnd,
            ParserReductionOperator::FortOr => AccReductionOperator::FortOr,
            ParserReductionOperator::FortEqv => AccReductionOperator::FortEqv,
            ParserReductionOperator::FortNeqv => AccReductionOperator::FortNeqv,
            ParserReductionOperator::FortIand => AccReductionOperator::FortIand,
            ParserReductionOperator::FortIor => AccReductionOperator::FortIor,
            ParserReductionOperator::FortIeor => AccReductionOperator::FortIeor,
            ParserReductionOperator::UserDefined => {
                let identifier = user_defined_identifier.as_ref().ok_or_else(|| {
                    AstBuildError::ClauseConversion(
                        "user-defined OpenACC reduction missing operator identifier".to_string(),
                    )
                })?;
                AccReductionOperator::UserDefined(Identifier::new(identifier.as_ref()))
            }
        };

        let variables = variables
            .iter()
            .map(|item| Identifier::new(item.as_ref()))
            .collect();

        return Ok(AccClausePayload::Reduction(AccReductionClause {
            operator: op,
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
        let modifier = match modifier {
            Some(VectorModifier::Length) => Some(AccVectorModifier::Length),
            None if !variables.is_empty() => Some(AccVectorModifier::ExprOnly),
            _ => None,
        };
        let values = variables
            .iter()
            .map(|value| Expression::new(value.as_ref(), parser_config))
            .collect();
        return Ok(AccClausePayload::Vector(AccVectorClause {
            modifier,
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
        let modifier = match modifier {
            Some(WorkerModifier::Num) => Some(AccWorkerModifier::Num),
            None if !variables.is_empty() => Some(AccWorkerModifier::ExprOnly),
            _ => None,
        };
        let values = variables
            .iter()
            .map(|value| Expression::new(value.as_ref(), parser_config))
            .collect();
        return Ok(AccClausePayload::Worker(AccWorkerClause {
            modifier,
            values,
        }));
    }

    Ok(AccClausePayload::IdentifierList(clause_variable_list(
        &clause.kind,
    )))
}

fn build_acc_gang_clause(
    clause: &Clause<'_>,
    parser_config: &ParserConfig,
) -> Result<AccClausePayload, AstBuildError> {
    if let ClauseKind::GangClause {
        modifier,
        variables,
        ..
    } = &clause.kind
    {
        let modifier = modifier.map(|m| match m {
            GangModifier::Num => AccGangModifier::Num,
            GangModifier::Static => AccGangModifier::Static,
            GangModifier::Dim => AccGangModifier::Dim,
        });
        let values = variables
            .iter()
            .map(|value| Expression::new(value.as_ref(), parser_config))
            .collect();
        return Ok(AccClausePayload::Gang(AccGangClause { modifier, values }));
    }

    Ok(build_identifier_list_payload(clause))
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

fn build_acc_device_type_clause(clause: &Clause<'_>) -> Result<AccClausePayload, AstBuildError> {
    let values = clause_variable_strings(&clause.kind)
        .into_iter()
        .map(|value| match value.trim().to_ascii_lowercase().as_str() {
            "host" => AccDeviceType::Host,
            "any" => AccDeviceType::Any,
            "multicore" => AccDeviceType::Multicore,
            "default" => AccDeviceType::Default,
            _ => AccDeviceType::Named(Identifier::new(value.trim())),
        })
        .collect();
    Ok(AccClausePayload::DeviceType(values))
}

fn build_acc_indirect_clause(clause: &Clause<'_>) -> AccClausePayload {
    match &clause.kind {
        ClauseKind::Bare => AccClausePayload::Indirect(AccIndirectClause {
            value: None,
            is_string_literal: false,
        }),
        ClauseKind::Parenthesized(content) => {
            let trimmed = content.as_ref().trim();
            if trimmed.is_empty() {
                return AccClausePayload::Indirect(AccIndirectClause {
                    value: None,
                    is_string_literal: false,
                });
            }

            if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
                return AccClausePayload::Indirect(AccIndirectClause {
                    value: Some(trimmed[1..trimmed.len() - 1].to_string()),
                    is_string_literal: true,
                });
            }

            AccClausePayload::Indirect(AccIndirectClause {
                value: Some(trimmed.to_string()),
                is_string_literal: false,
            })
        }
        other => {
            let raw = clause_content_from_kind(other)
                .unwrap_or_default()
                .into_owned();
            let trimmed = raw.trim();
            let value = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            };
            AccClausePayload::Indirect(AccIndirectClause {
                value,
                is_string_literal: false,
            })
        }
    }
}

fn build_identifier_list_payload(clause: &Clause<'_>) -> AccClausePayload {
    AccClausePayload::IdentifierList(clause_variable_list(&clause.kind))
}

fn build_acc_self_clause(clause: &Clause<'_>, parser_config: &ParserConfig) -> AccClausePayload {
    match &clause.kind {
        ClauseKind::Bare => AccClausePayload::Bare,
        ClauseKind::Parenthesized(content) => {
            let trimmed = content.as_ref().trim();
            if trimmed.is_empty() {
                return AccClausePayload::Bare;
            }
            let vars = parse_variable_list(trimmed);
            if vars.len() > 1 {
                AccClausePayload::IdentifierList(
                    vars.into_iter()
                        .map(|v| Identifier::new(v.as_ref()))
                        .collect(),
                )
            } else {
                AccClausePayload::Expression(Expression::new(trimmed, parser_config))
            }
        }
        ClauseKind::VariableList(items) => AccClausePayload::IdentifierList(
            items
                .iter()
                .map(|item| Identifier::new(item.as_ref()))
                .collect(),
        ),
        _ => build_identifier_list_payload(clause),
    }
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
            space_after_colon,
            variables,
        } => Some(Cow::Owned(format_gang_clause(
            *modifier,
            *space_after_colon,
            variables,
        ))),
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

fn format_gang_clause(
    modifier: Option<GangModifier>,
    space_after_colon: bool,
    variables: &[Cow<'_, str>],
) -> String {
    let joined = join_variable_list(variables);
    let Some(modifier) = modifier else {
        return joined;
    };

    let prefix = match modifier {
        GangModifier::Num => "num",
        GangModifier::Static => "static",
        GangModifier::Dim => "dim",
    };

    if joined.is_empty() {
        prefix.to_string()
    } else if space_after_colon {
        format!("{prefix}: {joined}")
    } else {
        format!("{prefix}:{joined}")
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
                separator: crate::parser::ClauseSeparator::Space,
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

    #[test]
    fn parses_reduction_directive() {
        let parser = crate::parser::openmp::parser();
        let result = parser.parse_ast(
            "#pragma omp parallel reduction(+:sum)",
            ClauseNormalizationMode::ParserParity,
            &ParserConfig::default(),
        );
        assert!(result.is_ok(), "reduction parse failed: {:?}", result.err());
    }

    #[test]
    fn parses_named_openacc_device_type_as_identifier() {
        let parser = crate::parser::openacc::parser();
        let ast = parser
            .parse_ast(
                "#pragma acc parallel device_type(nvidia, host)",
                ClauseNormalizationMode::Disabled,
                &ParserConfig::default(),
            )
            .expect("OpenACC AST conversion should succeed");

        let DirectiveBody::OpenAcc(acc) = ast.body else {
            panic!("expected OpenACC directive");
        };
        let AccClausePayload::DeviceType(values) = &acc.clauses[0].payload else {
            panic!("expected device_type payload");
        };

        assert_eq!(values.len(), 2);
        assert!(matches!(
            &values[0],
            AccDeviceType::Named(name) if name.as_str() == "nvidia"
        ));
        assert_eq!(values[1], AccDeviceType::Host);
    }

    #[test]
    fn parses_declare_reduction_parameter_payload() {
        let config = ParserConfig::default();
        let input = "(min : struct point) combiner(minproc(&omp_out, &omp_in)) initializer(omp_priv = { 1, 2 })";
        let parsed =
            parse_declare_reduction_param(input, &config).expect("declare reduction should parse");
        match parsed.operator {
            ReductionOperatorToken::Builtin(ReductionOperator::Min) => {}
            other => panic!("unexpected operator: {other:?}"),
        }
        assert_eq!(parsed.type_names, vec!["struct point".to_string()]);
        assert_eq!(parsed.combiner, "minproc(&omp_out, &omp_in)");
        assert_eq!(parsed.initializer.as_deref(), Some("omp_priv = { 1, 2 }"));
    }
}
