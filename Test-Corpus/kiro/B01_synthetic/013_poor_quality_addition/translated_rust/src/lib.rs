#![no_main]

use std::ffi::CStr;
use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        let s = unsafe { CStr::from_ptr(line) };
        println!("{}", s.to_str().unwrap());
    }
}

#[no_mangle]
pub extern "C" fn printIntLine(n: i32) {
    println!("{}", n);
}

#[no_mangle]
pub extern "C" fn bad() {
    let int_one: i32 = 1;
    let int_two: i32 = 1;
    let int_sum: i32 = 0;
    printIntLine(int_sum);
    let _ = int_one + int_two;
    printIntLine(int_sum);
}

#[no_mangle]
pub extern "C" fn good() {
    let int_one: i32 = 1;
    let int_two: i32 = 1;
    let mut int_sum: i32 = 0;
    printIntLine(int_sum);
    int_sum = int_one + int_two;
    printIntLine(int_sum);
}

#[export_name = "main"]
pub extern "C" fn driver_main(_argc: i32, _argv: *const *const c_char) -> i32 {
    printLine(b"Calling good()...\0".as_ptr() as *const c_char);
    good();
    printLine(b"Finished good()\0".as_ptr() as *const c_char);
    printLine(b"Calling bad()...\0".as_ptr() as *const c_char);
    bad();
    printLine(b"Finished bad()\0".as_ptr() as *const c_char);
    0
}
