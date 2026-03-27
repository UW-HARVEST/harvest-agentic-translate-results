use std::ffi::{c_char, c_int};

unsafe fn strchr(mut s: *const c_char, c: c_char) -> *const c_char {
    loop {
        if *s == c {
            return s;
        }
        if *s == 0 {
            return std::ptr::null();
        }
        s = s.add(1);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn foo(in_: *const c_char, c: c_char) -> c_int {
    let mut res: c_int = 0;
    let mut s = in_;
    loop {
        s = unsafe { strchr(s, c) };
        if s.is_null() {
            break;
        }
        res += 1;
        s = unsafe { s.add(1) };
    }
    res
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(in_: *const c_char) {
    let a = foo(in_, b'A' as c_char);
    let x = foo(in_, b'x' as c_char);
    unsafe {
        libc::printf(b"A: %d\n\0".as_ptr() as *const c_char, a);
        libc::printf(b"x: %d\n\0".as_ptr() as *const c_char, x);
    }
}
