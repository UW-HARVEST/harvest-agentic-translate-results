// Rust translation of c_src/src/driver.c (MIT Lincoln Laboratory, 2025).
//
// The C library exports exactly two public symbols:
//   T printLine
//   T driver
// Both are reproduced below with identical signatures and behavior,
// including the original out-of-bounds/overflow bugs, which are preserved
// verbatim (no bug fixes).

#![allow(non_snake_case)]

use core::ptr;
use std::ffi::{c_char, c_int};

// Use the C runtime's stdio directly so that output bytes, buffering and
// interleaving with any other C stdio in the process are identical.
unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

/// C:
/// ```c
/// void printLine (const char * line)
/// {
///     if(line != NULL)
///     {
///         printf("%s\n", line);
///     }
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        // "%s\n"
        const FMT: [c_char; 4] = [b'%' as c_char, b's' as c_char, b'\n' as c_char, 0];
        unsafe {
            printf(FMT.as_ptr(), line);
        }
    }
}

/// Faithful re-implementation of `strncpy(dst, src, n)`:
/// copies at most `n` bytes from `src`, stopping after the terminating NUL,
/// then zero-pads `dst` out to `n` bytes total.
///
/// `n` is deliberately taken as `usize` (like C's `size_t`) so that a negative
/// `int` argument in the caller sign-extends into an enormous count exactly as
/// it does in the original C code.
#[inline(never)]
unsafe fn strncpy_c(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char {
    unsafe {
        let mut i: usize = 0;
        // Copy up to and including the NUL terminator (bounded by n).
        while i < n {
            let ch = *src.wrapping_add(i);
            ptr::write(dst.wrapping_add(i), ch);
            i += 1;
            if ch == 0 {
                break;
            }
        }
        // Zero-pad the remainder of the n bytes.
        while i < n {
            ptr::write(dst.wrapping_add(i), 0);
            i += 1;
        }
        dst
    }
}

/// C:
/// ```c
/// void driver(int data)
/// {
///     char source[100];
///     char dest[100] = "";
///     memset(source, 'A', 100-1);
///     source[100-1] = '\0';
///     if (data < 100)
///     {
///         strncpy(dest, source, data);
///         dest[data] = '\0';
///     }
///     printLine(dest);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(data: c_int) {
    // `char source[100];` (uninitialized in C, fully written below)
    let mut source: [c_char; 100] = [0; 100];
    // `char dest[100] = "";` -> zero-initialized in its entirety.
    let mut dest: [c_char; 100] = [0; 100];

    let source_ptr: *mut c_char = source.as_mut_ptr();
    let dest_ptr: *mut c_char = dest.as_mut_ptr();

    unsafe {
        // memset(source, 'A', 100-1);
        ptr::write_bytes(source_ptr, b'A', 100 - 1);
        // source[100-1] = '\0';
        ptr::write(source_ptr.add(100 - 1), 0);

        if data < 100 {
            // strncpy(dest, source, data);  -- `data` converts to size_t,
            // sign-extending when negative (original bug preserved).
            strncpy_c(dest_ptr, source_ptr, data as usize);
            // dest[data] = '\0';  -- negative index writes out of bounds
            // (original bug preserved).
            ptr::write(dest_ptr.wrapping_offset(data as isize), 0);
        }

        printLine(dest_ptr);
    }
}
