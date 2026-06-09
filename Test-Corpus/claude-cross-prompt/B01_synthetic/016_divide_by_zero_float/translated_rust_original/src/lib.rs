// Translated from c_src/src/main.c
// Preserves exact behavior, including the C use of fgets/atof/printf and
// the cast of (double) to (int) (truncation toward zero).

use libc::{atof, c_char, c_int, fgets, printf, FILE};

const CHAR_ARRAY_SIZE: c_int = 20;

#[allow(non_upper_case_globals)]
unsafe extern "C" {
    static mut stdin: *mut FILE;
}

#[inline]
fn stdin_ptr() -> *mut FILE {
    unsafe { stdin }
}

/// Convert f64 to c_int matching C's truncation cast semantics for in-range
/// values. Out-of-range values follow Rust's saturating semantics.
#[inline]
fn f64_to_cint(f: f64) -> c_int {
    f as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(b"%s\n\0".as_ptr() as *const c_char, line);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntLine(int_number: c_int) {
    unsafe {
        printf(b"%d\n\0".as_ptr() as *const c_char, int_number);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    let mut data: f32;
    data = 0.0_f32;
    {
        let mut input_buffer: [c_char; CHAR_ARRAY_SIZE as usize] = [0; CHAR_ARRAY_SIZE as usize];
        unsafe {
            if !fgets(input_buffer.as_mut_ptr(), CHAR_ARRAY_SIZE, stdin_ptr()).is_null() {
                data = atof(input_buffer.as_ptr()) as f32;
            } else {
                printLine(b"fgets() failed.\0".as_ptr() as *const c_char);
            }
        }
    }
    {
        let result: c_int = f64_to_cint(100.0_f64 / data as f64);
        unsafe {
            printIntLine(result);
        }
    }
}

fn good_g2_b() {
    let data: f32 = 2.0_f32;
    {
        let result: c_int = f64_to_cint(100.0_f64 / data as f64);
        unsafe {
            printIntLine(result);
        }
    }
}

fn good_b2_g() {
    let mut data: f32;
    data = 0.0_f32;
    {
        let mut input_buffer: [c_char; CHAR_ARRAY_SIZE as usize] = [0; CHAR_ARRAY_SIZE as usize];
        unsafe {
            if !fgets(input_buffer.as_mut_ptr(), CHAR_ARRAY_SIZE, stdin_ptr()).is_null() {
                data = atof(input_buffer.as_ptr()) as f32;
            } else {
                printLine(b"fgets() failed.\0".as_ptr() as *const c_char);
            }
        }
    }
    if (data as f64).abs() > 0.000001_f64 {
        let result: c_int = f64_to_cint(100.0_f64 / data as f64);
        unsafe {
            printIntLine(result);
        }
    } else {
        unsafe {
            printLine(b"This would result in a divide by zero\0".as_ptr() as *const c_char);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    good_g2_b();
    good_b2_g();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn main(_argc: c_int, _argv: *const *const c_char) -> c_int {
    unsafe {
        printLine(b"Calling good()...\0".as_ptr() as *const c_char);
        good();
        printLine(b"Finished good()\0".as_ptr() as *const c_char);
        printLine(b"Calling bad()...\0".as_ptr() as *const c_char);
        bad();
        printLine(b"Finished bad()\0".as_ptr() as *const c_char);
    }
    0
}
