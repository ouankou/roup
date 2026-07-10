use roup::api::{OpenMpConfig, ParsedOpenMpDirective};
use roup::ast::{
    OmpClauseKind, OmpDirectiveKind, OmpDirectiveParameter, OmpFlushListItem, OmpMapperId,
    OmpReductionIdentifier,
};
use roup::diagnostic::{Diagnostic, DiagnosticCode};
use roup::host::TokenKind;
use roup::ir::{ClauseData, LinearModifier, MapModifier, MapType, MemoryOrder};
use roup::version::{CStandard, FortranStandard, HostLanguageProfile, OpenMpVersion, SourceForm};

fn c(source: &str) -> Result<ParsedOpenMpDirective, Diagnostic> {
    OpenMpConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .expect("valid C profile")
        .parser()
        .parse(source)
}

fn c_v6(source: &str) -> Result<ParsedOpenMpDirective, Diagnostic> {
    c_exact(OpenMpVersion::V6_0, source)
}

fn c_exact(version: OpenMpVersion, source: &str) -> Result<ParsedOpenMpDirective, Diagnostic> {
    OpenMpConfig::exact(
        version,
        HostLanguageProfile::C(CStandard::C23),
        SourceForm::Pragma,
    )
    .expect("valid C profile")
    .parser()
    .parse(source)
}

fn fortran(source: &str) -> Result<ParsedOpenMpDirective, Diagnostic> {
    OpenMpConfig::new(
        HostLanguageProfile::Fortran(FortranStandard::Fortran2023),
        SourceForm::FortranFree,
    )
    .expect("valid Fortran profile")
    .parser()
    .parse(source)
}

fn fortran_v6(source: &str) -> Result<ParsedOpenMpDirective, Diagnostic> {
    OpenMpConfig::exact(
        OpenMpVersion::V6_0,
        HostLanguageProfile::Fortran(FortranStandard::Fortran2023),
        SourceForm::FortranFree,
    )
    .expect("valid Fortran profile")
    .parser()
    .parse(source)
}

#[test]
fn declare_mapper_has_a_typed_declarator_and_map_clauses() {
    let parsed = c("#pragma omp declare mapper(myvec_t v) map(v, v.data[0:v.len])")
        .expect("standard declare mapper must parse");
    assert_eq!(parsed.directive().kind(), OmpDirectiveKind::DeclareMapper);
    assert_eq!(parsed.directive().clauses()[0].kind(), OmpClauseKind::Map);

    let Some(OmpDirectiveParameter::DeclareMapper(mapper)) = parsed.directive().parameter() else {
        panic!("declare mapper must retain its typed signature");
    };
    assert!(mapper.identifier().is_none());
    assert_eq!(mapper.variable().as_str(), "v");

    let parsed = c("#pragma omp declare mapper(short * a) map(to: a)")
        .expect("spaced pointer declarator must parse");
    let Some(OmpDirectiveParameter::DeclareMapper(mapper)) = parsed.directive().parameter() else {
        panic!("declare mapper must retain its typed signature");
    };
    assert_eq!(mapper.variable().as_str(), "a");
    assert!(matches!(
        mapper.type_name().tokens().last(),
        Some(TokenKind::Star)
    ));

    let parsed = c("#pragma omp declare mapper(default: const int *a) map(a)")
        .expect("adjacent pointer declarator must parse");
    let Some(OmpDirectiveParameter::DeclareMapper(mapper)) = parsed.directive().parameter() else {
        panic!("declare mapper must retain its typed signature");
    };
    assert!(matches!(mapper.identifier(), Some(OmpMapperId::Default)));
    assert_eq!(mapper.variable().as_str(), "a");
    assert!(matches!(
        mapper.type_name().tokens().last(),
        Some(TokenKind::Star)
    ));
}

