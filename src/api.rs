//! Strict, version-aware public parsing facade.
//!
//! The facade performs no clause normalization or repair pass and applies an
//! explicit directive-spelling availability catalog after typed AST creation.
//! Syntax availability is cumulative: both `VersionPolicy::Any` and exact
//! modes accept standardized historical forms even when a later specification
//! removed them. Exact modes reject only syntax introduced after the selected
//! version.
//!
//! Compatibility is intersected across the directive spelling and every typed
//! clause, modifier, argument form, historical alias, and nested directive.
//! The parser does not yet retain token spans in its errors, so diagnostics
//! cover the complete checked input rather than inventing a narrower location.

use crate::ast::{AccDirective, OmpDirective, RoupDirective};
use crate::availability::{
    OPENACC_NONSTANDARD_SPELLINGS, OPENMP_NONSTANDARD_SPELLINGS, openacc_compatible_versions,
    openacc_directive_availability, openmp_compatible_versions, openmp_directive_availability,
};
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::feature_availability::{
    FeatureAvailability, openacc_clause_syntax_availability,
    openacc_directive_parameter_availability, openmp_clause_syntax_availability,
    openmp_directive_spelling_availability,
};
use crate::ir::ParserConfig;
use crate::lexer::Language as LexerLanguage;
use crate::parser::{self, AstBuildError};
use crate::source::Span;
use crate::validation::{
    SemanticFacts, require_openacc_semantic_facts, require_openmp_semantic_facts, validate_openacc,
    validate_openacc_with_facts, validate_openmp, validate_openmp_with_facts,
};
use crate::version::{
    HostLanguageProfile, OpenAccVersion, OpenMpVersion, SourceForm, VersionPolicy, VersionSet,
};

/// Strict OpenMP parser configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OpenMpConfig {
    version_policy: VersionPolicy<OpenMpVersion>,
    host: HostLanguageProfile,
    source_form: SourceForm,
    source_compatibility: bool,
}

impl OpenMpConfig {
    /// Creates a configuration using union-version parsing.
    pub fn new(host: HostLanguageProfile, source_form: SourceForm) -> Result<Self, Diagnostic> {
        validate_host_source_form(host, source_form)?;
        Ok(Self {
            version_policy: VersionPolicy::Any,
            host,
            source_form,
            source_compatibility: false,
        })
    }

    /// Creates a configuration restricted to one OpenMP version.
    pub fn exact(
        version: OpenMpVersion,
        host: HostLanguageProfile,
        source_form: SourceForm,
    ) -> Result<Self, Diagnostic> {
        Self::new(host, source_form)
            .map(|config| config.with_version_policy(VersionPolicy::Exact(version)))
    }

    /// Replaces the version policy while retaining the validated host profile.
    #[must_use]
    pub const fn with_version_policy(
        mut self,
        version_policy: VersionPolicy<OpenMpVersion>,
    ) -> Self {
        self.version_policy = version_policy;
        self
    }

    #[must_use]
    pub const fn version_policy(self) -> VersionPolicy<OpenMpVersion> {
        self.version_policy
    }

    #[must_use]
    pub const fn host(self) -> HostLanguageProfile {
        self.host
    }

    #[must_use]
    pub const fn source_form(self) -> SourceForm {
        self.source_form
    }

    /// Match the accepted source contract of the historical parser frontend.
    ///
    /// The result remains a fully typed AST. This mode relaxes only
    /// specification-level validation that the historical frontend did not
    /// perform; malformed syntax and unrepresentable data remain hard errors.
    #[must_use]
    pub const fn with_source_compatibility(mut self) -> Self {
        self.source_compatibility = true;
        self
    }

    #[must_use]
    pub const fn parser(self) -> OpenMpParser {
        OpenMpParser { config: self }
    }
}

/// Strict OpenACC parser configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OpenAccConfig {
    version_policy: VersionPolicy<OpenAccVersion>,
    host: HostLanguageProfile,
    source_form: SourceForm,
    source_compatibility: bool,
}

