use roup::api::{OpenMpConfig, OpenMpParser};
use roup::ast::{
    OmpClauseKind, OmpDirectiveKind, OmpDirectiveParameter, OmpSelectorDeviceTrait,
    OmpSelectorEntry, OmpSelectorNameListKind, OmpSelectorTraitValue,
};
use roup::ir::ClauseData;
use roup::version::{CStandard, HostLanguageProfile, SourceForm};

fn parser() -> OpenMpParser {
    OpenMpConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .unwrap()
        .parser()
}

#[test]
fn context_selectors_and_nested_directive_are_fully_typed() {
    let parsed = parser()
        .parse(
            "#pragma omp metadirective when(device={kind(cpu), isa(avx2)}, implementation={vendor(llvm)}, user={condition(enabled)}, construct={parallel}: parallel num_threads(4)) otherwise(nothing)",
        )
        .expect("valid metadirective");
    let directive = parsed.directive();

    assert_eq!(directive.kind(), OmpDirectiveKind::Metadirective);
    assert_eq!(directive.clauses()[0].kind(), OmpClauseKind::When);
    let ClauseData::MetadirectiveSelector { selector } = directive.clauses()[0].payload() else {
        panic!("when must have a typed selector");
    };
    assert_eq!(selector.entries().len(), 4);

    let OmpSelectorEntry::Device { traits, .. } = &selector.entries()[0] else {
        panic!("first selector set must be device");
    };
    assert!(matches!(
        &traits[0],
        OmpSelectorDeviceTrait::NameList(value)
            if value.kind() == OmpSelectorNameListKind::Kind
                && matches!(value.properties(), [OmpSelectorTraitValue::Identifier(name)] if name.as_str() == "cpu")
    ));
    assert!(matches!(
        &traits[1],
        OmpSelectorDeviceTrait::NameList(value)
            if value.kind() == OmpSelectorNameListKind::Isa
                && matches!(value.properties(), [OmpSelectorTraitValue::Identifier(name)] if name.as_str() == "avx2")
    ));
    assert!(matches!(
        &selector.entries()[2],
        OmpSelectorEntry::User { condition, .. } if condition.source() == "enabled"
    ));
    assert!(matches!(
        selector.entries()[3],
        OmpSelectorEntry::Construct { .. }
    ));

    let nested = selector
        .nested_directive()
        .expect("when selector must retain its nested directive");
    assert_eq!(nested.kind(), OmpDirectiveKind::Parallel);
    assert_eq!(nested.clauses()[0].kind(), OmpClauseKind::NumThreads);

    assert_eq!(directive.clauses()[1].kind(), OmpClauseKind::Otherwise);
    let ClauseData::MetadirectiveSelector { selector } = directive.clauses()[1].payload() else {
        panic!("otherwise must have a typed nested directive");
    };
    assert_eq!(
        selector.nested_directive().map(|nested| nested.kind()),
        Some(OmpDirectiveKind::Nothing)
    );
}

#[test]
fn nested_directive_parameters_survive_without_render_and_reparse() {
    let parsed = parser()
        .parse("#pragma omp metadirective when(device={kind(cpu)}: critical(lock))")
        .expect("valid nested critical directive");
    let ClauseData::MetadirectiveSelector { selector } = parsed.directive().clauses()[0].payload()
    else {
        panic!("expected typed selector");
    };
    let nested = selector.nested_directive().unwrap();

    assert_eq!(nested.kind(), OmpDirectiveKind::Critical);
    assert!(matches!(
        nested.parameter(),
        Some(OmpDirectiveParameter::CriticalSection(name)) if name.as_str() == "lock"
    ));
}

#[test]
fn malformed_selector_or_nested_suffix_is_a_hard_error() {
    for source in [
        "#pragma omp metadirective when(device: parallel)",
        "#pragma omp metadirective when(device={}: parallel)",
        "#pragma omp metadirective when(device={kind(cpu)}, device={isa(avx2)}: parallel)",
        "#pragma omp metadirective when(device={kind(cpu)}: critical(lock) @junk)",
        "#pragma omp metadirective when(user={condition(flag)}:)",
    ] {
        assert!(parser().parse(source).is_err(), "{source} must be rejected");
    }
}
