#![forbid(unsafe_code)]

pub(crate) mod ast_builder;
pub(crate) mod clause;
mod directive;
pub(crate) mod directive_kind;
pub(crate) mod openacc;
pub(crate) mod openmp;
pub(crate) mod semantic;

pub(crate) use ast_builder::AstBuildError;

pub(crate) use clause::{
    Clause, ClauseKind, ClauseName, ClauseRegistry, ClauseRegistryBuilder, ClauseRule,
    lookup_clause_name,
};
pub(crate) use directive::{
    Directive, DirectiveRegistry, DirectiveRegistryBuilder, LocatedDirective,
};

use super::lexer::{self, Language};
use crate::ast::{OmpDirective, OmpDirectiveKind, RoupDirective};
use crate::ir::ParserConfig;
use nom::{IResult, Parser as _};

pub(crate) struct Parser {
    clause_registry: ClauseRegistry,
    directive_registry: DirectiveRegistry,
    language: Language,
    dialect: Dialect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Dialect {
    OpenMp,
    OpenAcc,
}

impl Dialect {
    const fn keyword(self) -> &'static str {
        match self {
            Self::OpenMp => "omp",
            Self::OpenAcc => "acc",
        }
    }
}

impl Parser {
    pub(crate) fn new(
        directive_registry: DirectiveRegistry,
        clause_registry: ClauseRegistry,
        language: Language,
        dialect: Dialect,
    ) -> Self {
        Self {
            clause_registry,
            directive_registry,
            language,
            dialect,
        }
    }

    pub(crate) fn with_language(mut self, language: Language) -> Self {
        self.language = language;

        // Enable case-insensitive matching for Fortran
        // C language uses default case-sensitive matching (no changes needed)
        if matches!(language, Language::FortranFree | Language::FortranFixed) {
            self.directive_registry = self.directive_registry.with_case_insensitive(true);
            self.clause_registry = self.clause_registry.with_case_insensitive(true);
            if self.dialect == Dialect::OpenMp {
                self.directive_registry = self
                    .directive_registry
                    .with_optional_keyword_whitespace(true);
            }
        }

        self
    }