#[test]
fn malformed_mapper_declarators_are_hard_errors() {
    for source in [
        "#pragma omp declare mapper(const int *) map(a)",
        "#pragma omp declare mapper(*a) map(a)",
        "#pragma omp declare mapper(const int a[4]) map(a)",
        "#pragma omp declare mapper(default: const int *a)",
        "#pragma omp declare mapper(short *a) map(to: w, e, r)",
    ] {
        assert!(c(source).is_err(), "unexpectedly accepted {source:?}");
    }
}

#[test]
fn removed_declare_target_list_remains_valid_in_openmp_6() {
    let parsed = c_v6("#pragma omp declare target (x, y, z)")
        .expect("a standardized historical form stays accepted");
    assert_eq!(parsed.directive().kind(), OmpDirectiveKind::DeclareTarget);
    let Some(OmpDirectiveParameter::DeclareTargetList(items)) = parsed.directive().parameter()
    else {
        panic!("historical extended list must remain typed");
    };
    assert_eq!(
        items.iter().map(ToString::to_string).collect::<Vec<_>>(),
        ["x", "y", "z"]
    );

    let parsed = fortran_v6("!$omp declare target(s, t, f)")
        .expect("Fortran historical extended list stays accepted");
    let Some(OmpDirectiveParameter::DeclareTargetList(items)) = parsed.directive().parameter()
    else {
        panic!("Fortran extended list must remain typed");
    };
    assert_eq!(
        items.iter().map(ToString::to_string).collect::<Vec<_>>(),
        ["s", "t", "f"]
    );
}

#[test]
fn fortran_end_directives_accept_only_their_standard_end_clauses() {
    let critical =
        fortran("!$omp end critical(test3)").expect("named end critical must retain its name");
    let Some(OmpDirectiveParameter::CriticalSection(name)) = critical.directive().parameter()
    else {
        panic!("end critical must carry a typed critical name");
    };
    assert_eq!(name.as_str(), "test3");
    fortran("!$omp endcritical(test3)").expect("standard compact Fortran alias must canonicalize");

    for source in [
        "!$omp end do nowait",
        "!$omp end do simd nowait",
        "!$omp end sections nowait",
        "!$omp end single nowait",
        "!$omp end workshare nowait",
        "!$omp end scope nowait",
        "!$omp end single copyprivate(to, b, c)",
    ] {
        fortran(source).unwrap_or_else(|error| panic!("rejected {source:?}: {error}"));
    }

    let error = fortran("!$omp end single copyprivate(a) nowait")
        .expect_err("copyprivate and nowait conflict");
    assert_eq!(error.code(), DiagnosticCode::ConflictingClauses);
}

#[test]
fn loop_constituents_expose_their_standard_clause_sets() {
    for source in [
        "#pragma omp loop private(a) lastprivate(b) reduction(+: sum) bind(thread)",
        "#pragma omp simd private(a) lastprivate(b)",
    ] {
        c(source).unwrap_or_else(|error| panic!("rejected {source:?}: {error}"));
    }
    for source in [
        "#pragma omp parallel loop bind(parallel)",
        "#pragma omp teams loop bind(teams)",
    ] {
        c_v6(source).unwrap_or_else(|error| panic!("rejected {source:?}: {error}"));
    }
}

#[test]
fn historical_master_taskloop_inherits_taskloop_clauses_in_openmp_6() {
    for source in [
        "#pragma omp master taskloop if(taskloop: ready) shared(s) private(i) firstprivate(seed) lastprivate(result) default(shared) grainsize(3) collapse(2) final(done) priority(4) untied mergeable nogroup allocate(i)",
        "#pragma omp master taskloop num_tasks(4) reduction(+: sum) in_reduction(+: total)",
        "#pragma omp master taskloop simd safelen(4) simdlen(4) linear(i: 1) nontemporal(a) order(concurrent)",
    ] {
        c_v6(source).unwrap_or_else(|error| panic!("rejected {source:?}: {error}"));
    }
}

