use std::ffi::CString;
use std::os::raw::c_char;

/// Validate several OpenACC directive kinds against the generated ROUP_ACC macros
/// and ensure the 'end' paired directive kind is returned in the OpenACC numeric namespace.
#[test]
fn acc_multiple_directives_and_end_pairs_match_generated_macros() {
    let header =
        std::fs::read_to_string("src/roup_constants.h").expect("failed to read generated header");

    let directives = [
        ("parallel", "ROUP_ACC_DIRECTIVE_PARALLEL"),
        ("loop", "ROUP_ACC_DIRECTIVE_LOOP"),
        ("kernels", "ROUP_ACC_DIRECTIVE_KERNELS"),
    ];

    for (text, macro_name) in &directives {
        let re = regex::Regex::new(&format!(r"#define\s+{}\s+(\-?\d+)", macro_name)).unwrap();
        let caps = re
            .captures(&header)
            .unwrap_or_else(|| panic!("{} not found in header", macro_name));
        let expected: i32 = caps.get(1).unwrap().as_str().parse().unwrap();

        let input = CString::new(format!("#pragma acc {}", text)).unwrap();
        let dir = roup::acc_parse(input.as_ptr() as *const c_char);
        assert!(!dir.is_null(), "acc_parse returned NULL for '{}'", text);
        let kind = roup::acc_directive_kind(dir);
        assert_eq!(
            kind, expected,
            "directive '{}' returned {}, header macro = {}",
            text, kind, expected
        );
        roup::acc_directive_free(dir);
    }

    // Now test an end-paired directive: `end parallel` should return the ROUP_ACC_DIRECTIVE_PARALLEL
    let header =
        std::fs::read_to_string("src/roup_constants.h").expect("failed to read generated header");
    let re = regex::Regex::new(r"#define\s+ROUP_ACC_DIRECTIVE_PARALLEL\s+(\-?\d+)").unwrap();
    let caps = re
        .captures(&header)
        .unwrap_or_else(|| panic!("ROUP_ACC_DIRECTIVE_PARALLEL not found in header"));
    let expected_parallel: i32 = caps.get(1).unwrap().as_str().parse().unwrap();

    let end_input = CString::new("#pragma acc end parallel").unwrap();
    let end_dir = roup::acc_parse(end_input.as_ptr() as *const c_char);
    assert!(
        !end_dir.is_null(),
        "acc_parse returned NULL for 'end parallel'"
    );
    let paired = roup::acc_directive_end_paired_kind(end_dir);
    assert_eq!(
        paired, expected_parallel,
        "end paired kind {} != expected {}",
        paired, expected_parallel
    );
    roup::acc_directive_free(end_dir);
}
