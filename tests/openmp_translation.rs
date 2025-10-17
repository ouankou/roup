//! Integration tests for translating C pragmas into Fortran sentinels
//!
//! The goal is to ensure language-aware rendering preserves semantic
//! information (directive kinds and clauses) while adapting syntax to the
//! Fortran sentinel form required by legacy toolchains.

use roup::ir::{
    translate::{translate_c_to_fortran, translate_c_to_fortran_ir, TranslationError},
    Language, ParserConfig,
};

#[test]
fn translates_parallel_for_to_parallel_do() {
    let output = translate_c_to_fortran("#pragma omp parallel for private(i) schedule(static, 4)")
        .expect("translation should succeed");

    assert_eq!(output, "!$omp parallel do private(i) schedule(static, 4)");
}

#[test]
fn translates_for_loop_to_do_nowait() {
    let output =
        translate_c_to_fortran("#pragma omp for nowait").expect("translation should succeed");

    assert_eq!(output, "!$omp do nowait");
}

#[test]
fn translates_target_teams_distribute_parallel_for_simd() {
    let output =
        translate_c_to_fortran("#pragma omp target teams distribute parallel for simd collapse(2)")
            .expect("translation should succeed");

    assert_eq!(
        output,
        "!$omp target teams distribute parallel do simd collapse(2)"
    );
}

#[test]
fn translate_ir_variant_sets_language() {
    let config = ParserConfig::with_parsing(Language::C);
    let ir = translate_c_to_fortran_ir("#pragma omp parallel for", config)
        .expect("translation should succeed");

    assert!(ir.language().is_fortran());
    assert_eq!(ir.to_string(), "!$omp parallel do");
}

#[test]
fn rejects_empty_input() {
    let err = translate_c_to_fortran("").expect_err("empty input should error");
    assert!(matches!(err, TranslationError::EmptyInput));
}
