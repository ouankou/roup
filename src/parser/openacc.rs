use std::borrow::Cow;

use super::{
    ClauseRegistry, ClauseRegistryBuilder, ClauseRule, DirectiveRegistry, DirectiveRegistryBuilder,
    Parser,
};

const OPENACC_DEFAULT_CLAUSE_RULE: ClauseRule = ClauseRule::Flexible;

macro_rules! openacc_clauses {
    ($( $variant:ident => { name: $name:literal, rule: $rule:expr } ),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum OpenAccClause {
            $( $variant, )+
        }

        impl OpenAccClause {
            pub const ALL: &'static [OpenAccClause] = &[ $( OpenAccClause::$variant, )+ ];

            pub const fn name(self) -> &'static str {
                match self {
                    $( OpenAccClause::$variant => $name, )+
                }
            }

            pub const fn rule(self) -> ClauseRule {
                match self {
                    $( OpenAccClause::$variant => $rule, )+
                }
            }
        }
    };
}

openacc_clauses! {
    Async => { name: "async", rule: ClauseRule::Flexible },
    Attach => { name: "attach", rule: ClauseRule::Parenthesized },
    Auto => { name: "auto", rule: ClauseRule::Bare },
    Bind => { name: "bind", rule: ClauseRule::Flexible },
    Capture => { name: "capture", rule: ClauseRule::Bare },
    Collapse => { name: "collapse", rule: ClauseRule::Parenthesized },
    Copy => { name: "copy", rule: ClauseRule::Parenthesized },
    PCopy => { name: "pcopy", rule: ClauseRule::Parenthesized },
    PresentOrCopy => { name: "present_or_copy", rule: ClauseRule::Parenthesized },
    Copyin => { name: "copyin", rule: ClauseRule::Parenthesized },
    PCopyIn => { name: "pcopyin", rule: ClauseRule::Parenthesized },
    PresentOrCopyIn => { name: "present_or_copyin", rule: ClauseRule::Parenthesized },
    Copyout => { name: "copyout", rule: ClauseRule::Parenthesized },
    PCopyOut => { name: "pcopyout", rule: ClauseRule::Parenthesized },
    PresentOrCopyOut => { name: "present_or_copyout", rule: ClauseRule::Parenthesized },
    Create => { name: "create", rule: ClauseRule::Parenthesized },
    PCreate => { name: "pcreate", rule: ClauseRule::Parenthesized },
    PresentOrCreate => { name: "present_or_create", rule: ClauseRule::Parenthesized },
    Default => { name: "default", rule: ClauseRule::Parenthesized },
    DefaultAsync => { name: "default_async", rule: ClauseRule::Parenthesized },
    Delete => { name: "delete", rule: ClauseRule::Parenthesized },
    Detach => { name: "detach", rule: ClauseRule::Parenthesized },
    Device => { name: "device", rule: ClauseRule::Parenthesized },
    DeviceNum => { name: "device_num", rule: ClauseRule::Parenthesized },
    DeviceResident => { name: "device_resident", rule: ClauseRule::Parenthesized },
    DeviceType => { name: "device_type", rule: ClauseRule::Flexible },
    DType => { name: "dtype", rule: ClauseRule::Flexible },
    Deviceptr => { name: "deviceptr", rule: ClauseRule::Parenthesized },
    Finalize => { name: "finalize", rule: ClauseRule::Bare },
    Firstprivate => { name: "firstprivate", rule: ClauseRule::Parenthesized },
    Gang => { name: "gang", rule: ClauseRule::Flexible },
    Host => { name: "host", rule: ClauseRule::Parenthesized },
    If => { name: "if", rule: ClauseRule::Parenthesized },
    IfPresent => { name: "if_present", rule: ClauseRule::Bare },
    Independent => { name: "independent", rule: ClauseRule::Bare },
    Link => { name: "link", rule: ClauseRule::Parenthesized },
    NoCreate => { name: "no_create", rule: ClauseRule::Parenthesized },
    Nohost => { name: "nohost", rule: ClauseRule::Bare },
    NumGangs => { name: "num_gangs", rule: ClauseRule::Parenthesized },
    NumWorkers => { name: "num_workers", rule: ClauseRule::Parenthesized },
    Present => { name: "present", rule: ClauseRule::Parenthesized },
    Private => { name: "private", rule: ClauseRule::Parenthesized },
    Reduction => { name: "reduction", rule: ClauseRule::Parenthesized },
    Read => { name: "read", rule: ClauseRule::Bare },
    SelfClause => { name: "self", rule: ClauseRule::Flexible },
    Seq => { name: "seq", rule: ClauseRule::Bare },
    Tile => { name: "tile", rule: ClauseRule::Parenthesized },
    Update => { name: "update", rule: ClauseRule::Flexible },
    UseDevice => { name: "use_device", rule: ClauseRule::Parenthesized },
    Vector => { name: "vector", rule: ClauseRule::Flexible },
    VectorLength => { name: "vector_length", rule: ClauseRule::Parenthesized },
    Wait => { name: "wait", rule: ClauseRule::Flexible },
    Worker => { name: "worker", rule: ClauseRule::Flexible },
    Write => { name: "write", rule: ClauseRule::Bare },
}

