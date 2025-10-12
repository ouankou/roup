mod clause;
mod directive;
pub mod openmp;

pub use clause::{Clause, ClauseKind, ClauseRegistry, ClauseRegistryBuilder, ClauseRule};
pub use directive::{Directive, DirectiveRegistry, DirectiveRegistryBuilder, DirectiveRule};

use super::lexer::{self, Language};
use nom::{IResult, Parser as _};

pub struct Parser {
    clause_registry: ClauseRegistry,
    directive_registry: DirectiveRegistry,
    language: Language,
}

impl Parser {
    pub fn new(directive_registry: DirectiveRegistry, clause_registry: ClauseRegistry) -> Self {
        Self {
            clause_registry,
            directive_registry,
            language: Language::default(),
        }
    }

    pub fn with_language(mut self, language: Language) -> Self {
        self.language = language;
        
        // Enable case-insensitive matching for Fortran
        match language {
            Language::FortranFree | Language::FortranFixed => {
                self.directive_registry = self.directive_registry.with_case_insensitive(true);
                self.clause_registry = self.clause_registry.with_case_insensitive(true);
            }
            Language::C => {}
        }
        
        self
    }

    pub fn parse<'a>(&self, input: &'a str) -> IResult<&'a str, Directive<'a>> {
        let input = match self.language {
            Language::C => {
                let (input, _) = (
                    lexer::lex_pragma,
                    lexer::skip_space1_and_comments,
                    lexer::lex_omp,
                    lexer::skip_space1_and_comments,
                )
                    .parse(input)?;
                input
            }
            Language::FortranFree => {
                let (input, _) = (
                    lexer::lex_fortran_free_sentinel,
                    lexer::skip_space1_and_comments,
                )
                    .parse(input)?;
                input
            }
            Language::FortranFixed => {
                let (input, _) = (
                    lexer::lex_fortran_fixed_sentinel,
                    lexer::skip_space1_and_comments,
                )
                    .parse(input)?;
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
    let parser = Parser::default();
    parser.parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer;

    #[test]
    fn parses_full_pragma_with_default_registries() {
        let input = "#pragma omp parallel private(a, b) nowait";

        let (rest, directive) = parse_omp_directive(input).expect("parsing should succeed");

        assert_eq!(rest, "");
        assert_eq!(directive.name, "parallel");
        assert_eq!(directive.clauses.len(), 2);
        assert_eq!(directive.clauses[0].name, "private");
        assert_eq!(directive.clauses[0].kind, ClauseKind::Parenthesized("a, b"));
        assert_eq!(directive.clauses[1].name, "nowait");
        assert_eq!(directive.clauses[1].kind, ClauseKind::Bare);
    }

    #[test]
    fn parser_uses_custom_registries() {
        fn parse_only_bare<'a>(name: &'a str, input: &'a str) -> IResult<&'a str, Clause<'a>> {
            let (input, _) = nom::character::complete::char('(')(input)?;
            let (input, value) = lexer::lex_clause(input)?;
            let (input, _) = nom::character::complete::char(')')(input)?;

            Ok((
                input,
                Clause {
                    name,
                    kind: ClauseKind::Parenthesized(value),
                },
            ))
        }

        let clause_registry = ClauseRegistry::builder()
            .register_custom("device", parse_only_bare)
            .build();

        fn parse_prefixed<'a>(
            name: &'a str,
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

            Ok((input, Directive { name, clauses }))
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
        assert_eq!(directive.clauses[0].kind, ClauseKind::Parenthesized("gpu"));
    }
}
