use roup::api::{OpenMpConfig, ParsedOpenMpDirective};
use roup::ast::{OmpClausePayload, OmpDirectiveKind, OmpDirectiveParameter};
use roup::diagnostic::DiagnosticCode;
use roup::ir::{AtomicOp, ClauseData, ExtendedAtomicKind, MemoryOrder, RequireModifier};
use roup::version::{CStandard, HostLanguageProfile, OpenMpVersion, SourceForm};

fn c23() -> HostLanguageProfile {
    HostLanguageProfile::C(CStandard::C23)
}

fn parse(source: &str) -> ParsedOpenMpDirective {
    OpenMpConfig::new(c23(), SourceForm::Pragma)
        .expect("valid C configuration")
        .parser()
        .parse(source)
        .unwrap_or_else(|error| panic!("failed to parse {source:?}: {error}"))
}

fn payload(source: &str) -> OmpClausePayload {
    let parsed = parse(source);
    let clauses = parsed.directive().clauses();
    assert_eq!(clauses.len(), 1, "expected one clause in {source:?}");
    clauses[0].payload().clone()
}

fn assert_rejected_in_5_2(source: &str) {
    let error = OpenMpConfig::exact(OpenMpVersion::V5_2, c23(), SourceForm::Pragma)
        .expect("valid exact-version configuration")
        .parser()
        .parse(source)
        .unwrap_err();
    assert_eq!(
        error.code(),
        DiagnosticCode::NotAvailableInVersion,
        "unexpected error for {source:?}: {error}"
    );
}

fn assert_accepted(version: OpenMpVersion, source: &str) {
    OpenMpConfig::exact(version, c23(), SourceForm::Pragma)
        .expect("valid exact-version configuration")
        .parser()
        .parse(source)
        .unwrap_or_else(|error| panic!("{source:?} rejected in {version}: {error}"));
}

#[test]
fn openmp_60_condition_bearing_forms_are_versioned_independently_from_bare_forms() {
    let conditioned = [
        "#pragma omp requires reverse_offload(flag)",
        "#pragma omp requires unified_address(flag)",
        "#pragma omp requires unified_shared_memory(flag)",
        "#pragma omp requires dynamic_allocators(flag)",
        "#pragma omp requires self_maps(flag)",
        "#pragma omp requires device_safesync(flag)",
        "#pragma omp atomic read(flag)",
        "#pragma omp atomic update(flag)",
        "#pragma omp atomic write(flag)",
        "#pragma omp atomic capture(flag)",
        "#pragma omp atomic compare(flag)",
        "#pragma omp atomic compare weak(flag)",
        "#pragma omp atomic acq_rel(flag)",
        "#pragma omp atomic acquire(flag)",
        "#pragma omp atomic relaxed(flag)",
        "#pragma omp atomic release(flag)",
        "#pragma omp atomic seq_cst(flag)",
        "#pragma omp for nowait(flag)",
        "#pragma omp taskloop nogroup(flag)",
        "#pragma omp declare simd inbranch(flag)",
        "#pragma omp declare simd notinbranch(flag)",
        "#pragma omp unroll full(flag)",
        "#pragma omp task mergeable(flag)",
        "#pragma omp task untied(flag)",
        "#pragma omp ordered simd(flag)",
        "#pragma omp ordered threads(flag)",
        "#pragma omp assume no_openmp(flag)",
        "#pragma omp assume no_openmp_constructs(flag)",
        "#pragma omp assume no_openmp_routines(flag)",
        "#pragma omp assume no_parallelism(flag)",
    ];

    for source in conditioned {
        assert_rejected_in_5_2(source);
        assert_accepted(OpenMpVersion::V6_0, source);
    }

    let historical_bare = [
        "#pragma omp requires reverse_offload",
        "#pragma omp requires unified_address",
        "#pragma omp requires unified_shared_memory",
        "#pragma omp requires dynamic_allocators",
        "#pragma omp atomic read",
        "#pragma omp atomic update",
        "#pragma omp atomic write",
        "#pragma omp atomic capture",
        "#pragma omp atomic compare",
        "#pragma omp atomic compare weak",
        "#pragma omp atomic acq_rel",
        "#pragma omp atomic acquire",
        "#pragma omp atomic relaxed",
        "#pragma omp atomic release",
        "#pragma omp atomic seq_cst",
        "#pragma omp flush seq_cst",
        "#pragma omp for nowait",
        "#pragma omp taskloop nogroup",
        "#pragma omp declare simd inbranch",
        "#pragma omp declare simd notinbranch",
        "#pragma omp unroll full",
        "#pragma omp task mergeable",
        "#pragma omp task untied",
        "#pragma omp ordered simd",
        "#pragma omp ordered threads",
        "#pragma omp assume no_openmp",
        "#pragma omp assume no_openmp_routines",
        "#pragma omp assume no_parallelism",
    ];

    for source in historical_bare {
        assert_accepted(OpenMpVersion::V5_2, source);
        assert_accepted(OpenMpVersion::V6_0, source);
    }

    assert_accepted(OpenMpVersion::V6_0, "#pragma omp requires self_maps");
    assert_accepted(OpenMpVersion::V6_0, "#pragma omp requires device_safesync");
}

