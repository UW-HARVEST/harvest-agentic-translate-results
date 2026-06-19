use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

const PRINTF_LINE_FORMAT: &[u8] = b"%s\n\0";
#[allow(dead_code)]
const HELPER_BAD_LINE: &[u8] = b"helperBad()\0";
const BAD_LINE: &[u8] = b"bad()\0";
const HELPER_GOOD_LINE: &[u8] = b"helperGood()\0";
const GOOD_LINE: &[u8] = b"good()\0";
const CALLING_GOOD_LINE: &[u8] = b"Calling good()...\0";
const FINISHED_GOOD_LINE: &[u8] = b"Finished good()\0";
const CALLING_BAD_LINE: &[u8] = b"Calling bad()...\0";
const FINISHED_BAD_LINE: &[u8] = b"Finished bad()\0";

#[inline]
fn c_str(bytes: &'static [u8]) -> *const c_char {
    bytes.as_ptr().cast()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(c_str(PRINTF_LINE_FORMAT), line);
        }
    }
}

#[allow(dead_code)]
fn helper_bad() {
    unsafe {
        printLine(c_str(HELPER_BAD_LINE));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    unsafe {
        printLine(c_str(BAD_LINE));
    }
}

fn helper_good() {
    unsafe {
        printLine(c_str(HELPER_GOOD_LINE));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    unsafe {
        printLine(c_str(GOOD_LINE));
    }
    helper_good();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver() {
    unsafe {
        printLine(c_str(CALLING_GOOD_LINE));
        good();
        printLine(c_str(FINISHED_GOOD_LINE));
        printLine(c_str(CALLING_BAD_LINE));
        bad();
        printLine(c_str(FINISHED_BAD_LINE));
    }
}
