use std::ffi::{c_char, c_int};
use std::ptr;

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            libc::printf(b"%s\n\0".as_ptr() as *const c_char, line);
        }
    }
}

/// Reproduces the C bug: returns a pointer to a local stack buffer (UB).
#[inline(never)]
fn helper_bad() -> *mut c_char {
    unsafe {
        let mut buf: [c_char; 17] = [0; 17];
        let src = b"helperBad string\0";
        ptr::copy_nonoverlapping(src.as_ptr(), buf.as_mut_ptr() as *mut u8, 17);
        buf.as_mut_ptr()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    printLine(helper_bad());
}

fn helper_good1() -> *mut c_char {
    static mut CHAR_STRING: [c_char; 19] = [0; 19];
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| unsafe {
        let src = b"helperGood1 string\0";
        let p = ptr::addr_of_mut!(CHAR_STRING) as *mut u8;
        ptr::copy_nonoverlapping(src.as_ptr(), p, 19);
    });
    ptr::addr_of_mut!(CHAR_STRING) as *mut c_char
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
