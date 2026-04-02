use std::ffi::CStr;
use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        let s = unsafe { CStr::from_ptr(line) };
        println!("{}", s.to_str().unwrap());
    }
}

#[allow(dead_code)]
fn helper_bad() {
    printLine(b"helperBad()\0".as_ptr() as *const c_char);
}

#[no_mangle]
pub extern "C" fn bad() {
    printLine(b"bad()\0".as_ptr() as *const c_char);
}

fn helper_good() {
    printLine(b"helperGood()\0".as_ptr() as *const c_char);
}

#[no_mangle]
pub extern "C" fn good() {
    printLine(b"good()\0".as_ptr() as *const c_char);
    helper_good();
}

#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn main() -> i32 {
    printLine(b"Calling good()...\0".as_ptr() as *const c_char);
    good();
    printLine(b"Finished good()\0".as_ptr() as *const c_char);
    printLine(b"Calling bad()...\0".as_ptr() as *const c_char);
    bad();
    printLine(b"Finished bad()\0".as_ptr() as *const c_char);
    0
}
