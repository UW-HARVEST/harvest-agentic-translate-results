use std::ffi::{c_char, c_int, CStr};

unsafe fn print_line(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            let s = CStr::from_ptr(line);
            if let Ok(rs) = s.to_str() {
                println!("{}", rs);
            }
        }
    }
}

unsafe fn helper_bad() -> *const c_char {
    let char_string: [u8; 17] = *b"helperBad string\0";
    char_string.as_ptr() as *const c_char
}

fn helper_good1() -> *const c_char {
    static CHAR_STRING: &[u8; 19] = b"helperGood1 string\0";
    CHAR_STRING.as_ptr() as *const c_char
}

unsafe fn bad() {
    unsafe {
        print_line(helper_bad());
    }
}

fn good() {
    unsafe {
        print_line(helper_good1());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        unsafe {
            bad();
        }
    }
}