macro_rules! openacc_directives {
    ($( $variant:ident => $name:literal ),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum OpenAccDirective {
            $( $variant, )+
        }

        impl OpenAccDirective {
            pub const ALL: &'static [OpenAccDirective] = &[ $( OpenAccDirective::$variant, )+ ];

            pub const fn as_str(self) -> &'static str {
                match self {
                    $( OpenAccDirective::$variant => $name, )+
                }
            }
        }
    };
}

openacc_directives! {
    Atomic => "atomic",
    Cache => "cache",
    Data => "data",
    Declare => "declare",
    End => "end",
    EnterData => "enter data",
    EnterDataUnderscore => "enter_data",
    ExitData => "exit data",
    ExitDataUnderscore => "exit_data",
    HostData => "host_data",
    HostDataSpace => "host data",
    Init => "init",
    Kernels => "kernels",
    KernelsLoop => "kernels loop",
    Loop => "loop",
    Parallel => "parallel",
    ParallelLoop => "parallel loop",
    Routine => "routine",
    Serial => "serial",
    SerialLoop => "serial loop",
    Set => "set",
    Shutdown => "shutdown",
    Update => "update",
    Wait => "wait",
}

pub fn clause_registry() -> ClauseRegistry {
    let mut builder = ClauseRegistryBuilder::new().with_default_rule(OPENACC_DEFAULT_CLAUSE_RULE);

    for clause in OpenAccClause::ALL {
        // Skip clauses that will have custom parsers
        let name = clause.name();
        if matches!(
            name,
            "copyin" | "pcopyin" | "present_or_copyin" |
            "copyout" | "pcopyout" | "present_or_copyout" |
            "create" | "pcreate" | "present_or_create" |
            "reduction" |
            // Variable list clauses (need custom parsing to split comma-separated vars)
            "gang" | "worker" | "vector" | "wait" | "private" | "firstprivate" |
            "device_type" | "dtype" |
            "copy" | "pcopy" | "present_or_copy" | "present" | "attach" | "detach" |
            "use_device" | "link" | "device_resident" | "host" | "device" |
            "deviceptr" | "delete"
        ) {
            continue;
        }
        builder.register_with_rule_mut(name, clause.rule());
    }

    // Register custom parsers for clauses with modifiers/operators
    builder.register_with_rule_mut("copyin", ClauseRule::Custom(parse_copyin_clause));
    builder.register_with_rule_mut("pcopyin", ClauseRule::Custom(parse_copyin_clause));
    builder.register_with_rule_mut("present_or_copyin", ClauseRule::Custom(parse_copyin_clause));

    builder.register_with_rule_mut("copyout", ClauseRule::Custom(parse_copyout_clause));
    builder.register_with_rule_mut("pcopyout", ClauseRule::Custom(parse_copyout_clause));
    builder.register_with_rule_mut(
        "present_or_copyout",
        ClauseRule::Custom(parse_copyout_clause),
    );

    builder.register_with_rule_mut("create", ClauseRule::Custom(parse_create_clause));
    builder.register_with_rule_mut("pcreate", ClauseRule::Custom(parse_create_clause));
    builder.register_with_rule_mut("present_or_create", ClauseRule::Custom(parse_create_clause));

    builder.register_with_rule_mut("reduction", ClauseRule::Custom(parse_reduction_clause));

    // Register specialized parsers for gang/worker/vector with modifier support
    builder.register_with_rule_mut("gang", ClauseRule::Custom(parse_gang_clause));
    builder.register_with_rule_mut("worker", ClauseRule::Custom(parse_worker_clause));
    builder.register_with_rule_mut("vector", ClauseRule::Custom(parse_vector_clause));

    // Register variable list parsers for clauses that take comma-separated variable lists
    // These need custom parsing to split "a, b, c" into ["a", "b", "c"] for proper deduplication
    // NOTE: tile, collapse, num_gangs, and async are NOT included here because they take
    // parameter lists (literal values), not variable lists, and should not be merged
    let var_list_clauses = [
        "wait",
        "private",
        "firstprivate",
        "device_type",
        "dtype",
        "copy",
        "pcopy",
        "present_or_copy",
        "present",
        "attach",
        "detach",
        "use_device",
        "link",
        "device_resident",
        "host",
        "device",
        "deviceptr",
        "delete",
    ];
    for clause_name in &var_list_clauses {
        builder.register_with_rule_mut(clause_name, ClauseRule::Custom(parse_variable_list_clause));
    }

    builder.build()
}

