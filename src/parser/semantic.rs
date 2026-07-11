//! Parser-boundary semantic payload parsers.
//!
//! This module is the only runtime boundary that may interpret directive and
//! clause payload text as OpenMP/OpenACC keywords. IR conversion code consumes
//! the typed results from here instead of branching on raw strings.

use std::{borrow::Cow, collections::HashSet};

use crate::ast::{
    OmpDirective, OmpDirectiveKind, OmpFortranReductionIntrinsic, OmpIdExpression,
    OmpInductionIdentifier, OmpReductionIdentifier, OmpSelector, OmpSelectorConstruct,
    OmpSelectorDeviceTrait, OmpSelectorEntry, OmpSelectorExtensionProperty,
    OmpSelectorExtensionTrait, OmpSelectorImplementationTrait, OmpSelectorImplementationTraitKind,
    OmpSelectorNameListKind, OmpSelectorNameListTrait, OmpSelectorRequirement,
    OmpSelectorTraitValue,
};
use crate::host::{BinaryOp, ExprKind, HostLanguage, Literal, UnaryOp};
use crate::ir::{
    AdjustArgsModifier, AllocateSourceSyntax, AtKind, AtomicOp, BindModifier, ClauseData,
    ClauseItem, ConversionError, DefaultKind, DefaultmapBehavior, DefaultmapCategory,
    DependIterator, DependType, DepobjUpdateDependence, DeviceModifier, DeviceType, DoacrossType,
    Expression, ExtendedAtomicKind, FirstprivateModifier, GrainsizeModifier, Identifier, LValue,
    LastprivateModifier, LinearModifier, LinearSourceSyntax, MapModifier, MapRefKind, MapType,
    MapTypeSpelling, MemoryOrder, MemscopeKind, NumTasksModifier, OmpAppendOperation,
    OmpApplyLoopKind, OmpApplyLoopModifier, OmpCount, OmpDependence, OmpDoacrossIteration,
    OmpDoacrossOffset, OmpDoacrossVectorItem, OmpForeignRuntimeIdentifier, OmpInductionModifier,
    OmpInteropInitModifiers, OmpInteropType, OmpLocator, OmpMemorySpace, OmpParameterListItem,
    OmpParameterRange, OmpPreferenceSelector, OmpPreferenceSpecification, OrderKind, OrderModifier,
    OriginalSharing, ParserConfig, ProcBind, ReductionModifier, RequireModifier, ScanClauseMode,
    ScheduleKind, ScheduleModifier, SeverityKind, ThreadsetKind, UsesAllocatorBuiltin,
    UsesAllocatorKind, UsesAllocatorSourceSyntax, UsesAllocatorSpec, Variable, lang,
};
use crate::lexer::{Language as LexerLanguage, LogicalSource};
use crate::parser::clause::ReductionOperator as ParserReductionOperator;
use crate::parser::clause::{lookup_clause_name, parse_variable_list};
use crate::parser::directive_kind::{DirectiveName, lookup_directive_name};
use crate::parser::{Clause, ClauseKind, ClauseName};

/// Return a canonical payload keyword only for a case-insensitive host
/// language. C and C++ payload spelling is retained verbatim so a case variant
/// of a standardized keyword cannot be mistaken for that keyword.
fn payload_keyword<'a>(value: &'a str, config: &ParserConfig) -> Cow<'a, str> {
    if matches!(config.host_language(), HostLanguage::Fortran) {
        Cow::Owned(value.to_ascii_lowercase())
    } else {
        Cow::Borrowed(value)
    }
}

fn payload_keyword_eq(value: &str, expected: &str, config: &ParserConfig) -> bool {
    if matches!(config.host_language(), HostLanguage::Fortran) {
        value.eq_ignore_ascii_case(expected)
    } else {
        value == expected
    }
}

fn skip_host_trivia<'a>(input: &'a str, config: &ParserConfig) -> Result<&'a str, ConversionError> {
    let result = if matches!(config.host_language(), HostLanguage::Fortran) {
        crate::lexer::skip_fortran_space_and_comments(input)
    } else {
        crate::lexer::skip_space_and_comments(input)
    };
    result.map(|(rest, _)| rest).map_err(|error| {
        ConversionError::InvalidClauseSyntax(format!("invalid host-language trivia: {error:?}"))
    })
}

fn strip_payload_keyword<'a>(
    value: &'a str,
    expected: &str,
    config: &ParserConfig,
) -> Option<&'a str> {
    let prefix = value.get(..expected.len())?;
    payload_keyword_eq(prefix, expected, config).then(|| &value[expected.len()..])
}

pub(crate) fn parse_identifier_list(
    content: &str,
    config: &ParserConfig,
) -> Result<Vec<ClauseItem>, ConversionError> {
    lang::parse_clause_item_list(content, config)?
        .into_iter()
        .map(|item| match item {
            ClauseItem::Identifier(_)
            | ClauseItem::Variable(_)
            | ClauseItem::FortranCommonBlock(_) => Ok(item),
            ClauseItem::Expression(expression) => Err(ConversionError::InvalidClauseSyntax(
                format!("variable or locator list contains a general expression: `{expression}`"),
            )),
        })
        .collect()
}

pub(crate) fn parse_acc_identifier_list(
    content: &str,
    config: &ParserConfig,
) -> Result<Vec<ClauseItem>, ConversionError> {
    lang::parse_clause_item_list(content, config)?
        .into_iter()
        .map(|item| match item {
            ClauseItem::Identifier(_)
            | ClauseItem::Variable(_)
            | ClauseItem::FortranCommonBlock(_) => Ok(item),
            ClauseItem::Expression(expression) => Err(ConversionError::InvalidClauseSyntax(
                format!("variable or locator list contains a general expression: `{expression}`"),
            )),
        })
        .collect()
}

/// Parse exactly one clause assignment-expression.
///
/// Top-level commas delimit OpenMP/OpenACC arguments. A comma operator in a
/// single-expression slot therefore needs its own parentheses; calls and
/// explicitly parenthesized comma expressions remain single entries.
pub(crate) fn parse_single_clause_expression(
    source: &str,
    config: &ParserConfig,
    subject: &str,
) -> Result<Expression, ConversionError> {
    let entries = split_top_level_items(source)?;
    let [entry] = entries.as_slice() else {
        return Err(ConversionError::InvalidClauseSyntax(format!(
            "{subject} requires exactly one assignment-expression"
        )));
    };
    Expression::new(entry.trim(), config).map_err(ConversionError::from)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ObviousIntegerValue {
    Unknown,
    NonIntegerLiteral,
    Negative,
    NonNegative(u128),
}

fn obvious_integer_value(expression: &Expression) -> ObviousIntegerValue {
    fn classify(expression: &crate::host::Expr) -> ObviousIntegerValue {
        match &expression.kind {
            ExprKind::Parenthesized(inner) => classify(inner),
            ExprKind::Literal(Literal::Integer(value)) => {
                ObviousIntegerValue::NonNegative(value.value)
            }
            ExprKind::Unary {
                op: UnaryOp::Plus,
                operand,
            } => classify(operand),
            ExprKind::Unary {
                op: UnaryOp::Minus,
                operand,
            } => match classify(operand) {
                ObviousIntegerValue::NonNegative(0) => ObviousIntegerValue::NonNegative(0),
                ObviousIntegerValue::NonNegative(_) => ObviousIntegerValue::Negative,
                ObviousIntegerValue::NonIntegerLiteral => ObviousIntegerValue::NonIntegerLiteral,
                ObviousIntegerValue::Negative | ObviousIntegerValue::Unknown => {
                    ObviousIntegerValue::Unknown
                }
            },
            ExprKind::Literal(_) => ObviousIntegerValue::NonIntegerLiteral,
            _ => ObviousIntegerValue::Unknown,
        }
    }
    classify(expression.ast())
}

fn is_obviously_non_string(expression: &crate::host::Expr) -> bool {
    match &expression.kind {
        ExprKind::Parenthesized(inner) => is_obviously_non_string(inner),
        ExprKind::Literal(Literal::String(_)) => false,
        ExprKind::Literal(_) => true,
        ExprKind::Unary {
            op: UnaryOp::Plus | UnaryOp::Minus | UnaryOp::LogicalNot | UnaryOp::BitwiseNot,
            operand,
        } => is_obviously_non_string(operand),
        _ => false,
    }
}

fn convert_parser_reduction_operator(
    operator: ParserReductionOperator,
    user_identifier: Option<&str>,
    config: &ParserConfig,
) -> Result<OmpReductionIdentifier, ConversionError> {
    Ok(match operator {
        ParserReductionOperator::Add => OmpReductionIdentifier::Add,
        ParserReductionOperator::Sub => OmpReductionIdentifier::Subtract,
        ParserReductionOperator::Mul => OmpReductionIdentifier::Multiply,
        ParserReductionOperator::Max => {
            named_or_fortran_intrinsic_reduction("max", OmpFortranReductionIntrinsic::Max, config)?
        }
        ParserReductionOperator::Min => {
            named_or_fortran_intrinsic_reduction("min", OmpFortranReductionIntrinsic::Min, config)?
        }
        ParserReductionOperator::BitAnd if config.host_language() != HostLanguage::Fortran => {
            OmpReductionIdentifier::BitwiseAnd
        }
        ParserReductionOperator::BitOr if config.host_language() != HostLanguage::Fortran => {
            OmpReductionIdentifier::BitwiseOr
        }
        ParserReductionOperator::BitXor if config.host_language() != HostLanguage::Fortran => {
            OmpReductionIdentifier::BitwiseXor
        }
        ParserReductionOperator::LogAnd if config.host_language() != HostLanguage::Fortran => {
            OmpReductionIdentifier::LogicalAnd
        }
        ParserReductionOperator::LogOr if config.host_language() != HostLanguage::Fortran => {
            OmpReductionIdentifier::LogicalOr
        }
        ParserReductionOperator::FortAnd if config.host_language() == HostLanguage::Fortran => {
            OmpReductionIdentifier::FortranLogicalAnd
        }
        ParserReductionOperator::FortOr if config.host_language() == HostLanguage::Fortran => {
            OmpReductionIdentifier::FortranLogicalOr
        }
        ParserReductionOperator::FortIand => named_or_fortran_intrinsic_reduction(
            "iand",
            OmpFortranReductionIntrinsic::Iand,
            config,
        )?,
        ParserReductionOperator::FortIor => {
            named_or_fortran_intrinsic_reduction("ior", OmpFortranReductionIntrinsic::Ior, config)?
        }
        ParserReductionOperator::FortIeor => named_or_fortran_intrinsic_reduction(
            "ieor",
            OmpFortranReductionIntrinsic::Ieor,
            config,
        )?,
        ParserReductionOperator::FortEqv if config.host_language() == HostLanguage::Fortran => {
            OmpReductionIdentifier::FortranLogicalEqv
        }
        ParserReductionOperator::FortNeqv if config.host_language() == HostLanguage::Fortran => {
            OmpReductionIdentifier::FortranLogicalNeqv
        }
        ParserReductionOperator::BitAnd
        | ParserReductionOperator::BitOr
        | ParserReductionOperator::BitXor
        | ParserReductionOperator::LogAnd
        | ParserReductionOperator::LogOr
        | ParserReductionOperator::FortAnd
        | ParserReductionOperator::FortOr
        | ParserReductionOperator::FortEqv
        | ParserReductionOperator::FortNeqv => {
            return Err(ConversionError::InvalidClauseSyntax(
                "reduction identifier is not standardized for the configured host language"
                    .to_string(),
            ));
        }
        ParserReductionOperator::UserDefined => {
            let identifier = user_identifier
                .map(str::trim)
                .filter(|identifier| !identifier.is_empty())
                .ok_or_else(|| {
                    ConversionError::InvalidClauseSyntax(
                        "user-defined reduction operator is missing its identifier".to_string(),
                    )
                })?;
            super::ast_builder::parse_reduction_identifier(identifier, config)
                .map_err(|error| ConversionError::InvalidClauseSyntax(error.to_string()))?
        }
    })
}

fn named_or_fortran_intrinsic_reduction(
    name: &str,
    intrinsic: OmpFortranReductionIntrinsic,
    config: &ParserConfig,
) -> Result<OmpReductionIdentifier, ConversionError> {
    if config.host_language() == HostLanguage::Fortran {
        return Ok(OmpReductionIdentifier::FortranIntrinsic(intrinsic));
    }
    Ok(OmpReductionIdentifier::Name(OmpIdExpression::Name(
        crate::host::QualifiedName {
            global: false,
            segments: vec![Identifier::new(name)?],
        },
    )))
}

fn convert_reduction_modifier(
    modifier: crate::parser::clause::ReductionModifier,
    arguments: &[std::borrow::Cow<'_, str>],
    config: &ParserConfig,
) -> Result<ReductionModifier, ConversionError> {
    match modifier {
        crate::parser::clause::ReductionModifier::Task if arguments.is_empty() => {
            Ok(ReductionModifier::Task)
        }
        crate::parser::clause::ReductionModifier::Inscan if arguments.is_empty() => {
            Ok(ReductionModifier::Inscan)
        }
        crate::parser::clause::ReductionModifier::Default if arguments.is_empty() => {
            Ok(ReductionModifier::Default)
        }
        crate::parser::clause::ReductionModifier::Original => {
            parse_original_reduction_modifier(arguments, config)
        }
        _ => Err(ConversionError::InvalidClauseSyntax(
            "reduction modifier has unexpected arguments".to_string(),
        )),
    }
}

fn parse_original_reduction_modifier(
    arguments: &[impl AsRef<str>],
    config: &ParserConfig,
) -> Result<ReductionModifier, ConversionError> {
    let [argument] = arguments else {
        return Err(ConversionError::InvalidClauseSyntax(
            "original reduction modifier requires exactly one sharing= value".to_string(),
        ));
    };
    let (name, value) = argument.as_ref().split_once('=').ok_or_else(|| {
        ConversionError::InvalidClauseSyntax(
            "original reduction modifier requires sharing=default|private|shared".to_string(),
        )
    })?;
    if !payload_keyword_eq(name.trim(), "sharing", config) {
        return Err(ConversionError::InvalidClauseSyntax(
            "original reduction modifier requires a sharing= argument".to_string(),
        ));
    }
    let sharing_keyword = payload_keyword(value.trim(), config);
    let sharing = match sharing_keyword.as_ref() {
        "default" => OriginalSharing::Default,
        "private" => OriginalSharing::Private,
        "shared" => OriginalSharing::Shared,
        _ => {
            return Err(ConversionError::InvalidClauseSyntax(format!(
                "unknown original sharing value: {}",
                value.trim()
            )));
        }
    };
    Ok(ReductionModifier::Original(sharing))
}

/// Parse a schedule clause
///
/// Format: `schedule([modifier[, modifier]:] kind[, chunk_size])`
///
/// ## Example
///
/// ```ignore
/// # use roup::ir::ParserConfig;
/// let config = ParserConfig::c();
/// let clause = parse_schedule_clause("static, 10", &config).unwrap();
/// // Returns ClauseData::Schedule with kind=Static, chunk_size=Some(10)
/// ```
pub fn parse_schedule_clause(
    content: &str,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    // A colon starts the modifier prefix only when every token before it is a
    // schedule modifier. This prevents a colon in a ternary chunk expression
    // from being mistaken for the modifier separator.
    let modifier_prefix = if let Some((prefix, rest)) = lang::split_once_top_level(content, ':')? {
        let tokens = lang::split_top_level(prefix, ',', &[('(', ')'), ('[', ']'), ('{', '}')])?;
        if tokens.iter().all(|token| {
            matches!(
                payload_keyword(token.trim(), config).as_ref(),
                "monotonic" | "nonmonotonic" | "simd"
            )
        }) {
            Some((prefix, rest))
        } else {
            None
        }
    } else {
        None
    };

    let (modifiers, rest) = if let Some((mod_str, kind_str)) = modifier_prefix {
        let mut seen_modifiers: HashSet<ScheduleModifier> = HashSet::new();
        let mut mods: Vec<ScheduleModifier> = Vec::new();

        for raw in lang::split_top_level(mod_str, ',', &[('(', ')'), ('[', ']'), ('{', '}')])? {
            let raw = raw.trim();
            let keyword = payload_keyword(raw, config);
            let modifier = match keyword.as_ref() {
                "monotonic" => ScheduleModifier::Monotonic,
                "nonmonotonic" => ScheduleModifier::Nonmonotonic,
                "simd" => ScheduleModifier::Simd,
                _ => {
                    return Err(ConversionError::InvalidClauseSyntax(format!(
                        "Unknown schedule modifier: {raw}"
                    )));
                }
            };

            if (modifier == ScheduleModifier::Monotonic
                && seen_modifiers.contains(&ScheduleModifier::Nonmonotonic))
                || (modifier == ScheduleModifier::Nonmonotonic
                    && seen_modifiers.contains(&ScheduleModifier::Monotonic))
            {
                return Err(ConversionError::InvalidClauseSyntax(
                    "schedule clause cannot combine monotonic and nonmonotonic modifiers"
                        .to_string(),
                ));
            }

            if !seen_modifiers.insert(modifier) {
                return Err(ConversionError::InvalidClauseSyntax(format!(
                    "Duplicate schedule modifier: {raw}"
                )));
            }

            mods.push(modifier);
        }

        (mods, kind_str.trim())
    } else {
        (vec![], content)
    };

    // Parse kind and optional chunk size (comma-separated)
    let parts = lang::split_top_level(rest, ',', &[('(', ')'), ('[', ']'), ('{', '}')])?;

    if parts.len() > 2 {
        return Err(ConversionError::InvalidClauseSyntax(
            "schedule clause accepts only a kind and optional chunk expression".to_string(),
        ));
    }

    let kind_token = parts.first().map(|value| value.trim()).ok_or_else(|| {
        ConversionError::InvalidClauseSyntax("schedule clause requires a kind".to_string())
    })?;
    let kind_keyword = payload_keyword(kind_token, config);
    let kind = match kind_keyword.as_ref() {
        "static" => ScheduleKind::Static,
        "dynamic" => ScheduleKind::Dynamic,
        "guided" => ScheduleKind::Guided,
        "auto" => ScheduleKind::Auto,
        "runtime" => ScheduleKind::Runtime,
        _ => {
            return Err(ConversionError::InvalidClauseSyntax(format!(
                "unknown schedule kind: {kind_token}"
            )));
        }
    };

    let chunk_size = match parts.get(1) {
        Some(value) if value.trim().is_empty() => {
            return Err(ConversionError::InvalidClauseSyntax(
                "schedule chunk expression must not be empty".to_string(),
            ));
        }
        Some(value) => Some(Expression::new(value.trim(), config)?),
        None => None,
    };

    if chunk_size.is_some() && matches!(kind, ScheduleKind::Auto | ScheduleKind::Runtime) {
        return Err(ConversionError::InvalidClauseSyntax(
            "schedule(auto) and schedule(runtime) do not accept a chunk expression".to_string(),
        ));
    }

    Ok(ClauseData::Schedule {
        kind,
        modifiers,
        chunk_size,
    })
}

/// Parse a map clause
///
/// Format: `map([[mapper(mapper-identifier),] map-type:] list)`
///
/// Supports mapper syntax and respects nesting when finding colons.
///
/// ## Example
///
/// ```ignore
/// # use roup::ir::ParserConfig;
/// let config = ParserConfig::c();
/// let clause = parse_map_clause("to: arr", &config).unwrap();
/// // Returns ClauseData::Map with map_type=To, items=[arr]
///
/// let clause = parse_map_clause("mapper(custom), to: arr[0:N]", &config).unwrap();
/// // Returns ClauseData::Map with mapper=Some(custom), map_type=To, items=[arr[0:N]]
/// ```
pub fn parse_map_clause(
    content: &str,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    let mut mapper = None;
    let mut modifiers = Vec::new();
    let mut iterators = Vec::new();
    let mut map_type_spelling = MapTypeSpelling::Canonical;

    // Find map-type using top-level colon detection
    let (map_type, items_str) =
        if let Some((type_str, items)) = lang::split_once_top_level(content.trim(), ':')? {
            let mut map_type = None;
            let terms = parse_map_header_terms(type_str, config)?;
            for term in terms {
                let token = match term {
                    MapHeaderTerm::Keyword(token) => token,
                    MapHeaderTerm::Mapper(mapper_body) => {
                        if mapper.is_some() {
                            return Err(ConversionError::InvalidClauseSyntax(
                                "duplicate mapper map modifier".to_string(),
                            ));
                        }
                        let mapper_body = mapper_body.trim();
                        mapper = Some(if payload_keyword_eq(mapper_body, "default", config) {
                            crate::ast::OmpMapperId::Default
                        } else {
                            crate::ast::OmpMapperId::User(Identifier::new(mapper_body)?)
                        });
                        continue;
                    }
                    MapHeaderTerm::Iterator(iterator_body) => {
                        push_unique_map_modifier(&mut modifiers, MapModifier::Iterator)?;
                        iterators = parse_iterator_block(iterator_body, config)?;
                        continue;
                    }
                };
                let keyword = payload_keyword(token, config);
                let parsed_type = match keyword.as_ref() {
                    "to" => Some(MapType::To),
                    "from" => Some(MapType::From),
                    "tofrom" => Some(MapType::ToFrom),
                    "alloc" => {
                        map_type_spelling = MapTypeSpelling::Alloc;
                        Some(MapType::Storage)
                    }
                    "release" => {
                        map_type_spelling = MapTypeSpelling::Release;
                        Some(MapType::Storage)
                    }
                    "storage" => {
                        map_type_spelling = MapTypeSpelling::Canonical;
                        Some(MapType::Storage)
                    }
                    "always" => {
                        push_unique_map_modifier(&mut modifiers, MapModifier::Always)?;
                        None
                    }
                    "close" => {
                        push_unique_map_modifier(&mut modifiers, MapModifier::Close)?;
                        None
                    }
                    "present" => {
                        push_unique_map_modifier(&mut modifiers, MapModifier::Present)?;
                        None
                    }
                    "self" => {
                        push_unique_map_modifier(&mut modifiers, MapModifier::SelfMap)?;
                        None
                    }
                    "ref_ptee" => {
                        push_unique_map_modifier(
                            &mut modifiers,
                            MapModifier::Ref(MapRefKind::Pointee),
                        )?;
                        None
                    }
                    "ref_ptr" => {
                        push_unique_map_modifier(
                            &mut modifiers,
                            MapModifier::Ref(MapRefKind::Pointer),
                        )?;
                        None
                    }
                    "ref_ptr_ptee" => {
                        push_unique_map_modifier(
                            &mut modifiers,
                            MapModifier::Ref(MapRefKind::PointerAndPointee),
                        )?;
                        None
                    }
                    "delete" => {
                        push_unique_map_modifier(&mut modifiers, MapModifier::Delete)?;
                        None
                    }
                    _ => {
                        return Err(ConversionError::InvalidClauseSyntax(format!(
                            "Unknown map modifier or type: {token}"
                        )));
                    }
                };
                if let Some(parsed_type) = parsed_type
                    && map_type.replace(parsed_type).is_some()
                {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "map clause specifies more than one map type".to_string(),
                    ));
                }
            }
            if modifiers.contains(&MapModifier::Delete) && map_type.is_none() {
                map_type = Some(MapType::Storage);
                map_type_spelling = MapTypeSpelling::Delete;
            }
            (map_type, items.trim())
        } else {
            (None, content.trim())
        };

    let locators = parse_omp_locator_list(items_str, config)?;

    Ok(ClauseData::Map {
        map_type,
        map_type_spelling,
        modifiers,
        mapper,
        iterators,
        locators,
    })
}

