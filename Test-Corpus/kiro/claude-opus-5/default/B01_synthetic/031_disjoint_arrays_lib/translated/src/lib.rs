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

// Use the C runtime's printf so that output ordering/buffering matches the
// original library exactly when it is loaded into a C program.
unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// `out[i] = mul1[i] * mul2[i] + add[i]` for `i` in `0..len`.
///
/// The C original declares `out` as `int *restrict`; Rust has no equivalent
/// qualifier, so the pointers are simply treated as non-aliasing by contract.
/// Signed overflow is undefined in C; gcc/clang wrap in practice, so wrapping
/// arithmetic is used here.
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

/// Builds `ones`/`zeros` vectors and runs `fma_array`, returning the last
/// element of the result (which, given the operands, is `data[len - 1]`).
///
/// A `len` below zero is undefined behaviour in the C original (negative VLA
/// size); here it is treated the same as an empty input.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn call_fma(data: *const c_int, len: c_int) -> c_int {
    if len == 0 {
        return 0;
    }
    if len < 0 {
        // UB in C (`int out[len]` with negative length). Avoid an allocation
        // panic across the FFI boundary.
        return 0;
    }

    let n = len as usize;
    let mut out: Vec<c_int> = vec![0; n];
    let ones: Vec<c_int> = vec![1; n];
    let zeros: Vec<c_int> = vec![0; n];

    // `out[0] = 0;` in the C source; the rest of `out` is left uninitialised
    // there, but every element is overwritten by `fma_array` below.

    unsafe {
        fma_array(out.as_mut_ptr(), ones.as_ptr(), data, zeros.as_ptr(), len);
    }

    out[n - 1]
}

/// True for the characters `isspace()` accepts in the C locale.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// Emulates a single `sscanf(s, "%d%zn", &value, &nb)`.
///
/// Returns `Some((value, nb))` on a successful conversion, where `nb` is the
/// number of bytes consumed (leading whitespace and sign included), or `None`
/// on a matching failure / input failure.
///
/// Overflow follows glibc: the digits are converted as a `long` (64-bit here)
/// which saturates at `LONG_MIN`/`LONG_MAX`, and the result is then truncated
/// on assignment to `int`.
fn scan_int(s: &[u8]) -> Option<(c_int, usize)> {
    let mut pos = 0usize;

    while pos < s.len() && is_c_space(s[pos]) {
        pos += 1;
    }

    let negative = match s.get(pos) {
        Some(b'-') => {
            pos += 1;
            true
        }
        Some(b'+') => {
            pos += 1;
            false
        }
        _ => false,
    };

    let digits_start = pos;
    let mut acc: i128 = 0;
    let mut saturated = false;
    while pos < s.len() && s[pos].is_ascii_digit() {
        if !saturated {
            acc = acc * 10 + i128::from(s[pos] - b'0');
            if acc > i128::from(u64::MAX) {
                saturated = true;
            }
        }
        pos += 1;
    }

    if pos == digits_start {
        // No digits: matching failure (or input failure at end of string).
        return None;
    }

    let as_long: i64 = if negative {
        let neg = -acc;
        if neg < i128::from(i64::MIN) {
            i64::MIN
        } else {
            neg as i64
        }
    } else if acc > i128::from(i64::MAX) {
        i64::MAX
    } else {
        acc as i64
    };

    Some((as_long as c_int, pos))
}

/// Parses up to 100 decimal integers out of `in_`, then prints the last one
/// (or `0` when nothing parsed) followed by a newline.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(in_: *const c_char) {
    // `data` is uninitialised in the C original; only the first `i` entries are
    // ever read, so zero-filling is equivalent.
    let mut data: [c_int; 100] = [0; 100];

    let mut rest: &[u8] = unsafe { c_str_bytes(in_) };

    let mut i: usize = 0;
    while i < 100 {
        match scan_int(rest) {
            Some((value, nb)) => {
                data[i] = value;
                rest = &rest[nb..];
            }
            None => break,
        }
        i += 1;
    }

    let result = unsafe { call_fma(data.as_ptr(), i as c_int) };
    unsafe {
        printf(c"%d\n".as_ptr(), result);
    }
}

/// Borrows the NUL-terminated string at `p` as a byte slice.
unsafe fn c_str_bytes(p: *const c_char) -> &'static [u8] {
    let mut len = 0usize;
    unsafe {
        while *p.add(len) != 0 {
            len += 1;
        }
        std::slice::from_raw_parts(p as *const u8, len)
    }
}
