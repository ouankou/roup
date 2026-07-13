use super::clause::LocatedClause;
use super::clause::{Clause, ClauseKind, ClauseName, lookup_clause_name};
use super::directive::Directive;
use super::semantic::{
    parse_acc_identifier_list, parse_clause_data, parse_directive_name_modifier,
    parse_identifier_list, parse_memory_order,
};
use super::{Dialect, LocatedDirective};
use crate::ast::{
    AccBindTarget, AccCacheDirective, AccCacheItem, AccClause, AccClauseKind, AccClausePayload,
    AccClauseSourceAlias, AccCollapseClause, AccCopyClause, AccCopyKind, AccCreateClause,
    AccCreateKind, AccDataClause, AccDataKind, AccDataModifier, AccDefaultKind, AccDeviceType,
    AccDirective, AccDirectiveKind, AccDirectiveParameter, AccEndKind, AccGangArgument,
    AccGangClause, AccReductionClause, AccReductionOperator, AccRoutineDirective,
    AccSizeExpression, AccVectorClause, AccWaitClause, AccWaitDirective, AccWorkerClause,
    OmpClause, OmpClauseKind, OmpClauseSourceAlias, OmpConstructType, OmpCppOperatorFunctionId,
    OmpCppOperatorQualifier, OmpCppReductionOperator, OmpCppTemplateId, OmpDeclareInduction,
    OmpDeclareMapper, OmpDeclareReduction, OmpDeclareReductionSyntax, OmpDeclareTargetListItem,
    OmpDeclareVariantTarget, OmpDirective, OmpDirectiveKind, OmpDirectiveParameter,
    OmpDirectiveSourceAlias, OmpFlushListItem, OmpFortranAssignment, OmpFortranReductionIntrinsic,
    OmpFunctionName, OmpIdExpression, OmpInductionIdentifier, OmpInductionTypeSpecifier,
    OmpInductorExpression, OmpInitializerValue, OmpMapperId, OmpReductionCombiner,
    OmpReductionIdentifier, OmpReductionInitializer, OmpSimdTarget, OmpStorageListItem,
    OmpxPayload, OmpxPayloadItem, RoupDirective,
};
use crate::host::{ExprKind, Literal, QualifiedName, TokenKind, TypeName};
use crate::ir::{
    ClauseData, ClauseItem, Expression, FirstprivateModifier, Identifier, LValue, ParserConfig,
    ProcBind, Variable, lang,
};
use crate::lexer::{LogicalSource, LogicalSourceError};
use crate::version::{HostLanguage, OpenMpVersion, VersionPolicy};
use std::borrow::Cow;

fn parse_type_name(
    source: &str,
    parser_config: &ParserConfig,
) -> Result<TypeName, crate::host::TypeNameError> {
    if parser_config.source_extensions() {
        TypeName::parse_extension_with_profile(source, parser_config.profile())
    } else {
        TypeName::parse_with_profile(source, parser_config.profile())
    }
}

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

impl From<crate::ir::ExpressionError> for AstBuildError {
    fn from(error: crate::ir::ExpressionError) -> Self {
        AstBuildError::ClauseConversion(error.to_string())
    }
}

impl From<crate::ir::IdentifierError> for AstBuildError {
    fn from(error: crate::ir::IdentifierError) -> Self {
        AstBuildError::ClauseConversion(error.to_string())
    }
}

impl From<crate::host::TypeNameError> for AstBuildError {
    fn from(error: crate::host::TypeNameError) -> Self {
        AstBuildError::ClauseConversion(error.to_string())
    }
}

impl From<crate::ast::AstInvariantError> for AstBuildError {
    fn from(error: crate::ast::AstInvariantError) -> Self {
        AstBuildError::ClauseConversion(error.to_string())
    }
}

impl From<LogicalSourceError> for AstBuildError {
    fn from(error: LogicalSourceError) -> Self {
        AstBuildError::ParseFailure(error.to_string())
    }
}

pub(crate) fn build_ompx_directive(
    payload_source: &str,
    sentinel_source: &str,
    parser_config: &ParserConfig,
    source: &LogicalSource<'_>,
) -> Result<RoupDirective, AstBuildError> {
    let mut remaining = payload_source.trim();
    let mut items = Vec::new();
    while !remaining.is_empty() {
        let mut characters = remaining.char_indices();
        let Some((_, first)) = characters.next() else {
            break;
        };
        if !(first == '_' || first.is_alphabetic()) {
            return Err(AstBuildError::ParseFailure(
                "OMPX payload items must start with an identifier".to_string(),
            ));
        }
        let name_end = characters
            .find_map(|(index, character)| {
                (!(character == '_' || character.is_alphanumeric())).then_some(index)
            })
            .unwrap_or(remaining.len());
        let name = Identifier::new(&remaining[..name_end])?;
        let after_name = &remaining[name_end..];
        let after_trivia = after_name.trim_start();
        if after_trivia.starts_with('(') {
            let Some(close) = lang::find_matching_delimiter(after_trivia, 0, '(', ')')? else {
                return Err(AstBuildError::ParseFailure(
                    "OMPX invocation has an unclosed argument list".to_string(),
                ));
            };
            let arguments_source = after_trivia[1..close].trim();
            let arguments = if arguments_source.is_empty() {
                Vec::new()
            } else {
                super::semantic::split_top_level_items(arguments_source)?
                    .into_iter()
                    .map(|argument| Expression::new(argument, parser_config))
                    .collect::<Result<Vec<_>, _>>()?
            };
            items.push(OmpxPayloadItem::Invocation { name, arguments });
            let after_call = &after_trivia[close + 1..];
            if !after_call.is_empty() && !after_call.chars().next().is_some_and(char::is_whitespace)
            {
                return Err(AstBuildError::ParseFailure(
                    "OMPX payload items must be separated by whitespace".to_string(),
                ));
            }
            remaining = after_call.trim_start();
        } else {
            items.push(OmpxPayloadItem::Identifier(name));
            if !after_name.is_empty() && !after_name.chars().next().is_some_and(char::is_whitespace)
            {
                return Err(AstBuildError::ParseFailure(
                    "OMPX payload items must be separated by whitespace".to_string(),
                ));
            }
            remaining = after_name.trim_start();
        }
    }

    let parameter = OmpDirectiveParameter::Ompx(OmpxPayload::new(items)?);
    Ok(RoupDirective::OpenMp(Box::new(OmpDirective::new(
        OmpDirectiveKind::Ompx,
        Some(parameter),
        Vec::new(),
        None,
        source.span_of(sentinel_source)?,
    )?)))
}

/// Convert a parsed directive into the enum-based ROUP AST.
pub(crate) fn build_roup_directive(
    directive: &LocatedDirective<'_>,
    dialect: Dialect,
    parser_config: &ParserConfig,
    host_language: HostLanguage,
    source: &LogicalSource<'_>,
) -> Result<RoupDirective, AstBuildError> {
    match dialect {
        Dialect::OpenMp => Ok(RoupDirective::OpenMp(Box::new(build_omp_directive(
            directive,
            parser_config,
            host_language,
            source,
        )?))),
        Dialect::OpenAcc => Ok(RoupDirective::OpenAcc(Box::new(build_acc_directive(
            directive,
            parser_config,
            source,
        )?))),
    }
}

fn build_omp_directive(
    directive: &LocatedDirective<'_>,
    parser_config: &ParserConfig,
    host_language: HostLanguage,
    source: &LogicalSource<'_>,
) -> Result<OmpDirective, AstBuildError> {
    let directive_span = source.span_of(directive.name_source())?;
    let directive_name = directive.name.clone();

    let kind = OmpDirectiveKind::try_from(directive_name).map_err(|name| {
        AstBuildError::UnsupportedDirective(format!("{name:?} not supported for OpenMP"))
    })?;

    let has_flush_memory_order_argument = directive
        .clauses
        .iter()
        .any(|clause| matches!(&clause.kind, ClauseKind::FlushMemoryOrderArgument(_)));
    if directive
        .clauses
        .last()
        .is_some_and(LocatedClause::followed_by_trailing_comma)
        && !parser_config.source_extensions()
    {
        return Err(AstBuildError::ClauseConversion(
            "a directive clause sequence must not end with a comma".to_string(),
        ));
    }
    if has_flush_memory_order_argument
        && matches!(
            parser_config.openmp_version_policy(),
            VersionPolicy::Exact(version) if version <= OpenMpVersion::V5_2
        )
    {
        return Err(AstBuildError::ClauseConversion(
            "OpenMP 5.2 and earlier forbid a flush list when a memory-order clause is specified"
                .to_string(),
        ));
    }

    let clause_config = *parser_config;
    let clauses = directive
        .clauses
        .iter()
        .filter(|clause| !omp_clause_is_parameter_owned(kind, clause))
        .map(|clause| convert_clause_to_omp(clause, &clause_config, kind, source))
        .collect::<Result<Vec<_>, _>>()?;

    validate_omp_directive(kind, &clauses, host_language)?;

    let parameter = if has_flush_memory_order_argument && parser_config.source_extensions() {
        let content = directive
            .clauses
            .iter()
            .find_map(|clause| match &clause.kind {
                ClauseKind::FlushMemoryOrderArgument(content) => Some(content.as_ref()),
                _ => None,
            })
            .ok_or_else(|| {
                AstBuildError::ClauseConversion(
                    "flush memory-order argument disappeared during AST construction".to_string(),
                )
            })?;
        let items = super::semantic::split_top_level_items(content)?
            .into_iter()
            .map(|item| parse_flush_list_item(item, &clause_config))
            .collect::<Result<Vec<_>, _>>()?;
        Some(OmpDirectiveParameter::FlushList(items))
    } else if has_flush_memory_order_argument {
        None
    } else {
        build_omp_directive_parameter(directive, &clause_config)?
    };
    let source_alias = omp_directive_source_alias(kind, directive.name_source());

    Ok(OmpDirective::new(
        kind,
        parameter,
        clauses,
        source_alias,
        directive_span,
    )?)
}

fn omp_clause_is_parameter_owned(kind: OmpDirectiveKind, clause: &LocatedClause<'_>) -> bool {
    kind == OmpDirectiveKind::DeclareReduction
        && matches!(
            lookup_clause_name(clause.name.as_ref()),
            ClauseName::Combiner | ClauseName::Initializer
        )
}

fn omp_directive_source_alias(
    kind: OmpDirectiveKind,
    source_name: &str,
) -> Option<OmpDirectiveSourceAlias> {
    let source_name = crate::lexer::collapse_line_continuations(source_name);
    let source_name = source_name.trim();
    if kind == OmpDirectiveKind::Teams
        && source_name
            .split_ascii_whitespace()
            .next()
            .is_some_and(|token| token.eq_ignore_ascii_case("omp"))
    {
        return Some(OmpDirectiveSourceAlias::FortranRedundantOmp);
    }
    if source_name.contains('_') {
        return Some(OmpDirectiveSourceAlias::OpenMp60Underscore);
    }

    let canonical = kind.as_str();
    if !canonical.contains(' ') {
        return None;
    }
    let source_compact = source_name
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .map(|byte| byte.to_ascii_lowercase());
    let canonical_compact = canonical
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .map(|byte| byte.to_ascii_lowercase());
    let source_has_all_spaces = source_name
        .split_whitespace()
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
        == canonical;

    (!source_has_all_spaces && source_compact.eq(canonical_compact))
        .then_some(OmpDirectiveSourceAlias::FortranCompact)
}

