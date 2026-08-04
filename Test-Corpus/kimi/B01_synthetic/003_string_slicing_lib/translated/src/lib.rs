use std::ffi::{CStr, c_char, c_int};
use std::os::raw::c_int as RawCInt;

#[unsafe(no_mangle)]
pub extern "C" fn slice(mystr: *mut c_char, start_ptr: *mut c_int, stop_ptr: *mut c_int) -> c_int {
    if mystr.is_null() {
        return 1;
    }

    let c_str = unsafe { CStr::from_ptr(mystr) };
    let bytes = c_str.to_bytes();
    let len = bytes.len();

    let start = if !start_ptr.is_null() {
        let s = unsafe { *start_ptr };
        if s as usize > len {
            eprintln!("Error: start is off the end of the string!");
            return 1;
        }
        s as usize
    } else {
        0
    };

    let stop = if !stop_ptr.is_null() {
        let s = unsafe { *stop_ptr };
        if s as usize > len {
            eprintln!("Error: stop is off the end of the string!");
            return 1;
        }
        if s as usize <= start {
            eprintln!("Error: stop must come after start!");
            return 1;
        }
        s as usize
    } else {
        len
    };

    let slice = &bytes[start..stop];
    if let Ok(s) = std::str::from_utf8(slice) {
        println!("{}", s);
    }

    0
}