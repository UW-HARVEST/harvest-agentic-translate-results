// Rust translation of c_src/src/driver.c — preserves byte-identical output.

use std::ffi::c_char;
use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        // printf("%s\n", line);
        let fmt = b"%s\n\0".as_ptr() as *const c_char;
        unsafe {
            printf(fmt, line);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn printIntLine(int_number: c_int) {
    // printf("%d\n", intNumber);
    let fmt = b"%d\n\0".as_ptr() as *const c_char;
    unsafe {
        printf(fmt, int_number);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad(data: c_int) {
    let mut buffer: [c_int; 10] = [0; 10];
    if data >= 0 {
        // Reproduce C behavior exactly, including potential out-of-bounds write
        // when data >= 10. Use raw pointer arithmetic to mirror C semantics.
        unsafe {
            let p = buffer.as_mut_ptr().offset(data as isize);
            *p = 1;
        }
        for i in 0..10 {
            printIntLine(buffer[i]);
        }
    } else {
        let msg = b"ERROR: Array index is negative.\0".as_ptr() as *const c_char;
        printLine(msg);
    }
}

fn good_g2b() {
    let data: c_int = 7;
    let mut buffer: [c_int; 10] = [0; 10];
    if data >= 0 {
        buffer[data as usize] = 1;
        for i in 0..10 {
            printIntLine(buffer[i]);
        }
    } else {
        let msg = b"ERROR: Array index is negative.\0".as_ptr() as *const c_char;
        printLine(msg);
    }
}

fn good_b2g(data: c_int) {
    let mut buffer: [c_int; 10] = [0; 10];
    if data >= 0 && data < 10 {
        buffer[data as usize] = 1;
        for i in 0..10 {
            printIntLine(buffer[i]);
        }
    } else {
        let msg = b"ERROR: Array index is out-of-bounds\0".as_ptr() as *const c_char;
        printLine(msg);
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
    let finished_good = b"Finished good()\0".as_ptr() as *const c_char;
    let calling_bad = b"Calling bad()...\0".as_ptr() as *const c_char;
    let finished_bad = b"Finished bad()\0".as_ptr() as *const c_char;

    printLine(calling_good);
    good(good_data);
    printLine(finished_good);
    printLine(calling_bad);
    bad(bad_data);
    printLine(finished_bad);
}
