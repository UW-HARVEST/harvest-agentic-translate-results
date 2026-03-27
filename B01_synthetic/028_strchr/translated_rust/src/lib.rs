use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

fn foo_impl(input: &[u8], c: u8) -> i32 {
    let mut count: i32 = 0;
    for &b in input {
        if b == 0 {
            break;
        }
        if b == c {
            count += 1;
        }
    }
    count
}

#[no_mangle]
pub extern "C" fn foo(input: *const c_char, c: c_char) -> c_int {
    let cstr = unsafe { CStr::from_ptr(input) };
    foo_impl(cstr.to_bytes_with_nul(), c as u8)
}

#[no_mangle]
pub extern "C" fn driver(input: *const c_char) {
    let cstr = unsafe { CStr::from_ptr(input) };
    let bytes = cstr.to_bytes_with_nul();
    print!("A: {}\n", foo_impl(bytes, b'A'));
    print!("x: {}\n", foo_impl(bytes, b'x'));
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> c_int {
    let mut buf = [0u8; 1000];
    unsafe {
        let stdin = libc::fdopen(0, b"r\0".as_ptr() as *const c_char);
        libc::fread(buf.as_mut_ptr() as *mut libc::c_void, 1, 1000, stdin);
    }
    driver(buf.as_ptr() as *const c_char);
    0
}
