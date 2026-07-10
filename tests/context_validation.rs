use roup::api::{OpenAccConfig, OpenAccParser, OpenMpConfig, OpenMpParser};
use roup::ast::{AccDirectiveKind, OmpClauseKind, OmpDirectiveKind};
use roup::diagnostic::DiagnosticCode;
use roup::source::Span;
use roup::validation::{
    AssociationKind, ContextValidator, IntegerEvaluation, OmpClauseSite, OmpExpressionSite,
    SemanticFacts,
};
use roup::version::{CStandard, HostLanguageProfile, OpenAccVersion, OpenMpVersion, SourceForm};

fn c23() -> HostLanguageProfile {
    HostLanguageProfile::C(CStandard::C23)
}

fn omp() -> OpenMpParser {
    OpenMpConfig::new(c23(), SourceForm::Pragma)
        .expect("valid OpenMP configuration")
        .parser()
}

fn acc() -> OpenAccParser {
    OpenAccConfig::new(c23(), SourceForm::Pragma)
        .expect("valid OpenACC configuration")
        .parser()
}

#[test]
fn openmp_rejects_a_clause_on_the_wrong_directive() {
    let error = omp()
        .parse("#pragma omp parallel schedule(static)")
        .expect_err("schedule is not valid on a plain parallel directive");

    assert_eq!(error.code(), DiagnosticCode::ClauseNotAllowed);
}

#[test]
fn openmp_reports_the_first_duplicate_unique_clause() {
    let error = omp()
        .parse("#pragma omp parallel num_threads(2) num_threads(4)")
        .expect_err("num_threads may not be repeated");

    assert_eq!(error.code(), DiagnosticCode::DuplicateClause);
}

#[test]
fn openmp_requires_mandatory_action_clauses() {
    let error = omp()
        .parse("#pragma omp target enter data")
        .expect_err("target enter data requires map");

    assert_eq!(error.code(), DiagnosticCode::MissingRequiredClause);
}

#[test]
fn openmp_detects_clause_conflicts_after_local_legality() {
    let error = omp()
        .parse("#pragma omp for ordered schedule(auto)")
        .expect_err("ordered conflicts with schedule(auto)");

    assert_eq!(error.code(), DiagnosticCode::ConflictingClauses);
}

#[test]
fn openmp_constant_expression_status_must_be_supplied() {
    let source = "#pragma omp simd safelen(vector_width)";
    let site = OmpExpressionSite::new(OmpClauseSite::new(OmpClauseKind::Safelen, 0), 0);
    omp()
        .parse(source)
        .expect("standalone parsing performs context-independent validation only");

    let error = omp()
        .parse_with_facts(source, &SemanticFacts::new())
        .expect_err("explicit semantic validation requires the constant-expression fact");
    assert_eq!(error.code(), DiagnosticCode::MissingSemanticFact);

    let invalid = SemanticFacts::new().with_constant_expression(site, false);
    let error = omp()
        .parse_with_facts(source, &invalid)
        .expect_err("the embedding compiler rejected the expression as non-constant");
    assert_eq!(error.code(), DiagnosticCode::ConstantExpressionRequired);

    let valid =
        SemanticFacts::new().with_integer_evaluation(site, IntegerEvaluation::NonNegative(4));
    omp()
        .parse_with_facts(source, &valid)
        .expect("a confirmed constant expression is accepted");
}

#[test]
fn declaration_position_is_never_guessed() {
    let source = "#pragma omp declare target";
    omp()
        .parse(source)
        .expect("standalone parsing does not claim to validate declaration placement");

    let error = omp()
        .parse_with_facts(source, &SemanticFacts::new())
        .expect_err("explicit semantic validation requires declaration placement");
    assert_eq!(error.code(), DiagnosticCode::MissingSemanticFact);

    let invalid = SemanticFacts::new().with_declaration_position(false);
    let error = omp()
        .parse_with_facts(source, &invalid)
        .expect_err("an invalid declaration position must be rejected");
    assert_eq!(error.code(), DiagnosticCode::InvalidDeclarationPosition);

    let valid = SemanticFacts::new().with_declaration_position(true);
    omp()
        .parse_with_facts(source, &valid)
        .expect("a confirmed declaration position is accepted");
}

