// Copyright 2025 MIT Lincoln Laboratory
// Translation to Rust preserving byte-identical output.

use std::ffi::c_char;
use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

fn print_line(line: *const c_char) {
    if !line.is_null() {
        // "%s\n"
        let fmt = b"%s\n\0".as_ptr() as *const c_char;
        unsafe {
            printf(fmt, line);
        }
    }
}

fn print_int_line(int_number: c_int) {
    // "%d\n"
    let fmt = b"%d\n\0".as_ptr() as *const c_char;
    unsafe {
        printf(fmt, int_number);
    }
}

fn bad() {
    let int_one: c_int = 1;
    let int_two: c_int = 1;
    let int_sum: c_int = 0;
    print_int_line(int_sum);
    // The original C statement `intOne + intTwo;` has no effect.
    // Reproduce its no-op semantics: compute and discard.
    let _ = int_one.wrapping_add(int_two);
    print_int_line(int_sum);
}

fn good() {
    let int_one: c_int = 1;
    let int_two: c_int = 1;
    let mut int_sum: c_int = 0;
    print_int_line(int_sum);
    int_sum = int_one.wrapping_add(int_two);
    print_int_line(int_sum);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver() {
    let calling_good = b"Calling good()...\0".as_ptr() as *const c_char;
    let finished_good = b"Finished good()\0".as_ptr() as *const c_char;
    let calling_bad = b"Calling bad()...\0".as_ptr() as *const c_char;
    let finished_bad = b"Finished bad()\0".as_ptr() as *const c_char;

    print_line(calling_good);
    good();
    print_line(finished_good);
    print_line(calling_bad);
    bad();
    print_line(finished_bad);
}
