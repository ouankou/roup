use std::{
    collections::{HashMap, HashSet},
    fmt,
};

use nom::{error::ErrorKind, IResult};

use super::clause::{Clause, ClauseRegistry};

type DirectiveParserFn =
    for<'a> fn(&'a str, &'a str, &ClauseRegistry) -> IResult<&'a str, Directive<'a>>;

#[derive(Debug, PartialEq, Eq)]
pub struct Directive<'a> {
    pub name: &'a str,
    pub clauses: Vec<Clause<'a>>,
}

impl<'a> Directive<'a> {
    pub fn to_pragma_string(&self) -> String {
        self.to_string()
    }
}

impl<'a> fmt::Display for Directive<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#pragma omp {}", self.name)?;
        if !self.clauses.is_empty() {
            write!(f, " ")?;
            for (idx, clause) in self.clauses.iter().enumerate() {
                if idx > 0 {
                    write!(f, " ")?;
                }
                write!(f, "{}", clause)?;
            }
        }
        Ok(())
    }
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
    /// Normalized lowercase map for case-insensitive lookups (built at construction)
    normalized_rules: HashMap<String, DirectiveRule>,
    prefixes: HashSet<String>,
    default_rule: DirectiveRule,
    case_insensitive: bool,
}

impl DirectiveRegistry {
    pub fn builder() -> DirectiveRegistryBuilder {
        DirectiveRegistryBuilder::new()
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

        rule.parse(name, input, clause_registry)
    }

    fn lex_name<'a>(&self, input: &'a str) -> IResult<&'a str, &'a str> {
        use crate::lexer::is_identifier_char as is_ident_char;

        let mut chars = input.char_indices();
        // skip leading whitespace
        let start = loop {
            match chars.next() {
                Some((_, ch)) if ch.is_whitespace() => continue,
                Some((idx, _)) => break idx,
                None => {
                    return Err(nom::Err::Error(nom::error::Error::new(
                        input,
                        ErrorKind::Tag,
                    )))
                }
            }
        };

        let mut idx = start;
        let mut last_match_end = None;

        while let Some((pos, ch)) = input[idx..].char_indices().next() {
            // advance over one identifier token
            if !is_ident_char(ch) {
                break;
            }
            // find end of identifier token starting at idx
            let mut j = idx + pos;
            while let Some((p, ch2)) = input[j..].char_indices().next() {
                if !is_ident_char(ch2) {
                    break;
                }
                j = j + p + ch2.len_utf8();
            }

            let candidate = &input[start..j];
            let has_rule = if self.case_insensitive {
                self.rules
                    .keys()
                    .any(|k| k.to_lowercase() == candidate.to_lowercase())
            } else {
                self.rules.contains_key(candidate)
            };

            if has_rule {
                last_match_end = Some(j);
            }

            // advance idx past any whitespace following the identifier
            idx = j;
            while let Some((p, chw)) = input[idx..].char_indices().next() {
                if chw.is_whitespace() {
                    idx = idx + p + chw.len_utf8();
                } else {
                    break;
                }
            }

            // if next character starts an identifier, loop to extend candidate
            if let Some((_, next_ch)) = input[idx..].char_indices().next() {
                if is_ident_char(next_ch) {
                    // check if prefix is registered; if so, continue to extend
                    let prefix_candidate = input[start..idx].trim_end();
                    let has_prefix = if self.case_insensitive {
                        self.prefixes
                            .iter()
                            .any(|p| p.to_lowercase() == prefix_candidate.to_lowercase())
                            || self
                                .rules
                                .keys()
                                .any(|k| k.to_lowercase() == prefix_candidate.to_lowercase())
                    } else {
                        self.prefixes.contains(prefix_candidate)
                            || self.rules.contains_key(prefix_candidate)
                    };

                    if has_prefix {
                        continue;
                    }
                }
            }

            break;
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
    case_insensitive: bool,
}

impl DirectiveRegistryBuilder {
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
            prefixes: HashSet::new(),
            default_rule: DirectiveRule::Generic,
            case_insensitive: false,
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

    pub fn with_case_insensitive(mut self, enabled: bool) -> Self {
        self.case_insensitive = enabled;
        self
    }

    pub fn build(self) -> DirectiveRegistry {
        // Build normalized map if case-insensitive mode is enabled
        let normalized_rules = if self.case_insensitive {
            self.rules
                .iter()
                .map(|(k, v)| (k.to_lowercase(), *v))
                .collect()
        } else {
            HashMap::new() // Empty map for case-sensitive mode
        };

        DirectiveRegistry {
            rules: self.rules,
            normalized_rules,
            prefixes: self.prefixes,
            default_rule: self.default_rule,
            case_insensitive: self.case_insensitive,
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

impl Default for DirectiveRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// legacy byte-based identifier checker removed in favor of char-based helper

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

    #[test]
    fn directive_display_includes_all_clauses() {
        let directive = Directive {
            name: "parallel",
            clauses: vec![
                Clause {
                    name: "private",
                    kind: ClauseKind::Parenthesized("a, b"),
                },
                Clause {
                    name: "nowait",
                    kind: ClauseKind::Bare,
                },
            ],
        };

        assert_eq!(
            directive.to_string(),
            "#pragma omp parallel private(a, b) nowait"
        );
        assert_eq!(
            directive.to_pragma_string(),
            "#pragma omp parallel private(a, b) nowait"
        );
    }

    #[test]
    fn directive_display_without_clauses() {
        let directive = Directive {
            name: "barrier",
            clauses: vec![],
        };

        assert_eq!(directive.to_string(), "#pragma omp barrier");
        assert_eq!(directive.to_pragma_string(), "#pragma omp barrier");
    }
}
