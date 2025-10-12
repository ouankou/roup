use std::{collections::HashMap, fmt};

use nom::{multi::separated_list0, IResult, Parser};

use crate::lexer;

type ClauseParserFn = for<'a> fn(&'a str, &'a str) -> IResult<&'a str, Clause<'a>>;

#[derive(Debug, PartialEq, Eq)]
pub enum ClauseKind<'a> {
    Bare,
    Parenthesized(&'a str),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Clause<'a> {
    pub name: &'a str,
    pub kind: ClauseKind<'a>,
}

impl<'a> Clause<'a> {
    pub fn to_source_string(&self) -> String {
        self.to_string()
    }
}

impl<'a> fmt::Display for Clause<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            ClauseKind::Bare => write!(f, "{}", self.name),
            ClauseKind::Parenthesized(value) => write!(f, "{}({})", self.name, value),
        }
    }
}

#[derive(Clone, Copy)]
pub enum ClauseRule {
    Bare,
    Parenthesized,
    Flexible,
    Custom(ClauseParserFn),
    Unsupported,
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
            ClauseRule::Parenthesized => parse_parenthesized_clause(name, input),
            ClauseRule::Flexible => {
                if starts_with_parenthesis(input) {
                    parse_parenthesized_clause(name, input)
                } else {
                    ClauseRule::Bare.parse(name, input)
                }
            }
            ClauseRule::Custom(parser) => parser(name, input),
            ClauseRule::Unsupported => Err(nom::Err::Failure(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Fail,
            ))),
        }
    }
}

pub struct ClauseRegistry {
    rules: HashMap<&'static str, ClauseRule>,
    /// Normalized lowercase map for case-insensitive lookups (built at construction)
    normalized_rules: HashMap<String, ClauseRule>,
    default_rule: ClauseRule,
    case_insensitive: bool,
}

impl ClauseRegistry {
    pub fn builder() -> ClauseRegistryBuilder {
        ClauseRegistryBuilder::new()
    }

    pub fn with_case_insensitive(mut self, enabled: bool) -> Self {
        self.case_insensitive = enabled;
        // Rebuild normalized map if enabling case-insensitive mode
        if enabled && self.normalized_rules.is_empty() {
            self.normalized_rules = self
                .rules
                .iter()
                .map(|(k, v)| (k.to_lowercase(), *v))
                .collect();
        } else if !enabled {
            // Clear normalized map if disabling case-insensitive mode
            self.normalized_rules.clear();
        }
        self
    }

    pub fn parse_sequence<'a>(&self, input: &'a str) -> IResult<&'a str, Vec<Clause<'a>>> {
        let (input, _) = crate::lexer::skip_space_and_comments(input)?;
        let parse_clause = |input| self.parse_clause(input);
        // allow comments and whitespace between clauses (and before the first clause)
        let (input, clauses) =
            separated_list0(|i| crate::lexer::skip_space1_and_comments(i), parse_clause)
                .parse(input)?;
        let (input, _) = crate::lexer::skip_space_and_comments(input)?;
        Ok((input, clauses))
    }

    fn parse_clause<'a>(&self, input: &'a str) -> IResult<&'a str, Clause<'a>> {
        let (input, name) = lexer::lex_clause(input)?;

        // Use efficient lookup based on case sensitivity mode
        let rule = if self.case_insensitive {
            // Use pre-built normalized map for O(1) case-insensitive lookup
            self.normalized_rules
                .get(&name.to_lowercase())
                .copied()
                .unwrap_or(self.default_rule)
        } else {
            // Direct HashMap lookup for case-sensitive mode (no allocation)
            self.rules.get(name).copied().unwrap_or(self.default_rule)
        };

        rule.parse(name, input)
    }
}

impl Default for ClauseRegistry {
    fn default() -> Self {
        ClauseRegistry::builder().build()
    }
}

pub struct ClauseRegistryBuilder {
    rules: HashMap<&'static str, ClauseRule>,
    default_rule: ClauseRule,
    case_insensitive: bool,
}

