// Translated from C to Rust. Preserves the original (buggy) semantics.

use std::ffi::c_char;
use std::ffi::c_int;

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn scanf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        let fmt = b"%s\n\0".as_ptr() as *const c_char;
        printf(fmt, line);
    }
}

// Reproduces the C bug: returns a pointer to a local (stack) array.
// This is undefined behavior in both C and Rust, but we mirror the original.
unsafe fn helper_bad() -> *mut c_char {
    let mut char_string: [c_char; 17] = [
        b'h' as c_char,
        b'e' as c_char,
        b'l' as c_char,
        b'p' as c_char,
        b'e' as c_char,
        b'r' as c_char,
        b'B' as c_char,
        b'a' as c_char,
        b'd' as c_char,
        b' ' as c_char,
        b's' as c_char,
        b't' as c_char,
        b'r' as c_char,
        b'i' as c_char,
        b'n' as c_char,
        b'g' as c_char,
        0,
    ];
    char_string.as_mut_ptr()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    printLine(helper_bad());
}

unsafe fn helper_good1() -> *mut c_char {
    static mut CHAR_STRING: [c_char; 19] = [
        b'h' as c_char,
        b'e' as c_char,
        b'l' as c_char,
        b'p' as c_char,
        b'e' as c_char,
        b'r' as c_char,
        b'G' as c_char,
        b'o' as c_char,
        b'o' as c_char,
        b'd' as c_char,
        b'1' as c_char,
        b' ' as c_char,
        b's' as c_char,
        b't' as c_char,
        b'r' as c_char,
        b'i' as c_char,
        b'n' as c_char,
        b'g' as c_char,
        0,
    ];
    (&raw mut CHAR_STRING) as *mut c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    printLine(helper_good1());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn main() -> c_int {
    let mut x: c_int = 0;
    let fmt = b"%d\0".as_ptr() as *const c_char;
    scanf(fmt, &mut x as *mut c_int);

    if x != 0 {
        good();
    } else {
        bad();
    }

    0
}