fn validate_omp_directive(
    kind: OmpDirectiveKind,
    clauses: &[OmpClause],
    host_language: HostLanguage,
) -> Result<(), AstBuildError> {
    let has_clause = |expected| clauses.iter().any(|clause| clause.kind() == expected);
    if kind == OmpDirectiveKind::DeclareInduction
        && (!has_clause(OmpClauseKind::Collector) || !has_clause(OmpClauseKind::Inductor))
    {
        return Err(AstBuildError::ParseFailure(
            "declare induction requires both collector and inductor clauses".to_string(),
        ));
    }

    if kind == OmpDirectiveKind::Scan {
        let scan_clause_count = [
            OmpClauseKind::Exclusive,
            OmpClauseKind::Inclusive,
            OmpClauseKind::InitComplete,
        ]
        .into_iter()
        .filter(|expected| has_clause(*expected))
        .count();
        if scan_clause_count != 1 {
            return Err(AstBuildError::ParseFailure(
                "scan requires exactly one of exclusive, inclusive, or init_complete".to_string(),
            ));
        }
    }

    if matches!(host_language, HostLanguage::Fortran) {
        if matches!(kind, OmpDirectiveKind::Do | OmpDirectiveKind::DoSimd)
            && clauses
                .iter()
                .any(|clause| matches!(clause.kind(), OmpClauseKind::Nowait))
        {
            return Err(AstBuildError::ParseFailure(
                "Fortran DO directives accept NOWAIT only on the terminating directive".to_string(),
            ));
        }

        if matches!(kind, OmpDirectiveKind::EndDo | OmpDirectiveKind::EndDoSimd) {
            for clause in clauses {
                if !matches!(clause.kind(), OmpClauseKind::Nowait) {
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
            let raw_construct = param_str.trim();
            let construct_keyword =
                if matches!(parser_config.host_language(), HostLanguage::Fortran) {
                    std::borrow::Cow::Owned(raw_construct.to_ascii_lowercase())
                } else {
                    std::borrow::Cow::Borrowed(raw_construct)
                };
            let construct = match construct_keyword.as_ref() {
                "parallel" => OmpConstructType::Parallel,
                "sections" => OmpConstructType::Sections,
                "for" | "do" => OmpConstructType::For,
                "taskgroup" => OmpConstructType::Taskgroup,
                _ => {
                    return Err(AstBuildError::ParseFailure(format!(
                        "unknown cancel construct: {raw_construct}"
                    )));
                }
            };
            return Ok(Some(OmpDirectiveParameter::Construct(construct)));
        }
        DirectiveName::Critical | DirectiveName::EndCritical => {
            let cleaned = require_exact_parenthesized(param_str, "critical section name")?;
            return Ok(Some(OmpDirectiveParameter::CriticalSection(
                Identifier::new(cleaned)?,
            )));
        }
        DirectiveName::Depobj => {
            return Ok(Some(OmpDirectiveParameter::Depobj(parse_depobj_parameter(
                param_str,
                parser_config,
            )?)));
        }
        DirectiveName::Flush => {
            let list = parse_flush_list_parameter(param_str, parser_config)?;
            return Ok(Some(OmpDirectiveParameter::FlushList(list)));
        }
        DirectiveName::DeclareSimd => {
            return Ok(Some(OmpDirectiveParameter::DeclareSimd(
                parse_declare_simd_target(param_str, parser_config)?,
            )));
        }
        DirectiveName::DeclareMapper => {
            let mapper = parse_declare_mapper_param(param_str, parser_config)?;
            return Ok(Some(OmpDirectiveParameter::DeclareMapper(mapper)));
        }
        DirectiveName::DeclareReduction => {
            let reduction =
                parse_declare_reduction_param(param_str, &directive.clauses, parser_config)?;
            return Ok(Some(OmpDirectiveParameter::DeclareReduction(Box::new(
                reduction,
            ))));
        }
        DirectiveName::DeclareInduction => {
            let induction = parse_declare_induction_param(param_str, parser_config)?;
            return Ok(Some(OmpDirectiveParameter::DeclareInduction(induction)));
        }
        DirectiveName::DeclareVariant => {
            return Ok(Some(OmpDirectiveParameter::DeclareVariant(
                parse_declare_variant_target(param_str, parser_config)?,
            )));
        }
        DirectiveName::Allocate => {
            return Ok(Some(OmpDirectiveParameter::AllocateList(
                parse_storage_list_parameter(param_str, parser_config, "allocate")?,
            )));
        }
        DirectiveName::Threadprivate => {
            return Ok(Some(OmpDirectiveParameter::ThreadprivateList(
                parse_storage_list_parameter(param_str, parser_config, "threadprivate")?,
            )));
        }
        DirectiveName::Groupprivate => {
            return Ok(Some(OmpDirectiveParameter::GroupprivateList(
                parse_storage_list_parameter(param_str, parser_config, "groupprivate")?,
            )));
        }
        DirectiveName::DeclareTarget | DirectiveName::DeclareTargetUnderscore => {
            return Ok(Some(OmpDirectiveParameter::DeclareTargetList(
                parse_declare_target_list_parameter(param_str, parser_config)?,
            )));
        }
        _ => {}
    }

    Err(AstBuildError::ParseFailure(format!(
        "{} directive does not accept a parameter",
        directive.name.as_str()
    )))
}

fn parse_depobj_parameter(
    raw: &str,
    parser_config: &ParserConfig,
) -> Result<Expression, AstBuildError> {
    let content = require_exact_parenthesized(raw, "depobj")?;
    let items = super::semantic::split_top_level_items(content)?;
    if items.len() != 1 {
        return Err(AstBuildError::ParseFailure(
            "depobj requires exactly one lvalue".to_string(),
        ));
    }
    let expression = Expression::new(items[0], parser_config)?;
    if !parser_config.source_extensions() {
        LValue::from_expression(expression.clone()).map_err(|error| {
            AstBuildError::ParseFailure(format!("invalid depobj target: {error}"))
        })?;
    }
    Ok(expression)
}

fn parse_storage_list_parameter(
    raw: &str,
    parser_config: &ParserConfig,
    directive_name: &str,
) -> Result<Vec<OmpStorageListItem>, AstBuildError> {
    let content = require_exact_parenthesized(raw, directive_name)?;
    super::semantic::split_top_level_items(content)?
        .into_iter()
        .map(|item| {
            if let Some(common_block) =
                parse_fortran_common_block(item, parser_config, directive_name)?
            {
                return Ok(OmpStorageListItem::FortranCommonBlock(common_block));
            }
            Ok(OmpStorageListItem::Name(parse_whole_entity_name(
                item,
                parser_config,
                directive_name,
            )?))
        })
        .collect()
}

fn parse_declare_target_list_parameter(
    raw: &str,
    parser_config: &ParserConfig,
) -> Result<Vec<OmpDeclareTargetListItem>, AstBuildError> {
    let context = "declare target extended list";
    let content = require_exact_parenthesized(raw, context)?;
    super::semantic::split_top_level_items(content)?
        .into_iter()
        .map(|item| {
            if let Some(common_block) = parse_fortran_common_block(item, parser_config, context)? {
                return Ok(OmpDeclareTargetListItem::FortranCommonBlock(common_block));
            }
            Ok(OmpDeclareTargetListItem::Name(parse_whole_entity_name(
                item,
                parser_config,
                context,
            )?))
        })
        .collect()
}

fn parse_whole_entity_name(
    source: &str,
    parser_config: &ParserConfig,
    context: &str,
) -> Result<QualifiedName, AstBuildError> {
    let expression = Expression::new(source.trim(), parser_config)?;
    match &expression.ast().kind {
        ExprKind::Name(name) => Ok(name.clone()),
        _ => Err(AstBuildError::ParseFailure(format!(
            "{context} only accepts whole variable or procedure names; parts of variables are forbidden"
        ))),
    }
}

fn parse_fortran_common_block(
    source: &str,
    parser_config: &ParserConfig,
    context: &str,
) -> Result<Option<Identifier>, AstBuildError> {
    if !matches!(parser_config.host_language(), HostLanguage::Fortran) {
        return Ok(None);
    }

    let source = source.trim();
    if !source.starts_with('/') && !source.ends_with('/') {
        return Ok(None);
    }
    let Some(name) = source
        .strip_prefix('/')
        .and_then(|inner| inner.strip_suffix('/'))
        .map(str::trim)
        .filter(|name| !name.is_empty() && !name.contains('/'))
    else {
        return Err(AstBuildError::ParseFailure(format!(
            "malformed Fortran named common block in {context}"
        )));
    };
    Ok(Some(Identifier::new(name.to_ascii_lowercase())?))
}

fn parse_flush_list_parameter(
    raw: &str,
    parser_config: &ParserConfig,
) -> Result<Vec<OmpFlushListItem>, AstBuildError> {
    let content = require_exact_parenthesized(raw, "flush variable list")?;
    super::semantic::split_top_level_items(content)?
        .into_iter()
        .map(|item| parse_flush_list_item(item, parser_config))
        .collect()
}

fn parse_flush_list_item(
    source: &str,
    parser_config: &ParserConfig,
) -> Result<OmpFlushListItem, AstBuildError> {
    let source = source.trim();
    if let Some(name) = parse_fortran_common_block(source, parser_config, "flush list")? {
        return Ok(OmpFlushListItem::FortranCommonBlock(name));
    }

    let mut items = parse_identifier_list(source, parser_config)?;
    let item = items.pop().ok_or_else(|| {
        AstBuildError::ParseFailure("flush variable list contains an empty item".to_string())
    })?;
    if !items.is_empty() {
        return Err(AstBuildError::ParseFailure(
            "flush variable list item contains an unexpected separator".to_string(),
        ));
    }
    match item {
        ClauseItem::Identifier(identifier) => Ok(OmpFlushListItem::Identifier(identifier)),
        ClauseItem::Variable(variable) => Ok(OmpFlushListItem::Variable(variable)),
        ClauseItem::FortranCommonBlock(name) => Ok(OmpFlushListItem::FortranCommonBlock(name)),
        ClauseItem::Expression(expression) => Err(AstBuildError::ParseFailure(format!(
            "flush variable list contains a general expression: `{expression}`"
        ))),
        ClauseItem::OmpparserTrailingSlash(identifier) => {
            Err(AstBuildError::ParseFailure(format!(
                "flush variable list contains an ompparser trailing-slash item: `{identifier}/`"
            )))
        }
    }
}

fn parse_declare_simd_target(
    raw: &str,
    parser_config: &ParserConfig,
) -> Result<OmpSimdTarget, AstBuildError> {
    if !matches!(parser_config.host_language(), HostLanguage::Fortran) {
        return Err(AstBuildError::ParseFailure(
            "a parenthesized declare simd procedure name is only valid in Fortran".to_string(),
        ));
    }
    let function_source = require_exact_parenthesized(raw, "declare simd procedure name")?;
    let (function_source, hash_prefixed) = if parser_config.source_extensions() {
        function_source
            .strip_prefix('#')
            .map_or((function_source, false), |function| (function, true))
    } else {
        (function_source, false)
    };
    let function = Identifier::new(function_source.to_ascii_lowercase())?;
    Ok(OmpSimdTarget::new(function, hash_prefixed))
}

fn parse_declare_variant_target(
    raw: &str,
    parser_config: &ParserConfig,
) -> Result<OmpDeclareVariantTarget, AstBuildError> {
    let inner = require_exact_parenthesized(raw, "declare variant target")?;
    let (base, variant_source) =
        if let Some((base, variant)) = lang::split_once_top_level(inner, ':')? {
            let base = base.trim();
            let variant = variant.trim();
            if base.is_empty() || variant.is_empty() {
                return Err(AstBuildError::ParseFailure(
                    "declare variant base and variant names must not be empty".to_string(),
                ));
            }
            if lang::split_once_top_level(variant, ':')?.is_some() {
                return Err(AstBuildError::ParseFailure(
                    "declare variant accepts at most one base-name separator".to_string(),
                ));
            }
            (Some(Identifier::new(base)?), variant)
        } else {
            (None, inner)
        };

    Ok(OmpDeclareVariantTarget::new(
        base,
        parse_omp_function_name(variant_source, parser_config)?,
    ))
}

fn parse_omp_function_name(
    source: &str,
    parser_config: &ParserConfig,
) -> Result<OmpFunctionName, AstBuildError> {
    if let Ok(expression) = Expression::new(source, parser_config)
        && let ExprKind::Name(name) = &expression.ast().kind
    {
        return Ok(OmpFunctionName::Name(name.clone()));
    }

    if matches!(parser_config.host_language(), HostLanguage::Cpp) {
        let template_id = parse_type_name(source, parser_config)?;
        if is_cpp_qualified_template_id(template_id.tokens()) {
            return Ok(OmpFunctionName::CppTemplateId(OmpCppTemplateId::new(
                template_id,
            )));
        }
    }

    Err(AstBuildError::ParseFailure(
        "declare variant requires a host-language function name".to_string(),
    ))
}

fn is_cpp_qualified_template_id(tokens: &[TokenKind]) -> bool {
    let mut index = 0usize;
    if matches!(tokens.first(), Some(TokenKind::Scope)) {
        index += 1;
    }
    if index == tokens.len() {
        return false;
    }

    let mut saw_template_id = false;
    loop {
        if !matches!(tokens.get(index), Some(TokenKind::Identifier(_))) {
            return false;
        }
        index += 1;

        if matches!(tokens.get(index), Some(TokenKind::Less)) {
            let Some(after_arguments) = consume_cpp_template_arguments(tokens, index) else {
                return false;
            };
            saw_template_id = true;
            index = after_arguments;
        }

        if index == tokens.len() {
            return saw_template_id;
        }
        if !matches!(tokens.get(index), Some(TokenKind::Scope)) {
            return false;
        }
        index += 1;
        if index == tokens.len() {
            return false;
        }
    }
}

fn consume_cpp_template_arguments(tokens: &[TokenKind], start: usize) -> Option<usize> {
    if !matches!(tokens.get(start), Some(TokenKind::Less)) {
        return None;
    }
    let mut frames = vec![(false, false)]; // (current argument has tokens, saw comma)
    let mut delimiters = Vec::new();

    for (index, token) in tokens.iter().enumerate().skip(start + 1) {
        if !delimiters.is_empty() {
            if let Some(frame) = frames.last_mut() {
                frame.0 = true;
            }
            match token {
                TokenKind::LeftParen => delimiters.push(TokenKind::RightParen),
                TokenKind::LeftBracket => delimiters.push(TokenKind::RightBracket),
                TokenKind::RightParen
                    if matches!(delimiters.last(), Some(TokenKind::RightParen)) =>
                {
                    delimiters.pop();
                }
                TokenKind::RightBracket
                    if matches!(delimiters.last(), Some(TokenKind::RightBracket)) =>
                {
                    delimiters.pop();
                }
                _ => {}
            }
            continue;
        }

        match token {
            TokenKind::LeftParen => {
                frames.last_mut()?.0 = true;
                delimiters.push(TokenKind::RightParen);
            }
            TokenKind::LeftBracket => {
                frames.last_mut()?.0 = true;
                delimiters.push(TokenKind::RightBracket);
            }
            TokenKind::Less => {
                frames.last_mut()?.0 = true;
                frames.push((false, false));
            }
            TokenKind::Comma => {
                let frame = frames.last_mut()?;
                if !frame.0 {
                    return None;
                }
                frame.0 = false;
                frame.1 = true;
            }
            TokenKind::Greater => {
                if !close_cpp_template_frame(&mut frames) {
                    return None;
                }
                if frames.is_empty() {
                    return Some(index + 1);
                }
            }
            TokenKind::ShiftRight => {
                for _ in 0..2 {
                    if !close_cpp_template_frame(&mut frames) {
                        return None;
                    }
                }
                if frames.is_empty() {
                    return Some(index + 1);
                }
            }
            _ => frames.last_mut()?.0 = true,
        }
    }
    None
}

fn close_cpp_template_frame(frames: &mut Vec<(bool, bool)>) -> bool {
    let Some((has_tokens, saw_comma)) = frames.pop() else {
        return false;
    };
    if saw_comma && !has_tokens {
        return false;
    }
    if let Some(parent) = frames.last_mut() {
        parent.0 = true;
    }
    true
}

fn parse_declare_mapper_param(
    raw: &str,
    parser_config: &ParserConfig,
) -> Result<OmpDeclareMapper, AstBuildError> {
    let inner = require_exact_parenthesized(raw, "declare mapper")?;
    let (mapper_id, declaration) = if let Some((candidate, declaration)) =
        crate::ir::lang::split_once_top_level(inner, ':')?
    {
        let candidate = candidate.trim();
        if candidate.is_empty() {
            return Err(AstBuildError::ParseFailure(
                "declare mapper identifier before ':' must not be empty".to_string(),
            ));
        }
        (Some(candidate), declaration.trim())
    } else {
        (None, inner.trim())
    };

    let (type_part, variable_part, declarator_separator) = match parser_config.host_language() {
        HostLanguage::Fortran => {
            let (type_part, variable_part) = declaration.rsplit_once("::").ok_or_else(|| {
                AstBuildError::ParseFailure(
                    "Fortran declare mapper requires `type-name :: variable`".to_string(),
                )
            })?;
            (
                type_part,
                variable_part,
                crate::ast::OmpDeclaratorSeparator::Space,
            )
        }
        HostLanguage::C | HostLanguage::Cpp => split_c_mapper_declaration(declaration)?,
    };
    let type_part = type_part.trim();
    let variable_part = variable_part.trim();
    if type_part.is_empty() || variable_part.is_empty() {
        return Err(AstBuildError::ParseFailure(
            "declare mapper type name and variable must not be empty".to_string(),
        ));
    }

    let identifier = match mapper_id {
        Some(id) if host_keyword_eq(id, "default", parser_config) => Some(OmpMapperId::Default),
        Some(id) => Some(OmpMapperId::User(Identifier::new(id)?)),
        None => None,
    };

    Ok(OmpDeclareMapper::new(
        identifier,
        parse_type_name(type_part, parser_config)?,
        Identifier::new(variable_part)?,
        declarator_separator,
    ))
}

/// Split the C/C++ declare-mapper declaration into a type and its required
/// mapper variable without losing pointer or reference operators.
///
/// OpenMP requires the mapper variable itself to be an identifier.  Therefore
/// the final identifier token is the declarator and every preceding token,
/// including adjacent `*`, `&`, or `&&`, is part of the typed type name.
fn split_c_mapper_declaration(
    declaration: &str,
) -> Result<(&str, &str, crate::ast::OmpDeclaratorSeparator), AstBuildError> {
    let declaration = declaration.trim();
    let end = declaration.len();
    let mut start = end;
    for (index, character) in declaration.char_indices().rev() {
        if character == '_' || character.is_alphanumeric() {
            start = index;
        } else {
            break;
        }
    }

    if start == end {
        return Err(AstBuildError::ParseFailure(
            "declare mapper requires a trailing variable identifier".to_string(),
        ));
    }

    let variable = &declaration[start..end];
    // Validate the lexical identifier boundary before trying the type so an
    // input such as `T 1name` cannot be misdiagnosed as a malformed type.
    Identifier::new(variable)?;
    let separator = if declaration[..start]
        .chars()
        .next_back()
        .is_some_and(char::is_whitespace)
    {
        crate::ast::OmpDeclaratorSeparator::Space
    } else {
        crate::ast::OmpDeclaratorSeparator::Adjacent
    };
    let type_name = declaration[..start].trim_end();
    if type_name.is_empty() {
        return Err(AstBuildError::ParseFailure(
            "declare mapper requires a type name before its variable".to_string(),
        ));
    }

    Ok((type_name, variable, separator))
}

fn parse_declare_induction_param(
    raw: &str,
    parser_config: &ParserConfig,
) -> Result<OmpDeclareInduction, AstBuildError> {
    let inner = require_exact_parenthesized(raw, "declare induction")?;
    let (identifier_source, types_source) = crate::ir::lang::split_once_top_level(inner, ':')?
        .ok_or_else(|| {
            AstBuildError::ParseFailure(
                "declare induction requires `induction-identifier : type-specifier-list`"
                    .to_string(),
            )
        })?;
    let identifier_source = identifier_source.trim();
    let types_source = types_source.trim();
    if identifier_source.is_empty() || types_source.is_empty() {
        return Err(AstBuildError::ParseFailure(
            "declare induction identifier and type list must not be empty".to_string(),
        ));
    }

    let identifier = parse_induction_identifier(identifier_source, parser_config)?;
    let type_specifiers =
        crate::ir::lang::split_top_level(types_source, ',', &[('(', ')'), ('[', ']'), ('<', '>')])?
            .into_iter()
            .map(str::trim)
            .map(|source| parse_induction_type_specifier(source, parser_config))
            .collect::<Result<Vec<_>, _>>()?;

    Ok(OmpDeclareInduction::new(identifier, type_specifiers)?)
}

pub(crate) fn parse_induction_identifier(
    source: &str,
    parser_config: &ParserConfig,
) -> Result<OmpInductionIdentifier, AstBuildError> {
    let source = source.trim();
    match source {
        "+" => return Ok(OmpInductionIdentifier::Add),
        "*" => return Ok(OmpInductionIdentifier::Multiply),
        _ => {}
    }

    if parser_config.host_language() == HostLanguage::Fortran && source.starts_with('.') {
        return Ok(OmpInductionIdentifier::DefinedOperator(
            parse_fortran_defined_operator(source, "declare induction")?,
        ));
    }

    Ok(OmpInductionIdentifier::Name(parse_openmp_id_expression(
        source,
        parser_config,
        "declare induction identifier",
    )?))
}

fn parse_openmp_id_expression(
    source: &str,
    parser_config: &ParserConfig,
    context: &str,
) -> Result<OmpIdExpression, AstBuildError> {
    if let Ok(expression) = Expression::new(source, parser_config)
        && let crate::host::ExprKind::Name(name) = &expression.ast().kind
    {
        if parser_config.host_language() != HostLanguage::Cpp
            && (name.global || name.segments.len() != 1)
        {
            return Err(AstBuildError::ParseFailure(format!(
                "{context} must be an unqualified base-language identifier"
            )));
        }
        return Ok(OmpIdExpression::Name(name.clone()));
    }

    if parser_config.host_language() == HostLanguage::Cpp {
        if let Some(operator_function) = parse_cpp_operator_function_id(source, parser_config)? {
            return Ok(OmpIdExpression::CppOperatorFunction(operator_function));
        }

        let syntax = parse_type_name(source, parser_config)?;
        if is_cpp_qualified_template_id(syntax.tokens()) {
            return Ok(OmpIdExpression::CppTemplateId(OmpCppTemplateId::new(
                syntax,
            )));
        }
    }

    Err(AstBuildError::ParseFailure(format!(
        "{context} must be a base-language id-expression"
    )))
}

fn parse_cpp_operator_function_id(
    source: &str,
    parser_config: &ParserConfig,
) -> Result<Option<OmpCppOperatorFunctionId>, AstBuildError> {
    let source = source.trim();
    let Some(operator_index) = source.rfind("operator") else {
        return Ok(None);
    };
    let (prefix, operator_source) = source.split_at(operator_index);
    let Some(operator_source) = operator_source.strip_prefix("operator") else {
        return Err(AstBuildError::ParseFailure(
            "failed to split a C++ operator-function-id at its keyword".to_string(),
        ));
    };
    let operator_source = operator_source.trim();
    let operator = match operator_source {
        "+" => OmpCppReductionOperator::Add,
        "-" => OmpCppReductionOperator::Subtract,
        "*" => OmpCppReductionOperator::Multiply,
        "&" => OmpCppReductionOperator::BitwiseAnd,
        "|" => OmpCppReductionOperator::BitwiseOr,
        "^" => OmpCppReductionOperator::BitwiseXor,
        "&&" => OmpCppReductionOperator::LogicalAnd,
        "||" => OmpCppReductionOperator::LogicalOr,
        _ => return Ok(None),
    };

    let prefix = prefix.trim_end();
    let (global, qualifier_source) = if prefix.is_empty() {
        (false, None)
    } else if prefix == "::" {
        (true, None)
    } else {
        let Some(qualifier) = prefix.strip_suffix("::") else {
            return Ok(None);
        };
        let qualifier = qualifier.trim();
        let (global, qualifier) = if let Some(qualifier) = qualifier.strip_prefix("::") {
            (true, qualifier.trim_start())
        } else {
            (false, qualifier)
        };
        if qualifier.is_empty() {
            return Ok(None);
        }
        (global, Some(qualifier))
    };

    let qualifier = qualifier_source
        .map(|qualifier| parse_cpp_operator_qualifier(qualifier, parser_config))
        .transpose()?;
    Ok(Some(OmpCppOperatorFunctionId::new(
        global, qualifier, operator,
    )))
}

fn parse_cpp_operator_qualifier(
    source: &str,
    parser_config: &ParserConfig,
) -> Result<OmpCppOperatorQualifier, AstBuildError> {
    if let Ok(expression) = Expression::new(source, parser_config)
        && let ExprKind::Name(name) = &expression.ast().kind
        && !name.global
    {
        return Ok(OmpCppOperatorQualifier::Name(name.clone()));
    }

    let syntax = parse_type_name(source, parser_config)?;
    if is_cpp_qualified_template_id(syntax.tokens())
        && !matches!(syntax.tokens().first(), Some(TokenKind::Scope))
    {
        return Ok(OmpCppOperatorQualifier::TemplateId(OmpCppTemplateId::new(
            syntax,
        )));
    }

    Err(AstBuildError::ParseFailure(
        "C++ operator-function qualifier must be a qualified name or template-id".to_string(),
    ))
}

fn parse_fortran_defined_operator(
    source: &str,
    context: &str,
) -> Result<Identifier, AstBuildError> {
    let Some(name) = source
        .strip_prefix('.')
        .and_then(|value| value.strip_suffix('.'))
    else {
        return Err(AstBuildError::ParseFailure(format!(
            "{context} has a malformed Fortran defined operator"
        )));
    };
    if name.is_empty()
        || !name
            .chars()
            .all(|character| character.is_ascii_alphabetic())
    {
        return Err(AstBuildError::ParseFailure(format!(
            "{context} has an invalid Fortran defined-operator name"
        )));
    }
    let name = name.to_ascii_lowercase();
    if matches!(
        name.as_str(),
        "and"
            | "or"
            | "eqv"
            | "neqv"
            | "not"
            | "eq"
            | "ne"
            | "lt"
            | "le"
            | "gt"
            | "ge"
            | "true"
            | "false"
    ) {
        return Err(AstBuildError::ParseFailure(format!(
            "{context} cannot use an intrinsic dotted Fortran token as a user-defined operator"
        )));
    }
    Ok(Identifier::new(name)?)
}

fn parse_induction_type_specifier(
    source: &str,
    parser_config: &ParserConfig,
) -> Result<OmpInductionTypeSpecifier, AstBuildError> {
    if source.is_empty() {
        return Err(AstBuildError::ParseFailure(
            "declare induction type list contains an empty entry".to_string(),
        ));
    }

    if source.starts_with('(') {
        let inner = require_exact_parenthesized(source, "declare induction type pair")?;
        let parts =
            crate::ir::lang::split_top_level(inner, ',', &[('(', ')'), ('[', ']'), ('<', '>')])?;
        if parts.len() != 2 || parts.iter().any(|part| part.trim().is_empty()) {
            return Err(AstBuildError::ParseFailure(
                "declare induction type pair requires exactly two type names".to_string(),
            ));
        }
        return Ok(OmpInductionTypeSpecifier::Pair {
            variable: parse_type_name(parts[0].trim(), parser_config)?,
            step: parse_type_name(parts[1].trim(), parser_config)?,
        });
    }

    Ok(OmpInductionTypeSpecifier::Same(parse_type_name(
        source,
        parser_config,
    )?))
}

fn parse_declare_reduction_param(
    raw: &str,
    clauses: &[LocatedClause<'_>],
    parser_config: &ParserConfig,
) -> Result<OmpDeclareReduction, AstBuildError> {
    let inner = require_exact_parenthesized(raw, "declare reduction")?;
    let (identifier_source, after_identifier) = crate::ir::lang::split_once_top_level(inner, ':')?
        .ok_or_else(|| {
            AstBuildError::ParseFailure(
                "declare reduction requires `identifier : type-list`".to_string(),
            )
        })?;
    let identifier_source = identifier_source.trim();
    let after_identifier = after_identifier.trim();
    if identifier_source.is_empty() || after_identifier.is_empty() {
        return Err(AstBuildError::ParseFailure(
            "declare reduction identifier and type list must not be empty".to_string(),
        ));
    }

    let clause_combiner = unique_reduction_clause_argument(clauses, ClauseName::Combiner)?;
    let clause_initializer = unique_reduction_clause_argument(clauses, ClauseName::Initializer)?;
    let (types_source, combiner_source, source_syntax) = if let Some((
        types_source,
        inline_combiner,
    )) =
        crate::ir::lang::split_once_top_level(after_identifier, ':')?
    {
        if clause_combiner.is_some() {
            return Err(AstBuildError::ParseFailure(
                    "historical inline declare-reduction syntax must not also specify a combiner clause"
                        .to_string(),
                ));
        }
        (
            types_source.trim(),
            inline_combiner.trim(),
            OmpDeclareReductionSyntax::InlineCombiner,
        )
    } else {
        let combiner = clause_combiner.ok_or_else(|| {
            AstBuildError::ParseFailure(
                "declare reduction requires exactly one combiner clause".to_string(),
            )
        })?;
        (
            after_identifier,
            combiner.trim(),
            OmpDeclareReductionSyntax::CombinerClause,
        )
    };
    if types_source.is_empty() || combiner_source.is_empty() {
        return Err(AstBuildError::ParseFailure(
            "declare reduction type list and combiner must not be empty".to_string(),
        ));
    }

    let identifier = parse_reduction_identifier(identifier_source, parser_config)?;
    let type_names =
        crate::ir::lang::split_top_level(types_source, ',', &[('(', ')'), ('[', ']'), ('<', '>')])?
            .into_iter()
            .map(str::trim)
            .map(|type_name| {
                if type_name.is_empty() {
                    return Err(AstBuildError::ParseFailure(
                        "declare reduction type list contains an empty entry".to_string(),
                    ));
                }
                parse_type_name(type_name, parser_config).map_err(AstBuildError::from)
            })
            .collect::<Result<Vec<_>, _>>()?;
    let combiner = parse_reduction_combiner(combiner_source, parser_config)?;
    let initializer = clause_initializer
        .map(|source| parse_reduction_initializer(source.trim(), parser_config))
        .transpose()?;

    Ok(OmpDeclareReduction::new(
        identifier,
        type_names,
        combiner,
        initializer,
        source_syntax,
    )?)
}

fn unique_reduction_clause_argument<'a>(
    clauses: &'a [LocatedClause<'_>],
    expected: ClauseName,
) -> Result<Option<&'a str>, AstBuildError> {
    let clause_name = match expected {
        ClauseName::Combiner => "combiner",
        ClauseName::Initializer => "initializer",
        _ => {
            return Err(AstBuildError::ParseFailure(
                "internal declare-reduction clause classification failure".to_string(),
            ));
        }
    };
    let mut result = None;
    for clause in clauses {
        if lookup_clause_name(clause.name.as_ref()) != expected {
            continue;
        }
        if result.is_some() {
            return Err(AstBuildError::ParseFailure(format!(
                "declare reduction contains duplicate {clause_name} clauses"
            )));
        }
        let ClauseKind::Parenthesized(content) = &clause.kind else {
            return Err(AstBuildError::ParseFailure(format!(
                "declare reduction {clause_name} clause requires parentheses"
            )));
        };
        let content = content.as_ref().trim();
        if content.is_empty() {
            return Err(AstBuildError::ParseFailure(format!(
                "declare reduction {clause_name} clause must not be empty"
            )));
        }
        result = Some(content);
    }
    Ok(result)
}