#[test]
fn requirement_and_atomic_conditions_are_preserved_in_typed_payloads() {
    for (name, expected) in [
        ("reverse_offload", RequireModifier::ReverseOffload),
        ("unified_address", RequireModifier::UnifiedAddress),
        (
            "unified_shared_memory",
            RequireModifier::UnifiedSharedMemory,
        ),
        ("dynamic_allocators", RequireModifier::DynamicAllocators),
        ("self_maps", RequireModifier::SelfMaps),
        ("device_safesync", RequireModifier::DeviceSafesync),
    ] {
        let source = format!("#pragma omp requires {name}(required_flag)");
        let ClauseData::Requirement {
            requirement,
            required: Some(expression),
        } = payload(&source)
        else {
            panic!("expected a typed requirement payload for {source:?}");
        };
        assert_eq!(requirement, expected);
        assert_eq!(expression.to_string(), "required_flag");
    }

    assert!(matches!(
        payload("#pragma omp requires reverse_offload(required_flag)"),
        ClauseData::Requirement {
            requirement: RequireModifier::ReverseOffload,
            required: Some(expression),
        } if expression.to_string() == "required_flag"
    ));
    assert!(matches!(
        payload("#pragma omp requires reverse_offload"),
        ClauseData::Requirement {
            requirement: RequireModifier::ReverseOffload,
            required: None,
        }
    ));
    assert!(matches!(
        payload("#pragma omp atomic read(use_it)"),
        ClauseData::AtomicOperation {
            op: AtomicOp::Read,
            use_semantics: Some(expression),
        } if expression.to_string() == "use_it"
    ));
    assert!(matches!(
        payload("#pragma omp atomic read"),
        ClauseData::AtomicOperation {
            op: AtomicOp::Read,
            use_semantics: None,
        }
    ));
    assert!(matches!(
        payload("#pragma omp atomic compare(use_it)"),
        ClauseData::ExtendedAtomic {
            kind: ExtendedAtomicKind::Compare,
            use_semantics: Some(expression),
        } if expression.to_string() == "use_it"
    ));
    assert!(matches!(
        payload("#pragma omp atomic acquire(use_it)"),
        ClauseData::MemoryOrder {
            order: MemoryOrder::Acquire,
            use_semantics: Some(expression),
        } if expression.to_string() == "use_it"
    ));
    assert!(matches!(
        payload("#pragma omp flush seq_cst(use_it)"),
        ClauseData::MemoryOrder {
            order: MemoryOrder::SeqCst,
            use_semantics: Some(expression),
        } if expression.to_string() == "use_it"
    ));
}

#[test]
fn atomic_operation_spellings_share_one_canonical_directive_kind() {
    for source in [
        "#pragma omp atomic read",
        "#pragma omp atomic write",
        "#pragma omp atomic update",
        "#pragma omp atomic capture",
        "#pragma omp atomic compare capture",
    ] {
        assert_eq!(parse(source).directive().kind(), OmpDirectiveKind::Atomic);
    }

    let compound = parse("#pragma omp atomic compare capture");
    assert_eq!(compound.directive().clauses().len(), 2);
    assert!(matches!(
        compound.directive().clauses()[0].payload(),
        ClauseData::ExtendedAtomic {
            kind: ExtendedAtomicKind::Compare,
            ..
        }
    ));
    assert!(matches!(
        compound.directive().clauses()[1].payload(),
        ClauseData::ExtendedAtomic {
            kind: ExtendedAtomicKind::Capture,
            ..
        }
    ));
}

#[test]
fn flush_does_not_reassign_a_memory_order_argument_to_its_directive_list() {
    let source = "#pragma omp flush acq_rel(use_it)";

    let historical = OpenMpConfig::exact(OpenMpVersion::V5_2, c23(), SourceForm::Pragma)
        .expect("valid exact-version configuration")
        .parser()
        .parse(source);
    assert!(
        historical.is_err(),
        "OpenMP 5.2 forbids combining a memory-order clause with a flush list"
    );

    let parsed = parse(source);
    assert!(parsed.directive().parameter().is_none());
    assert!(matches!(
        parsed.directive().clauses()[0].payload(),
        ClauseData::MemoryOrder {
            order: MemoryOrder::AcqRel,
            use_semantics: Some(expression),
        } if expression.to_string() == "use_it"
    ));

    assert!(
        OpenMpConfig::new(c23(), SourceForm::Pragma)
            .expect("valid C configuration")
            .parser()
            .parse("#pragma omp flush(a) acq_rel")
            .is_err(),
        "a flush list and a memory-order clause must conflict"
    );

    let list = parse("#pragma omp flush(a)");
    assert!(matches!(
        list.directive().parameter(),
        Some(OmpDirectiveParameter::FlushList(items)) if items.len() == 1
    ));
    assert!(list.directive().clauses().is_empty());
}

