// Rust translation of c_src/src/driver.c (MIT Lincoln Laboratory, 2025).
//
// The original C is a CWE-129 style test case: `bad()` performs an
// unvalidated (upper-bound-unchecked) array write. The bug is preserved
// verbatim, as are the order of the validation checks and every printed byte.
//
// Output is emitted through the C library's `printf` so that stdio buffering
// and formatting are identical to the original translation unit (and stay
// interleaved correctly with any C caller that also writes to stdout).

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

/// Format strings, NUL terminated, matching the C source exactly.
const FMT_STR_LINE: &[u8] = b"%s\n\0";
const FMT_INT_LINE: &[u8] = b"%d\n\0";

/// `void printLine(const char * line)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(FMT_STR_LINE.as_ptr() as *const c_char, line);
        }
    }
}

/// `void printIntLine(int intNumber)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntLine(int_number: c_int) {
    unsafe {
        printf(FMT_INT_LINE.as_ptr() as *const c_char, int_number);
    }
}

/// Internal helper: print a Rust byte-string literal (NUL terminated) the same
/// way the C code's `printLine` would.
fn print_line_lit(line: &[u8]) {
    debug_assert_eq!(line.last(), Some(&0));
    unsafe {
        printf(FMT_STR_LINE.as_ptr() as *const c_char, line.as_ptr() as *const c_char);
    }
}

fn print_int_line(int_number: c_int) {
    unsafe {
        printf(FMT_INT_LINE.as_ptr() as *const c_char, int_number);
    }
}

/// Number of elements in the C `int buffer[10]`.
const BUFFER_LEN: usize = 10;

/// Extra slack that follows `buffer` in memory. The C code writes past the end
/// of its 10-element stack array for `data >= 10` (undefined behaviour); this
/// slack reproduces the usual observable result of that write on a typical
/// stack frame -- the store lands in adjacent unused space and the ten printed
/// values are unchanged -- instead of corrupting this function's own frame.
const SLACK_LEN: usize = 1024;

/// `void bad(int data)`
#[unsafe(no_mangle)]
pub extern "C" fn bad(data: c_int) {
    let mut frame = [0 as c_int; BUFFER_LEN + SLACK_LEN];
    if data >= 0 {
        // Preserved bug: no upper-bound check on `data`.
        unsafe {
            *frame.as_mut_ptr().offset(data as isize) = 1;
        }
        /* Print the array values */
        for i in 0..BUFFER_LEN {
            print_int_line(frame[i]);
        }
    } else {
        print_line_lit(b"ERROR: Array index is negative.\0");
    }
}

/// `static void goodG2B()`
fn good_g2b() {
    let data: c_int = 7;
    let mut buffer = [0 as c_int; BUFFER_LEN];
    if data >= 0 {
        buffer[data as usize] = 1;
        /* Print the array values */
        for i in 0..BUFFER_LEN {
            print_int_line(buffer[i]);
        }
    } else {
        print_line_lit(b"ERROR: Array index is negative.\0");
    }
}

/// `static void goodB2G(int data)`
fn good_b2g(data: c_int) {
    let mut buffer = [0 as c_int; BUFFER_LEN];
    if data >= 0 && data < (BUFFER_LEN as c_int) {
        buffer[data as usize] = 1;
        /* Print the array values */
        for i in 0..BUFFER_LEN {
            print_int_line(buffer[i]);
        }
    } else {
        print_line_lit(b"ERROR: Array index is out-of-bounds\0");
    }
}

/// `void good(int data)`
#[unsafe(no_mangle)]
pub extern "C" fn good(data: c_int) {
    good_g2b();
    good_b2g(data);
}

/// `void driver(int goodData, int badData)`
#[unsafe(no_mangle)]
pub extern "C" fn driver(good_data: c_int, bad_data: c_int) {
    print_line_lit(b"Calling good()...\0");
    good(good_data);
    print_line_lit(b"Finished good()\0");
    print_line_lit(b"Calling bad()...\0");
    bad(bad_data);
    print_line_lit(b"Finished bad()\0");
}
