use std::os::raw::{c_char, c_int};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn scanf(fmt: *const c_char, ...) -> c_int;
}

#[no_mangle]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(b"%s\n\0".as_ptr() as *const c_char, line);
        }
    }
}

#[no_mangle]
pub extern "C" fn printHexCharLine(char_hex: c_char) {
    let as_int = char_hex as i32;
    let as_uint = as_int as u32;
    unsafe {
        printf(b"%02x\n\0".as_ptr() as *const c_char, as_uint);
    }
}

#[no_mangle]
pub extern "C" fn bad() {
    let data: i8 = i8::MAX;
    if data > 0 {
        let result: i8 = data.wrapping_mul(2);
        printHexCharLine(result);
    }
}

fn good_g2b() {
    let data: i8 = 2;
    if data > 0 {
        let result: i8 = data.wrapping_mul(2);
        printHexCharLine(result);
    }
}

fn good_b2g() {
    let data: i8 = i8::MAX;
    if data > 0 {
        if data < (i8::MAX / 2) {
            let result: i8 = data.wrapping_mul(2);
            printHexCharLine(result);
        } else {
            printLine(b"data value is too large to perform arithmetic safely.\0".as_ptr() as *const c_char);
        }
    }
}

#[no_mangle]
pub extern "C" fn good() {
    good_g2b();
    good_b2g();
}

/// Exported as `main` for the cdylib. The bin target has its own Rust main.
#[cfg(not(feature = "_bin"))]
#[export_name = "main"]
pub extern "C" fn c_main() -> c_int {
    let mut x: c_int = 0;
    unsafe {
        scanf(b"%d\0".as_ptr() as *const c_char, &mut x as *mut c_int);
    }
    if x != 0 {
        good();
    } else {
        bad();
    }
    0
}
