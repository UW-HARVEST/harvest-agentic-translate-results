use std::ffi::{c_char, c_double, c_float, c_int};

#[allow(non_snake_case)]
unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

const STRING_LINE_FORMAT: &[u8] = b"%s\n\0";
const INT_LINE_FORMAT: &[u8] = b"%d\n\0";
const CALLING_GOOD: &[u8] = b"Calling good()...\0";
const FINISHED_GOOD: &[u8] = b"Finished good()\0";
const CALLING_BAD: &[u8] = b"Calling bad()...\0";
const FINISHED_BAD: &[u8] = b"Finished bad()\0";
const DIVIDE_BY_ZERO: &[u8] = b"This would result in a divide by zero\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(STRING_LINE_FORMAT.as_ptr().cast(), line);
        }
    }
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub unsafe extern "C" fn printIntLine(intNumber: c_int) {
    unsafe {
        printf(INT_LINE_FORMAT.as_ptr().cast(), intNumber);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad(data: c_float) {
    let result = divide_as_c_int(data);
    unsafe {
        printIntLine(result);
    }
}

fn good_g2b() {
    let data: c_float = 2.0;
    let result = divide_as_c_int(data);
    unsafe {
        printIntLine(result);
    }
}

fn good_b2g(data: c_float) {
    if (data as c_double).abs() > 0.000001 {
        let result = divide_as_c_int(data);
        unsafe {
            printIntLine(result);
        }
    } else {
        unsafe {
            printLine(DIVIDE_BY_ZERO.as_ptr().cast());
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn good(data: c_float) {
    good_g2b();
    good_b2g(data);
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub unsafe extern "C" fn driver(goodData: c_float, badData: c_float) {
    unsafe {
        printLine(CALLING_GOOD.as_ptr().cast());
        good(goodData);
        printLine(FINISHED_GOOD.as_ptr().cast());
        printLine(CALLING_BAD.as_ptr().cast());
        bad(badData);
        printLine(FINISHED_BAD.as_ptr().cast());
    }
}

fn divide_as_c_int(data: c_float) -> c_int {
    let value = 100.0f64 / data as c_double;
    unsafe { value.to_int_unchecked::<c_int>() }
}