pub(crate) fn parse_reduction_identifier(
    source: &str,
    parser_config: &ParserConfig,
) -> Result<OmpReductionIdentifier, AstBuildError> {
    let source = source.trim();
    match parser_config.host_language() {
        HostLanguage::C | HostLanguage::Cpp => match source {
            "+" => Ok(OmpReductionIdentifier::Add),
            "-" => Ok(OmpReductionIdentifier::Subtract),
            "*" => Ok(OmpReductionIdentifier::Multiply),
            "&" => Ok(OmpReductionIdentifier::BitwiseAnd),
            "|" => Ok(OmpReductionIdentifier::BitwiseOr),
            "^" => Ok(OmpReductionIdentifier::BitwiseXor),
            "&&" => Ok(OmpReductionIdentifier::LogicalAnd),
            "||" => Ok(OmpReductionIdentifier::LogicalOr),
            _ => Ok(OmpReductionIdentifier::Name(parse_openmp_id_expression(
                source,
                parser_config,
                "declare reduction identifier",
            )?)),
        },
        HostLanguage::Fortran => {
            let canonical = source.to_ascii_lowercase();
            Ok(match canonical.as_str() {
                "+" => OmpReductionIdentifier::Add,
                "-" => OmpReductionIdentifier::Subtract,
                "*" => OmpReductionIdentifier::Multiply,
                ".and." => OmpReductionIdentifier::FortranLogicalAnd,
                ".or." => OmpReductionIdentifier::FortranLogicalOr,
                ".eqv." => OmpReductionIdentifier::FortranLogicalEqv,
                ".neqv." => OmpReductionIdentifier::FortranLogicalNeqv,
                "max" => {
                    OmpReductionIdentifier::FortranIntrinsic(OmpFortranReductionIntrinsic::Max)
                }
                "min" => {
                    OmpReductionIdentifier::FortranIntrinsic(OmpFortranReductionIntrinsic::Min)
                }
                "iand" => {
                    OmpReductionIdentifier::FortranIntrinsic(OmpFortranReductionIntrinsic::Iand)
                }
                "ior" => {
                    OmpReductionIdentifier::FortranIntrinsic(OmpFortranReductionIntrinsic::Ior)
                }
                "ieor" => {
                    OmpReductionIdentifier::FortranIntrinsic(OmpFortranReductionIntrinsic::Ieor)
                }
                _ if source.starts_with('.') => OmpReductionIdentifier::FortranDefinedOperator(
                    parse_fortran_defined_operator(source, "declare reduction")?,
                ),
                _ => OmpReductionIdentifier::Name(parse_openmp_id_expression(
                    source,
                    parser_config,
                    "declare reduction identifier",
                )?),
            })
        }
    }
}