#[test]
fn every_optional_argument_family_has_a_canonical_option_field() {
    assert!(matches!(
        payload("#pragma omp for nowait(skip_barrier)"),
        ClauseData::Nowait {
            do_not_synchronize: Some(expression),
        } if expression.to_string() == "skip_barrier"
    ));
    assert!(matches!(
        payload("#pragma omp taskloop nogroup(skip_group)"),
        ClauseData::Nogroup {
            do_not_synchronize: Some(expression),
        } if expression.to_string() == "skip_group"
    ));
    assert!(matches!(
        payload("#pragma omp declare simd inbranch(branch_only)"),
        ClauseData::Branch {
            condition: Some(expression),
            ..
        } if expression.to_string() == "branch_only"
    ));
    assert!(matches!(
        payload("#pragma omp unroll full(all_iterations)"),
        ClauseData::Full {
            fully_unroll: Some(expression),
        } if expression.to_string() == "all_iterations"
    ));
    assert!(matches!(
        payload("#pragma omp unroll partial(8)"),
        ClauseData::Partial {
            unroll_factor: Some(expression),
        } if expression.to_string() == "8"
    ));
    assert!(matches!(
        payload("#pragma omp task mergeable(may_merge)"),
        ClauseData::Mergeable {
            can_merge: Some(expression),
        } if expression.to_string() == "may_merge"
    ));
    assert!(matches!(
        payload("#pragma omp task untied(may_move)"),
        ClauseData::Untied {
            can_change_threads: Some(expression),
        } if expression.to_string() == "may_move"
    ));
    assert!(matches!(
        payload("#pragma omp ordered simd(use_lanes)"),
        ClauseData::Simd {
            apply_to_simd: Some(expression),
        } if expression.to_string() == "use_lanes"
    ));
    assert!(matches!(
        payload("#pragma omp ordered threads(use_threads)"),
        ClauseData::Threads {
            apply_to_threads: Some(expression),
        } if expression.to_string() == "use_threads"
    ));
    assert!(matches!(
        payload("#pragma omp assume no_parallelism(trust_me)"),
        ClauseData::Assumption {
            can_assume: Some(expression),
            ..
        } if expression.to_string() == "trust_me"
    ));
    assert!(matches!(
        payload("#pragma omp declare target indirect(via_pointer)"),
        ClauseData::Indirect {
            invoked_by_fptr: Some(expression),
        } if expression.to_string() == "via_pointer"
    ));
    assert!(matches!(
        payload("#pragma omp task replayable(replay_it)"),
        ClauseData::Replayable {
            replayable_expression: Some(expression),
        } if expression.to_string() == "replay_it"
    ));
    assert!(matches!(
        payload("#pragma omp parallel safesync(4)"),
        ClauseData::Safesync {
            width: Some(expression),
        } if expression.to_string() == "4"
    ));
    assert!(matches!(
        payload("#pragma omp task transparent(omp_import)"),
        ClauseData::Transparent {
            impex_type: Some(expression),
        } if expression.to_string() == "omp_import"
    ));
}

#[test]
fn optional_arguments_that_predate_60_remain_available() {
    assert_accepted(
        OpenMpVersion::V5_2,
        "#pragma omp declare target indirect(via_pointer)",
    );
    assert_accepted(OpenMpVersion::V5_2, "#pragma omp unroll partial(4)");
}

#[test]
fn empty_optional_arguments_are_hard_errors() {
    for source in [
        "#pragma omp requires reverse_offload()",
        "#pragma omp atomic read()",
        "#pragma omp atomic seq_cst()",
        "#pragma omp for nowait()",
        "#pragma omp taskloop nogroup()",
        "#pragma omp declare simd inbranch()",
        "#pragma omp unroll full()",
        "#pragma omp task mergeable()",
        "#pragma omp task untied()",
        "#pragma omp ordered simd()",
        "#pragma omp ordered threads()",
        "#pragma omp assume no_parallelism()",
        "#pragma omp declare target indirect()",
        "#pragma omp task replayable()",
        "#pragma omp parallel safesync()",
        "#pragma omp task transparent()",
    ] {
        assert!(
            OpenMpConfig::new(c23(), SourceForm::Pragma)
                .expect("valid C configuration")
                .parser()
                .parse(source)
                .is_err(),
            "unexpectedly accepted {source:?}"
        );
    }
}
