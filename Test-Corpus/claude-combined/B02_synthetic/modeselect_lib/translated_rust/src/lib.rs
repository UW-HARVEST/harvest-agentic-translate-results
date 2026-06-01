// Rust translation of c_src/src/lib.c
// Produces byte-identical output by delegating printf to libc.

use std::ffi::c_char;
use std::os::raw::{c_double, c_int, c_long, c_void};

// time_t on Linux x86_64 is a 64-bit signed integer.
#[allow(non_camel_case_types)]
type time_t = c_long;

extern "C" {
    fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
    fn time(t: *mut time_t) -> time_t;
    fn printf(fmt: *const c_char, ...) -> c_int;
}

// --- format strings (NUL-terminated, matching the C source) ---
static FMT_SELECTED_MODE: &[u8] = b"Selected mode: %s (0x%X)\n\0";
static FMT_COMPLEXITY: &[u8] = b"Complexity level: %d, Multiplier: 0x%X\n\0";
static FMT_MODIFIED_TIME: &[u8] = b"Modified time: %ld, Hash: 0x%X\n\0";
static FMT_CONVERTING_OVERFLOW: &[u8] = b"Converting double %.2e to int (may overflow)...\n\0";
static FMT_RESULT1: &[u8] = b"Result 1: %d (0x%X)\n\0";
static FMT_CONVERTING_UNDERFLOW: &[u8] = b"Converting double %.2e to int (may underflow)...\n\0";
static FMT_RESULT2: &[u8] = b"Result 2: %d (0x%X)\n\0";
static FMT_FINAL: &[u8] = b"\nFinal result: %d (0x%X)\n\0";

// Mode strings. NUL-terminated for use with strcmp/printf %s.
static MODE_STANDARD: &[u8] = b"standard\0";
static MODE_ENHANCED: &[u8] = b"enhanced\0";
static MODE_TURBO: &[u8] = b"turbo\0";
static MODE_EXTREME: &[u8] = b"extreme\0";

