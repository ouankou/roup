use std::ffi::CStr;
use std::os::raw::c_char;

extern "C" {
    fn roup_parse(input: *const c_char) -> *mut std::ffi::c_void;
    fn roup_directive_free(dir: *mut std::ffi::c_void);
}

fn main() {
    let input = "#pragma omp for schedule(monotonic,simd:runtime,2)\0";
    unsafe {
        let dir = roup_parse(input.as_ptr() as *const c_char);
        if dir.is_null() {
            println!("Failed to parse");
        } else {
            println!("Parsed successfully");
            roup_directive_free(dir);
        }
    }
}