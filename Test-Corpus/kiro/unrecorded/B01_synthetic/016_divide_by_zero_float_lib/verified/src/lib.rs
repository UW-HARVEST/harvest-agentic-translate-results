use std::os::raw::{c_char, c_int};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn fabs(x: f64) -> f64;
}

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe { printf(b"%s\n\0".as_ptr() as *const c_char, line); }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn printIntLine(int_number: c_int) {
    unsafe { printf(b"%d\n\0".as_ptr() as *const c_char, int_number); }
}

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
    if unsafe { fabs(data as f64) } > 0.000001 {
        let result: c_int = (100.0f64 / data as f64) as c_int;
        printIntLine(result);
    } else {
        printLine(b"This would result in a divide by zero\0".as_ptr() as *const c_char);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn good(data: f32) {
    good_g2b();
    good_b2g(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(good_data: f32, bad_data: f32) {
    printLine(b"Calling good()...\0".as_ptr() as *const c_char);
    good(good_data);
    printLine(b"Finished good()\0".as_ptr() as *const c_char);
    printLine(b"Calling bad()...\0".as_ptr() as *const c_char);
    bad(bad_data);
    printLine(b"Finished bad()\0".as_ptr() as *const c_char);
}
