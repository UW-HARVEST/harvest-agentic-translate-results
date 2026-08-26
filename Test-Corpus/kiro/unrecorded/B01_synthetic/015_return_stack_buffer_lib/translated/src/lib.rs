use std::ffi::{c_char, c_int};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(b"%s\n\0".as_ptr() as *const c_char, line);
        }
    }
}

/// Reproduces the C UB: returns a pointer to a local stack buffer.
unsafe fn helper_bad() -> *mut c_char {
    let mut char_string: [u8; 17] = *b"helperBad string\0";
    char_string.as_mut_ptr() as *mut c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    printLine(unsafe { helper_bad() });
}

/// Returns pointer to a static local — well-defined.
fn helper_good1() -> *mut c_char {
    static mut CHAR_STRING: [u8; 19] = *b"helperGood1 string\0";
    std::ptr::addr_of_mut!(CHAR_STRING) as *mut c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    printLine(helper_good1());
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
