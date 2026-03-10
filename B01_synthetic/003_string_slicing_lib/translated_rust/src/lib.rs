use std::ffi::{c_char, c_int};

#[unsafe(no_mangle)]
pub extern "C" fn slice(
    mystr: *mut c_char,
    start_ptr: *mut c_int,
    stop_ptr: *mut c_int,
) -> c_int {
    unsafe {
        let len = libc::strlen(mystr) as c_int;

        let start = if !start_ptr.is_null() {
            let s = *start_ptr;
            if s > len {
                libc::printf(b"Error: start is off the end of the string!\n\0".as_ptr() as *const c_char);
                return 1;
            }
            s
        } else {
            0
        };

        let stop = if !stop_ptr.is_null() {
            let s = *stop_ptr;
            if s > len {
                libc::printf(b"Error: stop is off the end of the string!\n\0".as_ptr() as *const c_char);
                return 1;
            }
            if s <= start {
                libc::printf(b"Error: stop must come after start!\n\0".as_ptr() as *const c_char);
                return 1;
            }
            s
        } else {
            len
        };

        libc::printf(b"%.*s\n\0".as_ptr() as *const c_char, stop - start, mystr.offset(start as isize));

        0
    }
}
