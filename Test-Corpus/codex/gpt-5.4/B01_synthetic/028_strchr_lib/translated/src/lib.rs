use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn strchr(cs: *const c_char, c: c_int) -> *const c_char;
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn foo(in_: *const c_char, c: c_char) -> c_int {
    let mut res: c_int = 0;
    let mut s = in_;

    loop {
        s = strchr(s, c as c_int);
        if s.is_null() {
            break;
        }
        res += 1;
        s = s.add(1);
    }

    res
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(in_: *const c_char) {
    printf(c"A: %d\n".as_ptr(), foo(in_, b'A' as c_char));
    printf(c"x: %d\n".as_ptr(), foo(in_, b'x' as c_char));
}
