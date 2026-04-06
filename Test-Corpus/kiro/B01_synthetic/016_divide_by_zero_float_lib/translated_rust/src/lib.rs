use std::ffi::{c_int, CString};
use std::os::raw::c_char;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe { libc::printf(b"%s\n\0".as_ptr() as *const c_char, line) };
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn printIntLine(int_number: c_int) {
    unsafe { libc::printf(b"%d\n\0".as_ptr() as *const c_char, int_number) };
}

// Test helpers to expose private functions
pub fn print_int_line_for_test(n: c_int) { printIntLine(n); }
pub fn print_line_for_test(s: &CString) { unsafe { printLine(s.as_ptr()); } }
pub fn print_line_null_for_test() { unsafe { printLine(std::ptr::null()); } }

#[unsafe(no_mangle)]
pub extern "C" fn bad(data: f32) {
    let result: c_int = (100.0f64 / data as f64) as c_int;
    printIntLine(result);
}

fn good_g2b() {
    let data: f32 = 2.0;
    let result: c_int = (100.0f64 / data as f64) as c_int;
    printIntLine(result);
}

fn good_b2g(data: f32) {
    if (data as f64).abs() > 0.000001 {
        let result: c_int = (100.0f64 / data as f64) as c_int;
        printIntLine(result);
    } else {
        unsafe { printLine(b"This would result in a divide by zero\0".as_ptr() as *const c_char) };
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn good(data: f32) {
    good_g2b();
    good_b2g(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(good_data: f32, bad_data: f32) {
    unsafe {
        printLine(b"Calling good()...\0".as_ptr() as *const c_char);
        good(good_data);
        printLine(b"Finished good()\0".as_ptr() as *const c_char);
        printLine(b"Calling bad()...\0".as_ptr() as *const c_char);
        bad(bad_data);
        printLine(b"Finished bad()\0".as_ptr() as *const c_char);
    }
}
