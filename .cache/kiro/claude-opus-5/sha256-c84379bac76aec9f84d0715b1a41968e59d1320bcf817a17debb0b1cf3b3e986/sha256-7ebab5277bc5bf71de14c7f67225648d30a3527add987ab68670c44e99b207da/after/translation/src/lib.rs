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

use std::ffi::{CStr, c_char, c_int};

// `driver.h` declares only `void driver(const char *in);` and contains no
// namespace/renaming macros, so the linker symbols are the plain source-level
// names. `fma_array` and `call_fma` are non-static in the C translation unit,
// so they are exported from the shared library too and are kept exported here.
unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// C: `void fma_array(int *restrict out, const int *mul1, const int *mul2,
///                   const int *add, int len)`
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
        // Signed overflow is UB in C; on the usual targets it wraps, so wrap here.
        let v = unsafe {
            (*mul1.offset(idx))
                .wrapping_mul(*mul2.offset(idx))
                .wrapping_add(*add.offset(idx))
        };
        unsafe { *out.offset(idx) = v };
        i += 1;
    }
}

/// C: `int call_fma(const int *data, int len)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn call_fma(data: *const c_int, len: c_int) -> c_int {
    if len == 0 {
        return 0;
    }
    // A negative `len` makes the C VLA declarations and the `out[len-1]` read
    // undefined behaviour; there is no observable behaviour to reproduce.
    if len < 0 {
        return 0;
    }

    let n = len as usize;
    // C leaves these VLAs uninitialized; every element that is later read is
    // written before use, so zero-filling is equivalent.
    let mut out: Vec<c_int> = vec![0; n];
    let mut ones: Vec<c_int> = vec![0; n];
    let mut zeros: Vec<c_int> = vec![0; n];

    out[0] = 0;
    for i in 0..n {
        ones[i] = 1;
        zeros[i] = 0;
    }

    unsafe {
        fma_array(
            out.as_mut_ptr(),
            ones.as_ptr(),
            data,
            zeros.as_ptr(),
            len,
        );
    }
    out[n - 1]
}

/// C: `void driver(const char *in)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(input: *const c_char) {
    // sscanf reads up to the terminating NUL, so snapshot the whole string once
    // and then walk it with an offset (the C code advances `in` by `%zn`).
    let s = unsafe { CStr::from_ptr(input) }.to_bytes();

    let mut data: [c_int; 100] = [0; 100];
    let mut pos: usize = 0;
    let mut i: usize = 0;
    while i < 100 {
        // `sscanf(in, "%d%zn", &data[i], &nb) != 1` -> break
        match scan_d(&s[pos..]) {
            Some((value, nb)) => {
                data[i] = value;
                pos += nb;
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

/// `isspace()` in the C locale.
fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// One `%d` conversion as performed by glibc's `sscanf`.
///
/// Returns `Some((converted_value, chars_consumed))` on success (the character
/// count matches what `%zn` would store, i.e. it includes the skipped leading
/// whitespace and the optional sign), or `None` for an input/matching failure.
///
/// glibc converts the collected digit string with `strtol`, which saturates at
/// `LONG_MAX`/`LONG_MIN`, and then assigns that `long` to the `int *` argument,
/// truncating it. That saturate-then-truncate behaviour is reproduced here.
fn scan_d(s: &[u8]) -> Option<(c_int, usize)> {
    let mut p: usize = 0;
    while p < s.len() && is_space(s[p]) {
        p += 1;
    }

    let negative = if p < s.len() && (s[p] == b'+' || s[p] == b'-') {
        let neg = s[p] == b'-';
        p += 1;
        neg
    } else {
        false
    };

    let digits_start = p;
    let mut magnitude: u64 = 0;
    let mut saturated = false;
    while p < s.len() && s[p].is_ascii_digit() {
        if !saturated {
            match magnitude
                .checked_mul(10)
                .and_then(|v| v.checked_add((s[p] - b'0') as u64))
            {
                Some(v) => magnitude = v,
                None => saturated = true,
            }
        }
        p += 1;
    }

    if p == digits_start {
        // No digits: matching failure (or input failure at end of string).
        return None;
    }

    let as_long: i64 = if negative {
        const MIN_MAGNITUDE: u64 = 1u64 << 63; // |LONG_MIN|
        if saturated || magnitude > MIN_MAGNITUDE {
            i64::MIN
        } else {
            (magnitude as i64).wrapping_neg()
        }
    } else if saturated || magnitude > i64::MAX as u64 {
        i64::MAX
    } else {
        magnitude as i64
    };

    Some((as_long as c_int, p))
}