#[test]
fn openmp_52_removed_clause_syntax_remains_typed_in_openmp_6() {
    let linear_source = "#pragma omp declare simd linear(val(x): 2)";
    c_exact(OpenMpVersion::V5_2, linear_source)
        .expect("the historical linear syntax remains valid in OpenMP 5.2");
    let linear = c_v6(linear_source)
        .expect("the historical linear-modifier(list) form remains standardized input");
    assert!(matches!(
        linear.directive().clauses()[0].payload(),
        ClauseData::Linear {
            modifier: Some(LinearModifier::Val),
            items,
            step: Some(_),
            ..
        } if items.len() == 1
    ));

    let reduction_source = "#pragma omp parallel reduction(-: total)";
    c_exact(OpenMpVersion::V5_2, reduction_source)
        .expect("the subtraction reduction remains valid in OpenMP 5.2");
    let reduction = c_v6(reduction_source)
        .expect("the historical subtraction reduction remains standardized input");
    assert!(matches!(
        reduction.directive().clauses()[0].payload(),
        ClauseData::Reduction {
            operator: OmpReductionIdentifier::Subtract,
            ..
        }
    ));

    let historical_map = "#pragma omp target map(always close to: value)";
    c_exact(OpenMpVersion::V5_2, historical_map)
        .expect("map modifiers without separators remain valid in OpenMP 5.2");
    let historical = c_v6(historical_map)
        .expect("map modifiers without comma separators remain standardized input");
    let canonical = c_v6("#pragma omp target map(always, close, to: value)")
        .expect("the replacement map spelling must parse");
    assert_eq!(
        historical.directive().clauses()[0].payload(),
        canonical.directive().clauses()[0].payload(),
    );
    assert!(matches!(
        historical.directive().clauses()[0].payload(),
        ClauseData::Map {
            map_type: Some(MapType::To),
            modifiers,
            ..
        } if modifiers == &[MapModifier::Always, MapModifier::Close]
    ));

    let mapper = c_v6("#pragma omp target map(always mapper(custom) to: value)")
        .expect("historical separators apply to complex map modifiers too");
    assert!(matches!(
        mapper.directive().clauses()[0].payload(),
        ClauseData::Map {
            map_type: Some(MapType::To),
            mapper: Some(OmpMapperId::User(identifier)),
            ..
        } if identifier.as_str() == "custom"
    ));

    for source in [
        "#pragma omp target uses_allocators(my_alloc(my_traits))",
        "#pragma omp metadirective default(parallel)",
        "#pragma omp declare target",
        "#pragma omp declare target to(value)",
        "#pragma omp depobj(handle) destroy",
        "#pragma omp ordered depend(source)",
        "#pragma omp ordered depend(sink: i - 1)",
        "#pragma omp declare variant(fast) match(implementation={requires(unified_address)})",
    ] {
        for version in [OpenMpVersion::V5_2, OpenMpVersion::V6_0] {
            c_exact(version, source).unwrap_or_else(|error| {
                panic!(
                    "OpenMP {version} rejected standardized historical syntax {source:?}: {error}"
                )
            });
        }
    }
}

