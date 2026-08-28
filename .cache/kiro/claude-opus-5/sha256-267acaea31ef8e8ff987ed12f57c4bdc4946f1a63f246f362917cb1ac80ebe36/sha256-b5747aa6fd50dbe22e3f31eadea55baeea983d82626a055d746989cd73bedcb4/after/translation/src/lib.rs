// Rust translation of c_src/src/lib.c
//
// Behaviour-preserving port. All observable output is produced through the C
// library's `printf` so that formatting (`%.2e`, `%X`, ...) and stdio buffering
// are byte-identical to the original.
//
// Quirks of the C code that are deliberately reproduced (not "fixed"):
//   * `apply_multiplier` relies on switch fall-through.
//   * `convert_time_factor` / `convert_negative_overflow` perform out-of-range
//     double -> int casts.  On x86-64 the generated `cvttsd2si` yields the
//     "integer indefinite" value 0x80000000 (INT_MIN); Rust's `as` would
//     saturate instead, so the cast is emulated explicitly.
//   * `modeselect` computes `mode_selector % 4`, which is negative for negative
//     selectors and therefore indexes `modes[]` out of bounds.
//   * All integer arithmetic wraps (signed overflow in C is UB, in practice
//     two's-complement wrap-around).

use std::ffi::{c_char, c_double, c_int, c_long};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn time(tloc: *mut TimeT) -> TimeT;
}

/// `time_t` on x86-64 Linux.
#[allow(non_camel_case_types)]
type TimeT = i64;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// `strcmp(p, lit) == 0`, where `lit` carries no trailing NUL.
///
/// Reads `p` byte-by-byte exactly like `strcmp` does: it stops at the first
/// difference, so it never reads past the end of a shorter string.
unsafe fn c_str_eq(p: *const c_char, lit: &[u8]) -> bool {
    for (i, &b) in lit.iter().enumerate() {
        if unsafe { *p.add(i) } as u8 != b {
            return false;
        }
    }
    (unsafe { *p.add(lit.len()) }) as u8 == 0
}

/// C's `(int)double` conversion as implemented by `cvttsd2si` on x86-64:
/// truncate toward zero, and yield `INT_MIN` for NaN / infinities / values
/// whose truncation does not fit in an `int`.
fn double_to_int(v: f64) -> c_int {
    let truncated = v.trunc();
    if truncated >= -2147483648.0 && truncated <= 2147483647.0 {
        truncated as c_int
    } else {
        c_int::MIN
    }
}

// ---------------------------------------------------------------------------
// public API
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn classify_mode(mode: *const c_char) -> c_int {
    if unsafe { c_str_eq(mode, b"standard") } {
        0x10
    } else if unsafe { c_str_eq(mode, b"enhanced") } {
        0x20
    } else if unsafe { c_str_eq(mode, b"turbo") } {
        0x30
    } else if unsafe { c_str_eq(mode, b"extreme") } {
        0x40
    } else {
        0x00
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn apply_multiplier(base: c_int, level: c_int) -> c_int {
    let mut result = base;

    // Fall-through accumulation from the original `switch`.
    match level {
        4 => result = result.wrapping_add(0xFF + 0xAB + 0x7E + 0x1C + 0x05),
        3 => result = result.wrapping_add(0xAB + 0x7E + 0x1C + 0x05),
        2 => result = result.wrapping_add(0x7E + 0x1C + 0x05),
        1 => result = result.wrapping_add(0x1C + 0x05),
        0 => result = result.wrapping_add(0x05),
        _ => result = 0xDEAD,
    }

    result
}

#[unsafe(no_mangle)]
pub extern "C" fn convert_time_factor(factor: c_double) -> c_int {
    let scaled = factor * 1e12;
    double_to_int(scaled)
}

#[unsafe(no_mangle)]
pub extern "C" fn convert_negative_overflow(value: c_double) -> c_int {
    let extreme = value * -1e15;
    double_to_int(extreme)
}

#[unsafe(no_mangle)]
pub extern "C" fn get_modified_time(offset_days: c_int, offset_hours: c_int) -> TimeT {
    let mut current: TimeT = unsafe { time(std::ptr::null_mut()) };
    current >>= 29;

    // The C expression is evaluated in `int` and only then widened to time_t.
    let offset_int = offset_days
        .wrapping_mul(86400)
        .wrapping_add(offset_hours.wrapping_mul(3600));
    let offset = offset_int as TimeT;

    current.wrapping_add(offset)
}

#[unsafe(no_mangle)]
pub extern "C" fn hash_time_value(t: TimeT) -> c_int {
    let mut hash: u32 = 0x5A5A5A5A;
    // Object representation of `t` on a little-endian target.
    let bytes = t.to_le_bytes();

    for i in 0..size_of::<TimeT>() {
        hash ^= (bytes[i] as u32) << ((i % 4) * 8);
        hash = hash.wrapping_mul(0x1F);
    }

    (hash as c_int) & 0x7FFFFFFF
}

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

    let mode_index = mode_selector % 4;
    // Faithful to the C: a negative `mode_index` indexes out of bounds.
    let selected_mode = unsafe { *modes.as_ptr().offset(mode_index as isize) };
    let mode_value = unsafe { classify_mode(selected_mode) };

    unsafe {
        printf(
            c"Selected mode: %s (0x%X)\n".as_ptr(),
            selected_mode,
            mode_value,
        );
    }
    result = result.wrapping_add(mode_value);

    let complexity_level = complexity % 5;
    let multiplier = apply_multiplier(0xA0, complexity_level);

    unsafe {
        printf(
            c"Complexity level: %d, Multiplier: 0x%X\n".as_ptr(),
            complexity_level,
            multiplier,
        );
    }
    result = result.wrapping_add(multiplier);

    let modified_time = get_modified_time(time_offset, seed % 24);
    let time_hash = hash_time_value(modified_time);

    unsafe {
        printf(
            c"Modified time: %ld, Hash: 0x%X\n".as_ptr(),
            modified_time as c_long,
            time_hash,
        );
    }
    result = result.wrapping_add(time_hash % 0x1000);

    let factor1 = (seed as f64) * 1e8;
    let factor2 = (time_offset as f64) * -1e7;

    unsafe {
        printf(
            c"Converting double %.2e to int (may overflow)...\n".as_ptr(),
            factor1,
        );
    }

    let result1 = convert_time_factor(factor1);
    unsafe {
        printf(c"Result 1: %d (0x%X)\n".as_ptr(), result1, result1);
    }

    unsafe {
        printf(
            c"Converting double %.2e to int (may underflow)...\n".as_ptr(),
            factor2,
        );
    }
    let result2 = convert_negative_overflow(factor2);
    unsafe {
        printf(c"Result 2: %d (0x%X)\n".as_ptr(), result2, result2);
    }

    result ^= result1 & 0xFF;
    result ^= result2 & 0xFF00;

    result = result.wrapping_mul(0x10).wrapping_add(0xBEEF);

    unsafe {
        printf(c"\nFinal result: %d (0x%X)\n".as_ptr(), result, result);
    }

    result
}
