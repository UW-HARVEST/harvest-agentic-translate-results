use std::ffi::c_char;
use std::os::raw::c_int;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn foo(input: *const c_char, c: c_char) -> c_int {
    if input.is_null() {
        return 0;
    }
    let mut res: c_int = 0;
    let mut s = input;
    loop {
        // Replicate strchr semantics: find c (matched as char, NUL terminator
        // is also matchable). When c is '\0', strchr returns pointer to the
        // terminating NUL byte. Otherwise returns NULL when not found.
        let found = strchr_impl(s, c);
        if found.is_null() {
            break;
        }
        res += 1;
        s = found.add(1);
    }
    res
}

#[inline]
unsafe fn strchr_impl(s: *const c_char, c: c_char) -> *const c_char {
    let mut p = s;
    loop {
        let ch = *p;
        if ch == c {
            return p;
        }
        if ch == 0 {
            return std::ptr::null();
        }
        p = p.add(1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(input: *const c_char) {
    let fmt_a = b"A: %d\n\0".as_ptr() as *const c_char;
    let fmt_x = b"x: %d\n\0".as_ptr() as *const c_char;
    printf(fmt_a, foo(input, b'A' as c_char) as c_int);
    printf(fmt_x, foo(input, b'x' as c_char) as c_int);
}
