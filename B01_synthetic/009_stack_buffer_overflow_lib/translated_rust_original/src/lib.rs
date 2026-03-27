use std::ffi::{c_char, c_int};

unsafe fn print_line(line: *const c_char) {
    if !line.is_null() {
        unsafe { libc::printf(b"%s\n\0".as_ptr() as *const c_char, line) };
    }
}

fn print_int_line(int_number: c_int) {
    unsafe { libc::printf(b"%d\n\0".as_ptr() as *const c_char, int_number as c_int) };
}

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    unsafe { print_line(line) };
}

#[unsafe(no_mangle)]
pub extern "C" fn printIntLine(int_number: c_int) {
    print_int_line(int_number);
}

#[unsafe(no_mangle)]
pub extern "C" fn bad(data: c_int) {
    let mut buffer: [c_int; 10] = [0; 10];
    if data >= 0 {
        // Intentional bug: no upper bound check — reproduce C behavior exactly
        unsafe { *buffer.as_mut_ptr().offset(data as isize) = 1 };
        for i in 0..10 {
            print_int_line(buffer[i]);
        }
    } else {
        unsafe { print_line(b"ERROR: Array index is negative.\0".as_ptr() as *const c_char) };
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
        unsafe { print_line(b"ERROR: Array index is negative.\0".as_ptr() as *const c_char) };
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
        unsafe { print_line(b"ERROR: Array index is out-of-bounds\0".as_ptr() as *const c_char) };
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn good(data: c_int) {
    good_g2b();
    good_b2g(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(good_data: c_int, bad_data: c_int) {
    unsafe {
        print_line(b"Calling good()...\0".as_ptr() as *const c_char);
        good(good_data);
        print_line(b"Finished good()\0".as_ptr() as *const c_char);
        print_line(b"Calling bad()...\0".as_ptr() as *const c_char);
        bad(bad_data);
        print_line(b"Finished bad()\0".as_ptr() as *const c_char);
    }
}