impl OpenAccConfig {
    /// Creates a configuration using union-version parsing.
    pub fn new(host: HostLanguageProfile, source_form: SourceForm) -> Result<Self, Diagnostic> {
        validate_host_source_form(host, source_form)?;
        Ok(Self {
            version_policy: VersionPolicy::Any,
            host,
            source_form,
            source_compatibility: false,
        })
    }

    /// Creates a configuration restricted to one OpenACC version.
    pub fn exact(
        version: OpenAccVersion,
        host: HostLanguageProfile,
        source_form: SourceForm,
    ) -> Result<Self, Diagnostic> {
        Self::new(host, source_form)
            .map(|config| config.with_version_policy(VersionPolicy::Exact(version)))
    }

    /// Replaces the version policy while retaining the validated host profile.
    #[must_use]
    pub const fn with_version_policy(
        mut self,
        version_policy: VersionPolicy<OpenAccVersion>,
    ) -> Self {
        self.version_policy = version_policy;
        self
    }

    #[must_use]
    pub const fn version_policy(self) -> VersionPolicy<OpenAccVersion> {
        self.version_policy
    }

    #[must_use]
    pub const fn host(self) -> HostLanguageProfile {
        self.host
    }

    #[must_use]
    pub const fn source_form(self) -> SourceForm {
        self.source_form
    }

    /// Match the accepted source contract of the historical parser frontend.
    ///
    /// The result remains a fully typed AST. This mode relaxes only
    /// specification-level validation that the historical frontend did not
    /// perform; malformed syntax and unrepresentable data remain hard errors.
    #[must_use]
    pub const fn with_source_compatibility(mut self) -> Self {
        self.source_compatibility = true;
        self
    }

    #[must_use]
    pub const fn parser(self) -> OpenAccParser {
        OpenAccParser { config: self }
    }
}

/// A configured strict OpenMP parser.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OpenMpParser {
    config: OpenMpConfig,
}

impl OpenMpParser {
    #[must_use]
    pub const fn new(config: OpenMpConfig) -> Self {
        Self { config }
    }

    #[must_use]
    pub const fn config(self) -> OpenMpConfig {
        self.config
    }

    /// Parse and apply every context-independent structural and clause rule.
    ///
    /// This method deliberately does not claim to validate facts that require
    /// an embedding compiler, such as declaration placement or host-language
    /// constant-expression classification. Use [`Self::parse_with_facts`] when
    /// those semantic checks are required.
    pub fn parse(&self, source: &str) -> Result<ParsedOpenMpDirective, Diagnostic> {
        self.parse_impl(source, None)
    }

    /// Parse and validate with semantic facts supplied by the embedding compiler.
    /// Every required fact must be present; missing facts are hard errors.
    pub fn parse_with_facts(
        &self,
        source: &str,
        facts: &SemanticFacts,
    ) -> Result<ParsedOpenMpDirective, Diagnostic> {
        self.parse_impl(source, Some(facts))
    }