#[test]
fn association_is_never_inferred_from_a_standalone_directive() {
    let source = "#pragma omp section";
    omp()
        .parse(source)
        .expect("standalone parsing does not claim an enclosing association");

    let error = omp()
        .parse_with_facts(source, &SemanticFacts::new())
        .expect_err("explicit semantic validation requires an association fact");
    assert_eq!(error.code(), DiagnosticCode::MissingContext);

    let invalid = SemanticFacts::new().with_association(AssociationKind::SectionRegion, false);
    let error = omp()
        .parse_with_facts(source, &invalid)
        .expect_err("a section outside sections is invalid");
    assert_eq!(error.code(), DiagnosticCode::InvalidAssociation);

    let valid = SemanticFacts::new().with_association(AssociationKind::SectionRegion, true);
    omp()
        .parse_with_facts(source, &valid)
        .expect("a confirmed sections association is accepted");
}

#[test]
fn exact_openmp_six_accepts_standardized_historical_syntax() {
    let parser = OpenMpConfig::exact(OpenMpVersion::V6_0, c23(), SourceForm::Pragma)
        .expect("valid OpenMP configuration")
        .parser();

    for source in [
        "#pragma omp master",
        "#pragma omp parallel master",
        "#pragma omp parallel proc_bind(master)",
    ] {
        parser
            .parse(source)
            .unwrap_or_else(|error| panic!("OpenMP 6.0 rejected historical {source:?}: {error}"));
    }
}

#[test]
fn openmp_exact_policy_intersects_directive_and_clause_introductions() {
    let before_proc_bind = OpenMpConfig::exact(OpenMpVersion::V3_1, c23(), SourceForm::Pragma)
        .unwrap()
        .parser();
    let error = before_proc_bind
        .parse("#pragma omp parallel proc_bind(close)")
        .expect_err("proc_bind was introduced in OpenMP 4.0");
    assert_eq!(error.code(), DiagnosticCode::NotAvailableInVersion);

    let with_proc_bind = OpenMpConfig::exact(OpenMpVersion::V4_0, c23(), SourceForm::Pragma)
        .unwrap()
        .parser();
    with_proc_bind
        .parse("#pragma omp parallel proc_bind(close)")
        .expect("OpenMP 4.0 accepts proc_bind");

    let before_defaultmap = OpenMpConfig::exact(OpenMpVersion::V4_0, c23(), SourceForm::Pragma)
        .unwrap()
        .parser();
    let error = before_defaultmap
        .parse("#pragma omp target defaultmap(tofrom:scalar)")
        .expect_err("defaultmap was introduced in OpenMP 4.5");
    assert_eq!(error.code(), DiagnosticCode::NotAvailableInVersion);

    let with_defaultmap = OpenMpConfig::exact(OpenMpVersion::V4_5, c23(), SourceForm::Pragma)
        .unwrap()
        .parser();
    with_defaultmap
        .parse("#pragma omp target defaultmap(tofrom:scalar)")
        .expect("OpenMP 4.5 accepts the original defaultmap form");
}

#[test]
fn combined_compatibility_set_starts_at_the_latest_used_feature() {
    let parsed = omp()
        .parse("#pragma omp target defaultmap(tofrom:scalar)")
        .expect("union policy accepts the directive specification");

    assert!(!parsed.compatible_versions().contains(OpenMpVersion::V4_0));
    assert!(parsed.compatible_versions().contains(OpenMpVersion::V4_5));
    assert!(parsed.compatible_versions().contains(OpenMpVersion::V6_0));
}

