use std::ffi::{c_char, CStr};
use std::os::raw::c_int;

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

fn bad(data: c_int) {
    let mut buffer = [0; 10];
    if data >= 0 {
        let idx = data as usize;
        if idx < buffer.len() {
            buffer[idx] = 1;
        }
        for i in 0..10 {
            print_int_line(buffer[i]);
        }
    } else {
        print_line("ERROR: Array index is negative.".as_ptr() as *const c_char);
    }
}

fn good_g2b() {
    let data: c_int = 7;
    let mut buffer = [0; 10];
    if data >= 0 {
        let idx = data as usize;
        if idx < buffer.len() {
            buffer[idx] = 1;
        }
        for i in 0..10 {
            print_int_line(buffer[i]);
        }
    } else {
        print_line("ERROR: Array index is negative.".as_ptr() as *const c_char);
    }
}

fn good_b2g(data: c_int) {
    let mut buffer = [0; 10];
    if data >= 0 && data < 10 {
        let idx = data as usize;
        buffer[idx] = 1;
        for i in 0..10 {
            print_int_line(buffer[i]);
        }
    } else {
        print_line("ERROR: Array index is out-of-bounds".as_ptr() as *const c_char);
    }
}

fn good(data: c_int) {
    good_g2b();
    good_b2g(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(good_data: c_int, bad_data: c_int) {
    print_line("Calling good()...".as_ptr() as *const c_char);
    good(good_data);
    print_line("Finished good()".as_ptr() as *const c_char);
    print_line("Calling bad()...".as_ptr() as *const c_char);
    bad(bad_data);
    print_line("Finished bad()".as_ptr() as *const c_char);
}