pub(crate) fn parse_inductor_expression(
    source: &str,
    parser_config: &ParserConfig,
) -> Result<OmpInductorExpression, AstBuildError> {
    if parser_config.host_language() != HostLanguage::Fortran {
        return Ok(OmpInductorExpression::COrCppExpression(Expression::new(
            source,
            parser_config,
        )?));
    }

    if let Some(assignment) = parse_fortran_assignment(
        source,
        "omp_var",
        true,
        "inductor expression",
        parser_config,
    )? {
        return Ok(OmpInductorExpression::FortranAssignment(Box::new(
            assignment,
        )));
    }

    let expression = Expression::new(source, parser_config)?;
    match &expression.ast().kind {
        ExprKind::FortranApply { designator, .. } if is_unqualified_name(designator) => {
            Ok(OmpInductorExpression::FortranSubroutineCall(expression))
        }
        _ => Err(AstBuildError::ParseFailure(
            "Fortran inductor expression must be an assignment to omp_var or a subroutine reference"
                .to_string(),
        )),
    }
}

fn parse_reduction_combiner(
    source: &str,
    parser_config: &ParserConfig,
) -> Result<OmpReductionCombiner, AstBuildError> {
    if parser_config.host_language() != HostLanguage::Fortran {
        return Ok(OmpReductionCombiner::COrCppExpression(Expression::new(
            source,
            parser_config,
        )?));
    }

    if let Some(assignment) = parse_fortran_assignment(
        source,
        "omp_out",
        true,
        "declare-reduction combiner",
        parser_config,
    )? {
        return Ok(OmpReductionCombiner::FortranAssignment(Box::new(
            assignment,
        )));
    }

    let expression = Expression::new(source, parser_config)?;
    match &expression.ast().kind {
        crate::host::ExprKind::FortranApply { designator, .. }
            if is_unqualified_name(designator) =>
        {
            Ok(OmpReductionCombiner::FortranSubroutineCall(expression))
        }
        _ => Err(AstBuildError::ParseFailure(
            "Fortran declare-reduction combiner must be an assignment to omp_out or a subroutine reference"
                .to_string(),
        )),
    }
}

fn parse_reduction_initializer(
    source: &str,
    parser_config: &ParserConfig,
) -> Result<OmpReductionInitializer, AstBuildError> {
    match parser_config.host_language() {
        HostLanguage::C => parse_c_reduction_initializer(source, parser_config),
        HostLanguage::Cpp => parse_cpp_reduction_initializer(source, parser_config),
        HostLanguage::Fortran => parse_fortran_reduction_initializer(source, parser_config),
    }
}

fn parse_c_reduction_initializer(
    source: &str,
    parser_config: &ParserConfig,
) -> Result<OmpReductionInitializer, AstBuildError> {
    if let Some(value) = strip_special_assignment(source, "omp_priv") {
        return Ok(OmpReductionInitializer::CAssignment(
            parse_initializer_value(value, parser_config)?,
        ));
    }
    let call = Expression::new(source, parser_config)?;
    if is_c_or_cpp_named_call(call.ast()) {
        require_initializer_call_private_argument(call.ast(), HostLanguage::C)?;
        Ok(OmpReductionInitializer::COrCppFunctionCall(call))
    } else {
        Err(AstBuildError::ParseFailure(
            "C declare-reduction initializer must assign omp_priv or call a function".to_string(),
        ))
    }
}

fn parse_cpp_reduction_initializer(
    source: &str,
    parser_config: &ParserConfig,
) -> Result<OmpReductionInitializer, AstBuildError> {
    let source = source.trim();
    if let Some(value) = strip_special_assignment(source, "omp_priv") {
        return Ok(OmpReductionInitializer::CppCopy(parse_initializer_value(
            value,
            parser_config,
        )?));
    }
    if let Some(rest) = source.strip_prefix("omp_priv") {
        let rest = rest.trim_start();
        if rest.starts_with('{') {
            let OmpInitializerValue::Braced(initializer) =
                parse_initializer_value(rest, parser_config)?
            else {
                return Err(AstBuildError::ParseFailure(
                    "C++ list initializer requires braces".to_string(),
                ));
            };
            return Ok(OmpReductionInitializer::CppList(initializer));
        }
    }

    let expression = Expression::new(source, parser_config)?;
    if is_named_call_to(expression.ast(), "omp_priv") {
        Ok(OmpReductionInitializer::CppDirect(expression))
    } else if is_c_or_cpp_named_call(expression.ast()) {
        require_initializer_call_private_argument(expression.ast(), HostLanguage::Cpp)?;
        Ok(OmpReductionInitializer::COrCppFunctionCall(expression))
    } else {
        Err(AstBuildError::ParseFailure(
            "C++ declare-reduction initializer must initialize omp_priv or call a function"
                .to_string(),
        ))
    }
}

fn parse_fortran_reduction_initializer(
    source: &str,
    parser_config: &ParserConfig,
) -> Result<OmpReductionInitializer, AstBuildError> {
    if let Some(assignment) = parse_fortran_assignment(
        source,
        "omp_priv",
        false,
        "declare-reduction initializer",
        parser_config,
    )? {
        return Ok(OmpReductionInitializer::FortranAssignment(Box::new(
            assignment,
        )));
    }

    let expression = Expression::new(source, parser_config)?;
    match &expression.ast().kind {
        crate::host::ExprKind::FortranApply { designator, .. }
            if is_unqualified_name(designator) =>
        {
            require_fortran_initializer_private_argument(expression.ast())?;
            Ok(OmpReductionInitializer::FortranSubroutineCall(expression))
        }
        _ => Err(AstBuildError::ParseFailure(
            "Fortran declare-reduction initializer must assign omp_priv or reference a subroutine"
                .to_string(),
        )),
    }
}

fn require_initializer_call_private_argument(
    expression: &crate::host::Expr,
    language: HostLanguage,
) -> Result<(), AstBuildError> {
    let ExprKind::Call { arguments, .. } = &expression.kind else {
        return Err(AstBuildError::ParseFailure(
            "initializer function-call classification requires a call expression".to_string(),
        ));
    };
    let has_private = arguments.iter().any(|argument| match language {
        HostLanguage::C => is_address_of_special_name(argument, "omp_priv"),
        HostLanguage::Cpp => {
            is_exact_special_name(argument, "omp_priv")
                || is_address_of_special_name(argument, "omp_priv")
        }
        HostLanguage::Fortran => false,
    });
    if has_private {
        Ok(())
    } else {
        Err(AstBuildError::ParseFailure(match language {
            HostLanguage::C => {
                "C initializer function call requires an argument that is the address of omp_priv"
                    .to_string()
            }
            HostLanguage::Cpp => {
                "C++ initializer function call requires an omp_priv or &omp_priv argument"
                    .to_string()
            }
            HostLanguage::Fortran => {
                "Fortran does not use C/C++ initializer function-call syntax".to_string()
            }
        }))
    }
}

fn require_fortran_initializer_private_argument(
    expression: &crate::host::Expr,
) -> Result<(), AstBuildError> {
    let ExprKind::FortranApply { arguments, .. } = &expression.kind else {
        return Err(AstBuildError::ParseFailure(
            "initializer subroutine classification requires a Fortran application".to_string(),
        ));
    };
    let has_private = arguments.iter().any(|argument| match argument {
        crate::host::FortranArgument::Positional(value)
        | crate::host::FortranArgument::Keyword { value, .. } => {
            is_exact_special_name(value, "omp_priv")
        }
        crate::host::FortranArgument::Section(_) => false,
    });
    if has_private {
        Ok(())
    } else {
        Err(AstBuildError::ParseFailure(
            "Fortran initializer subroutine reference requires an omp_priv argument".to_string(),
        ))
    }
}

fn is_address_of_special_name(expression: &crate::host::Expr, expected: &str) -> bool {
    match &expression.kind {
        ExprKind::Unary {
            op: crate::host::UnaryOp::AddressOf,
            operand,
        } => is_exact_special_name(operand, expected),
        ExprKind::Parenthesized(inner) => is_address_of_special_name(inner, expected),
        _ => false,
    }
}

fn is_exact_special_name(expression: &crate::host::Expr, expected: &str) -> bool {
    match &expression.kind {
        ExprKind::Name(name) => {
            !name.global && name.segments.len() == 1 && name.segments[0].as_str() == expected
        }
        ExprKind::Parenthesized(inner) => is_exact_special_name(inner, expected),
        _ => false,
    }
}

fn parse_fortran_assignment(
    source: &str,
    expected_target: &str,
    allow_component_selectors: bool,
    context: &str,
    parser_config: &ParserConfig,
) -> Result<Option<OmpFortranAssignment>, AstBuildError> {
    let Some((target_source, value_source)) = lang::split_once_top_level(source, '=')? else {
        return Ok(None);
    };
    let target_source = target_source.trim();
    let value_source = value_source.trim();
    if target_source.is_empty() || value_source.is_empty() || value_source.starts_with('=') {
        return Err(AstBuildError::ParseFailure(format!(
            "Fortran {context} has a malformed assignment statement"
        )));
    }

    let target = Variable::parse(target_source, parser_config).map_err(|error| {
        AstBuildError::ParseFailure(format!(
            "Fortran {context} has an invalid assignment target: {error}"
        ))
    })?;
    let valid_target = if allow_component_selectors {
        is_fortran_component_chain(target.ast(), expected_target)
    } else {
        target
            .simple_identifier()
            .is_some_and(|identifier| identifier.as_str() == expected_target)
    };
    if !valid_target {
        return Err(AstBuildError::ParseFailure(format!(
            "Fortran {context} has an invalid {expected_target} assignment target"
        )));
    }

    let value = Expression::new(value_source, parser_config)?;
    Ok(Some(OmpFortranAssignment::new(target, value)))
}

fn is_fortran_component_chain(expression: &crate::host::Expr, expected_root: &str) -> bool {
    match &expression.kind {
        ExprKind::Name(name) => {
            !name.global && name.segments.len() == 1 && name.segments[0].as_str() == expected_root
        }
        ExprKind::Member {
            base,
            access: crate::host::MemberAccess::FortranComponent,
            ..
        } => is_fortran_component_chain(base, expected_root),
        ExprKind::Literal(_)
        | ExprKind::This
        | ExprKind::Sizeof(_)
        | ExprKind::CppTemplateId { .. }
        | ExprKind::LegacyQualifiedInteger { .. }
        | ExprKind::LegacyQualifiedName { .. }
        | ExprKind::LegacyFortranSubscript { .. }
        | ExprKind::LegacyFortranUnaryDesignator { .. }
        | ExprKind::Parenthesized(_)
        | ExprKind::Unary { .. }
        | ExprKind::FortranDefinedUnary { .. }
        | ExprKind::Binary { .. }
        | ExprKind::FortranDefinedBinary { .. }
        | ExprKind::Conditional { .. }
        | ExprKind::Assignment { .. }
        | ExprKind::Call { .. }
        | ExprKind::Subscript { .. }
        | ExprKind::Member { .. }
        | ExprKind::Postfix { .. }
        | ExprKind::FortranApply { .. } => false,
    }
}

fn strip_special_assignment<'a>(source: &'a str, name: &str) -> Option<&'a str> {
    let rest = source.trim().strip_prefix(name)?;
    let rest = rest.trim_start().strip_prefix('=')?;
    let value = rest.trim();
    (!value.is_empty()).then_some(value)
}

fn parse_initializer_value(
    source: &str,
    parser_config: &ParserConfig,
) -> Result<OmpInitializerValue, AstBuildError> {
    let source = source.trim();
    if !source.starts_with('{') {
        return Ok(OmpInitializerValue::Expression(Expression::new(
            source,
            parser_config,
        )?));
    }

    let nested_config = parser_config.enter_nested_structure().map_err(|message| {
        AstBuildError::ParseFailure(format!(
            "braced initializer exceeds the parser limit: {message}"
        ))
    })?;

    let (content, remainder) = lang::extract_bracket_content(source, '{', '}')?;
    if !remainder.trim().is_empty() {
        return Err(AstBuildError::ParseFailure(
            "unexpected syntax after braced initializer".to_string(),
        ));
    }
    let mut content = content.trim();
    if let Some(without_trailing_comma) = content.strip_suffix(',') {
        content = without_trailing_comma.trim_end();
    }
    let elements = if content.is_empty() {
        Vec::new()
    } else {
        lang::split_top_level(content, ',', &[('(', ')'), ('[', ']'), ('{', '}')])?
            .into_iter()
            .map(|element| parse_initializer_value(element, &nested_config))
            .collect::<Result<Vec<_>, _>>()?
    };
    Ok(OmpInitializerValue::Braced(
        crate::ast::OmpBracedInitializer::new(elements),
    ))
}

fn is_c_or_cpp_named_call(expression: &crate::host::Expr) -> bool {
    matches!(
        &expression.kind,
        crate::host::ExprKind::Call { callee, .. }
            if matches!(callee.kind, crate::host::ExprKind::Name(_))
    )
}

fn is_named_call_to(expression: &crate::host::Expr, expected: &str) -> bool {
    matches!(
        &expression.kind,
        crate::host::ExprKind::Call { callee, .. }
            if is_unqualified_name(callee)
                && designator_root_name(callee).is_some_and(|name| name == expected)
    )
}

fn is_unqualified_name(expression: &crate::host::Expr) -> bool {
    matches!(
        &expression.kind,
        crate::host::ExprKind::Name(name) if !name.global && name.segments.len() == 1
    )
}

fn designator_root_name(expression: &crate::host::Expr) -> Option<&str> {
    match &expression.kind {
        crate::host::ExprKind::Name(name) if !name.global && name.segments.len() == 1 => {
            name.segments.first().map(Identifier::as_str)
        }
        crate::host::ExprKind::Parenthesized(inner)
        | crate::host::ExprKind::Member { base: inner, .. }
        | crate::host::ExprKind::Subscript { base: inner, .. }
        | crate::host::ExprKind::FortranApply {
            designator: inner, ..
        } => designator_root_name(inner),
        _ => None,
    }
}

fn require_exact_parenthesized<'a>(raw: &'a str, context: &str) -> Result<&'a str, AstBuildError> {
    let trimmed = raw.trim();
    let Some(after_open) = trimmed.strip_prefix('(') else {
        return Err(AstBuildError::ParseFailure(format!(
            "{context} requires parenthesized syntax"
        )));
    };
    let close = lang::find_matching_after_open(after_open, '(', ')')
        .map_err(AstBuildError::from)?
        .ok_or_else(|| {
            AstBuildError::ParseFailure(format!("{context} has unbalanced parentheses"))
        })?;
    if !after_open[close + 1..].trim().is_empty() {
        return Err(AstBuildError::ParseFailure(format!(
            "{context} has unexpected trailing syntax"
        )));
    }
    let content = after_open[..close].trim();
    if content.is_empty() {
        return Err(AstBuildError::ParseFailure(format!(
            "{context} payload must not be empty"
        )));
    }
    Ok(content)
}

