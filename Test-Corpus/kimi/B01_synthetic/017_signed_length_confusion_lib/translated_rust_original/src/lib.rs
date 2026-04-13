use std::ffi::{c_char, CStr};
use std::os::raw::c_int;

fn print_line(line: *const c_char) {
    if !line.is_null() {
        let c_str = unsafe { CStr::from_ptr(line) };
        if let Ok(s) = c_str.to_str() {
            println!("{}", s);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(data: c_int) {
    let data = data as usize;
    let mut source = [b'A'; 99];
    source[98] = b'\0';
    let source_len = source.len() - 1;
    
    let mut dest = vec![0u8; 100];
    
    if data < 100 {
        let copy_len = data.min(source_len);
        dest[..copy_len].copy_from_slice(&source[..copy_len]);
        dest[copy_len] = 0;
    }
    
    let c_string = std::ffi::CString::new(&dest[..dest.iter().position(|&b| b == 0).unwrap_or(100)])
        .unwrap_or_default();
    print_line(c_string.as_ptr());
}
