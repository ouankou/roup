use std::ffi::CString;
use std::os::raw::c_char;

/// This integration test ensures the OpenACC C API returns the same numeric
/// directive kind values that the generated `ROUP_ACC_DIRECTIVE_*` macros
/// expose in `src/roup_constants.h`. It prevents regressions where the
/// C API returned reduced values (0..N) while the generated macros used the
/// ACC_DIRECTIVE_BASE offset.
#[test]
fn acc_directive_kind_matches_generated_macro() {
    // Read the generated header and extract the numeric macro value for
    // ROUP_ACC_DIRECTIVE_PARALLEL so we can compare runtime outputs against it.
    let header =
        std::fs::read_to_string("src/roup_constants.h").expect("failed to read generated header");
    let re = regex::Regex::new(r"#define\s+ROUP_ACC_DIRECTIVE_PARALLEL\s+(\-?\d+)").unwrap();
    let caps = re
        .captures(&header)
        .expect("ROUP_ACC_DIRECTIVE_PARALLEL not found in header");
    let expected: i32 = caps.get(1).unwrap().as_str().parse().unwrap();

    // Prepare a simple OpenACC directive string
    let input = CString::new("#pragma acc parallel").unwrap();

    // Call the C API (from Rust module) to parse the directive
    // Use the public `acc_parse` symbol re-exported by the crate
    let dir = roup::acc_parse(input.as_ptr() as *const c_char);
    assert!(
        !dir.is_null(),
        "acc_parse returned NULL for a valid directive"
    );

    let kind = roup::acc_directive_kind(dir);

    // The generated macro should equal the canonical OpenACC numeric value
    assert_eq!(
        kind, expected,
        "acc_directive_kind returned {}, header macro = {}",
        kind, expected
    );

    // Clean up
    roup::acc_directive_free(dir);
}