fn build_acc_directive(
    directive: &LocatedDirective<'_>,
    parser_config: &ParserConfig,
    source: &LogicalSource<'_>,
) -> Result<AccDirective, AstBuildError> {
    if directive
        .clauses
        .last()
        .is_some_and(LocatedClause::followed_by_trailing_comma)
        && !parser_config.source_extensions()
    {
        return Err(AstBuildError::ClauseConversion(
            "a directive clause sequence must not end with a comma".to_string(),
        ));
    }
    let directive_span = source.span_of(directive.name_source())?;
    let kind = AccDirectiveKind::try_from(directive.name.clone()).map_err(|name| {
        AstBuildError::UnsupportedDirective(format!("{name:?} not supported for OpenACC"))
    })?;

    let clause_config = *parser_config;
    let clauses = directive
        .clauses
        .iter()
        .map(|clause| convert_clause_to_acc(clause, kind, &clause_config, source))
        .collect::<Result<Vec<_>, _>>()?;

    let parameter = build_acc_directive_parameter(directive, kind, &clause_config)?;

    Ok(AccDirective::new(kind, parameter, clauses, directive_span)?)
}

fn build_acc_directive_parameter(
    directive: &Directive<'_>,
    kind: AccDirectiveKind,
    parser_config: &ParserConfig,
) -> Result<Option<AccDirectiveParameter>, AstBuildError> {
    if kind == AccDirectiveKind::Cache {
        let content = required_parenthesized_directive_parameter(directive, "OpenACC cache")?;
        let (readonly, variables_source) =
            if let Some((modifier, variables)) = lang::split_once_top_level(content, ':')? {
                if !acc_keyword_eq(modifier.trim(), "readonly", parser_config) {
                    return Err(AstBuildError::ParseFailure(format!(
                        "unknown OpenACC cache modifier: {}",
                        modifier.trim()
                    )));
                }
                (true, variables.trim())
            } else {
                (false, content.trim())
            };
        let items = parse_acc_identifier_list(variables_source, parser_config)?
            .into_iter()
            .map(|item| match item {
                ClauseItem::Variable(variable) => {
                    AccCacheItem::new(variable).map_err(AstBuildError::from)
                }
                ClauseItem::Identifier(identifier) if parser_config.source_extensions() => {
                    Ok(AccCacheItem::Scalar(identifier))
                }
                ClauseItem::Identifier(identifier) => Err(AstBuildError::ParseFailure(format!(
                    "OpenACC cache item must be an array element or contiguous subarray, not scalar `{identifier}`"
                ))),
                ClauseItem::Expression(expression) => Err(AstBuildError::ParseFailure(format!(
                    "OpenACC cache item is not an array designator: `{expression}`"
                ))),
                ClauseItem::FortranCommonBlock(common_block) => {
                    Err(AstBuildError::ParseFailure(format!(
                        "OpenACC cache item cannot be a Fortran common block `/{common_block}/`"
                    )))
                }
                ClauseItem::OmpparserTrailingSlash(identifier) => {
                    Err(AstBuildError::ParseFailure(format!(
                        "OpenACC cache item cannot end in an ompparser slash: `{identifier}/`"
                    )))
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        if items.is_empty() {
            return Err(AstBuildError::ParseFailure(
                "OpenACC cache directive requires a non-empty cache-item list".to_string(),
            ));
        }
        return Ok(Some(AccDirectiveParameter::Cache(AccCacheDirective::new(
            readonly, items,
        )?)));
    }

    if kind == AccDirectiveKind::Wait {
        let Some(parameter) = directive.parameter.as_deref() else {
            return Ok(None);
        };
        let content = parse_parenthesized_directive_parameter(parameter, "OpenACC wait")?;
        let (devnum, queue_source, queues_keyword) =
            parse_acc_wait_modifiers(content, parser_config)?;
        let queues = parse_acc_wait_queue_list(queue_source, parser_config)?;
        return Ok(Some(AccDirectiveParameter::Wait(Box::new(
            AccWaitDirective::new(devnum, queues, queues_keyword)?,
        ))));
    }

    if kind == AccDirectiveKind::Routine
        && let Some(param) = directive.parameter.as_ref()
    {
        let inner = parse_parenthesized_directive_parameter(param, "OpenACC routine")?.trim();
        if inner.is_empty() {
            return Err(AstBuildError::ParseFailure(
                "OpenACC routine parentheses require a routine name".to_string(),
            ));
        }
        let ident = Identifier::new(inner)?;
        return Ok(Some(AccDirectiveParameter::Routine(
            AccRoutineDirective::new(ident),
        )));
    }

    if kind == AccDirectiveKind::End {
        let param = directive.parameter.as_ref().ok_or_else(|| {
            AstBuildError::ParseFailure(
                "OpenACC end directive requires a directive kind".to_string(),
            )
        })?;
        if param.as_ref().trim().is_empty() {
            return Err(AstBuildError::ParseFailure(
                "OpenACC end directive requires a directive kind".to_string(),
            ));
        }
        let parameter_name = param
            .as_ref()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let end_kind = AccEndKind::ALL
            .iter()
            .copied()
            .find(|kind| acc_keyword_eq(&parameter_name, kind.as_str(), parser_config))
            .ok_or_else(|| {
                AstBuildError::ParseFailure(format!(
                    "OpenACC end directive cannot pair with: {}",
                    param.as_ref()
                ))
            })?;
        return Ok(Some(AccDirectiveParameter::End(end_kind)));
    }

    Ok(None)
}

fn required_parenthesized_directive_parameter<'a>(
    directive: &'a Directive<'_>,
    subject: &str,
) -> Result<&'a str, AstBuildError> {
    let parameter = directive.parameter.as_deref().ok_or_else(|| {
        AstBuildError::ParseFailure(format!("{subject} directive requires parentheses"))
    })?;
    parse_parenthesized_directive_parameter(parameter, subject)
}

fn parse_parenthesized_directive_parameter<'a>(
    parameter: &'a str,
    subject: &str,
) -> Result<&'a str, AstBuildError> {
    let parameter = parameter.trim();
    if !parameter.starts_with('(') {
        return Err(AstBuildError::ParseFailure(format!(
            "{subject} parameter must start with '('"
        )));
    }
    let (content, remainder) = lang::extract_bracket_content(parameter, '(', ')')?;
    if !remainder.trim().is_empty() {
        return Err(AstBuildError::ParseFailure(format!(
            "unexpected text after {subject} parameter"
        )));
    }
    Ok(content)
}

fn parse_acc_wait_modifiers<'a>(
    content: &'a str,
    parser_config: &ParserConfig,
) -> Result<(Option<Expression>, &'a str, bool), AstBuildError> {
    let content = content.trim();
    if content.is_empty() {
        return Err(AstBuildError::ParseFailure(
            "parenthesized OpenACC wait directive requires a wait argument".to_string(),
        ));
    }

    let Some((first, remainder)) = lang::split_once_top_level(content, ':')? else {
        return Ok((None, content, false));
    };
    let first = first.trim();
    let remainder = remainder.trim();

    if acc_keyword_eq(first, "queues", parser_config) {
        if remainder.is_empty() {
            return Err(AstBuildError::ParseFailure(
                "OpenACC wait queues modifier requires a queue list".to_string(),
            ));
        }
        return Ok((None, remainder, true));
    }

    if !acc_keyword_eq(first, "devnum", parser_config) {
        return Err(AstBuildError::ParseFailure(format!(
            "unknown OpenACC wait modifier: {first}"
        )));
    }

    let (device_source, queue_source) =
        lang::split_once_top_level(remainder, ':')?.ok_or_else(|| {
            AstBuildError::ParseFailure(
                "OpenACC wait devnum modifier requires a trailing ':' and queue list".to_string(),
            )
        })?;
    let device_source = device_source.trim();
    if device_source.is_empty() {
        return Err(AstBuildError::ParseFailure(
            "OpenACC wait devnum modifier requires a device expression".to_string(),
        ));
    }
    let devnum = Expression::new_with_legacy_qualified_value(device_source, parser_config)?;

    let queue_source = queue_source.trim();
    let (queue_source, queues_keyword) =
        if let Some((modifier, queues)) = lang::split_once_top_level(queue_source, ':')? {
            if !acc_keyword_eq(modifier.trim(), "queues", parser_config) {
                return Err(AstBuildError::ParseFailure(format!(
                    "unknown OpenACC wait modifier after devnum: {}",
                    modifier.trim()
                )));
            }
            (queues.trim(), true)
        } else {
            (queue_source, false)
        };
    if queue_source.is_empty() {
        return Err(AstBuildError::ParseFailure(
            "OpenACC wait directive requires a queue list after devnum".to_string(),
        ));
    }
    Ok((Some(devnum), queue_source, queues_keyword))
}

fn parse_acc_wait_queue_list(
    source: &str,
    parser_config: &ParserConfig,
) -> Result<Vec<Expression>, AstBuildError> {
    let entries = lang::split_top_level(source, ',', &[('(', ')'), ('[', ']'), ('{', '}')])?;
    entries
        .into_iter()
        .map(|entry| {
            Expression::new_with_legacy_qualified_value(entry.trim(), parser_config)
                .map_err(AstBuildError::from)
        })
        .collect()
}

fn host_keyword_eq(source: &str, keyword: &str, parser_config: &ParserConfig) -> bool {
    if matches!(parser_config.host_language(), HostLanguage::Fortran) {
        source.eq_ignore_ascii_case(keyword)
    } else {
        source == keyword
    }
}

fn acc_keyword_eq(source: &str, keyword: &str, parser_config: &ParserConfig) -> bool {
    host_keyword_eq(source, keyword, parser_config)
}

fn omp_universal_modifier_syntax_is_enabled(
    clause_name: &ClauseName,
    parser_config: &ParserConfig,
) -> bool {
    matches!(clause_name, ClauseName::If | ClauseName::Firstprivate)
        || matches!(
            parser_config.openmp_version_policy(),
            VersionPolicy::Any | VersionPolicy::Exact(OpenMpVersion::V6_0)
        )
}

fn borrowed_clause_argument_view<'view>(
    clause: &'view Clause<'_>,
    argument: &'view str,
) -> Clause<'view> {
    Clause {
        name: Cow::Borrowed(clause.name.as_ref()),
        kind: if argument.trim().is_empty() {
            ClauseKind::Bare
        } else {
            match clause.kind {
                ClauseKind::FlushMemoryOrderArgument(_) => {
                    ClauseKind::FlushMemoryOrderArgument(Cow::Borrowed(argument.trim()))
                }
                _ => ClauseKind::Parenthesized(Cow::Borrowed(argument.trim())),
            }
        },
    }
}

fn parse_firstprivate_with_universal_modifier(
    clause: &Clause<'_>,
    directive_kind: OmpDirectiveKind,
    parser_config: &ParserConfig,
) -> Result<Option<(ClauseData, OmpDirectiveKind)>, AstBuildError> {
    let ClauseKind::Parenthesized(content) = &clause.kind else {
        return Ok(None);
    };
    let Some((prefix, items)) = lang::split_once_top_level(content.as_ref(), ':')? else {
        return Ok(None);
    };

    let mut directive = None;
    let mut saved = false;
    for entry in super::semantic::split_top_level_items(prefix)? {
        let entry = entry.trim();
        if host_keyword_eq(entry, "saved", parser_config) {
            if saved {
                return Err(AstBuildError::ClauseConversion(
                    "duplicate saved firstprivate modifier".to_string(),
                ));
            }
            saved = true;
            continue;
        }
        let Ok(parsed) = parse_directive_name_modifier(entry, parser_config) else {
            return Ok(None);
        };
        if !parser_config.source_extensions()
            && !crate::validation::omp_modifier_names_directive_or_constituent(
                directive_kind,
                parsed,
            )
        {
            return Ok(None);
        }
        if directive.replace(parsed).is_some() {
            return Err(AstBuildError::ClauseConversion(
                "duplicate firstprivate directive-name modifier".to_string(),
            ));
        }
    }

    let Some(directive) = directive else {
        return Ok(None);
    };
    if lang::split_once_top_level(items, ':')?.is_some() {
        return Err(AstBuildError::ClauseConversion(
            "firstprivate modifiers require one comma-separated modifier list before one ':'"
                .to_string(),
        ));
    }
    let items = parse_identifier_list(items.trim(), parser_config)
        .map_err(|error| AstBuildError::ClauseConversion(error.to_string()))?;
    if items.is_empty() {
        return Err(AstBuildError::ClauseConversion(
            "firstprivate clause requires a non-empty variable list".to_string(),
        ));
    }
    Ok(Some((
        ClauseData::Firstprivate {
            modifier: saved.then_some(FirstprivateModifier::Saved),
            items,
        },
        directive,
    )))
}

fn parse_omp_clause_semantics(
    clause: &Clause<'_>,
    clause_name: &ClauseName,
    directive_kind: OmpDirectiveKind,
    parser_config: &ParserConfig,
    source: &LogicalSource<'_>,
) -> Result<(ClauseData, Option<OmpDirectiveKind>), AstBuildError> {
    if let ClauseKind::ReductionClause {
        directive_name_modifier: Some(candidate),
        ..
    } = &clause.kind
    {
        let parsed = parse_directive_name_modifier(candidate, parser_config)
            .map_err(|error| AstBuildError::ClauseConversion(error.to_string()))?;
        if !crate::validation::omp_modifier_names_directive_or_constituent(directive_kind, parsed) {
            return Err(AstBuildError::ClauseConversion(format!(
                "directive-name modifier {} does not name {:?} or one of its constituents",
                parsed.as_str(),
                directive_kind
            )));
        }
        let payload = parse_clause_data(clause, directive_kind, parser_config, source)
            .map_err(|error| AstBuildError::ClauseConversion(error.to_string()))?;
        return Ok((payload, Some(parsed)));
    }
    if omp_universal_modifier_syntax_is_enabled(clause_name, parser_config)
        && matches!(clause_name, ClauseName::Firstprivate)
        && let Some((payload, modifier)) =
            parse_firstprivate_with_universal_modifier(clause, directive_kind, parser_config)?
    {
        return Ok((payload, Some(modifier)));
    }

    if omp_universal_modifier_syntax_is_enabled(clause_name, parser_config) {
        let content = match &clause.kind {
            ClauseKind::Parenthesized(content) | ClauseKind::FlushMemoryOrderArgument(content) => {
                Some(content.as_ref())
            }
            _ => None,
        };
        if let Some(content) = content
            && let Some((candidate, argument)) = lang::split_once_top_level(content, ':')?
            && let Ok(parsed) = parse_directive_name_modifier(candidate.trim(), parser_config)
            && crate::validation::omp_modifier_names_directive_or_constituent(
                directive_kind,
                parsed,
            )
        {
            let view = borrowed_clause_argument_view(clause, argument);
            let payload = match &view.kind {
                ClauseKind::FlushMemoryOrderArgument(content) => {
                    let content = content.as_ref().trim();
                    if content.is_empty() {
                        return Err(AstBuildError::ClauseConversion(
                            "use_semantics requires a non-empty expression".to_string(),
                        ));
                    }
                    ClauseData::MemoryOrder {
                        order: parse_memory_order(clause.name.as_ref(), parser_config)
                            .map_err(|error| AstBuildError::ClauseConversion(error.to_string()))?,
                        use_semantics: Some(Expression::new(content, parser_config)?),
                    }
                }
                _ => parse_clause_data(&view, directive_kind, parser_config, source)
                    .map_err(|error| AstBuildError::ClauseConversion(error.to_string()))?,
            };
            return Ok((payload, Some(parsed)));
        }
    }

    let payload = match &clause.kind {
        ClauseKind::FlushMemoryOrderArgument(content) => {
            let content = content.as_ref().trim();
            if content.is_empty() {
                return Err(AstBuildError::ClauseConversion(
                    "use_semantics requires a non-empty expression".to_string(),
                ));
            }
            ClauseData::MemoryOrder {
                order: parse_memory_order(clause.name.as_ref(), parser_config)
                    .map_err(|error| AstBuildError::ClauseConversion(error.to_string()))?,
                use_semantics: if parser_config.source_extensions() {
                    None
                } else {
                    Some(Expression::new(content, parser_config)?)
                },
            }
        }
        _ => parse_clause_data(clause, directive_kind, parser_config, source)
            .map_err(|error| AstBuildError::ClauseConversion(error.to_string()))?,
    };
    Ok((payload, None))
}

