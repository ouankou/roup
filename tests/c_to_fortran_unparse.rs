//! Integration tests for cross-language unparsing.
//!
//! These tests exercise the new ability to parse C/C++ OpenMP directives and
//! emit Fortran output. The functionality is required for downstream projects
//! like DataRaceBench that ship C benchmarks but need Fortran equivalents.

use roup::ir::{convert::convert_directive, Language, ParserConfig, SourceLocation};
use roup::parser::parse_omp_directive;

fn convert_to_fortran(input: &str) -> String {
    let (_, directive) = parse_omp_directive(input).expect("parsing should succeed");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("conversion should succeed");

    ir.to_string_in_language(Language::Fortran)
}

#[test]
fn converts_parallel_for_to_fortran_do() {
    let input = "#pragma omp parallel for private(i) schedule(static, 4)";
    let output = convert_to_fortran(input);

    assert_eq!(
        output, "!$omp parallel do private(i) schedule(static, 4)",
        "parallel for should become parallel do"
    );
}

#[test]
fn converts_for_with_nowait_clause() {
    let input = "#pragma omp for nowait";
    let output = convert_to_fortran(input);

    assert_eq!(output, "!$omp do nowait");
}

#[test]
fn converts_target_teams_distribute_parallel_for_simd() {
    let input = "#pragma omp target teams distribute parallel for simd collapse(2)";
    let output = convert_to_fortran(input);

    assert_eq!(
        output, "!$omp target teams distribute parallel do simd collapse(2)",
        "combined constructs should use Fortran naming"
    );
}

#[test]
fn converts_distribute_parallel_for() {
    let input = "#pragma omp distribute parallel for";
    let output = convert_to_fortran(input);

    assert_eq!(output, "!$omp distribute parallel do");
}
