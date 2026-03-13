use std::ffi::{c_char, c_int};

extern "C" {
    fn strchr(s: *const c_char, c: c_int) -> *const c_char;
    fn printf(fmt: *const c_char, ...) -> c_int;
}

fn foo(r#in: *const c_char, c: c_char) -> c_int {
    let mut res: c_int = 0;
    let mut s = r#in;
    loop {
        s = unsafe { strchr(s, c as c_int) };
        if s.is_null() {
            break;
        }
        res += 1;
        s = unsafe { s.add(1) };
    }
    res
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(r#in: *const c_char) {
    unsafe {
        printf(b"A: %d\n\0".as_ptr() as *const c_char, foo(r#in, b'A' as c_char));
        printf(b"x: %d\n\0".as_ptr() as *const c_char, foo(r#in, b'x' as c_char));
    }
}
