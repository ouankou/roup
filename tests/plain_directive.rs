use roup::ir::{convert::convert_directive, Language, ParserConfig, SourceLocation};
use roup::lexer::Language as LexerLanguage;
use roup::parser::{openmp, parse_omp_directive};
use std::ffi::{CStr, CString};

fn convert(input: &str) -> String {
    let (_, directive) = parse_omp_directive(input).expect("directive should parse");
    let config = ParserConfig::with_parsing(Language::C);
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("conversion should succeed");
    ir.to_plain_string()
}

fn convert_fortran(input: &str) -> String {
    let parser = openmp::parser().with_language(LexerLanguage::FortranFree);
    let (_, directive) = parser.parse(input).expect("directive should parse");
    let config = ParserConfig::with_parsing(Language::Fortran);
    let ir = convert_directive(
        &directive,
        SourceLocation::start(),
        Language::Fortran,
        &config,
    )
    .expect("conversion should succeed");
    ir.to_plain_string()
}

#[test]
fn plain_string_for_target_data_maps() {
    let input = "#pragma omp target data map(tofrom: a[0:N]) map(to: b[0:N])";
    let plain = convert(input);
    assert_eq!(
        plain,
        "#pragma omp target data map(tofrom: ...) map(to: ...)"
    );
    assert!(!plain.contains("a[0:N]"));
    assert!(!plain.contains("b[0:N]"));
}

#[test]
fn plain_string_for_parallel_for_with_clauses() {
    let input =
        "#pragma omp parallel for if(parallel: n > 0) reduction(+: sum) schedule(dynamic, 8)";
    let plain = convert(input);
    assert_eq!(
        plain,
        "#pragma omp parallel for if(parallel: ...) reduction(+: ...) schedule(dynamic, ...)",
    );
}

#[test]
fn plain_string_for_fortran_parallel() {
    let input = "!$omp parallel reduction(+: sum)";
    let plain = convert_fortran(input);
    assert!(plain.starts_with("!$omp parallel"));
    assert!(plain.contains("reduction"));
    assert!(plain.contains("..."));
}

#[test]
fn plain_string_for_invalid_proc_bind_uses_placeholder() {
    let input = CString::new("#pragma omp parallel proc_bind(foo)").unwrap();
    let directive = roup::roup_parse(input.as_ptr());
    assert!(!directive.is_null());

    let plain_ptr = roup::roup_directive_plain(directive);
    assert!(!plain_ptr.is_null());
    let plain = unsafe { CStr::from_ptr(plain_ptr) }.to_str().unwrap();
    assert_eq!(plain, "#pragma omp parallel proc_bind(...)");

    roup::roup_directive_free(directive);
}
