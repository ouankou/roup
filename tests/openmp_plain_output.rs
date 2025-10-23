use std::ffi::{CStr, CString};

use roup::ir::{convert::convert_directive, Language as IrLanguage, ParserConfig, SourceLocation};
use roup::parser::parse_omp_directive;
use roup::{roup_directive_free, roup_directive_plain_string, roup_parse};

fn expected_plain() -> &'static str {
    "#pragma omp target data map(tofrom: ) map(to: )"
}

#[test]
fn plain_string_from_ir_conversion_matches_expectation() {
    let source =
        "#pragma omp target data map(tofrom: a[0:ARRAY_SIZE], num_teams) map(to: b[0:ARRAY_SIZE])";
    let (_, directive) = parse_omp_directive(source).expect("parser should succeed");

    let config = ParserConfig::with_parsing(IrLanguage::C);
    let ir = convert_directive(&directive, SourceLocation::start(), IrLanguage::C, &config)
        .expect("conversion to IR should succeed");

    assert_eq!(ir.to_plain_string(), expected_plain());
}

#[test]
fn plain_string_available_through_c_api() {
    let source = CString::new(
        "#pragma omp target data map(tofrom: a[0:ARRAY_SIZE], num_teams) map(to: b[0:ARRAY_SIZE])",
    )
    .unwrap();

    let directive = roup_parse(source.as_ptr());
    assert!(!directive.is_null(), "directive should parse");

    let plain_ptr = roup_directive_plain_string(directive);
    assert!(!plain_ptr.is_null(), "plain string pointer should be valid");

    let plain = unsafe { CStr::from_ptr(plain_ptr) }
        .to_str()
        .expect("valid UTF-8");
    assert_eq!(plain, expected_plain());

    roup_directive_free(directive);
}
