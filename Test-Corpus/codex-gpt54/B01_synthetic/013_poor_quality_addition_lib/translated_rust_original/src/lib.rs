use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

const STR_FMT: &[u8] = b"%s\n\0";
const INT_FMT: &[u8] = b"%d\n\0";
const CALLING_GOOD: &[u8] = b"Calling good()...\0";
const FINISHED_GOOD: &[u8] = b"Finished good()\0";
const CALLING_BAD: &[u8] = b"Calling bad()...\0";
const FINISHED_BAD: &[u8] = b"Finished bad()\0";

fn print_line(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(STR_FMT.as_ptr().cast(), line);
        }
    }
}

fn print_int_line(int_number: c_int) {
    unsafe {
        printf(INT_FMT.as_ptr().cast(), int_number);
    }
}

fn bad() {
    let int_one: c_int = 1;
    let int_two: c_int = 1;
    let int_sum: c_int = 0;

    print_int_line(int_sum);
    let _ = int_one + int_two;
    print_int_line(int_sum);
}

fn good() {
    let int_one: c_int = 1;
    let int_two: c_int = 1;
    let mut int_sum: c_int = 0;

    print_int_line(int_sum);
    int_sum = int_one + int_two;
    print_int_line(int_sum);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver() {
    print_line(CALLING_GOOD.as_ptr().cast());
    good();
    print_line(FINISHED_GOOD.as_ptr().cast());
    print_line(CALLING_BAD.as_ptr().cast());
    bad();
    print_line(FINISHED_BAD.as_ptr().cast());
}
