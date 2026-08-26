use std::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
    fn printf(format: *const c_char, ...) -> c_int;
    fn fread(ptr: *mut c_void, size: usize, count: usize, stream: *mut c_void) -> usize;

    #[link_name = "stdin"]
    static mut C_STDIN: *mut c_void;
}

#[no_mangle]
pub unsafe extern "C" fn foo(input: *const c_char, needle: c_char) -> c_int {
    let mut result = 0;
    let mut cursor = input;

    loop {
        cursor = unsafe { strchr(cursor, needle as c_int) };
        if cursor.is_null() {
            return result;
        }

        result += 1;
        cursor = unsafe { cursor.add(1) };
    }
}

#[no_mangle]
pub unsafe extern "C" fn driver(input: *const c_char) {
    unsafe {
        printf(c"A: %d\n".as_ptr(), foo(input, b'A' as c_char));
        printf(c"x: %d\n".as_ptr(), foo(input, b'x' as c_char));
    }
}

#[no_mangle]
pub unsafe extern "C" fn main() -> c_int {
    let mut input = [0 as c_char; 1000];

    unsafe {
        fread(input.as_mut_ptr().cast(), 1, input.len(), C_STDIN);
        driver(input.as_ptr());
    }

    0
}
