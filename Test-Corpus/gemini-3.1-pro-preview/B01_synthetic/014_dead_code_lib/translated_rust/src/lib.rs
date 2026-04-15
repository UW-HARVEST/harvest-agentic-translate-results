use std::ffi::CStr;
use std::os::raw::c_char;

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        if let Ok(s) = unsafe { CStr::from_ptr(line) }.to_str() {
            println!("{}", s);
        }
    }
}

#[allow(dead_code)]
fn helperBad() {
    printLine(c"helperBad()".as_ptr());
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    printLine(c"bad()".as_ptr());
}

fn helperGood() {
    printLine(c"helperGood()".as_ptr());
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    printLine(c"good()".as_ptr());
    helperGood();
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
