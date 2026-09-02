// Rust translation of c_src/src/lib.c
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
// Behavioural notes (deliberate bug-for-bug fidelity with the C):
//
//   * `(int)double` conversions in the C are undefined when the truncated value
//     does not fit in an `int`. On x86-64 gcc emits `cvttsd2si`, which yields
//     the "integer indefinite" value INT_MIN. `d2i` below reproduces that
//     exactly instead of Rust's saturating `as` cast.
//   * `offset_days * 86400 + offset_hours * 3600` in `get_modified_time` is
//     `int` arithmetic that overflows for large inputs. gcc wraps; we use
//     `wrapping_*` and then sign-extend to `time_t`, matching the C.
//   * `hash *= 0x1F` and `bytes[i] << 24` in `hash_time_value` overflow signed
//     `int`. We compute in `u32` and reinterpret, matching gcc's wrapping.
//   * `apply_multiplier`'s `switch` has no `break` between cases 4..=1, so the
//     cases fall through and accumulate. That is reproduced literally.
//   * All stdout output goes through libc `printf` with the identical format
//     strings, so formatting (`%.2e`, `%X` of negatives, `-0.00e+00`) and
//     stdio buffering are byte-identical to the C.

#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_double, c_int, c_long};

/// C `time_t` on Linux/x86-64.
pub type time_t = i64;