#[unsafe(no_mangle)]
pub extern "C" fn classify_mode(mode: *const c_char) -> c_int {
    unsafe {
        if strcmp(mode, MODE_STANDARD.as_ptr() as *const c_char) == 0 {
            return 0x10;
        } else if strcmp(mode, MODE_ENHANCED.as_ptr() as *const c_char) == 0 {
            return 0x20;
        } else if strcmp(mode, MODE_TURBO.as_ptr() as *const c_char) == 0 {
            return 0x30;
        } else if strcmp(mode, MODE_EXTREME.as_ptr() as *const c_char) == 0 {
            return 0x40;
        }
        0x00
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn apply_multiplier(base: c_int, level: c_int) -> c_int {
    // Reproduces the C switch statement with fallthroughs exactly.
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

#[inline]
fn f64_to_i32_c(v: f64) -> c_int {
    // Match the C behavior on x86_64: cvttsd2si returns INT_MIN (0x80000000)
    // when the value is out of range or NaN. Rust's `as i32` saturates, which
    // does NOT match C. Use to_int_unchecked which lowers to fptosi/cvttsd2si.
    // For in-range values both produce the same result; for out-of-range we
    // need cvttsd2si semantics. to_int_unchecked is UB for out-of-range, but
    // on x86_64 LLVM lowers it to cvttsd2si, which matches the C compiler's
    // emitted code for `(int)double_expr`.
    unsafe { v.to_int_unchecked::<c_int>() }
}

#[unsafe(no_mangle)]
pub extern "C" fn convert_time_factor(factor: c_double) -> c_int {
    let scaled = factor * 1e12;
    f64_to_i32_c(scaled)
}

#[unsafe(no_mangle)]
pub extern "C" fn convert_negative_overflow(value: c_double) -> c_int {
    let extreme = value * -1e15;
    f64_to_i32_c(extreme)
}

#[unsafe(no_mangle)]
pub extern "C" fn get_modified_time(offset_days: c_int, offset_hours: c_int) -> time_t {
    let mut current: time_t = unsafe { time(std::ptr::null_mut()) };
    // C: current = current >> 29; (signed arithmetic shift)
    current >>= 29;
    // C: time_t offset = (offset_days * 86400) + (offset_hours * 3600);
    // The multiplications are done in `int` (the type of the operands), then
    // promoted/assigned to time_t. Replicate that exactly.
    let mul1: c_int = (offset_days as c_int).wrapping_mul(86400);
    let mul2: c_int = (offset_hours as c_int).wrapping_mul(3600);
    let sum: c_int = mul1.wrapping_add(mul2);
    let offset: time_t = sum as time_t;
    current.wrapping_add(offset)
}

#[unsafe(no_mangle)]
pub extern "C" fn hash_time_value(t: time_t) -> c_int {
    let mut hash: c_int = 0x5A5A5A5A;
    let bytes = t.to_ne_bytes();
    for i in 0..std::mem::size_of::<time_t>() {
        let b = bytes[i] as c_int;
        let shift = ((i % 4) * 8) as u32;
        hash ^= b.wrapping_shl(shift);
        hash = hash.wrapping_mul(0x1F);
    }
    hash & 0x7FFFFFFF
}

#[unsafe(no_mangle)]
pub extern "C" fn modeselect(
    mode_selector: c_int,
    time_offset: c_int,
    complexity: c_int,
    seed: c_int,
) -> c_int {
    let mut result: c_int = 0;
    let modes: [&[u8]; 4] = [MODE_STANDARD, MODE_ENHANCED, MODE_TURBO, MODE_EXTREME];

    // C: int mode_index = mode_selector % 4;  -- C's % can be negative.
    let mode_index = mode_selector % 4;
    let selected_mode = modes[mode_index as usize];
    let mode_value = classify_mode(selected_mode.as_ptr() as *const c_char);

    unsafe {
        printf(
            FMT_SELECTED_MODE.as_ptr() as *const c_char,
            selected_mode.as_ptr() as *const c_char,
            mode_value,
        );
    }
    result = result.wrapping_add(mode_value);

    let complexity_level = complexity % 5;
    let multiplier = apply_multiplier(0xA0, complexity_level);

    unsafe {
        printf(
            FMT_COMPLEXITY.as_ptr() as *const c_char,
            complexity_level,
            multiplier,
        );
    }
    result = result.wrapping_add(multiplier);

    let modified_time = get_modified_time(time_offset, seed % 24);
    let time_hash = hash_time_value(modified_time);

    unsafe {
        printf(
            FMT_MODIFIED_TIME.as_ptr() as *const c_char,
            modified_time as c_long,
            time_hash,
        );
    }
    result = result.wrapping_add(time_hash % 0x1000);

    let factor1: f64 = (seed as f64) * 1e8;
    let factor2: f64 = (time_offset as f64) * -1e7;

    unsafe {
        printf(FMT_CONVERTING_OVERFLOW.as_ptr() as *const c_char, factor1);
    }

    let result1 = convert_time_factor(factor1);
    unsafe {
        printf(FMT_RESULT1.as_ptr() as *const c_char, result1, result1);
    }

    unsafe {
        printf(FMT_CONVERTING_UNDERFLOW.as_ptr() as *const c_char, factor2);
    }
    let result2 = convert_negative_overflow(factor2);
    unsafe {
        printf(FMT_RESULT2.as_ptr() as *const c_char, result2, result2);
    }

    result ^= result1 & 0xFF;
    result ^= result2 & 0xFF00;

    result = result.wrapping_mul(0x10).wrapping_add(0xBEEF);

    unsafe {
        printf(FMT_FINAL.as_ptr() as *const c_char, result, result);
    }

    // suppress unused warning
    let _ = std::ptr::null::<c_void>();

    result
}
