use std::ffi::{c_char, c_int};

fn foo(input: *const c_char, c: c_char) -> c_int {
    let mut res: c_int = 0;
    unsafe {
        let mut s = input;
        loop {
            s = libc::strchr(s, c as c_int);
            if s.is_null() {
                break;
            }
            res += 1;
            s = s.add(1);
        }
    }
    res
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(input: *const c_char) {
    unsafe {
        libc::printf(b"A: %d\n\0".as_ptr() as *const c_char, foo(input, b'A' as c_char));
        libc::printf(b"x: %d\n\0".as_ptr() as *const c_char, foo(input, b'x' as c_char));
    }
}