    fn skip_trivia<'a>(&self, input: &'a str) -> IResult<&'a str, &'a str> {
        match self.language {
            Language::C => lexer::skip_space_and_comments(input),
            Language::FortranFree | Language::FortranFixed => {
                lexer::skip_fortran_space_and_comments(input)
            }
        }
    }

    pub(crate) fn parse<'a>(&self, input: &'a str) -> IResult<&'a str, LocatedDirective<'a>> {
        // IMPORTANT: ROUP normalizes continuation markers before parsing
        //
        // Supported continuation forms:
        // - C / C++: trailing backslash (`\`) merges the next line
        // - Fortran: trailing `&` with optional sentinel on the following line
        //
        // The lexer collapses these continuations into a single logical line so the
        // directive and clause registries operate on canonical whitespace.

        if let Err(offset) =
            lexer::validate_logical_line(input, self.language, self.dialect.keyword())
        {
            return Err(nom::Err::Failure(nom::error::Error::new(
                &input[offset..],
                nom::error::ErrorKind::Verify,
            )));
        }

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
                let (input, _) = (
                    |i| match self.dialect {
                        Dialect::OpenMp => lexer::lex_fortran_free_sentinel_with_prefix(i, "omp"),
                        Dialect::OpenAcc => lexer::lex_fortran_free_sentinel_with_prefix(i, "acc"),
                    },
                    lexer::skip_fortran_space1_and_comments,
                )
                    .parse(input)?;

                input
            }
            Language::FortranFixed => {
                let (input, _) = (
                    |i| match self.dialect {
                        Dialect::OpenMp => lexer::lex_fortran_fixed_sentinel_with_prefix(i, "omp"),
                        Dialect::OpenAcc => lexer::lex_fortran_fixed_sentinel_with_prefix(i, "acc"),
                    },
                    lexer::skip_fortran_space1_and_comments,
                )
                    .parse(input)?;

                input
            }
        };
        self.directive_registry.parse(input, &self.clause_registry)
    }

    pub(crate) fn parse_ast(
        &self,
        input: &str,
        parser_config: &ParserConfig,
    ) -> Result<RoupDirective, AstBuildError> {
        if self.dialect == Dialect::OpenMp
            && parser_config.source_compatibility()
            && matches!(
                self.language,
                Language::FortranFree | Language::FortranFixed
            )
        {
            let sentinel = match self.language {
                Language::FortranFree => {
                    lexer::lex_fortran_free_sentinel_with_prefix(input, "ompx")
                }
                Language::FortranFixed => {
                    lexer::lex_fortran_fixed_sentinel_with_prefix(input, "ompx")
                }
                Language::C => unreachable!(),
            };
            if sentinel.is_ok() {
                let logical = lexer::LogicalSource::new(input, self.language, "ompx")
                    .map_err(|error| AstBuildError::ParseFailure(error.to_string()))?;
                let (remaining, sentinel_source) = match self.language {
                    Language::FortranFree => {
                        lexer::lex_fortran_free_sentinel_with_prefix(logical.text(), "ompx")
                    }
                    Language::FortranFixed => {
                        lexer::lex_fortran_fixed_sentinel_with_prefix(logical.text(), "ompx")
                    }
                    Language::C => unreachable!(),
                }
                .map_err(|error| AstBuildError::ParseFailure(format!("{error:?}")))?;
                if !remaining.chars().next().is_some_and(char::is_whitespace) {
                    return Err(AstBuildError::ParseFailure(
                        "OMPX sentinel must be followed by whitespace and a payload".to_string(),
                    ));
                }
                let payload = remaining.trim();
                if payload.is_empty() {
                    return Err(AstBuildError::ParseFailure(
                        "OMPX sentinel requires a non-empty typed payload".to_string(),
                    ));
                }
                return ast_builder::build_ompx_directive(
                    payload,
                    sentinel_source,
                    parser_config,
                    &logical,
                );
            }
        }
        let ir_language = parser_config.language();
        let logical = lexer::LogicalSource::new(input, self.language, self.dialect.keyword())
            .map_err(|error| AstBuildError::ParseFailure(error.to_string()))?;
        let (rest, directive) = self
            .parse(logical.text())
            .map_err(|err| AstBuildError::ParseFailure(format!("{err:?}")))?;

        // Reject trailing tokens once a directive has been parsed to prevent
        // accidentally accepting malformed pragmas like "safelen" (missing
        // parentheses) or bare branch hints with leftover text.
        let rest = self
            .skip_trivia(rest)
            .map(|(remaining, _)| remaining)
            .map_err(|error| AstBuildError::ParseFailure(format!("{error:?}")))?;
        let trimmed_rest = rest.trim();
        if !trimmed_rest.is_empty() {
            return Err(AstBuildError::ParseFailure(format!(
                "unexpected trailing tokens: {trimmed_rest:?}"
            )));
        }

        ast_builder::build_roup_directive(
            &directive,
            self.dialect,
            parser_config,
            ir_language,
            &logical,
        )
    }

    /// Parse a directive body that has already been isolated from its pragma or
    /// Fortran sentinel. This is used for nested typed directives and never
    /// fabricates a source prefix or reparses a rendered buffer.
    pub(crate) fn parse_body_ast_in_source(
        &self,
        input: &str,
        parser_config: &ParserConfig,
        source: &lexer::LogicalSource<'_>,
    ) -> Result<RoupDirective, AstBuildError> {
        source.span_of(input)?;
        let input = input.trim_start();
        let (rest, directive) = self
            .directive_registry
            .parse(input, &self.clause_registry)
            .map_err(|error| AstBuildError::ParseFailure(format!("{error:?}")))?;
        let rest = self
            .skip_trivia(rest)
            .map(|(remaining, _)| remaining)
            .map_err(|error| AstBuildError::ParseFailure(format!("{error:?}")))?;
        if !rest.trim().is_empty() {
            return Err(AstBuildError::ParseFailure(format!(
                "unexpected trailing tokens: {:?}",
                rest.trim()
            )));
        }

        ast_builder::build_roup_directive(
            &directive,
            self.dialect,
            parser_config,
            parser_config.language(),
            source,
        )
    }

    /// Parse one construct trait directly from its bounded selector source.
    /// Trait properties use clause syntax inside parentheses; no rendered or
    /// reconstructed directive buffer is introduced.
    pub(crate) fn parse_construct_trait_ast_in_source(
        &self,
        input: &str,
        parser_config: &ParserConfig,
        source: &lexer::LogicalSource<'_>,
    ) -> Result<(OmpDirective, Option<crate::ir::Expression>), AstBuildError> {
        let input = input.trim();
        source.span_of(input)?;
        let (after_name, (name, name_source)) = self
            .directive_registry
            .lex_name(input)
            .map_err(|error| AstBuildError::ParseFailure(format!("{error:?}")))?;
        let remainder = after_name.trim();

        let (clauses, score) = if remainder.is_empty() {
            (Vec::new(), None)
        } else {
            let after_open = remainder.strip_prefix('(').ok_or_else(|| {
                AstBuildError::ParseFailure(
                    "construct selector trait properties require parentheses".to_string(),
                )
            })?;
            let close = lexer::find_matching_parenthesis(
                after_open,
                matches!(
                    self.language,
                    Language::FortranFree | Language::FortranFixed
                ),
            )
            .ok_or_else(|| {
                AstBuildError::ParseFailure(
                    "construct selector has unbalanced trait properties".to_string(),
                )
            })?;
            if !after_open[close + 1..].trim().is_empty() {
                return Err(AstBuildError::ParseFailure(
                    "construct selector has trailing text after trait properties".to_string(),
                ));
            }
            let property_source = after_open[..close].trim();
            if property_source.is_empty() {
                return Err(AstBuildError::ParseFailure(
                    "construct selector property list must not be empty".to_string(),
                ));
            }
            let (score, property_source) = if parser_config.source_compatibility() {
                semantic::parse_scored_value(property_source, parser_config)
                    .map_err(|error| AstBuildError::ParseFailure(error.to_string()))?
            } else {
                (None, property_source)
            };
            let (rest, clauses) = self
                .clause_registry
                .parse_sequence(property_source)
                .map_err(|error| AstBuildError::ParseFailure(format!("{error:?}")))?;
            if !rest.trim().is_empty() {
                return Err(AstBuildError::ParseFailure(format!(
                    "unexpected construct selector property: {:?}",
                    rest.trim()
                )));
            }
            (clauses, score)
        };

        let directive = LocatedDirective::new(Directive::new(name, None, clauses), name_source);
        let ast = ast_builder::build_roup_directive(
            &directive,
            self.dialect,
            parser_config,
            parser_config.language(),
            source,
        )?;
        let RoupDirective::OpenMp(directive) = ast else {
            return Err(AstBuildError::ParseFailure(
                "construct selector produced a non-OpenMP directive".to_string(),
            ));
        };
        if !parser_config.source_compatibility()
            && directive.kind() != OmpDirectiveKind::Simd
            && !directive.clauses().is_empty()
        {
            return Err(AstBuildError::ParseFailure(
                "only the simd construct selector trait accepts clause properties".to_string(),
            ));
        }
        Ok((*directive, score))
    }
}

