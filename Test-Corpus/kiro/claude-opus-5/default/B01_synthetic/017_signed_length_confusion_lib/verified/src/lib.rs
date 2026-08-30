// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the “Software”),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! Rust translation of `c_src/src/driver.c`.
//!
//! The translation is intentionally literal. In particular:
//!
//! * `data` is not validated against negative values, exactly as in the C. A
//!   negative `data` is converted to `size_t` by `strncpy` (yielding a huge
//!   count) and used as an array index, just as the original does.
//! * Output is emitted through C `printf`, so stdout buffering, flushing and
//!   interleaving with any C code in the same process are identical to the
//!   original shared library.

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

/// `"%s\n"` format string, NUL terminated, as handed to `printf` by the C code.
static FMT_LINE: [u8; 4] = *b"%s\n\0";

/// Translation of the C `strncpy` used by `driver`.
///
/// Copies at most `n` bytes from `src` to `dst`, stopping after the NUL
/// terminator, then NUL-pads the remainder of the `n` bytes. No bounds
/// checking is performed, mirroring the C library function.
///
/// # Safety
///
/// Same contract as C `strncpy`: `src` must be a NUL-terminated string and
/// `dst` must have room for `n` bytes.
unsafe fn strncpy(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut i: usize = 0;

    // Copy up to and including the terminating NUL, but never more than n.
    while i < n {
        let ch = unsafe { *src.add(i) };
        unsafe { *dst.add(i) = ch };
        if ch == 0 {
            break;
        }
        i += 1;
    }

    // If the source ran out early, pad the rest of the n bytes with NUL.
    while i < n {
        unsafe { *dst.add(i) = 0 };
        i += 1;
    }

    dst
}

/// `void printLine(const char * line)`
///
/// Prints `line` followed by a newline, unless it is NULL.
///
/// # Safety
///
/// `line` must be NULL or point to a NUL-terminated string.
#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(FMT_LINE.as_ptr() as *const c_char, line);
        }
    }
}

/// `void driver(int data)`
///
/// Builds a 99-character run of `'A'`, copies `data` bytes of it into a second
/// 100-byte buffer when `data < 100`, terminates that copy at index `data`, and
/// prints the result. When `data >= 100` the destination stays empty and an
/// empty line is printed.
#[unsafe(no_mangle)]
pub extern "C" fn driver(data: c_int) {
    // char source[100];  /* filled in full by the memset + explicit NUL */
    let mut source = [0u8; 100];
    // char dest[100] = "";  /* zero initialized */
    let mut dest = [0u8; 100];

    // memset(source, 'A', 100-1);
    source[..99].fill(b'A');
    // source[100-1] = '\0';
    source[99] = 0;

    if data < 100 {
        // strncpy(dest, source, data);
        //
        // `data as usize` reproduces the C conversion of `int` to `size_t`,
        // including the wrap-around for negative values.
        unsafe {
            strncpy(dest.as_mut_ptr(), source.as_ptr(), data as usize);
        }
        // dest[data] = '\0';
        unsafe {
            *dest.as_mut_ptr().offset(data as isize) = 0;
        }
    }

    // printLine(dest);
    unsafe {
        printLine(dest.as_ptr() as *const c_char);
    }
}
