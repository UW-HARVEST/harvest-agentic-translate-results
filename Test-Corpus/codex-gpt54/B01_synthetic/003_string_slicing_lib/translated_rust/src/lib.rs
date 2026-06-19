use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn strlen(s: *const c_char) -> usize;
}

const START_OFF_END: &[u8] = b"Error: start is off the end of the string!\n\0";
const STOP_OFF_END: &[u8] = b"Error: stop is off the end of the string!\n\0";
const STOP_AFTER_START: &[u8] = b"Error: stop must come after start!\n\0";
const SLICE_FORMAT: &[u8] = b"%.*s\n\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn slice(
    mystr: *mut c_char,
    start_ptr: *mut c_int,
    stop_ptr: *mut c_int,
) -> c_int {
    let len = unsafe { strlen(mystr.cast_const()) };

    let start = if start_ptr.is_null() {
        0
    } else {
        let start = unsafe { *start_ptr };
        if start < 0 || (start as usize) > len {
            unsafe { printf(START_OFF_END.as_ptr().cast()) };
            return 1;
        }
        start
    };

    let stop = if stop_ptr.is_null() {
        len as c_int
    } else {
        let stop = unsafe { *stop_ptr };
        if stop < 0 || (stop as usize) > len {
            unsafe { printf(STOP_OFF_END.as_ptr().cast()) };
            return 1;
        }
        if stop <= start {
            unsafe { printf(STOP_AFTER_START.as_ptr().cast()) };
            return 1;
        }
        stop
    };

    let start_offset = start as isize;
    let slice_ptr = unsafe { mystr.cast::<u8>().offset(start_offset) }.cast::<c_char>();
    unsafe { printf(SLICE_FORMAT.as_ptr().cast(), stop - start, slice_ptr) };

    0
}
