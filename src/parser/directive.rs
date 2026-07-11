use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    ops::Deref,
};

use nom::{IResult, error::ErrorKind};

use super::clause::{ClauseRegistry, LocatedClause};
use crate::parser::directive_kind::DirectiveName;

type DirectiveParserFn =
    for<'a> fn(Cow<'a, str>, &'a str, &ClauseRegistry) -> IResult<&'a str, Directive<'a>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Directive<'a> {
    pub(crate) name: DirectiveName,
    pub(crate) parameter: Option<Cow<'a, str>>,
    pub(crate) clauses: Vec<LocatedClause<'a>>,
}

impl<'a> Directive<'a> {
    pub(crate) fn new<N: Into<DirectiveName>>(
        name: N,
        parameter: Option<Cow<'a, str>>,
        clauses: Vec<LocatedClause<'a>>,
    ) -> Self {
        Self {
            name: name.into(),
            parameter,
            clauses,
        }
    }
}

/// Directive syntax paired with the exact source spelling of its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocatedDirective<'a> {
    syntax: Directive<'a>,
    name_source: &'a str,
}

impl<'a> LocatedDirective<'a> {
    pub(crate) fn new(syntax: Directive<'a>, name_source: &'a str) -> Self {
        Self {
            syntax,
            name_source,
        }
    }

    pub(crate) const fn name_source(&self) -> &'a str {
        self.name_source
    }
}

impl<'a> Deref for LocatedDirective<'a> {
    type Target = Directive<'a>;

    fn deref(&self) -> &Self::Target {
        &self.syntax
    }
}

#[derive(Clone, Copy)]
pub(crate) enum DirectiveRule {
    Generic,
    Custom(DirectiveParserFn),
}

impl DirectiveRule {
    fn parse<'a>(
        self,
        name: Cow<'a, str>,
        input: &'a str,
        clause_registry: &ClauseRegistry,
    ) -> IResult<&'a str, Directive<'a>> {
        match self {
            DirectiveRule::Generic => {
                let (input, clauses) = clause_registry.parse_sequence(input)?;
                Ok((input, Directive::new(name, None, clauses)))
            }
            DirectiveRule::Custom(parser) => parser(name, input, clause_registry),
        }
    }
}

pub(crate) struct DirectiveRegistry {
    rules: HashMap<&'static str, DirectiveRule>,
    prefixes: HashSet<String>,
    default_rule: DirectiveRule,
    case_insensitive: bool,
    optional_keyword_whitespace: bool,
}

impl DirectiveRegistry {
    pub(crate) fn builder() -> DirectiveRegistryBuilder {
        DirectiveRegistryBuilder::new()
    }

    pub(crate) fn with_case_insensitive(mut self, enabled: bool) -> Self {
        self.case_insensitive = enabled;
        self
    }

    /// Apply the OpenMP Fortran rule that blanks between adjacent keywords in
    /// a directive name may be omitted. [`LocatedDirective`] still retains the
    /// exact source slice so typed lowering can record compact-spelling
    /// provenance without storing a raw string in the AST.
    pub(crate) fn with_optional_keyword_whitespace(mut self, enabled: bool) -> Self {
        self.optional_keyword_whitespace = enabled;
        self
    }

