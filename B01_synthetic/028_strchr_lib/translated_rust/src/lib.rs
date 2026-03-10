use std::ffi::{c_char, c_int};

unsafe fn foo(inp: *const c_char, c: c_char) -> c_int {
    let mut res: c_int = 0;
    let mut s = inp;
    loop {
        s = unsafe { libc::strchr(s, c as c_int) };
        if s.is_null() {
            break;
        }
        res += 1;
        s = unsafe { s.add(1) };
    }
    res
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(inp: *const c_char) {
    let a_count = unsafe { foo(inp, b'A' as c_char) };
    let x_count = unsafe { foo(inp, b'x' as c_char) };
    print!("A: {}\n", a_count);
    print!("x: {}\n", x_count);
}
