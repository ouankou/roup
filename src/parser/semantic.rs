//! Parser-boundary semantic payload parsers.
//!
//! This module is the only runtime boundary that may interpret directive and
//! clause payload text as OpenMP/OpenACC keywords. IR conversion code consumes
//! the typed results from here instead of branching on raw strings.

use std::collections::HashSet;

use crate::ast::{
    OmpClauseKind, OmpDirective, OmpDirectiveKind, OmpSelector, OmpSelectorConstruct,
    OmpSelectorConstructs, OmpSelectorDevice, OmpSelectorImpl, OmpSelectorScoredValue,
    OmpSelectorUser,
};
use crate::ir::{
    lang, AdjustArgsModifier, AffinityModifier, ApplyTransform, ApplyTransformKind, AtKind,
    AtomicOp, BindModifier, ClauseData, ClauseItem, ConversionError, DefaultKind,
    DefaultmapBehavior, DefaultmapCategory, DependIterator, DependType, DepobjUpdateDependence,
    DeviceModifier, DeviceType, DoacrossType, Expression, GrainsizeModifier, Identifier,
    IfModifier, InductionItem, InitKind, Language, LastprivateModifier, LinearModifier,
    MapModifier, MapType, MemoryOrder, NowaitModifier, NumTasksModifier, OrderKind, OrderModifier,
    ParserConfig, ProcBind, ReductionModifier, ReductionOperator, RequireModifier, ScanClauseMode,
    ScheduleKind, ScheduleModifier, SeverityKind, UsesAllocatorBuiltin, UsesAllocatorKind,
    UsesAllocatorSpec,
};
use crate::lexer::Language as LexerLanguage;
use crate::parser::clause::lookup_clause_name;
use crate::parser::clause::ReductionOperator as ParserReductionOperator;
use crate::parser::directive_kind::{lookup_directive_name, DirectiveName};
use crate::parser::{Clause, ClauseKind, ClauseName};

fn parse_identifier_list(
    content: &str,
    config: &ParserConfig,
) -> Result<Vec<ClauseItem>, ConversionError> {
    lang::parse_clause_item_list(content, config)
}

/// Parse a reduction operator from a string
///
/// ## Example
///
/// ```
/// # use roup::ir::{convert::parse_reduction_operator, ReductionOperator};
/// let op = parse_reduction_operator("+").unwrap();
/// assert_eq!(op, ReductionOperator::Add);
///
/// let op = parse_reduction_operator("min").unwrap();
/// assert_eq!(op, ReductionOperator::Min);
/// ```
pub fn parse_reduction_operator(op_str: &str) -> Result<ReductionOperator, ConversionError> {
    match op_str {
        "+" => Ok(ReductionOperator::Add),
        "-" => Ok(ReductionOperator::Subtract),
        "*" => Ok(ReductionOperator::Multiply),
        ".and." => Ok(ReductionOperator::LogicalAnd),
        ".or." => Ok(ReductionOperator::LogicalOr),
        "iand" => Ok(ReductionOperator::BitwiseAnd),
        "ior" => Ok(ReductionOperator::BitwiseOr),
        "ieor" => Ok(ReductionOperator::BitwiseXor),
        "&" => Ok(ReductionOperator::BitwiseAnd),
        "|" => Ok(ReductionOperator::BitwiseOr),
        "^" => Ok(ReductionOperator::BitwiseXor),
        "&&" => Ok(ReductionOperator::LogicalAnd),
        "||" => Ok(ReductionOperator::LogicalOr),
        "min" => Ok(ReductionOperator::Min),
        "max" => Ok(ReductionOperator::Max),
        _ => Ok(ReductionOperator::Custom),
    }
}

/// Parse a schedule clause
///
/// Format: `schedule([modifier[, modifier]:] kind[, chunk_size])`
///
/// ## Example
///
/// ```
/// # use roup::ir::{convert::parse_schedule_clause, ParserConfig, Language};
/// let config = ParserConfig::with_parsing(Language::C);
/// let clause = parse_schedule_clause("static, 10", &config).unwrap();
/// // Returns ClauseData::Schedule with kind=Static, chunk_size=Some(10)
/// ```
pub fn parse_schedule_clause(
    content: &str,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    // Check for modifiers (they end with a colon)
    let (modifiers, rest) = if let Some(colon_pos) = content.find(':') {
        let (mod_str, kind_str) = content.split_at(colon_pos);
        let kind_str = kind_str[1..].trim(); // Skip the ':'

        let mut seen_modifiers: HashSet<ScheduleModifier> = HashSet::new();
        let mut mods: Vec<ScheduleModifier> = Vec::new();

        for raw in mod_str
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            let modifier = match raw.to_ascii_lowercase().as_str() {
                "monotonic" => ScheduleModifier::Monotonic,
                "nonmonotonic" => ScheduleModifier::Nonmonotonic,
                "simd" => ScheduleModifier::Simd,
                _ => {
                    return Err(ConversionError::InvalidClauseSyntax(format!(
                        "Unknown schedule modifier: {raw}"
                    )))
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

        (mods, kind_str)
    } else {
        (vec![], content)
    };

    // Parse kind and optional chunk size (comma-separated)
    let parts: Vec<&str> = rest.split(',').map(|s| s.trim()).collect();

    let kind_token = parts.first().map(|s| s.to_ascii_lowercase());

    let kind = match kind_token.as_deref() {
        Some("static") => ScheduleKind::Static,
        Some("dynamic") => ScheduleKind::Dynamic,
        Some("guided") => ScheduleKind::Guided,
        Some("auto") => ScheduleKind::Auto,
        Some("runtime") => ScheduleKind::Runtime,
        Some(_) => {
            let s = parts.first().copied().unwrap_or_default();
            return Err(ConversionError::InvalidClauseSyntax(format!(
                "Unknown schedule kind: {s}"
            )));
        }
        None => {
            return Err(ConversionError::InvalidClauseSyntax(
                "schedule clause requires a kind".to_string(),
            ))
        }
    };

    let chunk_size = parts.get(1).map(|s| Expression::new(*s, config));

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
/// ```
/// # use roup::ir::{convert::parse_map_clause, ParserConfig, Language};
/// let config = ParserConfig::with_parsing(Language::C);
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
    let mut remainder = content.trim();
    let mut mapper = None;
    let mut modifiers = Vec::new();
    let mut iterators = Vec::new();

    if let Some((iterator_content, rest)) = extract_iterator_block(remainder) {
        iterators = parse_iterator_block(&iterator_content, config)?;
        remainder = rest.trim_start();
        if remainder.starts_with(',') {
            remainder = remainder[1..].trim_start();
        }
        modifiers.push(MapModifier::Iterator);
    }

    // Check for mapper(...) prefix
    if remainder.len() >= 6 && remainder[..6].eq_ignore_ascii_case("mapper") {
        let after_keyword = remainder[6..].trim_start();
        if after_keyword.starts_with('(') {
            // Extract mapper identifier
            let (mapper_body, rest) = extract_parenthesized(after_keyword)?;
            mapper = Some(Identifier::new(mapper_body.trim()));
            remainder = rest.trim_start();

            // Skip optional comma
            if remainder.starts_with(',') {
                remainder = remainder[1..].trim_start();
            }
        }
    }

    // Find map-type using top-level colon detection
    let (map_type, items_str) =
        if let Some((type_str, items)) = lang::split_once_top_level(remainder, ':') {
            let mut map_type = None;
            let tokens = type_str
                .split(',')
                .map(|t| t.trim())
                .filter(|t| !t.is_empty());
            for token in tokens {
                match token.to_ascii_lowercase().as_str() {
                    "to" => map_type = Some(MapType::To),
                    "from" => map_type = Some(MapType::From),
                    "tofrom" => map_type = Some(MapType::ToFrom),
                    "alloc" => map_type = Some(MapType::Alloc),
                    "release" => map_type = Some(MapType::Release),
                    "delete" => map_type = Some(MapType::Delete),
                    "always" => modifiers.push(MapModifier::Always),
                    "close" => modifiers.push(MapModifier::Close),
                    "present" => modifiers.push(MapModifier::Present),
                    "self" => modifiers.push(MapModifier::SelfMap),
                    "iterator" => modifiers.push(MapModifier::Iterator),
                    "ompx_hold" => modifiers.push(MapModifier::OmpxHold),
                    other => {
                        return Err(ConversionError::InvalidClauseSyntax(format!(
                            "Unknown map modifier or type: {other}"
                        )))
                    }
                }
            }
            (map_type, items.trim())
        } else {
            (None, remainder)
        };

    let items = parse_identifier_list(items_str, config)?;

    Ok(ClauseData::Map {
        map_type,
        modifiers,
        mapper,
        iterators,
        items,
    })
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
/// ```
/// # use roup::ir::{convert::parse_depend_type, DependType};
/// let dt = parse_depend_type("in").unwrap();
/// assert_eq!(dt, DependType::In);
/// ```
pub fn parse_depend_type(type_str: &str) -> Result<DependType, ConversionError> {
    match type_str.trim().to_ascii_lowercase().as_str() {
        "in" => Ok(DependType::In),
        "out" => Ok(DependType::Out),
        "inout" => Ok(DependType::Inout),
        "inoutset" => Ok(DependType::Inoutset),
        "mutexinoutset" => Ok(DependType::Mutexinoutset),
        "depobj" => Ok(DependType::Depobj),
        "source" => Ok(DependType::Source),
        "sink" => Ok(DependType::Sink),
        _ => Err(ConversionError::InvalidClauseSyntax(format!(
            "Unknown depend type: {type_str}"
        ))),
    }
}

pub(crate) fn parse_if_modifier(text: &str) -> IfModifier {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return IfModifier::Unspecified;
    }
    match lookup_directive_name(trimmed) {
        DirectiveName::Parallel => IfModifier::Parallel,
        DirectiveName::Task => IfModifier::Task,
        DirectiveName::Taskloop => IfModifier::Taskloop,
        DirectiveName::Target => IfModifier::Target,
        DirectiveName::TargetData | DirectiveName::TargetDataUnderscore => IfModifier::TargetData,
        DirectiveName::TargetEnterData => IfModifier::TargetEnterData,
        DirectiveName::TargetExitData => IfModifier::TargetExitData,
        DirectiveName::TargetUpdate => IfModifier::TargetUpdate,
        DirectiveName::Simd => IfModifier::Simd,
        DirectiveName::Cancel => IfModifier::Cancel,
        DirectiveName::Other(name) => IfModifier::User(Identifier::new(name.as_ref())),
        _ => IfModifier::User(Identifier::new(trimmed)),
    }
}

/// Extract a leading iterator(...) block, returning the inner text and the
/// remaining clause content after the closing parenthesis.
pub(crate) fn extract_iterator_block(content: &str) -> Option<(String, &str)> {
    let trimmed = content.trim_start();
    const KEYWORD: &str = "iterator";
    if !trimmed.starts_with(KEYWORD) {
        return None;
    }

    let mut idx = KEYWORD.len();
    let bytes = trimmed.as_bytes();

    // Skip whitespace between keyword and '('
    while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
        idx += 1;
    }
    if idx >= bytes.len() || bytes[idx] != b'(' {
        return None;
    }

    let mut depth = 1usize;
    let mut i = idx + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    let inner = trimmed[(idx + 1)..i].to_string();
                    let remainder = &trimmed[(i + 1)..];
                    return Some((inner, remainder));
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn parse_iterator_definition(
    def: &str,
    config: &ParserConfig,
) -> Result<DependIterator, ConversionError> {
    let (lhs, rhs) = def.split_once('=').ok_or_else(|| {
        ConversionError::InvalidClauseSyntax("iterator definition missing '='".into())
    })?;

    let mut lhs_tokens: Vec<&str> = lhs.split_whitespace().collect();
    if lhs_tokens.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "iterator definition missing variable name".into(),
        ));
    }

    let name = lhs_tokens.pop().unwrap().trim();
    let type_name = if lhs_tokens.is_empty() {
        None
    } else {
        Some(lhs_tokens.join(" "))
    };

    let range = rhs.trim();
    let mut parts = range.split(':');
    let start_str = parts.next().ok_or_else(|| {
        ConversionError::InvalidClauseSyntax("iterator missing start expression".into())
    })?;
    let end_str = parts.next().ok_or_else(|| {
        ConversionError::InvalidClauseSyntax("iterator missing end expression".into())
    })?;
    let step_str = parts.next();

    if parts.next().is_some() {
        return Err(ConversionError::InvalidClauseSyntax(
            "iterator has too many ':' separators".into(),
        ));
    }

    let start = Expression::new(start_str, config);
    let end = Expression::new(end_str, config);
    let step = step_str.map(|s| Expression::new(s, config));

    Ok(DependIterator {
        type_name,
        name: Identifier::new(name),
        start,
        end,
        step,
    })
}

