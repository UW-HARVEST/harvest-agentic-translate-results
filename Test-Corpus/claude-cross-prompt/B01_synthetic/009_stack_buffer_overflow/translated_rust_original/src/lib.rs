// Translated from MIT Lincoln Laboratory C source.
// See c_src/CMakeLists.txt for the original copyright notice.

#![allow(non_snake_case)]
#![allow(unused_assignments)]

use std::ffi::c_char;
use std::os::raw::c_int;

// Use libc directly so output is byte-identical to the C version
// (printf, fgets, atoi all match the C runtime exactly).
extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn fgets(s: *mut c_char, size: c_int, stream: *mut libc::FILE) -> *mut c_char;
    fn atoi(s: *const c_char) -> c_int;
    static stdin: *mut libc::FILE;
}

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(b"%s\n\0".as_ptr() as *const c_char, line);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn printIntLine(intNumber: c_int) {
    unsafe {
        printf(b"%d\n\0".as_ptr() as *const c_char, intNumber);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    let mut data: c_int;
    /* Initialize data */
    data = -1;
    {
        // char inputBuffer[14] = "";
        let mut input_buffer: [c_char; 14] = [0; 14];
        let res = unsafe {
            fgets(
                input_buffer.as_mut_ptr(),
                14,
                stdin,
            )
        };
        if !res.is_null() {
            /* Convert to int */
            data = unsafe { atoi(input_buffer.as_ptr()) };
        } else {
            let msg = b"fgets() failed.\0";
            printLine(msg.as_ptr() as *const c_char);
        }
    }
    {
        let mut buffer: [c_int; 10] = [0; 10];
        if data >= 0 {
            // Reproduces C behavior exactly, including out-of-bounds writes.
            // Use unchecked indexing via raw pointer to mirror C's lack of
            // bounds checking. (CWE-129 demonstration code.)
            unsafe {
                let p = buffer.as_mut_ptr().offset(data as isize);
                *p = 1;
            }
            /* Print the array values */
            for i in 0..10usize {
                printIntLine(buffer[i]);
            }
        } else {
            let msg = b"ERROR: Array index is negative.\0";
            printLine(msg.as_ptr() as *const c_char);
        }
    }
}

/* goodG2B uses the GoodSource with the BadSink */
fn goodG2B() {
    let mut data: c_int;
    /* Initialize data */
    data = -1;
    data = 7;
    {
        let mut buffer: [c_int; 10] = [0; 10];
        if data >= 0 {
            buffer[data as usize] = 1;
            /* Print the array values */
            for i in 0..10usize {
                printIntLine(buffer[i]);
            }
        } else {
            let msg = b"ERROR: Array index is negative.\0";
            printLine(msg.as_ptr() as *const c_char);
        }
    }
}

/* goodB2G uses the BadSource with the GoodSink */
fn goodB2G() {
    let mut data: c_int;
    /* Initialize data */
    data = -1;
    {
        let mut input_buffer: [c_char; 14] = [0; 14];
        let res = unsafe {
            fgets(
                input_buffer.as_mut_ptr(),
                14,
                stdin,
            )
        };
        if !res.is_null() {
            /* Convert to int */
            data = unsafe { atoi(input_buffer.as_ptr()) };
        } else {
            let msg = b"fgets() failed.\0";
            printLine(msg.as_ptr() as *const c_char);
        }
    }
    {
        let mut buffer: [c_int; 10] = [0; 10];
        if data >= 0 && data < 10 {
            buffer[data as usize] = 1;
            /* Print the array values */
            for i in 0..10usize {
                printIntLine(buffer[i]);
            }
        } else {
            let msg = b"ERROR: Array index is out-of-bounds\0";
            printLine(msg.as_ptr() as *const c_char);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    goodG2B();
    goodB2G();
}

#[unsafe(no_mangle)]
pub extern "C" fn main(_argc: c_int, _argv: *const *const c_char) -> c_int {
    let m1 = b"Calling good()...\0";
    printLine(m1.as_ptr() as *const c_char);
    good();
    let m2 = b"Finished good()\0";
    printLine(m2.as_ptr() as *const c_char);
    let m3 = b"Calling bad()...\0";
    printLine(m3.as_ptr() as *const c_char);
    bad();
    let m4 = b"Finished bad()\0";
    printLine(m4.as_ptr() as *const c_char);
    0
}
