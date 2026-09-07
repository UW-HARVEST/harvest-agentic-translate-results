// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the “Software”),
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
// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
//
// Rust translation of c_src/src/lib.c -- ABI compatible, byte-identical output.

#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_double, c_int, c_long, c_void};

/// `time_t` on Linux/x86-64 (and every other 64-bit glibc target) is `long`.
pub type time_t = c_long;

unsafe extern "C" {
    /// Variadic `printf` from libc -- used directly so that every byte of the
    /// formatted output (including `%.2e` rounding and `%X` casing) is produced
    /// by the exact same code path as the original C library.
    #[link_name = "printf"]
    unsafe fn c_printf(format: *const c_char, ...) -> c_int;

    #[link_name = "strcmp"]
    unsafe fn c_strcmp(s1: *const c_char, s2: *const c_char) -> c_int;

    #[link_name = "time"]
    unsafe fn c_time(tloc: *mut time_t) -> time_t;
}

// ---------------------------------------------------------------------------
// String literals (NUL terminated, exactly as emitted by the C compiler).
// ---------------------------------------------------------------------------

const S_STANDARD: &[u8] = b"standard\0";
const S_ENHANCED: &[u8] = b"enhanced\0";
const S_TURBO: &[u8] = b"turbo\0";
const S_EXTREME: &[u8] = b"extreme\0";

const FMT_SELECTED_MODE: &[u8] = b"Selected mode: %s (0x%X)\n\0";
const FMT_COMPLEXITY: &[u8] = b"Complexity level: %d, Multiplier: 0x%X\n\0";
const FMT_MODIFIED_TIME: &[u8] = b"Modified time: %ld, Hash: 0x%X\n\0";
const FMT_CONVERT_OVERFLOW: &[u8] = b"Converting double %.2e to int (may overflow)...\n\0";
const FMT_CONVERT_UNDERFLOW: &[u8] = b"Converting double %.2e to int (may underflow)...\n\0";
const FMT_RESULT1: &[u8] = b"Result 1: %d (0x%X)\n\0";
const FMT_RESULT2: &[u8] = b"Result 2: %d (0x%X)\n\0";
const FMT_FINAL: &[u8] = b"\nFinal result: %d (0x%X)\n\0";

#[inline(always)]
fn cstr(bytes: &'static [u8]) -> *const c_char {
    bytes.as_ptr() as *const c_char
}

// ---------------------------------------------------------------------------
// Helpers reproducing C's (undefined-behaviour-laden) arithmetic exactly as it
// behaves on x86-64 with gcc.
// ---------------------------------------------------------------------------

/// `(int)double` on x86-64: `cvttsd2si` truncates toward zero and yields the
/// "integer indefinite" value 0x80000000 when the result is not representable
/// (this includes NaN and infinities).  C says this is undefined behaviour; the
/// original library relies on it, so we reproduce it bit for bit instead of
/// saturating the way Rust's `as` operator would.
#[inline]
fn d2i(v: c_double) -> c_int {
    if v.is_nan() {
        return c_int::MIN;
    }
    let truncated = v.trunc();
    if truncated >= -2147483648.0_f64 && truncated <= 2147483647.0_f64 {
        truncated as c_int
    } else {
        c_int::MIN
    }
}

// ---------------------------------------------------------------------------
// int classify_mode(const char *mode)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn classify_mode(mode: *const c_char) -> c_int {
    unsafe {
        if c_strcmp(mode, cstr(S_STANDARD)) == 0 {
            0x10
        } else if c_strcmp(mode, cstr(S_ENHANCED)) == 0 {
            0x20
        } else if c_strcmp(mode, cstr(S_TURBO)) == 0 {
            0x30
        } else if c_strcmp(mode, cstr(S_EXTREME)) == 0 {
            0x40
        } else {
            0x00
        }
    }
}

// ---------------------------------------------------------------------------
// int apply_multiplier(int base, int level)
//
// The C switch deliberately falls through from case 4 down to case 0.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn apply_multiplier(base: c_int, level: c_int) -> c_int {
    let mut result: c_int = base;

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

// ---------------------------------------------------------------------------
// int convert_time_factor(double factor)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn convert_time_factor(factor: c_double) -> c_int {
    let scaled: c_double = factor * 1e12;
    let result: c_int = d2i(scaled);

    result
}

