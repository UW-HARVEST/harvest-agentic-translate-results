use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn puts(string: *const c_char) -> c_int;
    fn strncpy(destination: *mut c_char, source: *const c_char, count: usize) -> *mut c_char;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            puts(line);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(data: c_int) {
    let mut source = [b'A' as c_char; 100];
    let mut destination = [0 as c_char; 100];
    source[99] = 0;

    if data < 100 {
        unsafe {
            strncpy(destination.as_mut_ptr(), source.as_ptr(), data as usize);
            *destination.as_mut_ptr().offset(data as isize) = 0;
        }
    }

    unsafe {
        printLine(destination.as_ptr());
    }
}
