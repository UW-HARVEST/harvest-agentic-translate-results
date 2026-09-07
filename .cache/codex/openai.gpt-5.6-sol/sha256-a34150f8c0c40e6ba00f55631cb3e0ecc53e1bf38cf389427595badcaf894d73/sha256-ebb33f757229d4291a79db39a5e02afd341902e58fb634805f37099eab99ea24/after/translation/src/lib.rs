use std::ffi::{c_char, c_float, c_int};

const STRING_FORMAT: &[u8] = b"%s\n\0";
const INTEGER_FORMAT: &[u8] = b"%d\n\0";
const DIVIDE_BY_ZERO_MESSAGE: &[u8] = b"This would result in a divide by zero\0";
const CALLING_GOOD_MESSAGE: &[u8] = b"Calling good()...\0";
const FINISHED_GOOD_MESSAGE: &[u8] = b"Finished good()\0";
const CALLING_BAD_MESSAGE: &[u8] = b"Calling bad()...\0";
const FINISHED_BAD_MESSAGE: &[u8] = b"Finished bad()\0";

unsafe extern "C" {
    #[link_name = "printf"]
    fn c_printf(format: *const c_char, ...) -> c_int;
}

#[inline]
fn c_double_to_int(value: f64) -> c_int {
    // GCC's x86-64 conversion yields INT_MIN for NaN and positive overflow.
    if value.is_nan() || value >= 2_147_483_648.0 {
        c_int::MIN
    } else {
        value as c_int
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            c_printf(STRING_FORMAT.as_ptr().cast(), line);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn printIntLine(int_number: c_int) {
    unsafe {
        c_printf(INTEGER_FORMAT.as_ptr().cast(), int_number);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad(data: c_float) {
    let result = c_double_to_int(100.0 / f64::from(data));
    printIntLine(result);
}

fn good_g2b() {
    let data = 2.0_f32;
    let result = c_double_to_int(100.0 / f64::from(data));
    printIntLine(result);
}

fn good_b2g(data: f32) {
    if f64::from(data).abs() > 0.000001 {
        let result = c_double_to_int(100.0 / f64::from(data));
        printIntLine(result);
    } else {
        unsafe {
            printLine(DIVIDE_BY_ZERO_MESSAGE.as_ptr().cast());
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn good(data: c_float) {
    good_g2b();
    good_b2g(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(good_data: c_float, bad_data: c_float) {
    unsafe {
        printLine(CALLING_GOOD_MESSAGE.as_ptr().cast());
    }
    good(good_data);
    unsafe {
        printLine(FINISHED_GOOD_MESSAGE.as_ptr().cast());
        printLine(CALLING_BAD_MESSAGE.as_ptr().cast());
    }
    bad(bad_data);
    unsafe {
        printLine(FINISHED_BAD_MESSAGE.as_ptr().cast());
    }
}