impl ClauseRegistryBuilder {
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
            default_rule: ClauseRule::Flexible,
            case_insensitive: false,
        }
    }

    // Allow construction via Default in addition to new()

    pub fn register_with_rule(mut self, name: &'static str, rule: ClauseRule) -> Self {
        self.register_with_rule_mut(name, rule);
        self
    }

    pub fn register_with_rule_mut(&mut self, name: &'static str, rule: ClauseRule) -> &mut Self {
        self.rules.insert(name, rule);
        self
    }

    pub fn register_bare(self, name: &'static str) -> Self {
        self.register_with_rule(name, ClauseRule::Bare)
    }

    pub fn register_parenthesized(self, name: &'static str) -> Self {
        self.register_with_rule(name, ClauseRule::Parenthesized)
    }

    pub fn register_custom(self, name: &'static str, parser: ClauseParserFn) -> Self {
        self.register_with_rule(name, ClauseRule::Custom(parser))
    }

    pub fn with_default_rule(mut self, rule: ClauseRule) -> Self {
        self.default_rule = rule;
        self
    }

    pub fn with_case_insensitive(mut self, enabled: bool) -> Self {
        self.case_insensitive = enabled;
        self
    }

    pub fn build(self) -> ClauseRegistry {
        // Build normalized map if case-insensitive mode is enabled
        let normalized_rules = if self.case_insensitive {
            self.rules
                .iter()
                .map(|(k, v)| (k.to_lowercase(), *v))
                .collect()
        } else {
            HashMap::new() // Empty map for case-sensitive mode
        };

        ClauseRegistry {
            rules: self.rules,
            normalized_rules,
            default_rule: self.default_rule,
            case_insensitive: self.case_insensitive,
        }
    }
}

impl Default for ClauseRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

fn starts_with_parenthesis(input: &str) -> bool {
    input.trim_start().starts_with('(')
}

fn parse_parenthesized_clause<'a>(name: &'a str, input: &'a str) -> IResult<&'a str, Clause<'a>> {
    let mut iter = input.char_indices();

    while let Some((idx, ch)) = iter.next() {
        if ch.is_whitespace() {
            continue;
        }

        if ch != '(' {
            return Err(nom::Err::Error(nom::error::Error::new(
                &input[idx..],
                nom::error::ErrorKind::Fail,
            )));
        }

        let start = idx;
        let mut depth = 1;
        let mut end_index = None;
        for (inner_idx, inner_ch) in iter.by_ref() {
            match inner_ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        end_index = Some(inner_idx);
                        break;
                    }
                }
                _ => {}
            }
        }

        let end_index = end_index.ok_or_else(|| {
            nom::Err::Error(nom::error::Error::new(
                &input[start..],
                nom::error::ErrorKind::Fail,
            ))
        })?;

        let content_start = start + 1;
        let content = input[content_start..end_index].trim();
        let rest = &input[end_index + 1..];

        return Ok((
            rest,
            Clause {
                name,
                kind: ClauseKind::Parenthesized(content),
            },
        ));
    }

    Err(nom::Err::Error(nom::error::Error::new(
        input,
        nom::error::ErrorKind::Fail,
    )))
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
        assert_eq!(clauses[0].kind, ClauseKind::Parenthesized("a, b, c"));
    }

    #[test]
    fn clause_display_roundtrips_bare_clause() {
        let clause = Clause {
            name: "nowait",
            kind: ClauseKind::Bare,
        };

        assert_eq!(clause.to_string(), "nowait");
        assert_eq!(clause.to_source_string(), "nowait");
    }

    #[test]
    fn clause_display_roundtrips_parenthesized_clause() {
        let clause = Clause {
            name: "private",
            kind: ClauseKind::Parenthesized("a, b"),
        };

        assert_eq!(clause.to_string(), "private(a, b)");
        assert_eq!(clause.to_source_string(), "private(a, b)");
    }

    fn parse_single_identifier<'a>(name: &'a str, input: &'a str) -> IResult<&'a str, Clause<'a>> {
        let (input, _) = char('(')(input)?;
        let (input, identifier) = lexer::lex_clause(input)?;
        let (input, _) = char(')')(input)?;

        Ok((
            input,
            Clause {
                name,
                kind: ClauseKind::Parenthesized(identifier),
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
        assert_eq!(clauses[0].kind, ClauseKind::Parenthesized("gpu"));
    }

    #[test]
    fn rejects_unregistered_clause_when_default_is_unsupported() {
        let registry = ClauseRegistry::builder()
            .with_default_rule(ClauseRule::Unsupported)
            .register_bare("nowait")
            .build();

        let result = registry.parse_sequence("unknown");

        assert!(result.is_err());
    }
}