// ---------------------------------------------------------------------------
// int convert_negative_overflow(double value)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn convert_negative_overflow(value: c_double) -> c_int {
    let extreme: c_double = value * -1e15;
    let result: c_int = d2i(extreme);

    result
}

// ---------------------------------------------------------------------------
// time_t get_modified_time(int offset_days, int offset_hours)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_modified_time(offset_days: c_int, offset_hours: c_int) -> time_t {
    let mut current: time_t = unsafe { c_time(std::ptr::null_mut()) };
    // Arithmetic (sign propagating) shift, as for a signed C integer type.
    current >>= 29;
    // The two products and their sum are evaluated in `int` in C and only then
    // widened to `time_t`; signed overflow wraps on the target ABI.
    let offset: time_t = offset_days
        .wrapping_mul(86400)
        .wrapping_add(offset_hours.wrapping_mul(3600)) as time_t;
    current.wrapping_add(offset)
}

// ---------------------------------------------------------------------------
// int hash_time_value(time_t t)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn hash_time_value(t: time_t) -> c_int {
    // `hash` is a signed int in C, but every operation performed on it (xor,
    // multiply, and finally masking with 0x7FFFFFFF) is bit-identical to the
    // wrapping unsigned computation below.
    let mut hash: u32 = 0x5A5A_5A5A;
    let bytes: [u8; std::mem::size_of::<time_t>()] = t.to_ne_bytes();

    for i in 0..std::mem::size_of::<time_t>() {
        hash ^= (bytes[i] as u32) << ((i % 4) * 8);
        hash = hash.wrapping_mul(0x1F);
    }

    (hash & 0x7FFF_FFFF) as c_int
}

// ---------------------------------------------------------------------------
// int modeselect(int mode_selector, int time_offset, int complexity, int seed)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn modeselect(
    mode_selector: c_int,
    time_offset: c_int,
    complexity: c_int,
    seed: c_int,
) -> c_int {
    unsafe {
        let mut result: c_int = 0;
        let modes: [*const c_char; 4] = [
            cstr(S_STANDARD),
            cstr(S_ENHANCED),
            cstr(S_TURBO),
            cstr(S_EXTREME),
        ];

        // C's `%` truncates toward zero, so a negative `mode_selector` yields a
        // negative index and an out-of-bounds read -- reproduced verbatim.
        let mode_index: c_int = mode_selector % 4;
        let selected_mode: *const c_char =
            std::ptr::read_volatile(modes.as_ptr().offset(mode_index as isize));
        let mode_value: c_int = classify_mode(selected_mode);

        c_printf(
            cstr(FMT_SELECTED_MODE),
            selected_mode as *const c_void,
            mode_value,
        );
        result = result.wrapping_add(mode_value);

        let complexity_level: c_int = complexity % 5;
        let multiplier: c_int = apply_multiplier(0xA0, complexity_level);

        c_printf(cstr(FMT_COMPLEXITY), complexity_level, multiplier);
        result = result.wrapping_add(multiplier);

        let modified_time: time_t = get_modified_time(time_offset, seed % 24);
        let time_hash: c_int = hash_time_value(modified_time);

        c_printf(
            cstr(FMT_MODIFIED_TIME),
            modified_time as c_long,
            time_hash,
        );
        result = result.wrapping_add(time_hash % 0x1000);

        let factor1: c_double = (seed as c_double) * 1e8;
        let factor2: c_double = (time_offset as c_double) * -1e7;

        c_printf(cstr(FMT_CONVERT_OVERFLOW), factor1);

        let result1: c_int = convert_time_factor(factor1);
        c_printf(cstr(FMT_RESULT1), result1, result1);

        c_printf(cstr(FMT_CONVERT_UNDERFLOW), factor2);
        let result2: c_int = convert_negative_overflow(factor2);
        c_printf(cstr(FMT_RESULT2), result2, result2);

        result ^= result1 & 0xFF;
        result ^= result2 & 0xFF00;

        result = result.wrapping_mul(0x10).wrapping_add(0xBEEF);

        c_printf(cstr(FMT_FINAL), result, result);

        result
    }
}