pub(crate) fn parse_iterator_block(
    block: &str,
    config: &ParserConfig,
) -> Result<Vec<DependIterator>, ConversionError> {
    let mut iterators = Vec::new();
    let mut current = String::new();
    let mut depth: i32 = 0;

    for ch in block.chars() {
        match ch {
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                let def = current.trim();
                if !def.is_empty() {
                    iterators.push(parse_iterator_definition(def, config)?);
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    if !current.trim().is_empty() {
        iterators.push(parse_iterator_definition(current.trim(), config)?);
    }

    Ok(iterators)
}

fn split_apply_tokens(input: &str) -> (Vec<String>, bool) {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut depth: i32 = 0;
    let mut used_comma = false;
    for ch in input.chars() {
        match ch {
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                if depth > 0 {
                    depth -= 1;
                }
                current.push(ch);
            }
            ',' => {
                if depth == 0 {
                    used_comma = true;
                    let trimmed = current.trim();
                    if !trimmed.is_empty() {
                        tokens.push(trimmed.to_string());
                    }
                    current.clear();
                } else {
                    current.push(ch);
                }
            }
            c if c.is_whitespace() && depth == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    tokens.push(trimmed.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        tokens.push(trimmed.to_string());
    }
    (tokens, used_comma)
}

pub(crate) fn parse_apply_clause(
    content: &str,
) -> Result<(Option<Identifier>, Vec<ApplyTransform>, bool), ConversionError> {
    let (label_part, transforms_part) =
        if let Some((label, rest)) = lang::split_once_top_level(content, ':') {
            (Some(label.trim()), rest.trim())
        } else {
            (None, content.trim())
        };

    let label = label_part.filter(|s| !s.is_empty()).map(Identifier::new);

    let mut transforms = Vec::new();
    let (tokens, used_comma) = split_apply_tokens(transforms_part);
    let mut i = 0;
    while i < tokens.len() {
        let tok = tokens[i].trim();
        let lower = tok.to_ascii_lowercase();
        let next = tokens.get(i + 1).map(|s| s.as_str());

        let kind;
        let mut argument: Option<String> = None;
        if lower.starts_with("unroll") {
            if let Some(arg) = extract_paren_arg(tok) {
                kind = ApplyTransformKind::UnrollPartial;
                argument = Some(arg.trim().to_string());
            } else if let Some(n) = next {
                let nl = n.trim().to_ascii_lowercase();
                if nl.starts_with("partial") {
                    kind = ApplyTransformKind::UnrollPartial;
                    if let Some(arg) = extract_paren_arg(n) {
                        argument = Some(arg.trim().to_string());
                    }
                    i += 1;
                } else if nl.starts_with("full") {
                    kind = ApplyTransformKind::UnrollFull;
                    i += 1;
                } else {
                    kind = ApplyTransformKind::Unroll;
                }
            } else {
                kind = ApplyTransformKind::Unroll;
            }
        } else if lower.starts_with("partial") {
            kind = ApplyTransformKind::UnrollPartial;
            if let Some(arg) = extract_paren_arg(tok) {
                argument = Some(arg.trim().to_string());
            }
        } else if lower.starts_with("full") {
            kind = ApplyTransformKind::UnrollFull;
        } else if lower.starts_with("reverse") {
            kind = ApplyTransformKind::Reverse;
        } else if lower.starts_with("interchange") {
            kind = ApplyTransformKind::Interchange;
        } else if lower.starts_with("nothing") {
            kind = ApplyTransformKind::Nothing;
        } else if lower.starts_with("tile") || lower.starts_with("sizes") {
            kind = ApplyTransformKind::TileSizes;
            if let Some(arg) = extract_paren_arg(tok) {
                argument = Some(arg.trim().to_string());
            } else if let Some(n) = next {
                if n.to_ascii_lowercase().starts_with("sizes") {
                    if let Some(arg) = extract_paren_arg(n) {
                        argument = Some(arg.trim().to_string());
                    }
                    i += 1;
                }
            }
        } else if lower.starts_with("apply") {
            kind = ApplyTransformKind::NestedApply;
            let arg = extract_paren_arg(tok).ok_or_else(|| {
                ConversionError::InvalidClauseSyntax(
                    "nested apply transform requires parenthesized content".to_string(),
                )
            })?;
            argument = Some(arg.trim().to_string());
        } else {
            return Err(ConversionError::InvalidClauseSyntax(format!(
                "unknown apply transform: {tok}"
            )));
        }

        transforms.push(ApplyTransform { kind, argument });
        i += 1;
    }

    Ok((label, transforms, used_comma))
}

pub(crate) fn parse_at_clause(kind: &ClauseKind<'_>) -> Result<ClauseData, ConversionError> {
    if let ClauseKind::Parenthesized(ref content) = kind {
        let value = content.as_ref().trim().to_ascii_lowercase();
        let at_kind = match value.as_str() {
            "compilation" => AtKind::Compilation,
            "execution" => AtKind::Execution,
            "" => {
                return Err(ConversionError::InvalidClauseSyntax(
                    "at clause requires a value".to_string(),
                ))
            }
            other => {
                return Err(ConversionError::InvalidClauseSyntax(format!(
                    "Unknown at clause value: {other}"
                )))
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
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    if let ClauseKind::Parenthesized(ref content) = kind {
        let mut init_kind = InitKind::Unspecified;
        let mut prefer_type = None;
        let mut operand = None;
        let text = content.as_ref().trim();
        if let Some((lhs, rhs)) = lang::split_once_top_level(text, ':') {
            let lhs_trim = lhs.trim();
            if !lhs_trim.is_empty() {
                let modifiers = parse_init_modifier_list(lhs_trim, config)?;
                init_kind = modifiers.kind;
                prefer_type = modifiers.prefer_type;
            }
            let rhs_trim = rhs.trim();
            if !rhs_trim.is_empty() {
                operand = Some(Expression::new(rhs_trim, config));
            }
        } else if !text.is_empty() {
            let modifiers = parse_init_modifier_list(text, config)?;
            init_kind = modifiers.kind;
            prefer_type = modifiers.prefer_type;
        }
        Ok(ClauseData::Init {
            kind: init_kind,
            prefer_type,
            operand,
        })
    } else {
        Err(ConversionError::InvalidClauseSyntax(
            "init clause requires parenthesized content".to_string(),
        ))
    }
}

struct InitModifiers {
    kind: InitKind,
    prefer_type: Option<Expression>,
}

fn parse_init_modifier_list(
    modifiers: &str,
    config: &ParserConfig,
) -> Result<InitModifiers, ConversionError> {
    let mut has_target = false;
    let mut has_targetsync = false;
    let mut saw_modifier = false;
    let mut prefer_type = None;

    for raw in split_top_level_items(modifiers) {
        let modifier = raw.trim();
        if modifier.is_empty() {
            continue;
        }
        saw_modifier = true;

        match modifier.to_ascii_lowercase().as_str() {
            "target" => {
                if has_target {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "duplicate target init modifier".to_string(),
                    ));
                }
                has_target = true;
            }
            "targetsync" => {
                if has_targetsync {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "duplicate targetsync init modifier".to_string(),
                    ));
                }
                has_targetsync = true;
            }
            _ if is_prefer_type_init_modifier(modifier) => {
                if prefer_type.is_some() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "duplicate prefer_type init modifier".to_string(),
                    ));
                }
                let spec = extract_prefer_type_init_modifier(modifier).expect("validated above");
                prefer_type = Some(Expression::new(spec, config));
            }
            _ => {
                return Err(ConversionError::InvalidClauseSyntax(format!(
                    "Unknown init clause modifier: {modifier}"
                )))
            }
        }
    }

    let kind = match (has_target, has_targetsync, saw_modifier) {
        (true, true, _) => InitKind::TargetAndTargetsync,
        (true, false, _) => InitKind::Target,
        (false, true, _) => InitKind::Targetsync,
        (false, false, true) => {
            return Err(ConversionError::InvalidClauseSyntax(
                "init modifier list requires target or targetsync".to_string(),
            ))
        }
        (false, false, false) => InitKind::Unspecified,
    };

    Ok(InitModifiers { kind, prefer_type })
}

fn is_prefer_type_init_modifier(modifier: &str) -> bool {
    extract_prefer_type_init_modifier(modifier).is_some()
}

