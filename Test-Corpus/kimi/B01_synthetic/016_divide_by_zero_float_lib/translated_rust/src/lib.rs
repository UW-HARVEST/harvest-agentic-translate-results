use std::ffi::{c_char, CStr, CString};
use std::os::raw::c_float;

#[unsafe(no_mangle)]
pub extern "C" fn driver(good_data: c_float, bad_data: c_float) {
    unsafe {
        print_line("Calling good()...");
        good(good_data);
        print_line("Finished good()");
        print_line("Calling bad()...");
        bad(bad_data);
        print_line("Finished bad()");
    }
}

unsafe fn print_line(line: *const c_char) {
    if !line.is_null() {
        let c_str = CStr::from_ptr(line);
        if let Ok(s) = c_str.to_str() {
            println!("{}", s);
        }
    }
}

fn print_int_line(int_number: i32) {
    println!("{}", int_number);
}

unsafe fn bad(data: c_float) {
    let result = (100.0 / data) as i32;
    print_int_line(result);
}

fn good_g2b() {
    let data: f32 = 2.0;
    let result = (100.0 / data) as i32;
    print_int_line(result);
}

fn good_b2g(data: c_float) {
    if data.abs() > 0.000001 {
        let result = (100.0 / data) as i32;
        print_int_line(result);
    } else {
        let msg = CString::new("This would result in a divide by zero").unwrap();
        unsafe {
            print_line(msg.as_ptr());
        }
    }
}

unsafe fn good(data: c_float) {
    good_g2b();
    good_b2g(data);
}