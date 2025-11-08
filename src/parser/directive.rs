use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt,
};

use nom::{error::ErrorKind, IResult};

use super::clause::{Clause, ClauseRegistry};

type DirectiveParserFn =
    for<'a> fn(Cow<'a, str>, &'a str, &ClauseRegistry) -> IResult<&'a str, Directive<'a>>;

#[derive(Debug, PartialEq, Eq)]
pub struct WaitDirectiveData<'a> {
    pub devnum: Option<Cow<'a, str>>,
    pub has_queues: bool,
    pub queue_exprs: Vec<Cow<'a, str>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CacheDirectiveData<'a> {
    pub readonly: bool,
    pub variables: Vec<Cow<'a, str>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Directive<'a> {
    pub name: Cow<'a, str>,
    pub parameter: Option<Cow<'a, str>>,
    pub clauses: Vec<Clause<'a>>,
    // Structured data for specific directive types
    pub wait_data: Option<WaitDirectiveData<'a>>,
    pub cache_data: Option<CacheDirectiveData<'a>>,
}

impl<'a> Directive<'a> {
    pub fn new(
        name: Cow<'a, str>,
        parameter: Option<Cow<'a, str>>,
        clauses: Vec<Clause<'a>>,
    ) -> Self {
        Self {
            name,
            parameter,
            clauses,
            wait_data: None,
            cache_data: None,
        }
    }

    /// Merge duplicate clauses and deduplicate variables (accparser compatibility)
    ///
    /// For clauses that can appear multiple times (gang, worker, vector, reduction, etc.),
    /// this merges them into a single clause with deduplicated variables/expressions.
    ///
    /// Example: `gang(a,b) gang(b,c)` becomes `gang(a,b,c)`
    pub fn merge_clauses(&mut self) {
        use super::clause::{Clause, ClauseKind};
        use std::collections::{HashMap, HashSet};

        // Group clauses by name AND modifier/kind for merging
        // Key: (name, kind_discriminant, modifier_value)
        type MergeKey = (String, u8, u32);
        let mut merged: HashMap<MergeKey, Vec<Clause<'a>>> = HashMap::new();

        for clause in self.clauses.drain(..) {
            // Create key based on clause name, kind, and modifier/content
            // For Parenthesized clauses, we hash the content to distinguish collapse(1) from collapse(2)
            let kind_disc = match &clause.kind {
                ClauseKind::Bare => 0,
                ClauseKind::Parenthesized(content) => {
                    // Use hash of content to distinguish different parameter values
                    use std::collections::hash_map::DefaultHasher;
                    use std::hash::{Hash, Hasher};
                    let mut hasher = DefaultHasher::new();
                    content.as_ref().hash(&mut hasher);
                    1_000_000 + (hasher.finish() % 1_000_000) as u32
                }
                ClauseKind::VariableList(_) => 2,
                ClauseKind::GangClause {
                    modifier,
                    variables,
                } => {
                    if variables.is_empty() {
                        0 // Bare gang - same as Bare
                    } else {
                        3 + modifier.map_or(0, |m| m as u32)
                    }
                }
                ClauseKind::WorkerClause {
                    modifier,
                    variables,
                } => {
                    if variables.is_empty() {
                        0 // Bare worker
                    } else {
                        10 + modifier.map_or(0, |m| m as u32)
                    }
                }
                ClauseKind::VectorClause {
                    modifier,
                    variables,
                } => {
                    if variables.is_empty() {
                        0 // Bare vector
                    } else {
                        20 + modifier.map_or(0, |m| m as u32)
                    }
                }
                ClauseKind::ReductionClause { operator, .. } => 30 + *operator as u32,
                ClauseKind::CopyinClause { modifier, .. } => 40 + modifier.map_or(0, |m| m as u32),
                ClauseKind::CopyoutClause { modifier, .. } => 50 + modifier.map_or(0, |m| m as u32),
                ClauseKind::CreateClause { modifier, .. } => 60 + modifier.map_or(0, |m| m as u32),
            };

            let key = (clause.name.to_string(), kind_disc as u8, kind_disc);
            merged.entry(key).or_default().push(clause);
        }

        // Rebuild clauses list with merging
        let mut new_clauses = Vec::new();
        for (_, group) in merged {
            if group.len() == 1 {
                // Single occurrence - keep as is
                new_clauses.push(group.into_iter().next().unwrap());
            } else {
                // Multiple occurrences - merge based on clause kind
                let first = group[0].clone();
                match &first.kind {
                    ClauseKind::GangClause { .. }
                    | ClauseKind::WorkerClause { .. }
                    | ClauseKind::VectorClause { .. }
                    | ClauseKind::VariableList(_) => {
                        // Merge variable lists with deduplication
                        let mut vars = Vec::new();
                        let mut seen = HashSet::new();
                        for clause in &group {
                            let clause_vars = match &clause.kind {
                                ClauseKind::VariableList(v) => v.as_slice(),
                                ClauseKind::GangClause { variables, .. } => variables.as_slice(),
                                ClauseKind::WorkerClause { variables, .. } => variables.as_slice(),
                                ClauseKind::VectorClause { variables, .. } => variables.as_slice(),
                                _ => &[],
                            };
                            for var in clause_vars {
                                let var_str = var.as_ref();
                                if seen.insert(var_str.to_string()) {
                                    vars.push(Cow::Owned(var_str.to_string()));
                                }
                            }
                        }
                        // Create merged clause with same structure as first
                        let merged_clause = match &first.kind {
                            ClauseKind::VariableList(_) => Clause {
                                name: first.name.clone(),
                                kind: ClauseKind::VariableList(vars),
                            },
                            ClauseKind::GangClause { modifier, .. } => Clause {
                                name: first.name.clone(),
                                kind: ClauseKind::GangClause {
                                    modifier: *modifier,
                                    variables: vars,
                                },
                            },
                            ClauseKind::WorkerClause { modifier, .. } => Clause {
                                name: first.name.clone(),
                                kind: ClauseKind::WorkerClause {
                                    modifier: *modifier,
                                    variables: vars,
                                },
                            },
                            ClauseKind::VectorClause { modifier, .. } => Clause {
                                name: first.name.clone(),
                                kind: ClauseKind::VectorClause {
                                    modifier: *modifier,
                                    variables: vars,
                                },
                            },
                            _ => unreachable!(),
                        };
                        new_clauses.push(merged_clause);
                    }
                    ClauseKind::ReductionClause { operator, .. } => {
                        // Merge reduction clauses with same operator
                        let mut vars = Vec::new();
                        let mut seen = HashSet::new();
                        for clause in &group {
                            if let ClauseKind::ReductionClause {
                                operator: op,
                                variables,
                            } = &clause.kind
                            {
                                if op == operator {
                                    for var in variables {
                                        let var_str = var.as_ref();
                                        if seen.insert(var_str.to_string()) {
                                            vars.push(Cow::Owned(var_str.to_string()));
                                        }
                                    }
                                }
                            }
                        }
                        new_clauses.push(Clause {
                            name: first.name.clone(),
                            kind: ClauseKind::ReductionClause {
                                operator: *operator,
                                variables: vars,
                            },
                        });
                    }
                    ClauseKind::CopyinClause { .. }
                    | ClauseKind::CopyoutClause { .. }
                    | ClauseKind::CreateClause { .. } => {
                        // Merge data clauses with same modifier
                        let mut vars = Vec::new();
                        let mut seen = HashSet::new();
                        for clause in &group {
                            let clause_vars = match &clause.kind {
                                ClauseKind::CopyinClause { variables, .. } => variables.as_slice(),
                                ClauseKind::CopyoutClause { variables, .. } => variables.as_slice(),
                                ClauseKind::CreateClause { variables, .. } => variables.as_slice(),
                                _ => &[],
                            };
                            for var in clause_vars {
                                let var_str = var.as_ref();
                                if seen.insert(var_str.to_string()) {
                                    vars.push(Cow::Owned(var_str.to_string()));
                                }
                            }
                        }
                        let merged_clause = match &first.kind {
                            ClauseKind::CopyinClause { modifier, .. } => Clause {
                                name: first.name.clone(),
                                kind: ClauseKind::CopyinClause {
                                    modifier: *modifier,
                                    variables: vars,
                                },
                            },
                            ClauseKind::CopyoutClause { modifier, .. } => Clause {
                                name: first.name.clone(),
                                kind: ClauseKind::CopyoutClause {
                                    modifier: *modifier,
                                    variables: vars,
                                },
                            },
                            ClauseKind::CreateClause { modifier, .. } => Clause {
                                name: first.name.clone(),
                                kind: ClauseKind::CreateClause {
                                    modifier: *modifier,
                                    variables: vars,
                                },
                            },
                            _ => unreachable!(),
                        };
                        new_clauses.push(merged_clause);
                    }
                    _ => {
                        // For other clause types, keep only the first occurrence
                        new_clauses.push(first.clone());
                    }
                }
            }
        }

        self.clauses = new_clauses;
    }

    pub fn to_pragma_string(&self) -> String {
        self.to_string()
    }

    /// Convert directive to pragma string with custom prefix
    ///
    /// # Example
    /// ```
    /// # use roup::parser::{Directive, Clause, ClauseKind};
    /// # use std::borrow::Cow;
    /// let directive = Directive {
    ///     name: Cow::Borrowed("parallel"),
    ///     parameter: None,
    ///     clauses: vec![],
    ///     cache_data: None,
    ///     wait_data: None,
    /// };
    /// assert_eq!(directive.to_pragma_string_with_prefix("#pragma acc"), "#pragma acc parallel");
    /// ```
    pub fn to_pragma_string_with_prefix(&self, prefix: &str) -> String {
        // Default to no commas for backward compatibility (OpenMP style)
        self.to_pragma_string_with_prefix_and_separator(prefix, false)
    }

    /// Convert directive to pragma string with custom prefix and clause separator
    ///
    /// # Example
    /// ```
    /// # use roup::parser::{Directive, Clause, ClauseKind};
    /// # use std::borrow::Cow;
    /// let directive = Directive {
    ///     name: Cow::Borrowed("parallel"),
    ///     parameter: None,
    ///     clauses: vec![
    ///         Clause { name: Cow::Borrowed("async"), kind: ClauseKind::Parenthesized(Cow::Borrowed("1")) },
    ///         Clause { name: Cow::Borrowed("wait"), kind: ClauseKind::Parenthesized(Cow::Borrowed("2")) },
    ///     ],
    ///     cache_data: None,
    ///     wait_data: None,
    /// };
    /// assert_eq!(directive.to_pragma_string_with_prefix_and_separator("#pragma acc", true), "#pragma acc parallel async(1), wait(2)");
    /// ```
    pub fn to_pragma_string_with_prefix_and_separator(
        &self,
        prefix: &str,
        use_commas: bool,
    ) -> String {
        let mut output = String::new();
        output.push_str(prefix);
        output.push(' ');
        output.push_str(self.name.as_ref());
        render_parameter_into(&mut output, self.parameter.as_deref());
        if !self.clauses.is_empty() {
            output.push(' ');
            for (idx, clause) in self.clauses.iter().enumerate() {
                if idx > 0 {
                    if use_commas {
                        output.push(',');
                    }
                    output.push(' ');
                }
                output.push_str(&clause.to_string());
            }
        }
        output
    }
}

impl fmt::Display for Directive<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#pragma omp {}", self.name.as_ref())?;
        if let Some(param) = &self.parameter {
            // Use the same rendering rules as the helper to avoid divergence
            let mut tmp = String::new();
            render_parameter_into(&mut tmp, Some(param.as_ref()));
            write!(f, "{}", tmp)?;
        }
        if !self.clauses.is_empty() {
            write!(f, " ")?;
            for (idx, clause) in self.clauses.iter().enumerate() {
                if idx > 0 {
                    write!(f, " ")?;
                }
                write!(f, "{clause}")?;
            }
        }
        Ok(())
    }
}

