use std::ffi::CString;

#[test]
fn acc_parse_and_kind_roundtrip() {
    // Parse a simple OpenACC directive and ensure the C API returns the
    // expected directive kind. The C API maps 'parallel' to 0 (same as OpenMP).
    let input = CString::new("#pragma acc parallel").unwrap();
    let dir = roup::c_api::acc_parse(input.as_ptr());
    assert!(!dir.is_null(), "acc_parse returned NULL");

    let kind = roup::c_api::acc_directive_kind(dir);
    // After migrating the OpenACC C API to expose canonical ACC namespace values,
    // OpenACC directive kinds are returned as ACC_DIRECTIVE_BASE + raw (10000 + raw).
    // 'parallel' raw value is 0, so the expected returned kind is 10000.
    assert_eq!(kind, 10000, "expected acc_directive_kind == 10000 for 'parallel'");

    // Clean up
    roup::c_api::acc_directive_free(dir);
}