fn parse_cache_directive<'a>(
    name: Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::{CacheDirectiveData, Directive};
    use crate::lexer;

    let (input, _) = lexer::skip_space_and_comments(input)?;
    let (rest_after_paren, content) = parse_parenthesized_content_inner(input)?;
    let (rest, clauses) = clause_registry.parse_sequence(rest_after_paren)?;

    // Parse cache directive: cache(readonly: x, y, z) or cache(x, y, z)
    let content_trimmed = content.trim();
    let (readonly, var_part) = if let Some(stripped) = content_trimmed.strip_prefix("readonly:") {
        (true, stripped.trim())
    } else if let Some(stripped) = content_trimmed.strip_prefix("readonly :") {
        (true, stripped.trim())
    } else {
        (false, content_trimmed)
    };

    // Parse variable list (parse_variable_list returns owned Cow values)
    let variables: Vec<Cow<'a, str>> = parse_variable_list(var_part)
        .into_iter()
        .map(|v| Cow::Owned(v.into_owned()))
        .collect();

    // For backward compat: keep normalized parameter
    let normalized = normalize_directive_parameter(content.trim());
    let parameter = format!("({})", normalized);

    Ok((
        rest,
        Directive {
            name,
            parameter: Some(Cow::Owned(parameter)),
            clauses,
            wait_data: None,
            cache_data: Some(CacheDirectiveData {
                readonly,
                variables,
            }),
        },
    ))
}

/// Normalize whitespace in wait/cache directive parameters
/// Format as "keyword: value": "devnum : 23 : queues: 1, 2" -> "devnum: 23: queues: 1, 2"
fn normalize_directive_parameter(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    let mut prev_char = ' ';
    let mut after_colon = false;

    for ch in s.chars() {
        if ch == ':' {
            // Remove trailing spaces before colon
            while result.ends_with(' ') {
                result.pop();
            }
            result.push(':');
            result.push(' '); // Always add one space after colon
            after_colon = true;
            prev_char = ':';
        } else if ch == ' ' || ch == '\t' {
            if after_colon {
                // Skip leading spaces after colon (we already added one)
                continue;
            }
            if prev_char != ' ' {
                result.push(' ');
                prev_char = ' ';
            }
        } else {
            after_colon = false;
            result.push(ch);
            prev_char = ch;
        }
    }

    result.trim().to_string()
}

