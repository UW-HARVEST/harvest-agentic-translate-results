use std::ffi::{c_char, c_int};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
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
pub extern "C" fn bad(data: c_int) {
    let mut buffer = [0i32; 10];
    if data >= 0 {
        // Reproduce C bug: no upper-bound check, allows out-of-bounds write
        unsafe { *buffer.as_mut_ptr().offset(data as isize) = 1; }
        for i in 0..10 {
            printIntLine(buffer[i]);
        }
    } else {
        printLine(b"ERROR: Array index is negative.\0".as_ptr() as *const c_char);
    }
}

fn good_g2b() {
    let data: c_int = 7;
    let mut buffer = [0i32; 10];
    if data >= 0 {
        buffer[data as usize] = 1;
        for i in 0..10 {
            printIntLine(buffer[i]);
        }
    } else {
        printLine(b"ERROR: Array index is negative.\0".as_ptr() as *const c_char);
    }
}

fn good_b2g(data: c_int) {
    let mut buffer = [0i32; 10];
    if data >= 0 && data < 10 {
        buffer[data as usize] = 1;
        for i in 0..10 {
            printIntLine(buffer[i]);
        }
    } else {
        printLine(b"ERROR: Array index is out-of-bounds\0".as_ptr() as *const c_char);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn good(data: c_int) {
    good_g2b();
    good_b2g(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(good_data: c_int, bad_data: c_int) {
    printLine(b"Calling good()...\0".as_ptr() as *const c_char);
    good(good_data);
    printLine(b"Finished good()\0".as_ptr() as *const c_char);
    printLine(b"Calling bad()...\0".as_ptr() as *const c_char);
    bad(bad_data);
    printLine(b"Finished bad()\0".as_ptr() as *const c_char);
}
