use roup::ir::{convert_directive_language, Language};

#[test]
fn converts_c_to_fortran() {
    let input = "#pragma omp parallel for private(i, j)";
    let output = convert_directive_language(input, Language::C, Language::Fortran)
        .expect("conversion should succeed");
    assert_eq!(output, "!$omp parallel do private(i, j)");
}

#[test]
fn converts_fortran_to_c() {
    let input = "!$OMP TARGET TEAMS DISTRIBUTE PARALLEL DO";
    let output = convert_directive_language(input, Language::Fortran, Language::C)
        .expect("conversion should succeed");
    assert_eq!(output, "#pragma omp target teams distribute parallel for");
}

#[test]
fn rejects_invalid_directives() {
    let err = convert_directive_language("not a pragma", Language::C, Language::Fortran)
        .expect_err("conversion should fail");
    assert!(err.to_string().contains("failed to parse"));
}