    fn skip_trivia<'a>(&self, input: &'a str) -> IResult<&'a str, &'a str> {
        if self.case_insensitive {
            crate::lexer::skip_fortran_space_and_comments(input)
        } else {
            crate::lexer::skip_space_and_comments(input)
        }
    }

    fn matching_rule_name(&self, candidate: &str) -> Option<&'static str> {
        let exact = if self.case_insensitive {
            self.rules
                .keys()
                .copied()
                .find(|name| name.eq_ignore_ascii_case(candidate))
        } else {
            self.rules.keys().copied().find(|name| *name == candidate)
        };
        if exact.is_some() || !self.optional_keyword_whitespace {
            return exact;
        }

        // Reject ambiguous compact forms instead of allowing hash-map
        // iteration order to select a directive.
        let mut matches = self
            .rules
            .keys()
            .copied()
            .filter(|name| compact_name_eq(name, candidate));
        let matched = matches.next()?;
        matches.next().is_none().then_some(matched)
    }

    fn has_matching_prefix(&self, candidate: &str) -> bool {
        let exact = if self.case_insensitive {
            self.prefixes
                .iter()
                .any(|prefix| prefix.eq_ignore_ascii_case(candidate))
                || self
                    .rules
                    .keys()
                    .any(|name| name.eq_ignore_ascii_case(candidate))
        } else {
            self.prefixes.contains(candidate) || self.rules.contains_key(candidate)
        };
        exact
            || (self.optional_keyword_whitespace
                && (self
                    .prefixes
                    .iter()
                    .any(|prefix| compact_name_eq(prefix, candidate))
                    || self
                        .rules
                        .keys()
                        .any(|name| compact_name_eq(name, candidate))))
    }

    pub(crate) fn parse<'a>(
        &self,
        input: &'a str,
        clause_registry: &ClauseRegistry,
    ) -> IResult<&'a str, LocatedDirective<'a>> {
        let (rest, (name, name_source)) = self.lex_name(input)?;
        let (rest, syntax) = self.parse_with_name(name, rest, clause_registry)?;
        Ok((rest, LocatedDirective::new(syntax, name_source)))
    }

    pub(crate) fn parse_with_name<'a>(
        &self,
        name: Cow<'a, str>,
        input: &'a str,
        clause_registry: &ClauseRegistry,
    ) -> IResult<&'a str, Directive<'a>> {
        // Use efficient lookup based on case sensitivity mode
        let lookup_name = name.as_ref();
        let rule = if self.case_insensitive {
            // Case-insensitive lookup using eq_ignore_ascii_case (O(n) linear search)
            // Performance note: For small registries (~17 directives), linear search with
            // eq_ignore_ascii_case is optimal. Alternative (normalized HashMap) would require
            // building/maintaining a separate HashMap with lowercase keys (~memory overhead).
            // Benchmarking shows O(n) scan is faster than HashMap for n < ~50 items.
            self.rules
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(lookup_name))
                .map(|(_, v)| *v)
                .unwrap_or(self.default_rule)
        } else {
            // Direct HashMap lookup for case-sensitive mode (O(1), zero allocations)
            self.rules
                .get(lookup_name)
                .copied()
                .unwrap_or(self.default_rule)
        };

        rule.parse(name, input, clause_registry)
    }

    pub(crate) fn lex_name<'a>(&self, input: &'a str) -> IResult<&'a str, (Cow<'a, str>, &'a str)> {
        use crate::lexer::is_identifier_char as is_ident_char;

        let (after_trivia, _) = self.skip_trivia(input)?;
        let start = input.len() - after_trivia.len();

        let mut idx = start;
        let mut last_match = None;
        let mut candidate = String::new();

        while let Some(ch) = input[idx..].chars().next() {
            if !is_ident_char(ch) {
                break;
            }

            let token_start = idx;
            while let Some(ch2) = input[idx..].chars().next() {
                if !is_ident_char(ch2) {
                    break;
                }
                idx += ch2.len_utf8();
            }

            if !candidate.is_empty() {
                candidate.push(' ');
            }
            candidate.push_str(&input[token_start..idx]);
            if let Some(rule_name) = self.matching_rule_name(&candidate) {
                last_match = Some((idx, rule_name));
            }

            let (remaining, _) = self.skip_trivia(&input[idx..])?;
            let next = input.len() - remaining.len();

            if remaining.chars().next().is_some_and(is_ident_char)
                && self.has_matching_prefix(&candidate)
            {
                idx = next;
                continue;
            }

            break;
        }

        let (name_end, rule_name) = last_match
            .ok_or_else(|| nom::Err::Error(nom::error::Error::new(input, ErrorKind::Tag)))?;

        let raw_name = &input[start..name_end];
        let normalized = Cow::Borrowed(rule_name);

        let rest = &input[name_end..];

        Ok((rest, (normalized, raw_name)))
    }
}

