use std::ffi::c_char;
use std::os::raw::c_int;

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn strlen(s: *const c_char) -> usize;
}

/*
Index into a passed string
and print the substring indexed by [*start_ptr, *stop_ptr).
If there is no start, use 0.
If there is no stop, use the end of the string.
*/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn slice(
    mystr: *mut c_char,
    start_ptr: *mut c_int,
    stop_ptr: *mut c_int,
) -> c_int {
    let len: usize = unsafe { strlen(mystr) };

    let start: c_int;
    let stop: c_int;

    if !start_ptr.is_null() {
        start = unsafe { *start_ptr };
        // C: `if (start > len)` — `start` (int) is promoted to size_t (unsigned)
        // so negative values become very large and trigger this branch.
        if (start as usize) > len {
            let fmt = b"Error: start is off the end of the string!\n\0";
            unsafe {
                printf(fmt.as_ptr() as *const c_char);
            }
            return 1;
        }
    } else {
        start = 0;
    }

    if !stop_ptr.is_null() {
        stop = unsafe { *stop_ptr };
        if (stop as usize) > len {
            let fmt = b"Error: stop is off the end of the string!\n\0";
            unsafe {
                printf(fmt.as_ptr() as *const c_char);
            }
            return 1;
        }
        if stop <= start {
            let fmt = b"Error: stop must come after start!\n\0";
            unsafe {
                printf(fmt.as_ptr() as *const c_char);
            }
            return 1;
        }
    } else {
        stop = len as c_int;
    }

    /* char arithmetic: skip ahead `start` characters in the array */
    let fmt = b"%.*s\n\0";
    unsafe {
        printf(
            fmt.as_ptr() as *const c_char,
            stop - start,
            mystr.offset(start as isize),
        );
    }

    0
}