fn parse_wait_directive<'a>(
    name: Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::{Directive, WaitDirectiveData};
    use crate::lexer;

    let (input, _) = lexer::skip_space_and_comments(input)?;
    if input.trim_start().starts_with('(') {
        let (rest, content) = parse_parenthesized_content_inner(input)?;
        let (rest, clauses) = clause_registry.parse_sequence(rest)?;

        // Parse wait directive: wait(devnum: 23: 1, 2, 3) or wait(devnum: 23: queues: 1, 2, 3) or wait(1, 2, 3)
        let content_str = content.as_str().trim();
        let mut devnum = None;
        let mut has_queues = false;
        let mut queue_exprs = Vec::new();

        if let Some(stripped) = content_str
            .strip_prefix("devnum:")
            .or_else(|| content_str.strip_prefix("devnum :"))
        {
            // Parse: "devnum: 23" or "devnum: 23: queues: ..."
            let after_devnum = stripped.trim();

            if let Some(colon_pos) = after_devnum.find(':') {
                // Has queue list or additional content after devnum
                devnum = Some(Cow::Owned(after_devnum[..colon_pos].trim().to_string()));
                let rest = after_devnum[colon_pos + 1..].trim();

                if let Some(queues_stripped) = rest
                    .strip_prefix("queues:")
                    .or_else(|| rest.strip_prefix("queues :"))
                {
                    has_queues = true;
                    queue_exprs = parse_variable_list(queues_stripped.trim());
                } else {
                    queue_exprs = parse_variable_list(rest);
                }
            } else {
                // Only devnum, no queue list
                devnum = Some(Cow::Owned(after_devnum.to_string()));
            }
        } else if let Some(stripped) = content_str.strip_prefix("queues:") {
            // Parse: "queues: 1, 2, 3"
            has_queues = true;
            queue_exprs = parse_variable_list(stripped.trim());
        } else if let Some(stripped) = content_str.strip_prefix("queues :") {
            // Parse: "queues : 1, 2, 3"
            has_queues = true;
            queue_exprs = parse_variable_list(stripped.trim());
        } else if !content_str.is_empty() {
            // Parse: "1, 2, 3"
            queue_exprs = parse_variable_list(content_str);
        }

        // Ensure all queue_exprs are owned (not borrowed from content)
        let queue_exprs_owned: Vec<Cow<'a, str>> = queue_exprs
            .into_iter()
            .map(|v| Cow::Owned(v.into_owned()))
            .collect();

        // For backward compat: keep normalized parameter
        let normalized = normalize_directive_parameter(content.trim());
        let parameter = format!("({})", normalized);

        return Ok((
            rest,
            Directive {
                name,
                parameter: Some(Cow::Owned(parameter)),
                clauses,
                wait_data: Some(WaitDirectiveData {
                    devnum,
                    has_queues,
                    queue_exprs: queue_exprs_owned,
                }),
                cache_data: None,
            },
        ));
    }

    let (rest, clauses) = clause_registry.parse_sequence(input)?;
    Ok((
        rest,
        Directive {
            name,
            parameter: None,
            clauses,
            wait_data: None,
            cache_data: None,
        },
    ))
}

fn parse_end_directive<'a>(
    name: Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::Directive;
    use crate::lexer;

    let (mut rest, _) = lexer::skip_space_and_comments(input)?;

    // FIX: Read full multi-word directive name (e.g., "parallel loop") instead of just one token
    // Keep reading identifiers until we hit a non-identifier
    let mut directive_parts = Vec::new();
    while let Ok((new_rest, token)) = lexer::lex_identifier_token(rest) {
        let collapsed = lexer::collapse_line_continuations(token);
        directive_parts.push(collapsed.as_ref().to_string());
        rest = new_rest;

        // Skip whitespace and try again
        if let Ok((new_rest, _)) = lexer::skip_space_and_comments(rest) {
            rest = new_rest;
        }
    }

    // Store the directive being ended as a parameter
    let directive = directive_parts.join(" ");
    let (rest, clauses) = clause_registry.parse_sequence(rest)?;
    // Store the directive token without presentation spacing; rendering should add spaces.
    let parameter = directive.to_string();

    Ok((
        rest,
        Directive {
            name,
            parameter: Some(Cow::Owned(parameter)),
            clauses,
            wait_data: None,
            cache_data: None,
        },
    ))
}

fn parse_routine_directive<'a>(
    name: Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::Directive;
    use crate::lexer;

    let (input, _) = lexer::skip_space_and_comments(input)?;
    if input.trim_start().starts_with('(') {
        let (rest_after_paren, content) = parse_parenthesized_content_inner(input)?;
        let (rest, clauses) = clause_registry.parse_sequence(rest_after_paren)?;
        // Store routine name with parentheses for round-trip compatibility
        let parameter = format!("({})", content.trim());
        return Ok((
            rest,
            Directive {
                name,
                parameter: Some(Cow::Owned(parameter)),
                clauses,
                wait_data: None,
                cache_data: None,
            },
        ));
    }

    let (rest, clauses) = clause_registry.parse_sequence(input)?;
    Ok((
        rest,
        Directive {
            name,
            parameter: None,
            clauses,
            wait_data: None,
            cache_data: None,
        },
    ))
}

pub fn directive_registry() -> DirectiveRegistry {
    let mut builder = DirectiveRegistryBuilder::new();

    builder = builder.register_custom("cache", parse_cache_directive);
    builder = builder.register_custom("wait", parse_wait_directive);
    builder = builder.register_custom("end", parse_end_directive);
    builder = builder.register_custom("routine", parse_routine_directive);

    for directive in OpenAccDirective::ALL {
        let name = directive.as_str();
        if matches!(name, "cache" | "wait" | "end" | "routine") {
            continue;
        }
        builder = builder.register_generic(name);
    }

    builder.build()
}

pub fn parser() -> Parser {
    Parser::new(directive_registry(), clause_registry()).with_dialect(super::Dialect::OpenAcc)
}

