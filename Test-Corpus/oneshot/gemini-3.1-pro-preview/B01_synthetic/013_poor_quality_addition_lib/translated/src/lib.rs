use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

fn print_line(line: *const c_char) {
    if !line.is_null() {
        if let Ok(s) = unsafe { CStr::from_ptr(line) }.to_str() {
            println!("{}", s);
        }
    }
}

fn print_int_line(int_number: c_int) {
    println!("{}", int_number);
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
    print_line(c"Calling good()...".as_ptr());
    good();
    print_line(c"Finished good()".as_ptr());
    print_line(c"Calling bad()...".as_ptr());
    bad();
    print_line(c"Finished bad()".as_ptr());
}
