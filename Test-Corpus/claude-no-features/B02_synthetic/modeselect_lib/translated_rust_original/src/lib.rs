// Translation of c_src/src/lib.c to Rust
// Reproduces byte-identical output to the C version on Linux x86_64.

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

#[allow(non_camel_case_types)]
type time_t = i64;
#[allow(non_camel_case_types)]
type c_long = i64;

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn time(tloc: *mut time_t) -> time_t;
}

fn classify_mode(mode: &CStr) -> i32 {
    match mode.to_bytes() {
        b"standard" => 0x10,
        b"enhanced" => 0x20,
        b"turbo" => 0x30,
        b"extreme" => 0x40,
        _ => 0x00,
    }
}

fn apply_multiplier(base: i32, level: i32) -> i32 {
    // Mimics C switch fall-through behavior.
    let mut result = base;
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

// Mimic C cast of double -> int as performed by x86_64 cvttsd2si:
// truncates toward zero; if out of range or NaN, returns 0x80000000 (INT_MIN).
fn double_to_int_c(v: f64) -> i32 {
    if v.is_nan() {
        return i32::MIN;
    }
    // i32 range: [-2^31, 2^31 - 1]. If outside [-2^31, 2^31), cvttsd2si returns INT_MIN.
    // Note: cvttsd2si returns INT_MIN for values >= 2^31 too.
    if v >= 2147483648.0 || v < -2147483648.0 {
        return i32::MIN;
    }
    // Truncation toward zero — for in-range values, `as i32` matches C cast.
    v as i32
}

fn convert_time_factor(factor: f64) -> i32 {
    let scaled = factor * 1e12;
    double_to_int_c(scaled)
}

fn convert_negative_overflow(value: f64) -> i32 {
    let extreme = value * -1e15;
    double_to_int_c(extreme)
}

fn get_modified_time(offset_days: c_int, offset_hours: c_int) -> time_t {
    let mut current: time_t = unsafe { time(std::ptr::null_mut()) };
    current = current >> 29;
    let offset: time_t =
        (offset_days as time_t).wrapping_mul(86400)
            .wrapping_add((offset_hours as time_t).wrapping_mul(3600));
    current.wrapping_add(offset)
}

fn hash_time_value(t: time_t) -> i32 {
    // Use u32 internally to avoid Rust's signed-shift/overflow checks
    // while preserving the C bit-level result.
    let mut hash: u32 = 0x5A5A5A5A;
    let bytes: [u8; 8] = t.to_ne_bytes();
    let size = std::mem::size_of::<time_t>();
    for i in 0..size {
        let shift = ((i % 4) * 8) as u32;
        hash ^= (bytes[i] as u32).wrapping_shl(shift);
        hash = hash.wrapping_mul(0x1F);
    }
    (hash & 0x7FFFFFFF) as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn modeselect(
    mode_selector: c_int,
    time_offset: c_int,
    complexity: c_int,
    seed: c_int,
) -> c_int {
    let mut result: i32 = 0;

    let modes: [&CStr; 4] = [
        unsafe { CStr::from_bytes_with_nul_unchecked(b"standard\0") },
        unsafe { CStr::from_bytes_with_nul_unchecked(b"enhanced\0") },
        unsafe { CStr::from_bytes_with_nul_unchecked(b"turbo\0") },
        unsafe { CStr::from_bytes_with_nul_unchecked(b"extreme\0") },
    ];

    let mode_index = (mode_selector % 4) as usize;
    let selected_mode = modes[mode_index];
    let mode_value = classify_mode(selected_mode);

    unsafe {
        printf(
            b"Selected mode: %s (0x%X)\n\0".as_ptr() as *const c_char,
            selected_mode.as_ptr(),
            mode_value as c_int,
        );
    }
    result = result.wrapping_add(mode_value);

    let complexity_level = complexity % 5;
    let multiplier = apply_multiplier(0xA0, complexity_level);

    unsafe {
        printf(
            b"Complexity level: %d, Multiplier: 0x%X\n\0".as_ptr() as *const c_char,
            complexity_level as c_int,
            multiplier as c_int,
        );
    }
    result = result.wrapping_add(multiplier);

    let modified_time = get_modified_time(time_offset, seed % 24);
    let time_hash = hash_time_value(modified_time);

    unsafe {
        printf(
            b"Modified time: %ld, Hash: 0x%X\n\0".as_ptr() as *const c_char,
            modified_time as c_long,
            time_hash as c_int,
        );
    }
    result = result.wrapping_add(time_hash % 0x1000);

    let factor1: f64 = (seed as f64) * 1e8;
    let factor2: f64 = (time_offset as f64) * -1e7;

    unsafe {
        printf(
            b"Converting double %.2e to int (may overflow)...\n\0".as_ptr() as *const c_char,
            factor1,
        );
    }

    let result1 = convert_time_factor(factor1);
    unsafe {
        printf(
            b"Result 1: %d (0x%X)\n\0".as_ptr() as *const c_char,
            result1 as c_int,
            result1 as c_int,
        );
    }

    unsafe {
        printf(
            b"Converting double %.2e to int (may underflow)...\n\0".as_ptr() as *const c_char,
            factor2,
        );
    }
    let result2 = convert_negative_overflow(factor2);
    unsafe {
        printf(
            b"Result 2: %d (0x%X)\n\0".as_ptr() as *const c_char,
            result2 as c_int,
            result2 as c_int,
        );
    }

    result ^= result1 & 0xFF;
    result ^= result2 & 0xFF00u32 as i32;

    result = result.wrapping_mul(0x10).wrapping_add(0xBEEF);

    unsafe {
        printf(
            b"\nFinal result: %d (0x%X)\n\0".as_ptr() as *const c_char,
            result as c_int,
            result as c_int,
        );
    }

    result as c_int
}