    fn parse_impl(
        &self,
        source: &str,
        facts: Option<&SemanticFacts>,
    ) -> Result<ParsedOpenMpDirective, Diagnostic> {
        let parser =
            parser::openmp::parser().with_language(lexer_language(self.config.source_form));
        let parser_config = parser_config(self.config.host)
            .with_openmp_version_policy(self.config.version_policy)
            .with_source_compatibility(self.config.source_compatibility);
        let directive = parser
            .parse_ast(source, &parser_config)
            .map_err(|error| ast_error(error, source))?;

        let RoupDirective::OpenMp(openmp) = directive else {
            return Err(Diagnostic::new(
                DiagnosticCode::InvalidDirective,
                Span::entire(source),
                "OpenMP parser produced an OpenACC directive body",
            ));
        };
        let spelling = openmp.kind().as_str();

        let mut compatible_versions = match openmp_directive_availability(spelling) {
            Some(availability) => {
                openmp_compatible_versions(availability, self.config.host.language())
            }
            None if self.config.source_compatibility
                && matches!(
                    openmp.kind(),
                    crate::ast::OmpDirectiveKind::Ompx | crate::ast::OmpDirectiveKind::EndSection
                ) =>
            {
                VersionSet::empty()
            }
            None => {
                return Err(uncatalogued_directive(
                    "OpenMP",
                    spelling,
                    source,
                    OPENMP_NONSTANDARD_SPELLINGS.contains(&spelling),
                ));
            }
        };
        let spelling_availability =
            openmp_directive_spelling_availability(&openmp, self.config.host.language());
        match spelling_availability {
            FeatureAvailability::Standardized { .. } => {
                compatible_versions = compatible_versions.intersection(
                    spelling_availability
                        .compatible_versions()
                        .ok_or_else(|| internal_availability_error("OpenMP", source))?,
                );
            }
            FeatureAvailability::Nonstandard { .. } if self.config.source_compatibility => {
                compatible_versions = VersionSet::empty();
            }
            FeatureAvailability::Nonstandard { reason } => {
                return Err(nonstandard_directive("OpenMP", spelling, reason, source));
            }
        }
        for clause in openmp.clauses() {
            let availability = openmp_clause_syntax_availability(
                openmp.kind(),
                clause,
                self.config.host.language(),
            );
            match availability {
                FeatureAvailability::Standardized { .. } => {
                    compatible_versions = compatible_versions.intersection(
                        availability
                            .compatible_versions()
                            .ok_or_else(|| internal_availability_error("OpenMP", source))?,
                    );
                }
                FeatureAvailability::Nonstandard { .. } if self.config.source_compatibility => {
                    compatible_versions = VersionSet::empty();
                }
                FeatureAvailability::Nonstandard { reason } => {
                    return Err(nonstandard_clause(
                        "OpenMP",
                        clause.kind().as_str(),
                        reason,
                        source,
                    ));
                }
            }
        }
        if !self.config.source_compatibility
            || matches!(self.config.version_policy, VersionPolicy::Exact(_))
        {
            enforce_openmp_policy(
                self.config.version_policy,
                spelling,
                compatible_versions,
                source,
            )?;
        }
        match facts {
            Some(facts) if self.config.source_compatibility => require_openmp_semantic_facts(
                &openmp,
                self.config.version_policy,
                Span::entire(source),
                facts,
            )?,
            Some(facts) => validate_openmp_with_facts(
                &openmp,
                self.config.version_policy,
                Span::entire(source),
                facts,
            )?,
            None if self.config.source_compatibility => {}
            None => validate_openmp(&openmp, self.config.version_policy, Span::entire(source))?,
        }

        Ok(ParsedOpenMpDirective {
            directive: *openmp,
            compatible_versions,
        })
    }
}

/// A configured strict OpenACC parser.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OpenAccParser {
    config: OpenAccConfig,
}

impl OpenAccParser {
    #[must_use]
    pub const fn new(config: OpenAccConfig) -> Self {
        Self { config }
    }

    #[must_use]
    pub const fn config(self) -> OpenAccConfig {
        self.config
    }

    /// Parse and apply every context-independent structural and clause rule.
    pub fn parse(&self, source: &str) -> Result<ParsedOpenAccDirective, Diagnostic> {
        self.parse_impl(source, None)
    }

    /// Parse and validate with semantic facts supplied by the embedding compiler.
    /// Every required fact must be present; missing facts are hard errors.
    pub fn parse_with_facts(
        &self,
        source: &str,
        facts: &SemanticFacts,
    ) -> Result<ParsedOpenAccDirective, Diagnostic> {
        self.parse_impl(source, Some(facts))
    }