#[derive(Clone, Copy)]
enum MapHeaderTerm<'a> {
    Keyword(&'a str),
    Mapper(&'a str),
    Iterator(&'a str),
}

/// Parse both the current comma-separated map modifier list and the
/// standardized historical form in which commas after modifiers were
/// optional. Terms still require an unambiguous lexical boundary; arbitrary
/// text is never repaired or split heuristically.
fn parse_map_header_terms<'a>(
    source: &'a str,
    config: &ParserConfig,
) -> Result<Vec<MapHeaderTerm<'a>>, ConversionError> {
    let mut terms = Vec::new();
    let mut remaining = source.trim();

    if remaining.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "map modifier list must not be empty".to_string(),
        ));
    }

    loop {
        if remaining.starts_with(',') {
            return Err(ConversionError::InvalidClauseSyntax(
                "map modifier list contains an empty entry".to_string(),
            ));
        }

        let keyword_end = remaining
            .char_indices()
            .take_while(|(_, ch)| ch.is_ascii_alphanumeric() || *ch == '_')
            .last()
            .map_or(0, |(index, ch)| index + ch.len_utf8());
        if keyword_end == 0 {
            return Err(ConversionError::InvalidClauseSyntax(
                "map modifier list requires a keyword".to_string(),
            ));
        }

        let keyword = &remaining[..keyword_end];
        let after_keyword = &remaining[keyword_end..];
        let (term, rest, complex_boundary) = if payload_keyword_eq(keyword, "mapper", config)
            || payload_keyword_eq(keyword, "iterator", config)
        {
            let parenthesized = after_keyword.trim_start();
            if !parenthesized.starts_with('(') {
                return Err(ConversionError::InvalidClauseSyntax(format!(
                    "map {keyword} modifier requires parentheses"
                )));
            }
            let (body, rest) = extract_parenthesized(parenthesized)?;
            if body.trim().is_empty() {
                return Err(ConversionError::InvalidClauseSyntax(format!(
                    "map {keyword} modifier requires non-empty content"
                )));
            }
            let term = if payload_keyword_eq(keyword, "mapper", config) {
                MapHeaderTerm::Mapper(body)
            } else {
                MapHeaderTerm::Iterator(body)
            };
            (term, rest, true)
        } else {
            (MapHeaderTerm::Keyword(keyword), after_keyword, false)
        };
        terms.push(term);

        if rest.is_empty() {
            break;
        }
        if let Some(after_comma) = rest.strip_prefix(',') {
            remaining = after_comma.trim_start();
            if remaining.is_empty() || remaining.starts_with(',') {
                return Err(ConversionError::InvalidClauseSyntax(
                    "map modifier list contains an empty entry".to_string(),
                ));
            }
            continue;
        }

        let trimmed = rest.trim_start();
        if trimmed.len() != rest.len() {
            remaining = trimmed;
            continue;
        }
        if complex_boundary {
            // A closing ')' provides a lexical token boundary even when the
            // historical optional comma and optional whitespace are absent.
            remaining = rest;
            continue;
        }

        return Err(ConversionError::InvalidClauseSyntax(format!(
            "map modifier keyword {keyword:?} is not separated from the following token"
        )));
    }

    Ok(terms)
}

fn parse_exact_complex_modifier<'a>(
    source: &'a str,
    keyword: &str,
    config: &ParserConfig,
) -> Result<Option<&'a str>, ConversionError> {
    let Some(after_keyword) = strip_payload_keyword(source.trim(), keyword, config) else {
        return Ok(None);
    };
    if after_keyword
        .chars()
        .next()
        .is_some_and(|character| character == '_' || character.is_ascii_alphanumeric())
    {
        return Ok(None);
    }
    let after_keyword = after_keyword.trim_start();
    if !after_keyword.starts_with('(') {
        return Err(ConversionError::InvalidClauseSyntax(format!(
            "{keyword} modifier requires parentheses"
        )));
    }
    let (body, remainder) = extract_parenthesized(after_keyword)?;
    if !remainder.trim().is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(format!(
            "unexpected tokens after {keyword} modifier"
        )));
    }
    if body.trim().is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(format!(
            "{keyword} modifier requires non-empty content"
        )));
    }
    Ok(Some(body.trim()))
}

fn parse_omp_locator_list(
    source: &str,
    config: &ParserConfig,
) -> Result<Vec<OmpLocator>, ConversionError> {
    parse_omp_locator_list_with_reserved(source, config, false)
}

fn parse_omp_locator_list_with_reserved(
    source: &str,
    config: &ParserConfig,
    allow_all_memory: bool,
) -> Result<Vec<OmpLocator>, ConversionError> {
    let items = lang::parse_clause_item_list(source, config)?;
    if items.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "data-motion clause requires at least one locator".to_string(),
        ));
    }
    fn contains_fortran_apply(expression: &crate::host::Expr) -> bool {
        match &expression.kind {
            ExprKind::FortranApply { .. } => true,
            ExprKind::Parenthesized(inner)
            | ExprKind::Unary { operand: inner, .. }
            | ExprKind::Postfix { operand: inner, .. } => contains_fortran_apply(inner),
            ExprKind::Binary { left, right, .. } => {
                contains_fortran_apply(left) || contains_fortran_apply(right)
            }
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => {
                contains_fortran_apply(condition)
                    || contains_fortran_apply(then_expr)
                    || contains_fortran_apply(else_expr)
            }
            ExprKind::Assignment { target, value, .. } => {
                contains_fortran_apply(target) || contains_fortran_apply(value)
            }
            ExprKind::Call { callee, arguments } => {
                contains_fortran_apply(callee) || arguments.iter().any(contains_fortran_apply)
            }
            ExprKind::Subscript { base, subscript } => {
                contains_fortran_apply(base)
                    || match subscript {
                        crate::host::Subscript::Index(index) => contains_fortran_apply(index),
                        crate::host::Subscript::Section(section) => section
                            .lower
                            .iter()
                            .chain(section.upper_or_length.iter())
                            .chain(section.stride.iter())
                            .any(|value| contains_fortran_apply(value)),
                    }
            }
            ExprKind::Member { base, .. } => contains_fortran_apply(base),
            ExprKind::Literal(_) | ExprKind::Name(_) => false,
        }
    }

    fn is_potential_lvalue(expression: &Expression) -> bool {
        match expression.language() {
            HostLanguage::Cpp => {
                fn ambiguous(expression: &crate::host::Expr) -> bool {
                    match &expression.kind {
                        ExprKind::Parenthesized(inner) => ambiguous(inner),
                        ExprKind::Call { .. }
                        | ExprKind::Conditional { .. }
                        | ExprKind::Assignment { .. }
                        | ExprKind::Binary {
                            op: BinaryOp::Comma,
                            ..
                        } => true,
                        _ => false,
                    }
                }
                ambiguous(expression.ast())
            }
            HostLanguage::Fortran => contains_fortran_apply(expression.ast()),
            HostLanguage::C => false,
        }
    }

    fn classify(expression: Expression) -> Result<OmpLocator, ConversionError> {
        if is_potential_lvalue(&expression) {
            return Ok(OmpLocator::PotentialLValue(expression));
        }
        Ok(OmpLocator::LValue(LValue::from_expression(expression)?))
    }

    items
        .into_iter()
        .map(|item| match item {
            ClauseItem::FortranCommonBlock(name) => Ok(OmpLocator::FortranCommonBlock(name)),
            ClauseItem::Identifier(identifier) => {
                if payload_keyword_eq(identifier.as_str(), "omp_all_memory", config) {
                    return if allow_all_memory {
                        Ok(OmpLocator::AllMemory)
                    } else {
                        Err(ConversionError::InvalidClauseSyntax(
                            "omp_all_memory is not permitted in this locator list".to_string(),
                        ))
                    };
                }
                classify(Expression::new(identifier.as_str(), config)?)
            }
            ClauseItem::Variable(variable) => classify(variable.expression().clone()),
            ClauseItem::Expression(expression) => classify(expression),
        })
        .collect()
}

fn parse_depend_locator_list(
    source: &str,
    config: &ParserConfig,
) -> Result<Vec<OmpLocator>, ConversionError> {
    let locators = parse_omp_locator_list_with_reserved(source, config, true)?;
    if locators
        .iter()
        .any(|locator| matches!(locator, OmpLocator::FortranCommonBlock(_)))
    {
        return Err(ConversionError::InvalidClauseSyntax(
            "Fortran common block names are not permitted in depend clauses".to_string(),
        ));
    }
    Ok(locators)
}

fn parse_data_motion_clause(
    clause_name: &ClauseName,
    content: &str,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    let (header, locator_source) = match lang::split_once_top_level(content.trim(), ':')? {
        Some((header, locators)) => (Some(header.trim()), locators.trim()),
        None => (None, content.trim()),
    };
    let mut present = false;
    let mut mapper = None;
    let mut iterators = Vec::new();
    if let Some(header) = header {
        for modifier in split_top_level_items(header)? {
            let modifier = modifier.trim();
            if payload_keyword_eq(modifier, "present", config) {
                if present {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "duplicate present data-motion modifier".to_string(),
                    ));
                }
                present = true;
            } else if let Some(identifier) =
                parse_exact_complex_modifier(modifier, "mapper", config)?
            {
                if mapper.is_some() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "duplicate mapper data-motion modifier".to_string(),
                    ));
                }
                mapper = Some(if payload_keyword_eq(identifier, "default", config) {
                    crate::ast::OmpMapperId::Default
                } else {
                    crate::ast::OmpMapperId::User(Identifier::new(identifier)?)
                });
            } else if let Some(definitions) =
                parse_exact_complex_modifier(modifier, "iterator", config)?
            {
                if !iterators.is_empty() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "duplicate iterator data-motion modifier".to_string(),
                    ));
                }
                iterators = parse_iterator_block(definitions, config)?;
            } else {
                return Err(ConversionError::InvalidClauseSyntax(format!(
                    "unknown data-motion modifier: {modifier}"
                )));
            }
        }
    }
    let locators = parse_omp_locator_list(locator_source, config)?;
    match clause_name {
        ClauseName::To => Ok(ClauseData::To {
            present,
            mapper,
            iterators,
            locators,
        }),
        ClauseName::From => Ok(ClauseData::From {
            present,
            mapper,
            iterators,
            locators,
        }),
        _ => Err(ConversionError::InvalidClauseSyntax(
            "internal data-motion clause dispatch mismatch".to_string(),
        )),
    }
}

fn parse_declare_target_enter_clause(
    content: &str,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    let (automap, item_source) = match lang::split_once_top_level(content.trim(), ':')? {
        Some((modifier, items)) if payload_keyword_eq(modifier.trim(), "automap", config) => {
            (true, items.trim())
        }
        Some((modifier, _)) => {
            return Err(ConversionError::InvalidClauseSyntax(format!(
                "unknown declare-target enter modifier: {}",
                modifier.trim()
            )));
        }
        None => (false, content.trim()),
    };
    if automap && !matches!(config.host_language(), HostLanguage::Fortran) {
        return Err(ConversionError::InvalidClauseSyntax(
            "declare-target enter automap is only valid in Fortran".to_string(),
        ));
    }
    let items = parse_identifier_list(item_source, config)?;
    if items.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "enter clause requires a non-empty extended list".to_string(),
        ));
    }
    Ok(ClauseData::Enter { automap, items })
}

fn push_unique_map_modifier(
    modifiers: &mut Vec<MapModifier>,
    modifier: MapModifier,
) -> Result<(), ConversionError> {
    let same_family = |existing: &MapModifier| {
        existing == &modifier
            || matches!(
                (existing, &modifier),
                (MapModifier::Ref(_), MapModifier::Ref(_))
            )
    };
    if modifiers.iter().any(same_family) {
        return Err(ConversionError::InvalidClauseSyntax(format!(
            "duplicate map modifier: {modifier}"
        )));
    }
    modifiers.push(modifier);
    Ok(())
}

/// Extract content from parentheses, handling nesting.
///
/// Returns (content, remainder) where content is what's inside the first set of parens.
///
/// This is a thin wrapper around the lang module's bracket extraction helper.
fn extract_parenthesized(input: &str) -> Result<(&str, &str), ConversionError> {
    // Delegate to the lang module's generic bracket extraction
    lang::extract_bracket_content(input, '(', ')')
}

/// Parse a dependence type from a string
///
/// ## Example
///
/// ```ignore
/// # use roup::ir::DependType;
/// # use roup::parser::semantic::parse_depend_type;
/// let dt = parse_depend_type("in").unwrap();
/// assert_eq!(dt, DependType::In);
/// ```
pub fn parse_depend_type(
    type_str: &str,
    config: &ParserConfig,
) -> Result<DependType, ConversionError> {
    let keyword = payload_keyword(type_str.trim(), config);
    match keyword.as_ref() {
        "in" => Ok(DependType::In),
        "out" => Ok(DependType::Out),
        "inout" => Ok(DependType::Inout),
        "inoutset" => Ok(DependType::Inoutset),
        "mutexinoutset" => Ok(DependType::Mutexinoutset),
        _ => Err(ConversionError::InvalidClauseSyntax(format!(
            "Unknown depend type: {type_str}"
        ))),
    }
}

fn lookup_payload_directive_name(text: &str, config: &ParserConfig) -> DirectiveName {
    let separated = if matches!(config.host_language(), HostLanguage::Fortran) {
        Cow::Owned(text.replace('+', " "))
    } else {
        Cow::Borrowed(text)
    };
    let normalized = separated.split_whitespace().collect::<Vec<_>>().join(" ");
    let parsed = lookup_directive_name(&normalized);
    if !matches!(config.host_language(), HostLanguage::Fortran) && parsed.as_str() != normalized {
        DirectiveName::Other(Cow::Owned(normalized))
    } else {
        parsed
    }
}

pub(crate) fn parse_directive_name_modifier(
    text: &str,
    config: &ParserConfig,
) -> Result<OmpDirectiveKind, ConversionError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "directive-name modifier must not be empty".to_string(),
        ));
    }
    OmpDirectiveKind::try_from(lookup_payload_directive_name(trimmed, config)).map_err(|_| {
        ConversionError::InvalidClauseSyntax(format!(
            "unknown OpenMP directive-name modifier: {trimmed}"
        ))
    })
}

/// Extract a leading iterator(...) block, returning the inner text and the
/// remaining clause content after the closing parenthesis.
pub(crate) fn extract_iterator_block<'a>(
    content: &'a str,
    config: &ParserConfig,
) -> Result<Option<(&'a str, &'a str)>, ConversionError> {
    let trimmed = content.trim_start();
    const KEYWORD: &str = "iterator";
    if !trimmed
        .get(..KEYWORD.len())
        .is_some_and(|prefix| payload_keyword_eq(prefix, KEYWORD, config))
    {
        return Ok(None);
    }
    if trimmed[KEYWORD.len()..]
        .chars()
        .next()
        .is_some_and(|character| character == '_' || character.is_ascii_alphanumeric())
    {
        return Ok(None);
    }

    let after_keyword = &trimmed[KEYWORD.len()..];
    let after_trivia = crate::lexer::skip_space_and_comments(after_keyword)
        .map(|(remaining, _)| remaining)
        .map_err(|error| {
            ConversionError::InvalidClauseSyntax(format!(
                "malformed trivia after iterator modifier: {error:?}"
            ))
        })?;
    if !after_trivia.starts_with('(') {
        return Ok(None);
    }
    let (inner, remainder) = lang::extract_bracket_content(after_trivia, '(', ')')?;
    Ok(Some((inner, remainder)))
}

fn parse_iterator_definition(
    def: &str,
    config: &ParserConfig,
) -> Result<DependIterator, ConversionError> {
    let (lhs, rhs) = lang::split_once_top_level(def, '=')?.ok_or_else(|| {
        ConversionError::InvalidClauseSyntax("iterator definition missing '='".into())
    })?;

    let lhs = lhs.trim();
    if lhs.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "iterator definition missing variable name".into(),
        ));
    }

    let Some(name) = lhs.split_whitespace().last() else {
        return Err(ConversionError::InvalidClauseSyntax(
            "iterator definition missing variable name".into(),
        ));
    };
    let name_start = lhs.rfind(name).ok_or_else(|| {
        ConversionError::InvalidClauseSyntax("iterator variable name is not in its source".into())
    })?;
    let type_source = lhs[..name_start].trim();
    let type_name = if type_source.is_empty() {
        None
    } else {
        Some(crate::host::TypeName::parse_with_profile(
            type_source,
            config.profile(),
        )?)
    };

    let range = rhs.trim();
    let Some((start_str, end_and_step)) = lang::split_once_top_level(range, ':')? else {
        return Err(ConversionError::InvalidClauseSyntax(
            "iterator missing end expression".into(),
        ));
    };
    let (end_str, step_str) = match lang::split_once_top_level(end_and_step, ':')? {
        Some((end, step)) => (end, Some(step)),
        None => (end_and_step, None),
    };
    if step_str
        .map(|step| lang::split_once_top_level(step, ':'))
        .transpose()?
        .flatten()
        .is_some()
    {
        return Err(ConversionError::InvalidClauseSyntax(
            "iterator has too many ':' separators".into(),
        ));
    }

    if start_str.trim().is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "iterator missing start expression".into(),
        ));
    }
    if end_str.trim().is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "iterator missing end expression".into(),
        ));
    }
    if step_str.is_some_and(|step| step.trim().is_empty()) {
        return Err(ConversionError::InvalidClauseSyntax(
            "iterator missing step expression".into(),
        ));
    }

    let start = Expression::new(start_str, config)?;
    let end = Expression::new(end_str, config)?;
    let step = step_str
        .map(|source| Expression::new(source, config))
        .transpose()?;

    Ok(DependIterator::new(
        type_name,
        Identifier::new(name)?,
        start,
        end,
        step,
    ))
}

pub(crate) fn parse_iterator_block(
    block: &str,
    config: &ParserConfig,
) -> Result<Vec<DependIterator>, ConversionError> {
    split_top_level_items(block)?
        .into_iter()
        .map(|definition| parse_iterator_definition(definition.trim(), config))
        .collect()
}

pub(crate) fn parse_apply_clause(
    content: &str,
    config: &ParserConfig,
    source: &LogicalSource<'_>,
) -> Result<(Option<OmpApplyLoopModifier>, Vec<OmpDirective>), ConversionError> {
    let content = content.trim();
    if content.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "apply clause requires at least one applied directive".to_string(),
        ));
    }

    let (loop_modifier, directives_source) = if let Some((modifier_source, directives_source)) =
        lang::split_once_top_level(content, ':')?
    {
        let modifier_source = modifier_source.trim();
        let directives_source = directives_source.trim();
        if modifier_source.is_empty() || directives_source.is_empty() {
            return Err(ConversionError::InvalidClauseSyntax(
                "apply loop modifier and applied-directive list must not be empty".to_string(),
            ));
        }
        (
            Some(parse_apply_loop_modifier(modifier_source, config)?),
            directives_source,
        )
    } else {
        (None, content)
    };

    let applied_directives = split_top_level_items(directives_source)?
        .into_iter()
        .map(str::trim)
        .map(|directive_source| {
            parse_nested_directive(directive_source, config, source)?.ok_or_else(|| {
                ConversionError::InvalidClauseSyntax(
                    "apply clause requires an OpenMP directive specification".to_string(),
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    if applied_directives.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "apply clause requires at least one applied directive".to_string(),
        ));
    }
    Ok((loop_modifier, applied_directives))
}

fn parse_apply_loop_modifier(
    source: &str,
    config: &ParserConfig,
) -> Result<OmpApplyLoopModifier, ConversionError> {
    let source = source.trim();
    let (keyword_source, indices_source) = if let Some(open) = source.find('(') {
        let indices = extract_paren_arg(source)?.ok_or_else(|| {
            ConversionError::InvalidClauseSyntax(
                "apply loop modifier must contain one complete indices list".to_string(),
            )
        })?;
        if indices.trim().is_empty() {
            return Err(ConversionError::InvalidClauseSyntax(
                "apply loop modifier indices must not be empty".to_string(),
            ));
        }
        (source[..open].trim(), Some(indices))
    } else {
        (source, None)
    };

    let kind = match payload_keyword(keyword_source, config).as_ref() {
        "fused" => OmpApplyLoopKind::Fused,
        "grid" => OmpApplyLoopKind::Grid,
        "identity" => OmpApplyLoopKind::Identity,
        "interchanged" => OmpApplyLoopKind::Interchanged,
        "intratile" => OmpApplyLoopKind::Intratile,
        "offsets" => OmpApplyLoopKind::Offsets,
        "reversed" => OmpApplyLoopKind::Reversed,
        "split" => OmpApplyLoopKind::Split,
        "unrolled" => OmpApplyLoopKind::Unrolled,
        _ => {
            return Err(ConversionError::InvalidClauseSyntax(format!(
                "unknown apply loop modifier: {keyword_source}"
            )));
        }
    };
    let indices = indices_source
        .map(split_top_level_items)
        .transpose()?
        .unwrap_or_default()
        .into_iter()
        .map(str::trim)
        .map(|index| Expression::new(index, config).map_err(ConversionError::from))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(OmpApplyLoopModifier { kind, indices })
}

pub(crate) fn parse_at_clause(
    kind: &ClauseKind<'_>,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    if let ClauseKind::Parenthesized(content) = kind {
        let value = content.as_ref().trim();
        let keyword = payload_keyword(value, config);
        let at_kind = match keyword.as_ref() {
            "compilation" => AtKind::Compilation,
            "execution" => AtKind::Execution,
            "" => {
                return Err(ConversionError::InvalidClauseSyntax(
                    "at clause requires a value".to_string(),
                ));
            }
            _ => {
                return Err(ConversionError::InvalidClauseSyntax(format!(
                    "Unknown at clause value: {value}"
                )));
            }
        };
        Ok(ClauseData::At(at_kind))
    } else {
        Err(ConversionError::InvalidClauseSyntax(
            "at clause requires parenthesized value".to_string(),
        ))
    }
}

