use roup::api::{OpenAccConfig, OpenMpConfig, ParsedOpenAccDirective, ParsedOpenMpDirective};
use roup::ast::{
    AccClausePayload, OmpClauseKind, OmpClausePayload, OmpDirectiveKind, OmpDirectiveParameter,
};
use roup::diagnostic::Diagnostic;
use roup::version::{CStandard, FortranStandard, HostLanguageProfile, SourceForm};

fn omp(input: &str) -> Result<ParsedOpenMpDirective, Diagnostic> {
    OpenMpConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .expect("valid C configuration")
        .parser()
        .parse(input)
}

fn omp_fortran(input: &str) -> Result<ParsedOpenMpDirective, Diagnostic> {
    OpenMpConfig::new(
        HostLanguageProfile::Fortran(FortranStandard::Fortran2023),
        SourceForm::FortranFree,
    )
    .expect("valid Fortran configuration")
    .parser()
    .parse(input)
}

fn acc(input: &str) -> Result<ParsedOpenAccDirective, Diagnostic> {
    OpenAccConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .expect("valid C configuration")
        .parser()
        .parse(input)
}

#[test]
fn schedule_keeps_nested_comma_expression() {
    let parsed = omp("#pragma omp for schedule(static, f(a,b))").unwrap();
    let OmpClausePayload::Schedule { chunk_size, .. } = parsed.directive().clauses()[0].payload()
    else {
        panic!("expected schedule payload");
    };
    assert_eq!(chunk_size.as_ref().unwrap().to_string(), "f(a, b)");
}

#[test]
fn schedule_rejects_silently_ignored_third_field() {
    assert!(omp("#pragma omp for schedule(static, 4, ignored)").is_err());
}

#[test]
fn device_accepts_ternary_expression_colon() {
    let parsed = omp("#pragma omp target device(flag ? 0 : 1)").unwrap();
    let OmpClausePayload::Device { device_num, .. } = parsed.directive().clauses()[0].payload()
    else {
        panic!("expected device payload");
    };
    assert_eq!(device_num.to_string(), "flag ? 0 : 1");
}

#[test]
fn if_modifier_disambiguation_preserves_ternary_expressions() {
    let ternary = omp("#pragma omp parallel if(flag ? enabled : disabled)")
        .expect("a ternary colon is part of the host expression");
    assert!(
        ternary.directive().clauses()[0]
            .directive_name_modifier()
            .is_none()
    );

    let modified = omp("#pragma omp target parallel if(target: enabled)")
        .expect("a real directive-name modifier must remain typed");
    assert_eq!(
        modified.directive().clauses()[0].directive_name_modifier(),
        Some(roup::ast::OmpDirectiveKind::Target)
    );

    assert!(omp("#pragma omp parallel if(typo: enabled)").is_err());
}

#[test]
fn scalar_openmp_slots_reject_top_level_comma_but_keep_nested_comma() {
    for source in [
        "#pragma omp parallel if(a, b)",
        "#pragma omp for collapse(a, b)",
        "#pragma omp task priority(a, b)",
        "#pragma omp masked filter(a, b)",
        "#pragma omp target device(a, b)",
    ] {
        assert!(omp(source).is_err(), "unexpectedly accepted {source:?}");
    }

    for source in [
        "#pragma omp parallel if((a, b))",
        "#pragma omp for collapse((a, b))",
        "#pragma omp task priority(select(a, b))",
        "#pragma omp masked filter(select(a, b))",
        "#pragma omp target device(select(a, b))",
    ] {
        omp(source).unwrap_or_else(|error| panic!("rejected {source:?}: {error}"));
    }
}

#[test]
fn message_rejects_parenthesized_and_signed_non_string_literals() {
    for source in [
        "#pragma omp error message((42))",
        "#pragma omp error message(+42)",
        "#pragma omp error message(-42)",
    ] {
        assert!(omp(source).is_err(), "unexpectedly accepted {source:?}");
    }
    assert!(omp("#pragma omp error message(runtime_message)").is_ok());
}