    fn parse_impl(
        &self,
        source: &str,
        facts: Option<&SemanticFacts>,
    ) -> Result<ParsedOpenAccDirective, Diagnostic> {
        let parser =
            parser::openacc::parser().with_language(lexer_language(self.config.source_form));
        let parser_config = parser_config(self.config.host)
            .with_source_compatibility(self.config.source_compatibility);
        let directive = parser
            .parse_ast(source, &parser_config)
            .map_err(|error| ast_error(error, source))?;

        let RoupDirective::OpenAcc(openacc) = directive else {
            return Err(Diagnostic::new(
                DiagnosticCode::InvalidDirective,
                Span::entire(source),
                "OpenACC parser produced an OpenMP directive body",
            ));
        };
        let spelling = openacc.kind().as_str();

        let Some(availability) = openacc_directive_availability(spelling) else {
            return Err(uncatalogued_directive(
                "OpenACC",
                spelling,
                source,
                OPENACC_NONSTANDARD_SPELLINGS.contains(&spelling),
            ));
        };
        let mut compatible_versions =
            openacc_compatible_versions(availability, self.config.host.language());
        let parameter_availability = openacc_directive_parameter_availability(&openacc);
        match parameter_availability {
            FeatureAvailability::Standardized { .. } => {
                compatible_versions = compatible_versions.intersection(
                    parameter_availability
                        .compatible_versions()
                        .ok_or_else(|| internal_availability_error("OpenACC", source))?,
                );
            }
            FeatureAvailability::Nonstandard { .. } if self.config.source_compatibility => {
                compatible_versions = VersionSet::empty();
            }
            FeatureAvailability::Nonstandard { reason } => {
                return Err(nonstandard_directive("OpenACC", spelling, reason, source));
            }
        }
        for clause in openacc.clauses() {
            let availability = openacc_clause_syntax_availability(openacc.kind(), clause);
            match availability {
                FeatureAvailability::Standardized { .. } => {
                    compatible_versions = compatible_versions.intersection(
                        availability
                            .compatible_versions()
                            .ok_or_else(|| internal_availability_error("OpenACC", source))?,
                    );
                }
                FeatureAvailability::Nonstandard { .. } if self.config.source_compatibility => {
                    compatible_versions = VersionSet::empty();
                }
                FeatureAvailability::Nonstandard { reason } => {
                    return Err(nonstandard_clause(
                        "OpenACC",
                        clause.kind().as_str(),
                        reason,
                        source,
                    ));
                }
            }
        }
        if !self.config.source_compatibility
            || matches!(self.config.version_policy, VersionPolicy::Exact(_))
        {
            enforce_openacc_policy(
                self.config.version_policy,
                spelling,
                compatible_versions,
                source,
            )?;
        }
        match facts {
            Some(facts) if self.config.source_compatibility => {
                require_openacc_semantic_facts(&openacc, Span::entire(source), facts)?
            }
            Some(facts) => validate_openacc_with_facts(
                &openacc,
                self.config.version_policy,
                Span::entire(source),
                facts,
            )?,
            None if self.config.source_compatibility => {}
            None => validate_openacc(&openacc, self.config.version_policy, Span::entire(source))?,
        }

        Ok(ParsedOpenAccDirective {
            directive: *openacc,
            compatible_versions,
        })
    }
}

/// OpenMP parse result with the standardized compatibility set for all syntax
/// in the directive. The set is empty when source-compatibility mode accepts a
/// nonstandard extension.
#[derive(Debug)]
pub struct ParsedOpenMpDirective {
    directive: OmpDirective,
    compatible_versions: VersionSet<OpenMpVersion>,
}

impl ParsedOpenMpDirective {
    #[must_use]
    pub const fn directive(&self) -> &OmpDirective {
        &self.directive
    }

    #[must_use]
    pub const fn compatible_versions(&self) -> VersionSet<OpenMpVersion> {
        self.compatible_versions
    }

    #[must_use]
    pub fn into_directive(self) -> OmpDirective {
        self.directive
    }
}

/// OpenACC parse result with the standardized compatibility set for all syntax
/// in the directive. The set is empty when source-compatibility mode accepts a
/// nonstandard extension.
#[derive(Debug)]
pub struct ParsedOpenAccDirective {
    directive: AccDirective,
    compatible_versions: VersionSet<OpenAccVersion>,
}

impl ParsedOpenAccDirective {
    #[must_use]
    pub const fn directive(&self) -> &AccDirective {
        &self.directive
    }

    #[must_use]
    pub const fn compatible_versions(&self) -> VersionSet<OpenAccVersion> {
        self.compatible_versions
    }

    #[must_use]
    pub fn into_directive(self) -> AccDirective {
        self.directive
    }
}

