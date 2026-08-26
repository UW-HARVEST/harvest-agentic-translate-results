use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn puts(string: *const c_char) -> c_int;
    fn strlen(string: *const c_char) -> usize;
}

const START_ERROR: &[u8] = b"Error: start is off the end of the string!\0";
const STOP_ERROR: &[u8] = b"Error: stop is off the end of the string!\0";
const ORDER_ERROR: &[u8] = b"Error: stop must come after start!\0";
const SLICE_FORMAT: &[u8] = b"%.*s\n\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn slice(
    mystr: *mut c_char,
    start_ptr: *mut c_int,
    stop_ptr: *mut c_int,
) -> c_int {
    let len = unsafe { strlen(mystr) };

    let start = if start_ptr.is_null() {
        0
    } else {
        let start = unsafe { *start_ptr };
        if (start as usize) > len {
            unsafe { puts(START_ERROR.as_ptr().cast()) };
            return 1;
        }
        start
    };

    let stop = if stop_ptr.is_null() {
        len as c_int
    } else {
        let stop = unsafe { *stop_ptr };
        if (stop as usize) > len {
            unsafe { puts(STOP_ERROR.as_ptr().cast()) };
            return 1;
        }
        if stop <= start {
            unsafe { puts(ORDER_ERROR.as_ptr().cast()) };
            return 1;
        }
        stop
    };

    unsafe {
        printf(
            SLICE_FORMAT.as_ptr().cast(),
            stop.wrapping_sub(start),
            mystr.add(start as usize),
        )
    };

    0
}
