// Rust translation of c_src/src/driver.c
//
// Original C (MIT Lincoln Laboratory, 2025) is a small shared library exposing
// `driver()` (declared in include/driver.h) and, as a side effect of having
// external linkage, `foo()`.
//
// Both symbols are reproduced here with their original linker names. The header
// contains no namespace/renaming macros, so the source-level names are the
// final linker symbols.

use std::ffi::{c_char, c_int};
use std::ptr;

unsafe extern "C" {
    // Used so that formatting and stdout buffering behave exactly like the C
    // library's, byte for byte.
    fn printf(format: *const c_char, ...) -> c_int;
}

/// Equivalent of C `strchr`: returns a pointer to the first occurrence of `c`
/// in the NUL-terminated string `s`, or NULL if not found.
///
/// Note that, exactly as in C, a `c` of `'\0'` matches the terminator itself
/// and therefore yields a pointer to it rather than NULL.
unsafe fn strchr(s: *const c_char, c: c_char) -> *const c_char {
    let mut p = s;
    loop {
        let b = unsafe { *p };
        if b == c {
            return p;
        }
        if b == 0 {
            return ptr::null();
        }
        p = unsafe { p.add(1) };
    }
}

/// int foo(const char *in, char c)
///
/// Counts the occurrences of `c` in `in`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foo(in_: *const c_char, c: c_char) -> c_int {
    let mut res: c_int = 0;
    let mut s: *const c_char = in_;
    loop {
        s = unsafe { strchr(s, c) };
        if s.is_null() {
            break;
        }
        res = res.wrapping_add(1);
        s = unsafe { s.add(1) };
    }
    res
}

/// void driver(const char *in)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(in_: *const c_char) {
    unsafe {
        printf(c"A: %d\n".as_ptr(), foo(in_, b'A' as c_char));
        printf(c"x: %d\n".as_ptr(), foo(in_, b'x' as c_char));
    }
}
