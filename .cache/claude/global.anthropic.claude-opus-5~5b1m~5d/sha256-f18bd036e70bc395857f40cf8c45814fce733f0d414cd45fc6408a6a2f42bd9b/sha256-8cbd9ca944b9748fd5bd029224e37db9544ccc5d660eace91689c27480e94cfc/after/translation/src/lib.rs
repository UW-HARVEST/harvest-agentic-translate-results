// Rust translation of the C library in c_src/ (MIT Lincoln Laboratory `driver`).
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

#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};

// The C code prints through C stdio (`printf`). We call the very same
// functions so that buffering, flushing and formatting are byte-identical
// with the original library (including when output is redirected to a file
// or interleaved with output produced by a C caller).
extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// void printLine (const char * line)
/// {
///     if(line != NULL)
///     {
///         printf("%s\n", line);
///     }
/// }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        printf(b"%s\n\0".as_ptr() as *const c_char, line);
    }
}

/// void printIntLine (int intNumber)
/// {
///     printf("%d\n", intNumber);
/// }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntLine(intNumber: c_int) {
    printf(b"%d\n\0".as_ptr() as *const c_char, intNumber);
}

/// void bad()
/// {
///     int * data;
///     data = (int *)alloca(10);       /* only 10 *bytes* -- CWE-806 */
///     {
///         int source[10] = {0};
///         size_t i;
///         for (i = 0; i < 10; i++)
///         {
///             data[i] = source[i];    /* writes 40 bytes: overflow */
///         }
///         printIntLine(data[0]);
///     }
/// }
///
/// The original code intentionally under-allocates: `alloca(10)` reserves 10
/// bytes but the loop stores ten `int`s (40 bytes) into it, running past the
/// end of the allocation.  In practice the surplus lands in unused stack slack
/// of the calling frame, so the observable behaviour of the C library is simply
/// printing `data[0]`, i.e. `0`.  The translation keeps that observable
/// behaviour (the copy of the ten zeroed `int`s and the printed value) while
/// backing the "allocation" with storage large enough to hold the stores, so
/// no unrelated memory is clobbered.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    // `alloca(10)` -- the region the C code believes it owns is 10 bytes.
    const ALLOCA_BYTES: usize = 10;
    // Stack slack that the stores past the 10-byte allocation land in.
    let mut data = [0i32; (ALLOCA_BYTES + 3) / 4 + 10];
    {
        let source = [0i32; 10];
        let mut i: usize = 0;
        while i < 10 {
            data[i] = source[i];
            i += 1;
        }
        printIntLine(data[0]);
    }
}

/// void good()
/// {
///     int * data;
///     data = NULL;
///     data = (int *)alloca(10*sizeof(int));
///     {
///         int source[10] = {0};
///         size_t i;
///         for (i = 0; i < 10; i++)
///         {
///             data[i] = source[i];
///         }
///         printIntLine(data[0]);
///     }
/// }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    // data = NULL; then data = alloca(10 * sizeof(int));
    let mut data = [0i32; 10];
    {
        let source = [0i32; 10];
        let mut i: usize = 0;
        while i < 10 {
            data[i] = source[i];
            i += 1;
        }
        printIntLine(data[0]);
    }
}

/// void driver(int useGood)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        good();
    } else {
        bad();
    }
}
