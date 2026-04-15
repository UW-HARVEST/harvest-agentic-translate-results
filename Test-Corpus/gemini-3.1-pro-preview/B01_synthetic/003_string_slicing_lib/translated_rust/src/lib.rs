use std::ffi::CStr;
use std::io::{self, Write};
use std::os::raw::{c_char, c_int};

#[unsafe(no_mangle)]
pub extern "C" fn slice(mystr: *mut c_char, start_ptr: *mut c_int, stop_ptr: *mut c_int) -> c_int {
    let c_str = unsafe { CStr::from_ptr(mystr) };
    let len = c_str.to_bytes().len();

    let start = if !start_ptr.is_null() {
        let s = unsafe { *start_ptr } as usize;
        if s > len {
            println!("Error: start is off the end of the string!");
            return 1;
        }
        s
    } else {
        0
    };

    let stop = if !stop_ptr.is_null() {
        let s = unsafe { *stop_ptr } as usize;
        if s > len {
            println!("Error: stop is off the end of the string!");
            return 1;
        }
        if s <= start {
            println!("Error: stop must come after start!");
            return 1;
        }
        s
    } else {
        len
    };

    let slice_bytes = &c_str.to_bytes()[start..stop];
    let mut stdout = io::stdout();
    let _ = stdout.write_all(slice_bytes);
    let _ = stdout.write_all(b"\n");

    0
}
