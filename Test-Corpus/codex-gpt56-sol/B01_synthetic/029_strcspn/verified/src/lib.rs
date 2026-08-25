use std::ffi::{c_char, c_int, c_void, CStr};

const BUFFER_SIZE: usize = 100;

type File = c_void;

unsafe extern "C" {
    static mut stdin: *mut File;

    fn fgets(buffer: *mut c_char, size: c_int, stream: *mut File) -> *mut c_char;
    fn printf(format: *const c_char, ...) -> c_int;
    fn strlen(value: *const c_char) -> usize;
}

unsafe fn span_without_rejected_bytes(s1: *const c_char, s2: *const c_char) -> usize {
    let value = unsafe { CStr::from_ptr(s1) }.to_bytes();
    let rejected = unsafe { CStr::from_ptr(s2) }.to_bytes();

    value
        .iter()
        .position(|byte| rejected.contains(byte))
        .unwrap_or(value.len())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(s1: *const c_char, s2: *const c_char) {
    let span = unsafe { span_without_rejected_bytes(s1, s2) };
    unsafe {
        printf(c"%zu\n".as_ptr(), span);
    }
}

#[cfg_attr(not(test), unsafe(export_name = "main"))]
pub unsafe extern "C" fn c_main() -> c_int {
    let mut s1 = [0 as c_char; BUFFER_SIZE];
    let mut s2 = [0 as c_char; BUFFER_SIZE];

    unsafe {
        fgets(s1.as_mut_ptr(), BUFFER_SIZE as c_int, stdin);
        fgets(s2.as_mut_ptr(), BUFFER_SIZE as c_int, stdin);

        *s1.as_mut_ptr().add(strlen(s1.as_ptr()).wrapping_sub(1)) = 0;
        *s2.as_mut_ptr().add(strlen(s2.as_ptr()).wrapping_sub(1)) = 0;

        driver(s1.as_ptr(), s2.as_ptr());
    }

    0
}