fn extract_prefer_type_init_modifier(modifier: &str) -> Option<&str> {
    let trimmed = modifier.trim();
    let open_paren = trimmed.find('(')?;

    if trimmed[..open_paren]
        .trim()
        .eq_ignore_ascii_case("prefer_type")
    {
        extract_paren_arg(trimmed)
    } else {
        None
    }
}

pub(crate) fn parse_induction_clause(
    kind: &ClauseKind<'_>,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    if let ClauseKind::Parenthesized(ref content) = kind {
        let text = content.as_ref().trim();
        if text.is_empty() {
            return Ok(ClauseData::Induction { items: Vec::new() });
        }

        let mut items = Vec::new();
        for token in split_top_level_items(text) {
            let part = token.trim();
            if part.is_empty() {
                continue;
            }
            let lower = part.to_ascii_lowercase();
            if lower.starts_with("step") {
                let paren_pos = part.find('(').ok_or_else(|| {
                    ConversionError::InvalidClauseSyntax(
                        "step entry in induction clause must be step(expr)".into(),
                    )
                })?;
                let prefix = part[..paren_pos].trim();
                if !prefix.eq_ignore_ascii_case("step") {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "step entry in induction clause must start with step(".into(),
                    ));
                }
                let (inner, rest) = extract_parenthesized(&part[paren_pos..])?;
                if !rest.trim().is_empty() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "step entry in induction clause must be step(expr)".into(),
                    ));
                }
                let expr_text = inner.trim();
                if expr_text.is_empty() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "step expression missing in induction clause".into(),
                    ));
                }
                items.push(InductionItem::Step(Expression::new(expr_text, config)));
                continue;
            }

            if let Some((label_part, expr_part)) = lang::split_once_top_level(part, ':') {
                let expr_text = expr_part.trim();
                if expr_text.is_empty() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "induction binding missing expression after ':'".into(),
                    ));
                }
                let label = label_part.trim();
                let label_id = if label.is_empty() {
                    None
                } else {
                    Some(Identifier::new(label))
                };
                items.push(InductionItem::Binding {
                    label: label_id,
                    expression: Expression::new(expr_text, config),
                });
            } else {
                items.push(InductionItem::Passthrough(Expression::new(part, config)));
            }
        }

        Ok(ClauseData::Induction { items })
    } else {
        Err(ConversionError::InvalidClauseSyntax(
            "induction clause requires parenthesized content".to_string(),
        ))
    }
}

/// Parse a linear clause
///
/// Format: `linear([modifier(list):] list[:step])`
///
/// Uses top-level colon detection to properly handle nested structures.
///
/// ## Example
///
/// ```
/// # use roup::ir::{convert::parse_linear_clause, ParserConfig, Language};
/// let config = ParserConfig::with_parsing(Language::C);
/// let clause = parse_linear_clause("x, y: 2", &config).unwrap();
/// // Returns ClauseData::Linear with items=[x, y], step=Some(2)
/// ```
pub fn parse_linear_clause(
    content: &str,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    let mut remaining = content.trim();
    let mut modifier_items: Vec<ClauseItem> = Vec::new();
    let mut modifier: Option<LinearModifier> = None;

    // Handle modifier(list): prefix. Accept val/ref/uval for OpenMP 5.1+.
    let lower = remaining.to_ascii_lowercase();
    let (modifier_len, modifier_kind) = if lower.starts_with("val") {
        (3, Some(LinearModifier::Val))
    } else if lower.starts_with("ref") {
        (3, Some(LinearModifier::Ref))
    } else if lower.starts_with("uval") {
        (4, Some(LinearModifier::Uval))
    } else {
        (0, None)
    };

    if let Some(kind) = modifier_kind {
        let after_keyword = remaining[modifier_len..].trim_start();
        if after_keyword.starts_with('(') {
            let (inner, rest) = extract_parenthesized(after_keyword)?;
            modifier_items = parse_identifier_list(inner.trim(), config)?;
            modifier = Some(kind);
            remaining = rest.trim_start();
            if remaining.starts_with(':') {
                remaining = remaining[1..].trim_start();
            } else if !remaining.is_empty() {
                return Err(ConversionError::InvalidClauseSyntax(
                    "linear modifier list must be followed by ':'".to_string(),
                ));
            }
        }
    }

    let (items_str, step_str) =
        if let Some((items_part, step_part)) = lang::rsplit_once_top_level(remaining, ':') {
            (items_part.trim(), Some(step_part.trim()))
        } else if modifier.is_some() && !remaining.is_empty() {
            ("", Some(remaining))
        } else {
            (remaining, None)
        };

    let mut items = if items_str.is_empty() {
        modifier_items.clone()
    } else {
        parse_identifier_list(items_str, config)?
    };

    if items.is_empty() {
        items = modifier_items;
    }

    let step = step_str.map(|s| Expression::new(s, config));

    Ok(ClauseData::Linear {
        modifier,
        items,
        step,
    })
}

pub(crate) fn parse_defaultmap_clause(
    kind: &ClauseKind<'_>,
) -> Result<ClauseData, ConversionError> {
    if let ClauseKind::Parenthesized(ref content) = kind {
        let text = content.as_ref().trim();
        if text.is_empty() {
            return Ok(ClauseData::Defaultmap {
                behavior: DefaultmapBehavior::Unspecified,
                category: None,
            });
        }

        let (behavior_str, category_str) =
            if let Some((behavior, rest)) = lang::split_once_top_level(text, ':') {
                (behavior.trim(), Some(rest.trim()))
            } else {
                (text, None)
            };

        let behavior = parse_defaultmap_behavior(behavior_str)?;
        let category = match category_str {
            Some(value) if !value.is_empty() => Some(parse_defaultmap_category(value)?),
            _ => None,
        };

        Ok(ClauseData::Defaultmap { behavior, category })
    } else {
        Err(ConversionError::InvalidClauseSyntax(
            "defaultmap clause requires parenthesized content".to_string(),
        ))
    }
}

#[allow(dead_code)]
pub(crate) fn parse_metadirective_selector(
    clause: &Clause<'_>,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    if let ClauseKind::Parenthesized(ref content) = clause.kind {
        let raw = content.as_ref();
        let mut selector = parse_selector_content(raw, config)?;
        selector.raw = Some(raw.trim().to_string());
        Ok(ClauseData::MetadirectiveSelector {
            selector: Box::new(selector),
        })
    } else {
        Err(ConversionError::InvalidClauseSyntax(
            "metadirective selector requires parentheses".to_string(),
        ))
    }
}

fn parse_selector_content(
    content: &str,
    config: &ParserConfig,
) -> Result<OmpSelector, ConversionError> {
    let trimmed = content.trim();
    let (selector_part, nested_directive_part) = split_selector_and_directive(trimmed);

    let mut selector = OmpSelector::default();

    // Parse selector key/value pairs (device, implementation, user, construct)
    if !selector_part.is_empty() {
        for entry in split_top_level_items(selector_part) {
            let entry = entry.trim();
            if entry.is_empty() {
                continue;
            }
            let Some((key, value)) = entry.split_once('=') else {
                continue;
            };
            let key = key.trim().to_ascii_lowercase();
            let value = value.trim();
            match key.as_str() {
                "device" => {
                    selector.device = Some(parse_device_selector(value, config)?);
                    selector.order.push(crate::ast::OmpSelectorKey::Device);
                }
                "implementation" | "impl" => {
                    selector.implementation = Some(parse_impl_selector(value)?);
                    selector
                        .order
                        .push(crate::ast::OmpSelectorKey::Implementation);
                }
                "user" => {
                    selector.user = Some(parse_user_selector(value, config)?);
                    selector.order.push(crate::ast::OmpSelectorKey::User);
                }
                "construct" | "constructs" => {
                    selector.constructs = Some(parse_constructs_selector(value, config)?);
                    selector.order.push(crate::ast::OmpSelectorKey::Construct);
                }
                "target_device" => {
                    selector.device = Some(parse_device_selector(value, config)?);
                    selector.is_target_device = true;
                    selector.order.push(crate::ast::OmpSelectorKey::Device);
                }
                other => {
                    return Err(ConversionError::InvalidClauseSyntax(format!(
                        "unknown metadirective selector key: {other}"
                    )))
                }
            }
        }
    }

    // Nested directive after colon (parse into nested_directive AST)
    if let Some(nested) = nested_directive_part {
        let nested_trimmed = nested.trim();
        if !nested_trimmed.is_empty() {
            if let Some(dir) = parse_nested_directive(nested_trimmed, config)? {
                selector.nested_directive = Some(Box::new(dir));
            }
        }
    }

    // For otherwise/default metadirective clauses that provide only a directive
    // payload (no selectors, no colon), treat the remaining text as the nested
    // directive rather than dropping it on the floor.
    if selector.nested_directive.is_none()
        && nested_directive_part.is_none()
        && selector.device.is_none()
        && selector.implementation.is_none()
        && selector.user.is_none()
        && selector.constructs.is_none()
    {
        let candidate = selector_part.trim();
        if !candidate.is_empty() {
            if let Some(dir) = parse_nested_directive(candidate, config)? {
                selector.nested_directive = Some(Box::new(dir));
            }
        }
    }

    Ok(selector)
}

pub(crate) fn parse_nested_directive(
    text: &str,
    config: &ParserConfig,
) -> Result<Option<OmpDirective>, ConversionError> {
    let lexer_lang = map_ir_language_to_lexer(config.language());
    // Use the OpenMP parser with its complete directive/clause registries so
    // combined constructs (e.g., teams distribute parallel for) parse the same
    // way as top-level directives instead of devolving into pseudo-clauses.
    let parser = crate::parser::openmp::parser().with_language(lexer_lang);
    let prefixed = match lexer_lang {
        LexerLanguage::C => format!("#pragma omp {text}"),
        LexerLanguage::FortranFree | LexerLanguage::FortranFixed => format!("!$omp {text}"),
    };
    match parser.parse(&prefixed) {
        Ok((_rest, directive)) => {
            let kind = lookup_directive_name(directive.name.as_ref());
            let directive_kind = OmpDirectiveKind::try_from(kind).map_err(|_| {
                ConversionError::InvalidClauseSyntax(format!(
                    "Unknown nested directive in selector: {}",
                    directive.name.as_ref()
                ))
            })?;
            let mut clauses = Vec::new();
            for clause in &directive.clauses {
                let payload = parse_clause_data(clause, config)?;
                let clause_name = lookup_clause_name(clause.name.as_ref());
                let kind = OmpClauseKind::try_from(clause_name.clone()).map_err(|_| {
                    ConversionError::InvalidClauseSyntax(format!(
                        "Unknown clause in nested directive: {}",
                        clause.name.as_ref()
                    ))
                })?;
                clauses.push(crate::ast::OmpClause {
                    kind,
                    payload,
                    separator: clause.separator,
                });
            }
            Ok(Some(OmpDirective {
                kind: directive_kind,
                parameter: None,
                clauses,
            }))
        }
        Err(_e) => {
            // Fallback: accept directive names that omit spaces (e.g., paralleldo)
            // to keep selector payload structured even when the nested directive
            // uses legacy formatting.
            if let Some(kind) = lookup_omp_construct(text) {
                return Ok(Some(OmpDirective {
                    kind,
                    parameter: None,
                    clauses: Vec::new(),
                }));
            }
            Ok(None)
        }
    }
}

