use std::ffi::{c_char, c_int};
#[cfg(not(test))]
use std::ptr;

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn puts(string: *const c_char) -> c_int;
}

const INTEGER_FORMAT: &[u8] = b"%d\n\0";
#[cfg(not(test))]
const CALLING_GOOD: &[u8] = b"Calling good()...\0";
#[cfg(not(test))]
const FINISHED_GOOD: &[u8] = b"Finished good()\0";
#[cfg(not(test))]
const CALLING_BAD: &[u8] = b"Calling bad()...\0";
#[cfg(not(test))]
const FINISHED_BAD: &[u8] = b"Finished bad()\0";

unsafe fn print_line_impl(line: *const c_char) {
    if !line.is_null() {
        puts(line);
    }
}

unsafe fn print_int_line_impl(number: c_int) {
    printf(INTEGER_FORMAT.as_ptr().cast(), number);
}

#[no_mangle]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    print_line_impl(line);
}

#[no_mangle]
pub unsafe extern "C" fn printIntLine(int_number: c_int) {
    print_int_line_impl(int_number);
}

#[no_mangle]
pub unsafe extern "C" fn bad() {
    let int_one = 1;
    let int_two = 1;
    let int_sum = 0;
    print_int_line_impl(int_sum);
    let _ = int_one + int_two;
    print_int_line_impl(int_sum);
}

#[no_mangle]
pub unsafe extern "C" fn good() {
    let int_one = 1;
    let int_two = 1;
    let mut int_sum = 0;
    print_int_line_impl(int_sum);
    int_sum = int_one + int_two;
    print_int_line_impl(int_sum);
}

#[cfg(not(test))]
unsafe fn run_impl() {
    print_line_impl(CALLING_GOOD.as_ptr().cast());
    good();
    print_line_impl(FINISHED_GOOD.as_ptr().cast());
    print_line_impl(CALLING_BAD.as_ptr().cast());
    bad();
    print_line_impl(FINISHED_BAD.as_ptr().cast());
}

#[no_mangle]
#[cfg(not(test))]
pub unsafe extern "C" fn main(_argc: c_int, _argv: *mut *mut c_char) -> c_int {
    run_impl();
    0
}

#[cfg(not(test))]
pub fn run() {
    unsafe {
        main(0, ptr::null_mut());
    }
}
