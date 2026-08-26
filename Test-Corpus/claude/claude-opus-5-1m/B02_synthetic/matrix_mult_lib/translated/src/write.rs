/*
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

//! Translation of `c_src/src/write.c`.

use core::ffi::{c_char, c_int};

use crate::cffi::{
    errno, fclose, fopen, fprintf, stderr_stream, strerror, EINVAL,
};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn write_to_file(filename: *const c_char, content: *const c_char) -> c_int {
    unsafe {
        if content.is_null() {
            fprintf(stderr_stream(), c"Error: Content is NULL.\n".as_ptr());
            return EINVAL;
        }

        let file = fopen(filename, c"w".as_ptr());
        if file.is_null() {
            fprintf(
                stderr_stream(),
                c"Error opening file '%s': %s\n".as_ptr(),
                filename,
                strerror(errno()),
            );
            return errno();
        }

        if fprintf(file, c"%s".as_ptr(), content) < 0 {
            fprintf(
                stderr_stream(),
                c"Error writing to file '%s': %s\n".as_ptr(),
                filename,
                strerror(errno()),
            );
            fclose(file);
            return errno();
        }

        if fclose(file) != 0 {
            fprintf(
                stderr_stream(),
                c"Error closing file '%s': %s\n".as_ptr(),
                filename,
                strerror(errno()),
            );
            return errno();
        }

        0
    }
}
