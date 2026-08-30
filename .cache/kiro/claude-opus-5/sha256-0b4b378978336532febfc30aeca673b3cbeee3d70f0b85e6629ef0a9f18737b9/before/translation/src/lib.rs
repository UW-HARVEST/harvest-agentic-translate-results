// Rust translation of c_src/src/driver.c
//
// Behaviour is preserved exactly, including the original code's quirks:
//   * `printLine` is a public (non-static) C symbol, so it is exported too.
//   * The `data < 100` guard is kept as-is; no additional validation is added
//     (a negative `data` reproduces the same out-of-bounds behaviour as the C).
//   * Output goes through libc `printf` so buffering and formatting are
//     byte-identical to the C library.

#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn strncpy(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
}

/// void printLine(const char * line)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(c"%s\n".as_ptr(), line);
        }
    }
}

/// void driver(int data)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(data: c_int) {
    // char source[100]; char dest[100] = "";
    let mut source = [0u8; 100];
    let mut dest = [0u8; 100];

    // memset(source, 'A', 100-1); source[100-1] = '\0';
    source[..100 - 1].fill(b'A');
    source[100 - 1] = 0;

    if data < 100 {
        unsafe {
            // strncpy(dest, source, data);
            strncpy(
                dest.as_mut_ptr().cast::<c_char>(),
                source.as_ptr().cast::<c_char>(),
                // Same implicit int -> size_t conversion the C performs.
                data as usize,
            );
            // dest[data] = '\0';
            *dest.as_mut_ptr().offset(data as isize) = 0;
        }
    }

    unsafe {
        printLine(dest.as_ptr().cast::<c_char>());
    }
}
