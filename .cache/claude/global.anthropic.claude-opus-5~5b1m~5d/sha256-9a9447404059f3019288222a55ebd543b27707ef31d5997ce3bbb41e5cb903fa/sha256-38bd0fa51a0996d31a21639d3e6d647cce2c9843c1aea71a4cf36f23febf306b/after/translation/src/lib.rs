// Rust translation of the C library in c_src/.
//
// Original copyright notice from the C sources:
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

unsafe extern "C" {
    // Use the platform C library's printf so that output buffering and
    // formatting are byte-for-byte identical to the original C library
    // (and so output interleaves with any other C stdio in the process
    // exactly as it did before).
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Format string `"%d\n"` used by `inner`.
static FMT_D_NL: [c_char; 4] = [b'%' as c_char, b'd' as c_char, b'\n' as c_char, 0];

/// C: `void fma_array(int *out, const int *mul1, const int *mul2, const int *add, int len)`
///
/// out[i] = mul1[i] * mul2[i] + add[i]
///
/// Signed overflow is UB in C; in practice the compiler emits wrapping
/// two's-complement arithmetic, which is what we reproduce here.
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
        unsafe {
            let v = (*mul1.offset(idx))
                .wrapping_mul(*mul2.offset(idx))
                .wrapping_add(*add.offset(idx));
            *out.offset(idx) = v;
        }
        i += 1;
    }
}

/// C: `static void inner(int *out, int len)`
unsafe fn inner(out: *mut c_int, len: c_int) {
    unsafe {
        fma_array(out, out, out, out, len);
        let mut i: c_int = 0;
        while i < len {
            printf(FMT_D_NL.as_ptr(), *out.offset(i as isize));
            i += 1;
        }
    }
}

/// C: `void driver(const int *data, int len)`
///
/// Uses a VLA `int out[len]` plus `memcpy(out, data, len * sizeof(int))`,
/// then calls `inner`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(data: *const c_int, len: c_int) {
    // `int out[len]` — a VLA. A non-positive `len` means the subsequent
    // loops/copies do nothing observable, so an empty buffer suffices.
    let n = if len > 0 { len as usize } else { 0 };
    let mut out: Vec<c_int> = vec![0; n];
    if n > 0 {
        unsafe {
            std::ptr::copy_nonoverlapping(data, out.as_mut_ptr(), n);
        }
    }
    unsafe {
        inner(out.as_mut_ptr(), len);
    }
}