#[test]
fn historical_openmp_clause_aliases_keep_their_own_introduction_versions() {
    let omp_four = OpenMpConfig::exact(OpenMpVersion::V4_0, c23(), SourceForm::Pragma)
        .unwrap()
        .parser();
    let error = omp_four
        .parse("#pragma omp ordered depend(source)")
        .expect_err("depend(source) was introduced in OpenMP 4.5");
    assert_eq!(error.code(), DiagnosticCode::NotAvailableInVersion);
    omp_four
        .parse("#pragma omp declare target to(x)")
        .expect("the historical declare-target to clause dates to OpenMP 4.0");

    let omp_four_five = OpenMpConfig::exact(OpenMpVersion::V4_5, c23(), SourceForm::Pragma)
        .unwrap()
        .parser();
    let historical = omp_four_five
        .parse("#pragma omp ordered depend(source)")
        .expect("OpenMP 4.5 accepts depend(source)");
    assert_eq!(
        historical.directive().clauses()[0].kind(),
        OmpClauseKind::Doacross
    );

    let omp_five = OpenMpConfig::exact(OpenMpVersion::V5_0, c23(), SourceForm::Pragma)
        .unwrap()
        .parser();
    let historical = omp_five
        .parse("#pragma omp metadirective default(parallel)")
        .expect("OpenMP 5.0 accepts the historical default spelling");
    assert_eq!(
        historical.directive().clauses()[0].kind(),
        OmpClauseKind::Otherwise
    );
}

#[test]
fn canonical_openmp_alias_replacements_use_their_later_introduction_versions() {
    let omp_five_one = OpenMpConfig::exact(OpenMpVersion::V5_1, c23(), SourceForm::Pragma)
        .unwrap()
        .parser();
    for source in [
        "#pragma omp ordered doacross(source)",
        "#pragma omp metadirective otherwise(parallel)",
        "#pragma omp declare target enter(x)",
    ] {
        let error = omp_five_one
            .parse(source)
            .expect_err("canonical replacement spelling was introduced in OpenMP 5.2");
        assert_eq!(error.code(), DiagnosticCode::NotAvailableInVersion);
    }

    let omp_five_two = OpenMpConfig::exact(OpenMpVersion::V5_2, c23(), SourceForm::Pragma)
        .unwrap()
        .parser();
    for source in [
        "#pragma omp ordered doacross(source)",
        "#pragma omp metadirective otherwise(parallel)",
        "#pragma omp declare target enter(x)",
    ] {
        omp_five_two
            .parse(source)
            .expect("OpenMP 5.2 accepts the canonical replacement spelling");
    }
}

#[test]
fn openacc_one_point_zero_aliases_are_typed_and_canonicalized() {
    let parser = OpenAccConfig::exact(OpenAccVersion::V1_0, c23(), SourceForm::Pragma)
        .unwrap()
        .parser();

    for alias in [
        "pcopy",
        "present_or_copy",
        "pcopyin",
        "present_or_copyin",
        "pcopyout",
        "present_or_copyout",
        "pcreate",
        "present_or_create",
    ] {
        let parsed = parser
            .parse(&format!("#pragma acc data {alias}(a)"))
            .expect("all historical OpenACC data aliases date to 1.0");
        assert!(matches!(
            parsed.directive().clauses()[0].kind(),
            roup::ast::AccClauseKind::Copy
                | roup::ast::AccClauseKind::CopyIn
                | roup::ast::AccClauseKind::CopyOut
                | roup::ast::AccClauseKind::Create
        ));
    }
}

