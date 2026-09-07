use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn foo(input: *const c_char, c: c_char) -> c_int {
    let mut result = 0;
    let mut cursor = input;

    loop {
        let found = unsafe { strchr(cursor, c as c_int) };
        if found.is_null() {
            return result;
        }

        result += 1;
        cursor = unsafe { found.add(1) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(input: *const c_char) {
    let a_count = unsafe { foo(input, b'A' as c_char) };
    unsafe {
        printf(c"A: %d\n".as_ptr(), a_count);
    }

    let x_count = unsafe { foo(input, b'x' as c_char) };
    unsafe {
        printf(c"x: %d\n".as_ptr(), x_count);
    }
}
