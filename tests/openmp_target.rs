use roup::api::{OpenMpConfig, OpenMpParser};
use roup::ast::{OmpClauseKind, OmpDirectiveKind, OmpReductionIdentifier};
use roup::ir::{
    ClauseData, ClauseItem, MapType, ScheduleKind, UsesAllocatorBuiltin, UsesAllocatorKind,
};
use roup::version::{CStandard, HostLanguageProfile, SourceForm};

fn parser() -> OpenMpParser {
    OpenMpConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .unwrap()
        .parser()
}

fn item_name(item: &ClauseItem) -> &str {
    match item {
        ClauseItem::Identifier(identifier) => identifier.as_str(),
        ClauseItem::Variable(variable) => variable.expression().source(),
        ClauseItem::FortranCommonBlock(name) => name.as_str(),
        ClauseItem::Expression(expression) => expression.source(),
        ClauseItem::OmpparserTrailingSlash(identifier) => identifier.as_str(),
    }
}

#[test]
fn target_mapping_keeps_map_semantics_and_array_section_ast() {
    let parsed = parser()
        .parse("#pragma omp target if(device) device(0) map(tofrom:array[0:N]) nowait")
        .expect("valid target directive");
    let directive = parsed.directive();

    assert_eq!(directive.kind(), OmpDirectiveKind::Target);
    assert_eq!(
        directive
            .clauses()
            .iter()
            .map(|clause| clause.kind())
            .collect::<Vec<_>>(),
        [
            OmpClauseKind::If,
            OmpClauseKind::Device,
            OmpClauseKind::Map,
            OmpClauseKind::Nowait,
        ]
    );

    let ClauseData::Map {
        map_type,
        modifiers,
        mapper,
        iterators,
        locators,
        ..
    } = directive.clauses()[2].payload()
    else {
        panic!("map must have a typed payload");
    };
    assert_eq!(*map_type, Some(MapType::ToFrom));
    assert!(modifiers.is_empty());
    assert!(mapper.is_none());
    assert!(iterators.is_empty());
    assert_eq!(
        locators.iter().map(ToString::to_string).collect::<Vec<_>>(),
        ["array[0:N]"]
    );
}

#[test]
fn deeply_combined_target_keeps_allocator_and_reduction_data() {
    let parsed = parser()
        .parse(
            "#pragma omp target teams distribute parallel for simd num_teams(4) thread_limit(128) schedule(dynamic,8) reduction(*:prod) uses_allocators(omp_default_mem_alloc)",
        )
        .expect("valid combined target directive");
    let directive = parsed.directive();

    assert_eq!(
        directive.kind(),
        OmpDirectiveKind::TargetTeamsDistributeParallelForSimd
    );
    let ClauseData::Schedule {
        kind, chunk_size, ..
    } = directive.clauses()[2].payload()
    else {
        panic!("expected schedule payload");
    };
    assert_eq!(*kind, ScheduleKind::Dynamic);
    assert_eq!(chunk_size.as_ref().map(|value| value.source()), Some("8"));

    let ClauseData::Reduction {
        operator, items, ..
    } = directive.clauses()[3].payload()
    else {
        panic!("expected reduction payload");
    };
    assert_eq!(operator, &OmpReductionIdentifier::Multiply);
    assert_eq!(items.iter().map(item_name).collect::<Vec<_>>(), ["prod"]);

    let ClauseData::UsesAllocators { allocators } = directive.clauses()[4].payload() else {
        panic!("uses_allocators must be typed");
    };
    assert_eq!(allocators.len(), 1);
    assert_eq!(
        allocators[0].allocator(),
        &UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::Default)
    );
}

#[test]
fn malformed_map_sections_are_hard_errors() {
    for source in [
        "#pragma omp target map(to:)",
        "#pragma omp target map(to:array[0:@])",
        "#pragma omp target map(bogus:array)",
    ] {
        assert!(parser().parse(source).is_err(), "{source} must be rejected");
    }
}
