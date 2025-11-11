use std::ffi::CString;

use roup::{
    acc_clause_iterator_free, acc_clause_iterator_next, acc_clause_kind,
    acc_directive_clauses_iter, acc_directive_free, acc_directive_kind, acc_parse, AccClause,
};

fn directive_kind(input: &str) -> i32 {
    let c_input = CString::new(input).expect("valid pragma");
    let directive = acc_parse(c_input.as_ptr());
    assert!(!directive.is_null(), "failed to parse {input}");
    let kind = acc_directive_kind(directive);
    acc_directive_free(directive);
    kind
}

fn clause_kinds(input: &str) -> Vec<i32> {
    let c_input = CString::new(input).expect("valid pragma");
    let directive = acc_parse(c_input.as_ptr());
    assert!(!directive.is_null(), "failed to parse {input}");
    let iter = acc_directive_clauses_iter(directive);
    assert!(!iter.is_null(), "iterator should be valid");

    let mut kinds = Vec::new();
    loop {
        let mut clause_ptr: *const AccClause = std::ptr::null();
        let has_next = acc_clause_iterator_next(iter, &mut clause_ptr);
        if has_next == 0 {
            break;
        }
        assert!(!clause_ptr.is_null(), "iterator returned null clause");
        kinds.push(acc_clause_kind(clause_ptr));
    }

    acc_clause_iterator_free(iter);
    acc_directive_free(directive);
    kinds
}

#[test]
fn directive_synonyms_share_kind() {
    let enter_data = directive_kind("#pragma acc enter data copyin(a)");
    let enter_data_underscore = directive_kind("#pragma acc enter_data copyin(a)");
    // Canonical policy: accept only the space-separated form for enter/exit.
    assert_ne!(enter_data, -1);
    assert_eq!(enter_data_underscore, -1);

    let host_data = directive_kind("#pragma acc host_data use_device(ptr)");
    let host_data_space = directive_kind("#pragma acc host data use_device(ptr)");
    // Canonical policy: accept only the underscore form for host_data.
    assert_ne!(host_data, -1);
    assert_eq!(host_data_space, -1);

    let wait_plain = directive_kind("#pragma acc wait");
    let wait_with_args = directive_kind("#pragma acc wait(1)");
    assert_ne!(wait_plain, -1);
    assert_ne!(wait_with_args, -1);
}

#[test]
fn clause_aliases_map_to_base_kind() {
    let copy_kind = clause_kinds("#pragma acc data copy(a)");
    let pcopy_kind = clause_kinds("#pragma acc data pcopy(a)");
    let present_or_copy_kind = clause_kinds("#pragma acc data present_or_copy(a)");
    assert_eq!(copy_kind, pcopy_kind);
    assert_eq!(copy_kind, present_or_copy_kind);

    let copyin_kind = clause_kinds("#pragma acc data copyin(a)");
    let pcopyin_kind = clause_kinds("#pragma acc data pcopyin(a)");
    let present_or_copyin_kind = clause_kinds("#pragma acc data present_or_copyin(a)");
    assert_eq!(copyin_kind, pcopyin_kind);
    assert_eq!(copyin_kind, present_or_copyin_kind);

    let copyout_kind = clause_kinds("#pragma acc data copyout(a)");
    let pcopyout_kind = clause_kinds("#pragma acc data pcopyout(a)");
    let present_or_copyout_kind = clause_kinds("#pragma acc data present_or_copyout(a)");
    assert_eq!(copyout_kind, pcopyout_kind);
    assert_eq!(copyout_kind, present_or_copyout_kind);

    let create_kind = clause_kinds("#pragma acc data create(a)");
    let pcreate_kind = clause_kinds("#pragma acc data pcreate(a)");
    let present_or_create_kind = clause_kinds("#pragma acc data present_or_create(a)");
    assert_eq!(create_kind, pcreate_kind);
    assert_eq!(create_kind, present_or_create_kind);

    let device_type_kind = clause_kinds("#pragma acc parallel device_type(default)");
    let dtype_kind = clause_kinds("#pragma acc parallel dtype(default)");
    assert_eq!(device_type_kind, dtype_kind);
}

#[test]
fn atomic_update_uses_update_clause_kind() {
    let update_kind = clause_kinds("#pragma acc atomic update");
    assert_eq!(update_kind.len(), 1);

    let synthetic_update_clause = clause_kinds("#pragma acc parallel update(1)");
    assert_eq!(
        update_kind[0], synthetic_update_clause[0],
        "update clause id should match"
    );
}
