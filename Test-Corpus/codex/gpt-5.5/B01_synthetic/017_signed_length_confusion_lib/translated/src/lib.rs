use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn puts(s: *const c_char) -> c_int;
    fn strncpy(dest: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
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
    let mut source = [0 as c_char; 100];
    let mut dest = [0 as c_char; 100];

    for byte in source.iter_mut().take(100 - 1) {
        *byte = b'A' as c_char;
    }
    source[100 - 1] = 0;

    if data < 100 {
        unsafe {
            strncpy(dest.as_mut_ptr(), source.as_ptr(), data as usize);
            *dest.as_mut_ptr().offset(data as isize) = 0;
        }
    }

    unsafe {
        printLine(dest.as_ptr());
    }
}