fn parse_parenthesized_content_inner(input: &str) -> nom::IResult<&str, String> {
    use crate::lexer;
    use nom::bytes::complete::tag;
    use nom::error::{Error, ErrorKind};

    let (input, _) = lexer::skip_space_and_comments(input)?;
    let (input, _) = tag("(")(input)?;

    let mut depth = 1;
    let mut end_index = None;

    for (idx, ch) in input.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    end_index = Some(idx);
                    break;
                }
            }
            _ => {}
        }
    }

    let Some(end_idx) = end_index else {
        return Err(nom::Err::Error(Error::new(input, ErrorKind::Tag)));
    };

    let (content, rest) = input.split_at(end_idx);
    let rest = &rest[1..];

    Ok((rest, content.to_string()))
}

/// Parse a comma-separated list of variable names/expressions
/// Handles: m, n, p
///          x[0:N], y[1:10]
///          max::x, readonly::y (C++ scope resolution)
fn parse_variable_list(input: &str) -> Vec<Cow<'_, str>> {
    let mut variables = Vec::new();
    let mut current = String::new();
    let mut depth = 0; // Track parenthesis/bracket depth for array sections

    for ch in input.chars() {
        match ch {
            ',' if depth == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    variables.push(Cow::Owned(trimmed.to_string()));
                }
                current.clear();
            }
            '(' | '[' => {
                depth += 1;
                current.push(ch);
            }
            ')' | ']' => {
                depth -= 1;
                current.push(ch);
            }
            _ => {
                current.push(ch);
            }
        }
    }

    // Don't forget the last variable
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        variables.push(Cow::Owned(trimmed.to_string()));
    }

    variables
}

/// Parse copyin clause: copyin(readonly: m, n) or copyin(m, n)
fn parse_copyin_clause<'a>(
    name: Cow<'a, str>,
    input: &'a str,
) -> nom::IResult<&'a str, super::Clause<'a>> {
    use crate::lexer;
    use crate::parser::clause::{ClauseKind, CopyinModifier};
    use nom::bytes::complete::tag;

    let (input, _) = lexer::skip_space_and_comments(input)?;
    let (input, _) = tag("(")(input)?;
    let (input, _) = lexer::skip_space_and_comments(input)?;

    // Try to parse "readonly:" modifier
    let (input, modifier) = if input.trim_start().starts_with("readonly:")
        || input.trim_start().starts_with("readonly :")
    {
        // Find the colon
        let colon_idx = input.find(':').unwrap();
        let after_colon = &input[colon_idx + 1..];
        (after_colon, Some(CopyinModifier::Readonly))
    } else {
        (input, None)
    };

    // Parse until closing paren
    let (input, _) = lexer::skip_space_and_comments(input)?;
    let mut depth = 0;
    let mut end_idx = None;

    for (idx, ch) in input.char_indices() {
        match ch {
            '(' | '[' => depth += 1,
            ')' if depth == 0 => {
                end_idx = Some(idx);
                break;
            }
            ')' | ']' => depth -= 1,
            _ => {}
        }
    }

    let end_idx = end_idx.ok_or_else(|| {
        nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag))
    })?;

    let var_list_str = &input[..end_idx];
    let variables = parse_variable_list(var_list_str);
    let rest = &input[end_idx + 1..];

    Ok((
        rest,
        super::Clause {
            name,
            kind: ClauseKind::CopyinClause {
                modifier,
                variables,
            },
        },
    ))
}

/// Parse copyout clause: copyout(zero: x, y) or copyout(x, y)
fn parse_copyout_clause<'a>(
    name: Cow<'a, str>,
    input: &'a str,
) -> nom::IResult<&'a str, super::Clause<'a>> {
    use crate::lexer;
    use crate::parser::clause::{ClauseKind, CopyoutModifier};
    use nom::bytes::complete::tag;

    let (input, _) = lexer::skip_space_and_comments(input)?;
    let (input, _) = tag("(")(input)?;
    let (input, _) = lexer::skip_space_and_comments(input)?;

    // Try to parse "zero:" modifier
    let (input, modifier) =
        if input.trim_start().starts_with("zero:") || input.trim_start().starts_with("zero :") {
            let colon_idx = input.find(':').unwrap();
            let after_colon = &input[colon_idx + 1..];
            (after_colon, Some(CopyoutModifier::Zero))
        } else {
            (input, None)
        };

    // Parse until closing paren
    let (input, _) = lexer::skip_space_and_comments(input)?;
    let mut depth = 0;
    let mut end_idx = None;

    for (idx, ch) in input.char_indices() {
        match ch {
            '(' | '[' => depth += 1,
            ')' if depth == 0 => {
                end_idx = Some(idx);
                break;
            }
            ')' | ']' => depth -= 1,
            _ => {}
        }
    }

    let end_idx = end_idx.ok_or_else(|| {
        nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag))
    })?;

    let var_list_str = &input[..end_idx];
    let variables = parse_variable_list(var_list_str);
    let rest = &input[end_idx + 1..];

    Ok((
        rest,
        super::Clause {
            name,
            kind: ClauseKind::CopyoutClause {
                modifier,
                variables,
            },
        },
    ))
}