fn validate_host_source_form(
    host: HostLanguageProfile,
    source_form: SourceForm,
) -> Result<(), Diagnostic> {
    host.validate_source_form(source_form).map_err(|error| {
        Diagnostic::new(
            DiagnosticCode::IncompatibleSourceForm,
            Span::start_of(""),
            error.to_string(),
        )
    })
}

const fn lexer_language(source_form: SourceForm) -> LexerLanguage {
    match source_form {
        SourceForm::Pragma => LexerLanguage::C,
        SourceForm::FortranFree => LexerLanguage::FortranFree,
        SourceForm::FortranFixed => LexerLanguage::FortranFixed,
    }
}

const fn parser_config(host: HostLanguageProfile) -> ParserConfig {
    ParserConfig::new(host)
}

fn ast_error(error: AstBuildError, source: &str) -> Diagnostic {
    let code = match &error {
        AstBuildError::UnsupportedDirective(_) => DiagnosticCode::InvalidDirective,
        AstBuildError::UnsupportedClause(_) | AstBuildError::ClauseConversion(_) => {
            DiagnosticCode::InvalidClause
        }
        AstBuildError::ParseFailure(_) => DiagnosticCode::UnexpectedToken,
    };
    Diagnostic::new(code, Span::entire(source), error.to_string())
}

fn uncatalogued_directive(
    dialect: &str,
    spelling: &str,
    source: &str,
    explicitly_nonstandard: bool,
) -> Diagnostic {
    let message = if explicitly_nonstandard {
        format!("{dialect} directive spelling {spelling:?} is not standardized")
    } else {
        format!(
            "{dialect} directive spelling {spelling:?} has no strict availability catalog entry"
        )
    };
    Diagnostic::new(
        DiagnosticCode::InvalidDirective,
        Span::entire(source),
        message,
    )
}

fn nonstandard_clause(dialect: &str, spelling: &str, reason: &str, source: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::InvalidClause,
        Span::entire(source),
        format!("{dialect} clause {spelling:?} is not standardized: {reason}"),
    )
}

fn nonstandard_directive(dialect: &str, spelling: &str, reason: &str, source: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::InvalidDirective,
        Span::entire(source),
        format!("{dialect} directive {spelling:?} is not standardized: {reason}"),
    )
}

fn internal_availability_error(dialect: &str, source: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::InvalidConfiguration,
        Span::entire(source),
        format!("{dialect} standardized feature has no compatible version set"),
    )
}

fn enforce_openmp_policy(
    policy: VersionPolicy<OpenMpVersion>,
    spelling: &str,
    compatible: VersionSet<OpenMpVersion>,
    source: &str,
) -> Result<(), Diagnostic> {
    match policy {
        VersionPolicy::Any if compatible.is_empty() => {
            Err(language_unavailable("OpenMP", spelling, source))
        }
        VersionPolicy::Any => Ok(()),
        VersionPolicy::Exact(version) if compatible.contains(version) => Ok(()),
        VersionPolicy::Exact(version) => Err(version_unavailable(
            "OpenMP", spelling, version, compatible, source,
        )),
    }
}

fn enforce_openacc_policy(
    policy: VersionPolicy<OpenAccVersion>,
    spelling: &str,
    compatible: VersionSet<OpenAccVersion>,
    source: &str,
) -> Result<(), Diagnostic> {
    match policy {
        VersionPolicy::Any if compatible.is_empty() => {
            Err(language_unavailable("OpenACC", spelling, source))
        }
        VersionPolicy::Any => Ok(()),
        VersionPolicy::Exact(version) if compatible.contains(version) => Ok(()),
        VersionPolicy::Exact(version) => Err(version_unavailable(
            "OpenACC", spelling, version, compatible, source,
        )),
    }
}

fn language_unavailable(dialect: &str, spelling: &str, source: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::NotAvailableInVersion,
        Span::entire(source),
        format!(
            "{dialect} directive {spelling:?} is not available for the configured host language"
        ),
    )
}

