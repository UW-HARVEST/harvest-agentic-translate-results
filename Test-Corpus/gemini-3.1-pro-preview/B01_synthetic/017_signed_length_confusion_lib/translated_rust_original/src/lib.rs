use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

fn print_line(line: *const c_char) {
    if !line.is_null() {
        let c_str = unsafe { CStr::from_ptr(line) };
        println!("{}", c_str.to_string_lossy());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(data: c_int) {
    let mut source = [0u8; 100];
    source[..99].fill(b'A');
    source[99] = 0;

    let mut dest = [0u8; 100];

    if data >= 0 && data < 100 {
        let data_usize = data as usize;
        dest[..data_usize].copy_from_slice(&source[..data_usize]);
        dest[data_usize] = 0;
    }

    print_line(dest.as_ptr() as *const c_char);
}