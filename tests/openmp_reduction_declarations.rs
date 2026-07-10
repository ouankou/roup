use roup::api::{OpenMpConfig, ParsedOpenMpDirective};
use roup::ast::{
    OmpDirectiveParameter, OmpFortranReductionIntrinsic, OmpIdExpression, OmpInductionIdentifier,
    OmpInductorExpression, OmpInitializerValue, OmpReductionCombiner, OmpReductionIdentifier,
    OmpReductionInitializer,
};
use roup::diagnostic::{Diagnostic, DiagnosticCode};
use roup::ir::ClauseData;
use roup::version::{
    CStandard, CppStandard, FortranStandard, HostLanguageProfile, OpenMpVersion, SourceForm,
};

fn c(version: OpenMpVersion, source: &str) -> Result<ParsedOpenMpDirective, Diagnostic> {
    OpenMpConfig::exact(
        version,
        HostLanguageProfile::C(CStandard::C23),
        SourceForm::Pragma,
    )
    .expect("valid C configuration")
    .parser()
    .parse(source)
}

fn cpp(source: &str) -> Result<ParsedOpenMpDirective, Diagnostic> {
    OpenMpConfig::exact(
        OpenMpVersion::V6_0,
        HostLanguageProfile::Cpp(CppStandard::Cpp23),
        SourceForm::Pragma,
    )
    .expect("valid C++ configuration")
    .parser()
    .parse(source)
}

fn fortran(version: OpenMpVersion, source: &str) -> Result<ParsedOpenMpDirective, Diagnostic> {
    OpenMpConfig::exact(
        version,
        HostLanguageProfile::Fortran(FortranStandard::Fortran2023),
        SourceForm::FortranFree,
    )
    .expect("valid Fortran configuration")
    .parser()
    .parse(source)
}

fn reduction(parsed: &ParsedOpenMpDirective) -> &roup::ast::OmpDeclareReduction {
    let Some(OmpDirectiveParameter::DeclareReduction(reduction)) = parsed.directive().parameter()
    else {
        panic!("expected a typed declare-reduction parameter");
    };
    reduction
}

#[test]
fn historical_and_openmp_60_declare_reduction_forms_share_typed_semantics() {
    let historical =
        "#pragma omp declare reduction(sum : int : omp_out += omp_in) initializer(omp_priv = 0)";
    for version in [OpenMpVersion::V5_2, OpenMpVersion::V6_0] {
        let parsed = c(version, historical)
            .unwrap_or_else(|error| panic!("OpenMP {version} rejected {historical:?}: {error}"));
        let reduction = reduction(&parsed);
        assert_eq!(reduction.identifier().to_string(), "sum");
        assert!(matches!(
            reduction.combiner(),
            OmpReductionCombiner::COrCppExpression(expression)
                if expression.to_string() == "omp_out += omp_in"
        ));
        assert!(matches!(
            reduction.initializer(),
            Some(OmpReductionInitializer::CAssignment(OmpInitializerValue::Expression(
                expression
            ))) if expression.to_string() == "0"
        ));
        assert!(parsed.directive().clauses().is_empty());
    }

    let current = "#pragma omp declare reduction(sum : int) combiner(omp_out += omp_in) initializer(omp_priv = 0)";
    let error = c(OpenMpVersion::V5_2, current)
        .expect_err("the clause-based declare-reduction grammar was introduced in OpenMP 6.0");
    assert_eq!(error.code(), DiagnosticCode::NotAvailableInVersion);

    let parsed = c(OpenMpVersion::V6_0, current).expect("OpenMP 6.0 current form must parse");
    assert_eq!(
        reduction(&parsed).combiner().to_string(),
        "omp_out += omp_in"
    );
    assert!(parsed.directive().clauses().is_empty());
}

#[test]
fn cpp_reduction_ids_and_initializer_forms_are_structural() {
    let parsed = cpp(
        "#pragma omp declare_reduction(ns::merge<int> : std::vector<int>) combiner(omp_out += omp_in) initializer(omp_priv(omp_orig))",
    )
    .expect("qualified C++ template-id and direct initializer must parse");
    assert!(matches!(
        reduction(&parsed).identifier(),
        OmpReductionIdentifier::Name(OmpIdExpression::CppTemplateId(_))
    ));
    assert!(matches!(
        reduction(&parsed).initializer(),
        Some(OmpReductionInitializer::CppDirect(expression))
            if expression.to_string() == "omp_priv(omp_orig)"
    ));

    let operator =
        cpp("#pragma omp declare_reduction(ns::operator+ : widget) combiner(omp_out += omp_in)")
            .expect("qualified C++ operator-function-id must parse");
    assert!(matches!(
        reduction(&operator).identifier(),
        OmpReductionIdentifier::Name(OmpIdExpression::CppOperatorFunction(_))
    ));

    for (source, expected) in [
        (
            "#pragma omp declare_reduction(copy : widget) combiner(omp_out = omp_in) initializer(omp_priv = {omp_orig})",
            "omp_priv = {omp_orig}",
        ),
        (
            "#pragma omp declare_reduction(list : widget) combiner(omp_out = omp_in) initializer(omp_priv{omp_orig})",
            "omp_priv{omp_orig}",
        ),
        (
            "#pragma omp declare_reduction(call : widget) combiner(omp_out = omp_in) initializer(ns::init(omp_priv, omp_orig))",
            "ns::init(omp_priv, omp_orig)",
        ),
    ] {
        let parsed = cpp(source).unwrap_or_else(|error| panic!("rejected {source:?}: {error}"));
        assert_eq!(
            reduction(&parsed)
                .initializer()
                .expect("initializer is present")
                .to_string(),
            expected
        );
    }
}

