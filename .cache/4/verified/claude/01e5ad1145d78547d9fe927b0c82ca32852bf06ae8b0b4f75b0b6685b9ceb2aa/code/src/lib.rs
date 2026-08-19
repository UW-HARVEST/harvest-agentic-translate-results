// Rust translation of c_src/src/driver.c (MIT Lincoln Laboratory, 2025).
//
// The C library is built as a shared object that globs all of c_src and
// exports the following public symbols (per `nm -D libdriver.so`):
//     bad, driver, good, printIntLine, printLine
// `goodG2B` and `goodB2G` are `static` in the C source and therefore are not
// exported; they are translated as private Rust functions.
//
// Output is produced through the C runtime's `printf` so that the emitted
// bytes -- and the stdout buffering behaviour -- are identical to the C
// library's.

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

/// `"%s\n"` format string used by `printLine`.
const FMT_STR: [u8; 4] = *b"%s\n\0";
/// `"%d\n"` format string used by `printIntLine`.
const FMT_INT: [u8; 4] = *b"%d\n\0";

/// void printLine (const char * line)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(FMT_STR.as_ptr() as *const c_char, line);
        }
    }
}

/// void printIntLine (int intNumber)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntLine(int_number: c_int) {
    unsafe {
        printf(FMT_INT.as_ptr() as *const c_char, int_number);
    }
}

/// Helper: print a `&'static str` literal (NUL terminated) like the C code's
/// string-literal calls to `printLine`.
#[inline]
fn print_line_lit<const N: usize>(s: &'static [u8; N]) {
    debug_assert_eq!(s[N - 1], 0);
    unsafe { printLine(s.as_ptr() as *const c_char) }
}

/// void bad(int data)
///
/// Reproduces the original (intentionally faulty) behaviour: only the lower
/// bound of `data` is validated, so `buffer[data] = 1` performs an
/// out-of-bounds write for `data >= 10`, exactly as in the C source.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad(data: c_int) {
    let mut buffer: [c_int; 10] = [0; 10];
    if data >= 0 {
        unsafe {
            *buffer.as_mut_ptr().offset(data as isize) = 1;
        }
        /* Print the array values */
        for i in 0..10usize {
            unsafe { printIntLine(*buffer.as_ptr().add(i)) }
        }
    } else {
        print_line_lit(b"ERROR: Array index is negative.\0");
    }
}

/// static void goodG2B()
fn good_g2b() {
    let data: c_int = 7;
    let mut buffer: [c_int; 10] = [0; 10];
    if data >= 0 {
        unsafe {
            *buffer.as_mut_ptr().offset(data as isize) = 1;
        }
        /* Print the array values */
        for i in 0..10usize {
            unsafe { printIntLine(*buffer.as_ptr().add(i)) }
        }
    } else {
        print_line_lit(b"ERROR: Array index is negative.\0");
    }
}

/// static void goodB2G(int data)
fn good_b2g(data: c_int) {
    let mut buffer: [c_int; 10] = [0; 10];
    if data >= 0 && data < 10 {
        unsafe {
            *buffer.as_mut_ptr().offset(data as isize) = 1;
        }
        /* Print the array values */
        for i in 0..10usize {
            unsafe { printIntLine(*buffer.as_ptr().add(i)) }
        }
    } else {
        print_line_lit(b"ERROR: Array index is out-of-bounds\0");
    }
}

/// void good(int data)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good(data: c_int) {
    good_g2b();
    good_b2g(data);
}

/// void driver(int goodData, int badData)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(good_data: c_int, bad_data: c_int) {
    print_line_lit(b"Calling good()...\0");
    unsafe { good(good_data) };
    print_line_lit(b"Finished good()\0");
    print_line_lit(b"Calling bad()...\0");
    unsafe { bad(bad_data) };
    print_line_lit(b"Finished bad()\0");
}