unsafe extern "C" {
    /// Variadic libc `printf`, used so output formatting matches the C exactly.
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn time(t: *mut time_t) -> time_t;
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// `(int)x` with x86-64 `cvttsd2si` semantics: any value whose truncation does
/// not fit in an `i32` (including NaN and infinities) produces INT_MIN.
#[inline]
fn d2i(x: f64) -> i32 {
    if x.is_nan() {
        return i32::MIN;
    }
    let t = x.trunc();
    if t >= -2147483648.0 && t <= 2147483647.0 {
        t as i32
    } else {
        i32::MIN
    }
}

/// Equivalent to `strcmp(p, s) == 0` where `s` is a NUL-free byte literal.
///
/// # Safety
/// `p` must point to a NUL-terminated C string.
#[inline]
unsafe fn cstr_eq(p: *const c_char, s: &[u8]) -> bool {
    unsafe {
        for (i, &b) in s.iter().enumerate() {
            if *p.add(i) as u8 != b {
                return false;
            }
        }
        *p.add(s.len()) as u8 == 0
    }
}

// ---------------------------------------------------------------------------
// public ABI
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn classify_mode(mode: *const c_char) -> c_int {
    unsafe {
        if cstr_eq(mode, b"standard") {
            0x10
        } else if cstr_eq(mode, b"enhanced") {
            0x20
        } else if cstr_eq(mode, b"turbo") {
            0x30
        } else if cstr_eq(mode, b"extreme") {
            0x40
        } else {
            0x00
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn apply_multiplier(base: c_int, level: c_int) -> c_int {
    let mut result: c_int = base;

    // The C `switch` intentionally falls through cases 4 -> 3 -> 2 -> 1 -> 0.
    match level {
        4 => {
            result = result.wrapping_add(0xFF);
            result = result.wrapping_add(0xAB);
            result = result.wrapping_add(0x7E);
            result = result.wrapping_add(0x1C);
            result = result.wrapping_add(0x05);
        }
        3 => {
            result = result.wrapping_add(0xAB);
            result = result.wrapping_add(0x7E);
            result = result.wrapping_add(0x1C);
            result = result.wrapping_add(0x05);
        }
        2 => {
            result = result.wrapping_add(0x7E);
            result = result.wrapping_add(0x1C);
            result = result.wrapping_add(0x05);
        }
        1 => {
            result = result.wrapping_add(0x1C);
            result = result.wrapping_add(0x05);
        }
        0 => {
            result = result.wrapping_add(0x05);
        }
        _ => {
            result = 0xDEAD;
        }
    }

    result
}

#[unsafe(no_mangle)]
pub extern "C" fn convert_time_factor(factor: c_double) -> c_int {
    let scaled: f64 = factor * 1e12;
    d2i(scaled)
}

#[unsafe(no_mangle)]
pub extern "C" fn convert_negative_overflow(value: c_double) -> c_int {
    let extreme: f64 = value * -1e15;
    d2i(extreme)
}

#[unsafe(no_mangle)]
pub extern "C" fn get_modified_time(offset_days: c_int, offset_hours: c_int) -> time_t {
    // SAFETY: `time(NULL)` is always valid.
    let mut current: time_t = unsafe { time(std::ptr::null_mut()) };
    current >>= 29;

    // Computed in `int` in the C, so it wraps, then sign-extends to time_t.
    let offset_i32: c_int = offset_days
        .wrapping_mul(86400)
        .wrapping_add(offset_hours.wrapping_mul(3600));
    let offset: time_t = offset_i32 as time_t;

    current.wrapping_add(offset)
}

#[unsafe(no_mangle)]
pub extern "C" fn hash_time_value(t: time_t) -> c_int {
    let mut hash: u32 = 0x5A5A_5A5A;
    let bytes = t.to_ne_bytes();

    for i in 0..std::mem::size_of::<time_t>() {
        hash ^= (bytes[i] as u32) << ((i % 4) * 8);
        hash = hash.wrapping_mul(0x1F);
    }

    (hash & 0x7FFF_FFFF) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn modeselect(
    mode_selector: c_int,
    time_offset: c_int,
    complexity: c_int,
    seed: c_int,
) -> c_int {
    let mut result: c_int = 0;

    static MODES: [&[u8]; 4] = [
        b"standard\0",
        b"enhanced\0",
        b"turbo\0",
        b"extreme\0",
    ];

    let mode_index: c_int = mode_selector % 4;

    // The C indexes a 4-element array with `mode_selector % 4`, which is
    // negative for negative selectors and reads out of bounds (undefined; the C
    // prints stack garbage or crashes). There is no faithful reproduction of
    // that, so out-of-range indices fall back to an empty string, which yields
    // the same `classify_mode` result of 0x00 that the C produced.
    let selected_mode: *const c_char = if (0..4).contains(&mode_index) {
        MODES[mode_index as usize].as_ptr() as *const c_char
    } else {
        b"\0".as_ptr() as *const c_char
    };

    // SAFETY: `selected_mode` is a NUL-terminated static string.
    let mode_value: c_int = unsafe { classify_mode(selected_mode) };

    // SAFETY: format string is NUL-terminated and arguments match it.
    unsafe {
        printf(
            b"Selected mode: %s (0x%X)\n\0".as_ptr() as *const c_char,
            selected_mode,
            mode_value,
        );
    }
    result = result.wrapping_add(mode_value);

    let complexity_level: c_int = complexity % 5;
    let multiplier: c_int = apply_multiplier(0xA0, complexity_level);

    // SAFETY: format string is NUL-terminated and arguments match it.
    unsafe {
        printf(
            b"Complexity level: %d, Multiplier: 0x%X\n\0".as_ptr() as *const c_char,
            complexity_level,
            multiplier,
        );
    }
    result = result.wrapping_add(multiplier);

    let modified_time: time_t = get_modified_time(time_offset, seed % 24);
    let time_hash: c_int = hash_time_value(modified_time);

    // SAFETY: format string is NUL-terminated and arguments match it.
    unsafe {
        printf(
            b"Modified time: %ld, Hash: 0x%X\n\0".as_ptr() as *const c_char,
            modified_time as c_long,
            time_hash,
        );
    }
    result = result.wrapping_add(time_hash % 0x1000);

    let factor1: f64 = (seed as f64) * 1e8;
    let factor2: f64 = (time_offset as f64) * -1e7;

    // SAFETY: format string is NUL-terminated and arguments match it.
    unsafe {
        printf(
            b"Converting double %.2e to int (may overflow)...\n\0".as_ptr() as *const c_char,
            factor1,
        );
    }

    let result1: c_int = convert_time_factor(factor1);
    // SAFETY: format string is NUL-terminated and arguments match it.
    unsafe {
        printf(
            b"Result 1: %d (0x%X)\n\0".as_ptr() as *const c_char,
            result1,
            result1,
        );
    }

    // SAFETY: format string is NUL-terminated and arguments match it.
    unsafe {
        printf(
            b"Converting double %.2e to int (may underflow)...\n\0".as_ptr() as *const c_char,
            factor2,
        );
    }
    let result2: c_int = convert_negative_overflow(factor2);
    // SAFETY: format string is NUL-terminated and arguments match it.
    unsafe {
        printf(
            b"Result 2: %d (0x%X)\n\0".as_ptr() as *const c_char,
            result2,
            result2,
        );
    }

    result ^= result1 & 0xFF;
    result ^= result2 & 0xFF00;

    result = result.wrapping_mul(0x10).wrapping_add(0xBEEF);

    // SAFETY: format string is NUL-terminated and arguments match it.
    unsafe {
        printf(
            b"\nFinal result: %d (0x%X)\n\0".as_ptr() as *const c_char,
            result,
            result,
        );
    }

    result
}
