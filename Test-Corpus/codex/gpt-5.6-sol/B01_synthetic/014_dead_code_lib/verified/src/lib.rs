use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn puts(line: *const c_char) -> c_int;
}

const HELPER_BAD: &[u8] = b"helperBad()\0";
const BAD: &[u8] = b"bad()\0";
const HELPER_GOOD: &[u8] = b"helperGood()\0";
const GOOD: &[u8] = b"good()\0";
const CALLING_GOOD: &[u8] = b"Calling good()...\0";
const FINISHED_GOOD: &[u8] = b"Finished good()\0";
const CALLING_BAD: &[u8] = b"Calling bad()...\0";
const FINISHED_BAD: &[u8] = b"Finished bad()\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            puts(line);
        }
    }
}

#[allow(dead_code)]
fn helper_bad() {
    unsafe {
        printLine(HELPER_BAD.as_ptr().cast());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    unsafe {
        printLine(BAD.as_ptr().cast());
    }
}

fn helper_good() {
    unsafe {
        printLine(HELPER_GOOD.as_ptr().cast());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    unsafe {
        printLine(GOOD.as_ptr().cast());
    }
    helper_good();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver() {
    unsafe {
        printLine(CALLING_GOOD.as_ptr().cast());
    }
    good();
    unsafe {
        printLine(FINISHED_GOOD.as_ptr().cast());
        printLine(CALLING_BAD.as_ptr().cast());
    }
    bad();
    unsafe {
        printLine(FINISHED_BAD.as_ptr().cast());
    }
}
