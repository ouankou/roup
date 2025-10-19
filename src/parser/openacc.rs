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
    Copyin => { name: "copyin", rule: ClauseRule::Parenthesized },
    Copyout => { name: "copyout", rule: ClauseRule::Parenthesized },
    Create => { name: "create", rule: ClauseRule::Parenthesized },
    PresentOrCopy => { name: "present_or_copy", rule: ClauseRule::Parenthesized },
    PresentOrCopyAlias => { name: "pcopy", rule: ClauseRule::Parenthesized },
    PresentOrCopyin => { name: "present_or_copyin", rule: ClauseRule::Parenthesized },
    PresentOrCopyinAlias => { name: "pcopyin", rule: ClauseRule::Parenthesized },
    PresentOrCopyout => { name: "present_or_copyout", rule: ClauseRule::Parenthesized },
    PresentOrCopyoutAlias => { name: "pcopyout", rule: ClauseRule::Parenthesized },
    PresentOrCreate => { name: "present_or_create", rule: ClauseRule::Parenthesized },
    PresentOrCreateAlias => { name: "pcreate", rule: ClauseRule::Parenthesized },
    Default => { name: "default", rule: ClauseRule::Parenthesized },
    DefaultAsync => { name: "default_async", rule: ClauseRule::Parenthesized },
    Delete => { name: "delete", rule: ClauseRule::Parenthesized },
    Detach => { name: "detach", rule: ClauseRule::Parenthesized },
    Device => { name: "device", rule: ClauseRule::Parenthesized },
    DeviceNum => { name: "device_num", rule: ClauseRule::Parenthesized },
    DeviceResident => { name: "device_resident", rule: ClauseRule::Parenthesized },
    DeviceType => { name: "device_type", rule: ClauseRule::Flexible },
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
    SelfClause => { name: "self", rule: ClauseRule::Bare },
    Seq => { name: "seq", rule: ClauseRule::Bare },
    Tile => { name: "tile", rule: ClauseRule::Parenthesized },
    Update => { name: "update", rule: ClauseRule::Parenthesized },
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
        builder.register_with_rule_mut(clause.name(), clause.rule());
    }

    builder.build()
}

fn parse_cache_directive<'a>(
    _name: Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::Directive;
    use crate::lexer;

    // FIX: Removed duplicate tag("(") - parse_parenthesized_content_inner handles it
    let (input, _) = lexer::skip_space_and_comments(input)?;
    let (rest_after_paren, content) = parse_parenthesized_content_inner(input)?;
    let (rest, clauses) = clause_registry.parse_sequence(rest_after_paren)?;

    let full_name = format!("cache({})", content.trim());

    Ok((
        rest,
        Directive {
            name: Cow::Owned(full_name),
            clauses,
        },
    ))
}

fn parse_wait_directive<'a>(
    name: Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::Directive;
    use crate::lexer;

    let (input, _) = lexer::skip_space_and_comments(input)?;
    if input.trim_start().starts_with('(') {
        // FIX: parse_parenthesized_content_inner already handles the opening paren
        let (rest, content) = parse_parenthesized_content_inner(input)?;
        let (rest, clauses) = clause_registry.parse_sequence(rest)?;
        let full_name = format!("wait({})", content.trim());
        return Ok((
            rest,
            Directive {
                name: Cow::Owned(full_name),
                clauses,
            },
        ));
    }

    let (rest, clauses) = clause_registry.parse_sequence(input)?;
    Ok((rest, Directive { name, clauses }))
}

fn parse_end_directive<'a>(
    _name: Cow<'a, str>,
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

    // Reassemble into full directive name
    let directive = directive_parts.join(" ");
    let (rest, clauses) = clause_registry.parse_sequence(rest)?;
    let full_name = format!("end {}", directive);

    Ok((
        rest,
        Directive {
            name: Cow::Owned(full_name),
            clauses,
        },
    ))
}

pub fn directive_registry() -> DirectiveRegistry {
    let mut builder = DirectiveRegistryBuilder::new();

    builder = builder.register_custom("cache", parse_cache_directive);
    builder = builder.register_custom("wait", parse_wait_directive);
    builder = builder.register_custom("end", parse_end_directive);

    for directive in OpenAccDirective::ALL {
        let name = directive.as_str();
        if matches!(name, "cache" | "wait" | "end") {
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
