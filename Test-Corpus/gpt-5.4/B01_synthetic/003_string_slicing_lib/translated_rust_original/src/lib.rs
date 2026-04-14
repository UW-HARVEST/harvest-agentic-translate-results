use std::ffi::{CStr, c_char};
use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn slice(mystr: *mut c_char, start_ptr: *mut c_int, stop_ptr: *mut c_int) -> c_int {
    if mystr.is_null() {
        return 1;
    }

    let bytes = unsafe { CStr::from_ptr(mystr.cast_const()) }.to_bytes();
    let len = bytes.len();

    let start = if start_ptr.is_null() {
        0usize
    } else {
        let value = unsafe { *start_ptr };
        if value < 0 {
            println!("Error: start is off the end of the string!");
            return 1;
        }
        let start = value as usize;
        if start > len {
            println!("Error: start is off the end of the string!");
            return 1;
        }
        start
    };

    let stop = if stop_ptr.is_null() {
        len
    } else {
        let value = unsafe { *stop_ptr };
        if value < 0 {
            println!("Error: stop is off the end of the string!");
            return 1;
        }
        let stop = value as usize;
        if stop > len {
            println!("Error: stop is off the end of the string!");
            return 1;
        }
        if stop <= start {
            println!("Error: stop must come after start!");
            return 1;
        }
        stop
    };

    let slice = &bytes[start..stop];
    println!("{}", String::from_utf8_lossy(slice));

    0
}
