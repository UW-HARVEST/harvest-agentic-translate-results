// Translated from C source. Preserves exact byte-identical output by calling
// the C standard library's printf directly, matching x86-64 CVTTSD2SI semantics
// for double->int conversions, and using wrapping arithmetic to match C's
// signed integer overflow behavior on x86-64.

#![allow(clippy::missing_safety_doc)]

use std::ffi::{c_char, c_double, c_int, c_long};
use std::ptr;

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn time(t: *mut c_long) -> c_long;
    fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
}

// Match x86-64 CVTTSD2SI semantics: out-of-range / NaN / infinity => INT_MIN.
// In Rust, `as i32` is saturating, which differs from C's behavior on x86-64.
#[inline]
fn cvttsd2si(val: f64) -> i32 {
    if val.is_nan() || val >= 2147483648.0_f64 || val < -2147483648.0_f64 {
        i32::MIN
    } else {
        // Safety: val is finite, not NaN, and after truncation fits in i32.
        unsafe { val.to_int_unchecked::<i32>() }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn classify_mode(mode: *const c_char) -> c_int {
    if unsafe { strcmp(mode, b"standard\0".as_ptr() as *const c_char) } == 0 {
        return 0x10;
    } else if unsafe { strcmp(mode, b"enhanced\0".as_ptr() as *const c_char) } == 0 {
        return 0x20;
    } else if unsafe { strcmp(mode, b"turbo\0".as_ptr() as *const c_char) } == 0 {
        return 0x30;
    } else if unsafe { strcmp(mode, b"extreme\0".as_ptr() as *const c_char) } == 0 {
        return 0x40;
    }
    0x00
}

#[unsafe(no_mangle)]
pub extern "C" fn apply_multiplier(base: c_int, level: c_int) -> c_int {
    let mut result: c_int = base;

    // Reproduce C switch fallthrough exactly.
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
    cvttsd2si(scaled)
}

#[unsafe(no_mangle)]
pub extern "C" fn convert_negative_overflow(value: c_double) -> c_int {
    let extreme: f64 = value * -1e15;
    cvttsd2si(extreme)
}

#[unsafe(no_mangle)]
pub extern "C" fn get_modified_time(offset_days: c_int, offset_hours: c_int) -> c_long {
    let mut current: c_long = unsafe { time(ptr::null_mut()) };
    current >>= 29;
    // In C, `(offset_days * 86400) + (offset_hours * 3600)` is computed as `int`
    // (with potential signed overflow as UB) and then assigned to `time_t`.
    // Reproduce by performing the arithmetic in i32 with wrapping, then
    // sign-extending to time_t (i64 on Linux x86-64).
    let offset_int: c_int = offset_days
        .wrapping_mul(86400)
        .wrapping_add(offset_hours.wrapping_mul(3600));
    let offset: c_long = offset_int as c_long;
    current.wrapping_add(offset)
}

#[unsafe(no_mangle)]
pub extern "C" fn hash_time_value(t: c_long) -> c_int {
    let mut hash: c_int = 0x5A5A5A5A;
    // On Linux x86-64, time_t is i64 stored little-endian.
    let bytes = t.to_le_bytes();
    for i in 0..bytes.len() {
        let shift: u32 = ((i % 4) * 8) as u32;
        // In C, `bytes[i] << shift` integer-promotes to int. For values
        // where bit 31 ends up set, the result is technically UB; in
        // practice gcc emits the raw bit pattern. Use u32 shift then
        // bit-cast to i32 to reproduce that bit pattern safely.
        let shifted: u32 = (bytes[i] as u32).wrapping_shl(shift);
        hash ^= shifted as i32;
        hash = hash.wrapping_mul(0x1F);
    }
    hash & 0x7FFF_FFFF
}

#[unsafe(no_mangle)]
pub extern "C" fn modeselect(
    mode_selector: c_int,
    time_offset: c_int,
    complexity: c_int,
    seed: c_int,
) -> c_int {
    // Matches C: const char *modes[] = {"standard", "enhanced", "turbo", "extreme"};
    static MODES: [&[u8]; 4] = [
        b"standard\0",
        b"enhanced\0",
        b"turbo\0",
        b"extreme\0",
    ];

    let mut result: c_int = 0;

    let mode_index_signed: c_int = mode_selector % 4;
    // In C, `modes[mode_index]` with negative mode_index is UB. We assume the
    // intended (non-negative) input range for this index.
    let mode_index: usize = mode_index_signed as usize;
    let selected_mode: *const c_char = MODES[mode_index].as_ptr() as *const c_char;
    let mode_value: c_int = unsafe { classify_mode(selected_mode) };

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

    unsafe {
        printf(
            b"Complexity level: %d, Multiplier: 0x%X\n\0".as_ptr() as *const c_char,
            complexity_level,
            multiplier,
        );
    }
    result = result.wrapping_add(multiplier);

    let modified_time: c_long = get_modified_time(time_offset, seed % 24);
    let time_hash: c_int = hash_time_value(modified_time);

    unsafe {
        printf(
            b"Modified time: %ld, Hash: 0x%X\n\0".as_ptr() as *const c_char,
            modified_time as c_long,
            time_hash,
        );
    }
    // time_hash is non-negative (sign bit cleared), 0x1000 is positive.
    result = result.wrapping_add(time_hash % 0x1000);

    let factor1: f64 = (seed as f64) * 1e8;
    let factor2: f64 = (time_offset as f64) * -1e7;

    unsafe {
        printf(
            b"Converting double %.2e to int (may overflow)...\n\0".as_ptr() as *const c_char,
            factor1,
        );
    }

    let result1: c_int = convert_time_factor(factor1);
    unsafe {
        printf(
            b"Result 1: %d (0x%X)\n\0".as_ptr() as *const c_char,
            result1,
            result1,
        );
    }

    unsafe {
        printf(
            b"Converting double %.2e to int (may underflow)...\n\0".as_ptr() as *const c_char,
            factor2,
        );
    }
    let result2: c_int = convert_negative_overflow(factor2);
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

    unsafe {
        printf(
            b"\nFinal result: %d (0x%X)\n\0".as_ptr() as *const c_char,
            result,
            result,
        );
    }

    result
}