#[test]
fn c_braced_and_function_initializers_are_not_raw_fallbacks() {
    let braced = c(
        OpenMpVersion::V6_0,
        "#pragma omp declare_reduction(pair : pair_t) combiner(omp_out = omp_in) initializer(omp_priv = {1, {2, 3}})",
    )
    .expect("nested C braced initializer must parse");
    assert!(matches!(
        reduction(&braced).initializer(),
        Some(OmpReductionInitializer::CAssignment(OmpInitializerValue::Braced(
            initializer
        ))) if initializer.elements().len() == 2
            && matches!(initializer.elements()[1], OmpInitializerValue::Braced(_))
    ));

    let call = c(
        OpenMpVersion::V6_0,
        "#pragma omp declare_reduction(init : item_t) combiner(omp_out = omp_in) initializer(init_item(&omp_priv, &omp_orig))",
    )
    .expect("C initializer function call must parse");
    assert!(matches!(
        reduction(&call).initializer(),
        Some(OmpReductionInitializer::COrCppFunctionCall(_))
    ));
}

#[test]
fn fortran_reduction_identifiers_and_statement_forms_remain_lossless() {
    let historical = "!$omp declare reduction(IAND : integer : omp_out = iand(omp_in, omp_out)) initializer(omp_priv = 0)";
    for version in [OpenMpVersion::V5_2, OpenMpVersion::V6_0] {
        let parsed = fortran(version, historical)
            .unwrap_or_else(|error| panic!("OpenMP {version} rejected {historical:?}: {error}"));
        assert!(matches!(
            reduction(&parsed).identifier(),
            OmpReductionIdentifier::FortranIntrinsic(OmpFortranReductionIntrinsic::Iand)
        ));
        assert!(matches!(
            reduction(&parsed).combiner(),
            OmpReductionCombiner::FortranAssignment(_)
        ));
        assert!(matches!(
            reduction(&parsed).initializer(),
            Some(OmpReductionInitializer::FortranAssignment(_))
        ));
    }

    let current = fortran(
        OpenMpVersion::V6_0,
        "!$omp declare_reduction(.combine. : integer) combiner(combine_values(omp_out, omp_in)) initializer(init_value(omp_priv, omp_orig))",
    )
    .expect("Fortran defined reduction operator and procedure forms must parse");
    assert!(matches!(
        reduction(&current).identifier(),
        OmpReductionIdentifier::FortranDefinedOperator(identifier)
            if identifier.as_str() == "combine"
    ));
    assert!(matches!(
        reduction(&current).combiner(),
        OmpReductionCombiner::FortranSubroutineCall(_)
    ));
    assert!(matches!(
        reduction(&current).initializer(),
        Some(OmpReductionInitializer::FortranSubroutineCall(_))
    ));
}

#[test]
fn ordinary_reduction_clauses_preserve_host_spelling_and_cpp_ids() {
    let parsed = fortran(
        OpenMpVersion::V6_0,
        "!$omp parallel reduction(.AND.: logical_value) reduction(IAND: bits) reduction(.custom.: object)",
    )
    .expect("Fortran reduction identifiers must parse");
    let identifiers = parsed
        .directive()
        .clauses()
        .iter()
        .map(|clause| {
            let ClauseData::Reduction { operator, .. } = clause.payload() else {
                panic!("expected reduction payload");
            };
            operator
        })
        .collect::<Vec<_>>();
    assert_eq!(identifiers[0], &OmpReductionIdentifier::FortranLogicalAnd);
    assert!(matches!(
        identifiers[1],
        OmpReductionIdentifier::FortranIntrinsic(OmpFortranReductionIntrinsic::Iand)
    ));
    assert!(matches!(
        identifiers[2],
        OmpReductionIdentifier::FortranDefinedOperator(identifier)
            if identifier.as_str() == "custom"
    ));
    assert_eq!(
        identifiers
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        [".and.", "iand", ".custom."]
    );

    let cpp = cpp("#pragma omp parallel reduction(ns::merge<int>: value)")
        .expect("C++ reduction clause accepts a qualified template-id");
    let ClauseData::Reduction { operator, .. } = cpp.directive().clauses()[0].payload() else {
        panic!("expected reduction payload");
    };
    assert!(matches!(
        operator,
        OmpReductionIdentifier::Name(OmpIdExpression::CppTemplateId(_))
    ));
}

