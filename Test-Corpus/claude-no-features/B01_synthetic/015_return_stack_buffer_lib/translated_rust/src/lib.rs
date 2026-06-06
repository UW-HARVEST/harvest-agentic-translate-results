use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        let fmt = b"%s\n\0".as_ptr() as *const c_char;
        unsafe {
            printf(fmt, line);
        }
    }
}

// Mirrors the C `helperBad` which returns a pointer to a stack-allocated
// array — undefined behavior preserved as in the original C source.
#[inline(never)]
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
    let p: *mut c_char = char_string.as_mut_ptr();
    // Touch memory to simulate the C compiler keeping the string materialized
    // on the stack; the returned pointer is then dangling, matching the C bug.
    let _ = char_string[0];
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    unsafe {
        let p = helper_bad();
        printLine(p);
    }
}

// Mirrors the C `helperGood1` which returns a pointer to a static array.
fn helper_good1() -> *mut c_char {
    // Static null-terminated string matching the C: "helperGood1 string"
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
    unsafe {
        let p = helper_good1();
        printLine(p);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(use_good: c_int) {
    unsafe {
        if use_good != 0 {
            good();
        } else {
            bad();
        }
    }
}