fn convert_clause_to_omp(
    clause: &LocatedClause<'_>,
    parser_config: &ParserConfig,
    directive_kind: OmpDirectiveKind,
    source: &LogicalSource<'_>,
) -> Result<OmpClause, AstBuildError> {
    reject_case_variant_clause_keyword(clause, parser_config)?;
    let clause_name = lookup_clause_name(clause.name.as_ref());
    let (payload, directive_name_modifier) =
        parse_omp_clause_semantics(clause, &clause_name, directive_kind, parser_config, source)?;
    let historical_doacross_has_explicit_iteration = if matches!(clause_name, ClauseName::Depend)
        && matches!(&payload, ClauseData::Doacross { .. })
    {
        match &clause.kind {
            ClauseKind::Parenthesized(content) => {
                lang::split_once_top_level(content.as_ref(), ':')?.is_some()
            }
            _ => false,
        }
    } else {
        false
    };
    let canonical_doacross_source_is_empty = matches!(clause_name, ClauseName::Doacross)
        && matches!(
            &clause.kind,
            ClauseKind::Parenthesized(content)
                if lang::split_once_top_level(content.as_ref(), ':')?
                    .is_some_and(|(_, iteration)| iteration.trim().is_empty())
        );
    let reduction_original_is_positional = matches!(
        &clause.kind,
        ClauseKind::ReductionClause {
            modifiers,
            modifier_items,
            ..
        } if modifiers.iter().zip(modifier_items).any(|(modifier, arguments)| {
            matches!(modifier, crate::parser::clause::ReductionModifier::Original)
                && matches!(arguments.as_slice(), [argument] if !argument.contains('='))
        })
    );
    let (canonical_name, source_alias) = match (&clause_name, &payload) {
        (ClauseName::Depend, ClauseData::Doacross { kind, iteration }) => (
            ClauseName::Doacross,
            Some(
                match (kind, iteration, historical_doacross_has_explicit_iteration) {
                    (
                        crate::ir::DoacrossType::Source,
                        crate::ir::OmpDoacrossIteration::Current,
                        false,
                    ) => OmpClauseSourceAlias::DependSource,
                    (
                        crate::ir::DoacrossType::Source,
                        crate::ir::OmpDoacrossIteration::Current,
                        true,
                    ) => OmpClauseSourceAlias::DependSourceCurrent,
                    (
                        crate::ir::DoacrossType::Sink,
                        crate::ir::OmpDoacrossIteration::Vector(_),
                        _,
                    ) => OmpClauseSourceAlias::DependSink,
                    (
                        crate::ir::DoacrossType::Sink,
                        crate::ir::OmpDoacrossIteration::PreviousCurrent,
                        _,
                    ) => OmpClauseSourceAlias::DependSinkPreviousCurrent,
                    _ => {
                        return Err(AstBuildError::ClauseConversion(
                            "invalid historical doacross payload".to_string(),
                        ));
                    }
                },
            ),
        ),
        (ClauseName::Default, ClauseData::MetadirectiveSelector { .. }) => (
            ClauseName::Otherwise,
            Some(OmpClauseSourceAlias::MetadirectiveDefault),
        ),
        (ClauseName::To, ClauseData::Enter { .. })
            if matches!(
                directive_kind,
                OmpDirectiveKind::DeclareTarget | OmpDirectiveKind::BeginDeclareTarget
            ) =>
        {
            (
                ClauseName::Enter,
                Some(OmpClauseSourceAlias::DeclareTargetTo),
            )
        }
        (ClauseName::ProcBind, ClauseData::ProcBind(ProcBind::Primary))
            if matches!(
                &clause.kind,
                ClauseKind::Parenthesized(content)
                    if host_keyword_eq(content.as_ref().trim(), "master", parser_config)
            ) =>
        {
            (
                ClauseName::ProcBind,
                Some(OmpClauseSourceAlias::ProcBindMaster),
            )
        }
        (
            ClauseName::Doacross,
            ClauseData::Doacross {
                kind: crate::ir::DoacrossType::Source,
                iteration: crate::ir::OmpDoacrossIteration::Current,
            },
        ) if canonical_doacross_source_is_empty => (
            ClauseName::Doacross,
            Some(OmpClauseSourceAlias::DoacrossSourceEmpty),
        ),
        (ClauseName::Reduction, ClauseData::Reduction { .. })
            if reduction_original_is_positional =>
        {
            (
                ClauseName::Reduction,
                Some(OmpClauseSourceAlias::ReductionOriginalPositional),
            )
        }
        (
            ClauseName::Other(_),
            ClauseData::Requirement {
                requirement:
                    crate::ir::RequireModifier::ExtImplementationDefinedRequirement(Some(_)),
                ..
            },
        ) => (ClauseName::ExtImplementationDefinedRequirement, None),
        _ => (clause_name, None),
    };
    let kind = OmpClauseKind::try_from(canonical_name)
        .map_err(|_| AstBuildError::UnsupportedClause(clause.name.as_ref().to_string()))?;
    if !parser_config.source_extensions()
        && let Some(modifier) = directive_name_modifier
        && !crate::validation::omp_clause_applies_to_named_constituent(
            directive_kind,
            modifier,
            kind,
        )
    {
        return Err(AstBuildError::ClauseConversion(format!(
            "clause {} does not apply to named constituent {} of {}",
            kind.as_str(),
            modifier.as_str(),
            directive_kind.as_str()
        )));
    }

    let span = source.span_of(clause.name_source())?;
    Ok(OmpClause::new(
        kind,
        payload,
        directive_name_modifier,
        source_alias,
        if clause.preceded_by_comma() {
            crate::ast::OmpClauseSourceSeparator::Comma
        } else {
            crate::ast::OmpClauseSourceSeparator::Space
        },
        span,
    )?)
}

fn convert_clause_to_acc(
    clause: &LocatedClause<'_>,
    directive_kind: AccDirectiveKind,
    parser_config: &ParserConfig,
    source: &LogicalSource<'_>,
) -> Result<AccClause, AstBuildError> {
    reject_case_variant_clause_keyword(clause, parser_config)?;
    let parsed_name = lookup_clause_name(clause.name.as_ref());
    let (clause_name, source_alias) = if directive_kind == AccDirectiveKind::Update
        && acc_keyword_eq(clause.name.as_ref(), "host", parser_config)
    {
        (
            ClauseName::SelfClause,
            Some(AccClauseSourceAlias::UpdateHost),
        )
    } else {
        match clause.name.as_ref() {
            "pcopy" => (parsed_name.clone(), Some(AccClauseSourceAlias::PCopy)),
            "present_or_copy" => (
                parsed_name.clone(),
                Some(AccClauseSourceAlias::PresentOrCopy),
            ),
            "pcopyin" => (parsed_name.clone(), Some(AccClauseSourceAlias::PCopyIn)),
            "present_or_copyin" => (
                parsed_name.clone(),
                Some(AccClauseSourceAlias::PresentOrCopyIn),
            ),
            "pcopyout" => (parsed_name.clone(), Some(AccClauseSourceAlias::PCopyOut)),
            "present_or_copyout" => (
                parsed_name.clone(),
                Some(AccClauseSourceAlias::PresentOrCopyOut),
            ),
            "pcreate" => (parsed_name.clone(), Some(AccClauseSourceAlias::PCreate)),
            "present_or_create" => (
                parsed_name.clone(),
                Some(AccClauseSourceAlias::PresentOrCreate),
            ),
            _ => (parsed_name.clone(), None),
        }
    };
    let kind = AccClauseKind::try_from(clause_name.clone())
        .map_err(|_| AstBuildError::UnsupportedClause(clause.name.as_ref().to_string()))?;

    Ok(AccClause::new(
        kind,
        build_acc_clause_payload(clause, directive_kind, clause_name, parser_config)?,
        source_alias,
        source.span_of(clause.name_source())?,
    )?)
}

fn reject_case_variant_clause_keyword(
    clause: &LocatedClause<'_>,
    parser_config: &ParserConfig,
) -> Result<(), AstBuildError> {
    let name = clause.name.as_ref();
    if !parser_config.source_extensions()
        && parser_config.host_language() != HostLanguage::Fortran
        && name.bytes().any(|byte| byte.is_ascii_uppercase())
    {
        return Err(AstBuildError::UnsupportedClause(name.to_string()));
    }
    Ok(())
}

fn build_acc_clause_payload(
    clause: &Clause<'_>,
    directive_kind: AccDirectiveKind,
    clause_name: ClauseName,
    parser_config: &ParserConfig,
) -> Result<AccClausePayload, AstBuildError> {
    use ClauseName::*;
    match clause_name {
        Copy | CopyIn | CopyOut => build_acc_copy_clause(clause, parser_config),
        Create => build_acc_create_clause(clause, parser_config),
        Reduction => build_acc_reduction_clause(clause, parser_config),
        Default => build_acc_default_clause(clause, parser_config),
        Wait => build_acc_wait_clause(clause, parser_config),
        Vector => build_acc_vector_clause(clause, parser_config),
        Worker => build_acc_worker_clause(clause, parser_config),
        Gang => build_acc_gang_clause(clause, parser_config),
        Attach => build_acc_data_clause(clause, AccDataKind::Attach, parser_config),
        Detach => build_acc_data_clause(clause, AccDataKind::Detach, parser_config),
        UseDevice => build_acc_data_clause(clause, AccDataKind::UseDevice, parser_config),
        Link => build_acc_data_clause(clause, AccDataKind::Link, parser_config),
        DeviceResident => build_acc_data_clause(clause, AccDataKind::DeviceResident, parser_config),
        Host => Err(AstBuildError::UnsupportedClause(
            "OpenACC host is only a spelling alias for self on update".to_string(),
        )),
        Device => build_acc_data_clause(clause, AccDataKind::Device, parser_config),
        Delete => build_acc_data_clause(clause, AccDataKind::Delete, parser_config),
        DeviceType => build_acc_device_type_clause(clause, parser_config),
        SelfClause if directive_kind == AccDirectiveKind::Update => {
            Ok(AccClausePayload::ItemList {
                kind: AccClauseKind::SelfClause,
                items: require_acc_item_list(clause, parser_config)?,
            })
        }
        SelfClause => build_acc_optional_expression_clause(clause, parser_config),
        Async => build_acc_optional_expression_clause(clause, parser_config),
        Collapse => build_acc_collapse_clause(clause, parser_config),
        NumGangs => build_acc_num_gangs_clause(clause, directive_kind, parser_config),
        Tile => build_acc_tile_clause(clause, parser_config),
        Bind => build_acc_bind_clause(clause, parser_config),
        Indirect => build_acc_indirect_clause(clause, parser_config),
        If | NumWorkers | VectorLength | DefaultAsync | DeviceNum => {
            build_acc_required_expression_clause(clause, parser_config)
        }
        Present | Private | Firstprivate | NoCreate | DevicePtr => {
            build_acc_item_list_clause(clause, parser_config)
        }
        Seq | Independent | Auto | NoHost | Finalize | IfPresent | Capture | Read | Update
        | Write => build_acc_bare_clause(clause),
        other => Err(AstBuildError::UnsupportedClause(format!(
            "OpenACC clause has no typed payload implementation: {other:?}"
        ))),
    }
}

fn build_acc_default_clause(
    clause: &Clause<'_>,
    parser_config: &ParserConfig,
) -> Result<AccClausePayload, AstBuildError> {
    let value = match &clause.kind {
        ClauseKind::Parenthesized(content) => content.as_ref().trim(),
        _ => {
            return Err(AstBuildError::ClauseConversion(
                "OpenACC default clause requires one parenthesized value".to_string(),
            ));
        }
    };

    let kind = if acc_keyword_eq(value, "none", parser_config) {
        AccDefaultKind::None
    } else if acc_keyword_eq(value, "present", parser_config) {
        AccDefaultKind::Present
    } else {
        match value {
            "" => {
                return Err(AstBuildError::ClauseConversion(
                    "OpenACC default clause requires 'none' or 'present'".to_string(),
                ));
            }
            other => {
                return Err(AstBuildError::ClauseConversion(format!(
                    "unknown OpenACC default value: {other}"
                )));
            }
        }
    };

    Ok(AccClausePayload::Default(kind))
}

fn build_acc_collapse_clause(
    clause: &Clause<'_>,
    parser_config: &ParserConfig,
) -> Result<AccClausePayload, AstBuildError> {
    let ClauseKind::Parenthesized(content) = &clause.kind else {
        return Err(AstBuildError::ClauseConversion(
            "OpenACC collapse clause requires a parenthesized count".to_string(),
        ));
    };
    let text = content.as_ref().trim();
    let (force, count) = match crate::ir::lang::split_once_top_level(text, ':')? {
        Some((modifier, count)) if acc_keyword_eq(modifier.trim(), "force", parser_config) => {
            (true, count.trim())
        }
        Some((modifier, _)) => {
            return Err(AstBuildError::ClauseConversion(format!(
                "unknown OpenACC collapse modifier: {}",
                modifier.trim()
            )));
        }
        None => (false, text),
    };
    if count.is_empty() {
        return Err(AstBuildError::ClauseConversion(
            "OpenACC collapse clause requires a count expression".to_string(),
        ));
    }
    Ok(AccClausePayload::Collapse(AccCollapseClause::new(
        force,
        parse_acc_single_expression(count, parser_config, "OpenACC collapse clause")?,
    )))
}

fn build_acc_num_gangs_clause(
    clause: &Clause<'_>,
    directive_kind: AccDirectiveKind,
    parser_config: &ParserConfig,
) -> Result<AccClausePayload, AstBuildError> {
    let ClauseKind::Parenthesized(content) = &clause.kind else {
        return Err(AstBuildError::ClauseConversion(
            "OpenACC num_gangs clause requires a parenthesized expression list".to_string(),
        ));
    };
    let parts = super::semantic::split_top_level_items(content.as_ref())
        .map_err(|error| AstBuildError::ClauseConversion(error.to_string()))?;
    let maximum = if matches!(
        directive_kind,
        AccDirectiveKind::Kernels | AccDirectiveKind::KernelsLoop
    ) {
        1
    } else {
        3
    };
    if parts.is_empty() || parts.len() > maximum {
        return Err(AstBuildError::ClauseConversion(if maximum == 1 {
            "OpenACC num_gangs on kernels requires exactly one non-empty expression".to_string()
        } else {
            "OpenACC num_gangs requires between one and three non-empty expressions".to_string()
        }));
    }
    let values = parts
        .into_iter()
        .map(|part| Expression::new(part.trim(), parser_config).map_err(AstBuildError::from))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(AccClausePayload::NumGangs(values))
}

