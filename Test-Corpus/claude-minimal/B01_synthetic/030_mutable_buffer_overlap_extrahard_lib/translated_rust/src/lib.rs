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

use std::os::raw::c_int;
use std::slice;

/// Computes out[i] = mul1[i] * mul2[i] + add[i] for each i in 0..len.
pub fn fma_array(out: &mut [c_int], mul1: &[c_int], mul2: &[c_int], add: &[c_int], len: usize) {
    for i in 0..len {
        out[i] = mul1[i].wrapping_mul(mul2[i]).wrapping_add(add[i]);
    }
}

fn inner(out: &mut [c_int], len: usize) {
    // Mirror the C behavior of calling fma_array with `out` for all four buffers.
    // To avoid simultaneous mutable+immutable borrows, copy the inputs first.
    let snapshot: Vec<c_int> = out[..len].to_vec();
    for i in 0..len {
        out[i] = snapshot[i]
            .wrapping_mul(snapshot[i])
            .wrapping_add(snapshot[i]);
    }
    for i in 0..len {
        println!("{}", out[i]);
    }
}

/// Safe Rust entry point.
pub fn driver_safe(data: &[c_int]) {
    let len = data.len();
    let mut out: Vec<c_int> = data.to_vec();
    inner(&mut out, len);
}

/// C-compatible entry point matching `void driver(const int *data, int len);`.
///
/// # Safety
/// `data` must point to a valid array of at least `len` `c_int` elements,
/// or `len` must be 0.
#[no_mangle]
pub unsafe extern "C" fn driver(data: *const c_int, len: c_int) {
    if len <= 0 || data.is_null() {
        return;
    }
    let len_usize = len as usize;
    let slice = slice::from_raw_parts(data, len_usize);
    driver_safe(slice);
}

/// C-compatible exported version of `fma_array`.
///
/// # Safety
/// All pointers must be valid for `len` `c_int` elements. `out` must be writable.
#[no_mangle]
pub unsafe extern "C" fn fma_array_c(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
) {
    if len <= 0 {
        return;
    }
    let len_usize = len as usize;
    let out_slice = slice::from_raw_parts_mut(out, len_usize);
    let mul1_slice = slice::from_raw_parts(mul1, len_usize);
    let mul2_slice = slice::from_raw_parts(mul2, len_usize);
    let add_slice = slice::from_raw_parts(add, len_usize);
    fma_array(out_slice, mul1_slice, mul2_slice, add_slice, len_usize);
}