/// Parse create clause: create(zero: a, b) or create(a, b)
fn parse_create_clause<'a>(
    name: Cow<'a, str>,
    input: &'a str,
) -> nom::IResult<&'a str, super::Clause<'a>> {
    use crate::lexer;
    use crate::parser::clause::{ClauseKind, CreateModifier};
    use nom::bytes::complete::tag;

    let (input, _) = lexer::skip_space_and_comments(input)?;
    let (input, _) = tag("(")(input)?;
    let (input, _) = lexer::skip_space_and_comments(input)?;

    // Try to parse "zero:" modifier
    let (input, modifier) =
        if input.trim_start().starts_with("zero:") || input.trim_start().starts_with("zero :") {
            let colon_idx = input.find(':').unwrap();
            let after_colon = &input[colon_idx + 1..];
            (after_colon, Some(CreateModifier::Zero))
        } else {
            (input, None)
        };

    // Parse until closing paren
    let (input, _) = lexer::skip_space_and_comments(input)?;
    let mut depth = 0;
    let mut end_idx = None;

    for (idx, ch) in input.char_indices() {
        match ch {
            '(' | '[' => depth += 1,
            ')' if depth == 0 => {
                end_idx = Some(idx);
                break;
            }
            ')' | ']' => depth -= 1,
            _ => {}
        }
    }

    let end_idx = end_idx.ok_or_else(|| {
        nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag))
    })?;

    let var_list_str = &input[..end_idx];
    let variables = parse_variable_list(var_list_str);
    let rest = &input[end_idx + 1..];

    Ok((
        rest,
        super::Clause {
            name,
            kind: ClauseKind::CreateClause {
                modifier,
                variables,
            },
        },
    ))
}

/// Parse reduction clause: reduction(+: sum) or reduction(max: x, y, z)
fn parse_reduction_clause<'a>(
    name: Cow<'a, str>,
    input: &'a str,
) -> nom::IResult<&'a str, super::Clause<'a>> {
    use crate::lexer;
    use crate::parser::clause::{ClauseKind, ReductionOperator};
    use nom::bytes::complete::tag;

    let (input, _) = lexer::skip_space_and_comments(input)?;
    let (input, _) = tag("(")(input)?;
    let (input, _) = lexer::skip_space_and_comments(input)?;

    // Parse operator (everything before the first colon)
    let colon_idx = input.find(':').ok_or_else(|| {
        nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag))
    })?;

    let op_str = input[..colon_idx].trim();
    let operator = match op_str {
        "+" => ReductionOperator::Add,
        "-" => ReductionOperator::Sub,
        "*" => ReductionOperator::Mul,
        "max" => ReductionOperator::Max,
        "min" => ReductionOperator::Min,
        "&" => ReductionOperator::BitAnd,
        "|" => ReductionOperator::BitOr,
        "^" => ReductionOperator::BitXor,
        "&&" => ReductionOperator::LogAnd,
        "||" => ReductionOperator::LogOr,
        ".and." => ReductionOperator::FortAnd,
        ".or." => ReductionOperator::FortOr,
        ".eqv." => ReductionOperator::FortEqv,
        ".neqv." => ReductionOperator::FortNeqv,
        "iand" => ReductionOperator::FortIand,
        "ior" => ReductionOperator::FortIor,
        "ieor" => ReductionOperator::FortIeor,
        _ => {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Tag,
            )))
        }
    };

    let input = &input[colon_idx + 1..];

    // Check if there's a space after the colon (for formatting preservation)
    let space_after_colon = input.starts_with(|c: char| c.is_whitespace());

    // Parse until closing paren
    let (input, _) = lexer::skip_space_and_comments(input)?;
    let mut depth = 0;
    let mut end_idx = None;

    for (idx, ch) in input.char_indices() {
        match ch {
            '(' | '[' => depth += 1,
            ')' if depth == 0 => {
                end_idx = Some(idx);
                break;
            }
            ')' | ']' => depth -= 1,
            _ => {}
        }
    }

    let end_idx = end_idx.ok_or_else(|| {
        nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag))
    })?;

    let var_list_str = &input[..end_idx];
    let variables = parse_variable_list(var_list_str);
    let rest = &input[end_idx + 1..];

    Ok((
        rest,
        super::Clause {
            name,
            kind: ClauseKind::ReductionClause {
                operator,
                variables,
                space_after_colon,
            },
        },
    ))
}

