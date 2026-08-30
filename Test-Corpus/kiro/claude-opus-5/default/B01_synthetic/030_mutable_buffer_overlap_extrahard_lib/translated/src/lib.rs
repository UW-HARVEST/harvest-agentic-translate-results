// Rust translation of c_src/src/driver.c
//
// Original C copyright notice (retained from the source being translated):
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

use std::ffi::{c_char, c_int};

// The C code prints with printf(3). Calling the platform's printf directly
// keeps stdout buffering (and therefore interleaving with any other C output
// from the host program) byte-for-byte identical to the original library.
extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// ```c
/// void fma_array(int *out, const int *mul1, const int *mul2, const int *add, int len)
/// ```
///
/// The pointers are deliberately not marked as non-aliasing: `driver` passes
/// the same buffer for every argument, exactly as the C does.
///
/// # Safety
/// `out`, `mul1`, `mul2` and `add` must be valid for `len` `c_int` elements
/// (readable, and `out` also writable), just as in C.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fma_array(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
) {
    let mut i: c_int = 0;
    while i < len {
        let idx = i as isize;
        // C `int` arithmetic here can overflow; overflow is wrapped rather
        // than panicking so the observable result matches the compiled C.
        let v = (*mul1.offset(idx))
            .wrapping_mul(*mul2.offset(idx))
            .wrapping_add(*add.offset(idx));
        *out.offset(idx) = v;
        i = i.wrapping_add(1);
    }
}

/// ```c
/// static void inner(int *out, int len)
/// ```
unsafe fn inner(out: *mut c_int, len: c_int) {
    fma_array(out, out, out, out, len);
    let mut i: c_int = 0;
    while i < len {
        printf(c"%d\n".as_ptr(), *out.offset(i as isize));
        i = i.wrapping_add(1);
    }
}

/// ```c
/// void driver(const int *data, int len)
/// ```
///
/// # Safety
/// `data` must be valid for reads of `len` `c_int` elements.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(data: *const c_int, len: c_int) {
    // C: `int out[len];` followed by `memcpy(out, data, len * sizeof(int));`
    let n = len as usize;
    let mut out: Vec<c_int> = vec![0; n];
    std::ptr::copy_nonoverlapping(data, out.as_mut_ptr(), n);
    inner(out.as_mut_ptr(), len);
}
