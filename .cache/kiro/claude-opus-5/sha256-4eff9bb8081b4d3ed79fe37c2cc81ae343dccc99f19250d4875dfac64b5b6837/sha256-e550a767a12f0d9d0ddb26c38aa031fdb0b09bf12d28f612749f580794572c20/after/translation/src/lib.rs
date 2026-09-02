// Rust translation of c_src/src/driver.c (MIT Lincoln Laboratory, 2025).
//
// The original C library is compiled into a single shared object that exports
// the following symbols (verified with `nm -D --defined-only libdriver.so`):
//
//     bad, driver, good, printIntLine, printLine
//
// `goodG2B` and `goodB2G` are `static` in the C source and therefore are NOT
// part of the public ABI; they are kept private here as well.
//
// Output is produced through libc's `printf` so that the byte stream and the
// stdout buffering behaviour are identical to the C library.

#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Format string `"%s\n"` used by `printLine`.
static FMT_S: [c_char; 4] = [b'%' as c_char, b's' as c_char, b'\n' as c_char, 0];
/// Format string `"%d\n"` used by `printIntLine`.
static FMT_D: [c_char; 4] = [b'%' as c_char, b'd' as c_char, b'\n' as c_char, 0];

/// C string literal helper: builds a NUL-terminated `[c_char; N + 1]`.
macro_rules! cstr {
    ($bytes:expr) => {{
        const SRC: &[u8] = $bytes;
        const N: usize = SRC.len();
        const BUF: [c_char; N + 1] = {
            let mut buf = [0 as c_char; N + 1];
            let mut i = 0;
            while i < N {
                buf[i] = SRC[i] as c_char;
                i += 1;
            }
            buf
        };
        BUF.as_ptr()
    }};
}

// void printLine (const char * line)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(FMT_S.as_ptr(), line);
        }
    }
}

// void printIntLine (int intNumber)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntLine(intNumber: c_int) {
    unsafe {
        printf(FMT_D.as_ptr(), intNumber);
    }
}

/// Number of `int` slots actually declared by the C source (`int buffer[10]`).
const BUFFER_LEN: usize = 10;
/// The C code writes `buffer[data] = 1` for any non-negative `data`, i.e. it
/// writes out of bounds when `data >= BUFFER_LEN` (CWE-787). That write is
/// undefined behaviour in C; in practice it lands in unused stack space and the
/// function keeps running, printing the ten (still zero) in-bounds elements.
///
/// To reproduce that observable behaviour instead of trapping, the 10-element
/// buffer is backed by a larger zeroed region so the overrun writes into slack
/// memory rather than clobbering the caller's frame. Only the first
/// `BUFFER_LEN` elements are ever read back, exactly as in the C source.
const BUFFER_SLACK: usize = 1024;

// void bad(int data)
//
// Faithful reproduction of the original, out-of-bounds write included; the bug
// is preserved, not fixed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad(data: c_int) {
    let mut backing: [c_int; BUFFER_SLACK] = [0; BUFFER_SLACK];
    let buffer = &mut backing[..BUFFER_LEN];
    if data >= 0 {
        // buffer[data] = 1;  (unchecked, exactly as in C)
        unsafe {
            *buffer.as_mut_ptr().offset(data as isize) = 1;
        }
        /* Print the array values */
        for i in 0..BUFFER_LEN {
            unsafe {
                printIntLine(buffer[i]);
            }
        }
    } else {
        unsafe {
            printLine(cstr!(b"ERROR: Array index is negative."));
        }
    }
}

// static void goodG2B()
fn goodG2B() {
    let data: c_int = 7;
    let mut buffer: [c_int; BUFFER_LEN] = [0; BUFFER_LEN];
    if data >= 0 {
        buffer[data as usize] = 1;
        /* Print the array values */
        for i in 0..BUFFER_LEN {
            unsafe {
                printIntLine(buffer[i]);
            }
        }
    } else {
        unsafe {
            printLine(cstr!(b"ERROR: Array index is negative."));
        }
    }
}

// static void goodB2G(int data)
fn goodB2G(data: c_int) {
    let mut buffer: [c_int; BUFFER_LEN] = [0; BUFFER_LEN];
    if data >= 0 && data < 10 {
        buffer[data as usize] = 1;
        /* Print the array values */
        for i in 0..BUFFER_LEN {
            unsafe {
                printIntLine(buffer[i]);
            }
        }
    } else {
        unsafe {
            printLine(cstr!(b"ERROR: Array index is out-of-bounds"));
        }
    }
}

// void good(int data)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good(data: c_int) {
    goodG2B();
    goodB2G(data);
}

// void driver(int goodData, int badData)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(goodData: c_int, badData: c_int) {
    unsafe {
        printLine(cstr!(b"Calling good()..."));
        good(goodData);
        printLine(cstr!(b"Finished good()"));
        printLine(cstr!(b"Calling bad()..."));
        bad(badData);
        printLine(cstr!(b"Finished bad()"));
    }
}
