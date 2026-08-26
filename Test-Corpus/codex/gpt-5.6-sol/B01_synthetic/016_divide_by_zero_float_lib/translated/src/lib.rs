use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn puts(string: *const c_char) -> c_int;
}

const INTEGER_FORMAT: &[u8] = b"%d\n\0";
const CALLING_GOOD: &[u8] = b"Calling good()...\0";
const FINISHED_GOOD: &[u8] = b"Finished good()\0";
const CALLING_BAD: &[u8] = b"Calling bad()...\0";
const FINISHED_BAD: &[u8] = b"Finished bad()\0";
const DIVIDE_BY_ZERO: &[u8] = b"This would result in a divide by zero\0";

fn c_double_to_int(value: f64) -> c_int {
    if !value.is_finite() || value >= (c_int::MAX as f64) + 1.0 || value < c_int::MIN as f64 {
        c_int::MIN
    } else {
        value as c_int
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            puts(line);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn printIntLine(int_number: c_int) {
    unsafe {
        printf(INTEGER_FORMAT.as_ptr().cast(), int_number);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad(data: f32) {
    let result = c_double_to_int(100.0 / f64::from(data));
    printIntLine(result);
}

#[unsafe(no_mangle)]
pub extern "C" fn good(data: f32) {
    printIntLine(50);

    if f64::from(data).abs() > 0.000001 {
        let result = c_double_to_int(100.0 / f64::from(data));
        printIntLine(result);
    } else {
        printLine(DIVIDE_BY_ZERO.as_ptr().cast());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(good_data: f32, bad_data: f32) {
    printLine(CALLING_GOOD.as_ptr().cast());
    good(good_data);
    printLine(FINISHED_GOOD.as_ptr().cast());
    printLine(CALLING_BAD.as_ptr().cast());
    bad(bad_data);
    printLine(FINISHED_BAD.as_ptr().cast());
}
