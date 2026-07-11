use std::borrow::Cow;

use super::{
    ClauseRegistry, ClauseRegistryBuilder, ClauseRule, DirectiveRegistry, DirectiveRegistryBuilder,
    Parser,
};

const OPENACC_DEFAULT_CLAUSE_RULE: ClauseRule = ClauseRule::Unsupported;

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
    Indirect => { name: "indirect", rule: ClauseRule::Flexible },
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
    ExitData => "exit data",
    HostData => "host_data",
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

pub(crate) fn clause_registry() -> ClauseRegistry {
    let mut builder = ClauseRegistryBuilder::new().with_default_rule(OPENACC_DEFAULT_CLAUSE_RULE);

    for clause in OpenAccClause::ALL {
        builder.register_with_rule_mut(clause.name(), clause.rule());
    }

    builder.build()
}

fn parse_cache_directive<'a>(
    name: Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::Directive;
    let (input, _) = clause_registry.skip_trivia(input)?;
    let (rest_after_paren, parameter) =
        parse_parenthesized_parameter(input, clause_registry.is_case_insensitive())?;
    let (rest, clauses) = clause_registry.parse_sequence(rest_after_paren)?;

    Ok((
        rest,
        Directive::new(name, Some(Cow::Borrowed(parameter)), clauses),
    ))
}

fn parse_wait_directive<'a>(
    name: Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::Directive;
    let (input, _) = clause_registry.skip_trivia(input)?;
    if input.trim_start().starts_with('(') {
        let (rest, parameter) =
            parse_parenthesized_parameter(input, clause_registry.is_case_insensitive())?;
        let (rest, clauses) = clause_registry.parse_sequence(rest)?;
        return Ok((
            rest,
            Directive::new(name, Some(Cow::Borrowed(parameter)), clauses),
        ));
    }

    let (rest, clauses) = clause_registry.parse_sequence(input)?;
    Ok((rest, Directive::new(name, None, clauses)))
}

fn parse_end_directive<'a>(
    name: Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::Directive;
    use crate::lexer;

    let (parameter_start, _) = clause_registry.skip_trivia(input)?;
    let mut rest = parameter_start;
    let mut parameter_len = 0;

    while let Ok((new_rest, _)) = lexer::lex_identifier_token(rest) {
        parameter_len = parameter_start.len() - new_rest.len();
        rest = new_rest;

        if let Ok((new_rest, _)) = clause_registry.skip_trivia(rest) {
            rest = new_rest;
        }
    }

    let (rest, clauses) = clause_registry.parse_sequence(rest)?;
    let parameter = &parameter_start[..parameter_len];

    Ok((
        rest,
        Directive::new(name, Some(Cow::Borrowed(parameter)), clauses),
    ))
}

fn parse_routine_directive<'a>(
    name: Cow<'a, str>,
    input: &'a str,
    clause_registry: &ClauseRegistry,
) -> nom::IResult<&'a str, super::Directive<'a>> {
    use super::Directive;
    let (input, _) = clause_registry.skip_trivia(input)?;
    if input.trim_start().starts_with('(') {
        let (rest_after_paren, parameter) =
            parse_parenthesized_parameter(input, clause_registry.is_case_insensitive())?;
        let (rest, clauses) = clause_registry.parse_sequence(rest_after_paren)?;
        return Ok((
            rest,
            Directive::new(name, Some(Cow::Borrowed(parameter)), clauses),
        ));
    }

    let (rest, clauses) = clause_registry.parse_sequence(input)?;
    Ok((rest, Directive::new(name, None, clauses)))
}

pub(crate) fn directive_registry() -> DirectiveRegistry {
    let mut builder = DirectiveRegistryBuilder::new();

    builder = builder.register_custom("cache", parse_cache_directive);
    builder = builder.register_custom("wait", parse_wait_directive);
    builder = builder.register_custom("end", parse_end_directive);
    builder = builder.register_custom("routine", parse_routine_directive);

    const CUSTOM_ACC_DIRECTIVES: &[OpenAccDirective] = &[
        OpenAccDirective::Cache,
        OpenAccDirective::Wait,
        OpenAccDirective::End,
        OpenAccDirective::Routine,
    ];

    for directive in OpenAccDirective::ALL {
        let name = directive.as_str();
        if !CUSTOM_ACC_DIRECTIVES.contains(directive) {
            builder = builder.register_generic(name);
        }
    }

    builder.build()
}

pub(crate) fn parser() -> Parser {
    Parser::new(
        directive_registry(),
        clause_registry(),
        crate::lexer::Language::C,
        super::Dialect::OpenAcc,
    )
}

fn parse_parenthesized_parameter(input: &str, case_insensitive: bool) -> nom::IResult<&str, &str> {
    use crate::lexer;
    use nom::bytes::complete::tag;
    use nom::error::{Error, ErrorKind};

    let (parameter_start, _) = if case_insensitive {
        lexer::skip_fortran_space_and_comments(input)?
    } else {
        lexer::skip_space_and_comments(input)?
    };
    let (content_start, _) = tag("(")(parameter_start)?;

    let Some(end_idx) = lexer::find_matching_parenthesis(content_start, case_insensitive) else {
        return Err(nom::Err::Error(Error::new(content_start, ErrorKind::Tag)));
    };

    let rest = &content_start[end_idx + 1..];
    let parameter_len = parameter_start.len() - rest.len();

    Ok((rest, &parameter_start[..parameter_len]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_and_wait_keep_one_unnormalized_parameter() {
        for (source, expected) in [
            (
                "#pragma acc cache( readonly : values[0:n], tile[index] )",
                "( readonly : values[0:n], tile[index] )",
            ),
            (
                "#pragma acc wait( devnum : device : queues : first, second )",
                "( devnum : device : queues : first, second )",
            ),
        ] {
            let parsed = parser().parse(source).expect("directive must parse").1;
            assert_eq!(parsed.parameter.as_deref(), Some(expected));
        }
    }
}
