use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn strlen(s: *const c_char) -> usize;
    fn printf(format: *const c_char, ...) -> c_int;
}

const ERR_START: &[u8] = b"Error: start is off the end of the string!\n\0";
const ERR_STOP: &[u8] = b"Error: stop is off the end of the string!\n\0";
const ERR_ORDER: &[u8] = b"Error: stop must come after start!\n\0";
const PRINT_SLICE: &[u8] = b"%.*s\n\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn slice(
    mystr: *mut c_char,
    start_ptr: *mut c_int,
    stop_ptr: *mut c_int,
) -> c_int {
    let len = unsafe { strlen(mystr.cast_const()) };

    let start: c_int = if !start_ptr.is_null() {
        let start = unsafe { *start_ptr };
        if (start as usize) > len {
            unsafe {
                printf(ERR_START.as_ptr().cast());
            }
            return 1;
        }
        start
    } else {
        0
    };

    let stop: c_int = if !stop_ptr.is_null() {
        let stop = unsafe { *stop_ptr };
        if (stop as usize) > len {
            unsafe {
                printf(ERR_STOP.as_ptr().cast());
            }
            return 1;
        }
        if stop <= start {
            unsafe {
                printf(ERR_ORDER.as_ptr().cast());
            }
            return 1;
        }
        stop
    } else {
        len as c_int
    };

    unsafe {
        printf(
            PRINT_SLICE.as_ptr().cast(),
            stop.wrapping_sub(start),
            mystr.offset(start as isize),
        );
    }

    0
}
