use std::ffi::c_int;
use std::os::raw::c_char;

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

fn print_line(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(b"%s\n\0".as_ptr() as *const c_char, line);
        }
    }
}

fn print_int_line(int_number: c_int) {
    unsafe {
        printf(b"%d\n\0".as_ptr() as *const c_char, int_number);
    }
}

fn bad(data: c_int) {
    let mut buffer: [c_int; 10] = [0; 10];
    if data >= 0 {
        // Reproduce the C behavior exactly: no upper-bound check.
        // For data in [0,10) this is a normal write; for data outside the
        // range this is undefined behavior (matching the original C bug).
        unsafe {
            let p = buffer.as_mut_ptr().offset(data as isize);
            *p = 1;
        }
        // Print the array values
        for i in 0..10 {
            print_int_line(buffer[i]);
        }
    } else {
        print_line(b"ERROR: Array index is negative.\0".as_ptr() as *const c_char);
    }
}

fn good_g2b() {
    let data: c_int = 7;
    let mut buffer: [c_int; 10] = [0; 10];
    if data >= 0 {
        buffer[data as usize] = 1;
        // Print the array values
        for i in 0..10 {
            print_int_line(buffer[i]);
        }
    } else {
        print_line(b"ERROR: Array index is negative.\0".as_ptr() as *const c_char);
    }
}

fn good_b2g(data: c_int) {
    let mut buffer: [c_int; 10] = [0; 10];
    if data >= 0 && data < 10 {
        buffer[data as usize] = 1;
        // Print the array values
        for i in 0..10 {
            print_int_line(buffer[i]);
        }
    } else {
        print_line(b"ERROR: Array index is out-of-bounds\0".as_ptr() as *const c_char);
    }
}

fn good(data: c_int) {
    good_g2b();
    good_b2g(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(good_data: c_int, bad_data: c_int) {
    print_line(b"Calling good()...\0".as_ptr() as *const c_char);
    good(good_data);
    print_line(b"Finished good()\0".as_ptr() as *const c_char);
    print_line(b"Calling bad()...\0".as_ptr() as *const c_char);
    bad(bad_data);
    print_line(b"Finished bad()\0".as_ptr() as *const c_char);
}
