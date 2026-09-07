// Rust translation of c_src/ (MIT Lincoln Laboratory `driver` library).
//
// Original C copyright notice from c_src/src/driver.c and c_src/include/driver.h:
//
// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
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
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
//
// NOTE ON FIDELITY:
// The original C code contains deliberate memory-safety defects (an unchecked
// negative / oversized `data` argument flowing into `strncpy`'s length and into
// `dest[data]`). Per the translation requirements these defects are reproduced
// exactly rather than fixed. The C standard-library routines (`memset`,
// `strncpy`, `printf`) are called directly so that observable behaviour --
// including stdout buffering and the out-of-bounds accesses -- is
// byte-for-byte identical to the C build.

#![allow(non_snake_case)]

use core::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    /// `int printf(const char *restrict format, ...)`
    fn printf(format: *const c_char, ...) -> c_int;
    /// `void *memset(void *s, int c, size_t n)`
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    /// `char *strncpy(char *dst, const char *src, size_t n)`
    fn strncpy(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
}

/// Format string for `printf("%s\n", line)`, NUL-terminated.
const FMT_STR_NL: &[u8; 4] = b"%s\n\0";

// ---------------------------------------------------------------------------
// void printLine (const char * line)
// ---------------------------------------------------------------------------
//
//     void printLine (const char * line)
//     {
//         if(line != NULL)
//         {
//             printf("%s\n", line);
//         }
//     }
//
// `printLine` has external linkage in the C source (it is not `static`), so it
// is part of the shared library's exported ABI even though it is absent from
// the public header.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(FMT_STR_NL.as_ptr() as *const c_char, line);
        }
    }
}

// ---------------------------------------------------------------------------
// void driver(int data)
// ---------------------------------------------------------------------------
//
//     void driver(int data)
//     {
//         char source[100];
//         char dest[100] = "";
//         memset(source, 'A', 100-1);
//         source[100-1] = '\0';
//         if (data < 100)
//         {
//             strncpy(dest, source, data);
//             dest[data] = '\0';
//         }
//         printLine(dest);
//     }
//
// Behaviour reproduced verbatim, in the original order:
//   * `source` is 99 'A' bytes followed by a NUL terminator.
//   * `dest` is zero-initialised (`char dest[100] = ""` zero-fills the whole
//     aggregate in C).
//   * The guard is the original *signed* `data < 100` comparison, so negative
//     values pass it. `data` is then converted to `size_t` for `strncpy`'s
//     length parameter -- exactly as the implicit C conversion does -- which
//     turns a negative value into a huge count, and `dest[data]` indexes below
//     the buffer. Both defects are preserved.
//   * When `data >= 100` the copy is skipped entirely and the still-empty
//     `dest` is printed, yielding a lone newline.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(data: c_int) {
    // char source[100];
    let mut source: [c_char; 100] = [0; 100];
    // char dest[100] = "";
    let mut dest: [c_char; 100] = [0; 100];

    let source_ptr: *mut c_char = source.as_mut_ptr();
    let dest_ptr: *mut c_char = dest.as_mut_ptr();

    unsafe {
        // memset(source, 'A', 100-1);
        memset(source_ptr as *mut c_void, b'A' as c_int, 100 - 1);
        // source[100-1] = '\0';
        *source_ptr.add(100 - 1) = 0;

        // if (data < 100)  -- signed comparison, as in the original.
        if data < 100 {
            // strncpy(dest, source, data);
            // `data` widens to size_t exactly as the C implicit conversion
            // does; a negative `data` therefore becomes an enormous length.
            strncpy(dest_ptr, source_ptr as *const c_char, data as usize);
            // dest[data] = '\0';
            *dest_ptr.offset(data as isize) = 0;
        }

        // printLine(dest);
        printLine(dest_ptr as *const c_char);
    }
}