/// Parse simple variable list clause: gang(a, b, c), wait(x, y), private(i, j), etc.
/// Also handles bare form: gang, wait, private
fn parse_variable_list_clause<'a>(
    name: Cow<'a, str>,
    input: &'a str,
) -> nom::IResult<&'a str, super::Clause<'a>> {
    use crate::lexer;
    use crate::parser::clause::ClauseKind;
    use nom::bytes::complete::tag;

    // Check if there are parentheses (peek, don't consume whitespace yet)
    let trimmed = input.trim_start();
    if trimmed.starts_with('(') {
        // Parenthesized form - now consume the whitespace properly
        let (input, _) = lexer::skip_space_and_comments(input)?;
        // Parenthesized form - parse variable list
        let (input, _) = tag("(")(input)?;
        let (input, _) = lexer::skip_space_and_comments(input)?;

        // Parse until closing paren
        let mut depth = 0;
        let mut end_idx = None;

        for (idx, ch) in input.char_indices() {
            match ch {
                '(' | '[' => depth += 1,
                ')' if depth == 0 => {
                    end_idx = Some(idx);
                    break;
                }
                ')' | ']' => depth -= 1,
                _ => {}
            }
        }

        let end_idx = end_idx.ok_or_else(|| {
            nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag))
        })?;

        let var_list_str = &input[..end_idx];
        let variables = parse_variable_list(var_list_str);
        let rest = &input[end_idx + 1..];

        Ok((
            rest,
            super::Clause {
                name,
                kind: ClauseKind::VariableList(variables),
            },
        ))
    } else {
        // Bare form - no variables
        Ok((
            input,
            super::Clause {
                name,
                kind: ClauseKind::Bare,
            },
        ))
    }
}

/// Parse gang clause: gang or gang(a, b, c) or gang(num: a, static: *)
/// Note: gang doesn't have formal modifiers in accparser, just variable list
fn parse_gang_clause<'a>(
    name: Cow<'a, str>,
    input: &'a str,
) -> nom::IResult<&'a str, super::Clause<'a>> {
    use crate::lexer;
    use crate::parser::clause::{ClauseKind, GangModifier};

    // Check if there are parentheses (peek, don't consume whitespace yet)
    let trimmed = input.trim_start();
    if trimmed.starts_with('(') {
        // Parenthesized form - now consume the whitespace properly
        let (input, _) = lexer::skip_space_and_comments(input)?;
        let (input, _) = nom::bytes::complete::tag("(")(input)?;
        let (input, _) = lexer::skip_space_and_comments(input)?;

        // Try to parse "num:" or "static:" modifier
        let (input, modifier) =
            if input.trim_start().starts_with("num:") || input.trim_start().starts_with("num :") {
                let colon_idx = input.find(':').unwrap();
                let after_colon = &input[colon_idx + 1..];
                (after_colon, Some(GangModifier::Num))
            } else if input.trim_start().starts_with("static:")
                || input.trim_start().starts_with("static :")
            {
                let colon_idx = input.find(':').unwrap();
                let after_colon = &input[colon_idx + 1..];
                (after_colon, Some(GangModifier::Static))
            } else {
                (input, None)
            };

        let (input, _) = lexer::skip_space_and_comments(input)?;

        // Parse until closing paren
        let mut depth = 0;
        let mut end_idx = None;

        for (idx, ch) in input.char_indices() {
            match ch {
                '(' | '[' => depth += 1,
                ')' if depth == 0 => {
                    end_idx = Some(idx);
                    break;
                }
                ')' | ']' => depth -= 1,
                _ => {}
            }
        }

        let end_idx = end_idx.ok_or_else(|| {
            nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag))
        })?;

        let var_list_str = &input[..end_idx];
        let variables = parse_variable_list(var_list_str);
        let rest = &input[end_idx + 1..];

        Ok((
            rest,
            super::Clause {
                name,
                kind: ClauseKind::GangClause {
                    modifier,
                    variables,
                },
            },
        ))
    } else {
        // Bare form - no variables
        Ok((
            input,
            super::Clause {
                name,
                kind: ClauseKind::GangClause {
                    modifier: None,
                    variables: vec![],
                },
            },
        ))
    }
}

