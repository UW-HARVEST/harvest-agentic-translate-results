use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn foo(input: *const c_char, c: c_char) -> c_int {
    let mut res: c_int = 0;
    let mut s = input;

    while {
        s = unsafe { strchr(s, c as c_int) };
        !s.is_null()
    } {
        res += 1;
        s = unsafe { s.add(1) };
    }

    res
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(input: *const c_char) {
    unsafe {
        printf(c"A: %d\n".as_ptr(), foo(input, b'A' as c_char));
        printf(c"x: %d\n".as_ptr(), foo(input, b'x' as c_char));
    }
}
