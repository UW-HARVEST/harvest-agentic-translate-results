/*
 * Rust translation of c_src/src/goto.c (public header: c_src/include/goto.h).
 *
 * Copyright 2025 MIT Lincoln Laboratory
 * Permission is hereby granted, free of charge,
 * to any person obtaining a copy of this software
 * and associated documentation files (the "Software"),
 * to deal in the Software without restriction,
 * including without limitation the rights to use, copy,
 * modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software,
 * and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice
 * shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
 * THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
 * IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
 * FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
 * TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
 * OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

//! Faithful Rust translation of the C `driver` library.
//!
//! The original C code performs all of its I/O through C `stdio`. To guarantee
//! byte-identical output (including the buffering behaviour of `stdout` versus
//! the unbuffered `stderr`, and therefore the interleaving of the two streams)
//! this translation calls the very same `stdio` entry points instead of using
//! Rust's own `std::io` handles, which have different buffering semantics.

use core::ffi::{c_char, c_int};

/// Opaque stand-in for C's `FILE`.
///
/// `open_with_cleanup` returns a `FILE *` as part of the public ABI, so the
/// type must be an opaque pointee — the library never inspects its contents.
#[repr(C)]
pub struct FILE {
    _opaque: [u8; 0],
}

unsafe extern "C" {
    /// glibc exposes `stderr` as a real global variable of type `FILE *`.
    static stderr: *mut FILE;

    fn printf(format: *const c_char, ...) -> c_int;
    fn fprintf(stream: *mut FILE, format: *const c_char, ...) -> c_int;
    fn fopen(filename: *const c_char, mode: *const c_char) -> *mut FILE;
    fn fgets(s: *mut c_char, n: c_int, stream: *mut FILE) -> *mut c_char;
    fn ferror(stream: *mut FILE) -> c_int;
    fn fclose(stream: *mut FILE) -> c_int;
}

/// ```c
/// int forward_goto_example(int x) {
///   if (x < 0) {
///     goto error;
///   }
///
///   printf("Processing: %d\n", x);
///   return x * 2;
///
/// error:
///   fprintf(stderr, "Error: negative input\n");
///   return -1;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn forward_goto_example(x: c_int) -> c_int {
    // The `goto error` branch is taken for negative inputs.
    if x < 0 {
        // error:
        unsafe {
            fprintf(stderr, c"Error: negative input\n".as_ptr());
        }
        return -1;
    }

    unsafe {
        printf(c"Processing: %d\n".as_ptr(), x);
    }

    // `x * 2` in C; `wrapping_mul` reproduces the two's-complement result the
    // C compiler emits for the (technically undefined) signed overflow case
    // instead of panicking.
    x.wrapping_mul(2)
}

/// ```c
/// FILE* open_with_cleanup(const char *filename) {
///   FILE* fp = fopen(filename, "r");
///   if (!fp) {
///     goto cleanup;
///   }
///
///   char buffer[100];
///   while (fgets(buffer, sizeof(buffer), fp)) {
///       printf("%s", buffer);
///   }
///
///   if (ferror(fp)) {
///       goto cleanup;
///   }
///
///   return fp;
///
/// cleanup:
///   fprintf(stderr, "Error: opening or processing file %s\n", filename);
///   if(fp) fclose(fp);
///   return NULL;
/// }
/// ```
///
/// Note that the caller receives an *open* `FILE *` on success and is
/// responsible for closing it, exactly as in the C original.
#[unsafe(no_mangle)]
pub extern "C" fn open_with_cleanup(filename: *const c_char) -> *mut FILE {
    unsafe {
        let fp: *mut FILE = fopen(filename, c"r".as_ptr());

        // Only jump straight to `cleanup` when the open failed; otherwise run
        // the read loop first.
        if !fp.is_null() {
            // `char buffer[100]` — C leaves it uninitialised, but it is only
            // ever read after a successful `fgets`, which NUL-terminates it.
            let mut buffer = [0 as c_char; 100];

            while !fgets(buffer.as_mut_ptr(), buffer.len() as c_int, fp).is_null() {
                // `printf("%s", buffer)` stops at the first NUL byte, so an
                // embedded NUL in the input truncates the echoed line. Going
                // through `printf` preserves that quirk.
                printf(c"%s".as_ptr(), buffer.as_ptr());
            }

            if ferror(fp) == 0 {
                return fp;
            }
            // fall through to cleanup
        }

        // cleanup:
        fprintf(
            stderr,
            c"Error: opening or processing file %s\n".as_ptr(),
            filename,
        );
        if !fp.is_null() {
            fclose(fp);
        }
        core::ptr::null_mut()
    }
}

/// ```c
/// int driver(int num, const char* filename) {
///   int res = forward_goto_example(num);
///   if (res == -1) {
///       return -1;
///   } else {
///       printf("Goto output: %d\n", res);
///   }
///
///   FILE* out = open_with_cleanup(filename);
///   if (out == NULL) {
///       return -2;
///   } else {
///      fclose(out);
///   }
///
///   return 0;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn driver(num: c_int, filename: *const c_char) -> c_int {
    let res = forward_goto_example(num);
    if res == -1 {
        return -1;
    } else {
        unsafe {
            printf(c"Goto output: %d\n".as_ptr(), res);
        }
    }

    let out = open_with_cleanup(filename);
    if out.is_null() {
        return -2;
    } else {
        unsafe {
            fclose(out);
        }
    }

    0
}
