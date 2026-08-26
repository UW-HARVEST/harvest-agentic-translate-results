use libc::time_t;
use std::ffi::{c_char, c_int};

const STANDARD: &[u8] = b"standard\0";
const ENHANCED: &[u8] = b"enhanced\0";
const TURBO: &[u8] = b"turbo\0";
const EXTREME: &[u8] = b"extreme\0";

const SELECTED_MODE_FMT: &[u8] = b"Selected mode: %s (0x%X)\n\0";
const COMPLEXITY_FMT: &[u8] = b"Complexity level: %d, Multiplier: 0x%X\n\0";
const MODIFIED_TIME_FMT: &[u8] = b"Modified time: %ld, Hash: 0x%X\n\0";
const CONVERT_OVERFLOW_FMT: &[u8] = b"Converting double %.2e to int (may overflow)...\n\0";
const RESULT_FMT: &[u8] = b"Result 1: %d (0x%X)\n\0";
const CONVERT_UNDERFLOW_FMT: &[u8] = b"Converting double %.2e to int (may underflow)...\n\0";
const RESULT2_FMT: &[u8] = b"Result 2: %d (0x%X)\n\0";
const FINAL_FMT: &[u8] = b"\nFinal result: %d (0x%X)\n\0";

fn classify_mode(mode: *const c_char) -> c_int {
    if unsafe { libc::strcmp(mode, STANDARD.as_ptr().cast::<c_char>()) } == 0 {
        0x10
    } else if unsafe { libc::strcmp(mode, ENHANCED.as_ptr().cast::<c_char>()) } == 0 {
        0x20
    } else if unsafe { libc::strcmp(mode, TURBO.as_ptr().cast::<c_char>()) } == 0 {
        0x30
    } else if unsafe { libc::strcmp(mode, EXTREME.as_ptr().cast::<c_char>()) } == 0 {
        0x40
    } else {
        0x00
    }
}

fn apply_multiplier(base: c_int, level: c_int) -> c_int {
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
            result = 0xDEAD_u32 as c_int;
        }
    }

    result
}

fn convert_like_c_int(value: f64) -> c_int {
    if value.is_nan() {
        0
    } else if value >= c_int::MAX as f64 || value <= c_int::MIN as f64 {
        c_int::MIN
    } else {
        value as c_int
    }
}

fn convert_time_factor(factor: f64) -> c_int {
    let scaled = factor * 1e12;
    convert_like_c_int(scaled)
}

fn convert_negative_overflow(value: f64) -> c_int {
    let extreme = value * -1e15;
    convert_like_c_int(extreme)
}

fn get_modified_time(offset_days: c_int, offset_hours: c_int) -> time_t {
    let mut current = unsafe { libc::time(std::ptr::null_mut()) };
    current >>= 29;
    let offset =
        (offset_days as time_t).wrapping_mul(86_400) + (offset_hours as time_t).wrapping_mul(3_600);
    current.wrapping_add(offset)
}

fn hash_time_value(t: time_t) -> c_int {
    let mut hash: c_int = 0x5A5A5A5A_u32 as c_int;
    let bytes = t.to_ne_bytes();

    for (i, byte) in bytes.iter().enumerate() {
        let shifted = (*byte as c_int) << ((i % 4) * 8);
        hash ^= shifted;
        hash = hash.wrapping_mul(0x1F);
    }

    hash & 0x7FFF_FFFF
}

unsafe fn printf_i32x2(fmt: &[u8], a: c_int, b: c_int) {
    unsafe {
        libc::printf(fmt.as_ptr().cast::<c_char>(), a, b);
    }
}

unsafe fn printf_double(fmt: &[u8], value: f64) {
    unsafe {
        libc::printf(fmt.as_ptr().cast::<c_char>(), value);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn modeselect(
    mode_selector: c_int,
    time_offset: c_int,
    complexity: c_int,
    seed: c_int,
) -> c_int {
    let mut result: c_int = 0;
    let modes = [
        STANDARD.as_ptr().cast::<c_char>(),
        ENHANCED.as_ptr().cast::<c_char>(),
        TURBO.as_ptr().cast::<c_char>(),
        EXTREME.as_ptr().cast::<c_char>(),
    ];

    let mode_index = mode_selector % 4;
    let selected_mode = unsafe { *modes.as_ptr().offset(mode_index as isize) };
    let mode_value = classify_mode(selected_mode);

    unsafe {
        libc::printf(
            SELECTED_MODE_FMT.as_ptr().cast::<c_char>(),
            selected_mode,
            mode_value,
        );
    }
    result = result.wrapping_add(mode_value);

    let complexity_level = complexity % 5;
    let multiplier = apply_multiplier(0xA0, complexity_level);

    unsafe {
        printf_i32x2(COMPLEXITY_FMT, complexity_level, multiplier);
    }
    result = result.wrapping_add(multiplier);

    let modified_time = get_modified_time(time_offset, seed % 24);
    let time_hash = hash_time_value(modified_time);

    unsafe {
        libc::printf(
            MODIFIED_TIME_FMT.as_ptr().cast::<c_char>(),
            modified_time as libc::c_long,
            time_hash,
        );
    }
    result = result.wrapping_add(time_hash % 0x1000);

    let factor1 = (seed as f64) * 1e8;
    let factor2 = (time_offset as f64) * -1e7;

    unsafe {
        printf_double(CONVERT_OVERFLOW_FMT, factor1);
    }
    let result1 = convert_time_factor(factor1);
    unsafe {
        printf_i32x2(RESULT_FMT, result1, result1);
    }

    unsafe {
        printf_double(CONVERT_UNDERFLOW_FMT, factor2);
    }
    let result2 = convert_negative_overflow(factor2);
    unsafe {
        printf_i32x2(RESULT2_FMT, result2, result2);
    }

    result ^= result1 & 0xFF;
    result ^= result2 & 0xFF00;

    result = result.wrapping_mul(0x10).wrapping_add(0xBEEF_u32 as c_int);

    unsafe {
        printf_i32x2(FINAL_FMT, result, result);
    }

    result
}