fn split_selector_and_directive(input: &str) -> (&str, Option<&str>) {
    if let Some(idx) = find_top_level_colon(input) {
        let left = input[..idx].trim();
        let right = input[idx + 1..].trim();
        (left, Some(right))
    } else {
        (input, None)
    }
}

fn find_top_level_colon(input: &str) -> Option<usize> {
    let mut depth = 0;
    for (idx, ch) in input.char_indices() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            ':' if depth == 0 => return Some(idx),
            _ => {}
        }
    }
    None
}

fn map_ir_language_to_lexer(lang: Language) -> LexerLanguage {
    match lang {
        Language::C => LexerLanguage::C,
        Language::Cpp => LexerLanguage::C,
        Language::Fortran => LexerLanguage::FortranFree,
        _ => LexerLanguage::C,
    }
}

fn parse_device_selector(
    value: &str,
    config: &ParserConfig,
) -> Result<OmpSelectorDevice, ConversionError> {
    let mut device = OmpSelectorDevice::default();
    let inner = strip_braces(value).trim();
    if inner.is_empty() {
        return Ok(device);
    }

    for item in split_top_level_items(inner) {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        if let Some(arg) = item.strip_prefix("kind") {
            let args = extract_paren_arg(arg).unwrap_or(arg);
            let (score, val) = parse_scored_value(args);
            device.kind = Some(OmpSelectorScoredValue { score, value: val });
        } else if let Some(arg) = item.strip_prefix("isa") {
            let args = extract_paren_arg(arg).unwrap_or(arg);
            for isa in split_top_level_items(args) {
                let isa = isa.trim();
                if !isa.is_empty() {
                    let (score, val) = parse_scored_value(isa);
                    device
                        .isa
                        .push(OmpSelectorScoredValue { score, value: val });
                }
            }
        } else if let Some(arg) = item.strip_prefix("arch") {
            let args = extract_paren_arg(arg).unwrap_or(arg);
            for arch in split_top_level_items(args) {
                let arch = arch.trim();
                if !arch.is_empty() {
                    let (score, val) = parse_scored_value(arch);
                    device
                        .arch
                        .push(OmpSelectorScoredValue { score, value: val });
                }
            }
        } else if let Some(arg) = item.strip_prefix("device_num") {
            if let Some(expr) = extract_paren_arg(arg) {
                let (score, val) = parse_scored_value(expr);
                let expr_text = val.trim();
                if !expr_text.is_empty() {
                    device.device_num = Some(Expression::new(expr_text, config));
                    device.device_num_score = score;
                }
            }
        } else {
            return Err(ConversionError::InvalidClauseSyntax(format!(
                "unknown device selector trait: {item}"
            )));
        }
    }

    Ok(device)
}

fn parse_impl_selector(value: &str) -> Result<OmpSelectorImpl, ConversionError> {
    let mut implementation = OmpSelectorImpl::default();
    let inner = strip_braces(value).trim();
    if inner.is_empty() {
        return Ok(implementation);
    }

    for item in split_top_level_items(inner) {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        if let Some(arg) = item.strip_prefix("vendor") {
            let args = extract_paren_arg(arg).unwrap_or(arg);
            let vendor = args.trim();
            if !vendor.is_empty() {
                let (score, val) = parse_scored_value(vendor);
                implementation.vendor = Some(val);
                implementation.vendor_score = score;
            }
        } else if let Some(arg) = item.strip_prefix("extension") {
            let args = extract_paren_arg(arg).unwrap_or(arg);
            for ext in split_top_level_items(args) {
                let ext = ext.trim();
                if !ext.is_empty() {
                    let (score, val) = parse_scored_value(ext);
                    implementation.extensions.push(val);
                    implementation.extension_scores.push(score);
                }
            }
        } else if let Some(arg) = item.strip_prefix("requires") {
            let args = extract_paren_arg(arg).unwrap_or(arg);
            for req in split_top_level_items(args) {
                let req = req.trim();
                if req.is_empty() {
                    continue;
                }
                let (score, val) = parse_scored_value(req);
                implementation.requires.push(val);
                implementation.require_scores.push(score);
            }
        } else {
            // Treat as user-defined implementation expression
            let (score, val) = parse_scored_value(item);
            let expr = val.trim();
            if !expr.is_empty() {
                implementation.user_expression = Some(expr.to_string());
                implementation.user_expression_score = score;
            }
        }
    }

    Ok(implementation)
}

fn parse_user_selector(
    value: &str,
    config: &ParserConfig,
) -> Result<OmpSelectorUser, ConversionError> {
    let mut user = OmpSelectorUser::default();
    let inner = strip_braces(value).trim();
    if inner.is_empty() {
        return Ok(user);
    }

    if let Some(arg) = inner.strip_prefix("condition") {
        if let Some(expr_body) = extract_paren_arg(arg) {
            let expr_text = expr_body.trim();
            if !expr_text.is_empty() {
                user.condition = Some(Expression::new(expr_text, config));
            }
        }
    } else {
        return Err(ConversionError::InvalidClauseSyntax(format!(
            "unknown user selector trait: {inner}"
        )));
    }

    Ok(user)
}

fn parse_constructs_selector(
    value: &str,
    config: &ParserConfig,
) -> Result<OmpSelectorConstructs, ConversionError> {
    let mut constructs = OmpSelectorConstructs::default();
    let inner = strip_braces(value).trim();
    if inner.is_empty() {
        return Ok(constructs);
    }

    for item in split_top_level_items(inner) {
        let text = item.trim();
        if text.is_empty() {
            continue;
        }

        // Extract optional score(...) prefix at the start of the selector item.
        let mut remaining = text;
        let mut score: Option<String> = None;
        if remaining.starts_with("score(") {
            if let Some(end) = find_matching_delim(remaining, 5, '(', ')') {
                let s = remaining[6..end].trim();
                if !s.is_empty() {
                    score = Some(s.to_string());
                }
                let after = remaining[end + 1..].trim_start();
                let after = after.strip_prefix(':').unwrap_or(after).trim_start();
                remaining = after;
            }
        }

        // Separate directive name and clause text (handle parens).
        let mut directive_text = remaining.trim().to_string();
        if let Some(open) = directive_text.find('(') {
            let close = find_matching_delim(&directive_text, open, '(', ')').ok_or_else(|| {
                ConversionError::InvalidClauseSyntax("Unbalanced construct selector".into())
            })?;
            let inner = directive_text[open + 1..close].trim();
            // If the inner starts with score(...):, strip that score and colon.
            let (inner_score, val, rest) = split_score_and_value(inner);
            if score.is_none() {
                score = inner_score;
            }
            let clause_part = rest.unwrap_or(val).trim();
            directive_text = format!("{} {}", directive_text[..open].trim(), clause_part)
                .trim()
                .to_string();
        }

        if let Some(dir) = parse_nested_directive(&directive_text, config)? {
            let kind = dir.kind;
            constructs.constructs.push(OmpSelectorConstruct {
                score: score.clone(),
                kind,
                directive: Box::new(dir),
            });
            constructs.scores.push(score);
        } else {
            // If we cannot fully parse the nested directive, fall back to a bare directive kind
            // to keep structured (enum) representation without raw strings.
            let trimmed = directive_text.trim();
            if trimmed.is_empty() {
                return Err(ConversionError::InvalidClauseSyntax(
                    "Empty construct selector".into(),
                ));
            }
            if let Some(kind) = lookup_omp_construct(trimmed) {
                constructs.constructs.push(OmpSelectorConstruct {
                    score: score.clone(),
                    kind,
                    directive: Box::new(OmpDirective {
                        kind,
                        parameter: None,
                        clauses: Vec::new(),
                    }),
                });
                constructs.scores.push(score);
            } else {
                return Err(ConversionError::InvalidClauseSyntax(format!(
                    "Unable to parse construct selector: {directive_text}"
                )));
            }
        }
    }

    Ok(constructs)
}

#[allow(dead_code)]
fn lookup_omp_construct(name: &str) -> Option<OmpDirectiveKind> {
    let canonical = crate::parser::directive_kind::lookup_directive_name(name);
    OmpDirectiveKind::try_from(canonical).ok()
}

fn parse_scored_value(input: &str) -> (Option<String>, String) {
    let trimmed = input.trim();
    let (score, rest, _) = split_score_and_value(trimmed);
    (score, rest.to_string())
}

fn split_score_and_value(input: &str) -> (Option<String>, &str, Option<&str>) {
    let mut score = None;
    let mut remainder = input;
    if let Some(start) = input.find("score(") {
        if let Some(end) = input[start..].find(')') {
            let score_val = &input[start + 6..start + end].trim();
            if !score_val.is_empty() {
                score = Some(score_val.to_string());
            }
            let after = input
                .get(start + end + 1..)
                .unwrap_or("")
                .trim_start_matches(':');
            remainder = after.trim();
        }
    }
    // Also split on colon if present (nested directive hint)
    if let Some(colon) = remainder.find(':') {
        let val = remainder[..colon].trim();
        let rest = remainder.get(colon + 1..).map(str::trim);
        // If the value before ':' is empty (e.g., " : nohost"), use the rest as the value.
        if val.is_empty() {
            if let Some(r) = rest {
                return (score, r, None);
            }
        }
        return (score, val, rest);
    }
    (score, remainder, None)
}

fn find_matching_delim(
    text: &str,
    open_pos: usize,
    open_ch: char,
    close_ch: char,
) -> Option<usize> {
    if text.as_bytes().get(open_pos)? != &(open_ch as u8) {
        return None;
    }
    let mut depth = 1;
    for (idx, ch) in text.chars().enumerate().skip(open_pos + 1) {
        if ch == open_ch {
            depth += 1;
        } else if ch == close_ch {
            depth -= 1;
            if depth == 0 {
                return Some(idx);
            }
        }
    }
    None
}

fn strip_braces(value: &str) -> &str {
    let trimmed = value.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') && trimmed.len() >= 2 {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    }
}

