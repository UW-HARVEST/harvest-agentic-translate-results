use std::ffi::{c_char, CStr};
use std::io::{self, Write};

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        let bytes = unsafe { CStr::from_ptr(line) }.to_bytes();
        let mut stdout = io::stdout().lock();
        let _ = stdout.write_all(bytes);
        let _ = stdout.write_all(b"\n");
    }
}

#[allow(dead_code)]
fn helper_bad() {
    printLine(c"helperBad()".as_ptr());
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    printLine(c"bad()".as_ptr());
}

fn helper_good() {
    printLine(c"helperGood()".as_ptr());
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    printLine(c"good()".as_ptr());
    helper_good();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver() {
    printLine(c"Calling good()...".as_ptr());
    good();
    printLine(c"Finished good()".as_ptr());
    printLine(c"Calling bad()...".as_ptr());
    bad();
    printLine(c"Finished bad()".as_ptr());
}