pub(crate) fn parse_init_clause(
    kind: &ClauseKind<'_>,
    directive_kind: OmpDirectiveKind,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    let ClauseKind::Parenthesized(content) = kind else {
        return Err(ConversionError::InvalidClauseSyntax(
            "init clause requires parenthesized content".to_string(),
        ));
    };
    let content = content.as_ref().trim();
    if content.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "init clause requires modifiers and an initialization variable".to_string(),
        ));
    }

    match directive_kind {
        OmpDirectiveKind::Interop => parse_interop_init_clause(content, config),
        OmpDirectiveKind::Depobj => parse_depobj_init_clause(content, config),
        _ => Err(ConversionError::InvalidClauseSyntax(format!(
            "init clause is not valid on {directive_kind:?}"
        ))),
    }
}

fn parse_interop_init_clause(
    content: &str,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    let (modifier_source, variable_source) =
        lang::split_once_top_level(content, ':')?.ok_or_else(|| {
            ConversionError::InvalidClauseSyntax(
                "interop init requires an interop-type modifier and ':'".to_string(),
            )
        })?;
    let modifier_source = modifier_source.trim();
    let variable_source = variable_source.trim();
    if modifier_source.is_empty() || variable_source.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "interop init modifiers and variable must not be empty".to_string(),
        ));
    }
    let modifiers = parse_interop_init_modifiers(modifier_source, config)?;
    Ok(ClauseData::InitInterop {
        interop_types: modifiers.interop_types,
        preferences: modifiers.preferences,
        variable: Variable::parse(variable_source, config)?,
    })
}

pub(crate) fn parse_interop_init_modifiers(
    source: &str,
    config: &ParserConfig,
) -> Result<OmpInteropInitModifiers, ConversionError> {
    let mut interop_types = Vec::new();
    let mut preferences = None;

    for entry in split_top_level_items(source)? {
        let entry = entry.trim();
        let keyword = payload_keyword(entry, config);
        let interop_type = match keyword.as_ref() {
            "target" => Some(OmpInteropType::Target),
            "targetsync" => Some(OmpInteropType::Targetsync),
            _ => None,
        };
        if let Some(interop_type) = interop_type {
            if interop_types.contains(&interop_type) {
                return Err(ConversionError::InvalidClauseSyntax(format!(
                    "duplicate {entry} interop type"
                )));
            }
            interop_types.push(interop_type);
            continue;
        }

        if let Some(argument) = extract_named_call(entry, "prefer_type", config)? {
            if preferences.is_some() {
                return Err(ConversionError::InvalidClauseSyntax(
                    "duplicate prefer_type init modifier".to_string(),
                ));
            }
            preferences = Some(parse_preference_specifications(argument, config)?);
            continue;
        }

        return Err(ConversionError::InvalidClauseSyntax(format!(
            "unknown interop init modifier: {entry}"
        )));
    }

    if interop_types.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "interop init requires target or targetsync".to_string(),
        ));
    }
    Ok(OmpInteropInitModifiers {
        interop_types,
        preferences: preferences.unwrap_or_default(),
    })
}

fn parse_depobj_init_clause(
    content: &str,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    let (depinfo_source, variable_source) =
        lang::split_once_top_level(content, ':')?.ok_or_else(|| {
            ConversionError::InvalidClauseSyntax(
                "depobj init requires a depinfo modifier and ':'".to_string(),
            )
        })?;
    let depinfo_source = depinfo_source.trim();
    let variable_source = variable_source.trim();
    if depinfo_source.is_empty() || variable_source.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "depobj init modifier and variable must not be empty".to_string(),
        ));
    }

    let open = depinfo_source.find('(').ok_or_else(|| {
        ConversionError::InvalidClauseSyntax(
            "depobj init requires in(locator), out(locator), inout(locator), inoutset(locator), or mutexinoutset(locator)"
                .to_string(),
        )
    })?;
    let dependence = match payload_keyword(depinfo_source[..open].trim(), config).as_ref() {
        "in" => DepobjUpdateDependence::In,
        "out" => DepobjUpdateDependence::Out,
        "inout" => DepobjUpdateDependence::Inout,
        "inoutset" => DepobjUpdateDependence::Inoutset,
        "mutexinoutset" => DepobjUpdateDependence::Mutexinoutset,
        other => {
            return Err(ConversionError::InvalidClauseSyntax(format!(
                "unknown depobj init dependence type: {other}"
            )));
        }
    };
    let locator_source = extract_paren_arg(depinfo_source)?.ok_or_else(|| {
        ConversionError::InvalidClauseSyntax(
            "depobj init depinfo modifier must contain exactly one locator".to_string(),
        )
    })?;
    let mut locators = parse_omp_locator_list(locator_source, config)?;
    if locators.len() != 1 {
        return Err(ConversionError::InvalidClauseSyntax(
            "depobj init depinfo modifier requires exactly one locator".to_string(),
        ));
    }
    Ok(ClauseData::InitDepobj {
        dependence,
        locator: locators.remove(0),
        variable: Variable::parse(variable_source, config)?,
    })
}

fn parse_preference_specifications(
    source: &str,
    config: &ParserConfig,
) -> Result<Vec<OmpPreferenceSpecification>, ConversionError> {
    let source = source.trim();
    if source.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "prefer_type requires at least one preference specification".to_string(),
        ));
    }
    split_top_level_items(source)?
        .into_iter()
        .map(|specification| parse_preference_specification(specification.trim(), config))
        .collect()
}

fn parse_preference_specification(
    source: &str,
    config: &ParserConfig,
) -> Result<OmpPreferenceSpecification, ConversionError> {
    if !source.starts_with('{') {
        return Ok(OmpPreferenceSpecification::ForeignRuntime(
            parse_foreign_runtime_identifier(source, config)?,
        ));
    }
    let (selectors_source, remainder) = lang::extract_bracket_content(source, '{', '}')?;
    if !remainder.trim().is_empty() || selectors_source.trim().is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "preference selector specification must be one non-empty `{...}` block".to_string(),
        ));
    }

    let mut selectors = Vec::new();
    let mut saw_foreign_runtime = false;
    for selector in split_top_level_items(selectors_source)? {
        let selector = selector.trim();
        if let Some(argument) = extract_named_call(selector, "fr", config)? {
            if saw_foreign_runtime {
                return Err(ConversionError::InvalidClauseSyntax(
                    "a preference specification may contain at most one fr selector".to_string(),
                ));
            }
            saw_foreign_runtime = true;
            selectors.push(OmpPreferenceSelector::ForeignRuntime(
                parse_foreign_runtime_identifier(argument, config)?,
            ));
        } else if let Some(argument) = extract_named_call(selector, "attr", config)? {
            let attributes = split_top_level_items(argument)?
                .into_iter()
                .map(|attribute| parse_preference_attribute(attribute.trim(), config))
                .collect::<Result<Vec<_>, _>>()?;
            selectors.push(OmpPreferenceSelector::Attributes(attributes));
        } else {
            return Err(ConversionError::InvalidClauseSyntax(format!(
                "unknown preference selector: {selector}"
            )));
        }
    }
    Ok(OmpPreferenceSpecification::Selectors(selectors))
}

fn parse_foreign_runtime_identifier(
    source: &str,
    config: &ParserConfig,
) -> Result<OmpForeignRuntimeIdentifier, ConversionError> {
    let expression = Expression::new(source.trim(), config)?;
    if let ExprKind::Literal(Literal::String(literal)) = &expression.ast().kind {
        Ok(OmpForeignRuntimeIdentifier::StringLiteral(literal.clone()))
    } else {
        Ok(OmpForeignRuntimeIdentifier::ConstantExpression(expression))
    }
}

fn parse_preference_attribute(
    source: &str,
    config: &ParserConfig,
) -> Result<crate::host::StringLiteral, ConversionError> {
    let expression = Expression::new(source, config)?;
    let ExprKind::Literal(Literal::String(literal)) = &expression.ast().kind else {
        return Err(ConversionError::InvalidClauseSyntax(
            "preference attr values must be string literals".to_string(),
        ));
    };
    if !literal.value.starts_with("ompx_") || literal.value.contains(',') {
        return Err(ConversionError::InvalidClauseSyntax(
            "preference attr strings must start with `ompx_` and contain no comma".to_string(),
        ));
    }
    Ok(literal.clone())
}

fn extract_named_call<'a>(
    source: &'a str,
    expected: &str,
    config: &ParserConfig,
) -> Result<Option<&'a str>, ConversionError> {
    let source = source.trim();
    let Some(open) = source.find('(') else {
        return Ok(None);
    };
    if !payload_keyword_eq(source[..open].trim(), expected, config) {
        return Ok(None);
    }
    let argument = extract_paren_arg(source)?.ok_or_else(|| {
        ConversionError::InvalidClauseSyntax(format!(
            "{expected} must contain exactly one parenthesized argument"
        ))
    })?;
    Ok(Some(argument))
}

pub(crate) fn parse_induction_clause(
    kind: &ClauseKind<'_>,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    let ClauseKind::Parenthesized(content) = kind else {
        return Err(ConversionError::InvalidClauseSyntax(
            "induction clause requires parenthesized content".to_string(),
        ));
    };
    let content = content.as_ref().trim();
    let (modifier_source, items_source) =
        lang::split_once_top_level(content, ':')?.ok_or_else(|| {
            ConversionError::InvalidClauseSyntax(
                "induction requires modifiers followed by ':' and a variable list".to_string(),
            )
        })?;
    let modifier_source = modifier_source.trim();
    let items_source = items_source.trim();
    if modifier_source.is_empty() || items_source.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "induction modifiers and variable list must not be empty".to_string(),
        ));
    }

    let mut modifier = None;
    let mut step = None;
    let mut identifier: Option<OmpInductionIdentifier> = None;
    for entry in split_top_level_items(modifier_source)? {
        let entry = entry.trim();
        let keyword = payload_keyword(entry, config);
        let parsed_modifier = match keyword.as_ref() {
            "relaxed" => Some(OmpInductionModifier::Relaxed),
            "strict" => Some(OmpInductionModifier::Strict),
            _ => None,
        };
        if let Some(parsed_modifier) = parsed_modifier {
            if modifier.replace(parsed_modifier).is_some() {
                return Err(ConversionError::InvalidClauseSyntax(
                    "induction specifies more than one strict/relaxed modifier".to_string(),
                ));
            }
            continue;
        }
        if let Some(step_source) = extract_named_call(entry, "step", config)? {
            if step.is_some() {
                return Err(ConversionError::InvalidClauseSyntax(
                    "induction requires exactly one step modifier".to_string(),
                ));
            }
            step = Some(parse_single_clause_expression(
                step_source,
                config,
                "induction step",
            )?);
            continue;
        }
        if identifier.is_some() {
            return Err(ConversionError::InvalidClauseSyntax(
                "induction requires exactly one induction identifier".to_string(),
            ));
        }
        identifier = Some(
            crate::parser::ast_builder::parse_induction_identifier(entry, config)
                .map_err(|error| ConversionError::InvalidClauseSyntax(error.to_string()))?,
        );
    }

    let step = step.ok_or_else(|| {
        ConversionError::InvalidClauseSyntax(
            "induction requires exactly one step(expression) modifier".to_string(),
        )
    })?;
    let identifier = identifier.ok_or_else(|| {
        ConversionError::InvalidClauseSyntax(
            "induction requires exactly one induction identifier".to_string(),
        )
    })?;
    let items = parse_identifier_list(items_source, config)?;
    if items.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "induction requires a non-empty variable list".to_string(),
        ));
    }

    Ok(ClauseData::Induction {
        modifier,
        step,
        identifier,
        items,
    })
}

/// Parse a linear clause
///
/// Format: `linear([modifier(list):] list[:step])`
///
/// Uses top-level colon detection to properly handle nested structures.
///
/// ## Example
///
/// ```ignore
/// # use roup::ir::ParserConfig;
/// let config = ParserConfig::c();
/// let clause = parse_linear_clause("x, y: 2", &config).unwrap();
/// // Returns ClauseData::Linear with items=[x, y], step=Some(2)
/// ```
pub fn parse_linear_clause(
    content: &str,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    fn modifier_keyword(source: &str, config: &ParserConfig) -> Option<LinearModifier> {
        if payload_keyword_eq(source, "val", config) {
            Some(LinearModifier::Val)
        } else if payload_keyword_eq(source, "ref", config) {
            Some(LinearModifier::Ref)
        } else if payload_keyword_eq(source, "uval", config) {
            Some(LinearModifier::Uval)
        } else {
            None
        }
    }

    let content = content.trim();
    if content.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "linear clause requires a non-empty variable list".to_string(),
        ));
    }

    // OpenMP 5.0/5.1 modifier-prefix form: linear(linear-type(list)[: step]).
    for (keyword, prefix_modifier) in [
        ("val", LinearModifier::Val),
        ("ref", LinearModifier::Ref),
        ("uval", LinearModifier::Uval),
    ] {
        let Some(after_keyword) = strip_payload_keyword(content, keyword, config) else {
            continue;
        };
        let after_keyword = after_keyword.trim_start();
        if !after_keyword.starts_with('(') {
            continue;
        }
        let (items_source, remainder) = extract_parenthesized(after_keyword)?;
        let items = parse_identifier_list(items_source.trim(), config)?;
        if items.is_empty() {
            return Err(ConversionError::InvalidClauseSyntax(
                "linear modifier requires a non-empty variable list".to_string(),
            ));
        }
        let remainder = remainder.trim();
        let step = if remainder.is_empty() {
            None
        } else {
            let Some(step_source) = remainder.strip_prefix(':') else {
                return Err(ConversionError::InvalidClauseSyntax(
                    "linear modifier list must be followed by ':' before its step".to_string(),
                ));
            };
            Some(parse_single_clause_expression(
                step_source.trim(),
                config,
                "linear step",
            )?)
        };
        if items
            .iter()
            .any(|item| matches!(item, ClauseItem::FortranCommonBlock(_)))
        {
            return Err(ConversionError::InvalidClauseSyntax(
                "Fortran named common blocks must not appear in a linear clause".to_string(),
            ));
        }
        return Ok(ClauseData::Linear {
            modifier: Some(prefix_modifier),
            items,
            step,
            source_syntax: LinearSourceSyntax::ModifierPrefix,
        });
    }

    let (items_source, rhs) = match lang::split_once_top_level(content, ':')? {
        Some((items, rhs)) => (items.trim(), Some(rhs.trim())),
        None => (content, None),
    };
    let items = parse_identifier_list(items_source, config)?;
    if items.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "linear clause requires a non-empty variable list".to_string(),
        ));
    }

    if items
        .iter()
        .any(|item| matches!(item, ClauseItem::FortranCommonBlock(_)))
    {
        return Err(ConversionError::InvalidClauseSyntax(
            "Fortran named common blocks must not appear in a linear clause".to_string(),
        ));
    }

    let mut modifier = None;
    let mut step = None;
    let mut source_syntax = LinearSourceSyntax::Historical;
    if let Some(rhs) = rhs {
        if rhs.is_empty() {
            return Err(ConversionError::InvalidClauseSyntax(
                "linear clause requires content after ':'".to_string(),
            ));
        }
        let entries = split_top_level_items(rhs)?;
        let has_canonical_modifier = entries.iter().any(|entry| {
            modifier_keyword(entry.trim(), config).is_some()
                || entry
                    .trim()
                    .get(..4)
                    .is_some_and(|prefix| payload_keyword_eq(prefix, "step", config))
        });
        if has_canonical_modifier {
            source_syntax = LinearSourceSyntax::CanonicalModifiers;
            for entry in entries {
                let entry = entry.trim();
                if let Some(arguments) = parse_exact_complex_modifier(entry, "step", config)? {
                    if step.is_some() {
                        return Err(ConversionError::InvalidClauseSyntax(
                            "linear clause has duplicate step modifiers".to_string(),
                        ));
                    }
                    step = Some(parse_single_clause_expression(
                        arguments,
                        config,
                        "linear step modifier",
                    )?);
                } else if let Some(value) = modifier_keyword(entry, config) {
                    if modifier.replace(value).is_some() {
                        return Err(ConversionError::InvalidClauseSyntax(
                            "linear clause has multiple linear-type modifiers".to_string(),
                        ));
                    }
                } else {
                    return Err(ConversionError::InvalidClauseSyntax(format!(
                        "unknown canonical linear modifier: {entry}"
                    )));
                }
            }
        } else {
            step = Some(parse_single_clause_expression(rhs, config, "linear step")?);
        }
    }

    Ok(ClauseData::Linear {
        modifier,
        items,
        step,
        source_syntax,
    })
}

pub(crate) fn parse_defaultmap_clause(
    kind: &ClauseKind<'_>,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    if let ClauseKind::Parenthesized(content) = kind {
        let text = content.as_ref().trim();
        if text.is_empty() {
            return Err(ConversionError::InvalidClauseSyntax(
                "defaultmap clause requires a behavior".to_string(),
            ));
        }

        let (behavior_str, category_str) =
            if let Some((behavior, rest)) = lang::split_once_top_level(text, ':')? {
                (behavior.trim(), Some(rest.trim()))
            } else {
                (text, None)
            };

        let behavior = parse_defaultmap_behavior(behavior_str, config)?;
        let category = match category_str {
            Some("") => {
                return Err(ConversionError::InvalidClauseSyntax(
                    "defaultmap category must not be empty".to_string(),
                ));
            }
            Some(value) => Some(parse_defaultmap_category(value, config)?),
            None => None,
        };

        Ok(ClauseData::Defaultmap { behavior, category })
    } else {
        Err(ConversionError::InvalidClauseSyntax(
            "defaultmap clause requires parenthesized content".to_string(),
        ))
    }
}

pub(crate) fn parse_metadirective_selector(
    clause: &Clause<'_>,
    config: &ParserConfig,
    source: &LogicalSource<'_>,
) -> Result<ClauseData, ConversionError> {
    if let ClauseKind::Parenthesized(ref content) = clause.kind {
        let raw = content.as_ref();
        let mode = match lookup_clause_name(clause.name.as_ref()) {
            ClauseName::When => SelectorClauseMode::When,
            ClauseName::Match => SelectorClauseMode::Match,
            ClauseName::Otherwise => SelectorClauseMode::Otherwise,
            other => {
                return Err(ConversionError::InvalidClauseSyntax(format!(
                    "{other:?} is not a metadirective selector clause"
                )));
            }
        };
        let selector = parse_selector_content(raw, config, mode, source)?;
        Ok(ClauseData::MetadirectiveSelector {
            selector: Box::new(selector),
        })
    } else {
        Err(ConversionError::InvalidClauseSyntax(
            "metadirective selector requires parentheses".to_string(),
        ))
    }
}

#[derive(Clone, Copy)]
enum SelectorClauseMode {
    When,
    Match,
    Otherwise,
}

fn parse_selector_content(
    content: &str,
    config: &ParserConfig,
    mode: SelectorClauseMode,
    source: &LogicalSource<'_>,
) -> Result<OmpSelector, ConversionError> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "metadirective selector content must not be empty".to_string(),
        ));
    }

    if matches!(mode, SelectorClauseMode::Otherwise) {
        let directive = parse_nested_directive(trimmed, config, source)?.ok_or_else(|| {
            ConversionError::InvalidClauseSyntax(
                "otherwise clause requires a nested directive".to_string(),
            )
        })?;
        return OmpSelector::new(Vec::new(), Some(Box::new(directive)))
            .map_err(|error| ConversionError::InvalidClauseSyntax(error.to_string()));
    }

    let (selector_part, nested_directive_part) = split_selector_and_directive(trimmed)?;
    match mode {
        SelectorClauseMode::When
            if nested_directive_part.is_none_or(|directive| directive.trim().is_empty()) =>
        {
            return Err(ConversionError::InvalidClauseSyntax(
                "when clause requires a directive variant after ':'".to_string(),
            ));
        }
        SelectorClauseMode::Match if nested_directive_part.is_some() => {
            return Err(ConversionError::InvalidClauseSyntax(
                "match clause does not accept a nested directive".to_string(),
            ));
        }
        _ => {}
    }
    if selector_part.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "context selector specification must not be empty".to_string(),
        ));
    }

    let mut entries = Vec::new();
    let mut nested_directive = None;
    let mut saw_device = false;
    let mut saw_target_device = false;
    let mut saw_implementation = false;
    let mut saw_user = false;
    let mut saw_construct = false;

    // Parse selector key/value pairs (device, implementation, user, construct)
    for entry in split_top_level_items(selector_part)? {
        let entry = entry.trim();
        if entry.is_empty() {
            return Err(ConversionError::InvalidClauseSyntax(
                "context selector entries must not be empty".to_string(),
            ));
        }
        let (key, value) = lang::split_once_top_level(entry, '=')?.ok_or_else(|| {
            ConversionError::InvalidClauseSyntax(format!(
                "context selector entry requires '=': {entry}"
            ))
        })?;
        let raw_key = key.trim();
        let key_source = raw_key.trim_start();
        let key_end = key_source
            .char_indices()
            .take_while(|(_, character)| *character == '_' || character.is_alphanumeric())
            .last()
            .map_or(0, |(index, character)| index + character.len_utf8());
        if key_end == 0 {
            return Err(ConversionError::InvalidClauseSyntax(format!(
                "context selector key must be an identifier: {raw_key}"
            )));
        }
        let key_remainder = skip_host_trivia(&key_source[key_end..], config)?;
        if !key_remainder.is_empty() {
            return Err(ConversionError::InvalidClauseSyntax(format!(
                "unexpected tokens after context selector key: {raw_key}"
            )));
        }
        let key = payload_keyword(&key_source[..key_end], config);
        let value = value.trim();
        if key.is_empty() || value.is_empty() {
            return Err(ConversionError::InvalidClauseSyntax(format!(
                "context selector key and value must not be empty: {entry}"
            )));
        }
        match key.as_ref() {
            "device" => {
                if saw_device {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "duplicate device selector".to_string(),
                    ));
                }
                saw_device = true;
                entries.push(OmpSelectorEntry::Device {
                    traits: parse_device_selector(value, config, false)?,
                });
            }
            "target_device" => {
                if saw_target_device {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "duplicate target_device selector".to_string(),
                    ));
                }
                saw_target_device = true;
                entries.push(OmpSelectorEntry::TargetDevice {
                    traits: parse_device_selector(value, config, true)?,
                });
            }
            "implementation" => {
                if saw_implementation {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "duplicate implementation selector".to_string(),
                    ));
                }
                saw_implementation = true;
                entries.push(OmpSelectorEntry::Implementation {
                    traits: parse_impl_selector(value, config)?,
                });
            }
            "user" => {
                if saw_user {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "duplicate user selector".to_string(),
                    ));
                }
                saw_user = true;
                let (score, condition) = parse_user_selector(value, config)?;
                entries.push(OmpSelectorEntry::User {
                    score: score.map(Box::new),
                    condition: Box::new(condition),
                });
            }
            "construct" => {
                if saw_construct {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "duplicate construct selector".to_string(),
                    ));
                }
                saw_construct = true;
                entries.push(OmpSelectorEntry::Construct {
                    constructs: parse_constructs_selector(value, config, source)?,
                });
            }
            _ => {
                return Err(ConversionError::InvalidClauseSyntax(format!(
                    "unknown metadirective selector key: {raw_key}"
                )));
            }
        }
    }

    // Nested directive after colon (parse into nested_directive AST)
    if let Some(nested) = nested_directive_part {
        let nested_trimmed = nested.trim();
        if !nested_trimmed.is_empty()
            && let Some(dir) = parse_nested_directive(nested_trimmed, config, source)?
        {
            nested_directive = Some(Box::new(dir));
        }
    }

    OmpSelector::new(entries, nested_directive)
        .map_err(|error| ConversionError::InvalidClauseSyntax(error.to_string()))
}

