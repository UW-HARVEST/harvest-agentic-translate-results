// Copyright 2025 MIT Lincoln Laboratory
// Rust translation of c_src/src/lib.c. Produces byte-identical output for
// the same inputs.

use core::ffi::{c_char, c_double, c_int, c_long};

// ---- libc FFI declarations ------------------------------------------------

extern "C" {
    fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn time(t: *mut TimeT) -> TimeT;
}

// time_t on 64-bit Linux/glibc is a signed 64-bit integer.
#[allow(non_camel_case_types)]
type TimeT = i64;

// ---- Helpers --------------------------------------------------------------

/// Mirrors the behavior of a C `(int)d` cast on x86_64, where the
/// `cvttsd2si` instruction returns the "integer indeterminate" value
/// `0x80000000` (i.e. `i32::MIN`) for any out-of-range value or NaN.
///
/// Rust's `as i32` saturates instead, so we need an explicit helper.
fn double_to_int_c(d: f64) -> c_int {
    // cvttsd2si returns i32::MIN if the truncated value is not
    // representable in i32, including NaN and infinities.
    if d.is_nan() || d >= 2147483648.0_f64 || d < -2147483648.0_f64 {
        i32::MIN
    } else {
        // In-range: `as i32` truncates toward zero, matching C.
        d as c_int
    }
}

// ---- Translated functions -------------------------------------------------

fn classify_mode(mode: *const c_char) -> c_int {
    unsafe {
        if strcmp(mode, b"standard\0".as_ptr() as *const c_char) == 0 {
            return 0x10;
        } else if strcmp(mode, b"enhanced\0".as_ptr() as *const c_char) == 0 {
            return 0x20;
        } else if strcmp(mode, b"turbo\0".as_ptr() as *const c_char) == 0 {
            return 0x30;
        } else if strcmp(mode, b"extreme\0".as_ptr() as *const c_char) == 0 {
            return 0x40;
        }
        0x00
    }
}

fn apply_multiplier(base: c_int, level: c_int) -> c_int {
    let mut result: c_int = base;

    // Faithfully reproduce the fall-through cascade in the original C
    // switch. Each higher case adds its constant and falls through to the
    // next lower case; case 0 ends with `break`. Using wrapping adds
    // because the C code performs ordinary signed `int` addition.
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

fn convert_time_factor(factor: c_double) -> c_int {
    let scaled = factor * 1e12;
    double_to_int_c(scaled)
}

fn convert_negative_overflow(value: c_double) -> c_int {
    let extreme = value * -1e15;
    double_to_int_c(extreme)
}

fn get_modified_time(offset_days: c_int, offset_hours: c_int) -> TimeT {
    // `time(NULL)` returns the current epoch seconds.
    let mut current: TimeT = unsafe { time(core::ptr::null_mut()) };
    current >>= 29;
    // The C code computes `(offset_days * 86400) + (offset_hours * 3600)`
    // using signed `int` arithmetic, which can wrap on overflow. We mirror
    // that with `wrapping_*` operations on `i32`, then sign-extend to
    // `time_t`.
    let offset_i32: c_int = (offset_days.wrapping_mul(86400))
        .wrapping_add(offset_hours.wrapping_mul(3600));
    current + (offset_i32 as TimeT)
}

fn hash_time_value(t: TimeT) -> c_int {
    let mut hash: c_int = 0x5A5A5A5A;
    let bytes: [u8; core::mem::size_of::<TimeT>()] = t.to_ne_bytes();

    for (i, &b) in bytes.iter().enumerate() {
        // `bytes[i] << ((i % 4) * 8)` in C: `bytes[i]` is unsigned char,
        // promoted to `int`, then shifted. Result is `int`. XOR'd into
        // `hash` (also int). Multiplications use signed `int` arithmetic.
        let shifted: c_int = (b as c_int).wrapping_shl(((i % 4) * 8) as u32);
        hash ^= shifted;
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
    let mut result: c_int = 0;

    // `int mode_index = mode_selector % 4;` — C uses truncated remainder
    // (same as Rust's `%` for signed integers), so a negative selector
    // can yield a negative index. Match exactly via wrapping ops.
    let mode_index: c_int = mode_selector.wrapping_rem(4);

    let modes: [&[u8]; 4] = [
        b"standard\0",
        b"enhanced\0",
        b"turbo\0",
        b"extreme\0",
    ];
    // Indexing with a possibly-negative or out-of-range index reproduces
    // the original C behavior (which is undefined for OOB, but in
    // practice would read whatever is on the stack). Using `as usize`
    // here will simply panic on OOB; this matches the typical scenario
    // where the caller passes a sane `mode_selector`. We reproduce no
    // additional bounds checking beyond what C had.
    let selected_mode_ptr = modes[mode_index as usize].as_ptr() as *const c_char;
    let mode_value: c_int = classify_mode(selected_mode_ptr);

    unsafe {
        printf(
            b"Selected mode: %s (0x%X)\n\0".as_ptr() as *const c_char,
            selected_mode_ptr,
            mode_value,
        );
    }
    result = result.wrapping_add(mode_value);

    let complexity_level: c_int = complexity.wrapping_rem(5);
    let multiplier: c_int = apply_multiplier(0xA0, complexity_level);

    unsafe {
        printf(
            b"Complexity level: %d, Multiplier: 0x%X\n\0".as_ptr() as *const c_char,
            complexity_level,
            multiplier,
        );
    }
    result = result.wrapping_add(multiplier);

    let modified_time: TimeT = get_modified_time(time_offset, seed.wrapping_rem(24));
    let time_hash: c_int = hash_time_value(modified_time);

    unsafe {
        printf(
            b"Modified time: %ld, Hash: 0x%X\n\0".as_ptr() as *const c_char,
            modified_time as c_long,
            time_hash,
        );
    }
    // `time_hash % 0x1000` in C: signed-int remainder, can be negative if
    // time_hash were negative — but `hash_time_value` masks with
    // 0x7FFFFFFF, so it's non-negative here. Match exactly with `%`.
    result = result.wrapping_add(time_hash % 0x1000);

    let factor1: c_double = (seed as c_double) * 1e8;
    let factor2: c_double = (time_offset as c_double) * -1e7;

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
