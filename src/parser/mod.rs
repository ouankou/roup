pub mod clause;
mod directive;
pub mod directive_kind;
pub mod openacc;
pub mod openmp;

pub use clause::{
    lookup_clause_name, Clause, ClauseKind, ClauseName, ClauseRegistry, ClauseRegistryBuilder,
    ClauseRule, CopyinModifier, CopyoutModifier, CreateModifier, GangModifier, ReductionOperator,
    VectorModifier, WorkerModifier,
};
pub use directive::{
    CacheDirectiveData, Directive, DirectiveRegistry, DirectiveRegistryBuilder, DirectiveRule,
    WaitDirectiveData,
};

use super::lexer::{self, Language};
use nom::{IResult, Parser as _};

pub struct Parser {
    clause_registry: ClauseRegistry,
    directive_registry: DirectiveRegistry,
    language: Language,
    dialect: Dialect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dialect {
    OpenMp,
    OpenAcc,
}

/// Skip duplicate prefix keyword after sentinel (e.g., "omp" after "!$omp")
/// This provides forgiving parsing for malformed directives like "!$omp omp teams"
fn skip_duplicate_keyword<'a>(input: &'a str, prefix: &str) -> &'a str {
    // Check if the input starts with the prefix (case-insensitive)
    // followed by whitespace or end of string
    if input.len() >= prefix.len()
        && input[..prefix.len()].eq_ignore_ascii_case(prefix)
        && (input.len() == prefix.len()
            || input
                .chars()
                .nth(prefix.len())
                .map_or(false, |c| c.is_whitespace()))
    {
        // Skip the prefix and any following whitespace
        let after_prefix = &input[prefix.len()..];
        after_prefix.trim_start()
    } else {
        input
    }
}

impl Parser {
    pub fn new(directive_registry: DirectiveRegistry, clause_registry: ClauseRegistry) -> Self {
        Self {
            clause_registry,
            directive_registry,
            language: Language::default(),
            dialect: Dialect::OpenMp,
        }
    }

    pub fn with_language(mut self, language: Language) -> Self {
        self.language = language;

        // Enable case-insensitive matching for Fortran
        // C language uses default case-sensitive matching (no changes needed)
        if matches!(language, Language::FortranFree | Language::FortranFixed) {
            self.directive_registry = self.directive_registry.with_case_insensitive(true);
            self.clause_registry = self.clause_registry.with_case_insensitive(true);
        }

        self
    }

    pub fn with_dialect(mut self, dialect: Dialect) -> Self {
        self.dialect = dialect;
        self
    }

    pub fn parse<'a>(&self, input: &'a str) -> IResult<&'a str, Directive<'a>> {
        // IMPORTANT: ROUP normalizes continuation markers before parsing
        //
        // Supported continuation forms:
        // - C / C++: trailing backslash (`\`) merges the next line
        // - Fortran: trailing `&` with optional sentinel on the following line
        //
        // The lexer collapses these continuations into a single logical line so the
        // directive and clause registries operate on canonical whitespace.

        let input = match self.language {
            Language::C => {
                let (input, _) = (
                    lexer::lex_pragma,
                    lexer::skip_space1_and_comments,
                    |i| match self.dialect {
                        Dialect::OpenMp => lexer::lex_dialect_keyword(i, "omp"),
                        Dialect::OpenAcc => lexer::lex_dialect_keyword(i, "acc"),
                    },
                    lexer::skip_space1_and_comments,
                )
                    .parse(input)?;
                input
            }
            Language::FortranFree => {
                let (mut input, _) = (
                    |i| match self.dialect {
                        Dialect::OpenMp => lexer::lex_fortran_free_sentinel_with_prefix(i, "omp"),
                        Dialect::OpenAcc => lexer::lex_fortran_free_sentinel_with_prefix(i, "acc"),
                    },
                    lexer::skip_space1_and_comments,
                )
                    .parse(input)?;

                // Skip duplicate prefix keyword if present (e.g., "!$omp omp teams" -> "!$omp teams")
                // This provides forgiving parsing for malformed directives
                let prefix = match self.dialect {
                    Dialect::OpenMp => "omp",
                    Dialect::OpenAcc => "acc",
                };
                input = skip_duplicate_keyword(input, prefix);
                input
            }
            Language::FortranFixed => {
                let (mut input, _) = (
                    |i| match self.dialect {
                        Dialect::OpenMp => lexer::lex_fortran_fixed_sentinel_with_prefix(i, "omp"),
                        Dialect::OpenAcc => lexer::lex_fortran_fixed_sentinel_with_prefix(i, "acc"),
                    },
                    lexer::skip_space1_and_comments,
                )
                    .parse(input)?;

                // Skip duplicate prefix keyword if present (e.g., "!$omp omp teams" -> "!$omp teams")
                // This provides forgiving parsing for malformed directives
                let prefix = match self.dialect {
                    Dialect::OpenMp => "omp",
                    Dialect::OpenAcc => "acc",
                };
                input = skip_duplicate_keyword(input, prefix);
                input
            }
        };
        self.directive_registry.parse(input, &self.clause_registry)
    }
}