fn build_acc_tile_clause(
    clause: &Clause<'_>,
    parser_config: &ParserConfig,
) -> Result<AccClausePayload, AstBuildError> {
    fn obvious_positive_integer(expression: &Expression) -> Result<(), AstBuildError> {
        fn classify(expression: &crate::host::Expr) -> Option<Result<u128, ()>> {
            match &expression.kind {
                ExprKind::Parenthesized(inner) => classify(inner),
                ExprKind::Literal(Literal::Integer(value)) => Some(Ok(value.value)),
                ExprKind::Unary {
                    op: crate::host::UnaryOp::Plus,
                    operand,
                } => classify(operand),
                ExprKind::Unary {
                    op: crate::host::UnaryOp::Minus,
                    operand,
                } if classify(operand).is_some() => Some(Err(())),
                ExprKind::Literal(_) => Some(Err(())),
                _ => None,
            }
        }
        if matches!(classify(expression.ast()), Some(Ok(0) | Err(()))) {
            return Err(AstBuildError::ClauseConversion(
                "OpenACC tile size must be a positive integer expression".to_string(),
            ));
        }
        Ok(())
    }

    let ClauseKind::Parenthesized(content) = &clause.kind else {
        return Err(AstBuildError::ClauseConversion(
            "OpenACC tile clause requires a parenthesized size list".to_string(),
        ));
    };
    let sizes = super::semantic::split_top_level_items(content.as_ref())
        .map_err(|error| AstBuildError::ClauseConversion(error.to_string()))?
        .into_iter()
        .map(|source| {
            let source = source.trim();
            if source == "*" {
                Ok(AccSizeExpression::Automatic)
            } else {
                let expression = Expression::new(source, parser_config)?;
                obvious_positive_integer(&expression)?;
                Ok(AccSizeExpression::Expression(Box::new(expression)))
            }
        })
        .collect::<Result<Vec<_>, AstBuildError>>()?;
    if sizes.is_empty() {
        return Err(AstBuildError::ClauseConversion(
            "OpenACC tile clause requires at least one size".to_string(),
        ));
    }
    Ok(AccClausePayload::Tile(sizes))
}

fn parse_acc_single_expression(
    source: &str,
    parser_config: &ParserConfig,
    subject: &str,
) -> Result<Expression, AstBuildError> {
    let entries = super::semantic::split_top_level_items(source)
        .map_err(|error| AstBuildError::ClauseConversion(error.to_string()))?;
    if entries.len() != 1 {
        return Err(AstBuildError::ClauseConversion(format!(
            "{subject} requires exactly one expression"
        )));
    }
    Expression::new(entries[0].trim(), parser_config).map_err(AstBuildError::from)
}

fn build_acc_copy_clause(
    clause: &Clause<'_>,
    parser_config: &ParserConfig,
) -> Result<AccClausePayload, AstBuildError> {
    let keyword = clause.name.as_ref();
    let kind = match keyword {
        "copy" | "pcopy" | "present_or_copy" => AccCopyKind::Copy,
        "copyin" | "pcopyin" | "present_or_copyin" => AccCopyKind::CopyIn,
        "copyout" | "pcopyout" | "present_or_copyout" => AccCopyKind::CopyOut,
        other => {
            return Err(AstBuildError::UnsupportedClause(format!(
                "unknown OpenACC copy clause keyword: {other}"
            )));
        }
    };

    let (modifiers, variables) = match &clause.kind {
        ClauseKind::Parenthesized(content) => parse_acc_data_clause_content(
            content.as_ref(),
            match kind {
                AccCopyKind::Copy => &[
                    AccDataModifier::Always,
                    AccDataModifier::AlwaysIn,
                    AccDataModifier::AlwaysOut,
                    AccDataModifier::Capture,
                ],
                AccCopyKind::CopyIn => &[
                    AccDataModifier::Always,
                    AccDataModifier::AlwaysIn,
                    AccDataModifier::Readonly,
                    AccDataModifier::Capture,
                ],
                AccCopyKind::CopyOut => &[
                    AccDataModifier::Always,
                    AccDataModifier::AlwaysOut,
                    AccDataModifier::Zero,
                    AccDataModifier::Capture,
                ],
            },
            parser_config,
        )?,
        _ => {
            return Err(AstBuildError::UnsupportedClause(
                "copy clause requires a variable list".to_string(),
            ));
        }
    };

    Ok(AccClausePayload::Copy(AccCopyClause::new(
        kind, modifiers, variables,
    )?))
}

fn build_acc_create_clause(
    clause: &Clause<'_>,
    parser_config: &ParserConfig,
) -> Result<AccClausePayload, AstBuildError> {
    let keyword = clause.name.as_ref();
    let kind = match keyword {
        "create" | "pcreate" | "present_or_create" => AccCreateKind::Create,
        other => {
            return Err(AstBuildError::UnsupportedClause(format!(
                "unknown OpenACC create clause keyword: {other}"
            )));
        }
    };

    let (modifiers, variables) = match &clause.kind {
        ClauseKind::Parenthesized(content) => parse_acc_data_clause_content(
            content.as_ref(),
            &[AccDataModifier::Zero, AccDataModifier::Capture],
            parser_config,
        )?,
        _ => {
            return Err(AstBuildError::UnsupportedClause(
                "create clause requires a variable list".to_string(),
            ));
        }
    };

    Ok(AccClausePayload::Create(AccCreateClause::new(
        kind, modifiers, variables,
    )?))
}

fn parse_acc_data_clause_content(
    content: &str,
    allowed: &[AccDataModifier],
    parser_config: &ParserConfig,
) -> Result<(Vec<AccDataModifier>, Vec<ClauseItem>), AstBuildError> {
    let content = content.trim();
    let (modifier_source, item_source) = match crate::ir::lang::split_once_top_level(content, ':')?
    {
        Some((modifiers, items)) => (Some(modifiers.trim()), items.trim()),
        None => (None, content),
    };
    if item_source.is_empty() {
        return Err(AstBuildError::ClauseConversion(
            "OpenACC data clause requires a non-empty variable list".to_string(),
        ));
    }

    let mut modifiers = Vec::new();
    if let Some(source) = modifier_source {
        if source.is_empty() {
            return Err(AstBuildError::ClauseConversion(
                "OpenACC data modifier list must not be empty".to_string(),
            ));
        }
        for raw in source.split(',') {
            let raw_modifier = raw.trim();
            let modifier = if acc_keyword_eq(raw_modifier, "always", parser_config) {
                AccDataModifier::Always
            } else if acc_keyword_eq(raw_modifier, "alwaysin", parser_config) {
                AccDataModifier::AlwaysIn
            } else if acc_keyword_eq(raw_modifier, "alwaysout", parser_config) {
                AccDataModifier::AlwaysOut
            } else if acc_keyword_eq(raw_modifier, "capture", parser_config) {
                AccDataModifier::Capture
            } else if acc_keyword_eq(raw_modifier, "readonly", parser_config) {
                AccDataModifier::Readonly
            } else if acc_keyword_eq(raw_modifier, "zero", parser_config) {
                AccDataModifier::Zero
            } else {
                match raw_modifier {
                    "" => {
                        return Err(AstBuildError::ClauseConversion(
                            "OpenACC data modifier list contains an empty entry".to_string(),
                        ));
                    }
                    other => {
                        return Err(AstBuildError::ClauseConversion(format!(
                            "unknown OpenACC data modifier: {other}"
                        )));
                    }
                }
            };
            if !allowed.contains(&modifier) && !parser_config.source_extensions() {
                return Err(AstBuildError::ClauseConversion(format!(
                    "OpenACC data modifier {modifier:?} is not allowed on this clause"
                )));
            }
            if modifiers.contains(&modifier) && !parser_config.source_extensions() {
                return Err(AstBuildError::ClauseConversion(format!(
                    "duplicate OpenACC data modifier: {modifier:?}"
                )));
            }
            modifiers.push(modifier);
        }
    }

    let items = parse_acc_identifier_list(item_source, parser_config)?;
    if items.is_empty() {
        return Err(AstBuildError::ClauseConversion(
            "OpenACC data clause requires a non-empty variable list".to_string(),
        ));
    }
    Ok((modifiers, items))
}

fn build_acc_reduction_clause(
    clause: &Clause<'_>,
    parser_config: &ParserConfig,
) -> Result<AccClausePayload, AstBuildError> {
    let ClauseKind::Parenthesized(content) = &clause.kind else {
        return Err(AstBuildError::ClauseConversion(
            "OpenACC reduction clause requires parenthesized content".to_string(),
        ));
    };
    let (operator_source, variables_source) = lang::split_once_top_level(content.as_ref(), ':')?
        .ok_or_else(|| {
            AstBuildError::ClauseConversion(
                "OpenACC reduction clause requires 'operator: variable-list'".to_string(),
            )
        })?;
    let operator_source = operator_source.trim();
    let op = match parser_config.host_language() {
        HostLanguage::C | HostLanguage::Cpp => match operator_source {
            "+" => Some(AccReductionOperator::Add),
            "-" if parser_config.source_extensions() => Some(AccReductionOperator::Sub),
            "*" => Some(AccReductionOperator::Mul),
            "&" => Some(AccReductionOperator::BitAnd),
            "|" => Some(AccReductionOperator::BitOr),
            "^" => Some(AccReductionOperator::BitXor),
            "&&" => Some(AccReductionOperator::LogAnd),
            "||" => Some(AccReductionOperator::LogOr),
            value if acc_keyword_eq(value, "max", parser_config) => Some(AccReductionOperator::Max),
            value if acc_keyword_eq(value, "min", parser_config) => Some(AccReductionOperator::Min),
            _ => None,
        },
        HostLanguage::Fortran => match operator_source {
            "+" => Some(AccReductionOperator::Add),
            "-" if parser_config.source_extensions() => Some(AccReductionOperator::Sub),
            "*" => Some(AccReductionOperator::Mul),
            "&" if parser_config.source_extensions() => Some(AccReductionOperator::BitAnd),
            "|" if parser_config.source_extensions() => Some(AccReductionOperator::BitOr),
            "^" if parser_config.source_extensions() => Some(AccReductionOperator::BitXor),
            "&&" if parser_config.source_extensions() => Some(AccReductionOperator::LogAnd),
            "||" if parser_config.source_extensions() => Some(AccReductionOperator::LogOr),
            value if acc_keyword_eq(value, "max", parser_config) => Some(AccReductionOperator::Max),
            value if acc_keyword_eq(value, "min", parser_config) => Some(AccReductionOperator::Min),
            value if acc_keyword_eq(value, ".and.", parser_config) => {
                Some(AccReductionOperator::FortAnd)
            }
            value if acc_keyword_eq(value, ".or.", parser_config) => {
                Some(AccReductionOperator::FortOr)
            }
            value if acc_keyword_eq(value, ".eqv.", parser_config) => {
                Some(AccReductionOperator::FortEqv)
            }
            value if acc_keyword_eq(value, ".neqv.", parser_config) => {
                Some(AccReductionOperator::FortNeqv)
            }
            value if acc_keyword_eq(value, "iand", parser_config) => {
                Some(AccReductionOperator::FortIand)
            }
            value if acc_keyword_eq(value, "ior", parser_config) => {
                Some(AccReductionOperator::FortIor)
            }
            value if acc_keyword_eq(value, "ieor", parser_config) => {
                Some(AccReductionOperator::FortIeor)
            }
            _ => None,
        },
    }
    .ok_or_else(|| {
        AstBuildError::ClauseConversion(if operator_source.is_empty() {
            "OpenACC reduction operator must not be empty".to_string()
        } else {
            format!(
                "OpenACC reduction operator {operator_source:?} is not valid for {:?}",
                parser_config.host_language()
            )
        })
    })?;
    let variables = parse_acc_identifier_list(variables_source.trim(), parser_config)?;
    if variables.is_empty() {
        return Err(AstBuildError::ClauseConversion(
            "OpenACC reduction clause requires a non-empty variable list".to_string(),
        ));
    }
    if variables
        .iter()
        .any(|item| matches!(item, ClauseItem::FortranCommonBlock(_)))
    {
        return Err(AstBuildError::ClauseConversion(
            "OpenACC reduction clauses do not accept Fortran common block names".to_string(),
        ));
    }

    Ok(AccClausePayload::Reduction(AccReductionClause::new(
        op, variables,
    )?))
}

fn build_acc_wait_clause(
    clause: &Clause<'_>,
    parser_config: &ParserConfig,
) -> Result<AccClausePayload, AstBuildError> {
    let content = match &clause.kind {
        ClauseKind::Parenthesized(content) => content.as_ref().trim(),
        ClauseKind::Bare => {
            return Ok(AccClausePayload::Wait(AccWaitClause::new(
                None,
                Vec::new(),
                false,
            )?));
        }
        _ => {
            return Err(AstBuildError::ClauseConversion(
                "OpenACC wait clause has an invalid payload shape".to_string(),
            ));
        }
    };
    if content.is_empty() {
        return Err(AstBuildError::ClauseConversion(
            "OpenACC wait parentheses require at least one queue expression".to_string(),
        ));
    }

    let (devnum, queue_source, queues_keyword) = parse_acc_wait_modifiers(content, parser_config)?;
    let queue_exprs = parse_acc_wait_queue_list(queue_source, parser_config)?;

    Ok(AccClausePayload::Wait(AccWaitClause::new(
        devnum,
        queue_exprs,
        queues_keyword,
    )?))
}

fn build_acc_vector_clause(
    clause: &Clause<'_>,
    parser_config: &ParserConfig,
) -> Result<AccClausePayload, AstBuildError> {
    let vector = match &clause.kind {
        ClauseKind::Bare => AccVectorClause::Bare,
        ClauseKind::Parenthesized(content) => {
            let content = content.as_ref().trim();
            if content.is_empty() {
                return Err(AstBuildError::ClauseConversion(
                    "OpenACC vector parentheses require an expression".to_string(),
                ));
            }
            if let Some((candidate, value_source)) = lang::split_once_top_level(content, ':')? {
                if acc_keyword_eq(candidate.trim(), "length", parser_config) {
                    AccVectorClause::Length(Box::new(parse_acc_single_expression(
                        value_source.trim(),
                        parser_config,
                        "OpenACC vector length modifier",
                    )?))
                } else {
                    return Err(AstBuildError::ClauseConversion(format!(
                        "unknown OpenACC vector modifier: {}",
                        candidate.trim()
                    )));
                }
            } else {
                AccVectorClause::Expression(Box::new(parse_acc_single_expression(
                    content,
                    parser_config,
                    "OpenACC vector clause",
                )?))
            }
        }
        _ => {
            return Err(AstBuildError::ClauseConversion(
                "OpenACC vector clause has an invalid payload shape".to_string(),
            ));
        }
    };
    Ok(AccClausePayload::Vector(vector))
}

