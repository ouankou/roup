use std::collections::HashMap;

use nom::{
    character::complete::{char, multispace0, multispace1},
    multi::{separated_list0, separated_list1},
    sequence::{delimited, tuple},
    IResult,
};

use crate::lexer;

type ClauseParserFn = for<'a> fn(&'a str, &'a str) -> IResult<&'a str, Clause<'a>>;

#[derive(Debug, PartialEq, Eq)]
pub enum ClauseKind<'a> {
    Bare,
    IdentifierList(Vec<&'a str>),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Clause<'a> {
    pub name: &'a str,
    pub kind: ClauseKind<'a>,
}

#[derive(Clone, Copy)]
pub enum ClauseRule {
    Bare,
    IdentifierList { delimiter: char },
    Custom(ClauseParserFn),
}

impl ClauseRule {
    fn parse<'a>(self, name: &'a str, input: &'a str) -> IResult<&'a str, Clause<'a>> {
        match self {
            ClauseRule::Bare => Ok((
                input,
                Clause {
                    name,
                    kind: ClauseKind::Bare,
                },
            )),
            ClauseRule::IdentifierList { delimiter } => {
                parse_identifier_list_clause(name, input, delimiter)
            }
            ClauseRule::Custom(parser) => parser(name, input),
        }
    }
}

pub struct ClauseRegistry {
    rules: HashMap<&'static str, ClauseRule>,
    default_rule: ClauseRule,
}

impl ClauseRegistry {
    pub fn builder() -> ClauseRegistryBuilder {
        ClauseRegistryBuilder::new()
    }

    pub fn parse_sequence<'a>(&self, input: &'a str) -> IResult<&'a str, Vec<Clause<'a>>> {
        let (input, _) = multispace0(input)?;
        let parse_clause = |input| self.parse_clause(input);
        let (input, clauses) = separated_list0(multispace1, parse_clause)(input)?;
        let (input, _) = multispace0(input)?;
        Ok((input, clauses))
    }

    fn parse_clause<'a>(&self, input: &'a str) -> IResult<&'a str, Clause<'a>> {
        let (input, name) = lexer::lex_clause(input)?;

        let rule = self.rules.get(name).copied().unwrap_or(self.default_rule);

        rule.parse(name, input)
    }
}

impl Default for ClauseRegistry {
    fn default() -> Self {
        ClauseRegistry::builder()
            .register_identifier_list("private", ',')
            .build()
    }
}

pub struct ClauseRegistryBuilder {
    rules: HashMap<&'static str, ClauseRule>,
    default_rule: ClauseRule,
}

impl ClauseRegistryBuilder {
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
            default_rule: ClauseRule::Bare,
        }
    }

    pub fn register_bare(mut self, name: &'static str) -> Self {
        self.rules.insert(name, ClauseRule::Bare);
        self
    }

    pub fn register_identifier_list(mut self, name: &'static str, delimiter: char) -> Self {
        self.rules
            .insert(name, ClauseRule::IdentifierList { delimiter });
        self
    }

    pub fn register_custom(mut self, name: &'static str, parser: ClauseParserFn) -> Self {
        self.rules.insert(name, ClauseRule::Custom(parser));
        self
    }

    pub fn with_default_rule(mut self, rule: ClauseRule) -> Self {
        self.default_rule = rule;
        self
    }

    pub fn build(self) -> ClauseRegistry {
        ClauseRegistry {
            rules: self.rules,
            default_rule: self.default_rule,
        }
    }
}

fn parse_identifier_list_clause<'a>(
    name: &'a str,
    input: &'a str,
    delimiter: char,
) -> IResult<&'a str, Clause<'a>> {
    let separator = tuple((multispace0, char(delimiter), multispace0));
    let (input, values) = delimited(
        char('('),
        separated_list1(separator, lexer::lex_clause),
        char(')'),
    )(input)?;

    Ok((
        input,
        Clause {
            name,
            kind: ClauseKind::IdentifierList(values),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer;
    use nom::character::complete::char;

    #[test]
    fn parses_empty_clause_sequence() {
        let registry = ClauseRegistry::default();

        let (rest, clauses) = registry.parse_sequence("").expect("parsing should succeed");

        assert_eq!(rest, "");
        assert!(clauses.is_empty());
    }

    #[test]
    fn parses_bare_clause_with_default_rule() {
        let registry = ClauseRegistry::default();

        let (rest, clauses) = registry
            .parse_sequence("nowait")
            .expect("parsing should succeed");

        assert_eq!(rest, "");
        assert_eq!(
            clauses,
            vec![Clause {
                name: "nowait",
                kind: ClauseKind::Bare,
            }]
        );
    }

    #[test]
    fn parses_identifier_list_clause() {
        let registry = ClauseRegistry::default();

        let (rest, clauses) = registry
            .parse_sequence("private(a, b, c)")
            .expect("parsing should succeed");

        assert_eq!(rest, "");
        assert_eq!(clauses.len(), 1);
        assert_eq!(clauses[0].name, "private");
        assert_eq!(
            clauses[0].kind,
            ClauseKind::IdentifierList(vec!["a", "b", "c"])
        );
    }

    fn parse_single_identifier<'a>(name: &'a str, input: &'a str) -> IResult<&'a str, Clause<'a>> {
        let (input, _) = char('(')(input)?;
        let (input, identifier) = lexer::lex_clause(input)?;
        let (input, _) = char(')')(input)?;

        Ok((
            input,
            Clause {
                name,
                kind: ClauseKind::IdentifierList(vec![identifier]),
            },
        ))
    }

    #[test]
    fn supports_custom_clause_rule() {
        let registry = ClauseRegistry::builder()
            .register_custom("device", parse_single_identifier)
            .build();

        let (rest, clauses) = registry
            .parse_sequence("device(gpu)")
            .expect("parsing should succeed");

        assert_eq!(rest, "");
        assert_eq!(clauses.len(), 1);
        assert_eq!(clauses[0].name, "device");
        assert_eq!(clauses[0].kind, ClauseKind::IdentifierList(vec!["gpu"]));
    }
}
