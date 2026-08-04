use std::ffi::c_char;
use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        let fmt = b"%s\n\0".as_ptr() as *const c_char;
        unsafe {
            printf(fmt, line);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn printIntLine(int_number: c_int) {
    let fmt = b"%d\n\0".as_ptr() as *const c_char;
    unsafe {
        printf(fmt, int_number);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    let int_one: c_int = 1;
    let int_two: c_int = 1;
    let int_sum: c_int = 0;
    printIntLine(int_sum);
    // The C code computes intOne + intTwo but discards the result (a bug).
    // Reproduce this exactly: compute and discard.
    let _ = int_one.wrapping_add(int_two);
    printIntLine(int_sum);
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    let int_one: c_int = 1;
    let int_two: c_int = 1;
    let mut int_sum: c_int = 0;
    printIntLine(int_sum);
    int_sum = int_one.wrapping_add(int_two);
    printIntLine(int_sum);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver() {
    printLine(b"Calling good()...\0".as_ptr() as *const c_char);
    good();
    printLine(b"Finished good()\0".as_ptr() as *const c_char);
    printLine(b"Calling bad()...\0".as_ptr() as *const c_char);
    bad();
    printLine(b"Finished bad()\0".as_ptr() as *const c_char);
}