#[test]
fn malformed_declare_reduction_shapes_are_hard_errors() {
    for source in [
        "#pragma omp declare_reduction(sum : int)",
        "#pragma omp declare_reduction(sum : int : omp_out += omp_in) combiner(omp_out += omp_in)",
        "#pragma omp declare_reduction(sum : int) combiner(omp_out += omp_in) combiner(omp_out += omp_in)",
        "#pragma omp declare_reduction(sum : int) combiner(omp_out += omp_in) initializer(other = 0)",
        "#pragma omp declare_reduction(sum : int) combiner(omp_out += omp_in) initializer({0})",
        "#pragma omp declare_reduction(sum : int) combiner(omp_out += omp_in) initializer(init_value(&omp_orig))",
    ] {
        assert!(
            c(OpenMpVersion::V6_0, source).is_err(),
            "unexpectedly accepted {source:?}"
        );
    }
    assert!(
        cpp(
            "#pragma omp declare_reduction(sum : int) combiner(omp_out += omp_in) initializer(init_value(omp_orig))",
        )
        .is_err(),
        "C++ initializer call must receive omp_priv or &omp_priv"
    );
    assert!(
        fortran(
            OpenMpVersion::V6_0,
            "!$omp declare_reduction(sum : integer) combiner(omp_out + omp_in)",
        )
        .is_err(),
        "Fortran combiner must be an assignment or subroutine reference"
    );
    for source in [
        "!$omp declare_reduction(sum : integer) combiner(omp_out(1) = omp_in)",
        "!$omp declare_reduction(sum : integer) combiner(omp_out = omp_in) initializer(omp_priv%field = 0)",
        "!$omp declare_reduction(sum : integer) combiner(omp_out = omp_in) initializer(init_value(omp_orig))",
    ] {
        assert!(
            fortran(OpenMpVersion::V6_0, source).is_err(),
            "unexpectedly accepted invalid Fortran assignment target in {source:?}"
        );
    }
}

#[test]
fn induction_identifiers_accept_cpp_ids_and_only_fortran_defined_operators() {
    for source in [
        "#pragma omp declare_induction(ns::step<int> : int) inductor(omp_var += omp_step) collector(omp_step * omp_idx)",
        "#pragma omp declare_induction(ns::operator+ : int) inductor(omp_var += omp_step) collector(omp_step * omp_idx)",
    ] {
        let parsed = cpp(source).unwrap_or_else(|error| panic!("rejected {source:?}: {error}"));
        let Some(OmpDirectiveParameter::DeclareInduction(induction)) =
            parsed.directive().parameter()
        else {
            panic!("expected typed declare-induction parameter");
        };
        assert!(matches!(induction.identifier(), OmpInductionIdentifier::Name(_)));
        assert!(parsed.directive().clauses().iter().any(|clause| matches!(
            clause.payload(),
            ClauseData::Inductor {
                expression: OmpInductorExpression::COrCppExpression(_)
            }
        )));
    }

    let defined = fortran(
        OpenMpVersion::V6_0,
        "!$omp declare_induction(.advance. : integer) inductor(omp_var = omp_var + omp_step) collector(omp_step * omp_idx)",
    )
    .expect("Fortran user-defined induction operator must parse");
    let Some(OmpDirectiveParameter::DeclareInduction(induction)) = defined.directive().parameter()
    else {
        panic!("expected typed declare-induction parameter");
    };
    assert!(matches!(
        induction.identifier(),
        OmpInductionIdentifier::DefinedOperator(identifier)
            if identifier.as_str() == "advance"
    ));
    assert!(defined.directive().clauses().iter().any(|clause| matches!(
        clause.payload(),
        ClauseData::Inductor {
            expression: OmpInductorExpression::FortranAssignment(_)
        }
    )));

    let procedure = fortran(
        OpenMpVersion::V6_0,
        "!$omp declare_induction(.advance. : integer) inductor(advance_value(omp_var, omp_step)) collector(omp_step * omp_idx)",
    )
    .expect("Fortran inductor subroutine reference must parse");
    assert!(procedure
        .directive()
        .clauses()
        .iter()
        .any(|clause| matches!(
            clause.payload(),
            ClauseData::Inductor {
                expression: OmpInductorExpression::FortranSubroutineCall(_)
            }
        )));

    assert!(
        fortran(
            OpenMpVersion::V6_0,
            "!$omp declare_induction(.advance. : integer) inductor(omp_var + omp_step) collector(omp_step * omp_idx)",
        )
        .is_err(),
        "a bare Fortran arithmetic expression is not an inductor expression"
    );

    for operator in ["and", "or", "eqv", "neqv", "not", "true", "false"] {
        let source = format!(
            "!$omp declare_induction(.{operator}. : integer) inductor(omp_var = omp_var + omp_step) collector(omp_step * omp_idx)"
        );
        assert!(
            fortran(OpenMpVersion::V6_0, &source).is_err(),
            "unexpectedly accepted intrinsic dotted token .{operator}."
        );
    }
}
