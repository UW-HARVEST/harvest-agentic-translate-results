use std::ffi::CString;
use std::os::raw::{c_char, c_int};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

fn print_line(line: *const c_char) {
    if !line.is_null() {
        let fmt = CString::new("%s\n").unwrap();
        unsafe {
            printf(fmt.as_ptr(), line);
        }
    }
}

fn print_line_str(s: &str) {
    let cs = CString::new(s).unwrap();
    print_line(cs.as_ptr());
}

fn print_int_line(int_number: c_int) {
    let fmt = CString::new("%d\n").unwrap();
    unsafe {
        printf(fmt.as_ptr(), int_number);
    }
}

fn bad(data: c_int) {
    let mut buffer: [c_int; 10] = [0; 10];
    if data >= 0 {
        // Reproduce C behavior exactly, including out-of-bounds writes.
        // Use raw pointer write to mimic the C semantics.
        unsafe {
            let p = buffer.as_mut_ptr().offset(data as isize);
            *p = 1;
        }
        for i in 0..10 {
            print_int_line(buffer[i]);
        }
    } else {
        print_line_str("ERROR: Array index is negative.");
    }
}

fn good_g2b() {
    let data: c_int = 7;
    let mut buffer: [c_int; 10] = [0; 10];
    if data >= 0 {
        buffer[data as usize] = 1;
        for i in 0..10 {
            print_int_line(buffer[i]);
        }
    } else {
        print_line_str("ERROR: Array index is negative.");
    }
}

fn good_b2g(data: c_int) {
    let mut buffer: [c_int; 10] = [0; 10];
    if data >= 0 && data < 10 {
        buffer[data as usize] = 1;
        for i in 0..10 {
            print_int_line(buffer[i]);
        }
    } else {
        print_line_str("ERROR: Array index is out-of-bounds");
    }
}

fn good(data: c_int) {
    good_g2b();
    good_b2g(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(good_data: c_int, bad_data: c_int) {
    print_line_str("Calling good()...");
    good(good_data);
    print_line_str("Finished good()");
    print_line_str("Calling bad()...");
    bad(bad_data);
    print_line_str("Finished bad()");
}
