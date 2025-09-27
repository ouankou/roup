use std::collections::HashMap;

use nom::IResult;

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
    default_rule: DirectiveRule,
}

impl DirectiveRegistry {
    pub fn builder() -> DirectiveRegistryBuilder {
        DirectiveRegistryBuilder::new()
    }

    pub fn parse<'a>(
        &self,
        name: &'a str,
        input: &'a str,
        clause_registry: &ClauseRegistry,
    ) -> IResult<&'a str, Directive<'a>> {
        let rule = self.rules.get(name).copied().unwrap_or(self.default_rule);

        rule.parse(name, input, clause_registry)
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
    default_rule: DirectiveRule,
}

impl DirectiveRegistryBuilder {
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
            default_rule: DirectiveRule::Generic,
        }
    }

    pub fn register_generic(mut self, name: &'static str) -> Self {
        self.rules.insert(name, DirectiveRule::Generic);
        self
    }

    pub fn register_custom(mut self, name: &'static str, parser: DirectiveParserFn) -> Self {
        self.rules.insert(name, DirectiveRule::Custom(parser));
        self
    }

    pub fn with_default_rule(mut self, rule: DirectiveRule) -> Self {
        self.default_rule = rule;
        self
    }

    pub fn build(self) -> DirectiveRegistry {
        DirectiveRegistry {
            rules: self.rules,
            default_rule: self.default_rule,
        }
    }
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
            .parse("parallel", " private(x, y) nowait", &clause_registry)
            .expect("parsing should succeed");

        assert_eq!(rest, "");
        assert_eq!(directive.name, "parallel");
        assert_eq!(directive.clauses.len(), 2);
        assert_eq!(directive.clauses[0].name, "private");
        assert_eq!(
            directive.clauses[0].kind,
            ClauseKind::IdentifierList(vec!["x", "y"])
        );
        assert_eq!(directive.clauses[1].name, "nowait");
        assert_eq!(directive.clauses[1].kind, ClauseKind::Bare);
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
            .parse("target", "custom: private(a)", &clause_registry)
            .expect("parsing should succeed");

        assert_eq!(rest, "");
        assert_eq!(directive.name, "target");
        assert_eq!(directive.clauses.len(), 1);
        assert_eq!(directive.clauses[0].name, "private");
        assert_eq!(
            directive.clauses[0].kind,
            ClauseKind::IdentifierList(vec!["a"])
        );
    }
}
