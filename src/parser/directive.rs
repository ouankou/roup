use std::collections::{HashMap, HashSet};

use nom::{error::ErrorKind, IResult};

use super::clause::{Clause, ClauseRegistry};

type DirectiveParserFn =
    for<'a> fn(&'a str, &'a str, &ClauseRegistry) -> IResult<&'a str, Directive<'a>>;

#[derive(Debug, PartialEq, Eq)]
pub struct Directive<'a> {
    pub name: &'a str,
    pub clauses: Vec<Clause<'a>>,
}

#[derive(Clone, Copy)]
pub enum DirectiveRule {
    Generic,
    Custom(DirectiveParserFn),
}

impl DirectiveRule {
    fn parse<'a>(
        self,
        name: &'a str,
        input: &'a str,
        clause_registry: &ClauseRegistry,
    ) -> IResult<&'a str, Directive<'a>> {
        match self {
            DirectiveRule::Generic => {
                let (input, clauses) = clause_registry.parse_sequence(input)?;
                Ok((input, Directive { name, clauses }))
            }
            DirectiveRule::Custom(parser) => parser(name, input, clause_registry),
        }
    }
}

pub struct DirectiveRegistry {
    rules: HashMap<&'static str, DirectiveRule>,
    prefixes: HashSet<String>,
    default_rule: DirectiveRule,
}

impl DirectiveRegistry {
    pub fn builder() -> DirectiveRegistryBuilder {
        DirectiveRegistryBuilder::new()
    }

    pub fn parse<'a>(
        &self,
        input: &'a str,
        clause_registry: &ClauseRegistry,
    ) -> IResult<&'a str, Directive<'a>> {
        let (rest, name) = self.lex_name(input)?;
        self.parse_with_name(name, rest, clause_registry)
    }

    pub fn parse_with_name<'a>(
        &self,
        name: &'a str,
        input: &'a str,
        clause_registry: &ClauseRegistry,
    ) -> IResult<&'a str, Directive<'a>> {
        let rule = self.rules.get(name).copied().unwrap_or(self.default_rule);
        rule.parse(name, input, clause_registry)
    }

    fn lex_name<'a>(&self, input: &'a str) -> IResult<&'a str, &'a str> {
        let bytes = input.as_bytes();
        let mut idx = 0;
        while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
            idx += 1;
        }

        let start = idx;
        let mut last_match_end = None;

        while idx < bytes.len() {
            let token_start = idx;
            while idx < bytes.len() && is_identifier(bytes[idx]) {
                idx += 1;
            }

            if token_start == idx {
                break;
            }

            let candidate = &input[start..idx];
            if self.rules.contains_key(candidate) {
                last_match_end = Some(idx);
            }

            let space_start = idx;
            while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
                idx += 1;
            }

            if idx > space_start {
                let prefix_candidate = input[start..idx].trim_end();
                if idx < bytes.len() && is_identifier(bytes[idx]) {
                    if self.prefixes.contains(prefix_candidate)
                        || self.rules.contains_key(prefix_candidate)
                    {
                        continue;
                    }

                    break;
                }

                break;
            } else {
                break;
            }
        }

        let name_end = last_match_end
            .ok_or_else(|| nom::Err::Error(nom::error::Error::new(input, ErrorKind::Tag)))?;

        let name = &input[start..name_end];
        let rest = &input[name_end..];

        Ok((rest, name))
    }
}

impl Default for DirectiveRegistry {
    fn default() -> Self {
        DirectiveRegistry::builder()
            .register_generic("parallel")
            .build()
    }
}

pub struct DirectiveRegistryBuilder {
    rules: HashMap<&'static str, DirectiveRule>,
    prefixes: HashSet<String>,
    default_rule: DirectiveRule,
}

impl DirectiveRegistryBuilder {
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
            prefixes: HashSet::new(),
            default_rule: DirectiveRule::Generic,
        }
    }

    pub fn register_generic(mut self, name: &'static str) -> Self {
        self.insert_rule(name, DirectiveRule::Generic);
        self
    }

    pub fn register_custom(mut self, name: &'static str, parser: DirectiveParserFn) -> Self {
        self.insert_rule(name, DirectiveRule::Custom(parser));
        self
    }

    pub fn with_default_rule(mut self, rule: DirectiveRule) -> Self {
        self.default_rule = rule;
        self
    }

    pub fn build(self) -> DirectiveRegistry {
        DirectiveRegistry {
            rules: self.rules,
            prefixes: self.prefixes,
            default_rule: self.default_rule,
        }
    }

    fn insert_rule(&mut self, name: &'static str, rule: DirectiveRule) {
        self.rules.insert(name, rule);
        self.register_prefixes(name);
    }

    fn register_prefixes(&mut self, name: &'static str) {
        let segments = name.split_whitespace().collect::<Vec<_>>();
        if segments.len() <= 1 {
            return;
        }

        let mut current = String::new();
        for segment in segments.iter().take(segments.len() - 1) {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(segment);
            self.prefixes.insert(current.clone());
        }
    }
}

fn is_identifier(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ClauseKind;
    use nom::bytes::complete::tag;

    #[test]
    fn parses_generic_directive_with_clauses() {
        let clause_registry = ClauseRegistry::default();
        let registry = DirectiveRegistry::default();

        let (rest, directive) = registry
            .parse_with_name("parallel", " private(x, y) nowait", &clause_registry)
            .expect("parsing should succeed");

        assert_eq!(rest, "");
        assert_eq!(directive.name, "parallel");
        assert_eq!(directive.clauses.len(), 2);
        assert_eq!(directive.clauses[0].name, "private");
        assert_eq!(directive.clauses[0].kind, ClauseKind::Parenthesized("x, y"));
        assert_eq!(directive.clauses[1].name, "nowait");
        assert_eq!(directive.clauses[1].kind, ClauseKind::Bare);
    }

    #[test]
    fn parses_longest_matching_name() {
        let clause_registry = ClauseRegistry::default();
        let registry = DirectiveRegistry::builder()
            .register_generic("target teams")
            .register_generic("target teams distribute")
            .register_generic("target teams distribute parallel for")
            .build();

        let (rest, directive) = registry
            .parse(
                "target teams distribute parallel for private(a)",
                &clause_registry,
            )
            .expect("parsing should succeed");

        assert_eq!(rest, "");
        assert_eq!(directive.name, "target teams distribute parallel for");
        assert_eq!(directive.clauses.len(), 1);
        assert_eq!(directive.clauses[0].name, "private");
    }

    fn parse_prefixed_directive<'a>(
        name: &'a str,
        input: &'a str,
        clause_registry: &ClauseRegistry,
    ) -> IResult<&'a str, Directive<'a>> {
        let (input, _) = tag("custom:")(input)?;
        let (input, clauses) = clause_registry.parse_sequence(input)?;

        Ok((input, Directive { name, clauses }))
    }

    #[test]
    fn supports_custom_directive_rule() {
        let clause_registry = ClauseRegistry::default();
        let registry = DirectiveRegistry::builder()
            .register_custom("target", parse_prefixed_directive)
            .build();

        let (rest, directive) = registry
            .parse_with_name("target", "custom: private(a)", &clause_registry)
            .expect("parsing should succeed");

        assert_eq!(rest, "");
        assert_eq!(directive.name, "target");
        assert_eq!(directive.clauses.len(), 1);
        assert_eq!(directive.clauses[0].name, "private");
        assert_eq!(directive.clauses[0].kind, ClauseKind::Parenthesized("a"));
    }
}