fn version_unavailable<V: crate::version::DirectiveVersion>(
    dialect: &str,
    spelling: &str,
    version: V,
    compatible: VersionSet<V>,
    source: &str,
) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::NotAvailableInVersion,
        Span::entire(source),
        format!(
            "{dialect} directive {spelling:?} is not standardized in version {version}; compatible versions are {compatible}"
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version::{CStandard, FortranStandard};

    fn c23() -> HostLanguageProfile {
        HostLanguageProfile::C(CStandard::C23)
    }

    fn fortran2023() -> HostLanguageProfile {
        HostLanguageProfile::Fortran(FortranStandard::Fortran2023)
    }

    #[test]
    fn new_configuration_defaults_to_union_version_policy() {
        let omp = OpenMpConfig::new(c23(), SourceForm::Pragma).expect("valid C profile");
        let acc = OpenAccConfig::new(c23(), SourceForm::Pragma).expect("valid C profile");

        assert_eq!(omp.version_policy(), VersionPolicy::Any);
        assert_eq!(acc.version_policy(), VersionPolicy::Any);
    }

    #[test]
    fn configuration_rejects_incompatible_source_form() {
        let error = OpenMpConfig::new(c23(), SourceForm::FortranFree)
            .expect_err("C cannot use a Fortran sentinel");
        assert_eq!(error.code(), DiagnosticCode::IncompatibleSourceForm);
        assert!(error.primary_span().is_empty());

        let error = OpenAccConfig::new(fortran2023(), SourceForm::Pragma)
            .expect_err("Fortran cannot use pragma source form");
        assert_eq!(error.code(), DiagnosticCode::IncompatibleSourceForm);
    }

    #[test]
    fn any_policy_accepts_removed_historical_openmp_syntax() {
        let parser = OpenMpConfig::new(c23(), SourceForm::Pragma)
            .expect("valid configuration")
            .parser();
        let parsed = parser
            .parse("#pragma omp master")
            .expect("union policy must retain historical master syntax");

        assert!(parsed.compatible_versions().contains(OpenMpVersion::V1_0));
        assert!(parsed.compatible_versions().contains(OpenMpVersion::V5_2));
        assert!(parsed.compatible_versions().contains(OpenMpVersion::V6_0));
    }

    #[test]
    fn exact_policy_accepts_removed_historical_openmp_syntax() {
        let parser = OpenMpConfig::exact(OpenMpVersion::V6_0, c23(), SourceForm::Pragma)
            .expect("valid configuration")
            .parser();
        let parsed = parser
            .parse("#pragma omp master")
            .expect("OpenMP 6.0 mode must accept maintained historical syntax");
        assert!(parsed.compatible_versions().contains(OpenMpVersion::V6_0));
    }

    #[test]
    fn exact_policy_checks_directive_introduction() {
        let old = OpenMpConfig::exact(OpenMpVersion::V3_1, c23(), SourceForm::Pragma)
            .expect("valid configuration")
            .parser();
        let error = old
            .parse("#pragma omp target")
            .expect_err("target was not present in OpenMP 3.1");
        assert_eq!(error.code(), DiagnosticCode::NotAvailableInVersion);

        let current = OpenMpConfig::exact(OpenMpVersion::V4_0, c23(), SourceForm::Pragma)
            .expect("valid configuration")
            .parser();
        assert!(current.parse("#pragma omp target").is_ok());
    }

    #[test]
    fn language_specific_directive_is_rejected_in_wrong_host_language() {
        let c_parser = OpenMpConfig::new(c23(), SourceForm::Pragma)
            .expect("valid configuration")
            .parser();
        let error = c_parser
            .parse("#pragma omp do")
            .expect_err("OpenMP do is Fortran-only");
        assert_eq!(error.code(), DiagnosticCode::NotAvailableInVersion);

        let fortran_parser = OpenMpConfig::new(fortran2023(), SourceForm::FortranFree)
            .expect("valid configuration")
            .parser();
        assert!(fortran_parser.parse("!$omp do").is_ok());
    }

    #[test]
    fn openmp_six_underscore_spelling_keeps_its_introduction_version() {
        let old = OpenMpConfig::exact(OpenMpVersion::V5_2, c23(), SourceForm::Pragma)
            .expect("valid configuration")
            .parser();
        let error = old
            .parse("#pragma omp declare_target(x)")
            .expect_err("underscore directive spelling was introduced in OpenMP 6.0");
        assert_eq!(error.code(), DiagnosticCode::NotAvailableInVersion);

        let current = OpenMpConfig::exact(OpenMpVersion::V6_0, c23(), SourceForm::Pragma)
            .expect("valid configuration")
            .parser();
        let parsed = current
            .parse("#pragma omp declare_target(x)")
            .expect("OpenMP 6.0 underscore spelling must be accepted");
        assert_eq!(
            parsed.directive().kind(),
            crate::ast::OmpDirectiveKind::DeclareTarget
        );

        let historical = OpenMpConfig::exact(OpenMpVersion::V4_0, c23(), SourceForm::Pragma)
            .expect("valid configuration")
            .parser();
        assert!(historical.parse("#pragma omp declare target(x)").is_ok());
    }

    #[test]
    fn target_data_is_standardized_only_from_openmp_six() {
        let old = OpenMpConfig::exact(OpenMpVersion::V5_2, c23(), SourceForm::Pragma)
            .expect("valid configuration")
            .parser();
        let error = old
            .parse("#pragma omp target_data map(to: x)")
            .expect_err("target_data spelling was introduced in OpenMP 6.0");
        assert_eq!(error.code(), DiagnosticCode::NotAvailableInVersion);

        let current = OpenMpConfig::exact(OpenMpVersion::V6_0, c23(), SourceForm::Pragma)
            .expect("valid configuration")
            .parser();
        assert!(current.parse("#pragma omp target_data map(to: x)").is_ok());
    }

    #[test]
    fn compact_openmp_names_are_fortran_only() {
        for source_form in [SourceForm::FortranFree, SourceForm::FortranFixed] {
            let parser = OpenMpConfig::exact(OpenMpVersion::V4_0, fortran2023(), source_form)
                .expect("valid Fortran configuration")
                .parser();
            assert!(parser.parse("!$omp cancellationpoint parallel").is_ok());
        }

        let c_parser = OpenMpConfig::new(c23(), SourceForm::Pragma)
            .expect("valid C configuration")
            .parser();
        assert!(
            c_parser
                .parse("#pragma omp cancellationpoint parallel")
                .is_err()
        );
    }

    #[test]
    fn parser_known_nonstandard_directives_are_hard_errors() {
        let parser = OpenMpConfig::new(c23(), SourceForm::Pragma)
            .expect("valid configuration")
            .parser();
        for source in [
            "#pragma omp target data composite",
            "#pragma omp end target enter data",
            "#pragma omp end section",
        ] {
            let error = parser
                .parse(source)
                .expect_err("parser-only directive must not reach the public typed AST");
            assert_eq!(error.code(), DiagnosticCode::InvalidDirective, "{source}");
        }
    }

    #[test]
    fn openacc_exact_policy_checks_introduction() {
        let old = OpenAccConfig::exact(OpenAccVersion::V2_5, c23(), SourceForm::Pragma)
            .expect("valid configuration")
            .parser();
        let error = old
            .parse("#pragma acc serial")
            .expect_err("serial was introduced after OpenACC 2.5");
        assert_eq!(error.code(), DiagnosticCode::NotAvailableInVersion);

        let newer = OpenAccConfig::exact(OpenAccVersion::V2_6, c23(), SourceForm::Pragma)
            .expect("valid configuration")
            .parser();
        assert!(newer.parse("#pragma acc serial").is_ok());
    }

    #[test]
    fn malformed_input_returns_one_checked_diagnostic() {
        let parser = OpenMpConfig::new(c23(), SourceForm::Pragma)
            .expect("valid configuration")
            .parser();
        let source = "#pragma omp definitely_not_a_directive";
        let error = parser
            .parse(source)
            .expect_err("unknown directive must be rejected");

        assert_eq!(error.code(), DiagnosticCode::UnexpectedToken);
        assert_eq!(error.primary_span().slice(source), Ok(source));
        assert!(error.related_spans().is_empty());
    }
}
