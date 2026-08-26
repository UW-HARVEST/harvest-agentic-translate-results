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
// This translation is intentionally bug-for-bug compatible with the original C
// implementation: signed integer overflow wraps (as GCC emits on x86-64),
// out-of-range double -> int conversions yield the x86-64 `cvttsd2si`
// "integer indefinite" value (INT_MIN), and all diagnostic output is emitted
// through the C library's `printf` so the bytes (and the stdout buffering
// behaviour) are identical to the C version.

#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_char, c_double, c_int, c_long};

// ---------------------------------------------------------------------------
// libc bindings (used so formatting / string handling matches C exactly)
// ---------------------------------------------------------------------------
unsafe extern "C" {
    #[link_name = "printf"]
    unsafe fn c_printf(fmt: *const c_char, ...) -> c_int;
    #[link_name = "strcmp"]
    unsafe fn c_strcmp(a: *const c_char, b: *const c_char) -> c_int;
    #[link_name = "time"]
    unsafe fn c_time(tloc: *mut time_t) -> time_t;
}

/// `time_t` on Linux/x86-64 (64-bit signed).
#[allow(non_camel_case_types)]
pub type time_t = i64;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Reproduce the x86-64 `cvttsd2si` (double -> int32) conversion used by the C
/// compiler, including the out-of-range / NaN behaviour (which is undefined in
/// C but deterministically yields INT_MIN on this target). Note that Rust's
/// `as` cast saturates instead, so the range checks below are required.
#[inline]
fn double_to_int(value: c_double) -> c_int {
    if value.is_nan() || value >= 2147483648.0 || value <= -2147483648.0 {
        i32::MIN as c_int
    } else {
        value as c_int
    }
}

// ---------------------------------------------------------------------------
// public ABI
// ---------------------------------------------------------------------------

/// int classify_mode(const char *mode);
#[unsafe(no_mangle)]
pub unsafe extern "C" fn classify_mode(mode: *const c_char) -> c_int {
    unsafe {
        if c_strcmp(mode, c"standard".as_ptr()) == 0 {
            return 0x10;
        } else if c_strcmp(mode, c"enhanced".as_ptr()) == 0 {
            return 0x20;
        } else if c_strcmp(mode, c"turbo".as_ptr()) == 0 {
            return 0x30;
        } else if c_strcmp(mode, c"extreme".as_ptr()) == 0 {
            return 0x40;
        }
        0x00
    }
}

/// int apply_multiplier(int base, int level);
///
/// The C `switch` deliberately falls through from one case to the next, so the
/// accumulated constants are additive.
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

/// int convert_time_factor(double factor);
#[unsafe(no_mangle)]
pub extern "C" fn convert_time_factor(factor: c_double) -> c_int {
    let scaled: c_double = factor * 1e12;
    double_to_int(scaled)
}

/// int convert_negative_overflow(double value);
#[unsafe(no_mangle)]
pub extern "C" fn convert_negative_overflow(value: c_double) -> c_int {
    let extreme: c_double = value * -1e15;
    double_to_int(extreme)
}

/// time_t get_modified_time(int offset_days, int offset_hours);
///
/// The offset is computed with `int` arithmetic in C (and only then widened to
/// `time_t`), so large day counts wrap around 32 bits.
#[unsafe(no_mangle)]
pub extern "C" fn get_modified_time(offset_days: c_int, offset_hours: c_int) -> time_t {
    let mut current: time_t = unsafe { c_time(core::ptr::null_mut()) };
    current >>= 29;
    let offset: time_t = offset_days
        .wrapping_mul(86400)
        .wrapping_add(offset_hours.wrapping_mul(3600)) as time_t;
    current.wrapping_add(offset)
}

/// int hash_time_value(time_t t);
#[unsafe(no_mangle)]
pub extern "C" fn hash_time_value(t: time_t) -> c_int {
    let mut hash: c_int = 0x5A5A5A5Au32 as c_int;
    let bytes = t.to_ne_bytes();

    for i in 0..core::mem::size_of::<time_t>() {
        let shift = ((i % 4) * 8) as u32;
        hash ^= (bytes[i] as c_int).wrapping_shl(shift);
        hash = hash.wrapping_mul(0x1F);
    }

    hash & 0x7FFF_FFFF
}

/// int modeselect(int mode_selector, int time_offset, int complexity, int seed);
#[unsafe(no_mangle)]
pub extern "C" fn modeselect(
    mode_selector: c_int,
    time_offset: c_int,
    complexity: c_int,
    seed: c_int,
) -> c_int {
    let mut result: c_int = 0;
    let modes: [*const c_char; 4] = [
        c"standard".as_ptr(),
        c"enhanced".as_ptr(),
        c"turbo".as_ptr(),
        c"extreme".as_ptr(),
    ];

    let mode_index: c_int = mode_selector % 4;
    // C's `%` truncates towards zero, so a negative `mode_selector` produces a
    // negative index and `modes[mode_index]` reads past the start of the array
    // (undefined behaviour). In practice the garbage value is not a valid
    // string pointer and the following strcmp faults; reproduce that fault.
    let selected_mode: *const c_char = if (0..4).contains(&mode_index) {
        modes[mode_index as usize]
    } else {
        core::ptr::null()
    };
    let mode_value: c_int = unsafe { classify_mode(selected_mode) };

    unsafe {
        c_printf(
            c"Selected mode: %s (0x%X)\n".as_ptr(),
            selected_mode,
            mode_value,
        );
    }
    result = result.wrapping_add(mode_value);

    let complexity_level: c_int = complexity % 5;
    let multiplier: c_int = apply_multiplier(0xA0, complexity_level);

    unsafe {
        c_printf(
            c"Complexity level: %d, Multiplier: 0x%X\n".as_ptr(),
            complexity_level,
            multiplier,
        );
    }
    result = result.wrapping_add(multiplier);

    let modified_time: time_t = get_modified_time(time_offset, seed % 24);
    let time_hash: c_int = hash_time_value(modified_time);

    unsafe {
        c_printf(
            c"Modified time: %ld, Hash: 0x%X\n".as_ptr(),
            modified_time as c_long,
            time_hash,
        );
    }
    result = result.wrapping_add(time_hash % 0x1000);

    let factor1: c_double = (seed as c_double) * 1e8;
    let factor2: c_double = (time_offset as c_double) * -1e7;

    unsafe {
        c_printf(
            c"Converting double %.2e to int (may overflow)...\n".as_ptr(),
            factor1,
        );
    }

    let result1: c_int = convert_time_factor(factor1);
    unsafe {
        c_printf(c"Result 1: %d (0x%X)\n".as_ptr(), result1, result1);
    }

    unsafe {
        c_printf(
            c"Converting double %.2e to int (may underflow)...\n".as_ptr(),
            factor2,
        );
    }
    let result2: c_int = convert_negative_overflow(factor2);
    unsafe {
        c_printf(c"Result 2: %d (0x%X)\n".as_ptr(), result2, result2);
    }

    result ^= result1 & 0xFF;
    result ^= result2 & 0xFF00;

    result = result.wrapping_mul(0x10).wrapping_add(0xBEEF);

    unsafe {
        c_printf(c"\nFinal result: %d (0x%X)\n".as_ptr(), result, result);
    }

    result
}
