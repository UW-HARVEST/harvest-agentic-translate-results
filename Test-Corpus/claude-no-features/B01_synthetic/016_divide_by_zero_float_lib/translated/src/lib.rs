use std::ffi::c_char;
use std::os::raw::c_int;
use std::ptr;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

// Format strings as null-terminated C strings.
const FMT_S_NL: &[u8] = b"%s\n\0";
const FMT_D_NL: &[u8] = b"%d\n\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(FMT_S_NL.as_ptr() as *const c_char, line);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntLine(int_number: c_int) {
    unsafe {
        printf(FMT_D_NL.as_ptr() as *const c_char, int_number);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad(data: f32) {
    // (int)(100.0 / data) — 100.0 is a double in C, so the division is done
    // in double precision, then truncated to int.
    let result: c_int = (100.0_f64 / (data as f64)) as c_int;
    unsafe {
        printIntLine(result);
    }
}

fn good_g2b() {
    let data: f32 = 2.0_f32;
    let result: c_int = (100.0_f64 / (data as f64)) as c_int;
    unsafe {
        printIntLine(result);
    }
}

fn good_b2g(data: f32) {
    // fabs() in C operates on double; data is promoted from float to double.
    if (data as f64).abs() > 0.000001 {
        let result: c_int = (100.0_f64 / (data as f64)) as c_int;
        unsafe {
            printIntLine(result);
        }
    } else {
        let msg = b"This would result in a divide by zero\0";
        unsafe {
            printLine(msg.as_ptr() as *const c_char);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn good(data: f32) {
    good_g2b();
    good_b2g(data);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(good_data: f32, bad_data: f32) {
    unsafe {
        let s1 = b"Calling good()...\0";
        printLine(s1.as_ptr() as *const c_char);
        good(good_data);
        let s2 = b"Finished good()\0";
        printLine(s2.as_ptr() as *const c_char);
        let s3 = b"Calling bad()...\0";
        printLine(s3.as_ptr() as *const c_char);
        bad(bad_data);
        let s4 = b"Finished bad()\0";
        printLine(s4.as_ptr() as *const c_char);
    }
}

// Suppress unused-import warning on platforms where `ptr` may not be used.
#[allow(dead_code)]
fn _unused() {
    let _ = ptr::null::<u8>();
}