#[test]
fn openacc_exact_policy_intersects_clause_introductions() {
    let acc_one = OpenAccConfig::exact(OpenAccVersion::V1_0, c23(), SourceForm::Pragma)
        .unwrap()
        .parser();
    let error = acc_one
        .parse("#pragma acc parallel loop auto")
        .expect_err("auto was introduced in OpenACC 2.0");
    assert_eq!(error.code(), DiagnosticCode::NotAvailableInVersion);

    let acc_two = OpenAccConfig::exact(OpenAccVersion::V2_0, c23(), SourceForm::Pragma)
        .unwrap()
        .parser();
    acc_two
        .parse("#pragma acc parallel loop auto")
        .expect("OpenACC 2.0 accepts auto");
    let error = acc_two
        .parse("#pragma acc exit data copyout(a) finalize")
        .expect_err("finalize was introduced in OpenACC 2.5");
    assert_eq!(error.code(), DiagnosticCode::NotAvailableInVersion);

    let acc_two_five = OpenAccConfig::exact(OpenAccVersion::V2_5, c23(), SourceForm::Pragma)
        .unwrap()
        .parser();
    acc_two_five
        .parse("#pragma acc exit data copyout(a) finalize")
        .expect("OpenACC 2.5 accepts finalize");
    let error = acc_two_five
        .parse("#pragma acc data no_create(a)")
        .expect_err("no_create was introduced in OpenACC 2.7");
    assert_eq!(error.code(), DiagnosticCode::NotAvailableInVersion);

    let acc_two_six = OpenAccConfig::exact(OpenAccVersion::V2_6, c23(), SourceForm::Pragma)
        .unwrap()
        .parser();
    let error = acc_two_six
        .parse("#pragma acc data no_create(a)")
        .expect_err("OpenACC 2.6 predates no_create");
    assert_eq!(error.code(), DiagnosticCode::NotAvailableInVersion);

    let acc_two_seven = OpenAccConfig::exact(OpenAccVersion::V2_7, c23(), SourceForm::Pragma)
        .unwrap()
        .parser();
    acc_two_seven
        .parse("#pragma acc data no_create(a)")
        .expect("OpenACC 2.7 accepts no_create");
}

#[test]
fn strict_facade_rejects_explicitly_nonstandard_clause_vocabulary() {
    let error = acc()
        .parse("#pragma acc routine indirect")
        .expect_err("indirect is not standardized by OpenACC 3.4");
    assert_eq!(error.code(), DiagnosticCode::UnexpectedToken);

    let error = omp()
        .parse("#pragma omp parallel device_resident(x)")
        .expect_err("device_resident is an OpenACC-only clause");
    assert_eq!(error.code(), DiagnosticCode::UnexpectedToken);
}

#[test]
fn migrated_clause_legality_cases_use_the_typed_validator() {
    omp()
        .parse("#pragma omp target device(ancestor: 1)")
        .expect("device is valid on target");

    let error = omp()
        .parse("#pragma omp parallel device(0)")
        .expect_err("device is not valid on parallel");
    assert_eq!(error.code(), DiagnosticCode::ClauseNotAllowed);

    omp()
        .parse("#pragma omp loop bind(parallel)")
        .expect("bind is valid on loop");
    let error = omp()
        .parse("#pragma omp parallel bind(parallel)")
        .expect_err("bind is not valid on parallel");
    assert_eq!(error.code(), DiagnosticCode::ClauseNotAllowed);
}

#[test]
fn openacc_rejects_wrong_and_always_required_clauses() {
    let error = acc()
        .parse("#pragma acc parallel finalize")
        .expect_err("finalize is only valid on exit data");
    assert_eq!(error.code(), DiagnosticCode::ClauseNotAllowed);

    acc()
        .parse("#pragma acc parallel async async")
        .expect("OpenACC clauses are repeatable unless a restriction says otherwise");

    let error = acc()
        .parse("#pragma acc parallel if(a) if(b)")
        .expect_err("compute constructs permit at most one if clause");
    assert_eq!(error.code(), DiagnosticCode::DuplicateClause);

    let error = acc()
        .parse("#pragma acc update")
        .expect_err("update has required an action clause since OpenACC 1.0");
    assert_eq!(error.code(), DiagnosticCode::MissingRequiredClause);
}

#[test]
fn later_openacc_restrictions_do_not_hide_older_standardized_source() {
    for source in [
        "#pragma acc data",
        "#pragma acc enter data if(enabled)",
        "#pragma acc exit data if(enabled)",
        "#pragma acc host_data",
        "#pragma acc routine",
        "#pragma acc loop seq gang",
        "#pragma acc loop auto independent",
    ] {
        acc()
            .parse(source)
            .expect("union acceptance retains syntax valid before later restrictions");
    }
}

