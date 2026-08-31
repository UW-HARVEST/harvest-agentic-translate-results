// Rust translation of c_src/src/driver.c
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

// Use the C library's `printf` rather than Rust's own `stdout` machinery so that
// the emitted bytes -- and the stdio buffering/interleaving behaviour when this
// library is loaded next to C code -- match the original exactly.
unsafe extern "C" {
    #[link_name = "printf"]
    unsafe fn c_printf(fmt: *const c_char, ...) -> c_int;
}

/// `"%d\n\0"` -- the format string used by `inner`.
const FMT_D_NL: [c_char; 4] = [b'%' as c_char, b'd' as c_char, b'\n' as c_char, 0];

/// void fma_array(int *out, const int *mul1, const int *mul2, const int *add, int len)
///
/// Fused multiply-add over parallel arrays. The C version performs the writes
/// one element at a time through possibly-aliasing pointers, so each iteration
/// re-reads its inputs after the previous store; the raw-pointer loop below
/// keeps that observable behaviour intact.
///
/// Signed overflow is undefined behaviour in C but wraps on every target this
/// code is built for, so wrapping arithmetic reproduces it.
///
/// # Safety
///
/// `out`, `mul1`, `mul2` and `add` must each be valid for `len` `c_int`
/// elements (`out` for writes), exactly as the C function requires. They may
/// alias. Nothing is read or written when `len <= 0`.
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
        let a = unsafe { *mul1.offset(idx) };
        let b = unsafe { *mul2.offset(idx) };
        let c = unsafe { *add.offset(idx) };
        let value = a.wrapping_mul(b).wrapping_add(c);
        unsafe { *out.offset(idx) = value };
        i += 1;
    }
}

/// static void inner(int *out, int len)
fn inner(out: &mut [c_int], len: c_int) {
    let base = out.as_mut_ptr();
    unsafe { fma_array(base, base, base, base, len) };

    let mut i: c_int = 0;
    while i < len {
        unsafe { c_printf(FMT_D_NL.as_ptr(), out[i as usize]) };
        i += 1;
    }
}

/// void driver(const int *data, int len)
///
/// The C original declares a variable-length array `int out[len]` and copies
/// `len * sizeof(int)` bytes into it; a heap buffer stands in for the VLA here.
///
/// # Safety
///
/// `data` must be valid for reads of `len` `c_int` elements. As in C, nothing is
/// read when `len <= 0`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(data: *const c_int, len: c_int) {
    let n = if len > 0 { len as usize } else { 0 };
    let mut out: Vec<c_int> = vec![0; n];
    if n > 0 {
        unsafe { std::ptr::copy_nonoverlapping(data, out.as_mut_ptr(), n) };
    }
    inner(&mut out, len);
}
