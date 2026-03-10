use std::ffi::c_float;
use std::os::raw::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn fabs(x: f64) -> f64;
}

unsafe fn print_line(line: *const c_char) {
    if !line.is_null() {
        unsafe { printf(b"%s\n\0".as_ptr() as *const c_char, line) };
    }
}

fn print_int_line(int_number: c_int) {
    unsafe { printf(b"%d\n\0".as_ptr() as *const c_char, int_number) };
}

fn bad(data: c_float) {
    let result = (100.0f64 / data as f64) as c_int;
    print_int_line(result);
}

fn good_g2b() {
    let data: c_float = 2.0;
    let result = (100.0f64 / data as f64) as c_int;
    print_int_line(result);
}

fn good_b2g(data: c_float) {
    if unsafe { fabs(data as f64) } > 0.000001 {
        let result = (100.0f64 / data as f64) as c_int;
        print_int_line(result);
    } else {
        unsafe { print_line(b"This would result in a divide by zero\0".as_ptr() as *const c_char) };
    }
}

fn good(data: c_float) {
    good_g2b();
    good_b2g(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(good_data: c_float, bad_data: c_float) {
    unsafe {
        print_line(b"Calling good()...\0".as_ptr() as *const c_char);
    }
    good(good_data);
    unsafe {
        print_line(b"Finished good()\0".as_ptr() as *const c_char);
        print_line(b"Calling bad()...\0".as_ptr() as *const c_char);
    }
    bad(bad_data);
    unsafe {
        print_line(b"Finished bad()\0".as_ptr() as *const c_char);
    }
}