#[test]
fn unknown_reduction_modifier_is_a_hard_error() {
    assert!(omp("#pragma omp parallel reduction(bogus,+:x)").is_err());
}

#[test]
fn reduction_modifier_lists_reject_empty_or_reconstructed_fields() {
    for source in [
        "#pragma omp parallel reduction(,+: x)",
        "#pragma omp parallel reduction(task,,+: x)",
        "#pragma omp parallel reduction(original():+: x)",
        "#pragma omp parallel reduction(original(sharing=private,),+: x)",
        "#pragma omp parallel reduction(original(,sharing=private),+: x)",
        "#pragma omp parallel reduction(original(sharing=private) junk,+: x)",
    ] {
        assert!(omp(source).is_err(), "accepted {source:?}");
    }
}

#[test]
fn duplicate_if_modifier_is_a_hard_error() {
    assert!(omp("#pragma omp parallel if(a) if(b)").is_err());
    assert!(omp("#pragma omp parallel if(parallel: a) if(parallel: b)").is_err());
}

#[test]
fn variable_list_clauses_never_fabricate_an_empty_typed_list() {
    for source in [
        "#pragma omp simd nontemporal",
        "#pragma omp declare simd uniform",
        "#pragma omp interop use",
    ] {
        assert!(
            omp(source).is_err(),
            "missing variable list unexpectedly parsed: {source:?}"
        );
    }
}

#[test]
fn declare_induction_rejects_unparsed_suffix() {
    assert!(omp("#pragma omp declare induction (x) @junk").is_err());
}

#[test]
fn nested_directive_preserves_parameter_and_rejects_suffix() {
    let parsed = omp("#pragma omp metadirective when(device={kind(cpu)}: critical(lock))").unwrap();
    let OmpClausePayload::MetadirectiveSelector { selector } =
        parsed.directive().clauses()[0].payload()
    else {
        panic!("expected metadirective selector");
    };
    let nested = selector.nested_directive().unwrap();
    assert!(matches!(
        nested.parameter(),
        Some(OmpDirectiveParameter::CriticalSection(name)) if name.as_str() == "lock"
    ));

    assert!(
        omp("#pragma omp metadirective when(device={kind(cpu)}: critical(lock) @junk)").is_err()
    );
}

#[test]
fn openacc_wait_handles_multibyte_expression_without_panicking() {
    let parsed = acc("#pragma acc parallel wait(devnum: λ: queues: 1)").unwrap();
    let AccClausePayload::Wait(wait) = parsed.directive().clauses()[0].payload() else {
        panic!("expected wait payload");
    };
    assert_eq!(wait.devnum().unwrap().to_string(), "λ");
}

#[test]
fn openacc_helper_uses_the_same_strict_path() {
    assert!(acc("#pragma acc parallel default(garbage)").is_err());
}

#[test]
fn openacc_variable_lists_reject_empty_entries() {
    for source in [
        "#pragma acc parallel private(a,,b)",
        "#pragma acc parallel private(,a)",
        "#pragma acc parallel private(a,)",
    ] {
        assert!(
            acc(source).is_err(),
            "empty list entry unexpectedly disappeared: {source:?}"
        );
    }
}

#[test]
fn compact_comparisons_remain_distinct_expression_list_entries() {
    let parsed = acc("#pragma acc parallel num_gangs(a<b,c>d)").unwrap();
    let AccClausePayload::NumGangs(values) = parsed.directive().clauses()[0].payload() else {
        panic!("expected num_gangs payload");
    };
    assert_eq!(values.len(), 2);
    assert_eq!(values[0].to_string(), "a < b");
    assert_eq!(values[1].to_string(), "c > d");
}

#[test]
fn malformed_delimiter_and_quote_lists_are_hard_errors() {
    for source in [
        "#pragma acc parallel num_gangs(a],b)",
        "#pragma acc parallel num_gangs(f(a],b)",
        "#pragma acc parallel num_gangs(\"unterminated,b)",
        "#pragma acc parallel num_gangs(a,,b)",
    ] {
        assert!(acc(source).is_err(), "malformed list parsed: {source:?}");
    }
}

