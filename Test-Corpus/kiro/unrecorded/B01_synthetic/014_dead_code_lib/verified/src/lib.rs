use std::ffi::{c_char, CStr};

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        let s = unsafe { CStr::from_ptr(line) };
        println!("{}", s.to_str().unwrap_or(""));
    }
}

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