pub(crate) fn parse_nested_directive(
    text: &str,
    config: &ParserConfig,
    source: &LogicalSource<'_>,
) -> Result<Option<OmpDirective>, ConversionError> {
    let lexer_lang = map_host_language_to_lexer(config.host_language());
    let nested_config = config.enter_nested_structure().map_err(|message| {
        ConversionError::InvalidClauseSyntax(format!(
            "nested directive exceeds the parser limit: {message}"
        ))
    })?;
    // Use the OpenMP parser with its complete directive/clause registries so
    // combined constructs (e.g., teams distribute parallel for) parse the same
    // way as top-level directives instead of devolving into pseudo-clauses.
    let parser = crate::parser::openmp::parser().with_language(lexer_lang);
    let nested = parser
        .parse_body_ast_in_source(text, &nested_config, source)
        .map_err(|error| {
            ConversionError::InvalidClauseSyntax(format!(
                "invalid nested OpenMP directive: {error}"
            ))
        })?;

    match nested {
        crate::ast::RoupDirective::OpenMp(directive) => Ok(Some(*directive)),
        crate::ast::RoupDirective::OpenAcc(_) => Err(ConversionError::InvalidClauseSyntax(
            "OpenACC directive is not valid in a nested OpenMP directive specification".to_string(),
        )),
    }
}

fn split_selector_and_directive(input: &str) -> Result<(&str, Option<&str>), ConversionError> {
    if let Some((left, right)) = lang::split_once_top_level(input, ':')? {
        Ok((left.trim(), Some(right.trim())))
    } else {
        Ok((input, None))
    }
}

fn map_host_language_to_lexer(lang: HostLanguage) -> LexerLanguage {
    match lang {
        HostLanguage::C | HostLanguage::Cpp => LexerLanguage::C,
        HostLanguage::Fortran => LexerLanguage::FortranFree,
    }
}

fn parse_device_selector(
    value: &str,
    config: &ParserConfig,
    target_device: bool,
) -> Result<Vec<OmpSelectorDeviceTrait>, ConversionError> {
    let mut traits = Vec::new();
    let mut names = HashSet::new();
    let selector_name = if target_device {
        "target_device"
    } else {
        "device"
    };
    let inner = require_selector_braces(value, selector_name, config)?;

    for item in split_top_level_items(inner)? {
        let item = item.trim();
        if item.is_empty() {
            return Err(ConversionError::InvalidClauseSyntax(
                "device selector traits must not be empty".to_string(),
            ));
        }
        if let Some(args) = parse_selector_trait_call(item, "kind", config)? {
            insert_selector_trait_name(&mut names, "kind")?;
            traits.push(OmpSelectorDeviceTrait::NameList(
                parse_name_list_selector_trait(args, OmpSelectorNameListKind::Kind, false, config)?
                    .1,
            ));
        } else if let Some(args) = parse_selector_trait_call(item, "isa", config)? {
            insert_selector_trait_name(&mut names, "isa")?;
            traits.push(OmpSelectorDeviceTrait::NameList(
                parse_name_list_selector_trait(args, OmpSelectorNameListKind::Isa, false, config)?
                    .1,
            ));
        } else if let Some(args) = parse_selector_trait_call(item, "arch", config)? {
            insert_selector_trait_name(&mut names, "arch")?;
            traits.push(OmpSelectorDeviceTrait::NameList(
                parse_name_list_selector_trait(args, OmpSelectorNameListKind::Arch, false, config)?
                    .1,
            ));
        } else if let Some(expr) = parse_selector_trait_call(item, "device_num", config)? {
            insert_selector_trait_name(&mut names, "device_num")?;
            if !target_device {
                return Err(ConversionError::InvalidClauseSyntax(
                    "device_num is only valid in a target_device selector set".to_string(),
                ));
            }
            let (score, val) = parse_scored_value(expr, config)?;
            if score.is_some() {
                return Err(ConversionError::InvalidClauseSyntax(
                    "trait scores are not permitted in target_device selector traits".to_string(),
                ));
            }
            let expr_text = val.trim();
            if expr_text.is_empty() {
                return Err(ConversionError::InvalidClauseSyntax(
                    "device_num selector requires an expression".to_string(),
                ));
            }
            traits.push(OmpSelectorDeviceTrait::DeviceNum(Expression::new(
                expr_text, config,
            )?));
        } else if let Some(args) = parse_selector_trait_call(item, "uid", config)? {
            insert_selector_trait_name(&mut names, "uid")?;
            if !target_device {
                return Err(ConversionError::InvalidClauseSyntax(
                    "uid is only valid in a target_device selector set".to_string(),
                ));
            }
            let (score, value) = parse_scored_value(args, config)?;
            if score.is_some() {
                return Err(ConversionError::InvalidClauseSyntax(
                    "trait scores are not permitted in target_device selector traits".to_string(),
                ));
            }
            if split_top_level_items(value)?.len() != 1 {
                return Err(ConversionError::InvalidClauseSyntax(
                    "uid selector requires exactly one property".to_string(),
                ));
            }
            traits.push(OmpSelectorDeviceTrait::Uid(parse_selector_trait_value(
                value, config,
            )?));
        } else {
            let extension = parse_selector_extension_trait(item, config)?;
            insert_selector_trait_name(&mut names, extension.name().as_str())?;
            traits.push(OmpSelectorDeviceTrait::Extension(extension));
        }
    }

    if traits.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "device selector requires at least one trait".to_string(),
        ));
    }
    Ok(traits)
}

fn parse_impl_selector(
    value: &str,
    config: &ParserConfig,
) -> Result<Vec<OmpSelectorImplementationTrait>, ConversionError> {
    let mut traits = Vec::new();
    let mut names = HashSet::new();
    let inner = require_selector_braces(value, "implementation", config)?;

    for item in split_top_level_items(inner)? {
        let item = item.trim();
        if item.is_empty() {
            return Err(ConversionError::InvalidClauseSyntax(
                "implementation selector traits must not be empty".to_string(),
            ));
        }
        if let Some(args) = parse_selector_trait_call(item, "vendor", config)? {
            insert_selector_trait_name(&mut names, "vendor")?;
            let (score, name_list) = parse_name_list_selector_trait(
                args,
                OmpSelectorNameListKind::Vendor,
                true,
                config,
            )?;
            traits.push(OmpSelectorImplementationTrait::new(
                score,
                OmpSelectorImplementationTraitKind::NameList(name_list),
            ));
        } else if let Some(args) = parse_selector_trait_call(item, "extension", config)? {
            insert_selector_trait_name(&mut names, "extension")?;
            let (score, name_list) = parse_name_list_selector_trait(
                args,
                OmpSelectorNameListKind::Extension,
                true,
                config,
            )?;
            traits.push(OmpSelectorImplementationTrait::new(
                score,
                OmpSelectorImplementationTraitKind::NameList(name_list),
            ));
        } else if let Some(args) =
            parse_selector_trait_call(item, "atomic_default_mem_order", config)?
        {
            insert_selector_trait_name(&mut names, "atomic_default_mem_order")?;
            let (score, order) = parse_scored_value(args, config)?;
            if split_top_level_items(order)?.len() != 1 {
                return Err(ConversionError::InvalidClauseSyntax(
                    "atomic_default_mem_order requires exactly one memory order".to_string(),
                ));
            }
            traits.push(OmpSelectorImplementationTrait::new(
                score,
                OmpSelectorImplementationTraitKind::AtomicDefaultMemOrder(parse_memory_order(
                    order, config,
                )?),
            ));
        } else if let Some(args) = parse_selector_trait_call(item, "requires", config)? {
            insert_selector_trait_name(&mut names, "requires")?;
            let (score, properties) = parse_scored_value(args, config)?;
            let requirements = split_top_level_items(properties)?
                .into_iter()
                .map(|property| parse_selector_requirement(property.trim(), config))
                .collect::<Result<Vec<_>, _>>()?;
            if requirements.is_empty() {
                return Err(ConversionError::InvalidClauseSyntax(
                    "requires selector needs at least one clause property".to_string(),
                ));
            }
            traits.push(OmpSelectorImplementationTrait::new(
                score,
                OmpSelectorImplementationTraitKind::Requires(requirements),
            ));
        } else {
            let (name, argument) = parse_selector_named_form(item, config)?;
            insert_selector_trait_name(&mut names, name.as_str())?;
            let normalized = payload_keyword(name.as_str(), config);
            let kind = if let Some(requirement) =
                parse_legacy_selector_requirement(normalized.as_ref(), argument, config)?
            {
                OmpSelectorImplementationTraitKind::Requirement(requirement)
            } else {
                let (score, properties) = match argument {
                    Some(argument) => {
                        let (score, properties) = parse_scored_value(argument, config)?;
                        (score, Some(properties))
                    }
                    None => (None, None),
                };
                traits.push(OmpSelectorImplementationTrait::new(
                    score,
                    OmpSelectorImplementationTraitKind::Extension(
                        parse_selector_extension_trait_from_parts(name, properties, config)?,
                    ),
                ));
                continue;
            };
            traits.push(OmpSelectorImplementationTrait::new(None, kind));
        }
    }

    if traits.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "implementation selector requires at least one trait".to_string(),
        ));
    }
    Ok(traits)
}

fn insert_selector_trait_name(
    names: &mut HashSet<String>,
    name: &str,
) -> Result<(), ConversionError> {
    if names.insert(name.to_string()) {
        Ok(())
    } else {
        Err(ConversionError::InvalidClauseSyntax(format!(
            "duplicate {name} selector trait"
        )))
    }
}

fn parse_name_list_selector_trait(
    arguments: &str,
    kind: OmpSelectorNameListKind,
    allow_score: bool,
    config: &ParserConfig,
) -> Result<(Option<Expression>, OmpSelectorNameListTrait), ConversionError> {
    let (score, property_source) = parse_scored_value(arguments, config)?;
    if score.is_some() && !allow_score {
        return Err(ConversionError::InvalidClauseSyntax(
            "trait scores are permitted only in the implementation selector set".to_string(),
        ));
    }
    let properties = split_top_level_items(property_source)?
        .into_iter()
        .map(|property| parse_selector_trait_value(property.trim(), config))
        .collect::<Result<Vec<_>, _>>()?;
    let name_list = OmpSelectorNameListTrait::new(kind, properties)
        .map_err(|error| ConversionError::InvalidClauseSyntax(error.to_string()))?;
    Ok((score, name_list))
}

fn parse_selector_requirement(
    source: &str,
    config: &ParserConfig,
) -> Result<OmpSelectorRequirement, ConversionError> {
    let (name, argument) = parse_selector_named_form(source, config)?;
    let normalized = payload_keyword(name.as_str(), config);
    let requirement = match normalized.as_ref() {
        "atomic_default_mem_order" => {
            let order = argument.ok_or_else(|| {
                ConversionError::InvalidClauseSyntax(
                    "atomic_default_mem_order requires one memory-order property".to_string(),
                )
            })?;
            return Ok(OmpSelectorRequirement::new(
                RequireModifier::AtomicDefaultMemOrder(parse_memory_order(order, config)?),
                None,
            ));
        }
        "reverse_offload" => RequireModifier::ReverseOffload,
        "unified_address" => RequireModifier::UnifiedAddress,
        "unified_shared_memory" => RequireModifier::UnifiedSharedMemory,
        "dynamic_allocators" => RequireModifier::DynamicAllocators,
        "self_maps" => RequireModifier::SelfMaps,
        "device_safesync" => RequireModifier::DeviceSafesync,
        name if name.starts_with("ext_") => RequireModifier::ExtImplementationDefinedRequirement(
            Some(name_from_selector(name, config)?),
        ),
        _ => {
            return Err(ConversionError::InvalidClauseSyntax(format!(
                "unknown requires selector clause property: {source}"
            )));
        }
    };
    let required = argument
        .map(str::trim)
        .filter(|argument| !argument.is_empty())
        .map(|argument| Expression::new(argument, config))
        .transpose()?;
    Ok(OmpSelectorRequirement::new(requirement, required))
}

fn parse_legacy_selector_requirement(
    name: &str,
    argument: Option<&str>,
    config: &ParserConfig,
) -> Result<Option<RequireModifier>, ConversionError> {
    let requirement = match name {
        "reverse_offload" => Some(RequireModifier::ReverseOffload),
        "unified_address" => Some(RequireModifier::UnifiedAddress),
        "unified_shared_memory" => Some(RequireModifier::UnifiedSharedMemory),
        "dynamic_allocators" => Some(RequireModifier::DynamicAllocators),
        "self_maps" => Some(RequireModifier::SelfMaps),
        "device_safesync" => Some(RequireModifier::DeviceSafesync),
        _ => None,
    };
    if requirement.is_some() && argument.is_some() {
        return Err(ConversionError::InvalidClauseSyntax(format!(
            "legacy implementation trait {name} is a non-property trait"
        )));
    }
    let _ = config;
    Ok(requirement)
}

fn name_from_selector(name: &str, config: &ParserConfig) -> Result<Identifier, ConversionError> {
    Identifier::new(payload_keyword(name, config).as_ref()).map_err(Into::into)
}

fn parse_selector_extension_trait(
    source: &str,
    config: &ParserConfig,
) -> Result<OmpSelectorExtensionTrait, ConversionError> {
    let (name, argument) = parse_selector_named_form(source, config)?;
    parse_selector_extension_trait_from_parts(name, argument, config)
}

fn parse_selector_extension_trait_from_parts(
    name: Identifier,
    argument: Option<&str>,
    config: &ParserConfig,
) -> Result<OmpSelectorExtensionTrait, ConversionError> {
    let properties = argument
        .map(split_top_level_items)
        .transpose()?
        .unwrap_or_default()
        .into_iter()
        .map(|property| parse_selector_extension_property(property.trim(), config))
        .collect::<Result<Vec<_>, _>>()?;
    OmpSelectorExtensionTrait::new(name, properties)
        .map_err(|error| ConversionError::InvalidClauseSyntax(error.to_string()))
}

fn parse_selector_extension_property(
    source: &str,
    config: &ParserConfig,
) -> Result<OmpSelectorExtensionProperty, ConversionError> {
    if let Ok(value) = parse_selector_trait_value(source, config) {
        return Ok(OmpSelectorExtensionProperty::Name(value));
    }
    if let Some((name, argument)) = parse_selector_extension_property_call(source, config)? {
        let nested_config = config.enter_nested_structure().map_err(|message| {
            ConversionError::InvalidClauseSyntax(format!(
                "nested selector extension property exceeds the parser limit: {message}"
            ))
        })?;
        let properties = split_top_level_items(argument)?
            .into_iter()
            .map(|property| parse_selector_extension_property(property.trim(), &nested_config))
            .collect::<Result<Vec<_>, _>>()?;
        if properties.is_empty() {
            return Err(ConversionError::InvalidClauseSyntax(
                "extension property calls require at least one property".to_string(),
            ));
        }
        return Ok(OmpSelectorExtensionProperty::Call { name, properties });
    }
    Ok(OmpSelectorExtensionProperty::ConstantInteger(
        Expression::new(source, config)?,
    ))
}

fn parse_selector_extension_property_call<'a>(
    source: &'a str,
    config: &ParserConfig,
) -> Result<Option<(Identifier, &'a str)>, ConversionError> {
    let trimmed = source.trim();
    let name_end = trimmed
        .char_indices()
        .take_while(|(_, character)| *character == '_' || character.is_alphanumeric())
        .last()
        .map_or(0, |(index, character)| index + character.len_utf8());
    if name_end == 0 {
        return Ok(None);
    }
    let rest = skip_host_trivia(&trimmed[name_end..], config)?;
    if !rest.starts_with('(') {
        return Ok(None);
    }
    let end = lang::find_matching_delimiter(rest, 0, '(', ')')?.ok_or_else(|| {
        ConversionError::InvalidClauseSyntax(
            "unbalanced selector extension property call".to_string(),
        )
    })?;
    if !skip_host_trivia(&rest[end + 1..], config)?.is_empty() {
        return Ok(None);
    }
    let argument = rest[1..end].trim();
    if argument.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "extension property calls require at least one property".to_string(),
        ));
    }
    Ok(Some((
        name_from_selector(&trimmed[..name_end], config)?,
        argument,
    )))
}

fn parse_selector_named_form<'a>(
    source: &'a str,
    config: &ParserConfig,
) -> Result<(Identifier, Option<&'a str>), ConversionError> {
    let trimmed = source.trim();
    let name_end = trimmed
        .char_indices()
        .take_while(|(_, character)| *character == '_' || character.is_alphanumeric())
        .last()
        .map_or(0, |(index, character)| index + character.len_utf8());
    if name_end == 0 {
        return Err(ConversionError::InvalidClauseSyntax(format!(
            "selector trait name must be an identifier: {source}"
        )));
    }
    let name = name_from_selector(&trimmed[..name_end], config)?;
    let rest = skip_host_trivia(&trimmed[name_end..], config)?;
    if rest.is_empty() {
        return Ok((name, None));
    }
    if !rest.starts_with('(') {
        return Err(ConversionError::InvalidClauseSyntax(format!(
            "unexpected tokens after selector trait name: {source}"
        )));
    }
    let end = lang::find_matching_delimiter(rest, 0, '(', ')')?.ok_or_else(|| {
        ConversionError::InvalidClauseSyntax("unbalanced selector trait property list".to_string())
    })?;
    if !skip_host_trivia(&rest[end + 1..], config)?.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(format!(
            "unexpected suffix after selector trait: {source}"
        )));
    }
    let argument = rest[1..end].trim();
    if argument.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "selector trait property list must not be empty".to_string(),
        ));
    }
    Ok((name, Some(argument)))
}

fn parse_user_selector(
    value: &str,
    config: &ParserConfig,
) -> Result<(Option<Expression>, Expression), ConversionError> {
    let inner = require_selector_braces(value, "user", config)?;

    if let Some(expr_body) = parse_selector_trait_call(inner, "condition", config)? {
        let (score, condition) = parse_scored_value(expr_body, config)?;
        let expr_text = condition.trim();
        if expr_text.is_empty() {
            return Err(ConversionError::InvalidClauseSyntax(
                "user condition requires an expression".to_string(),
            ));
        }
        Ok((score, Expression::new(expr_text, config)?))
    } else {
        Err(ConversionError::InvalidClauseSyntax(format!(
            "unknown user selector trait: {inner}"
        )))
    }
}

fn parse_constructs_selector(
    value: &str,
    config: &ParserConfig,
    source: &LogicalSource<'_>,
) -> Result<Vec<OmpSelectorConstruct>, ConversionError> {
    let mut constructs = Vec::new();
    let inner = require_selector_braces(value, "construct", config)?;

    for item in split_top_level_items(inner)? {
        let text = item.trim();
        if text.is_empty() {
            return Err(ConversionError::InvalidClauseSyntax(
                "construct selector entries must not be empty".to_string(),
            ));
        }

        if text
            .get(.."score".len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("score"))
        {
            return Err(ConversionError::InvalidClauseSyntax(
                "trait scores are not permitted in the construct selector set".to_string(),
            ));
        }

        let lexer_lang = map_host_language_to_lexer(config.host_language());
        let parser = crate::parser::openmp::parser().with_language(lexer_lang);
        let directive = parser
            .parse_construct_trait_ast_in_source(text, config, source)
            .map_err(|error| {
                ConversionError::InvalidClauseSyntax(format!(
                    "invalid construct selector trait: {error}"
                ))
            })?;
        constructs.push(OmpSelectorConstruct::new(directive));
    }

    if constructs.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "construct selector requires at least one directive".to_string(),
        ));
    }
    Ok(constructs)
}

fn parse_scored_value<'a>(
    input: &'a str,
    config: &ParserConfig,
) -> Result<(Option<Expression>, &'a str), ConversionError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "selector trait value must not be empty".to_string(),
        ));
    }

    let Some(prefix) = trimmed.get(.."score".len()) else {
        return Ok((None, trimmed));
    };
    if !prefix.eq_ignore_ascii_case("score") {
        return Ok((None, trimmed));
    }
    if !payload_keyword_eq(prefix, "score", config) {
        return Err(ConversionError::InvalidClauseSyntax(format!(
            "payload keyword {prefix:?} is case-sensitive in C and C++"
        )));
    }
    let after_keyword = &trimmed["score".len()..];
    if after_keyword
        .chars()
        .next()
        .is_some_and(|ch| ch == '_' || ch.is_ascii_alphanumeric())
    {
        return Ok((None, trimmed));
    }

    let after_keyword = after_keyword.trim_start();
    if !after_keyword.starts_with('(') {
        return Ok((None, trimmed));
    }
    let end = lang::find_matching_delimiter(after_keyword, 0, '(', ')')?.ok_or_else(|| {
        ConversionError::InvalidClauseSyntax("unbalanced selector score".to_string())
    })?;
    let score_text = after_keyword[1..end].trim();
    if score_text.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "selector score expression must not be empty".to_string(),
        ));
    }
    let remainder = after_keyword[end + 1..].trim_start();
    let value = remainder.strip_prefix(':').ok_or_else(|| {
        ConversionError::InvalidClauseSyntax(
            "selector score must be followed by ':' and a trait value".to_string(),
        )
    })?;
    let value = value.trim();
    if value.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "selector score requires a trait value".to_string(),
        ));
    }

    Ok((Some(Expression::new(score_text, config)?), value))
}

