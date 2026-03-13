use std::ffi::{c_char, c_int};

/// Exact translation of the C `slice` function.
/// Reproduces all original behavior including implicit int-to-size_t comparison semantics.
#[unsafe(no_mangle)]
pub extern "C" fn slice(
    mystr: *mut c_char,
    start_ptr: *const c_int,
    stop_ptr: *const c_int,
) -> c_int {
    // strlen equivalent — C strlen on the raw pointer
    let len = unsafe { libc::strlen(mystr) };

    let start: c_int;
    let stop: c_int;

    if !start_ptr.is_null() {
        start = unsafe { *start_ptr };
        // C compares (int > size_t): int is converted to size_t (unsigned)
        if (start as usize) > len {
            unsafe {
                libc::printf(
                    b"Error: start is off the end of the string!\n\0".as_ptr() as *const c_char,
                );
            }
            return 1;
        }
    } else {
        start = 0;
    }

    if !stop_ptr.is_null() {
        stop = unsafe { *stop_ptr };
        // C compares (int > size_t): int converted to size_t
        if (stop as usize) > len {
            unsafe {
                libc::printf(
                    b"Error: stop is off the end of the string!\n\0".as_ptr() as *const c_char,
                );
            }
            return 1;
        }
        if stop <= start {
            unsafe {
                libc::printf(
                    b"Error: stop must come after start!\n\0".as_ptr() as *const c_char,
                );
            }
            return 1;
        }
    } else {
        // C: stop = len; (size_t truncated to int)
        stop = len as c_int;
    }

    // printf("%.*s\n", stop - start, mystr + start)
    unsafe {
        libc::printf(
            b"%.*s\n\0".as_ptr() as *const c_char,
            stop - start,
            mystr.offset(start as isize),
        );
    }

    0
}