#[test]
fn deprecated_syntax_remains_accepted_in_every_later_c_specification() {
    let c_versions = [
        OpenMpVersion::V1_0,
        // OpenMP 1.1 was a Fortran-only specification, so there is no C
        // language profile at that exact version to test.
        OpenMpVersion::V2_0,
        OpenMpVersion::V2_5,
        OpenMpVersion::V3_0,
        OpenMpVersion::V3_1,
        OpenMpVersion::V4_0,
        OpenMpVersion::V4_5,
        OpenMpVersion::V5_0,
        OpenMpVersion::V5_1,
        OpenMpVersion::V5_2,
        OpenMpVersion::V6_0,
    ];
    let cases = [
        (OpenMpVersion::V1_0, "#pragma omp master"),
        (
            OpenMpVersion::V1_0,
            "#pragma omp parallel reduction(-: total)",
        ),
        (
            OpenMpVersion::V4_0,
            "#pragma omp parallel proc_bind(master)",
        ),
        (OpenMpVersion::V4_0, "#pragma omp declare target to(value)"),
        (OpenMpVersion::V4_0, "#pragma omp declare target"),
        (
            OpenMpVersion::V4_5,
            "#pragma omp declare simd linear(val(x): 2)",
        ),
        (OpenMpVersion::V4_5, "#pragma omp ordered depend(source)"),
        (
            OpenMpVersion::V4_5,
            "#pragma omp ordered depend(sink: i - 1)",
        ),
        (
            OpenMpVersion::V5_0,
            "#pragma omp target map(always close to: value)",
        ),
        (
            OpenMpVersion::V5_0,
            "#pragma omp target uses_allocators(my_alloc(my_traits))",
        ),
        (
            OpenMpVersion::V5_0,
            "#pragma omp metadirective default(parallel)",
        ),
        (OpenMpVersion::V5_0, "#pragma omp depobj(handle) destroy"),
        (
            OpenMpVersion::V5_0,
            "#pragma omp declare variant(fast) match(implementation={unified_address})",
        ),
        (
            OpenMpVersion::V5_1,
            "#pragma omp declare variant(fast) match(implementation={requires(unified_address)})",
        ),
        (OpenMpVersion::V5_0, "#pragma omp master taskloop"),
        (OpenMpVersion::V5_0, "#pragma omp master taskloop simd"),
        (OpenMpVersion::V5_0, "#pragma omp parallel master"),
        (OpenMpVersion::V5_0, "#pragma omp parallel master taskloop"),
        (
            OpenMpVersion::V5_0,
            "#pragma omp parallel master taskloop simd",
        ),
    ];

    for (introduced, source) in cases {
        for version in c_versions
            .iter()
            .copied()
            .filter(|version| *version >= introduced)
        {
            c_exact(version, source).unwrap_or_else(|error| {
                panic!("OpenMP {version} rejected cumulative syntax {source:?}: {error}")
            });
        }
    }
}

#[test]
fn removed_fortran_master_constructs_remain_accepted_in_openmp_6() {
    for source in [
        "!$omp master",
        "!$omp end master",
        "!$omp master taskloop",
        "!$omp end master taskloop",
        "!$omp master taskloop simd",
        "!$omp end master taskloop simd",
        "!$omp parallel master",
        "!$omp end parallel master",
        "!$omp parallel master taskloop",
        "!$omp end parallel master taskloop",
        "!$omp parallel master taskloop simd",
        "!$omp end parallel master taskloop simd",
        "!$omp parallel proc_bind(master)",
    ] {
        fortran_v6(source).unwrap_or_else(|error| {
            panic!("OpenMP 6.0 rejected historical Fortran syntax {source:?}: {error}")
        });
    }
}

