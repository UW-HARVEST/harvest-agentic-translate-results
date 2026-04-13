use std::ffi::{CStr, c_char};
use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn driver() {
    print_line("Calling good()...");
    good();
    print_line("Finished good()");
    print_line("Calling bad()...");
    bad();
    print_line("Finished bad()");
}

fn print_line(line: *const c_char) {
    if !line.is_null() {
        let c_str = unsafe { CStr::from_ptr(line) };
        if let Ok(s) = c_str.to_str() {
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
