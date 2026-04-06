use std::ffi::{c_char, c_int};

extern "C" {
    fn strlen(s: *const c_char) -> usize;
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn slice(
    mystr: *mut c_char,
    start_ptr: *const c_int,
    stop_ptr: *const c_int,
) -> c_int {
    unsafe {
        let len = strlen(mystr);

        let start: c_int;
        let stop: c_int;

        if !start_ptr.is_null() {
            start = *start_ptr;
            if (start as usize) > len {
                printf(b"Error: start is off the end of the string!\n\0".as_ptr() as *const c_char);
                return 1;
            }
        } else {
            start = 0;
        }

        if !stop_ptr.is_null() {
            stop = *stop_ptr;
            if (stop as usize) > len {
                printf(b"Error: stop is off the end of the string!\n\0".as_ptr() as *const c_char);
                return 1;
            }
            if stop <= start {
                printf(b"Error: stop must come after start!\n\0".as_ptr() as *const c_char);
                return 1;
            }
        } else {
            stop = len as c_int;
        }

        printf(
            b"%.*s\n\0".as_ptr() as *const c_char,
            stop - start,
            mystr.offset(start as isize),
        );

        0
    }
}