fn extract_paren_arg(input: &str) -> Option<&str> {
    let trimmed = input.trim();
    if let Some(start) = trimmed.find('(') {
        if trimmed.ends_with(')') && start < trimmed.len() - 1 {
            return Some(&trimmed[start + 1..trimmed.len() - 1]);
        }
    }
    None
}

fn parse_defaultmap_behavior(value: &str) -> Result<DefaultmapBehavior, ConversionError> {
    let normalized = value.trim().to_ascii_lowercase();
    let behavior = match normalized.as_str() {
        "" | "unspecified" => DefaultmapBehavior::Unspecified,
        "alloc" => DefaultmapBehavior::Alloc,
        "to" => DefaultmapBehavior::To,
        "from" => DefaultmapBehavior::From,
        "tofrom" => DefaultmapBehavior::Tofrom,
        "firstprivate" => DefaultmapBehavior::Firstprivate,
        "none" => DefaultmapBehavior::None,
        "default" => DefaultmapBehavior::Default,
        "present" => DefaultmapBehavior::Present,
        other => {
            return Err(ConversionError::InvalidClauseSyntax(format!(
                "Unknown defaultmap behavior: {other}"
            )))
        }
    };
    Ok(behavior)
}

fn parse_defaultmap_category(value: &str) -> Result<DefaultmapCategory, ConversionError> {
    let normalized = value.trim().to_ascii_lowercase();
    let category = match normalized.as_str() {
        "" | "unspecified" => DefaultmapCategory::Unspecified,
        "scalar" => DefaultmapCategory::Scalar,
        "aggregate" => DefaultmapCategory::Aggregate,
        "pointer" => DefaultmapCategory::Pointer,
        "all" => DefaultmapCategory::All,
        "allocatable" => DefaultmapCategory::Allocatable,
        other => {
            return Err(ConversionError::InvalidClauseSyntax(format!(
                "Unknown defaultmap category: {other}"
            )))
        }
    };
    Ok(category)
}

pub(crate) fn parse_uses_allocators_clause(
    kind: &ClauseKind<'_>,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    if let ClauseKind::Parenthesized(ref content) = kind {
        let entries = split_top_level_items(content.as_ref());
        let mut allocators = Vec::new();
        for raw in entries {
            let entry = raw.trim();
            if entry.is_empty() {
                continue;
            }
            let (allocator_name, traits_expr, traits_first) =
                if let Some(colon_idx) = find_top_level_colon(entry) {
                    let left = entry[..colon_idx].trim();
                    let right = entry[colon_idx + 1..].trim();
                    if right.is_empty() {
                        let (name, traits) = split_allocator_entry(entry)?;
                        (name, traits, false)
                    } else {
                        let traits_expr = if left.to_ascii_lowercase().starts_with("traits") {
                            if let Some(open) = left.find('(') {
                                let close = left.rfind(')');
                                close.map(|end| left[open + 1..end].trim())
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        (right, traits_expr, true)
                    }
                } else {
                    let (name, traits) = split_allocator_entry(entry)?;
                    (name, traits, false)
                };

            let allocator_kind = classify_allocator_name(allocator_name);
            let traits = traits_expr.and_then(|expr_text| {
                let trimmed = expr_text.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(Expression::new(trimmed, config))
                }
            });
            allocators.push(UsesAllocatorSpec {
                allocator: allocator_kind,
                traits,
                traits_first,
            });
        }

        Ok(ClauseData::UsesAllocators { allocators })
    } else {
        Err(ConversionError::InvalidClauseSyntax(
            "uses_allocators clause requires parenthesized content".to_string(),
        ))
    }
}

pub(crate) fn parse_requires_clause(
    kind: &ClauseKind<'_>,
    _config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    let raw_content = match kind {
        ClauseKind::Parenthesized(ref content) => content.as_ref().to_string(),
        ClauseKind::VariableList(list) => list
            .iter()
            .map(|c| c.as_ref().to_string())
            .collect::<Vec<_>>()
            .join(", "),
        ClauseKind::Bare => String::new(),
        _ => String::new(),
    };

    let items = split_requires_items(raw_content.as_str());
    let mut requirements = Vec::new();
    for raw in items {
        let token = raw.trim();
        if token.is_empty() {
            continue;
        }
        let normalized = token.to_ascii_lowercase();
        match normalized.as_str() {
            "reverse_offload" => requirements.push(RequireModifier::ReverseOffload),
            "unified_address" => requirements.push(RequireModifier::UnifiedAddress),
            "unified_shared_memory" => requirements.push(RequireModifier::UnifiedSharedMemory),
            "dynamic_allocators" => requirements.push(RequireModifier::DynamicAllocators),
            "self_maps" => requirements.push(RequireModifier::SelfMaps),
            "atomic_default_mem_order" => {
                return Err(ConversionError::InvalidClauseSyntax(
                    "atomic_default_mem_order requires a value".to_string(),
                ))
            }
            value if value.starts_with("atomic_default_mem_order") => {
                if let Some((_, order)) = value.split_once('(') {
                    let order = order.trim_end_matches(')').trim();
                    let mo = parse_memory_order(order)?;
                    requirements.push(RequireModifier::AtomicDefaultMemOrder(mo));
                } else {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "atomic_default_mem_order requires a value".to_string(),
                    ));
                }
            }
            "ext_implementation_defined_requirement" => {
                requirements.push(RequireModifier::ExtImplementationDefinedRequirement(None))
            }
            other => {
                let mo = parse_memory_order(other).ok();
                if let Some(order) = mo {
                    requirements.push(RequireModifier::AtomicDefaultMemOrder(order));
                } else {
                    requirements.push(RequireModifier::ExtImplementationDefinedRequirement(Some(
                        Identifier::new(token),
                    )));
                }
            }
        }
    }
    if requirements.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "requires clause must specify at least one requirement".to_string(),
        ));
    }
    Ok(ClauseData::Requires { requirements })
}

fn split_requires_items(input: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut depth = 0i32;

    for (idx, ch) in input.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' if depth > 0 => depth -= 1,
            ',' => {
                if depth == 0 {
                    if let Some(seg) = input.get(start..idx) {
                        let trimmed = seg.trim();
                        if !trimmed.is_empty() {
                            parts.push(trimmed);
                        }
                    }
                    start = idx + 1;
                }
            }
            _ if ch.is_whitespace() && depth == 0 => {
                if let Some(seg) = input.get(start..idx) {
                    let trimmed = seg.trim();
                    if !trimmed.is_empty() {
                        parts.push(trimmed);
                    }
                }
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }

    if let Some(seg) = input.get(start..) {
        let trimmed = seg.trim();
        if !trimmed.is_empty() {
            parts.push(trimmed);
        }
    }

    parts
}

pub(crate) fn parse_memory_order(value: &str) -> Result<MemoryOrder, ConversionError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "seq_cst" => Ok(MemoryOrder::SeqCst),
        "acq_rel" => Ok(MemoryOrder::AcqRel),
        "release" => Ok(MemoryOrder::Release),
        "acquire" => Ok(MemoryOrder::Acquire),
        "relaxed" => Ok(MemoryOrder::Relaxed),
        other => Err(ConversionError::InvalidClauseSyntax(format!(
            "Unknown memory order: {other}"
        ))),
    }
}

pub(crate) fn parse_device_clause(
    kind: &ClauseKind<'_>,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    if let ClauseKind::Parenthesized(ref content) = kind {
        let text = content.as_ref().trim();
        let (modifier, expr_text) = if let Some((m, rest)) = text.split_once(':') {
            let modifier = match m.trim() {
                "ancestor" => DeviceModifier::Ancestor,
                "device_num" => DeviceModifier::DeviceNum,
                other => {
                    return Err(ConversionError::InvalidClauseSyntax(format!(
                        "Unknown device modifier: {other}"
                    )))
                }
            };
            (modifier, rest.trim())
        } else {
            (DeviceModifier::Unspecified, text)
        };

        Ok(ClauseData::Device {
            modifier,
            device_num: Expression::new(expr_text, config),
        })
    } else {
        Err(ConversionError::InvalidClauseSyntax(
            "device clause requires parenthesized expression".to_string(),
        ))
    }
}

fn split_allocator_entry(input: &str) -> Result<(&str, Option<&str>), ConversionError> {
    if let Some(start) = input.find('(') {
        let mut depth = 0;
        let mut end = None;
        for (idx, ch) in input.char_indices().skip(start) {
            match ch {
                '(' => {
                    depth += 1;
                }
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(idx);
                        break;
                    }
                }
                _ => {}
            }
        }

        let end_idx = end.ok_or_else(|| {
            ConversionError::InvalidClauseSyntax(
                "uses_allocators clause has unmatched parentheses".to_string(),
            )
        })?;

        let name = input[..start].trim();
        let traits = input[start + 1..end_idx].trim();
        Ok((name, Some(traits)))
    } else {
        Ok((input.trim(), None))
    }
}

pub(crate) fn split_top_level_items(input: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (idx, ch) in input.char_indices() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            ',' if depth == 0 => {
                parts.push(&input[start..idx]);
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }
    if start < input.len() {
        parts.push(&input[start..]);
    }
    parts
}

pub(crate) fn classify_allocator_name(name: &str) -> UsesAllocatorKind {
    let trimmed = name.trim();
    let lower = trimmed.to_ascii_lowercase();
    match lower.as_str() {
        "omp_default_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::Default),
        "omp_large_cap_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::LargeCap),
        "omp_const_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::Const),
        "omp_high_bw_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::HighBw),
        "omp_low_lat_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::LowLat),
        "omp_cgroup_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::Cgroup),
        "omp_pteam_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::Pteam),
        "omp_thread_mem_alloc" => UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::Thread),
        _ => UsesAllocatorKind::Custom(Identifier::new(trimmed)),
    }
}

pub(crate) fn parse_scan_clause(
    mode: ScanClauseMode,
    clause: &Clause<'_>,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    let items = match &clause.kind {
        ClauseKind::Parenthesized(content) => parse_identifier_list(content.as_ref(), config)?,
        ClauseKind::VariableList(vars) => {
            let joined = vars.join(", ");
            parse_identifier_list(&joined, config)?
        }
        _ => {
            return Err(ConversionError::InvalidClauseSyntax(
                "scan clause requires a variable list".to_string(),
            ))
        }
    };

    if items.is_empty() {
        return Err(ConversionError::InvalidClauseSyntax(
            "scan clause requires a non-empty variable list".to_string(),
        ));
    }

    Ok(ClauseData::Scan { mode, items })
}

