use roup::api::{OpenAccConfig, OpenMpConfig, ParsedOpenAccDirective, ParsedOpenMpDirective};
use roup::ast::{OmpClausePayload, OmpSelectorEntry};
use roup::diagnostic::Diagnostic;
use roup::version::{CStandard, HostLanguageProfile, SourceForm};

fn omp(input: &str) -> Result<ParsedOpenMpDirective, Diagnostic> {
    OpenMpConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .expect("valid C configuration")
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
fn defaultmap_rejects_internal_sentinels_and_empty_fields() {
    for source in [
        "#pragma omp target defaultmap()",
        "#pragma omp target defaultmap(unspecified)",
        "#pragma omp target defaultmap(tofrom:)",
        "#pragma omp target defaultmap(:scalar)",
    ] {
        assert!(omp(source).is_err(), "unexpectedly accepted {source:?}");
    }
}

#[test]
fn historical_defaultmap_forms_remain_accepted() {
    for source in [
        "#pragma omp target defaultmap(alloc)",
        "#pragma omp target defaultmap(to:scalar)",
        "#pragma omp target defaultmap(from:aggregate)",
        "#pragma omp target defaultmap(tofrom:pointer)",
        "#pragma omp target defaultmap(firstprivate)",
        "#pragma omp target defaultmap(none)",
        "#pragma omp target defaultmap(default:all)",
        "#pragma omp target defaultmap(present)",
    ] {
        assert!(omp(source).is_ok(), "rejected historical syntax {source:?}");
    }
}

#[test]
fn uses_allocators_rejects_malformed_or_forbidden_modifiers() {
    for source in [
        "#pragma omp target uses_allocators()",
        "#pragma omp target uses_allocators(traits(): allocator)",
        "#pragma omp target uses_allocators(memspace(): allocator)",
        "#pragma omp target uses_allocators(traits(t), traits(u): allocator)",
        "#pragma omp target uses_allocators(memspace(m), memspace(n): allocator)",
        "#pragma omp target uses_allocators(memspace(custom_space): allocator)",
        "#pragma omp target uses_allocators(target, traits(t): allocator)",
        "#pragma omp target uses_allocators(unknown_modifier: allocator)",
        "#pragma omp target uses_allocators(traits(t) trailing: allocator)",
        "#pragma omp target uses_allocators(traits(t): omp_default_mem_alloc)",
        "#pragma omp target uses_allocators(omp_default_mem_alloc(t))",
        "#pragma omp target uses_allocators(traits(t): allocator;)",
        "#pragma omp target uses_allocators(; traits(t): allocator)",
    ] {
        assert!(omp(source).is_err(), "unexpectedly accepted {source:?}");
    }
}

#[test]
fn every_standardized_uses_allocators_shape_is_typed() {
    for source in [
        "#pragma omp target uses_allocators(omp_default_mem_alloc)",
        "#pragma omp target uses_allocators(custom_allocator(custom_traits), another_allocator(another_traits))",
        "#pragma omp target uses_allocators(traits(custom_traits), memspace(omp_high_bw_mem_space): custom_allocator)",
        "#pragma omp target uses_allocators(first_allocator; second_allocator)",
    ] {
        assert!(omp(source).is_ok(), "rejected standardized syntax {source:?}");
    }
}

#[test]
fn requires_rejects_unknown_and_malformed_requirements() {
    for source in [
        "#pragma omp requires typo_requirement",
        "#pragma omp requires atomic_default_mem_order",
        "#pragma omp requires atomic_default_mem_order()",
        "#pragma omp requires atomic_default_mem_order(seq_cst))",
    ] {
        assert!(omp(source).is_err(), "unexpectedly accepted {source:?}");
    }

    assert!(omp("#pragma omp requires unified_shared_memory").is_ok());
    assert!(omp("#pragma omp requires atomic_default_mem_order(seq_cst)").is_ok());
}

#[test]
fn selector_entries_are_never_dropped_or_overwritten() {
    for source in [
        "#pragma omp metadirective when(device: parallel)",
        "#pragma omp metadirective when(device=kind(cpu): parallel)",
        "#pragma omp metadirective when(device={}: parallel)",
        "#pragma omp metadirective when(device={kind cpu}: parallel)",
        "#pragma omp metadirective when(device={kind(cpu)}, device={isa(avx)}: parallel)",
        "#pragma omp metadirective when(device={kind(cpu), kind(gpu)}: parallel)",
        "#pragma omp metadirective when(impl={vendor(llvm)}: parallel)",
        "#pragma omp metadirective when(constructs={parallel}: parallel)",
    ] {
        assert!(omp(source).is_err(), "unexpectedly accepted {source:?}");
    }
}

#[test]
fn standardized_selector_spelling_still_builds_typed_data() {
    let parsed = omp(
        "#pragma omp metadirective when(device={kind(cpu), isa(avx2)}, implementation={vendor(llvm)}, user={condition(flag)}, construct={parallel}: parallel)",
    )
    .unwrap();
    let OmpClausePayload::MetadirectiveSelector { selector } =
        parsed.directive().clauses()[0].payload()
    else {
        panic!("expected typed metadirective selector");
    };
    assert!(selector
        .entries()
        .iter()
        .any(|entry| matches!(entry, OmpSelectorEntry::Device { .. })));
    assert!(selector
        .entries()
        .iter()
        .any(|entry| matches!(entry, OmpSelectorEntry::Implementation { .. })));
    assert!(selector
        .entries()
        .iter()
        .any(|entry| matches!(entry, OmpSelectorEntry::User { .. })));
    assert!(selector
        .entries()
        .iter()
        .any(|entry| matches!(entry, OmpSelectorEntry::Construct { .. })));
    assert!(selector.nested_directive().is_some());
}

#[test]
fn empty_openacc_variable_lists_are_hard_errors() {
    for source in [
        "#pragma acc parallel copy()",
        "#pragma acc parallel copyin()",
        "#pragma acc parallel copyout()",
        "#pragma acc parallel create()",
        "#pragma acc parallel present()",
        "#pragma acc parallel private()",
        "#pragma acc parallel device_type()",
        "#pragma acc parallel reduction(+:)",
    ] {
        assert!(acc(source).is_err(), "unexpectedly accepted {source:?}");
    }
}

#[test]
fn empty_openacc_expression_payloads_are_hard_errors() {
    for source in [
        "#pragma acc parallel async()",
        "#pragma acc parallel bind()",
        "#pragma acc parallel collapse()",
        "#pragma acc parallel default_async()",
        "#pragma acc parallel device_num()",
        "#pragma acc parallel if()",
        "#pragma acc parallel num_gangs()",
        "#pragma acc parallel num_workers()",
        "#pragma acc parallel tile()",
        "#pragma acc parallel vector_length()",
        "#pragma acc parallel wait()",
        "#pragma acc parallel gang()",
        "#pragma acc parallel worker()",
        "#pragma acc parallel vector()",
        "#pragma acc parallel indirect()",
    ] {
        assert!(acc(source).is_err(), "unexpectedly accepted {source:?}");
    }
}

#[test]
fn unknown_openacc_clauses_do_not_use_a_generic_payload() {
    assert!(acc("#pragma acc parallel typo_clause(value)").is_err());
}

#[test]
fn standardized_historical_openacc_aliases_remain_accepted() {
    for source in [
        "#pragma acc parallel pcopy(a)",
        "#pragma acc parallel present_or_copy(b)",
        "#pragma acc parallel pcopyin(c)",
        "#pragma acc parallel present_or_copyin(d)",
        "#pragma acc parallel pcopyout(e)",
        "#pragma acc parallel present_or_copyout(f)",
        "#pragma acc parallel pcreate(g)",
        "#pragma acc parallel present_or_create(h)",
        "#pragma acc parallel async",
        "#pragma acc parallel loop gang worker vector",
    ] {
        assert!(acc(source).is_ok(), "rejected historical syntax {source:?}");
    }
}

#[test]
fn openacc_directive_parameters_cannot_be_empty() {
    for source in [
        "#pragma acc cache()",
        "#pragma acc cache(readonly:)",
        "#pragma acc wait()",
        "#pragma acc wait(devnum: 1)",
        "#pragma acc routine()",
        "#pragma acc end",
    ] {
        assert!(acc(source).is_err(), "unexpectedly accepted {source:?}");
    }

    assert!(acc("#pragma acc cache(readonly: a[0])").is_ok());
    assert!(acc("#pragma acc wait(1)").is_ok());
    assert!(acc("#pragma acc routine(worker_fn)").is_ok());
    assert!(acc("#pragma acc end parallel").is_err());
}