// Helper to render a parameter into the provided output buffer.
// Inserts a separating space before the parameter unless it already
// starts with '(' or a leading space. Centralizing the rule avoids
// duplication between different render paths.
fn render_parameter_into(output: &mut String, param: Option<&str>) {
    if let Some(p) = param {
        if p.starts_with('(') || p.starts_with(' ') {
            output.push_str(p);
        } else {
            output.push(' ');
            output.push_str(p);
        }
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

pub struct DirectiveRegistry {
    rules: HashMap<&'static str, DirectiveRule>,
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

    fn lex_name<'a>(&self, input: &'a str) -> IResult<&'a str, Cow<'a, str>> {
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
            let candidate = crate::lexer::collapse_line_continuations(candidate);
            let candidate_ref = candidate.as_ref().trim();
            // Check if this candidate matches any registered directive
            let has_rule = if self.case_insensitive {
                self.rules
                    .keys()
                    .any(|k| k.eq_ignore_ascii_case(candidate_ref))
            } else {
                self.rules.contains_key(candidate_ref)
            };

            if has_rule {
                last_match_end = Some(j);
            }

            // advance idx past any whitespace following the identifier
            idx = j;
            if let Ok((remaining, _)) = crate::lexer::skip_space_and_comments(&input[idx..]) {
                let consumed = input[idx..].len() - remaining.len();
                idx += consumed;
            }

            // if next character starts an identifier, loop to extend candidate
            if let Some((_, next_ch)) = input[idx..].char_indices().next() {
                if is_ident_char(next_ch) {
                    // check if prefix is registered; if so, continue to extend
                    let prefix_candidate = input[start..idx].trim_end();
                    let prefix_candidate =
                        crate::lexer::collapse_line_continuations(prefix_candidate);
                    let prefix_candidate_ref = prefix_candidate.as_ref().trim_end();
                    // Check for prefixes
                    let has_prefix = if self.case_insensitive {
                        self.prefixes
                            .iter()
                            .any(|p| p.eq_ignore_ascii_case(prefix_candidate_ref))
                            || self
                                .rules
                                .keys()
                                .any(|k| k.eq_ignore_ascii_case(prefix_candidate_ref))
                    } else {
                        self.prefixes.contains(prefix_candidate_ref)
                            || self.rules.contains_key(prefix_candidate_ref)
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

        let raw_name = &input[start..name_end];
        let normalized = crate::lexer::collapse_line_continuations(raw_name);
        let normalized = if self.case_insensitive {
            let lowered = normalized.as_ref().to_ascii_lowercase();
            if lowered == normalized.as_ref() {
                normalized
            } else {
                Cow::Owned(lowered)
            }
        } else {
            normalized
        };

        let rest = &input[name_end..];

        Ok((rest, normalized))
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
        DirectiveRegistry {
            rules: self.rules,
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
                name,
                parameter: None,
                clauses,
                wait_data: None,
                cache_data: None,
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

    #[test]
    fn directive_display_includes_all_clauses() {
        let directive = Directive {
            name: "parallel".into(),
            parameter: None,
            clauses: vec![
                Clause {
                    name: "private".into(),
                    kind: ClauseKind::Parenthesized("a, b".into()),
                },
                Clause {
                    name: "nowait".into(),
                    kind: ClauseKind::Bare,
                },
            ],
            wait_data: None,
            cache_data: None,
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
            name: "barrier".into(),
            parameter: None,
            clauses: vec![],
            wait_data: None,
            cache_data: None,
        };

        assert_eq!(directive.to_string(), "#pragma omp barrier");
        assert_eq!(directive.to_pragma_string(), "#pragma omp barrier");
    }
}
