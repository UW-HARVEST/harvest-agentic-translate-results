// Translation of c_src/src/driver.c to Rust.
// Uses libc printf via FFI to ensure byte-identical output.

use std::ffi::c_char;
use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

// printf format strings as C-style null-terminated bytes
const FMT_STRING_NL: &[u8] = b"%s\n\0";
const FMT_INT_NL: &[u8] = b"%d\n\0";

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(FMT_STRING_NL.as_ptr() as *const c_char, line);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn printIntLine(int_number: c_int) {
    unsafe {
        printf(FMT_INT_NL.as_ptr() as *const c_char, int_number);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad(data: c_int) {
    let mut buffer: [c_int; 10] = [0; 10];
    if data >= 0 {
        // Reproduce the C buffer overflow vulnerability: no upper-bound check.
        // We write to buffer[data] regardless of whether data >= 10.
        // To preserve the bug while remaining sound in safe Rust, use raw pointer write.
        unsafe {
            let p = buffer.as_mut_ptr().offset(data as isize);
            *p = 1;
        }
        // Print the array values
        for i in 0..10 {
            printIntLine(buffer[i]);
        }
    } else {
        let msg = b"ERROR: Array index is negative.\0";
        printLine(msg.as_ptr() as *const c_char);
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
        let msg = b"ERROR: Array index is negative.\0";
        printLine(msg.as_ptr() as *const c_char);
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
        let msg = b"ERROR: Array index is out-of-bounds\0";
        printLine(msg.as_ptr() as *const c_char);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn good(data: c_int) {
    good_g2b();
    good_b2g(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(good_data: c_int, bad_data: c_int) {
    let calling_good = b"Calling good()...\0";
    let finished_good = b"Finished good()\0";
    let calling_bad = b"Calling bad()...\0";
    let finished_bad = b"Finished bad()\0";

    printLine(calling_good.as_ptr() as *const c_char);
    good(good_data);
    printLine(finished_good.as_ptr() as *const c_char);
    printLine(calling_bad.as_ptr() as *const c_char);
    bad(bad_data);
    printLine(finished_bad.as_ptr() as *const c_char);
}