#[cfg(test)]
pub(crate) fn parse_omp_directive(input: &str) -> IResult<&str, LocatedDirective<'_>> {
    openmp::parser().parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::ParserConfig;
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
    fn parse_ast_accepts_hint_clause() {
        let parser = openmp::parser();
        let config = ParserConfig::c();

        let result = parser.parse_ast("#pragma omp critical(test1) hint(test2)", &config);

        assert!(result.is_ok(), "parse_ast error: {:?}", result.err());
    }

    #[test]
    fn parser_uses_custom_registries() {
        fn parse_only_bare<'a>(
            name: Cow<'a, str>,
            input: &'a str,
            _case_insensitive: bool,
        ) -> IResult<&'a str, Clause<'a>> {
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
                },
            ))
        }

        let directive_registry = DirectiveRegistry::builder()
            .register_custom("target", parse_prefixed)
            .build();

        let parser = Parser::new(
            directive_registry,
            clause_registry,
            Language::C,
            Dialect::OpenMp,
        );

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
        let parser = openmp::parser();
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
        let parser = openmp::parser().with_language(Language::FortranFree);
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
        let parser = openmp::parser().with_language(Language::FortranFree);
        let input = "!$omp parallel do private(i, &\n!$omp& j, &\n!$omp& k)";

        let (_, directive) = parser.parse(input).expect("directive should parse");

        assert_eq!(directive.name, "parallel do");
        assert_eq!(directive.clauses.len(), 1);
        assert_eq!(directive.clauses[0].name, "private");
        assert_eq!(
            directive.clauses[0].kind,
            ClauseKind::Parenthesized("i,  j,  k".into())
        );
    }

    #[test]
    fn parses_fortran_fixed_multiline_directive() {
        let parser = openmp::parser().with_language(Language::FortranFixed);
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
