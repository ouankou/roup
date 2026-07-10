use roup::api::{OpenAccConfig, OpenMpConfig};
use roup::ast::{
    AccClauseKind, OmpClauseKind, OmpClausePayload, OmpDirectiveKind, OmpSelectorEntry,
};
use roup::version::{CStandard, HostLanguageProfile, SourceForm};

fn c23() -> HostLanguageProfile {
    HostLanguageProfile::C(CStandard::C23)
}

#[test]
fn c_spliced_directive_and_clause_spans_map_to_physical_source() {
    let source = concat!(
        "#pragma omp paral\\\n",
        "lel private(x) \\\n",
        " num_threads(2)"
    );
    let parsed = OpenMpConfig::new(c23(), SourceForm::Pragma)
        .unwrap()
        .parser()
        .parse(source)
        .unwrap();
    let directive = parsed.directive();

    assert_eq!(directive.kind(), OmpDirectiveKind::Parallel);
    assert_eq!(directive.span().slice(source), Ok("paral\\\nlel"));
    assert_eq!(directive.span().start().line(), 1);
    assert_eq!(directive.span().start().column(), 13);

    assert_eq!(directive.clauses().len(), 2);
    assert_eq!(directive.clauses()[0].kind(), OmpClauseKind::Private);
    assert_eq!(directive.clauses()[0].span().slice(source), Ok("private"));
    assert_eq!(directive.clauses()[0].span().start().line(), 2);
    assert_eq!(directive.clauses()[0].span().start().column(), 5);

    assert_eq!(directive.clauses()[1].kind(), OmpClauseKind::NumThreads);
    assert_eq!(
        directive.clauses()[1].span().slice(source),
        Ok("num_threads")
    );
    assert_eq!(directive.clauses()[1].span().start().line(), 3);
    assert_eq!(directive.clauses()[1].span().start().column(), 2);
}

#[test]
fn canonical_alias_keeps_the_exact_source_name_span() {
    let source = "#pragma omp ordered depend(source)";
    let parsed = OpenMpConfig::new(c23(), SourceForm::Pragma)
        .unwrap()
        .parser()
        .parse(source)
        .unwrap();
    let clause = &parsed.directive().clauses()[0];

    assert_eq!(clause.kind(), OmpClauseKind::Doacross);
    assert_eq!(clause.span().slice(source), Ok("depend"));
    assert_eq!(clause.span().start().column(), 21);
}

#[test]
fn openacc_clause_span_is_reported_without_input_scanning() {
    let source = "#pragma acc data present_or_copy(a) async(1)";
    let parsed = OpenAccConfig::new(c23(), SourceForm::Pragma)
        .unwrap()
        .parser()
        .parse(source)
        .unwrap();
    let directive = parsed.directive();

    assert_eq!(directive.span().slice(source), Ok("data"));
    assert_eq!(directive.clauses()[0].kind(), AccClauseKind::Copy);
    assert_eq!(
        directive.clauses()[0].span().slice(source),
        Ok("present_or_copy")
    );
    assert_eq!(directive.clauses()[1].kind(), AccClauseKind::Async);
    assert_eq!(directive.clauses()[1].span().slice(source), Ok("async"));
}

#[test]
fn metadirective_nested_spans_keep_the_outer_physical_source() {
    let source = concat!(
        "#pragma omp metadirective when(device={kind(cpu)}: paral\\\n",
        "lel private(x)) otherwise(nothing)"
    );
    let parsed = OpenMpConfig::new(c23(), SourceForm::Pragma)
        .unwrap()
        .parser()
        .parse(source)
        .unwrap();

    let OmpClausePayload::MetadirectiveSelector { selector } =
        parsed.directive().clauses()[0].payload()
    else {
        panic!("expected when selector payload");
    };
    let nested = selector.nested_directive().expect("nested when directive");
    assert_eq!(nested.span().slice(source), Ok("paral\\\nlel"));
    assert_eq!(nested.clauses()[0].span().slice(source), Ok("private"));

    let OmpClausePayload::MetadirectiveSelector { selector } =
        parsed.directive().clauses()[1].payload()
    else {
        panic!("expected otherwise selector payload");
    };
    let nested = selector
        .nested_directive()
        .expect("nested otherwise directive");
    assert_eq!(nested.span().slice(source), Ok("nothing"));
}

#[test]
fn construct_selector_parses_properties_without_reconstructed_source() {
    let source = concat!(
        "#pragma omp metadirective when(construct={si\\\n",
        "md(simdlen(4))}: parallel)"
    );
    let parsed = OpenMpConfig::new(c23(), SourceForm::Pragma)
        .unwrap()
        .parser()
        .parse(source)
        .unwrap();
    let OmpClausePayload::MetadirectiveSelector { selector } =
        parsed.directive().clauses()[0].payload()
    else {
        panic!("expected selector payload");
    };
    let construct = selector
        .entries()
        .iter()
        .find_map(|entry| match entry {
            OmpSelectorEntry::Construct { constructs } => constructs.first(),
            _ => None,
        })
        .expect("construct trait");

    assert_eq!(construct.directive().span().slice(source), Ok("si\\\nmd"));
    assert_eq!(
        construct.directive().clauses()[0].span().slice(source),
        Ok("simdlen")
    );
}
