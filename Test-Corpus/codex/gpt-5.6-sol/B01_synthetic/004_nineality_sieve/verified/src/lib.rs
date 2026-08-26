use std::ffi::{c_char, c_int, c_long};
use std::ptr;

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn puts(value: *const c_char) -> c_int;
    fn strtol(input: *const c_char, end: *mut *mut c_char, base: c_int) -> c_long;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    if argc != 2 {
        unsafe {
            puts(c"Error: should only be a single (integer) argument!".as_ptr());
        }
        return 1;
    }

    let input = unsafe { argv.add(1).read() };
    let mut end = ptr::null_mut();
    let parsed = unsafe { strtol(input, &mut end, 10) };
    if end == input {
        unsafe {
            puts(c"Error: first argument must be an integer!".as_ptr());
        }
        return 1;
    }

    let mut val = parsed as c_int;
    loop {
        unsafe {
            printf(c"%d\n".as_ptr(), val);
        }
        if val % 10 == 9 {
            break;
        }
        val = val.wrapping_add(1);
    }

    0
}
