use std::ffi::CString;

use roup::parser::parse_acc_directive;
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
    ];

    for s in &samples {
        let k = directive_kind(s);
        // Also attempt to fetch the parsed directive name from the C API for debugging
        let c_input = CString::new(*s).unwrap();
        let dir = acc_parse(c_input.as_ptr());
        if dir.is_null() {
            println!("{s} => parsed=NULL, kind={k}");
            continue;
        }
        let parsed_name = parse_acc_directive(s)
            .map(|(_, directive)| directive.name.as_ref().to_string())
            .unwrap_or_else(|_| "<parse-error>".to_string());
        println!("{s} => parsed=\"{parsed_name}\", kind={k}");
        acc_directive_free(dir);
    }
}