pub fn parse_clause_data<'a>(
    clause: &'a Clause<'a>,
    config: &ParserConfig,
) -> Result<ClauseData, ConversionError> {
    let clause_name = clause.name.as_ref();
    let clause_kind = lookup_clause_name(clause_name);

    match clause_kind {
        // Bare clauses (no parameters)
        ClauseName::Nowait => match &clause.kind {
            ClauseKind::Bare => Ok(ClauseData::Nowait { modifier: None }),
            ClauseKind::Parenthesized(content)
                if content.as_ref().trim().eq_ignore_ascii_case("is_deferred") =>
            {
                Ok(ClauseData::Nowait {
                    modifier: Some(NowaitModifier::IsDeferred),
                })
            }
            _ => Err(ConversionError::InvalidClauseSyntax(
                "nowait clause accepts only optional is_deferred modifier".to_string(),
            )),
        },

        ClauseName::Nogroup
        | ClauseName::Untied
        | ClauseName::Mergeable
        | ClauseName::SeqCst
        | ClauseName::Relaxed
        | ClauseName::Release
        | ClauseName::Acquire
        | ClauseName::AcqRel => match &clause.kind {
            ClauseKind::Bare => Ok(ClauseData::Bare(Identifier::new(clause_name))),
            _ => Err(ConversionError::InvalidClauseSyntax(format!(
                "{clause_name} clause does not take arguments"
            ))),
        },

        // default(kind)
        ClauseName::Default => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref();
                let kind_str = content.trim();
                let kind_norm = kind_str.to_ascii_lowercase();
                let kind = match kind_norm.as_str() {
                    "shared" => Some(DefaultKind::Shared),
                    "none" => Some(DefaultKind::None),
                    "private" => Some(DefaultKind::Private),
                    "firstprivate" => Some(DefaultKind::Firstprivate),
                    "variant" => Some(DefaultKind::Variant),
                    _ => None,
                };
                if let Some(kind) = kind {
                    Ok(ClauseData::Default(kind))
                } else {
                    let directive = parse_nested_directive(kind_str, config)?.ok_or_else(|| {
                        ConversionError::InvalidClauseSyntax(format!(
                            "Unrecognized default clause content: {kind_str}"
                        ))
                    })?;
                    Ok(ClauseData::MetadirectiveDefault { directive })
                }
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "default clause requires parenthesized content".to_string(),
                ))
            }
        }

        // Metadirective selectors: parse into typed selector data (raw today)
        ClauseName::When | ClauseName::Otherwise | ClauseName::Match => {
            parse_metadirective_selector(clause, config)
        }

        // defaultmap(behavior[:category])
        ClauseName::Defaultmap => parse_defaultmap_clause(&clause.kind),

        // sizes(list) on tile directive
        ClauseName::Sizes => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let items = parse_identifier_list(content.as_ref(), config)?;
                Ok(ClauseData::ItemList(items))
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

        // firstprivate(list)
        ClauseName::Firstprivate => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref();
                let items = parse_identifier_list(content, config)?;
                Ok(ClauseData::Firstprivate { items })
            } else {
                Ok(ClauseData::Firstprivate { items: vec![] })
            }
        }

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

        // to/from/link(list) used by declare target and friends
        ClauseName::To | ClauseName::From | ClauseName::Link => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                match parse_identifier_list(content.as_ref(), config) {
                    Ok(items) => {
                        if items.is_empty() {
                            Err(ConversionError::InvalidClauseSyntax(format!(
                                "{clause_name} clause requires a non-empty variable list"
                            )))
                        } else {
                            Ok(ClauseData::ItemList(items))
                        }
                    }
                    Err(_) => Ok(ClauseData::Expression(Expression::new(
                        content.as_ref().trim(),
                        config,
                    ))),
                }
            } else {
                Err(ConversionError::InvalidClauseSyntax(format!(
                    "{clause_name} clause requires parenthesized content"
                )))
            }
        }

        // interop/enter/local clauses expect a variable list payload
        ClauseName::Interop | ClauseName::Enter | ClauseName::Local => match &clause.kind {
            ClauseKind::Parenthesized(content) => {
                let items = parse_identifier_list(content.as_ref(), config)?;
                Ok(ClauseData::ItemList(items))
            }
            ClauseKind::VariableList(vars) => {
                let joined = vars.join(", ");
                let items = parse_identifier_list(&joined, config)?;
                Ok(ClauseData::ItemList(items))
            }
            _ => Err(ConversionError::InvalidClauseSyntax(format!(
                "{clause_name} clause requires a variable list"
            ))),
        },

        // num_threads(expr)
        ClauseName::NumThreads => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref();
                Ok(ClauseData::NumThreads {
                    num: Expression::new(content.trim(), config),
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
                // Check for directive-name modifier: "if(parallel: condition)"
                if let Some((modifier, condition)) = lang::split_once_top_level(content, ':') {
                    let modifier = modifier.trim();
                    Ok(ClauseData::If {
                        modifier: Some(parse_if_modifier(modifier)),
                        condition: Expression::new(condition.trim(), config),
                    })
                } else {
                    Ok(ClauseData::If {
                        modifier: None,
                        condition: Expression::new(content.trim(), config),
                    })
                }
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
                Ok(ClauseData::Collapse {
                    n: Expression::new(content.trim(), config),
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
            ClauseKind::Parenthesized(ref content) => Ok(ClauseData::Ordered {
                n: Some(Expression::new(content.as_ref().trim(), config)),
            }),
            // OpenACC-specific structured clauses should not appear in OpenMP context
            _ => Err(ConversionError::InvalidClauseSyntax(
                "Unexpected structured clause for 'ordered'".to_string(),
            )),
        },

        // reduction(operator: list)
        ClauseName::Reduction => match &clause.kind {
            ClauseKind::Parenthesized(ref content) => {
                let content = content.as_ref();
                // Find the colon separator between operator and list
                if let Some((op_str, items_str)) = lang::split_once_top_level(content, ':') {
                    let tokens: Vec<&str> = op_str
                        .split(',')
                        .map(|t| t.trim())
                        .filter(|t| !t.is_empty())
                        .collect();
                    let (modifier_tokens, op_token) = if tokens.len() > 1 {
                        (
                            tokens[..tokens.len() - 1].to_vec(),
                            tokens.last().copied().unwrap(),
                        )
                    } else {
                        (Vec::new(), op_str.trim())
                    };

                    let mut modifiers: Vec<ReductionModifier> = Vec::new();
                    let mut modifier_items: Vec<Vec<ClauseItem>> = Vec::new();
                    for m in modifier_tokens {
                        let m_trim = m.trim();
                        if m_trim.starts_with("original") {
                            modifiers.push(ReductionModifier::Original);
                            if let Some(start) = m_trim.find('(') {
                                if let Some(end) = m_trim.rfind(')') {
                                    if end > start + 1 {
                                        let inner = &m_trim[start + 1..end];
                                        let items = parse_identifier_list(inner, config)?;
                                        modifier_items.push(items);
                                        continue;
                                    }
                                }
                            }
                            modifier_items.push(Vec::new());
                        } else {
                            let maybe = match m_trim {
                                "task" => Some(ReductionModifier::Task),
                                "inscan" => Some(ReductionModifier::Inscan),
                                "default" => Some(ReductionModifier::Default),
                                _ => None,
                            };
                            if let Some(modifier) = maybe {
                                modifiers.push(modifier);
                                modifier_items.push(Vec::new());
                            }
                        }
                    }

                    let operator = parse_reduction_operator(op_token)?;
                    let user_identifier = match operator {
                        ReductionOperator::Custom => Some(Identifier::new(op_token)),
                        _ => None,
                    };
                    let items = parse_identifier_list(items_str.trim(), config)?;
                    let space_after_colon = items_str.starts_with(' ');
                    Ok(ClauseData::Reduction {
                        modifiers,
                        modifier_items,
                        operator,
                        user_identifier,
                        items,
                        space_after_colon,
                    })
                } else {
                    Err(ConversionError::InvalidClauseSyntax(
                        "reduction clause requires 'operator: list' format".to_string(),
                    ))
                }
            }
            ClauseKind::ReductionClause {
                modifiers,
                modifier_items,
                operator,
                user_defined_identifier,
                variables,
                space_after_colon,
            } => {
                let op_text = match operator {
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
                let operator = parse_reduction_operator(op_text.trim())?;
                let mut user_identifier = user_defined_identifier
                    .as_ref()
                    .map(|id| Identifier::new(id.as_ref()));
                if matches!(operator, ReductionOperator::Custom) && user_identifier.is_none() {
                    user_identifier = Some(Identifier::new(op_text.trim()));
                }
                let items = variables
                    .iter()
                    .map(|item| ClauseItem::Identifier(Identifier::new(item.as_ref())))
                    .collect();
                let mapped_modifiers: Vec<ReductionModifier> =
                    modifiers.iter().map(|m| (*m).into()).collect();
                let mapped_modifier_items: Vec<Vec<ClauseItem>> = modifier_items
                    .iter()
                    .map(|list| {
                        list.iter()
                            .map(|item| ClauseItem::Identifier(Identifier::new(item.as_str())))
                            .collect()
                    })
                    .collect();

                Ok(ClauseData::Reduction {
                    modifiers: mapped_modifiers,
                    modifier_items: mapped_modifier_items,
                    operator,
                    user_identifier,
                    items,
                    space_after_colon: *space_after_colon,
                })
            }
            _ => Err(ConversionError::InvalidClauseSyntax(
                "reduction clause requires parenthesized content".to_string(),
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

        // depend(dependence-type: list) or depend(source) or depend(sink)
        ClauseName::Depend => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let mut remaining = content.as_ref().trim();
                let mut iterators = Vec::new();

                if let Some((iterator_content, rest)) = extract_iterator_block(remaining) {
                    iterators = parse_iterator_block(&iterator_content, config)?;
                    remaining = rest.trim_start();
                    if remaining.starts_with(',') {
                        remaining = remaining[1..].trim_start();
                    }
                }

                // Find the colon separator using top-level detection
                if let Some((type_str, items_str)) = lang::split_once_top_level(remaining, ':') {
                    // Parse the dependence type
                    let depend_type = parse_depend_type(type_str.trim())?;

                    // Parse the item list
                    let items = parse_identifier_list(items_str.trim(), config)?;

                    Ok(ClauseData::Depend {
                        depend_type,
                        items,
                        iterators,
                    })
                } else {
                    // No colon found - could be depend(source) or depend(sink) without items
                    let type_str = remaining.trim();
                    let depend_type = parse_depend_type(type_str)?;

                    // Empty items list for source/sink without variables
                    Ok(ClauseData::Depend {
                        depend_type,
                        items: vec![],
                        iterators,
                    })
                }
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "depend clause requires parenthesized content".to_string(),
                ))
            }
        }

        // doacross(source|sink : deps)
        ClauseName::Doacross => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let inner = content.as_ref();
                let (kind_text, rest) = match inner.split_once(':') {
                    Some(parts) => (parts.0.trim(), parts.1.trim()),
                    None => (inner.trim(), ""),
                };
                let kind = match kind_text.to_ascii_lowercase().as_str() {
                    "source" => DoacrossType::Source,
                    "sink" => DoacrossType::Sink,
                    "" => {
                        return Err(ConversionError::InvalidClauseSyntax(
                            "doacross clause requires source or sink".to_string(),
                        ))
                    }
                    other => {
                        return Err(ConversionError::InvalidClauseSyntax(format!(
                            "unknown doacross kind: {other}"
                        )))
                    }
                };
                let items = if rest.is_empty() {
                    Vec::new()
                } else {
                    split_top_level_items(rest)
                        .into_iter()
                        .map(|s| ClauseItem::Expression(Expression::new(s.trim(), config)))
                        .collect()
                };
                Ok(ClauseData::Doacross { kind, items })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "doacross clause requires parenthesized content".to_string(),
                ))
            }
        }

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

        // bind(parallel|teams|thread|user)
        ClauseName::Bind => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let kind_str = content.as_ref().trim().to_ascii_lowercase();
                let binding = match kind_str.as_str() {
                    "teams" => BindModifier::Teams,
                    "parallel" => BindModifier::Parallel,
                    "thread" => BindModifier::Thread,
                    "user" => BindModifier::User,
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
                let kind_str = content.trim().to_ascii_lowercase();
                let proc_bind = match kind_str {
                    ref s if s == "master" => ProcBind::Master,
                    ref s if s == "close" => ProcBind::Close,
                    ref s if s == "spread" => ProcBind::Spread,
                    ref s if s == "primary" => ProcBind::Primary,
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
                    if let Some((modifier, rest)) = lang::split_once_top_level(content, ':') {
                        (Some(modifier.trim()), rest)
                    } else {
                        (None, content)
                    };

                let modifier = match modifier {
                    Some("") => None,
                    Some("conditional") => Some(LastprivateModifier::Conditional),
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
                Ok(ClauseData::NumTeams {
                    num: Expression::new(content.as_ref().trim(), config),
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
                Ok(ClauseData::ThreadLimit {
                    limit: Expression::new(content.as_ref().trim(), config),
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
                    if let Some((items, alignment)) = lang::split_once_top_level(content, ':') {
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
                            Some(Expression::new(value.trim(), config))
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
                    length: Expression::new(content.as_ref().trim(), config),
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
                    length: Expression::new(content.as_ref().trim(), config),
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "simdlen clause requires parenthesized expression".to_string(),
                ))
            }
        }

        // in_reduction/task_reduction share the reduction parser
        ClauseName::InReduction | ClauseName::TaskReduction => match &clause.kind {
            ClauseKind::Parenthesized(ref content) => {
                let content = content.as_ref();
                if let Some((op_str, items_str)) = lang::split_once_top_level(content, ':') {
                    let operator = parse_reduction_operator(op_str.trim())?;
                    let items = parse_identifier_list(items_str.trim(), config)?;
                    let space_after_colon = items_str.starts_with(' ');
                    Ok(ClauseData::Reduction {
                        modifiers: Vec::new(),
                        modifier_items: Vec::new(),
                        operator,
                        user_identifier: None,
                        items,
                        space_after_colon,
                    })
                } else {
                    Err(ConversionError::InvalidClauseSyntax(
                        "reduction-style clauses require 'operator: list' syntax".to_string(),
                    ))
                }
            }
            ClauseKind::ReductionClause {
                modifiers,
                operator,
                user_defined_identifier,
                variables,
                space_after_colon,
                modifier_items: _,
            } => {
                let op_text = match operator {
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
                let operator = parse_reduction_operator(op_text.trim())?;
                let mut user_identifier = user_defined_identifier
                    .as_ref()
                    .map(|id| Identifier::new(id.as_ref()));
                if matches!(operator, ReductionOperator::Custom) && user_identifier.is_none() {
                    user_identifier = Some(Identifier::new(op_text.trim()));
                }
                let items = variables
                    .iter()
                    .map(|item| ClauseItem::Identifier(Identifier::new(item.as_ref())))
                    .collect();
                Ok(ClauseData::Reduction {
                    modifiers: modifiers.iter().map(|m| (*m).into()).collect(),
                    modifier_items: Vec::new(),
                    operator,
                    user_identifier,
                    items,
                    space_after_colon: *space_after_colon,
                })
            }
            _ => Err(ConversionError::InvalidClauseSyntax(
                "reduction-style clauses require parenthesized content".to_string(),
            )),
        },

        // dist_schedule(kind[, chunk])
        ClauseName::DistSchedule => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let parts: Vec<&str> = content
                    .as_ref()
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect();
                if parts.is_empty() {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "dist_schedule requires a schedule kind".to_string(),
                    ));
                }
                let kind = match parts[0] {
                    "static" => ScheduleKind::Static,
                    "dynamic" => ScheduleKind::Dynamic,
                    "guided" => ScheduleKind::Guided,
                    other => {
                        return Err(ConversionError::InvalidClauseSyntax(format!(
                            "Unknown dist_schedule kind: {other}"
                        )))
                    }
                };
                let chunk_size = parts
                    .get(1)
                    .map(|value| Expression::new(value.trim(), config));
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
                let mut modifier = GrainsizeModifier::Unspecified;
                let mut expr_text = trimmed;

                if let Some(rest) = trimmed.strip_prefix("strict") {
                    let after = rest.trim_start();
                    if let Some(after_colon) = after.strip_prefix(':') {
                        modifier = GrainsizeModifier::Strict;
                        expr_text = after_colon.trim_start();
                    }
                }

                Ok(ClauseData::Grainsize {
                    modifier,
                    grain: Expression::new(expr_text, config),
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
                let mut modifier = NumTasksModifier::Unspecified;
                let mut expr_text = trimmed;

                if let Some(rest) = trimmed.strip_prefix("strict") {
                    let after = rest.trim_start();
                    if let Some(after_colon) = after.strip_prefix(':') {
                        modifier = NumTasksModifier::Strict;
                        expr_text = after_colon.trim_start();
                    }
                }

                Ok(ClauseData::NumTasks {
                    modifier,
                    num: Expression::new(expr_text, config),
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "num_tasks clause requires parenthesized expression".to_string(),
                ))
            }
        }

        // adjust_args([modifier:] expr-list) / append_args([modifier:] expr-list)
        ClauseName::AdjustArgs | ClauseName::AppendArgs => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let text = content.as_ref().trim();
                let (modifier_text, args_text) =
                    if let Some((mod_part, rest)) = lang::split_once_top_level(text, ':') {
                        (mod_part.trim(), Some(rest.trim()))
                    } else {
                        ("", Some(text))
                    };

                let (modifier, custom_modifier) = match modifier_text.to_ascii_lowercase().as_str()
                {
                    "" => (AdjustArgsModifier::Unspecified, None),
                    "need_device_ptr" => (AdjustArgsModifier::NeedDevicePtr, None),
                    _ => (
                        AdjustArgsModifier::Custom,
                        Some(Identifier::new(modifier_text)),
                    ),
                };

                let mut arguments = Vec::new();
                if let Some(args_part) = args_text {
                    for entry in split_top_level_items(args_part) {
                        let expr_text = entry.trim();
                        if expr_text.is_empty() {
                            continue;
                        }
                        arguments.push(Expression::new(expr_text, config));
                    }
                }

                Ok(ClauseData::AdjustArgs {
                    modifier,
                    custom_modifier,
                    arguments,
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(format!(
                    "{clause_name} clause requires parenthesized content"
                )))
            }
        }

        // apply([label:] transform-list)
        ClauseName::Apply => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let (label, transforms, comma_separated) =
                    parse_apply_clause(content.as_ref().trim())?;
                Ok(ClauseData::Apply {
                    label,
                    transforms,
                    comma_separated,
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "apply clause requires parenthesized content".to_string(),
                ))
            }
        }

        // collector(expression)
        ClauseName::Collector => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                Ok(ClauseData::Collector {
                    expression: Expression::unparsed(content.as_ref().trim()),
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "collector clause requires parenthesized expression".to_string(),
                ))
            }
        }

        // induction(step(...), [label:]expr, ...)
        ClauseName::Induction => parse_induction_clause(&clause.kind, config),

        // filter(expression)
        ClauseName::Filter => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                Ok(ClauseData::Filter {
                    thread_num: Expression::new(content.as_ref().trim(), config),
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "filter clause requires parenthesized expression".to_string(),
                ))
            }
        }

        // affinity(list)
        ClauseName::Affinity => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let mut modifier = AffinityModifier::Unspecified;
                let mut iterators = Vec::new();
                let mut remaining = content.as_ref().trim();

                if let Some((iterator_content, rest)) = extract_iterator_block(remaining) {
                    modifier = AffinityModifier::Iterator;
                    iterators = parse_iterator_block(&iterator_content, config)?;
                    remaining = rest.trim_start();
                    if remaining.starts_with(':') {
                        remaining = remaining[1..].trim_start();
                    }
                }

                let items = parse_identifier_list(remaining, config)?;
                Ok(ClauseData::Affinity {
                    modifier,
                    iterators,
                    items,
                })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "affinity clause requires a variable list".to_string(),
                ))
            }
        }

        // depobj_update(kind)
        ClauseName::DepobjUpdate => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let dep = match content.as_ref().trim() {
                    "in" => DepobjUpdateDependence::In,
                    "out" => DepobjUpdateDependence::Out,
                    "inout" => DepobjUpdateDependence::Inout,
                    "inoutset" => DepobjUpdateDependence::Inoutset,
                    "mutexinoutset" => DepobjUpdateDependence::Mutexinoutset,
                    "depobj" => DepobjUpdateDependence::Depobj,
                    "sink" => DepobjUpdateDependence::Sink,
                    "source" => DepobjUpdateDependence::Source,
                    other => {
                        return Err(ConversionError::InvalidClauseSyntax(format!(
                            "Unknown depobj_update dependence: {other}"
                        )))
                    }
                };
                Ok(ClauseData::DepobjUpdate { dependence: dep })
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "depobj_update clause requires parenthesized content".to_string(),
                ))
            }
        }

        // priority(expression)
        ClauseName::Priority => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                Ok(ClauseData::Priority {
                    priority: Expression::new(content.as_ref().trim(), config),
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
                Ok(ClauseData::Expression(Expression::new(
                    content.as_ref().trim(),
                    config,
                )))
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
                let device_type = match value {
                    "host" => DeviceType::Host,
                    "nohost" => DeviceType::Nohost,
                    "any" => DeviceType::Any,
                    other => {
                        return Err(ConversionError::InvalidClauseSyntax(format!(
                            "Unknown device_type value: {other}"
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
        ClauseName::At => parse_at_clause(&clause.kind),

        // severity(fatal|warning) for error directive
        ClauseName::Severity => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let value = content.as_ref().trim().to_ascii_lowercase();
                let kind = match value.as_str() {
                    "fatal" => SeverityKind::Fatal,
                    "warning" => SeverityKind::Warning,
                    "" => {
                        return Err(ConversionError::InvalidClauseSyntax(
                            "severity clause requires a value".to_string(),
                        ))
                    }
                    other => {
                        return Err(ConversionError::InvalidClauseSyntax(format!(
                            "Unknown severity value: {other}"
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

        // init([kind][:operand]) for interop
        ClauseName::Init => parse_init_clause(&clause.kind, config),

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

        // allocate([allocator:] list)
        ClauseName::Allocate => {
            if let ClauseKind::Parenthesized(ref content) = clause.kind {
                let content = content.as_ref();
                let (allocator_part, list_part) =
                    if let Some((alloc, rest)) = lang::split_once_top_level(content, ':') {
                        (Some(alloc.trim()), rest)
                    } else {
                        (None, content)
                    };
                let allocator = allocator_part
                    .filter(|value| !value.is_empty())
                    .map(classify_allocator_name);
                let items = parse_identifier_list(list_part.trim(), config)?;
                Ok(ClauseData::Allocate { allocator, items })
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
                if content.contains(':') {
                    return Err(ConversionError::InvalidClauseSyntax(
                        "allocator clause must not contain ':' separators".to_string(),
                    ));
                }
                Ok(ClauseData::Allocator {
                    allocator: classify_allocator_name(content),
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
                    if let Some((modifier_str, rest)) = lang::split_once_top_level(value, ':') {
                        let modifier = match modifier_str.trim().to_ascii_lowercase().as_str() {
                            "" => OrderModifier::Unspecified,
                            "reproducible" => OrderModifier::Reproducible,
                            "unconstrained" => OrderModifier::Unconstrained,
                            other => {
                                return Err(ConversionError::InvalidClauseSyntax(format!(
                                    "Unknown order modifier: {other}"
                                )))
                            }
                        };
                        (modifier, rest.trim())
                    } else {
                        (OrderModifier::Unspecified, value)
                    };

                match kind_str.to_ascii_lowercase().as_str() {
                    "concurrent" => Ok(ClauseData::Order {
                        modifier,
                        kind: OrderKind::Concurrent,
                    }),
                    other => Err(ConversionError::InvalidClauseSyntax(format!(
                        "Unknown order value: {other}"
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
                let order = match content.as_ref().trim() {
                    "seq_cst" => MemoryOrder::SeqCst,
                    "acq_rel" => MemoryOrder::AcqRel,
                    "release" => MemoryOrder::Release,
                    "acquire" => MemoryOrder::Acquire,
                    "relaxed" => MemoryOrder::Relaxed,
                    other => {
                        return Err(ConversionError::InvalidClauseSyntax(format!(
                            "Unknown atomic default memory order: {other}"
                        )))
                    }
                };
                Ok(ClauseData::AtomicDefaultMemOrder(order))
            } else {
                Err(ConversionError::InvalidClauseSyntax(
                    "atomic_default_mem_order clause requires parenthesized value".to_string(),
                ))
            }
        }

        // atomic operation clauses (read/write/update/capture)
        ClauseName::Read => Ok(ClauseData::AtomicOperation {
            op: AtomicOp::Read,
            memory_order: None,
        }),
        ClauseName::Write => Ok(ClauseData::AtomicOperation {
            op: AtomicOp::Write,
            memory_order: None,
        }),
        ClauseName::Update => match &clause.kind {
            ClauseKind::Bare => Ok(ClauseData::AtomicOperation {
                op: AtomicOp::Update,
                memory_order: None,
            }),
            _ => Err(ConversionError::InvalidClauseSyntax(
                "update clause does not accept arguments outside depobj".to_string(),
            )),
        },
        ClauseName::Capture => Ok(ClauseData::AtomicOperation {
            op: AtomicOp::Capture,
            memory_order: None,
        }),

        // branch hints and SIMD modifiers
        ClauseName::Nontemporal | ClauseName::Uniform => match &clause.kind {
            ClauseKind::Parenthesized(content) => {
                let items = parse_identifier_list(content.as_ref(), config)?;
                Ok(ClauseData::ItemList(items))
            }
            ClauseKind::VariableList(vars) => {
                let joined = vars.join(", ");
                let items = parse_identifier_list(&joined, config)?;
                Ok(ClauseData::ItemList(items))
            }
            ClauseKind::Bare => Ok(ClauseData::ItemList(Vec::new())),
            _ => Err(ConversionError::InvalidClauseSyntax(format!(
                "{clause_name} clause requires a variable list"
            ))),
        },
        ClauseName::Inbranch => Ok(ClauseData::Bare(Identifier::new("inbranch"))),
        ClauseName::Notinbranch => Ok(ClauseData::Bare(Identifier::new("notinbranch"))),
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
                        parse_memory_order(trimmed)?
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
            let directives: Vec<crate::parser::directive_kind::DirectiveName> =
                split_top_level_items(content)
                    .into_iter()
                    .map(str::trim)
                    .filter(|token| !token.is_empty())
                    .map(crate::parser::directive_kind::lookup_directive_name)
                    .collect();

            match clause_kind {
                ClauseName::Absent => Ok(ClauseData::Absent { directives }),
                ClauseName::Contains => Ok(ClauseData::Contains { directives }),
                _ => unreachable!("clause_kind filtered for absent/contains"),
            }
        }

        // requires(...) with modifiers
        ClauseName::Requires => parse_requires_clause(&clause.kind, config),

        ClauseName::ReverseOffload
        | ClauseName::UnifiedAddress
        | ClauseName::UnifiedSharedMemory
        | ClauseName::DynamicAllocators
        | ClauseName::SelfMaps => {
            let requirement = match clause_kind {
                ClauseName::ReverseOffload => RequireModifier::ReverseOffload,
                ClauseName::UnifiedAddress => RequireModifier::UnifiedAddress,
                ClauseName::UnifiedSharedMemory => RequireModifier::UnifiedSharedMemory,
                ClauseName::DynamicAllocators => RequireModifier::DynamicAllocators,
                ClauseName::SelfMaps => RequireModifier::SelfMaps,
                _ => unreachable!("requirement clause group is exhaustive"),
            };
            Ok(ClauseData::Requires {
                requirements: vec![requirement],
            })
        }

        ClauseName::ExtImplementationDefinedRequirement => match &clause.kind {
            ClauseKind::Bare => Ok(ClauseData::Requires {
                requirements: vec![RequireModifier::ExtImplementationDefinedRequirement(None)],
            }),
            ClauseKind::Parenthesized(content) => {
                let value = content.as_ref().trim();
                Ok(ClauseData::Requires {
                    requirements: vec![RequireModifier::ExtImplementationDefinedRequirement(
                        (!value.is_empty()).then(|| Identifier::new(value)),
                    )],
                })
            }
            ClauseKind::VariableList(items) => {
                let value = items
                    .first()
                    .map(|item| item.as_ref().trim())
                    .filter(|item| !item.is_empty())
                    .map(Identifier::new);
                Ok(ClauseData::Requires {
                    requirements: vec![RequireModifier::ExtImplementationDefinedRequirement(
                        value,
                    )],
                })
            }
            _ => Err(ConversionError::InvalidClauseSyntax(
                "ext_implementation_defined_requirement clause expects bare or parenthesized content"
                    .to_string(),
            )),
        },

        ClauseName::Compare
        | ClauseName::CompareCapture
        | ClauseName::Full
        | ClauseName::Threads
        | ClauseName::Simd
        | ClauseName::Weak
        | ClauseName::InitComplete
        | ClauseName::NoParallelism => match &clause.kind {
            ClauseKind::Bare => Ok(ClauseData::Bare(Identifier::new(clause_name))),
            _ => Err(ConversionError::InvalidClauseSyntax(format!(
                "{clause_name} clause does not take arguments"
            ))),
        },

        ClauseName::Align
        | ClauseName::Destroy
        | ClauseName::Final
        | ClauseName::Partial
        | ClauseName::Initializer
        | ClauseName::Message
        | ClauseName::Holds
        | ClauseName::GraphId
        | ClauseName::GraphReset
        | ClauseName::Threadset
        | ClauseName::Transparent
        | ClauseName::Replayable
        | ClauseName::Detach
        | ClauseName::Indirect
        | ClauseName::Safesync
        | ClauseName::DeviceSafesync
        | ClauseName::Memscope
        | ClauseName::Looprange
        | ClauseName::Permutation
        | ClauseName::Counts
        | ClauseName::Inductor
        | ClauseName::Combiner
        | ClauseName::NoOpenmp
        | ClauseName::NoOpenmpConstructs
        | ClauseName::NoOpenmpRoutines
        | ClauseName::Nocontext
        | ClauseName::Novariants => match &clause.kind {
            ClauseKind::Parenthesized(content) => Ok(ClauseData::Expression(Expression::new(
                content.as_ref().trim(),
                config,
            ))),
            ClauseKind::Bare => Ok(ClauseData::Bare(Identifier::new(clause_name))),
            ClauseKind::VariableList(items) => {
                let joined = items.join(", ");
                Ok(ClauseData::Expression(Expression::new(joined.trim(), config)))
            }
            _ => Err(ConversionError::InvalidClauseSyntax(format!(
                "{clause_name} clause expects bare, parenthesized, or variable-list content"
            ))),
        },

        ClauseName::Use => match &clause.kind {
            ClauseKind::Parenthesized(content) => {
                let items = parse_identifier_list(content.as_ref(), config)?;
                Ok(ClauseData::ItemList(items))
            }
            ClauseKind::VariableList(items) => {
                let joined = items.join(", ");
                let items = parse_identifier_list(&joined, config)?;
                Ok(ClauseData::ItemList(items))
            }
            ClauseKind::Bare => Ok(ClauseData::ItemList(Vec::new())),
            _ => Err(ConversionError::InvalidClauseSyntax(
                "use clause expects a variable list".to_string(),
            )),
        },

        _ => Err(ConversionError::UnknownClause(format!(
            "{clause_name:?} ({:?})",
            clause.kind
        ))),
    }
}