#[test]
fn openacc_declaration_position_is_caller_owned() {
    let source = "#pragma acc routine seq";
    acc()
        .parse(source)
        .expect("standalone parsing does not claim to validate declaration placement");

    let error = acc()
        .parse_with_facts(source, &SemanticFacts::new())
        .expect_err("explicit semantic validation requires declaration placement");
    assert_eq!(error.code(), DiagnosticCode::MissingSemanticFact);

    let valid = SemanticFacts::new().with_declaration_position(true);
    acc()
        .parse_with_facts(source, &valid)
        .expect("routine is accepted at a confirmed declaration position");
}

#[test]
fn context_validator_enforces_lifo_openmp_pairing_with_related_span() {
    let source = "parallel\ncritical\nend parallel";
    let parallel = Span::new(source, 0, 8).unwrap();
    let critical = Span::new(source, 9, 17).unwrap();
    let end_parallel = Span::new(source, 18, source.len()).unwrap();
    let mut context = ContextValidator::new();

    context
        .begin_openmp(OmpDirectiveKind::Parallel, parallel)
        .unwrap();
    context
        .begin_openmp(OmpDirectiveKind::Critical, critical)
        .unwrap();
    let error = context
        .end_openmp(OmpDirectiveKind::EndParallel, end_parallel)
        .expect_err("end parallel cannot skip the nested critical region");

    assert_eq!(error.code(), DiagnosticCode::MismatchedEndDirective);
    assert_eq!(error.related_spans().len(), 1);
    assert_eq!(error.related_spans()[0].span(), critical);
    assert_eq!(
        context.depth(),
        2,
        "failed closes must not mutate the stack"
    );
}

#[test]
fn context_validator_reports_unopened_and_unclosed_regions() {
    let source = "parallel";
    let span = Span::entire(source);
    let end = Span::point(source, source.len()).unwrap();
    let mut context = ContextValidator::new();

    let error = context
        .end_openmp(OmpDirectiveKind::EndParallel, span)
        .expect_err("an end directive needs an opener");
    assert_eq!(error.code(), DiagnosticCode::MissingContext);

    context
        .begin_openmp(OmpDirectiveKind::Parallel, span)
        .unwrap();
    let error = context
        .finish(end)
        .expect_err("an open region at EOF is a hard error");
    assert_eq!(error.code(), DiagnosticCode::MissingContext);
    assert_eq!(error.related_spans()[0].span(), span);
}

#[test]
fn context_validator_accepts_correctly_nested_regions() {
    let source = "parallel critical end-critical end-parallel";
    let span = Span::entire(source);
    let end = Span::point(source, source.len()).unwrap();
    let mut context = ContextValidator::new();

    context
        .begin_openmp(OmpDirectiveKind::Parallel, span)
        .unwrap();
    context
        .begin_openmp(OmpDirectiveKind::Critical, span)
        .unwrap();
    context
        .end_openmp(OmpDirectiveKind::EndCritical, span)
        .unwrap();
    context
        .end_openmp(OmpDirectiveKind::EndParallel, span)
        .unwrap();
    context.finish(end).unwrap();
    assert_eq!(context.depth(), 0);
}

#[test]
fn context_validator_enforces_openacc_pairing() {
    let source = "data\nend parallel";
    let data = Span::new(source, 0, 4).unwrap();
    let end_parallel = Span::new(source, 5, source.len()).unwrap();
    let mut context = ContextValidator::new();

    context.begin_openacc(AccDirectiveKind::Data, data).unwrap();
    let error = context
        .end_openacc(AccDirectiveKind::Parallel, end_parallel)
        .expect_err("end parallel cannot close a data region");

    assert_eq!(error.code(), DiagnosticCode::MismatchedEndDirective);
    assert_eq!(error.related_spans()[0].span(), data);
}