impl Default for DirectiveRegistry {
    fn default() -> Self {
        DirectiveRegistry::builder()
            .register_generic("parallel")
            .build()
    }
}

pub(crate) struct DirectiveRegistryBuilder {
    rules: HashMap<&'static str, DirectiveRule>,
    prefixes: HashSet<String>,
    default_rule: DirectiveRule,
    case_insensitive: bool,
}

impl DirectiveRegistryBuilder {
    pub(crate) fn new() -> Self {
        Self {
            rules: HashMap::new(),
            prefixes: HashSet::new(),
            default_rule: DirectiveRule::Generic,
            case_insensitive: false,
        }
    }

    pub(crate) fn register_generic(mut self, name: &'static str) -> Self {
        self.insert_rule(name, DirectiveRule::Generic);
        self
    }

    pub(crate) fn register_custom(mut self, name: &'static str, parser: DirectiveParserFn) -> Self {
        self.insert_rule(name, DirectiveRule::Custom(parser));
        self
    }

    pub(crate) fn build(self) -> DirectiveRegistry {
        DirectiveRegistry {
            rules: self.rules,
            prefixes: self.prefixes,
            default_rule: self.default_rule,
            case_insensitive: self.case_insensitive,
            optional_keyword_whitespace: false,
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

fn compact_name_eq(left: &str, right: &str) -> bool {
    left.bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .map(|byte| byte.to_ascii_lowercase())
        .eq(right
            .bytes()
            .filter(|byte| !byte.is_ascii_whitespace())
            .map(|byte| byte.to_ascii_lowercase()))
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
            .parse_with_name("parallel".into(), " private(x, y) nowait", &clause_registry)
            .expect("parsing should succeed");

        assert_eq!(rest, "");
        assert_eq!(directive.name, "parallel");
        assert_eq!(directive.clauses.len(), 2);
        assert_eq!(directive.clauses[0].name, "private");
        assert_eq!(
            directive.clauses[0].kind,
            ClauseKind::Parenthesized("x, y".into())
        );
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

    #[test]
    fn comments_are_trivia_but_punctuation_is_not_repaired() {
        let clause_registry = ClauseRegistry::default();
        let registry = DirectiveRegistry::builder()
            .register_generic("target")
            .register_generic("target enter data")
            .build();

        let (rest, directive) = registry
            .parse(
                "target/* between keywords */enter data nowait",
                &clause_registry,
            )
            .expect("a comment may separate directive-name tokens");
        assert_eq!(rest, "");
        assert_eq!(directive.name, "target enter data");

        let (rest, directive) = registry
            .parse("target,enter data", &clause_registry)
            .expect("the valid prefix is still recognized");
        assert_eq!(directive.name, "target");
        assert_eq!(rest, ",enter data");
    }

    fn parse_prefixed_directive<'a>(
        name: Cow<'a, str>,
        input: &'a str,
        clause_registry: &ClauseRegistry,
    ) -> IResult<&'a str, Directive<'a>> {
        let (input, _) = tag("custom:")(input)?;
        let (input, clauses) = clause_registry.parse_sequence(input)?;

        Ok((
            input,
            Directive {
                name: name.into(),
                parameter: None,
                clauses,
            },
        ))
    }

    #[test]
    fn supports_custom_directive_rule() {
        let clause_registry = ClauseRegistry::default();
        let registry = DirectiveRegistry::builder()
            .register_custom("target", parse_prefixed_directive)
            .build();

        let (rest, directive) = registry
            .parse_with_name("target".into(), "custom: private(a)", &clause_registry)
            .expect("parsing should succeed");

        assert_eq!(rest, "");
        assert_eq!(directive.name, "target");
        assert_eq!(directive.clauses.len(), 1);
        assert_eq!(directive.clauses[0].name, "private");
        assert_eq!(
            directive.clauses[0].kind,
            ClauseKind::Parenthesized("a".into())
        );
    }
}