fn parse_selector_trait_value(
    input: &str,
    config: &ParserConfig,
) -> Result<OmpSelectorTraitValue, ConversionError> {
    let expression = Expression::new(input, config)?;
    match &expression.ast().kind {
        crate::host::ExprKind::Name(name) if !name.global && name.segments.len() == 1 => {
            Ok(OmpSelectorTraitValue::Identifier(name.segments[0].clone()))
        }
        crate::host::ExprKind::Literal(crate::host::Literal::String(literal)) => {
            Ok(OmpSelectorTraitValue::StringLiteral(literal.clone()))
        }
        _ => Err(ConversionError::InvalidClauseSyntax(format!(
            "selector trait value must be an identifier or string literal: {input}"
        ))),
    }
}

fn require_selector_braces<'a>(
    value: &'a str,
    selector_name: &str,
    config: &ParserConfig,
) -> Result<&'a str, ConversionError> {
    let trimmed = value.trim();
    if !trimmed.starts_with('{') {
        return Err(ConversionError::InvalidClauseSyntax(format!(
            "{selector_name} selector requires '{{...}}'"
        )));
    }
    let end = lang::find_matching_delimiter(trimmed, 0, '{', '}')?.ok_or_else(|| {
        ConversionError::InvalidClauseSyntax(format!(
            "unbalanced braces in {selector_name} selector"
        ))
    })?;
    let suffix = skip_host_trivia(&trimmed[end + 1..], config)?;
    if !suffix.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(format!(
            "unexpected suffix after {selector_name} selector"
        )));
    }
    let inner = trimmed[1..end].trim();
    if inner.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(format!(
            "{selector_name} selector must not be empty"
        )));
    }
    Ok(inner)
}

fn parse_selector_trait_call<'a>(
    input: &'a str,
    keyword: &str,
    config: &ParserConfig,
) -> Result<Option<&'a str>, ConversionError> {
    let Some(prefix) = input.get(..keyword.len()) else {
        return Ok(None);
    };
    if !prefix.eq_ignore_ascii_case(keyword) {
        return Ok(None);
    }
    if !payload_keyword_eq(prefix, keyword, config) {
        return Err(ConversionError::InvalidClauseSyntax(format!(
            "payload keyword {prefix:?} is case-sensitive in C and C++"
        )));
    }
    let rest = &input[keyword.len()..];
    if rest
        .chars()
        .next()
        .is_some_and(|ch| ch == '_' || ch.is_ascii_alphanumeric())
    {
        return Ok(None);
    }
    let argument = extract_paren_arg(rest)?.ok_or_else(|| {
        ConversionError::InvalidClauseSyntax(format!(
            "{keyword} selector trait requires exactly one parenthesized argument"
        ))
    })?;
    if argument.trim().is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(format!(
            "{keyword} selector trait argument must not be empty"
        )));
    }
    Ok(Some(argument))
}

fn extract_paren_arg(input: &str) -> Result<Option<&str>, ConversionError> {
    let trimmed = input.trim();
    let Some(start) = trimmed.find('(') else {
        return Ok(None);
    };
    let end = lang::find_matching_delimiter(trimmed, start, '(', ')')?.ok_or_else(|| {
        ConversionError::InvalidClauseSyntax("unbalanced parenthesized argument".to_string())
    })?;
    if end + 1 == trimmed.len() {
        return Ok(Some(&trimmed[start + 1..end]));
    }
    Ok(None)
}

fn parse_defaultmap_behavior(
    value: &str,
    config: &ParserConfig,
) -> Result<DefaultmapBehavior, ConversionError> {
    let raw = value.trim();
    let normalized = payload_keyword(raw, config);
    let behavior = match normalized.as_ref() {
        "alloc" => DefaultmapBehavior::Alloc,
        "to" => DefaultmapBehavior::To,
        "from" => DefaultmapBehavior::From,
        "tofrom" => DefaultmapBehavior::Tofrom,
        "firstprivate" => DefaultmapBehavior::Firstprivate,
        "none" => DefaultmapBehavior::None,
        "default" => DefaultmapBehavior::Default,
        "present" => DefaultmapBehavior::Present,
        "private" => DefaultmapBehavior::Private,
        "self" => DefaultmapBehavior::SelfMap,
        "storage" => DefaultmapBehavior::Storage,
        _ => {
            return Err(ConversionError::InvalidClauseSyntax(format!(
                "Unknown defaultmap behavior: {raw}"
            )));
        }
    };
    Ok(behavior)
}

fn parse_defaultmap_category(
    value: &str,
    config: &ParserConfig,
) -> Result<DefaultmapCategory, ConversionError> {
    let raw = value.trim();
    let normalized = payload_keyword(raw, config);
    let category = match normalized.as_ref() {
        "scalar" => DefaultmapCategory::Scalar,
        "aggregate" => DefaultmapCategory::Aggregate,
        "pointer" => DefaultmapCategory::Pointer,
        "all" => DefaultmapCategory::All,
        "allocatable" => DefaultmapCategory::Allocatable,
        _ => {
            return Err(ConversionError::InvalidClauseSyntax(format!(
                "Unknown defaultmap category: {raw}"
            )));
        }
    };
    Ok(category)
}

pub(crate) fn parse_uses_allocators_clause(
    kind: &ClauseKind<'_>,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    let ClauseKind::Parenthesized(content) = kind else {
        return Err(ConversionError::InvalidClauseSyntax(
            "uses_allocators clause requires parenthesized content".to_string(),
        ));
    };

    let content = content.as_ref().trim();
    if content.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "uses_allocators clause requires at least one allocator".to_string(),
        ));
    }

    let clause_argument_specs =
        lang::split_top_level(content, ';', &[('(', ')'), ('[', ']'), ('{', '}')])?;

    let allocators = if clause_argument_specs.len() > 1 {
        clause_argument_specs
            .into_iter()
            .map(|entry| parse_uses_allocator_modifier_spec(entry, config))
            .collect::<Result<Vec<_>, _>>()?
    } else if lang::split_once_top_level(content, ':')?.is_some() {
        vec![parse_uses_allocator_modifier_spec(content, config)?]
    } else {
        split_top_level_items(content)?
            .into_iter()
            .map(|entry| parse_historical_uses_allocator_spec(entry, config))
            .collect::<Result<Vec<_>, _>>()?
    };

    Ok(ClauseData::UsesAllocators { allocators })
}

fn parse_historical_uses_allocator_spec(
    entry: &str,
    config: &ParserConfig,
) -> Result<UsesAllocatorSpec, ConversionError> {
    let (allocator_name, traits_source) = split_allocator_entry(entry)?;
    let allocator = classify_allocator_name(allocator_name, config)?;
    let traits = traits_source
        .map(|source| Variable::parse(source.trim(), config).map_err(ConversionError::from))
        .transpose()?;

    if traits.is_some() && matches!(allocator, UsesAllocatorKind::Builtin(_)) {
        return Err(ConversionError::InvalidClauseSyntax(
            "predefined allocators cannot specify allocator traits".to_string(),
        ));
    }

    UsesAllocatorSpec::new(
        allocator,
        traits,
        None,
        UsesAllocatorSourceSyntax::Historical,
    )
    .map_err(|message| ConversionError::InvalidClauseSyntax(message.to_string()))
}

fn parse_uses_allocator_modifier_spec(
    entry: &str,
    config: &ParserConfig,
) -> Result<UsesAllocatorSpec, ConversionError> {
    let entry = entry.trim();
    let (modifier_source, allocator_source) =
        if let Some((modifiers, allocator)) = lang::split_once_top_level(entry, ':')? {
            let modifiers = modifiers.trim();
            let allocator = allocator.trim();
            if modifiers.is_empty() || allocator.is_empty() {
                return Err(ConversionError::InvalidClauseSyntax(
                    "uses_allocators modifiers and allocator must not be empty".to_string(),
                ));
            }
            (Some(modifiers), allocator)
        } else {
            (None, entry)
        };

    let allocator = classify_allocator_name(allocator_source, config)?;
    let mut traits = None;
    let mut memspace = None;

    if let Some(modifier_source) = modifier_source {
        for raw_modifier in split_top_level_items(modifier_source)? {
            let raw_modifier = raw_modifier.trim();
            if let Some(arguments) = allocator_modifier_arguments(raw_modifier, "traits", config)? {
                if traits
                    .replace(Variable::parse(arguments.trim(), config)?)
                    .is_some()
                {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "duplicate traits uses_allocators modifier".to_string(),
                    ));
                }
                continue;
            }
            if let Some(arguments) = allocator_modifier_arguments(raw_modifier, "memspace", config)?
            {
                if memspace
                    .replace(parse_omp_memory_space(arguments, config)?)
                    .is_some()
                {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "duplicate memspace uses_allocators modifier".to_string(),
                    ));
                }
                continue;
            }

            return Err(ConversionError::InvalidClauseSyntax(format!(
                "unknown uses_allocators modifier: {raw_modifier}"
            )));
        }
    }

    if matches!(allocator, UsesAllocatorKind::Builtin(_))
        && (traits.is_some() || memspace.is_some())
    {
        return Err(ConversionError::InvalidClauseSyntax(
            "predefined allocators cannot have uses_allocators modifiers".to_string(),
        ));
    }

    UsesAllocatorSpec::new(
        allocator,
        traits,
        memspace,
        UsesAllocatorSourceSyntax::Modifier,
    )
    .map_err(|message| ConversionError::InvalidClauseSyntax(message.to_string()))
}

fn allocator_modifier_arguments<'a>(
    modifier: &'a str,
    expected_name: &str,
    config: &ParserConfig,
) -> Result<Option<&'a str>, ConversionError> {
    let Some(open) = modifier.find('(') else {
        return Ok(None);
    };
    if !payload_keyword_eq(modifier[..open].trim(), expected_name, config) {
        return Ok(None);
    }
    let (arguments, remainder) = lang::extract_bracket_content(&modifier[open..], '(', ')')?;
    if !remainder.trim().is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(format!(
            "unexpected text after {expected_name} uses_allocators modifier"
        )));
    }
    Ok(Some(arguments))
}

fn parse_omp_memory_space(
    source: &str,
    config: &ParserConfig,
) -> Result<OmpMemorySpace, ConversionError> {
    let source = source.trim();
    if source.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "uses_allocators memspace name must not be empty".to_string(),
        ));
    }
    let keyword = payload_keyword(source, config);
    Ok(match keyword.as_ref() {
        "omp_default_mem_space" => OmpMemorySpace::Default,
        "omp_large_cap_mem_space" => OmpMemorySpace::LargeCap,
        "omp_const_mem_space" => OmpMemorySpace::Const,
        "omp_high_bw_mem_space" => OmpMemorySpace::HighBw,
        "omp_low_lat_mem_space" => OmpMemorySpace::LowLat,
        _ => {
            return Err(ConversionError::InvalidClauseSyntax(format!(
                "unknown predefined OpenMP memory space: {source}"
            )));
        }
    })
}

pub(crate) fn parse_memory_order(
    value: &str,
    config: &ParserConfig,
) -> Result<MemoryOrder, ConversionError> {
    let raw = value.trim();
    let keyword = payload_keyword(raw, config);
    match keyword.as_ref() {
        "seq_cst" => Ok(MemoryOrder::SeqCst),
        "acq_rel" => Ok(MemoryOrder::AcqRel),
        "release" => Ok(MemoryOrder::Release),
        "acquire" => Ok(MemoryOrder::Acquire),
        "relaxed" => Ok(MemoryOrder::Relaxed),
        _ => Err(ConversionError::InvalidClauseSyntax(format!(
            "Unknown memory order: {raw}"
        ))),
    }
}

pub(crate) fn parse_device_clause(
    kind: &ClauseKind<'_>,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    if let ClauseKind::Parenthesized(content) = kind {
        let text = content.as_ref().trim();
        let (modifier, expr_text) = match lang::split_once_top_level(text, ':')? {
            Some((name, rest)) if payload_keyword_eq(name.trim(), "ancestor", config) => {
                (Some(DeviceModifier::Ancestor), rest.trim())
            }
            Some((name, rest)) if payload_keyword_eq(name.trim(), "device_num", config) => {
                (Some(DeviceModifier::DeviceNum), rest.trim())
            }
            Some((name, _)) => {
                return Err(ConversionError::InvalidClauseSyntax(format!(
                    "unknown device modifier: {}",
                    name.trim()
                )));
            }
            None => (None, text),
        };

        if expr_text.is_empty() {
            return Err(ConversionError::InvalidClauseSyntax(
                "device clause requires a non-empty expression".to_string(),
            ));
        }

        Ok(ClauseData::Device {
            modifier,
            device_num: parse_single_clause_expression(expr_text, config, "device clause")?,
        })
    } else {
        Err(ConversionError::InvalidClauseSyntax(
            "device clause requires parenthesized expression".to_string(),
        ))
    }
}

fn split_allocator_entry(input: &str) -> Result<(&str, Option<&str>), ConversionError> {
    let input = input.trim();
    if let Some(start) = input.find('(') {
        let name = input[..start].trim();
        if name.is_empty() {
            return Err(ConversionError::InvalidClauseSyntax(
                "uses_allocators entry requires an allocator before its traits".to_string(),
            ));
        }
        let (traits, remainder) = lang::extract_bracket_content(&input[start..], '(', ')')?;
        if !remainder.trim().is_empty() {
            return Err(ConversionError::InvalidClauseSyntax(
                "unexpected text after historical uses_allocators traits".to_string(),
            ));
        }
        Ok((name, Some(traits)))
    } else {
        Ok((input, None))
    }
}

pub(crate) fn split_top_level_items(input: &str) -> Result<Vec<&str>, ConversionError> {
    parse_variable_list(input).map_err(|error| {
        ConversionError::InvalidClauseSyntax(format!("malformed comma-separated list: {error:?}"))
    })
}

pub(crate) fn classify_allocator_name(
    name: &str,
    config: &ParserConfig,
) -> Result<UsesAllocatorKind, ConversionError> {
    let trimmed = name.trim();
    let canonical = if matches!(config.host_language(), HostLanguage::Fortran) {
        Cow::Owned(trimmed.to_ascii_lowercase())
    } else {
        Cow::Borrowed(trimmed)
    };
    Ok(match canonical.as_ref() {
        "omp_null_allocator" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::Null),
        "omp_default_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::Default),
        "omp_large_cap_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::LargeCap),
        "omp_const_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::Const),
        "omp_high_bw_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::HighBw),
        "omp_low_lat_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::LowLat),
        "omp_cgroup_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::Cgroup),
        "omp_pteam_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::Pteam),
        "omp_thread_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::Thread),
        _ => UsesAllocatorKind::Custom(Identifier::new(trimmed)?),
    })
}

fn parse_allocate_clause(
    content: &str,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    let content = content.trim();
    if content.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "allocate clause requires a variable list".to_string(),
        ));
    }

    let Some((prefix, list_source)) = lang::split_once_top_level(content, ':')? else {
        return Ok(ClauseData::Allocate {
            allocator: None,
            alignment: None,
            items: parse_identifier_list(content, config)?,
            source_syntax: AllocateSourceSyntax::Unmodified,
        });
    };
    let prefix = prefix.trim();
    let list_source = list_source.trim();
    if prefix.is_empty() || list_source.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "allocate modifiers and variable list must not be empty".to_string(),
        ));
    }

    let entries = split_top_level_items(prefix)?;
    let uses_complex_modifiers = entries.len() > 1
        || extract_named_call(entries[0], "allocator", config)?.is_some()
        || extract_named_call(entries[0], "align", config)?.is_some();

    if !uses_complex_modifiers {
        return Ok(ClauseData::Allocate {
            allocator: Some(parse_single_clause_expression(
                prefix,
                config,
                "allocate allocator",
            )?),
            alignment: None,
            items: parse_identifier_list(list_source, config)?,
            source_syntax: AllocateSourceSyntax::SimpleAllocator,
        });
    }

    let mut allocator = None;
    let mut alignment = None;
    for entry in entries {
        if let Some(argument) = extract_named_call(entry, "allocator", config)? {
            if allocator.is_some() {
                return Err(ConversionError::InvalidClauseSyntax(
                    "duplicate allocator modifier in allocate clause".to_string(),
                ));
            }
            allocator = Some(parse_single_clause_expression(
                argument,
                config,
                "allocate allocator modifier",
            )?);
            continue;
        }
        if let Some(argument) = extract_named_call(entry, "align", config)? {
            if alignment.is_some() {
                return Err(ConversionError::InvalidClauseSyntax(
                    "duplicate align modifier in allocate clause".to_string(),
                ));
            }
            let expression =
                parse_single_clause_expression(argument, config, "allocate align modifier")?;
            match obvious_integer_value(&expression) {
                ObviousIntegerValue::NonNegative(value)
                    if value != 0 && value.is_power_of_two() => {}
                ObviousIntegerValue::Unknown => {}
                _ => return Err(ConversionError::InvalidClauseSyntax(
                    "allocate align modifier requires a positive integer power-of-two expression"
                        .to_string(),
                )),
            }
            alignment = Some(expression);
            continue;
        }
        return Err(ConversionError::InvalidClauseSyntax(format!(
            "unknown allocate modifier: {}",
            entry.trim()
        )));
    }

    Ok(ClauseData::Allocate {
        allocator,
        alignment,
        items: parse_identifier_list(list_source, config)?,
        source_syntax: AllocateSourceSyntax::Modifiers,
    })
}

pub(crate) fn parse_scan_clause(
    mode: ScanClauseMode,
    clause: &Clause<'_>,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    let items = match &clause.kind {
        ClauseKind::Parenthesized(content) => parse_identifier_list(content.as_ref(), config)?,
        _ => {
            return Err(ConversionError::InvalidClauseSyntax(
                "scan clause requires a variable list".to_string(),
            ));
        }
    };

    if items.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "scan clause requires a non-empty variable list".to_string(),
        ));
    }

    Ok(ClauseData::Scan { mode, items })
}

fn parse_firstprivate_clause(
    clause: &Clause<'_>,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    let ClauseKind::Parenthesized(content) = &clause.kind else {
        return Err(ConversionError::InvalidClauseSyntax(
            "firstprivate clause requires a parenthesized variable list".to_string(),
        ));
    };

    let content = content.as_ref().trim();
    if content.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "firstprivate clause requires a non-empty variable list".to_string(),
        ));
    }

    let mut modifier = None;
    let items_source =
        if let Some((modifier_source, items_source)) = lang::split_once_top_level(content, ':')? {
            let modifier_source = modifier_source.trim();
            let items_source = items_source.trim();
            if modifier_source.is_empty() || items_source.is_empty() {
                return Err(ConversionError::InvalidClauseSyntax(
                    "firstprivate modifiers and variable list must not be empty".to_string(),
                ));
            }

            for raw_modifier in split_top_level_items(modifier_source)? {
                let raw_modifier = raw_modifier.trim();
                if payload_keyword_eq(raw_modifier, "saved", config) {
                    if modifier.replace(FirstprivateModifier::Saved).is_some() {
                        return Err(ConversionError::InvalidClauseSyntax(
                            "duplicate saved firstprivate modifier".to_string(),
                        ));
                    }
                    continue;
                }

                return Err(ConversionError::InvalidClauseSyntax(format!(
                    "unknown firstprivate modifier: {raw_modifier}"
                )));
            }
            items_source
        } else {
            content
        };

    let items = parse_identifier_list(items_source, config)?;
    if items.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "firstprivate clause requires a non-empty variable list".to_string(),
        ));
    }

    Ok(ClauseData::Firstprivate { modifier, items })
}

fn parse_optional_clause_expression(
    clause: &Clause<'_>,
    config: &ParserConfig,
    argument_name: &str,
) -> Result<Option<Expression>, ConversionError> {
    match &clause.kind {
        ClauseKind::Bare => Ok(None),
        ClauseKind::Parenthesized(content) => {
            let content = content.as_ref().trim();
            if content.is_empty() {
                return Err(ConversionError::InvalidClauseSyntax(format!(
                    "{argument_name} requires a non-empty expression"
                )));
            }
            Ok(Some(parse_single_clause_expression(
                content,
                config,
                argument_name,
            )?))
        }
        ClauseKind::FlushMemoryOrderArgument(_) | ClauseKind::ReductionClause { .. } => {
            Err(ConversionError::InvalidClauseSyntax(format!(
                "{argument_name} expects a bare clause or one parenthesized expression"
            )))
        }
    }
}

fn parse_adjust_args_operation(
    source: &str,
    config: &ParserConfig,
) -> Result<AdjustArgsModifier, ConversionError> {
    let source = source.trim();
    for (keyword, operation) in [
        ("nothing", AdjustArgsModifier::Nothing),
        ("need_device_ptr", AdjustArgsModifier::NeedDevicePtr),
        ("need_device_addr", AdjustArgsModifier::NeedDeviceAddr),
    ] {
        let Some(rest) = strip_payload_keyword(source, keyword, config) else {
            continue;
        };
        if skip_host_trivia(rest, config)?.is_empty() {
            return Ok(operation);
        }
    }
    Err(ConversionError::InvalidClauseSyntax(format!(
        "unknown adjust_args operation: {source}"
    )))
}

fn parse_parameter_range_bound(
    source: &str,
    config: &ParserConfig,
) -> Result<Option<Expression>, ConversionError> {
    let source = source.trim();
    if source.is_empty() {
        return Ok(None);
    }
    let expression = Expression::new(source, config)?;
    match obvious_integer_value(&expression) {
        ObviousIntegerValue::NonNegative(0)
        | ObviousIntegerValue::Negative
        | ObviousIntegerValue::NonIntegerLiteral => Err(ConversionError::InvalidClauseSyntax(
            "adjust_args range bounds must be positive constant integer expressions".to_string(),
        )),
        ObviousIntegerValue::Unknown | ObviousIntegerValue::NonNegative(_) => Ok(Some(expression)),
    }
}

