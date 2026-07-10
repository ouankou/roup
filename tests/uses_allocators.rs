use roup::api::{OpenMpConfig, ParsedOpenMpDirective};
use roup::diagnostic::Diagnostic;
use roup::ir::{ClauseData, UsesAllocatorBuiltin, UsesAllocatorKind};
use roup::version::{CStandard, FortranStandard, HostLanguageProfile, SourceForm};

fn parse(source: &str) -> Result<ParsedOpenMpDirective, Diagnostic> {
    OpenMpConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .expect("valid C configuration")
        .parser()
        .parse(source)
}

fn allocator_specs(source: &str) -> Vec<roup::ir::UsesAllocatorSpec> {
    let parsed = parse(source).expect("directive must parse");
    let ClauseData::UsesAllocators { allocators } = parsed.directive().clauses()[0].payload()
    else {
        panic!("expected a uses_allocators payload");
    };
    allocators.clone()
}

#[test]
fn historical_and_modifier_forms_share_one_semantic_shape() {
    let historical = allocator_specs(
        "#pragma omp target uses_allocators(omp_default_mem_alloc, custom_allocator(custom_traits))",
    );
    assert_eq!(historical.len(), 2);
    assert_eq!(
        historical[0].allocator(),
        &UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::Default)
    );
    assert!(historical[0].traits().is_none());
    assert!(historical[1].traits().is_some());
    assert!(historical.iter().all(|entry| entry.memspace().is_none()));

    let modifier = allocator_specs(
        "#pragma omp target uses_allocators(memspace(omp_high_bw_mem_space), traits(custom_traits): custom_allocator)",
    );
    assert_eq!(modifier.len(), 1);
    assert!(modifier[0].memspace().is_some());
    assert!(modifier[0].traits().is_some());

    let historical_custom =
        allocator_specs("#pragma omp target uses_allocators(custom_allocator(custom_traits))");
    let canonical_custom = allocator_specs(
        "#pragma omp target uses_allocators(traits(custom_traits): custom_allocator)",
    );
    assert_eq!(historical_custom, canonical_custom);
}

#[test]
fn semicolon_specs_are_not_flattened_into_a_comma_list() {
    let allocators = allocator_specs(
        "#pragma omp target uses_allocators(traits(first_traits): first_allocator; memspace(omp_low_lat_mem_space): second_allocator)",
    );
    assert_eq!(allocators.len(), 2);
    assert!(allocators[0].traits().is_some());
    assert!(allocators[0].memspace().is_none());
    assert!(allocators[1].traits().is_none());
    assert!(allocators[1].memspace().is_some());
}

#[test]
fn malformed_entries_and_predefined_allocator_modifiers_are_hard_errors() {
    for source in [
        "#pragma omp target uses_allocators()",
        "#pragma omp target uses_allocators(traits(): allocator)",
        "#pragma omp target uses_allocators(memspace(): allocator)",
        "#pragma omp target uses_allocators(traits(t), traits(u): allocator)",
        "#pragma omp target uses_allocators(memspace(m), memspace(n): allocator)",
        "#pragma omp target uses_allocators(unknown_modifier: allocator)",
        "#pragma omp target uses_allocators(traits(t) trailing: allocator)",
        "#pragma omp target uses_allocators(traits(t): omp_default_mem_alloc)",
        "#pragma omp target uses_allocators(omp_default_mem_alloc(t))",
        "#pragma omp target uses_allocators(traits(t): allocator;)",
        "#pragma omp target uses_allocators(; traits(t): allocator)",
    ] {
        assert!(parse(source).is_err(), "unexpectedly accepted {source:?}");
    }
}

#[test]
fn allocator_builtins_are_host_language_aware() {
    let c = allocator_specs("#pragma omp target uses_allocators(OMP_DEFAULT_MEM_ALLOC)");
    assert!(matches!(c[0].allocator(), UsesAllocatorKind::Custom(_)));

    let fortran = OpenMpConfig::new(
        HostLanguageProfile::Fortran(FortranStandard::Fortran2023),
        SourceForm::FortranFree,
    )
    .expect("valid Fortran configuration")
    .parser()
    .parse("!$omp target uses_allocators(OMP_DEFAULT_MEM_ALLOC)")
    .expect("Fortran built-in names are case-insensitive");
    let ClauseData::UsesAllocators { allocators } = fortran.directive().clauses()[0].payload()
    else {
        panic!("expected uses_allocators");
    };
    assert_eq!(
        allocators[0].allocator(),
        &UsesAllocatorKind::Builtin(UsesAllocatorBuiltin::Default)
    );
}
