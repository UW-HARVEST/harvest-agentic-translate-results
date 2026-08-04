use std::ffi::{CStr, c_char};
use std::os::raw::c_void;

unsafe fn print_line(line: *const c_char) {
    if !line.is_null() {
        let c_str = unsafe { CStr::from_ptr(line) };
        if let Ok(s) = c_str.to_str() {
            println!("{}", s);
        }
    }
}

fn helper_bad() {
    unsafe {
        print_line("helperBad()\0".as_ptr() as *const c_char);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    unsafe {
        print_line("bad()\0".as_ptr() as *const c_char);
    }
}

fn helper_good() {
    unsafe {
        print_line("helperGood()\0".as_ptr() as *const c_char);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    unsafe {
        print_line("good()\0".as_ptr() as *const c_char);
    }
    helper_good();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver() {
    unsafe {
        print_line("Calling good()...\0".as_ptr() as *const c_char);
    }
    good();
    unsafe {
        print_line("Finished good()\0".as_ptr() as *const c_char);
        print_line("Calling bad()...\0".as_ptr() as *const c_char);
    }
    bad();
    unsafe {
        print_line("Finished bad()\0".as_ptr() as *const c_char);
    }
}