#[test]
fn flush_memory_order_arguments_follow_exact_version_rules() {
    let exact = |version, source| {
        OpenMpConfig::exact(
            version,
            HostLanguageProfile::C(CStandard::C23),
            SourceForm::Pragma,
        )
        .expect("valid C profile")
        .parser()
        .parse(source)
    };

    for version in [
        OpenMpVersion::V5_0,
        OpenMpVersion::V5_1,
        OpenMpVersion::V5_2,
    ] {
        for source in [
            "#pragma omp flush acq_rel(one)",
            "#pragma omp flush acq_rel(one, two)",
        ] {
            assert!(
                exact(version, source).is_err(),
                "{version} accepted the forbidden memory-order plus flush-list form {source:?}"
            );
        }
        exact(version, "#pragma omp flush(one, two)")
            .expect("a historical flush list without memory order remains valid");
        exact(version, "#pragma omp flush release")
            .expect("a bare historical memory-order clause remains valid");
    }

    fortran_v6("!$OMP FLUSH RELEASE(USE_ORDER)")
        .expect("Fortran memory-order keywords remain case-insensitive");
    assert!(
        c_v6("#pragma omp flush ACQ_REL(use_order)").is_err(),
        "C memory-order keywords remain case-sensitive"
    );

    let current = c_v6("#pragma omp flush acq_rel(use_it)")
        .expect("OpenMP 6.0 resolves one expression as use_semantics");
    assert!(current.directive().parameter().is_none());
    assert!(matches!(
        current.directive().clauses()[0].payload(),
        ClauseData::MemoryOrder {
            order: MemoryOrder::AcqRel,
            use_semantics: Some(_),
        }
    ));
    let union = c("#pragma omp flush release(use_it)")
        .expect("union-version parsing accepts the OpenMP 6.0 argument");
    assert!(union.directive().parameter().is_none());
    assert!(matches!(
        union.directive().clauses()[0].payload(),
        ClauseData::MemoryOrder {
            order: MemoryOrder::Release,
            use_semantics: Some(_),
        }
    ));

    for source in [
        "#pragma omp flush(one, two) acq_rel",
        "!$omp flush(one, two) release",
    ] {
        let result = if source.starts_with("#pragma") {
            c_v6(source)
        } else {
            fortran_v6(source)
        };
        assert!(result.is_err(), "unexpectedly reordered {source:?}");
    }
}

#[test]
fn flush_lists_preserve_typed_designators_and_fortran_common_blocks() {
    let current_c = c_v6("#pragma omp flush(array[1:length], object.member)")
        .expect("C flush lists accept variable designators");
    let Some(OmpDirectiveParameter::FlushList(items)) = current_c.directive().parameter() else {
        panic!("C flush list must remain typed");
    };
    assert!(matches!(
        items.as_slice(),
        [OmpFlushListItem::Variable(array), OmpFlushListItem::Variable(member)]
            if array.to_string() == "array[1:length]"
                && member.to_string() == "object.member"
    ));

    let source = "!$omp flush(A(1:N), OBJ%FIELD, /BLOCK/)";
    let parsed = fortran_v6(source)
        .unwrap_or_else(|error| panic!("rejected typed Fortran flush list {source:?}: {error}"));
    let Some(OmpDirectiveParameter::FlushList(items)) = parsed.directive().parameter() else {
        panic!("Fortran flush list must remain typed for {source:?}");
    };
    assert!(
        matches!(
            items.as_slice(),
            [
                OmpFlushListItem::Variable(array),
                OmpFlushListItem::Variable(member),
                OmpFlushListItem::FortranCommonBlock(block),
            ] if array.dimensions() == 1
                && array.root_identifier().is_some_and(|name| name.as_str() == "a")
                && member.root_identifier().is_some_and(|name| name.as_str() == "obj")
                && block.as_str() == "block"
        ),
        "unexpected Fortran flush items: {items:#?}"
    );
}

#[test]
fn malformed_flush_items_are_hard_errors() {
    for source in [
        "#pragma omp flush(value + offset)",
        "#pragma omp flush(/block/)",
        "#pragma omp flush acq_rel(value, /block/)",
        "!$omp flush(//)",
        "!$omp flush(/BLOCK)",
        "!$omp flush(/FIRST/SECOND/)",
    ] {
        let result = if source.starts_with("#pragma") {
            c_v6(source)
        } else {
            fortran_v6(source)
        };
        assert!(result.is_err(), "unexpectedly accepted {source:?}");
    }
}

#[test]
fn historical_map_separators_do_not_relax_malformed_headers() {
    for source in [
        "#pragma omp target map(always,,to: value)",
        "#pragma omp target map(always mapper() to: value)",
        "#pragma omp target map(always always to: value)",
        "#pragma omp target map(mapper(one) mapper(two) to: value)",
        "#pragma omp target map(alwaysclose to: value)",
        "#pragma omp target map(always, to, : value)",
    ] {
        assert!(c_v6(source).is_err(), "unexpectedly accepted {source:?}");
    }
}
