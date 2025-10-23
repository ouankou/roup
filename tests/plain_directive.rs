use roup::ir::{convert::convert_directive, Language, ParserConfig, SourceLocation};
use roup::lexer::Language as ParserLanguage;
use roup::parser::{openmp, parse_omp_directive};

#[test]
fn plain_string_redacts_map_items() {
    let input = "#pragma omp target data map(to: arr[0:N], ptr) nowait";
    let (_, directive) = parse_omp_directive(input).expect("directive should parse");

    let mut config = ParserConfig::default();
    config.language = Language::C;

    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("conversion to IR should succeed");

    assert_eq!(
        ir.to_plain_string(),
        "#pragma omp target data map(to: <identifier>, <identifier>) nowait"
    );
}

#[test]
fn plain_string_fortran_prefix() {
    let parser = openmp::parser().with_language(ParserLanguage::FortranFree);
    let (_, directive) = parser
        .parse("!$omp parallel private(I)")
        .expect("fortran directive should parse");

    let mut config = ParserConfig::default();
    config.language = Language::Fortran;

    let ir = convert_directive(
        &directive,
        SourceLocation::start(),
        Language::Fortran,
        &config,
    )
    .expect("conversion to IR should succeed");

    assert_eq!(ir.to_plain_string(), "!$omp parallel private(<identifier>)");
}

#[test]
fn plain_string_combined_clauses_show_placeholders() {
    let input = "#pragma omp parallel for if(n > 10) schedule(dynamic, chunk) reduction(+: sum) collapse(2)";
    let (_, directive) = parse_omp_directive(input).expect("directive should parse");

    let mut config = ParserConfig::default();
    config.language = Language::C;

    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("conversion to IR should succeed");

    assert_eq!(
        ir.to_plain_string(),
        "#pragma omp parallel for if(<expr>) schedule(dynamic, <expr>) reduction(+: <identifier>) collapse(<expr>)"
    );
}

#[test]
fn plain_string_fortran_uppercase_input_normalizes_prefix() {
    let parser = openmp::parser().with_language(ParserLanguage::FortranFree);
    let (_, directive) = parser
        .parse("!$OMP target map(tofrom: A, B)")
        .expect("fortran directive should parse");

    let mut config = ParserConfig::default();
    config.language = Language::Fortran;

    let ir = convert_directive(
        &directive,
        SourceLocation::start(),
        Language::Fortran,
        &config,
    )
    .expect("conversion to IR should succeed");

    assert_eq!(
        ir.to_plain_string(),
        "!$omp target map(tofrom: <identifier>, <identifier>)"
    );
}
