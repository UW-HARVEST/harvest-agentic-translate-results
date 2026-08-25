use std::ffi::{c_char, c_int, c_void};

extern "C" {
    static mut stdin: *mut c_void;
    static mut stdout: *mut c_void;

    fn fgets(buffer: *mut c_char, count: c_int, stream: *mut c_void) -> *mut c_char;
    fn fputs(string: *const c_char, stream: *mut c_void) -> c_int;
}

#[no_mangle]
pub extern "C" fn main() -> c_int {
    let mut text = [0 as c_char; 128];

    unsafe {
        while !fgets(text.as_mut_ptr(), text.len() as c_int, stdin).is_null() {
            fputs(text.as_ptr(), stdout);
        }
    }

    0
}