fn parse_parameter_list_item(
    source: &str,
    config: &ParserConfig,
) -> Result<OmpParameterListItem, ConversionError> {
    let source = source.trim();
    if source.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "adjust_args parameter list must not contain empty items".to_string(),
        ));
    }

    if let Some((lower, upper)) = lang::split_once_top_level(source, ':')? {
        let lower = parse_parameter_range_bound(lower, config)?;
        let upper = parse_parameter_range_bound(upper, config)?;
        let range = OmpParameterRange::new(lower, upper).ok_or_else(|| {
            ConversionError::InvalidClauseSyntax(
                "adjust_args parameter range must have at least one bound".to_string(),
            )
        })?;
        return Ok(OmpParameterListItem::Range(Box::new(range)));
    }

    let expression = Expression::new(source, config)?;
    if let ExprKind::Literal(Literal::Integer(integer)) = &expression.ast().kind {
        if integer.value == 0 {
            return Err(ConversionError::InvalidClauseSyntax(
                "adjust_args parameter positions are one based".to_string(),
            ));
        }
        let position = u64::try_from(integer.value).map_err(|_| {
            ConversionError::InvalidClauseSyntax(
                "adjust_args parameter position exceeds the supported u64 range".to_string(),
            )
        })?;
        return Ok(OmpParameterListItem::Position(position));
    }

    let mut items = lang::parse_clause_item_list(source, config)?;
    if items.len() != 1 {
        return Err(ConversionError::InvalidClauseSyntax(
            "adjust_args parameter item must name exactly one parameter".to_string(),
        ));
    }
    let item = items.pop().ok_or_else(|| {
        ConversionError::InvalidClauseSyntax(
            "adjust_args parameter item must name exactly one parameter".to_string(),
        )
    })?;
    match item {
        ClauseItem::Identifier(identifier) => Ok(OmpParameterListItem::Named(identifier)),
        ClauseItem::Variable(_) | ClauseItem::FortranCommonBlock(_) | ClauseItem::Expression(_) => {
            Err(ConversionError::InvalidClauseSyntax(
                "adjust_args parameter item must be a parameter name, position, or range"
                    .to_string(),
            ))
        }
    }
}

fn parse_adjust_args_clause(
    kind: &ClauseKind<'_>,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    let ClauseKind::Parenthesized(content) = kind else {
        return Err(ConversionError::InvalidClauseSyntax(
            "adjust_args clause requires parenthesized content".to_string(),
        ));
    };
    let text = content.as_ref().trim();
    let Some((operation, parameter_list)) = lang::split_once_top_level(text, ':')? else {
        return Err(ConversionError::InvalidClauseSyntax(
            "adjust_args requires an adjustment operation followed by ':'".to_string(),
        ));
    };
    let operation = parse_adjust_args_operation(operation, config)?;
    if operation == AdjustArgsModifier::NeedDeviceAddr && config.host_language() == HostLanguage::C
    {
        return Err(ConversionError::InvalidClauseSyntax(
            "need_device_addr is not permitted in a C adjust_args clause".to_string(),
        ));
    }
    let parameters = split_top_level_items(parameter_list)?
        .into_iter()
        .map(|item| parse_parameter_list_item(item, config))
        .collect::<Result<Vec<_>, _>>()?;
    if parameters.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "adjust_args requires a non-empty parameter list".to_string(),
        ));
    }
    Ok(ClauseData::AdjustArgs {
        operation,
        parameters,
    })
}

fn parse_append_args_clause(
    kind: &ClauseKind<'_>,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    let ClauseKind::Parenthesized(content) = kind else {
        return Err(ConversionError::InvalidClauseSyntax(
            "append_args clause requires parenthesized content".to_string(),
        ));
    };
    let operations = split_top_level_items(content.as_ref())?
        .into_iter()
        .map(|source| {
            let source = source.trim();
            let operands = extract_named_call(source, "interop", config)?.ok_or_else(|| {
                ConversionError::InvalidClauseSyntax(format!(
                    "unsupported append_args operation: {source}"
                ))
            })?;
            parse_interop_init_modifiers(operands, config).map(OmpAppendOperation::Interop)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if operations.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "append_args requires at least one OpenMP operation".to_string(),
        ));
    }
    Ok(ClauseData::AppendArgs { operations })
}

fn parse_depend_objects(
    source: &str,
    config: &ParserConfig,
) -> Result<Vec<Variable>, ConversionError> {
    let objects = lang::parse_clause_item_list(source, config)?
        .into_iter()
        .map(|item| {
            let object = match item {
                ClauseItem::Identifier(identifier) => {
                    if payload_keyword_eq(identifier.as_str(), "omp_all_memory", config) {
                        return Err(ConversionError::InvalidClauseSyntax(
                            "omp_all_memory is not a depend object".to_string(),
                        ));
                    }
                    Variable::parse(identifier.as_str(), config)?
                }
                ClauseItem::Variable(variable) => variable,
                ClauseItem::FortranCommonBlock(_) | ClauseItem::Expression(_) => {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "depend(depobj: ...) accepts only depend-object variables".to_string(),
                    ));
                }
            };
            if object.has_array_section() {
                return Err(ConversionError::InvalidClauseSyntax(
                    "array sections are not permitted in depend(depobj: ...)".to_string(),
                ));
            }
            Ok(object)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if objects.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "depend(depobj: ...) requires at least one depend object".to_string(),
        ));
    }
    Ok(objects)
}

fn simple_expression_identifier(expression: &crate::host::Expr) -> Option<&Identifier> {
    match &expression.kind {
        ExprKind::Parenthesized(inner) => simple_expression_identifier(inner),
        ExprKind::Name(crate::host::QualifiedName {
            global: false,
            segments,
        }) if segments.len() == 1 => segments.first(),
        _ => None,
    }
}

fn unparenthesized_expression(mut expression: &crate::host::Expr) -> &crate::host::Expr {
    while let ExprKind::Parenthesized(inner) = &expression.kind {
        expression = inner;
    }
    expression
}

fn parse_doacross_vector_item(
    source: &str,
    config: &ParserConfig,
) -> Result<OmpDoacrossVectorItem, ConversionError> {
    let expression = Expression::new(source.trim(), config)?;
    let root = unparenthesized_expression(expression.ast());
    let (variable, offset) = match &root.kind {
        ExprKind::Name(_) => {
            let variable = simple_expression_identifier(root).ok_or_else(|| {
                ConversionError::InvalidClauseSyntax(
                    "doacross vector entries require one loop-iteration variable".to_string(),
                )
            })?;
            (variable.clone(), None)
        }
        ExprKind::Binary { op, left, right }
            if matches!(op, BinaryOp::Add | BinaryOp::Subtract) =>
        {
            let variable = simple_expression_identifier(left).ok_or_else(|| {
                ConversionError::InvalidClauseSyntax(
                    "doacross vector offsets must follow a loop-iteration variable".to_string(),
                )
            })?;
            let offset_expression = expression.subtree(right);
            match obvious_integer_value(&offset_expression) {
                ObviousIntegerValue::Negative | ObviousIntegerValue::NonIntegerLiteral => {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "doacross vector offsets must be non-negative constant integers"
                            .to_string(),
                    ));
                }
                ObviousIntegerValue::Unknown | ObviousIntegerValue::NonNegative(_) => {}
            }
            let offset = if *op == BinaryOp::Add {
                OmpDoacrossOffset::Add(offset_expression)
            } else {
                OmpDoacrossOffset::Subtract(offset_expression)
            };
            (variable.clone(), Some(offset))
        }
        _ => {
            return Err(ConversionError::InvalidClauseSyntax(
                "doacross vector entries must have the form variable[+/-offset]".to_string(),
            ));
        }
    };
    if payload_keyword_eq(variable.as_str(), "omp_cur_iteration", config) {
        return Err(ConversionError::InvalidClauseSyntax(
            "omp_cur_iteration is not a loop-vector variable".to_string(),
        ));
    }
    Ok(OmpDoacrossVectorItem { variable, offset })
}

fn parse_doacross_iteration(
    kind: DoacrossType,
    source: Option<&str>,
    config: &ParserConfig,
) -> Result<OmpDoacrossIteration, ConversionError> {
    let Some(source) = source else {
        return if kind == DoacrossType::Source {
            Ok(OmpDoacrossIteration::Current)
        } else {
            Err(ConversionError::InvalidClauseSyntax(
                "a sink doacross dependence requires an iteration specifier".to_string(),
            ))
        };
    };
    let source = source.trim();
    if source.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "a doacross ':' must be followed by an iteration specifier".to_string(),
        ));
    }

    let entries = split_top_level_items(source)?;
    if entries.len() == 1 {
        let expression = Expression::new(entries[0].trim(), config)?;
        if simple_expression_identifier(expression.ast())
            .is_some_and(|name| payload_keyword_eq(name.as_str(), "omp_cur_iteration", config))
        {
            return if kind == DoacrossType::Source {
                Ok(OmpDoacrossIteration::Current)
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "sink requires omp_cur_iteration - 1, not omp_cur_iteration".to_string(),
                ))
            };
        }
        if let ExprKind::Binary {
            op: BinaryOp::Subtract,
            left,
            right,
        } = &unparenthesized_expression(expression.ast()).kind
            && simple_expression_identifier(left)
                .is_some_and(|name| payload_keyword_eq(name.as_str(), "omp_cur_iteration", config))
            && matches!(
                obvious_integer_value(&expression.subtree(right)),
                ObviousIntegerValue::NonNegative(1)
            )
        {
            return if kind == DoacrossType::Sink {
                Ok(OmpDoacrossIteration::PreviousCurrent)
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "source may specify only omp_cur_iteration".to_string(),
                ))
            };
        }
    }

    if kind == DoacrossType::Source {
        return Err(ConversionError::InvalidClauseSyntax(
            "source may specify only omp_cur_iteration".to_string(),
        ));
    }
    let vector = entries
        .into_iter()
        .map(|entry| parse_doacross_vector_item(entry, config))
        .collect::<Result<Vec<_>, _>>()?;
    if vector.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "sink requires a non-empty doacross vector".to_string(),
        ));
    }
    Ok(OmpDoacrossIteration::Vector(vector))
}

fn parse_depend_clause(
    kind: &ClauseKind<'_>,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    let ClauseKind::Parenthesized(content) = kind else {
        return Err(ConversionError::InvalidClauseSyntax(
            "depend clause requires parenthesized content".to_string(),
        ));
    };
    let mut remaining = content.as_ref().trim();
    let mut iterators = Vec::new();
    if let Some((iterator_content, rest)) = extract_iterator_block(remaining, config)? {
        iterators = parse_iterator_block(iterator_content, config)?;
        remaining = rest.trim_start();
        let Some(after_comma) = remaining.strip_prefix(',') else {
            return Err(ConversionError::InvalidClauseSyntax(
                "depend iterator modifier must be followed by ','".to_string(),
            ));
        };
        remaining = after_comma.trim_start();
    }

    let split = lang::split_once_top_level(remaining, ':')?;
    let (type_source, arguments) = match split {
        Some((type_source, arguments)) => (type_source.trim(), Some(arguments)),
        None => (remaining.trim(), None),
    };
    let type_keyword = payload_keyword(type_source, config);
    if matches!(type_keyword.as_ref(), "source" | "sink") {
        if !iterators.is_empty() {
            return Err(ConversionError::InvalidClauseSyntax(
                "historical depend(source/sink) does not accept iterator modifiers".to_string(),
            ));
        }
        let kind = if type_keyword == "source" {
            DoacrossType::Source
        } else {
            DoacrossType::Sink
        };
        return Ok(ClauseData::Doacross {
            kind,
            iteration: parse_doacross_iteration(kind, arguments, config)?,
        });
    }

    let arguments = arguments.ok_or_else(|| {
        ConversionError::InvalidClauseSyntax(
            "task dependences require a dependence type followed by ':'".to_string(),
        )
    })?;
    let dependence = if type_keyword == "depobj" {
        OmpDependence::Depobjs {
            objects: parse_depend_objects(arguments, config)?,
        }
    } else {
        let kind = parse_depend_type(type_source, config)?;
        let locators = parse_depend_locator_list(arguments, config)?;
        if locators
            .iter()
            .any(|locator| matches!(locator, OmpLocator::AllMemory))
            && !matches!(kind, DependType::Out | DependType::Inout)
        {
            return Err(ConversionError::InvalidClauseSyntax(
                "omp_all_memory requires an out or inout dependence".to_string(),
            ));
        }
        OmpDependence::Locators { kind, locators }
    };
    Ok(ClauseData::Depend {
        dependence,
        iterators,
    })
}

fn parse_doacross_clause(
    kind: &ClauseKind<'_>,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    let ClauseKind::Parenthesized(content) = kind else {
        return Err(ConversionError::InvalidClauseSyntax(
            "doacross clause requires parenthesized content".to_string(),
        ));
    };
    let inner = content.as_ref().trim();
    let split = lang::split_once_top_level(inner, ':')?;
    let (kind_source, iteration_source) = match split {
        Some((kind_source, iteration_source)) => (kind_source.trim(), Some(iteration_source)),
        None => (inner, None),
    };
    let kind = match payload_keyword(kind_source, config).as_ref() {
        "source" => DoacrossType::Source,
        "sink" => DoacrossType::Sink,
        _ => {
            return Err(ConversionError::InvalidClauseSyntax(format!(
                "unknown doacross dependence type: {kind_source}"
            )));
        }
    };
    Ok(ClauseData::Doacross {
        kind,
        iteration: parse_doacross_iteration(kind, iteration_source, config)?,
    })
}

pub fn parse_clause_data<'a>(
    clause: &'a Clause<'a>,
    directive_kind: OmpDirectiveKind,
    config: &ParserConfig,
    source: &LogicalSource<'_>,
) -> Result<ClauseData, ConversionError> {
    match lookup_clause_name(clause.name.as_ref()) {
        ClauseName::Apply => parse_apply_clause_data(clause, config, source),
        ClauseName::When | ClauseName::Otherwise | ClauseName::Match => {
            parse_metadirective_selector(clause, config, source)
        }
        ClauseName::Default => parse_default_clause_data(clause, config, source),
        _ => parse_nonrecursive_clause_data(clause, directive_kind, config, source),
    }
}

fn parse_apply_clause_data(
    clause: &Clause<'_>,
    config: &ParserConfig,
    source: &LogicalSource<'_>,
) -> Result<ClauseData, ConversionError> {
    let ClauseKind::Parenthesized(content) = &clause.kind else {
        return Err(ConversionError::InvalidClauseSyntax(
            "apply clause requires parenthesized content".to_string(),
        ));
    };
    let (loop_modifier, applied_directives) =
        parse_apply_clause(content.as_ref().trim(), config, source)?;
    Ok(ClauseData::Apply {
        loop_modifier,
        applied_directives,
    })
}

fn parse_default_clause_data(
    clause: &Clause<'_>,
    config: &ParserConfig,
    source: &LogicalSource<'_>,
) -> Result<ClauseData, ConversionError> {
    let ClauseKind::Parenthesized(content) = &clause.kind else {
        return Err(ConversionError::InvalidClauseSyntax(
            "default clause requires parenthesized content".to_string(),
        ));
    };
    let kind_str = content.as_ref().trim();
    let (category, kind_str) =
        if let Some((category, value)) = lang::split_once_top_level(kind_str, ':')? {
            let category = parse_defaultmap_category(category.trim(), config)?;
            if category == DefaultmapCategory::Allocatable
                && !matches!(config.host_language(), HostLanguage::Fortran)
            {
                return Err(ConversionError::InvalidClauseSyntax(
                    "default allocatable category is only valid in Fortran".to_string(),
                ));
            }
            (Some(category), value.trim())
        } else {
            (None, kind_str)
        };
    let kind = match payload_keyword(kind_str, config).as_ref() {
        "shared" => Some(DefaultKind::Shared),
        "none" => Some(DefaultKind::None),
        "private" => Some(DefaultKind::Private),
        "firstprivate" => Some(DefaultKind::Firstprivate),
        _ => None,
    };
    if let Some(kind) = kind {
        return Ok(ClauseData::Default { category, kind });
    }
    if category.is_some() {
        return Err(ConversionError::InvalidClauseSyntax(format!(
            "unknown default data-sharing kind: {kind_str}"
        )));
    }
    let directive = parse_nested_directive(kind_str, config, source)?.ok_or_else(|| {
        ConversionError::InvalidClauseSyntax(format!(
            "Unrecognized default clause content: {kind_str}"
        ))
    })?;
    let selector = OmpSelector::new(Vec::new(), Some(Box::new(directive)))
        .map_err(|error| ConversionError::InvalidClauseSyntax(error.to_string()))?;
    Ok(ClauseData::MetadirectiveSelector {
        selector: Box::new(selector),
    })
}