impl Default for Parser {
    fn default() -> Self {
        openmp::parser()
    }
}

pub fn parse_omp_directive(input: &str) -> IResult<&str, Directive<'_>> {
    // Try the default C-style parser first for performance and compatibility.
    // If that fails, attempt Fortran free-form and fixed-form parsers so callers
    // using the convenience function `parse_omp_directive` can parse Fortran
    // sentinel forms (e.g. "!$omp ...") without having to construct a
    // language-specific parser manually.
    let c_parser = Parser::default();
    match c_parser.parse(input) {
        Ok((rest, dir)) => Ok((rest, dir)),
        Err(_) => {
            // Try Fortran free-form
            let ff_parser = Parser::default().with_language(Language::FortranFree);
            match ff_parser.parse(input) {
                Ok((rest, dir)) => Ok((rest, dir)),
                Err(_) => {
                    // Try Fortran fixed-form as a last resort
                    let fx_parser = Parser::default().with_language(Language::FortranFixed);
                    fx_parser.parse(input)
                }
            }
        }
    }
}

pub fn parse_acc_directive(input: &str) -> IResult<&str, Directive<'_>> {
    let parser = openacc::parser();
    parser.parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::{self, Language};
    use std::borrow::Cow;

    #[test]
    fn parses_full_pragma_with_default_registries() {
        let input = "#pragma omp parallel private(a, b) nowait";

        let (rest, directive) = parse_omp_directive(input).expect("parsing should succeed");

        assert_eq!(rest, "");
        assert_eq!(directive.name, "parallel");
        assert_eq!(directive.clauses.len(), 2);
        assert_eq!(directive.clauses[0].name, "private");
        assert_eq!(
            directive.clauses[0].kind,
            ClauseKind::Parenthesized("a, b".into())
        );
        assert_eq!(directive.clauses[1].name, "nowait");
        assert_eq!(directive.clauses[1].kind, ClauseKind::Bare);
    }

    #[test]
    fn parser_uses_custom_registries() {
        fn parse_only_bare<'a>(name: Cow<'a, str>, input: &'a str) -> IResult<&'a str, Clause<'a>> {
            let (input, _) = nom::character::complete::char('(')(input)?;
            let (input, value) = lexer::lex_clause(input)?;
            let (input, _) = nom::character::complete::char(')')(input)?;

            Ok((
                input,
                Clause {
                    name,
                    kind: ClauseKind::Parenthesized(value.into()),
                },
            ))
        }

        let clause_registry = ClauseRegistry::builder()
            .register_custom("device", parse_only_bare)
            .build();

        fn parse_prefixed<'a>(
            name: Cow<'a, str>,
            input: &'a str,
            clause_registry: &ClauseRegistry,
        ) -> IResult<&'a str, Directive<'a>> {
            let (input, _) = (
                nom::character::complete::multispace1,
                nom::bytes::complete::tag("use:"),
                nom::character::complete::multispace1,
            )
                .parse(input)?;
            let (input, clauses) = clause_registry.parse_sequence(input)?;

            Ok((
                input,
                Directive {
                    name: name.into(),
                    parameter: None,
                    clauses,
                    wait_data: None,
                    cache_data: None,
                },
            ))
        }

        let directive_registry = DirectiveRegistry::builder()
            .register_custom("target", parse_prefixed)
            .build();

        let parser = Parser::new(directive_registry, clause_registry);

        let (rest, directive) = parser
            .parse("#pragma omp target use: device(gpu)")
            .expect("parsing should succeed");

        assert_eq!(rest, "");
        assert_eq!(directive.name, "target");
        assert_eq!(directive.clauses.len(), 1);
        assert_eq!(directive.clauses[0].name, "device");
        assert_eq!(
            directive.clauses[0].kind,
            ClauseKind::Parenthesized("gpu".into())
        );
    }

    #[test]
    fn parses_c_multiline_directive_with_backslash() {
        let input = "#pragma omp parallel for \
            private(a, \
                    b) \
            nowait";
        let parser = Parser::default();
        let (_, directive) = parser.parse(input).expect("directive should parse");

        assert_eq!(directive.name, "parallel for");
        assert_eq!(directive.clauses.len(), 2);
        assert_eq!(directive.clauses[0].name, "private");
        assert_eq!(
            directive.clauses[0].kind,
            ClauseKind::Parenthesized("a, b".into())
        );
        assert_eq!(directive.clauses[1].name, "nowait");
        assert_eq!(directive.clauses[1].kind, ClauseKind::Bare);
    }

    #[test]
    fn parses_fortran_free_multiline_directive() {
        let parser = Parser::default().with_language(Language::FortranFree);
        let input = "!$omp target teams distribute &\n!$omp parallel do &\n!$omp& private(i, j)";

        let (_, directive) = parser.parse(input).expect("directive should parse");

        assert_eq!(directive.name, "target teams distribute parallel do");
        assert_eq!(directive.clauses.len(), 1);
        assert_eq!(directive.clauses[0].name, "private");
        assert_eq!(
            directive.clauses[0].kind,
            ClauseKind::Parenthesized("i, j".into())
        );
    }

    #[test]
    fn parses_fortran_parenthesized_clause_with_continuation() {
        let parser = Parser::default().with_language(Language::FortranFree);
        let input = "!$omp parallel do private(i, &\n!$omp& j, &\n!$omp& k)";

        let (_, directive) = parser.parse(input).expect("directive should parse");

        assert_eq!(directive.name, "parallel do");
        assert_eq!(directive.clauses.len(), 1);
        assert_eq!(directive.clauses[0].name, "private");
        assert_eq!(
            directive.clauses[0].kind,
            ClauseKind::Parenthesized("i, j, k".into())
        );
    }

    #[test]
    fn parses_fortran_fixed_multiline_directive() {
        let parser = Parser::default().with_language(Language::FortranFixed);
        let input = "      C$OMP   DO &\n      !$OMP& SCHEDULE(DYNAMIC) &\n      !$OMP PRIVATE(I)";

        let (_, directive) = parser.parse(input).expect("directive should parse");

        assert_eq!(directive.name, "do");
        assert_eq!(directive.clauses.len(), 2);
        assert_eq!(directive.clauses[0].name, "schedule");
        assert_eq!(
            directive.clauses[0].kind,
            ClauseKind::Parenthesized("DYNAMIC".into())
        );
        assert_eq!(directive.clauses[1].name, "private");
        assert_eq!(
            directive.clauses[1].kind,
            ClauseKind::Parenthesized("I".into())
        );
    }
}
