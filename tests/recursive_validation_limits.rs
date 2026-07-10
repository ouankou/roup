use roup::api::{OpenMpConfig, OpenMpParser};
use roup::diagnostic::DiagnosticCode;
use roup::ir::MAX_STRUCTURAL_NESTING_DEPTH;
use roup::version::{CStandard, HostLanguageProfile, SourceForm};

fn parser() -> OpenMpParser {
    OpenMpConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .expect("valid C configuration")
        .parser()
}

fn nested_metadirective(depth: usize) -> String {
    let mut body = "parallel".to_string();
    for _ in 0..depth {
        body = format!("metadirective when(device={{kind(cpu)}}: {body})");
    }
    format!("#pragma omp {body}")
}

fn nested_apply(depth: usize) -> String {
    let mut transform = "reverse".to_string();
    for _ in 0..depth {
        transform = format!("unroll partial(2) apply({transform})");
    }
    format!("#pragma omp tile sizes(1) apply(grid: {transform})")
}

fn nested_initializer(depth: usize) -> String {
    let value = format!("{}1{}", "{".repeat(depth), "}".repeat(depth));
    format!(
        "#pragma omp declare reduction(+: int : omp_out += omp_in) initializer(omp_priv = {value})"
    )
}

#[test]
fn nested_directive_variants_receive_full_legality_validation() {
    for source in [
        "#pragma omp metadirective when(device={kind(cpu)}: parallel schedule(static))",
        "#pragma omp metadirective when(device={kind(cpu)}: parallel default(shared) default(none))",
        "#pragma omp metadirective when(device={kind(cpu)}: target enter data)",
    ] {
        assert!(
            parser().parse(source).is_err(),
            "invalid nested directive unexpectedly parsed: {source}"
        );
    }

    parser()
        .parse(
            "#pragma omp metadirective when(device={kind(cpu)}: parallel default(shared) num_threads(4))",
        )
        .expect("a legal nested directive must remain valid");
}

#[test]
fn construct_selector_properties_receive_clause_validation() {
    let invalid =
        "#pragma omp metadirective when(construct={simd(simdlen(4) simdlen(8))}: parallel)";
    assert_eq!(
        parser()
            .parse(invalid)
            .expect_err("duplicate nested simd property must fail")
            .code(),
        DiagnosticCode::DuplicateClause
    );

    parser()
        .parse("#pragma omp metadirective when(construct={simd(simdlen(4))}: parallel)")
        .expect("one valid simd property must remain accepted");
}

#[test]
fn metadirective_nesting_limit_is_a_hard_error() {
    let limit = usize::from(MAX_STRUCTURAL_NESTING_DEPTH);
    parser()
        .parse(&nested_metadirective(limit))
        .expect("the documented nesting limit must parse");
    let error = parser()
        .parse(&nested_metadirective(limit + 1))
        .expect_err("one level beyond the nesting limit must fail");
    assert!(error.message().contains("nesting limit exceeded"));
}

#[test]
fn applied_directive_nesting_limit_is_a_hard_error() {
    let limit = usize::from(MAX_STRUCTURAL_NESTING_DEPTH);
    parser()
        .parse(&nested_apply(limit - 1))
        .expect("the documented nesting limit must parse");
    let error = parser()
        .parse(&nested_apply(limit))
        .expect_err("one level beyond the nesting limit must fail");
    assert!(error.message().contains("nesting limit exceeded"));
}

#[test]
fn braced_initializer_nesting_limit_is_a_hard_error() {
    let limit = usize::from(MAX_STRUCTURAL_NESTING_DEPTH);
    parser()
        .parse(&nested_initializer(limit))
        .expect("the documented nesting limit must parse");
    let error = parser()
        .parse(&nested_initializer(limit + 1))
        .expect_err("one level beyond the nesting limit must fail");
    assert!(error.message().contains("nesting limit exceeded"));
}