fn build_acc_worker_clause(
    clause: &Clause<'_>,
    parser_config: &ParserConfig,
) -> Result<AccClausePayload, AstBuildError> {
    let worker = match &clause.kind {
        ClauseKind::Bare => AccWorkerClause::Bare,
        ClauseKind::Parenthesized(content) => {
            let content = content.as_ref().trim();
            if content.is_empty() {
                return Err(AstBuildError::ClauseConversion(
                    "OpenACC worker parentheses require an expression".to_string(),
                ));
            }
            if let Some((candidate, value_source)) = lang::split_once_top_level(content, ':')? {
                if acc_keyword_eq(candidate.trim(), "num", parser_config) {
                    AccWorkerClause::Num(Box::new(parse_acc_single_expression(
                        value_source.trim(),
                        parser_config,
                        "OpenACC worker num modifier",
                    )?))
                } else {
                    return Err(AstBuildError::ClauseConversion(format!(
                        "unknown OpenACC worker modifier: {}",
                        candidate.trim()
                    )));
                }
            } else {
                AccWorkerClause::Expression(Box::new(parse_acc_single_expression(
                    content,
                    parser_config,
                    "OpenACC worker clause",
                )?))
            }
        }
        _ => {
            return Err(AstBuildError::ClauseConversion(
                "OpenACC worker clause has an invalid payload shape".to_string(),
            ));
        }
    };
    Ok(AccClausePayload::Worker(worker))
}

fn build_acc_bind_clause(
    clause: &Clause<'_>,
    parser_config: &ParserConfig,
) -> Result<AccClausePayload, AstBuildError> {
    let content = match &clause.kind {
        ClauseKind::Parenthesized(content) => content.as_ref().trim(),
        _ => {
            return Err(AstBuildError::ClauseConversion(
                "OpenACC bind clause requires a parenthesized name or string literal".to_string(),
            ));
        }
    };
    if content.is_empty() {
        return Err(AstBuildError::ClauseConversion(
            "OpenACC bind parentheses require a name or string literal".to_string(),
        ));
    }

    let expression = parse_acc_single_expression(content, parser_config, "OpenACC bind clause")?;
    let target = match &expression.ast().kind {
        ExprKind::Name(name) if !name.global && name.segments.len() == 1 => {
            AccBindTarget::Name(name.segments[0].clone())
        }
        ExprKind::Literal(Literal::String(literal)) => {
            AccBindTarget::StringLiteral(literal.clone())
        }
        _ => {
            return Err(AstBuildError::ClauseConversion(
                "OpenACC bind target must be one host-language name or string literal".to_string(),
            ));
        }
    };
    Ok(AccClausePayload::Bind(target))
}

fn build_acc_indirect_clause(
    clause: &Clause<'_>,
    parser_config: &ParserConfig,
) -> Result<AccClausePayload, AstBuildError> {
    if !parser_config.source_extensions() {
        return Err(AstBuildError::UnsupportedClause("indirect".to_string()));
    }
    if matches!(clause.kind, ClauseKind::Bare) {
        return Ok(AccClausePayload::Indirect(None));
    }
    let AccClausePayload::Bind(target) = build_acc_bind_clause(clause, parser_config)? else {
        unreachable!("bind builder returned a non-bind payload")
    };
    Ok(AccClausePayload::Indirect(Some(target)))
}

fn build_acc_gang_clause(
    clause: &Clause<'_>,
    parser_config: &ParserConfig,
) -> Result<AccClausePayload, AstBuildError> {
    let arguments = match &clause.kind {
        ClauseKind::Bare => Vec::new(),
        ClauseKind::Parenthesized(content) => {
            let content = content.as_ref().trim();
            if content.is_empty() {
                return Err(AstBuildError::ClauseConversion(
                    "OpenACC gang parentheses require an expression".to_string(),
                ));
            }
            super::semantic::split_top_level_items(content)
                .map_err(|error| AstBuildError::ClauseConversion(error.to_string()))?
                .into_iter()
                .map(|entry| {
                    let entry = entry.trim();
                    if let Some((candidate, value)) = lang::split_once_top_level(entry, ':')? {
                        let candidate = candidate.trim();
                        let value = value.trim();
                        if acc_keyword_eq(candidate, "num", parser_config) {
                            if value.is_empty() {
                                return Err(AstBuildError::ClauseConversion(
                                    "OpenACC gang num argument requires an expression".to_string(),
                                ));
                            }
                            return Ok(AccGangArgument::Num(Box::new(Expression::new(
                                value,
                                parser_config,
                            )?)));
                        }
                        if acc_keyword_eq(candidate, "dim", parser_config) {
                            if value.is_empty() {
                                return Err(AstBuildError::ClauseConversion(
                                    "OpenACC gang dim argument requires an expression".to_string(),
                                ));
                            }
                            return Ok(AccGangArgument::Dim(Box::new(Expression::new(
                                value,
                                parser_config,
                            )?)));
                        }
                        if acc_keyword_eq(candidate, "static", parser_config) {
                            if value.is_empty() {
                                return Err(AstBuildError::ClauseConversion(
                                    "OpenACC gang static argument requires a size".to_string(),
                                ));
                            }
                            let size = if value == "*" {
                                AccSizeExpression::Automatic
                            } else {
                                AccSizeExpression::Expression(Box::new(Expression::new(
                                    value,
                                    parser_config,
                                )?))
                            };
                            return Ok(AccGangArgument::Static(size));
                        }
                        return Err(AstBuildError::ClauseConversion(format!(
                            "unknown OpenACC gang argument: {candidate}"
                        )));
                    }

                    if entry == "*" {
                        Err(AstBuildError::ClauseConversion(
                            "OpenACC automatic gang size requires the static argument".to_string(),
                        ))
                    } else {
                        Ok(AccGangArgument::Positional(Box::new(Expression::new(
                            entry,
                            parser_config,
                        )?)))
                    }
                })
                .collect::<Result<Vec<_>, AstBuildError>>()?
        }
        _ => {
            return Err(AstBuildError::ClauseConversion(
                "OpenACC gang clause has an invalid payload shape".to_string(),
            ));
        }
    };
    if !parser_config.source_extensions() {
        let mut saw_num = false;
        let mut saw_dim = false;
        let mut saw_static = false;
        for argument in &arguments {
            let duplicate = match argument {
                AccGangArgument::Positional(_) | AccGangArgument::Num(_) => {
                    std::mem::replace(&mut saw_num, true)
                }
                AccGangArgument::Dim(_) => std::mem::replace(&mut saw_dim, true),
                AccGangArgument::Static(_) => std::mem::replace(&mut saw_static, true),
            };
            if duplicate {
                return Err(AstBuildError::ClauseConversion(
                    "OpenACC gang clause must not repeat an argument kind".to_string(),
                ));
            }
        }
    }
    Ok(AccClausePayload::Gang(AccGangClause::new(arguments)))
}

fn build_acc_data_clause(
    clause: &Clause<'_>,
    kind: AccDataKind,
    parser_config: &ParserConfig,
) -> Result<AccClausePayload, AstBuildError> {
    let variables = require_acc_item_list(clause, parser_config)?;
    Ok(AccClausePayload::Data(AccDataClause::new(kind, variables)?))
}

fn build_acc_device_type_clause(
    clause: &Clause<'_>,
    parser_config: &ParserConfig,
) -> Result<AccClausePayload, AstBuildError> {
    let raw_values = match &clause.kind {
        ClauseKind::Parenthesized(content) => {
            super::semantic::split_top_level_items(content.as_ref())
                .map_err(|error| AstBuildError::ClauseConversion(error.to_string()))?
        }
        _ => {
            return Err(AstBuildError::ClauseConversion(
                "OpenACC device_type clause requires a parenthesized name list".to_string(),
            ));
        }
    };
    if raw_values.is_empty() || raw_values.iter().any(|value| value.trim().is_empty()) {
        return Err(AstBuildError::ClauseConversion(
            "OpenACC device_type clause requires at least one non-empty name".to_string(),
        ));
    }
    let values = raw_values
        .into_iter()
        .map(|value| {
            let value = value.trim();
            if value == "*" {
                Ok(AccDeviceType::Wildcard)
            } else if acc_keyword_eq(value, "host", parser_config) {
                Ok(AccDeviceType::Host)
            } else if acc_keyword_eq(value, "multicore", parser_config) {
                Ok(AccDeviceType::Multicore)
            } else if acc_keyword_eq(value, "default", parser_config) {
                Ok(AccDeviceType::Default)
            } else if ["host", "multicore", "default"]
                .iter()
                .any(|keyword| value.eq_ignore_ascii_case(keyword))
            {
                Err(AstBuildError::ClauseConversion(format!(
                    "OpenACC device type {value:?} is case-sensitive in C and C++"
                )))
            } else {
                Ok(AccDeviceType::Named(Identifier::new(value)?))
            }
        })
        .collect::<Result<Vec<_>, AstBuildError>>()?;
    if values.len() > 1
        && values
            .iter()
            .any(|value| matches!(value, AccDeviceType::Wildcard))
    {
        return Err(AstBuildError::ClauseConversion(
            "OpenACC device_type wildcard must be the only list entry".to_string(),
        ));
    }
    Ok(AccClausePayload::DeviceType(values))
}

fn build_acc_optional_expression_clause(
    clause: &Clause<'_>,
    parser_config: &ParserConfig,
) -> Result<AccClausePayload, AstBuildError> {
    let kind = AccClauseKind::try_from(lookup_clause_name(clause.name.as_ref()))
        .map_err(|_| AstBuildError::UnsupportedClause(clause.name.as_ref().to_string()))?;
    match &clause.kind {
        ClauseKind::Bare => Ok(AccClausePayload::Bare { kind }),
        ClauseKind::Parenthesized(content) => {
            let content = content.as_ref().trim();
            if content.is_empty() {
                return Err(AstBuildError::ClauseConversion(format!(
                    "OpenACC {} parentheses require an expression",
                    clause.name.as_ref()
                )));
            }
            Ok(AccClausePayload::Expression {
                kind,
                value: parse_acc_single_expression(
                    content,
                    parser_config,
                    &format!("OpenACC {} clause", clause.name.as_ref()),
                )?,
            })
        }
        _ => Err(AstBuildError::ClauseConversion(format!(
            "OpenACC {} clause has an invalid payload shape",
            clause.name.as_ref()
        ))),
    }
}

fn build_acc_required_expression_clause(
    clause: &Clause<'_>,
    parser_config: &ParserConfig,
) -> Result<AccClausePayload, AstBuildError> {
    match build_acc_optional_expression_clause(clause, parser_config)? {
        AccClausePayload::Bare { .. } => Err(AstBuildError::ClauseConversion(format!(
            "OpenACC {} clause requires a parenthesized expression",
            clause.name.as_ref()
        ))),
        payload => Ok(payload),
    }
}

fn build_acc_item_list_clause(
    clause: &Clause<'_>,
    parser_config: &ParserConfig,
) -> Result<AccClausePayload, AstBuildError> {
    let items = require_acc_item_list(clause, parser_config)?;
    if matches!(
        lookup_clause_name(clause.name.as_ref()),
        ClauseName::DevicePtr | ClauseName::Present
    ) && items
        .iter()
        .any(|item| matches!(item, ClauseItem::FortranCommonBlock(_)))
    {
        return Err(AstBuildError::ClauseConversion(format!(
            "OpenACC {} clauses do not accept Fortran common block names",
            clause.name.as_ref()
        )));
    }
    let kind = AccClauseKind::try_from(lookup_clause_name(clause.name.as_ref()))
        .map_err(|_| AstBuildError::UnsupportedClause(clause.name.as_ref().to_string()))?;
    Ok(AccClausePayload::ItemList { kind, items })
}

fn require_acc_item_list(
    clause: &Clause<'_>,
    parser_config: &ParserConfig,
) -> Result<Vec<ClauseItem>, AstBuildError> {
    let source = match &clause.kind {
        ClauseKind::Parenthesized(content) => content.as_ref().trim(),
        _ => {
            return Err(AstBuildError::ClauseConversion(format!(
                "OpenACC {} clause requires a parenthesized variable list",
                clause.name.as_ref()
            )));
        }
    };

    let variables = parse_acc_identifier_list(source, parser_config)?;
    if variables.is_empty() {
        return Err(AstBuildError::ClauseConversion(format!(
            "OpenACC {} clause requires a non-empty variable list",
            clause.name.as_ref()
        )));
    }
    Ok(variables)
}

fn build_acc_bare_clause(clause: &Clause<'_>) -> Result<AccClausePayload, AstBuildError> {
    let kind = AccClauseKind::try_from(lookup_clause_name(clause.name.as_ref()))
        .map_err(|_| AstBuildError::UnsupportedClause(clause.name.as_ref().to_string()))?;
    match clause.kind {
        ClauseKind::Bare => Ok(AccClausePayload::Bare { kind }),
        _ => Err(AstBuildError::ClauseConversion(format!(
            "OpenACC {} clause does not accept arguments",
            clause.name.as_ref()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_basic_openmp_ast() {
        let config = ParserConfig::c();
        let ast = crate::parser::openmp::parser()
            .parse_ast("#pragma omp parallel nowait", &config)
            .expect("ast conversion should succeed");

        match ast {
            RoupDirective::OpenMp(omp) => {
                assert!(matches!(omp.kind(), OmpDirectiveKind::Parallel));
                assert_eq!(omp.clauses().len(), 1);
            }
            _ => panic!("expected OpenMP directive"),
        }
    }

    #[test]
    fn parses_reduction_directive() {
        let parser = crate::parser::openmp::parser();
        let result = parser.parse_ast("#pragma omp parallel reduction(+:sum)", &ParserConfig::c());
        assert!(result.is_ok(), "reduction parse failed: {:?}", result.err());
    }

    #[test]
    fn parses_named_openacc_device_type_as_identifier() {
        let parser = crate::parser::openacc::parser();
        let ast = parser
            .parse_ast(
                "#pragma acc parallel device_type(nvidia, host)",
                &ParserConfig::c(),
            )
            .expect("OpenACC AST conversion should succeed");

        let RoupDirective::OpenAcc(acc) = ast else {
            panic!("expected OpenACC directive");
        };
        let AccClausePayload::DeviceType(values) = acc.clauses()[0].payload() else {
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
        let config = ParserConfig::c();
        let ast = crate::parser::openmp::parser()
            .parse_ast(
                "#pragma omp declare reduction(min : struct point : minproc(&omp_out, &omp_in)) initializer(omp_priv = init(1, 2))",
                &config,
            )
            .expect("declare reduction should parse");
        let RoupDirective::OpenMp(directive) = ast else {
            panic!("expected OpenMP directive");
        };
        let Some(OmpDirectiveParameter::DeclareReduction(parsed)) = directive.parameter() else {
            panic!("expected typed declare-reduction parameter");
        };
        assert_eq!(parsed.identifier().to_string(), "min");
        assert_eq!(
            parsed
                .type_names()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            vec!["struct point"]
        );
        assert_eq!(parsed.combiner().to_string(), "minproc(&omp_out, &omp_in)");
        assert_eq!(
            parsed.initializer().map(ToString::to_string),
            Some("omp_priv = init(1, 2)".to_string())
        );
    }

    #[test]
    fn declare_reduction_keeps_a_typed_c_braced_initializer() {
        let ast = crate::parser::openmp::parser()
            .parse_ast(
                "#pragma omp declare reduction(min : struct point : minproc(&omp_out, &omp_in)) initializer(omp_priv = { 1, 2 })",
                &ParserConfig::c(),
            )
            .expect("standard C braced initializer should parse");
        let RoupDirective::OpenMp(directive) = ast else {
            panic!("expected OpenMP directive");
        };
        let Some(OmpDirectiveParameter::DeclareReduction(parsed)) = directive.parameter() else {
            panic!("expected typed declare-reduction parameter");
        };
        assert!(matches!(
            parsed.initializer(),
            Some(OmpReductionInitializer::CAssignment(OmpInitializerValue::Braced(
                initializer
            ))) if initializer.elements().len() == 2
        ));
    }
}
