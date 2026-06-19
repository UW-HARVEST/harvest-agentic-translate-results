use std::ffi::{c_char, c_int};
use std::mem::MaybeUninit;

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

static PRINTF_LINE_FORMAT: &[u8] = b"%s\n\0";
static STRING_LITERAL: &[u8] = b"string\0";

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(PRINTF_LINE_FORMAT.as_ptr().cast(), line);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    #[allow(invalid_value)]
    unsafe {
        let data: *mut c_char = MaybeUninit::<*mut c_char>::uninit().assume_init();
        printLine(data.cast_const());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    let data = STRING_LITERAL.as_ptr().cast::<c_char>();
    printLine(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