#[test]
fn structured_openmp_lists_reject_entries_that_used_to_disappear() {
    for source in [
        "#pragma omp target map(iterator(i=0:n,,j=0:m), to: a)",
        "#pragma omp target map(iterator(i=0:n,), to: a)",
        "#pragma omp tile apply(unroll,,reverse)",
        "#pragma omp tile apply(unroll,)",
        "#pragma omp tile apply(: unroll)",
        "#pragma omp tile induction()",
        "#pragma omp tile induction(step(2),)",
        "#pragma omp tile induction(: n)",
        "#pragma omp parallel allocate(: x)",
        "#pragma omp parallel allocate(omp_default_mem_alloc:)",
    ] {
        assert!(
            omp(source).is_err(),
            "malformed list unexpectedly parsed: {source:?}"
        );
    }
}

#[test]
fn historical_metadirective_default_has_the_canonical_otherwise_payload() {
    let historical = omp("#pragma omp metadirective default(parallel)").unwrap();
    let canonical = omp("#pragma omp metadirective otherwise(parallel)").unwrap();

    let historical_clause = &historical.directive().clauses()[0];
    let canonical_clause = &canonical.directive().clauses()[0];
    assert_eq!(historical_clause.kind(), OmpClauseKind::Otherwise);
    assert_eq!(canonical_clause.kind(), OmpClauseKind::Otherwise);
    for clause in [historical_clause, canonical_clause] {
        let OmpClausePayload::MetadirectiveSelector { selector } = clause.payload() else {
            panic!("expected one canonical selector payload");
        };
        assert!(selector.entries().is_empty());
        assert_eq!(
            selector
                .nested_directive()
                .map(|directive| directive.kind()),
            Some(OmpDirectiveKind::Parallel)
        );
    }
}

#[test]
fn malformed_recovery_inputs_are_hard_errors() {
    for input in [
        "#pragma omp parallel /*",
        "#pragma omp parallel \\",
        "#pragma omp parallel,,,,",
        "#pragma omp parallel, private(x)",
        "#pragma omp parallel private(x),",
        "#pragma omp atomic, read",
        "!$omp omp parallel",
        "!$omp parallel &",
    ] {
        let result = if input.starts_with("!$") {
            omp_fortran(input)
        } else {
            omp(input)
        };
        assert!(
            result.is_err(),
            "malformed input unexpectedly parsed: {input:?}"
        );
    }
}

#[test]
fn required_directive_parameters_are_never_reinterpreted_as_bare_forms() {
    for source in [
        "#pragma omp allocate",
        "#pragma omp allocate(a",
        "#pragma omp threadprivate",
        "#pragma omp threadprivate(a",
        "#pragma omp declare mapper map(a)",
        "#pragma omp declare mapper(a map(a)",
        "#pragma omp declare variant match(construct={parallel})",
        "#pragma omp depobj update(inout)",
        "#pragma omp cancel",
        "#pragma omp cancellation point",
        "#pragma omp groupprivate",
    ] {
        assert!(omp(source).is_err(), "unexpectedly accepted {source:?}");
    }
}

#[test]
fn convenience_parser_does_not_guess_the_source_language() {
    assert!(omp("!$omp parallel").is_err());
}

#[test]
fn continuation_processing_does_not_rewrite_string_literal_whitespace() {
    let parsed = omp("#pragma omp parallel if(f(\"a  b\", \\\n x))").unwrap();
    let OmpClausePayload::If { condition, .. } = parsed.directive().clauses()[0].payload() else {
        panic!("expected if payload");
    };
    assert_eq!(condition.to_string(), "f(\"a  b\", x)");
}

#[test]
fn delimiter_inside_string_literal_does_not_close_clause() {
    let parsed = omp("#pragma omp error message(\")\")").unwrap();
    let OmpClausePayload::Message { value: message } = parsed.directive().clauses()[0].payload()
    else {
        panic!("expected message expression");
    };
    assert_eq!(message.to_string(), "\")\"");
}
