/*
 * Rust translation of the C library in c_src/ (goto.c / goto.h).
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

#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int};

/// Opaque stand-in for the C library's `FILE` type.  The Rust code never
/// inspects its contents; every `FILE*` handed out or consumed here comes
/// straight from the platform C library so that the values are fully
/// interchangeable with the ones the original C code produced (callers may,
/// for example, `fclose()` the pointer returned by `open_with_cleanup`).
#[repr(C)]
pub struct FILE {
    _opaque: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

// The C standard library is used directly (rather than Rust's `std::io`) so
// that stream buffering, flush ordering and the exact bytes emitted by the
// `printf` family match the original translation unit byte for byte.
extern "C" {
    /// glibc/musl export `stderr` as a global `FILE *` object.
    #[link_name = "stderr"]
    static mut c_stderr: *mut FILE;

    fn printf(format: *const c_char, ...) -> c_int;
    fn fprintf(stream: *mut FILE, format: *const c_char, ...) -> c_int;
    fn fopen(filename: *const c_char, mode: *const c_char) -> *mut FILE;
    fn fgets(s: *mut c_char, n: c_int, stream: *mut FILE) -> *mut c_char;
    fn ferror(stream: *mut FILE) -> c_int;
    fn fclose(stream: *mut FILE) -> c_int;
}

#[inline]
unsafe fn stderr_stream() -> *mut FILE {
    // Read the libc global through a raw pointer to avoid taking a reference
    // to a `static mut`.
    *core::ptr::addr_of!(c_stderr)
}

/*
 * int forward_goto_example(int x) {
 *   if (x < 0) {
 *     goto error;
 *   }
 *
 *   printf("Processing: %d\n", x);
 *   return x * 2;
 *
 * error:
 *   fprintf(stderr, "Error: negative input\n");
 *   return -1;
 * }
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forward_goto_example(x: c_int) -> c_int {
    if x < 0 {
        // goto error;
        fprintf(
            stderr_stream(),
            b"Error: negative input\n\0".as_ptr() as *const c_char,
        );
        return -1;
    }

    printf(b"Processing: %d\n\0".as_ptr() as *const c_char, x);
    // Signed overflow in the original C is UB; reproduce the behaviour of the
    // generated code (a plain two's-complement `imul`) with a wrapping
    // multiply so no Rust panic can occur.
    x.wrapping_mul(2)
}

/*
 * FILE* open_with_cleanup(const char *filename) {
 *   FILE* fp = fopen(filename, "r");
 *   if (!fp) {
 *     goto cleanup;
 *   }
 *
 *   char buffer[100];
 *   while (fgets(buffer, sizeof(buffer), fp)) {
 *       printf("%s", buffer);
 *   }
 *
 *   if (ferror(fp)) {
 *       goto cleanup;
 *   }
 *
 *   return fp;
 *
 * cleanup:
 *   fprintf(stderr, "Error: opening or processing file %s\n", filename);
 *   if(fp) fclose(fp);
 *   return NULL;
 * }
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn open_with_cleanup(filename: *const c_char) -> *mut FILE {
    let fp: *mut FILE = fopen(filename, b"r\0".as_ptr() as *const c_char);

    // `goto cleanup` from either the failed `fopen` or the `ferror` check.
    let mut goto_cleanup = fp.is_null();

    if !goto_cleanup {
        let mut buffer = [0 as c_char; 100];
        while !fgets(buffer.as_mut_ptr(), buffer.len() as c_int, fp).is_null() {
            printf(
                b"%s\0".as_ptr() as *const c_char,
                buffer.as_ptr() as *const c_char,
            );
        }

        if ferror(fp) != 0 {
            goto_cleanup = true;
        }
    }

    if !goto_cleanup {
        return fp;
    }

    // cleanup:
    fprintf(
        stderr_stream(),
        b"Error: opening or processing file %s\n\0".as_ptr() as *const c_char,
        filename,
    );
    if !fp.is_null() {
        fclose(fp);
    }
    core::ptr::null_mut()
}

/*
 * int driver(int num, const char* filename) {
 *   int res = forward_goto_example(num);
 *   if (res == -1) {
 *       return -1;
 *   } else {
 *       printf("Goto output: %d\n", res);
 *   }
 *
 *   FILE* out = open_with_cleanup(filename);
 *   if (out == NULL) {
 *       return -2;
 *   } else {
 *      fclose(out);
 *   }
 *
 *   return 0;
 * }
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(num: c_int, filename: *const c_char) -> c_int {
    let res: c_int = forward_goto_example(num);
    if res == -1 {
        return -1;
    } else {
        printf(b"Goto output: %d\n\0".as_ptr() as *const c_char, res);
    }

    let out: *mut FILE = open_with_cleanup(filename);
    if out.is_null() {
        return -2;
    } else {
        fclose(out);
    }

    0
}
