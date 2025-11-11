use std::ffi::CString;

#[test]
fn acc_parse_and_kind_roundtrip() {
    // Parse a simple OpenACC directive and ensure the C API returns the
    // expected directive kind. The C API maps 'parallel' to 0 (same as OpenMP).
    let input = CString::new("#pragma acc parallel").unwrap();
    let dir = roup::c_api::acc_parse(input.as_ptr());
    assert!(!dir.is_null(), "acc_parse returned NULL");

    let kind = roup::c_api::acc_directive_kind(dir);
    // 'parallel' should map to directive code 0 in the current mapping
    assert_eq!(kind, 0, "expected acc_directive_kind == 0 for 'parallel'");

    // Clean up
    roup::c_api::acc_directive_free(dir);
}
