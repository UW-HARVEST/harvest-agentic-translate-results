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

//! Rust translation of `c_src/src/driver.c`.
//!
//! Exported ABI (matches `nm -D` of the C shared object):
//!   * `fma_array`
//!   * `call_fma`
//!   * `driver`
//!
//! The C original relies on `sscanf("%d%zn")` for parsing and `printf("%d\n")`
//! for output.  Both are called through the platform C library here so that the
//! parsing quirks (whitespace skipping, `strtol` saturation followed by
//! truncation to `int`, characters-consumed accounting) and the stdout
//! buffering behaviour are byte-for-byte identical to the C build.

use core::ffi::{c_char, c_int};

extern "C" {
    fn sscanf(s: *const c_char, format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

/// `void fma_array(int *restrict out, const int *mul1, const int *mul2, const int *add, int len)`
///
/// `out[i] = mul1[i] * mul2[i] + add[i]` for `i` in `[0, len)`.
/// Signed overflow (UB in C, wraps in practice with gcc) is reproduced with
/// wrapping arithmetic.  A non-positive `len` performs no iterations, exactly
/// like the C `for` loop.
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
        let a = *mul1.offset(idx);
        let b = *mul2.offset(idx);
        let c = *add.offset(idx);
        *out.offset(idx) = a.wrapping_mul(b).wrapping_add(c);
        i += 1;
    }
}

/// `int call_fma(const int *data, int len)`
///
/// Builds `ones` (all 1) and `zeros` (all 0) scratch arrays, runs
/// `fma_array(out, ones, data, zeros, len)` and returns `out[len - 1]`, i.e.
/// effectively `data[len - 1]`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn call_fma(data: *const c_int, len: c_int) -> c_int {
    if len == 0 {
        return 0;
    }

    // A negative `len` declares variable-length arrays of negative size and
    // then reads `out[len - 1]`: undefined behaviour in C (observed to return
    // uninitialised stack garbage).  Nothing meaningful can be reproduced, so
    // return without touching memory out of bounds.
    if len < 0 {
        return 0;
    }

    let n = len as usize;
    let mut out: Vec<c_int> = vec![0; n];
    let mut ones: Vec<c_int> = vec![0; n];
    let mut zeros: Vec<c_int> = vec![0; n];

    out[0] = 0;
    for i in 0..n {
        ones[i] = 1;
        zeros[i] = 0;
    }

    fma_array(out.as_mut_ptr(), ones.as_ptr(), data, zeros.as_ptr(), len);
    out[n - 1]
}

/// `void driver(const char *in)`
///
/// Scans up to 100 decimal integers out of `in` and prints the value of the
/// last successfully parsed one (0 when none could be parsed).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(input: *const c_char) {
    // The C code leaves `data` uninitialised; only the first `i` elements are
    // ever read, and those are always written by the scan loop below.
    let mut data: [c_int; 100] = [0; 100];
    let mut cursor = input;

    const SCAN_FMT: &[u8; 6] = b"%d%zn\0";

    let mut i: usize = 0;
    while i < 100 {
        let mut nb: usize = 0;
        if sscanf(
            cursor,
            SCAN_FMT.as_ptr() as *const c_char,
            &mut data[i] as *mut c_int,
            &mut nb as *mut usize,
        ) != 1
        {
            break;
        }
        cursor = cursor.add(nb);
        i += 1;
    }

    let result = call_fma(data.as_ptr(), i as c_int);

    const PRINT_FMT: &[u8; 4] = b"%d\n\0";
    printf(PRINT_FMT.as_ptr() as *const c_char, result);
}
