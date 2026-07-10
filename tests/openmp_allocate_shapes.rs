use roup::api::{OpenMpConfig, ParsedOpenMpDirective};
use roup::diagnostic::{Diagnostic, DiagnosticCode};
use roup::ir::{AllocateSourceSyntax, ClauseData};
use roup::version::{CStandard, CppStandard, HostLanguageProfile, OpenMpVersion, SourceForm};

fn parse(
    version: OpenMpVersion,
    profile: HostLanguageProfile,
    source: &str,
) -> Result<ParsedOpenMpDirective, Diagnostic> {
    OpenMpConfig::exact(version, profile, SourceForm::Pragma)
        .expect("valid parser configuration")
        .parser()
        .parse(source)
}

fn parse_c(version: OpenMpVersion, source: &str) -> Result<ParsedOpenMpDirective, Diagnostic> {
    parse(version, HostLanguageProfile::C(CStandard::C23), source)
}

#[test]
fn allocate_cumulative_grammars_have_one_typed_semantic_shape() {
    let unmodified = parse_c(
        OpenMpVersion::V5_0,
        "#pragma omp parallel private(first, second) allocate(first, second)",
    )
    .expect("unmodified OpenMP 5.0 allocate clause");
    assert!(matches!(
        unmodified.directive().clauses()[1].payload(),
        ClauseData::Allocate {
            allocator: None,
            alignment: None,
            items,
            source_syntax: AllocateSourceSyntax::Unmodified,
        } if items.len() == 2
    ));

    let historical_source =
        "#pragma omp parallel private(first, second) allocate(flag ? first_allocator : second_allocator: first, second)";
    for version in [OpenMpVersion::V5_0, OpenMpVersion::V6_0] {
        let historical = parse_c(version, historical_source)
            .unwrap_or_else(|error| panic!("historical allocate rejected in {version}: {error}"));
        assert!(matches!(
            historical.directive().clauses()[1].payload(),
            ClauseData::Allocate {
                allocator: Some(allocator),
                alignment: None,
                items,
                source_syntax: AllocateSourceSyntax::SimpleAllocator,
            } if allocator.to_string() == "flag ? first_allocator : second_allocator"
                && items.len() == 2
        ));
    }

    let modern_source =
        "#pragma omp parallel private(first, second) allocate(align(64), allocator(make_allocator(device)): first, second)";
    let error = parse_c(OpenMpVersion::V5_0, modern_source)
        .expect_err("complex allocate modifiers did not exist in OpenMP 5.0");
    assert_eq!(
        error.code(),
        DiagnosticCode::NotAvailableInVersion,
        "unexpected diagnostic: {error}"
    );
    for version in [OpenMpVersion::V5_1, OpenMpVersion::V6_0] {
        let modern = parse_c(version, modern_source)
            .unwrap_or_else(|error| panic!("modern allocate rejected in {version}: {error}"));
        assert!(matches!(
            modern.directive().clauses()[1].payload(),
            ClauseData::Allocate {
                allocator: Some(allocator),
                alignment: Some(alignment),
                items,
                source_syntax: AllocateSourceSyntax::Modifiers,
            } if allocator.to_string() == "make_allocator(device)"
                && alignment.to_string() == "64"
                && items.len() == 2
        ));
    }
}

#[test]
fn allocator_clause_is_an_open_typed_expression() {
    let conditional = parse_c(
        OpenMpVersion::V5_1,
        "#pragma omp allocate(storage) allocator(flag ? first_allocator : second_allocator)",
    )
    .expect("conditional allocator expression");
    assert!(matches!(
        conditional.directive().clauses()[0].payload(),
        ClauseData::Allocator { allocator }
            if allocator.to_string() == "flag ? first_allocator : second_allocator"
    ));

    let comma = parse_c(
        OpenMpVersion::V5_1,
        "#pragma omp allocate(storage) allocator((first_allocator, second_allocator))",
    )
    .expect("explicitly parenthesized comma expression");
    let ClauseData::Allocator { allocator } = comma.directive().clauses()[0].payload() else {
        panic!("expected allocator payload");
    };
    assert_eq!(
        allocator.to_string(),
        "(first_allocator , second_allocator)"
    );

    let cpp = parse(
        OpenMpVersion::V6_0,
        HostLanguageProfile::Cpp(CppStandard::Cpp23),
        "#pragma omp allocate(storage) allocator(::runtime::select_allocator(device))",
    )
    .expect("qualified C++ allocator expression");
    assert!(matches!(
        cpp.directive().clauses()[0].payload(),
        ClauseData::Allocator { allocator }
            if allocator.to_string() == "::runtime::select_allocator(device)"
    ));
}

#[test]
fn allocate_modifiers_are_unique_exclusive_and_complete() {
    for source in [
        "#pragma omp parallel private(x) allocate(: x)",
        "#pragma omp parallel private(x) allocate(allocator(): x)",
        "#pragma omp parallel private(x) allocate(align(): x)",
        "#pragma omp parallel private(x) allocate(allocator(a), allocator(b): x)",
        "#pragma omp parallel private(x) allocate(align(8), align(16): x)",
        "#pragma omp parallel private(x) allocate(a, allocator(b): x)",
        "#pragma omp parallel private(x) allocate(unknown(a), align(8): x)",
        "#pragma omp parallel private(x) allocate(allocator(a),: x)",
        "#pragma omp parallel private(x) allocate(allocator(a) trailing: x)",
        "#pragma omp parallel private(x) allocate(allocator(a), align(8) trailing: x)",
        "#pragma omp parallel private(x) allocate(allocator(a):)",
    ] {
        assert!(
            parse_c(OpenMpVersion::V6_0, source).is_err(),
            "malformed allocate grammar unexpectedly parsed: {source}"
        );
    }

    parse_c(
        OpenMpVersion::V5_0,
        "#pragma omp parallel private(x) allocate(unknown(a): x)",
    )
    .expect("an arbitrary call expression remains valid in the 5.0 allocator slot");
}

#[test]
fn allocate_alignment_and_scalar_expression_errors_are_immediate() {
    for source in [
        "#pragma omp parallel private(x) allocate(align(0): x)",
        "#pragma omp parallel private(x) allocate(align(3): x)",
        "#pragma omp parallel private(x) allocate(align(-8): x)",
        "#pragma omp parallel private(x) allocate(align(1.5): x)",
        "#pragma omp parallel private(x) allocate(align(\"eight\"): x)",
        "#pragma omp allocate(storage) allocator(first, second)",
    ] {
        assert!(
            parse_c(OpenMpVersion::V6_0, source).is_err(),
            "invalid allocation expression unexpectedly parsed: {source}"
        );
    }
}
