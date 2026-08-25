use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn strchr(string: *const c_char, character: c_int) -> *mut c_char;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn foo(input: *const c_char, character: c_char) -> c_int {
    let mut result = 0;
    let mut current = input;

    loop {
        let found = unsafe { strchr(current, c_int::from(character)) };
        if found.is_null() {
            return result;
        }

        result += 1;
        current = unsafe { found.add(1) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(input: *const c_char) {
    let uppercase_a_count = unsafe { foo(input, b'A' as c_char) };
    unsafe {
        printf(c"A: %d\n".as_ptr(), uppercase_a_count);
    }

    let lowercase_x_count = unsafe { foo(input, b'x' as c_char) };
    unsafe {
        printf(c"x: %d\n".as_ptr(), lowercase_x_count);
    }
}
