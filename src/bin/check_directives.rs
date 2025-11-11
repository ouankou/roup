use std::ffi::CString;

use roup::acc_directive_name;
use roup::{acc_directive_free, acc_directive_kind, acc_parse};

fn directive_kind(input: &str) -> i32 {
    let c_input = CString::new(input).expect("valid pragma");
    let directive = acc_parse(c_input.as_ptr());
    if directive.is_null() {
        return -1;
    }
    let kind = acc_directive_kind(directive);
    acc_directive_free(directive);
    kind
}

fn main() {
    let samples = [
        "#pragma acc enter data copyin(a)",
        "#pragma acc enter_data copyin(a)",
        "#pragma acc exit data delete(a)",
        "#pragma acc exit_data delete(a)",
        "#pragma acc host_data use_device(ptr)",
        "#pragma acc host data use_device(ptr)",
    ];

    for s in &samples {
        let k = directive_kind(s);
        // Also attempt to fetch the parsed directive name from the C API for debugging
        let c_input = CString::new(*s).unwrap();
        let dir = acc_parse(c_input.as_ptr());
        if dir.is_null() {
            println!("{s} => parsed=NULL, kind={}", k);
            continue;
        }
        let cname = acc_directive_name(dir);
        if cname.is_null() {
            println!("{s} => parsed=name=NULL, kind={}", k);
        } else {
            let parsed_name = unsafe {
                std::ffi::CStr::from_ptr(cname)
                    .to_string_lossy()
                    .into_owned()
            };
            println!("{s} => parsed=\"{}\", kind={}", parsed_name, k);
        }
        acc_directive_free(dir);
    }
}