/// Parse worker clause: worker or worker(a) or worker(num: a)
fn parse_worker_clause<'a>(
    name: Cow<'a, str>,
    input: &'a str,
) -> nom::IResult<&'a str, super::Clause<'a>> {
    use crate::lexer;
    use crate::parser::clause::{ClauseKind, WorkerModifier};

    // Check if there are parentheses (peek, don't consume whitespace yet)
    let trimmed = input.trim_start();
    if trimmed.starts_with('(') {
        // Parenthesized form - now consume the whitespace properly
        let (input, _) = lexer::skip_space_and_comments(input)?;
        let (input, _) = nom::bytes::complete::tag("(")(input)?;
        let (input, _) = lexer::skip_space_and_comments(input)?;

        // Try to parse "num:" modifier
        let (input, modifier) =
            if input.trim_start().starts_with("num:") || input.trim_start().starts_with("num :") {
                let colon_idx = input.find(':').unwrap();
                let after_colon = &input[colon_idx + 1..];
                (after_colon, Some(WorkerModifier::Num))
            } else {
                (input, None)
            };

        let (input, _) = lexer::skip_space_and_comments(input)?;

        // Parse until closing paren
        let mut depth = 0;
        let mut end_idx = None;

        for (idx, ch) in input.char_indices() {
            match ch {
                '(' | '[' => depth += 1,
                ')' if depth == 0 => {
                    end_idx = Some(idx);
                    break;
                }
                ')' | ']' => depth -= 1,
                _ => {}
            }
        }

        let end_idx = end_idx.ok_or_else(|| {
            nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag))
        })?;

        let var_list_str = &input[..end_idx];
        let variables = parse_variable_list(var_list_str);
        let rest = &input[end_idx + 1..];

        Ok((
            rest,
            super::Clause {
                name,
                kind: ClauseKind::WorkerClause {
                    modifier,
                    variables,
                },
            },
        ))
    } else {
        // Bare form - no variables
        Ok((
            input,
            super::Clause {
                name,
                kind: ClauseKind::WorkerClause {
                    modifier: None,
                    variables: vec![],
                },
            },
        ))
    }
}

/// Parse vector clause: vector or vector(a) or vector(length: a)
fn parse_vector_clause<'a>(
    name: Cow<'a, str>,
    input: &'a str,
) -> nom::IResult<&'a str, super::Clause<'a>> {
    use crate::lexer;
    use crate::parser::clause::{ClauseKind, VectorModifier};

    // Check if there are parentheses (peek, don't consume whitespace yet)
    let trimmed = input.trim_start();
    if trimmed.starts_with('(') {
        // Parenthesized form - now consume the whitespace properly
        let (input, _) = lexer::skip_space_and_comments(input)?;
        let (input, _) = nom::bytes::complete::tag("(")(input)?;
        let (input, _) = lexer::skip_space_and_comments(input)?;

        // Try to parse "length:" modifier
        let (input, modifier) = if input.trim_start().starts_with("length:")
            || input.trim_start().starts_with("length :")
        {
            let colon_idx = input.find(':').unwrap();
            let after_colon = &input[colon_idx + 1..];
            (after_colon, Some(VectorModifier::Length))
        } else {
            (input, None)
        };

        let (input, _) = lexer::skip_space_and_comments(input)?;

        // Parse until closing paren
        let mut depth = 0;
        let mut end_idx = None;

        for (idx, ch) in input.char_indices() {
            match ch {
                '(' | '[' => depth += 1,
                ')' if depth == 0 => {
                    end_idx = Some(idx);
                    break;
                }
                ')' | ']' => depth -= 1,
                _ => {}
            }
        }

        let end_idx = end_idx.ok_or_else(|| {
            nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag))
        })?;

        let var_list_str = &input[..end_idx];
        let variables = parse_variable_list(var_list_str);
        let rest = &input[end_idx + 1..];

        Ok((
            rest,
            super::Clause {
                name,
                kind: ClauseKind::VectorClause {
                    modifier,
                    variables,
                },
            },
        ))
    } else {
        // Bare form - no variables
        Ok((
            input,
            super::Clause {
                name,
                kind: ClauseKind::VectorClause {
                    modifier: None,
                    variables: vec![],
                },
            },
        ))
    }
}