fn parse_nonrecursive_clause_data<'a>(
    clause: &'a Clause<'a>,
    directive_kind: OmpDirectiveKind,
    config: &ParserConfig,
    source: &LogicalSource<'_>,
) -> Result<ClauseData, ConversionError> {
    let clause_name = clause.name.as_ref();
    let clause_kind = lookup_clause_name(clause_name);

    match clause_kind {
        ClauseName::Nowait => Ok(ClauseData::Nowait {
            do_not_synchronize: parse_optional_clause_expression(
                clause,
                config,
                "do_not_synchronize",
            )?,
        }),
        ClauseName::Nogroup => Ok(ClauseData::Nogroup {
            do_not_synchronize: parse_optional_clause_expression(
                clause,
                config,
                "do_not_synchronize",
            )?,
        }),
        ClauseName::Untied => Ok(ClauseData::Untied {
            can_change_threads: parse_optional_clause_expression(
                clause,
                config,
                "can_change_threads",
            )?,
        }),
        ClauseName::Mergeable => Ok(ClauseData::Mergeable {
            can_merge: parse_optional_clause_expression(clause, config, "can_merge")?,
        }),
        ClauseName::SeqCst
        | ClauseName::Relaxed
        | ClauseName::Release
        | ClauseName::Acquire
        | ClauseName::AcqRel => {
            let order = match clause_kind {
                ClauseName::SeqCst => MemoryOrder::SeqCst,
                ClauseName::Relaxed => MemoryOrder::Relaxed,
                ClauseName::Release => MemoryOrder::Release,
                ClauseName::Acquire => MemoryOrder::Acquire,
                ClauseName::AcqRel => MemoryOrder::AcqRel,
                _ => {
                    return Err(ConversionError::InvalidClauseSyntax(format!(
                        "internal memory-order dispatch mismatch for {clause_name}"
                    )))
                }
            };
            Ok(ClauseData::MemoryOrder {
                order,
                use_semantics: if matches!(
                    &clause.kind,
                    ClauseKind::FlushMemoryOrderArgument(_)
                ) {
                    None
                } else {
                    parse_optional_clause_expression(clause, config, "use_semantics")?
                },
            })
        }

        // Routed through the small public dispatcher so recursively nested
        // defaults never reserve this large nonrecursive match frame.
        ClauseName::Default => parse_default_clause_data(clause, config, source),

        // Metadirective selectors: parse into typed selector data (raw today)
        ClauseName::When | ClauseName::Otherwise | ClauseName::Match => {
            parse_metadirective_selector(clause, config, source)
        }

        // defaultmap(behavior[:category])
        ClauseName::Defaultmap => parse_defaultmap_clause(&clause.kind, config),

        // sizes(list) on tile/stripe directives
        ClauseName::Sizes => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let sizes = split_top_level_items(content.as_ref())?
                    .into_iter()
                    .map(|source| Expression::new(source.trim(), config).map_err(ConversionError::from))
                    .collect::<Result<Vec<_>, _>>()?;
                if sizes.is_empty() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "sizes clause requires at least one expression".to_string(),
                    ));
                }
                for size in &sizes {
                    match obvious_integer_value(size) {
                        ObviousIntegerValue::NonNegative(0)
                        | ObviousIntegerValue::Negative
                        | ObviousIntegerValue::NonIntegerLiteral => {
                            return Err(ConversionError::InvalidClauseSyntax(
                                "sizes entries must be positive integer expressions".to_string(),
                            ));
                        }
                        ObviousIntegerValue::Unknown
                        | ObviousIntegerValue::NonNegative(_) => {}
                    }
                }
                Ok(ClauseData::Sizes { sizes })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "sizes clause requires a parenthesized list".to_string(),
                ))
            }
        }

        // private(list)
        ClauseName::Private => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref();
                let items = parse_identifier_list(content, config)?;
                Ok(ClauseData::Private { items })
            } else {
                Ok(ClauseData::Private { items: vec![] })
            }
        }

        // firstprivate([directive-name,] [saved:] list)
        ClauseName::Firstprivate => parse_firstprivate_clause(clause, config),

        // shared(list)
        ClauseName::Shared => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref();
                let items = parse_identifier_list(content, config)?;
                Ok(ClauseData::Shared { items })
            } else {
                Ok(ClauseData::Shared { items: vec![] })
            }
        }

        // Historical declare-target `to` has enter semantics; target-update
        // data motion uses its own locator/modifier payload.
        ClauseName::To => match &clause.kind {
            ClauseKind::Parenthesized(content)
                if matches!(
                    directive_kind,
                    OmpDirectiveKind::DeclareTarget | OmpDirectiveKind::BeginDeclareTarget
                ) =>
            {
                parse_declare_target_enter_clause(content.as_ref(), config)
            }
            ClauseKind::Parenthesized(content) => {
                parse_data_motion_clause(&ClauseName::To, content.as_ref(), config)
            }
            _ => Err(ConversionError::InvalidClauseSyntax(
                "to clause requires parenthesized content".to_string(),
            )),
        },
        ClauseName::From => match &clause.kind {
            ClauseKind::Parenthesized(content) => {
                parse_data_motion_clause(&ClauseName::From, content.as_ref(), config)
            }
            _ => Err(ConversionError::InvalidClauseSyntax(
                "from clause requires parenthesized content".to_string(),
            )),
        },
        ClauseName::Link => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let items = parse_identifier_list(content.as_ref(), config)?;
                if items.is_empty() {
                    Err(ConversionError::InvalidClauseSyntax(
                        "link clause requires a non-empty variable list".to_string(),
                    ))
                } else {
                    Ok(ClauseData::ItemList(items))
                }
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "link clause requires parenthesized content".to_string(),
                ))
            }
        }

        ClauseName::Enter => match &clause.kind {
            ClauseKind::Parenthesized(content) => {
                parse_declare_target_enter_clause(content.as_ref(), config)
            }
            _ => Err(ConversionError::InvalidClauseSyntax(
                "enter clause requires a parenthesized extended list".to_string(),
            )),
        },

        // interop/local clauses expect an ordinary variable list payload.
        ClauseName::Interop | ClauseName::Local => match &clause.kind {
            ClauseKind::Parenthesized(content) => {
                let items = parse_identifier_list(content.as_ref(), config)?;
                Ok(ClauseData::ItemList(items))
            }
            _ => Err(ConversionError::InvalidClauseSyntax(format!(
                "{clause_name} clause requires a variable list"
            ))),
        },

        // num_threads(expr)
        ClauseName::NumThreads => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref().trim();
                let (strict, values_source) =
                    match lang::split_once_top_level(content, ':')? {
                        Some((modifier, values))
                            if payload_keyword_eq(modifier.trim(), "strict", config) =>
                        {
                            (true, values.trim())
                        }
                        _ => (false, content),
                    };
                let nthreads = split_top_level_items(values_source)?
                    .into_iter()
                    .map(|value| Expression::new(value.trim(), config).map_err(ConversionError::from))
                    .collect::<Result<Vec<_>, _>>()?;
                if nthreads.is_empty() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "num_threads requires at least one thread-count expression".to_string(),
                    ));
                }
                if nthreads.iter().any(|value| {
                    matches!(
                        obvious_integer_value(value),
                        ObviousIntegerValue::NonNegative(0)
                            | ObviousIntegerValue::Negative
                            | ObviousIntegerValue::NonIntegerLiteral
                    )
                }) {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "num_threads entries must be positive integer expressions".to_string(),
                    ));
                }
                Ok(ClauseData::NumThreads {
                    strict,
                    nthreads,
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "num_threads requires expression".to_string(),
                ))
            }
        }

        // if(expr)
        ClauseName::If => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref();
                Ok(ClauseData::If {
                    condition: parse_single_clause_expression(
                        content.trim(),
                        config,
                        "if clause",
                    )?,
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "if clause requires parenthesized content".to_string(),
                ))
            }
        }

        // collapse(n)
        ClauseName::Collapse => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref();
                if content.trim().is_empty() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "collapse requires a non-empty expression".to_string(),
                    ));
                }
                Ok(ClauseData::Collapse {
                    n: parse_single_clause_expression(
                        content.trim(),
                        config,
                        "collapse clause",
                    )?,
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "collapse requires expression".to_string(),
                ))
            }
        }

        // ordered or ordered(n)
        ClauseName::Ordered => match clause.kind {
            ClauseKind::Bare => Ok(ClauseData::Ordered { n: None }),
            ClauseKind::Parenthesized(ref content) => {
                if content.as_ref().trim().is_empty() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "ordered parentheses require a non-empty expression".to_string(),
                    ));
                }
                Ok(ClauseData::Ordered {
                    n: Some(parse_single_clause_expression(
                        content.as_ref().trim(),
                        config,
                        "ordered clause",
                    )?),
                })
            }
            // OpenACC-specific structured clauses should not appear in OpenMP context
            _ => Err(ConversionError::InvalidClauseSyntax(
                "Unexpected structured clause for 'ordered'".to_string(),
            )),
        },

        // reduction(operator: list)
        ClauseName::Reduction => match &clause.kind {
            ClauseKind::ReductionClause {
                directive_name_modifier: _,
                modifiers,
                modifier_items,
                operator,
                user_defined_identifier,
                variables_source,
            } => {
                let operator = convert_parser_reduction_operator(
                    *operator,
                    user_defined_identifier.as_deref(),
                    config,
                )?;
                let items = parse_identifier_list(variables_source.as_ref(), config)?;
                if modifiers.len() != modifier_items.len() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "reduction modifier payload is internally inconsistent".to_string(),
                    ));
                }
                let mapped_modifiers = modifiers
                    .iter()
                    .copied()
                    .zip(modifier_items)
                    .map(|(modifier, arguments)| {
                        convert_reduction_modifier(modifier, arguments, config)
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(ClauseData::Reduction {
                    modifiers: mapped_modifiers,
                    operator,
                    items,
                })
            }
            _ => Err(ConversionError::InvalidClauseSyntax(
                "reduction clause is missing its structured parser payload".to_string(),
            )),
        },

        // schedule([modifier[, modifier]:] kind[, chunk_size])
        ClauseName::Schedule => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref();
                parse_schedule_clause(content, config)
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "schedule clause requires parenthesized content".to_string(),
                ))
            }
        }

        // map([[mapper(mapper-identifier),] map-type:] list)
        ClauseName::Map => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref();
                parse_map_clause(content, config)
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "map clause requires parenthesized content".to_string(),
                ))
            }
        }

        // Historical depend(source/sink) spellings are canonicalized to the
        // same typed doacross payload while retaining private source
        // provenance on the enclosing clause.
        ClauseName::Depend => parse_depend_clause(&clause.kind, config),

        ClauseName::Doacross => parse_doacross_clause(&clause.kind, config),

        // linear([modifier(list):] list[:step])
        ClauseName::Linear => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref();
                parse_linear_clause(content, config)
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "linear clause requires parenthesized content".to_string(),
                ))
            }
        }

        // bind(parallel|teams|thread)
        ClauseName::Bind => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let kind_str = content.as_ref().trim();
                let keyword = payload_keyword(kind_str, config);
                let binding = match keyword.as_ref() {
                    "teams" => BindModifier::Teams,
                    "parallel" => BindModifier::Parallel,
                    "thread" => BindModifier::Thread,
                    _ => {
                        return Err(ConversionError::InvalidClauseSyntax(format!(
                            "Unknown bind kind: {kind_str}"
                        )))
                    }
                };
                Ok(ClauseData::Bind(binding))
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "bind clause requires parenthesized content".to_string(),
                ))
            }
        }

        // proc_bind(master|close|spread|primary)
        ClauseName::ProcBind => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref();
                let kind_str = content.trim();
                let keyword = payload_keyword(kind_str, config);
                let proc_bind = match keyword.as_ref() {
                    "master" => ProcBind::Primary,
                    "close" => ProcBind::Close,
                    "spread" => ProcBind::Spread,
                    "primary" => ProcBind::Primary,
                    _ => {
                        return Err(ConversionError::InvalidClauseSyntax(format!(
                            "Unknown proc_bind kind: {kind_str}"
                        )))
                    }
                };
                Ok(ClauseData::ProcBind(proc_bind))
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "proc_bind clause requires parenthesized content".to_string(),
                ))
            }
        }

        // lastprivate([modifier:] list)
        ClauseName::Lastprivate => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref();
                let (modifier, list_str) =
                    if let Some((modifier, rest)) = lang::split_once_top_level(content, ':')? {
                        (Some(modifier.trim()), rest)
                    } else {
                        (None, content)
                    };

                let modifier = match modifier {
                    Some("") => None,
                    Some(value) if payload_keyword_eq(value, "conditional", config) => {
                        Some(LastprivateModifier::Conditional)
                    }
                    Some(other) => {
                        return Err(ConversionError::InvalidClauseSyntax(format!(
                            "Unknown lastprivate modifier: {other}"
                        )))
                    }
                    None => None,
                };

                let items = parse_identifier_list(list_str.trim(), config)?;
                Ok(ClauseData::Lastprivate { modifier, items })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "lastprivate clause requires parenthesized content".to_string(),
                ))
            }
        }

        // copyin(list)
        ClauseName::CopyIn => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let items = parse_identifier_list(content.as_ref(), config)?;
                Ok(ClauseData::Copyin { items })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "copyin clause requires a variable list".to_string(),
                ))
            }
        }

        // copyprivate(list)
        ClauseName::Copyprivate => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let items = parse_identifier_list(content.as_ref(), config)?;
                Ok(ClauseData::Copyprivate { items })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "copyprivate clause requires a variable list".to_string(),
                ))
            }
        }

        // num_teams(expr)
        ClauseName::NumTeams => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref().trim();
                if content.is_empty() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "num_teams clause requires an expression".to_string(),
                    ));
                }
                let (lower_bound, upper_bound) =
                    match parse_single_clause_expression(content, config, "num_teams clause") {
                        Ok(upper) => (None, upper),
                        Err(single_error) => {
                            let Some((lower, upper)) = lang::split_once_top_level(content, ':')?
                            else {
                                return Err(single_error);
                            };
                            (
                                Some(parse_single_clause_expression(
                                    lower.trim(),
                                    config,
                                    "num_teams lower bound",
                                )?),
                                parse_single_clause_expression(
                                    upper.trim(),
                                    config,
                                    "num_teams upper bound",
                                )?,
                            )
                        }
                    };
                for bound in lower_bound.iter().chain(std::iter::once(&upper_bound)) {
                    if matches!(
                        obvious_integer_value(bound),
                        ObviousIntegerValue::NonNegative(0)
                            | ObviousIntegerValue::Negative
                            | ObviousIntegerValue::NonIntegerLiteral
                    ) {
                        return Err(ConversionError::InvalidClauseSyntax(
                            "num_teams bounds must be positive integer expressions".to_string(),
                        ));
                    }
                }
                if let Some(lower) = lower_bound.as_ref()
                    && let (
                        ObviousIntegerValue::NonNegative(lower),
                        ObviousIntegerValue::NonNegative(upper),
                    ) = (obvious_integer_value(lower), obvious_integer_value(&upper_bound))
                    && lower > upper
                {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "num_teams lower bound must not exceed its upper bound".to_string(),
                    ));
                }
                Ok(ClauseData::NumTeams {
                    lower_bound,
                    upper_bound,
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "num_teams clause requires parenthesized expression".to_string(),
                ))
            }
        }

        // thread_limit(expr)
        ClauseName::ThreadLimit => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                if content.as_ref().trim().is_empty() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "thread_limit clause requires an expression".to_string(),
                    ));
                }
                Ok(ClauseData::ThreadLimit {
                    limit: parse_single_clause_expression(
                        content.as_ref().trim(),
                        config,
                        "thread_limit clause",
                    )?,
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "thread_limit clause requires parenthesized expression".to_string(),
                ))
            }
        }

        // aligned(list[:alignment])
        ClauseName::Aligned => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref();
                let (items_part, alignment_part) =
                    if let Some((items, alignment)) = lang::split_once_top_level(content, ':')? {
                        (items, Some(alignment))
                    } else {
                        (content, None)
                    };

                let items = parse_identifier_list(items_part.trim(), config)?;
                if items.is_empty() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "aligned clause requires at least one variable".to_string(),
                    ));
                }
                let alignment =
                    match alignment_part {
                        Some(value) if !value.trim().is_empty() => {
                            Some(parse_single_clause_expression(
                                value.trim(),
                                config,
                                "aligned alignment",
                            )?)
                        }
                        Some(_) => return Err(ConversionError::InvalidClauseSyntax(
                            "aligned clause requires a non-empty alignment expression after ':'"
                                .to_string(),
                        )),
                        None => None,
                    };
                Ok(ClauseData::Aligned { items, alignment })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "aligned clause requires parenthesized content".to_string(),
                ))
            }
        }

        // safelen(length)
        ClauseName::Safelen => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                if content.as_ref().trim().is_empty() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "safelen clause requires a length expression".to_string(),
                    ));
                }
                Ok(ClauseData::Safelen {
                    length: parse_single_clause_expression(
                        content.as_ref().trim(),
                        config,
                        "safelen clause",
                    )?,
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "safelen clause requires parenthesized expression".to_string(),
                ))
            }
        }

        // simdlen(length)
        ClauseName::Simdlen => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                if content.as_ref().trim().is_empty() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "simdlen clause requires a length expression".to_string(),
                    ));
                }
                Ok(ClauseData::Simdlen {
                    length: parse_single_clause_expression(
                        content.as_ref().trim(),
                        config,
                        "simdlen clause",
                    )?,
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "simdlen clause requires parenthesized expression".to_string(),
                ))
            }
        }

        // in_reduction/task_reduction share the reduction parser
        ClauseName::InReduction | ClauseName::TaskReduction => match &clause.kind {
            ClauseKind::ReductionClause {
                directive_name_modifier: _,
                modifiers,
                operator,
                user_defined_identifier,
                variables_source,
                modifier_items: _,
            } => {
                let operator = convert_parser_reduction_operator(
                    *operator,
                    user_defined_identifier.as_deref(),
                    config,
                )?;
                let items = parse_identifier_list(variables_source.as_ref(), config)?;
                Ok(ClauseData::Reduction {
                    modifiers: modifiers
                        .iter()
                        .copied()
                        .map(|modifier| convert_reduction_modifier(modifier, &[], config))
                        .collect::<Result<Vec<_>, _>>()?,
                    operator,
                    items,
                })
            }
            _ => Err(ConversionError::InvalidClauseSyntax(
                "reduction-style clause is missing its structured parser payload".to_string(),
            )),
        },

        // dist_schedule(kind[, chunk])
        ClauseName::DistSchedule => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let parts = lang::split_top_level(
                    content.as_ref(),
                    ',',
                    &[('(', ')'), ('[', ']'), ('{', '}')],
                )?;
                if parts.len() > 2 {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "dist_schedule accepts only a kind and optional chunk expression"
                            .to_string(),
                    ));
                }
                let raw_kind = parts[0].trim();
                let kind_keyword = payload_keyword(raw_kind, config);
                let kind = match kind_keyword.as_ref() {
                    "static" => ScheduleKind::Static,
                    _ => {
                        return Err(ConversionError::InvalidClauseSyntax(format!(
                            "Unknown dist_schedule kind: {raw_kind}"
                        )))
                    }
                };
                let chunk_size = parts
                    .get(1)
                    .map(|value| Expression::new(value.trim(), config))
                    .transpose()?;
                Ok(ClauseData::DistSchedule { kind, chunk_size })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "dist_schedule clause requires parenthesized content".to_string(),
                ))
            }
        }

        // grainsize(expression)
        ClauseName::Grainsize => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let trimmed = content.as_ref().trim();
                let mut modifier = None;
                let mut expr_text = trimmed;

                if let Some(rest) = strip_payload_keyword(trimmed, "strict", config) {
                    let after = rest.trim_start();
                    if let Some(after_colon) = after.strip_prefix(':') {
                        modifier = Some(GrainsizeModifier::Strict);
                        expr_text = after_colon.trim_start();
                    }
                }

                Ok(ClauseData::Grainsize {
                    modifier,
                    grain: parse_single_clause_expression(
                        expr_text,
                        config,
                        "grainsize clause",
                    )?,
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "grainsize clause requires parenthesized expression".to_string(),
                ))
            }
        }

        // num_tasks(expression)
        ClauseName::NumTasks => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let trimmed = content.as_ref().trim();
                let mut modifier = None;
                let mut expr_text = trimmed;

                if let Some(rest) = strip_payload_keyword(trimmed, "strict", config) {
                    let after = rest.trim_start();
                    if let Some(after_colon) = after.strip_prefix(':') {
                        modifier = Some(NumTasksModifier::Strict);
                        expr_text = after_colon.trim_start();
                    }
                }

                Ok(ClauseData::NumTasks {
                    modifier,
                    num: parse_single_clause_expression(
                        expr_text,
                        config,
                        "num_tasks clause",
                    )?,
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "num_tasks clause requires parenthesized expression".to_string(),
                ))
            }
        }

        ClauseName::AdjustArgs => parse_adjust_args_clause(&clause.kind, config),

        // Parsed separately from adjust_args: append_args carries a list of
        // typed OpenMP operations, never parameter expressions.
        ClauseName::AppendArgs => parse_append_args_clause(&clause.kind, config),

        // apply([loop-modifier:] applied-directive-list)
        ClauseName::Apply => parse_apply_clause_data(clause, config, source),

        // collector(expression)
        ClauseName::Collector => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                if content.as_ref().trim().is_empty() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "collector clause requires an expression".to_string(),
                    ));
                }
                Ok(ClauseData::Collector {
                    expression: parse_single_clause_expression(
                        content.as_ref().trim(),
                        config,
                        "collector clause",
                    )?,
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "collector clause requires parenthesized expression".to_string(),
                ))
            }
        }

        // inductor(expression)
        ClauseName::Inductor => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                if content.as_ref().trim().is_empty() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "inductor clause requires an expression".to_string(),
                    ));
                }
                Ok(ClauseData::Inductor {
                    expression: super::ast_builder::parse_inductor_expression(
                        content.as_ref().trim(),
                        config,
                    )
                    .map_err(|error| {
                        ConversionError::InvalidClauseSyntax(error.to_string())
                    })?,
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "inductor clause requires parenthesized expression".to_string(),
                ))
            }
        }

        // induction([strict|relaxed,] step(expr), identifier: variable-list)
        ClauseName::Induction => parse_induction_clause(&clause.kind, config),

        // filter(expression)
        ClauseName::Filter => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                if content.as_ref().trim().is_empty() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "filter clause requires an expression".to_string(),
                    ));
                }
                Ok(ClauseData::Filter {
                    thread_num: parse_single_clause_expression(
                        content.as_ref().trim(),
                        config,
                        "filter clause",
                    )?,
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "filter clause requires parenthesized expression".to_string(),
                ))
            }
        }

        // affinity([iterator(...),] locator-list)
        ClauseName::Affinity => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref().trim();
                let mut iterators = Vec::new();
                let locator_source = if let Some((iterator_content, rest)) =
                    extract_iterator_block(content, config)?
                {
                    iterators = parse_iterator_block(iterator_content, config)?;
                    if iterators.is_empty() {
                        return Err(ConversionError::InvalidClauseSyntax(
                            "affinity iterator modifier requires an iterator".to_string(),
                        ));
                    }
                    let rest = rest.trim_start();
                    let Some(rest) = rest.strip_prefix(',') else {
                        return Err(ConversionError::InvalidClauseSyntax(
                            "affinity iterator modifier must be followed by a comma".to_string(),
                        ));
                    };
                    rest.trim_start()
                } else {
                    content
                };

                let locators = parse_omp_locator_list(locator_source, config)?;
                Ok(ClauseData::Affinity {
                    iterators,
                    locators,
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "affinity clause requires a locator list".to_string(),
                ))
            }
        }

        // depobj_update(kind)
        ClauseName::DepobjUpdate => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref().trim();
                let (raw_dependence, variable) =
                    match lang::split_once_top_level(content, ':')? {
                        Some((raw_dependence, variable)) => {
                            let raw_dependence = raw_dependence.trim();
                            let variable = variable.trim();
                            if raw_dependence.is_empty() || variable.is_empty() {
                                return Err(ConversionError::InvalidClauseSyntax(
                                    "depobj update dependence type and variable must not be empty"
                                        .to_string(),
                                ));
                            }
                            (
                                raw_dependence,
                                Some(Variable::parse(variable, config)?),
                            )
                        }
                        None => (content, None),
                    };
                let dependence_keyword = payload_keyword(raw_dependence, config);
                let dep = match dependence_keyword.as_ref() {
                    "in" => DepobjUpdateDependence::In,
                    "out" => DepobjUpdateDependence::Out,
                    "inout" => DepobjUpdateDependence::Inout,
                    "inoutset" => DepobjUpdateDependence::Inoutset,
                    "mutexinoutset" => DepobjUpdateDependence::Mutexinoutset,
                    _ => {
                        return Err(ConversionError::InvalidClauseSyntax(format!(
                            "Unknown depobj_update dependence: {raw_dependence}"
                        )))
                    }
                };
                Ok(ClauseData::DepobjUpdate {
                    dependence: dep,
                    variable,
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "depobj_update clause requires parenthesized content".to_string(),
                ))
            }
        }

        // priority(expression)
        ClauseName::Priority => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                if content.as_ref().trim().is_empty() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "priority clause requires an expression".to_string(),
                    ));
                }
                Ok(ClauseData::Priority {
                    priority: parse_single_clause_expression(
                        content.as_ref().trim(),
                        config,
                        "priority clause",
                    )?,
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "priority clause requires parenthesized expression".to_string(),
                ))
            }
        }

        // hint(expression)
        ClauseName::Hint => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                if content.as_ref().trim().is_empty() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "hint clause requires an expression".to_string(),
                    ));
                }
                Ok(ClauseData::Hint {
                    value: parse_single_clause_expression(
                        content.as_ref(),
                        config,
                        "hint clause",
                    )?,
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "hint clause requires parenthesized expression".to_string(),
                ))
            }
        }

        // device(expression)
        ClauseName::Device => parse_device_clause(&clause.kind, config),

        // device_type(host|nohost|any)
        ClauseName::DeviceType => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let value = content.as_ref().trim();
                let keyword = payload_keyword(value, config);
                let device_type = match keyword.as_ref() {
                    "host" => DeviceType::Host,
                    "nohost" => DeviceType::Nohost,
                    "any" => DeviceType::Any,
                    _ => {
                        return Err(ConversionError::InvalidClauseSyntax(format!(
                            "Unknown device_type value: {value}"
                        )))
                    }
                };
                Ok(ClauseData::DeviceType(device_type))
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "device_type clause requires parenthesized value".to_string(),
                ))
            }
        }

        // at(compilation|execution) for error directive
        ClauseName::At => parse_at_clause(&clause.kind, config),

        // severity(fatal|warning) for error directive
        ClauseName::Severity => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let value = content.as_ref().trim();
                let keyword = payload_keyword(value, config);
                let kind = match keyword.as_ref() {
                    "fatal" => SeverityKind::Fatal,
                    "warning" => SeverityKind::Warning,
                    "" => {
                        return Err(ConversionError::InvalidClauseSyntax(
                            "severity clause requires a value".to_string(),
                        ))
                    }
                    _ => {
                        return Err(ConversionError::InvalidClauseSyntax(format!(
                            "Unknown severity value: {value}"
                        )))
                    }
                };
                Ok(ClauseData::Severity(kind))
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "severity clause requires parenthesized value".to_string(),
                ))
            }
        }

        // Typed interop/depobj initialization.
        ClauseName::Init => parse_init_clause(&clause.kind, directive_kind, config),

        // use_device_ptr(list)
        ClauseName::UseDevicePtr => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let items = parse_identifier_list(content.as_ref(), config)?;
                Ok(ClauseData::UseDevicePtr { items })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "use_device_ptr clause requires a variable list".to_string(),
                ))
            }
        }

        // use_device_addr(list)
        ClauseName::UseDeviceAddr => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let items = parse_identifier_list(content.as_ref(), config)?;
                Ok(ClauseData::UseDeviceAddr { items })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "use_device_addr clause requires a variable list".to_string(),
                ))
            }
        }

        // is_device_ptr(list)
        ClauseName::IsDevicePtr => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let items = parse_identifier_list(content.as_ref(), config)?;
                Ok(ClauseData::IsDevicePtr { items })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "is_device_ptr clause requires a variable list".to_string(),
                ))
            }
        }

        // has_device_addr(list)
        ClauseName::HasDeviceAddr => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let items = parse_identifier_list(content.as_ref(), config)?;
                Ok(ClauseData::HasDeviceAddr { items })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "has_device_addr clause requires a variable list".to_string(),
                ))
            }
        }

        // allocate([allocator-expression:] list) or
        // allocate([allocator(expr),] [align(expr):] list)
        ClauseName::Allocate => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                parse_allocate_clause(content.as_ref(), config)
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "allocate clause requires parenthesized content".to_string(),
                ))
            }
        }

        // allocator(allocator-handle)
        ClauseName::Allocator => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref().trim();
                if content.is_empty() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "allocator clause requires an allocator handle".to_string(),
                    ));
                }
                Ok(ClauseData::Allocator {
                    allocator: parse_single_clause_expression(
                        content,
                        config,
                        "allocator clause",
                    )?,
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "allocator clause requires parenthesized content".to_string(),
                ))
            }
        }

        // order(concurrent)
        ClauseName::Order => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let value = content.as_ref().trim();
                let (modifier, kind_str) =
                    if let Some((modifier_str, rest)) = lang::split_once_top_level(value, ':')? {
                        let raw_modifier = modifier_str.trim();
                        let modifier_keyword = payload_keyword(raw_modifier, config);
                        let modifier = match modifier_keyword.as_ref() {
                            "reproducible" => Some(OrderModifier::Reproducible),
                            "unconstrained" => Some(OrderModifier::Unconstrained),
                            _ => {
                                return Err(ConversionError::InvalidClauseSyntax(format!(
                                    "Unknown order modifier: {raw_modifier}"
                                )))
                            }
                        };
                        (modifier, rest.trim())
                    } else {
                        (None, value)
                    };

                let kind_keyword = payload_keyword(kind_str, config);
                match kind_keyword.as_ref() {
                    "concurrent" => Ok(ClauseData::Order {
                        modifier,
                        kind: OrderKind::Concurrent,
                    }),
                    _ => Err(ConversionError::InvalidClauseSyntax(format!(
                        "Unknown order value: {kind_str}"
                    ))),
                }
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "order clause requires parenthesized value".to_string(),
                ))
            }
        }

        // atomic_default_mem_order(seq_cst|acq_rel|...)
        ClauseName::AtomicDefaultMemOrder => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let raw_order = content.as_ref().trim();
                let order_keyword = payload_keyword(raw_order, config);
                let order = match order_keyword.as_ref() {
                    "seq_cst" => MemoryOrder::SeqCst,
                    "acq_rel" => MemoryOrder::AcqRel,
                    "release" => MemoryOrder::Release,
                    "acquire" => MemoryOrder::Acquire,
                    "relaxed" => MemoryOrder::Relaxed,
                    _ => {
                        return Err(ConversionError::InvalidClauseSyntax(format!(
                            "Unknown atomic default memory order: {raw_order}"
                        )))
                    }
                };
                Ok(ClauseData::Requirement {
                    requirement: RequireModifier::AtomicDefaultMemOrder(order),
                    required: None,
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "atomic_default_mem_order clause requires parenthesized value".to_string(),
                ))
            }
        }

        // Mutually exclusive atomic operation clauses.
        ClauseName::Read | ClauseName::Write | ClauseName::Update => {
            let op = match clause_kind {
                ClauseName::Read => AtomicOp::Read,
                ClauseName::Write => AtomicOp::Write,
                ClauseName::Update => AtomicOp::Update,
                _ => {
                    return Err(ConversionError::InvalidClauseSyntax(format!(
                        "internal atomic-operation dispatch mismatch for {clause_name}"
                    )))
                }
            };
            Ok(ClauseData::AtomicOperation {
                op,
                use_semantics: parse_optional_clause_expression(
                    clause,
                    config,
                    "use_semantics",
                )?,
            })
        }

        // branch hints and SIMD modifiers
        ClauseName::Nontemporal => match &clause.kind {
            ClauseKind::Parenthesized(content) => {
                let items = parse_identifier_list(content.as_ref(), config)?;
                Ok(ClauseData::ItemList(items))
            }
            ClauseKind::Bare => Err(ConversionError::InvalidClauseSyntax(format!(
                "{clause_name} clause requires a non-empty variable list"
            ))),
            _ => Err(ConversionError::InvalidClauseSyntax(format!(
                "{clause_name} clause requires a variable list"
            ))),
        },
        ClauseName::Uniform => match &clause.kind {
            ClauseKind::Parenthesized(content) => {
                let items = parse_identifier_list(content.as_ref(), config)?;
                let parameters = items
                    .into_iter()
                    .map(|item| match item {
                        ClauseItem::Identifier(identifier) => Ok(identifier),
                        other => Err(ConversionError::InvalidClauseSyntax(format!(
                            "uniform accepts only named parameters, not `{other}`"
                        ))),
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                if parameters.is_empty() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "uniform requires at least one named parameter".to_string(),
                    ));
                }
                Ok(ClauseData::Uniform { parameters })
            }
            _ => Err(ConversionError::InvalidClauseSyntax(
                "uniform requires a parenthesized named-parameter list".to_string(),
            )),
        },
        ClauseName::Inbranch | ClauseName::Notinbranch => Ok(ClauseData::Branch {
            condition: parse_optional_clause_expression(clause, config, clause_name)?,
        }),
        ClauseName::Inclusive => parse_scan_clause(ScanClauseMode::Inclusive, clause, config),
        ClauseName::Exclusive => parse_scan_clause(ScanClauseMode::Exclusive, clause, config),

        // uses_allocators(allocator[(traits)], ...)
        ClauseName::UsesAllocators => parse_uses_allocators_clause(&clause.kind, config),

        // fail(memory-order) for atomic compare fail
        ClauseName::Fail => {
            let order = match &clause.kind {
                ClauseKind::Parenthesized(content) => {
                    let trimmed = content.as_ref().trim();
                    if trimmed.is_empty() {
                        MemoryOrder::SeqCst
                    } else {
                        parse_memory_order(trimmed, config)?
                    }
                }
                ClauseKind::Bare => MemoryOrder::SeqCst,
                _ => {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "fail clause expects an optional memory order".to_string(),
                    ))
                }
            };
            Ok(ClauseData::Fail { order })
        }

        // assume/assumes clauses: absent(directive-name-list) / contains(directive-name-list)
        clause_kind @ (ClauseName::Absent | ClauseName::Contains) => {
            let content = match &clause.kind {
                ClauseKind::Parenthesized(content) => content.as_ref(),
                _ => {
                    return Err(ConversionError::InvalidClauseSyntax(format!(
                        "{clause_name} clause requires parenthesized directive-name list"
                    )))
                }
            };
            let directives = split_top_level_items(content)?
                .into_iter()
                .map(str::trim)
                .map(|token| {
                    OmpDirectiveKind::try_from(lookup_payload_directive_name(token, config))
                        .map_err(|_| {
                        ConversionError::InvalidClauseSyntax(format!(
                            "unknown directive name in {clause_name} clause: {token}"
                        ))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;

            match clause_kind {
                ClauseName::Absent => Ok(ClauseData::Absent { directives }),
                ClauseName::Contains => Ok(ClauseData::Contains { directives }),
                _ => Err(ConversionError::InvalidClauseSyntax(format!(
                    "internal clause dispatch mismatch for {clause_name}"
                ))),
            }
        }

        ClauseName::Requires => Err(ConversionError::InvalidClauseSyntax(
            "requires is a directive; its requirements are separate clauses".to_string(),
        )),

        ClauseName::ReverseOffload
        | ClauseName::UnifiedAddress
        | ClauseName::UnifiedSharedMemory
        | ClauseName::DynamicAllocators
        | ClauseName::SelfMaps
        | ClauseName::DeviceSafesync => {
            let requirement = match clause_kind {
                ClauseName::ReverseOffload => RequireModifier::ReverseOffload,
                ClauseName::UnifiedAddress => RequireModifier::UnifiedAddress,
                ClauseName::UnifiedSharedMemory => RequireModifier::UnifiedSharedMemory,
                ClauseName::DynamicAllocators => RequireModifier::DynamicAllocators,
                ClauseName::SelfMaps => RequireModifier::SelfMaps,
                ClauseName::DeviceSafesync => RequireModifier::DeviceSafesync,
                _ => {
                    return Err(ConversionError::InvalidClauseSyntax(format!(
                        "internal requirement dispatch mismatch for {clause_name}"
                    )))
                }
            };
            Ok(ClauseData::Requirement {
                requirement,
                required: parse_optional_clause_expression(clause, config, "required")?,
            })
        }

        ClauseName::ExtImplementationDefinedRequirement => match &clause.kind {
            ClauseKind::Bare => Ok(ClauseData::Requirement {
                requirement: RequireModifier::ExtImplementationDefinedRequirement(None),
                required: None,
            }),
            ClauseKind::Parenthesized(content) => {
                let value = content.as_ref().trim();
                if value.is_empty() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "ext_implementation_defined_requirement parentheses require a value"
                            .to_string(),
                    ));
                }
                Ok(ClauseData::Requirement {
                    requirement: RequireModifier::ExtImplementationDefinedRequirement(Some(
                        Identifier::new(value)?,
                    )),
                    required: None,
                })
            }
            _ => Err(ConversionError::InvalidClauseSyntax(
                "ext_implementation_defined_requirement clause expects bare or one parenthesized value"
                    .to_string(),
            )),
        },

        ClauseName::Capture | ClauseName::Compare | ClauseName::Weak => Ok(ClauseData::ExtendedAtomic {
            kind: match clause_kind {
                ClauseName::Capture => ExtendedAtomicKind::Capture,
                ClauseName::Compare => ExtendedAtomicKind::Compare,
                ClauseName::Weak => ExtendedAtomicKind::Weak,
                _ => unreachable!("closed extended-atomic dispatch"),
            },
            use_semantics: parse_optional_clause_expression(clause, config, "use_semantics")?,
        }),
        ClauseName::Full => Ok(ClauseData::Full {
            fully_unroll: parse_optional_clause_expression(clause, config, "fully_unroll")?,
        }),
        ClauseName::Threads => Ok(ClauseData::Threads {
            apply_to_threads: parse_optional_clause_expression(
                clause,
                config,
                "apply_to_threads",
            )?,
        }),
        ClauseName::Simd => Ok(ClauseData::Simd {
            apply_to_simd: parse_optional_clause_expression(clause, config, "apply_to_simd")?,
        }),
        ClauseName::NoParallelism => Ok(ClauseData::Assumption {
            can_assume: parse_optional_clause_expression(clause, config, "can_assume")?,
        }),

        ClauseName::InitComplete => match &clause.kind {
            ClauseKind::Bare => Ok(ClauseData::InitComplete {
                create_init_phase: None,
            }),
            ClauseKind::Parenthesized(content) => {
                let content = content.as_ref().trim();
                if content.is_empty() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "init_complete parentheses require a logical expression".to_string(),
                    ));
                }
                Ok(ClauseData::InitComplete {
                    create_init_phase: Some(parse_single_clause_expression(
                        content,
                        config,
                        "init_complete clause",
                    )?),
                })
            }
            _ => Err(ConversionError::InvalidClauseSyntax(
                "init_complete accepts an optional parenthesized logical expression".to_string(),
            )),
        },

        ClauseName::Partial => Ok(ClauseData::Partial {
            unroll_factor: parse_optional_clause_expression(clause, config, "unroll_factor")?,
        }),
        ClauseName::Replayable => Ok(ClauseData::Replayable {
            replayable_expression: parse_optional_clause_expression(
                clause,
                config,
                "replayable_expression",
            )?,
        }),
        ClauseName::Indirect => Ok(ClauseData::Indirect {
            invoked_by_fptr: parse_optional_clause_expression(
                clause,
                config,
                "invoked_by_fptr",
            )?,
        }),
        ClauseName::Safesync => Ok(ClauseData::Safesync {
            width: parse_optional_clause_expression(clause, config, "width")?,
        }),
        ClauseName::Transparent => Ok(ClauseData::Transparent {
            impex_type: parse_optional_clause_expression(clause, config, "impex_type")?,
        }),
        ClauseName::NoOpenmp
        | ClauseName::NoOpenmpConstructs
        | ClauseName::NoOpenmpRoutines => Ok(ClauseData::Assumption {
            can_assume: parse_optional_clause_expression(clause, config, "can_assume")?,
        }),

        ClauseName::Threadset => {
            let ClauseKind::Parenthesized(content) = &clause.kind else {
                return Err(ConversionError::InvalidClauseSyntax(
                    "threadset requires exactly one parenthesized set keyword".to_string(),
                ));
            };
            let value = payload_keyword(content.as_ref().trim(), config);
            Ok(ClauseData::Threadset(match value.as_ref() {
                "omp_pool" => ThreadsetKind::OmpPool,
                "omp_team" => ThreadsetKind::OmpTeam,
                _ => {
                    return Err(ConversionError::InvalidClauseSyntax(format!(
                        "unknown threadset kind: {}",
                        content.as_ref().trim()
                    )))
                }
            }))
        }
        ClauseName::Memscope => {
            let ClauseKind::Parenthesized(content) = &clause.kind else {
                return Err(ConversionError::InvalidClauseSyntax(
                    "memscope requires exactly one parenthesized scope keyword".to_string(),
                ));
            };
            let value = payload_keyword(content.as_ref().trim(), config);
            Ok(ClauseData::Memscope(match value.as_ref() {
                "all" => MemscopeKind::All,
                "cgroup" => MemscopeKind::Cgroup,
                "device" => MemscopeKind::Device,
                _ => {
                    return Err(ConversionError::InvalidClauseSyntax(format!(
                        "unknown memscope kind: {}",
                        content.as_ref().trim()
                    )))
                }
            }))
        }
        ClauseName::Looprange => {
            let ClauseKind::Parenthesized(content) = &clause.kind else {
                return Err(ConversionError::InvalidClauseSyntax(
                    "looprange requires exactly two parenthesized expressions".to_string(),
                ));
            };
            let expressions = split_top_level_items(content.as_ref())?;
            let [first, count] = expressions.as_slice() else {
                return Err(ConversionError::InvalidClauseSyntax(
                    "looprange requires exactly two expressions: first, count".to_string(),
                ));
            };
            Ok(ClauseData::Looprange {
                first: Expression::new(first.trim(), config)?,
                count: Expression::new(count.trim(), config)?,
            })
        }
        ClauseName::GraphReset => match &clause.kind {
            ClauseKind::Bare => Ok(ClauseData::GraphReset { condition: None }),
            ClauseKind::Parenthesized(content) => {
                let condition = content.as_ref().trim();
                if condition.is_empty() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "graph_reset parentheses require a condition".to_string(),
                    ));
                }
                Ok(ClauseData::GraphReset {
                    condition: Some(parse_single_clause_expression(
                        condition,
                        config,
                        "graph_reset clause",
                    )?),
                })
            }
            _ => Err(ConversionError::InvalidClauseSyntax(
                "graph_reset expects a bare clause or one condition".to_string(),
            )),
        },

        ClauseName::Align => match &clause.kind {
            ClauseKind::Parenthesized(content) => {
                let alignment = parse_single_clause_expression(
                    content.as_ref(),
                    config,
                    "align clause",
                )?;
                match obvious_integer_value(&alignment) {
                    ObviousIntegerValue::NonNegative(value)
                        if value.is_power_of_two() && value != 0 => {}
                    ObviousIntegerValue::Unknown => {}
                    _ => {
                        return Err(ConversionError::InvalidClauseSyntax(
                            "align requires a positive integer power-of-two expression"
                                .to_string(),
                        ))
                    }
                }
                Ok(ClauseData::Align { alignment })
            }
            _ => Err(ConversionError::InvalidClauseSyntax(
                "align requires one parenthesized expression".to_string(),
            )),
        },
        ClauseName::Destroy => match &clause.kind {
            ClauseKind::Bare => Ok(ClauseData::Destroy { variable: None }),
            ClauseKind::Parenthesized(content) => Ok(ClauseData::Destroy {
                variable: Some(Variable::parse(content.as_ref().trim(), config)?),
            }),
            _ => Err(ConversionError::InvalidClauseSyntax(
                "destroy expects a bare clause or one variable".to_string(),
            )),
        },
        ClauseName::Final => match &clause.kind {
            ClauseKind::Parenthesized(content) => Ok(ClauseData::Final {
                condition: parse_single_clause_expression(
                    content.as_ref(),
                    config,
                    "final clause",
                )?,
            }),
            _ => Err(ConversionError::InvalidClauseSyntax(
                "final requires one parenthesized expression".to_string(),
            )),
        },
        ClauseName::Message => match &clause.kind {
            ClauseKind::Parenthesized(content) => {
                let value = parse_single_clause_expression(
                    content.as_ref(),
                    config,
                    "message clause",
                )?;
                if is_obviously_non_string(value.ast()) {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "message requires an expression of string OpenMP type".to_string(),
                    ));
                }
                Ok(ClauseData::Message { value })
            }
            _ => Err(ConversionError::InvalidClauseSyntax(
                "message requires one parenthesized string expression".to_string(),
            )),
        },
        ClauseName::Holds => match &clause.kind {
            ClauseKind::Parenthesized(content) => Ok(ClauseData::Holds {
                condition: parse_single_clause_expression(
                    content.as_ref(),
                    config,
                    "holds clause",
                )?,
            }),
            _ => Err(ConversionError::InvalidClauseSyntax(
                "holds requires one parenthesized expression".to_string(),
            )),
        },
        ClauseName::GraphId => match &clause.kind {
            ClauseKind::Parenthesized(content) => {
                let value = parse_single_clause_expression(
                    content.as_ref(),
                    config,
                    "graph_id clause",
                )?;
                if matches!(
                    obvious_integer_value(&value),
                    ObviousIntegerValue::NonIntegerLiteral
                ) {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "graph_id requires an integer expression".to_string(),
                    ));
                }
                Ok(ClauseData::GraphId { value })
            }
            _ => Err(ConversionError::InvalidClauseSyntax(
                "graph_id requires one parenthesized expression".to_string(),
            )),
        },
        ClauseName::Nocontext => match &clause.kind {
            ClauseKind::Parenthesized(content) => Ok(ClauseData::Nocontext {
                condition: parse_single_clause_expression(
                    content.as_ref(),
                    config,
                    "nocontext clause",
                )?,
            }),
            _ => Err(ConversionError::InvalidClauseSyntax(
                "nocontext requires one parenthesized expression".to_string(),
            )),
        },
        ClauseName::Novariants => match &clause.kind {
            ClauseKind::Parenthesized(content) => Ok(ClauseData::Novariants {
                condition: parse_single_clause_expression(
                    content.as_ref(),
                    config,
                    "novariants clause",
                )?,
            }),
            _ => Err(ConversionError::InvalidClauseSyntax(
                "novariants requires one parenthesized expression".to_string(),
            )),
        },

        ClauseName::Permutation => match &clause.kind {
            ClauseKind::Parenthesized(content) => {
                let positions = split_top_level_items(content.as_ref())?
                    .into_iter()
                    .map(|source| {
                        Expression::new(source.trim(), config).map_err(ConversionError::from)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                if positions.len() < 2 {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "permutation requires at least two entries".to_string(),
                    ));
                }
                let mut literal_values = HashSet::new();
                for position in &positions {
                    match obvious_integer_value(position) {
                        ObviousIntegerValue::NonNegative(value)
                            if value > 0 && value <= positions.len() as u128 =>
                        {
                            if !literal_values.insert(value) {
                                return Err(ConversionError::InvalidClauseSyntax(
                                    "permutation contains a duplicate literal position"
                                        .to_string(),
                                ));
                            }
                        }
                        ObviousIntegerValue::Unknown => {}
                        _ => {
                            return Err(ConversionError::InvalidClauseSyntax(
                                "permutation entries must be positive integer constants in 1..=n"
                                    .to_string(),
                            ))
                        }
                    }
                }
                Ok(ClauseData::Permutation { positions })
            }
            _ => Err(ConversionError::InvalidClauseSyntax(
                "permutation requires a parenthesized expression list".to_string(),
            )),
        },
        ClauseName::Counts => match &clause.kind {
            ClauseKind::Parenthesized(content) => {
                let mut fill_count = 0usize;
                let counts = split_top_level_items(content.as_ref())?
                    .into_iter()
                    .map(|source| {
                        let source = source.trim();
                        if payload_keyword_eq(source, "omp_fill", config) {
                            fill_count += 1;
                            return Ok(OmpCount::Fill);
                        }
                        let expression = Expression::new(source, config)?;
                        match obvious_integer_value(&expression) {
                            ObviousIntegerValue::Negative
                            | ObviousIntegerValue::NonIntegerLiteral => {
                                Err(ConversionError::InvalidClauseSyntax(
                                    "counts entries must be non-negative integer constants or omp_fill"
                                        .to_string(),
                                ))
                            }
                            ObviousIntegerValue::Unknown
                            | ObviousIntegerValue::NonNegative(_) => {
                                Ok(OmpCount::Expression(expression))
                            }
                        }
                    })
                    .collect::<Result<Vec<_>, ConversionError>>()?;
                if fill_count != 1 {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "counts requires exactly one omp_fill entry".to_string(),
                    ));
                }
                Ok(ClauseData::Counts { counts })
            }
            _ => Err(ConversionError::InvalidClauseSyntax(
                "counts requires a parenthesized count list".to_string(),
            )),
        },

        ClauseName::Detach => match &clause.kind {
            ClauseKind::Parenthesized(content) => {
                let content = content.as_ref().trim();
                if content.is_empty() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "detach requires one event-handle designator".to_string(),
                    ));
                }
                Ok(ClauseData::Detach {
                    event: Variable::parse(content, config)?,
                })
            }
            _ => Err(ConversionError::InvalidClauseSyntax(
                "detach requires one parenthesized event-handle designator".to_string(),
            )),
        },

        ClauseName::Use => match &clause.kind {
            ClauseKind::Parenthesized(content) => Ok(ClauseData::Use {
                interop_var: Variable::parse(content.as_ref().trim(), config)?,
            }),
            ClauseKind::Bare => Err(ConversionError::InvalidClauseSyntax(
                "use clause requires exactly one interop variable".to_string(),
            )),
            _ => Err(ConversionError::InvalidClauseSyntax(
                "use clause expects exactly one variable".to_string(),
            )),
        },

        _ => Err(ConversionError::UnknownClause(format!(
            "{clause_name:?} ({:?})",
            clause.kind
        ))),
    }
}
