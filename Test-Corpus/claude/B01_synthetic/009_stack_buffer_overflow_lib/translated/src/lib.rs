use std::ffi::c_char;
use std::ffi::c_int;

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

fn print_line(line: *const c_char) {
    if !line.is_null() {
        // Equivalent to printf("%s\n", line);
        let fmt = b"%s\n\0".as_ptr() as *const c_char;
        unsafe {
            printf(fmt, line);
        }
    }
}

fn print_int_line(int_number: c_int) {
    // Equivalent to printf("%d\n", intNumber);
    let fmt = b"%d\n\0".as_ptr() as *const c_char;
    unsafe {
        printf(fmt, int_number);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    print_line(line);
}

#[unsafe(no_mangle)]
pub extern "C" fn printIntLine(int_number: c_int) {
    print_int_line(int_number);
}

#[unsafe(no_mangle)]
pub extern "C" fn bad(data: c_int) {
    let mut buffer: [c_int; 10] = [0; 10];
    if data >= 0 {
        // Reproduce C behavior: write to buffer[data] without upper-bound check.
        // For in-bounds indices, this matches C exactly. Out-of-bounds writes
        // would be undefined behavior in the C original.
        unsafe {
            let p = buffer.as_mut_ptr().offset(data as isize);
            *p = 1;
        }
        for i in 0..10 {
            print_int_line(buffer[i]);
        }
    } else {
        let msg = b"ERROR: Array index is negative.\0".as_ptr() as *const c_char;
        print_line(msg);
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
        let msg = b"ERROR: Array index is negative.\0".as_ptr() as *const c_char;
        print_line(msg);
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
        let msg = b"ERROR: Array index is out-of-bounds\0".as_ptr() as *const c_char;
        print_line(msg);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn good(data: c_int) {
    good_g2b();
    good_b2g(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(good_data: c_int, bad_data: c_int) {
    let calling_good = b"Calling good()...\0".as_ptr() as *const c_char;
    print_line(calling_good);
    good(good_data);
    let finished_good = b"Finished good()\0".as_ptr() as *const c_char;
    print_line(finished_good);
    let calling_bad = b"Calling bad()...\0".as_ptr() as *const c_char;
    print_line(calling_bad);
    bad(bad_data);
    let finished_bad = b"Finished bad()\0".as_ptr() as *const c_char;
    print_line(finished_bad);
}
