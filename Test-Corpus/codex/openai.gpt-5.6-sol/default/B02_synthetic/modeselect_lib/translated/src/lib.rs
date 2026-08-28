use std::ffi::{c_char, c_double, c_int, c_long};
use std::mem::size_of;

unsafe extern "C" {
    fn strcmp(left: *const c_char, right: *const c_char) -> c_int;
    fn time(timer: *mut c_long) -> c_long;
    fn printf(format: *const c_char, ...) -> c_int;
}

const STANDARD: &[u8] = b"standard\0";
const ENHANCED: &[u8] = b"enhanced\0";
const TURBO: &[u8] = b"turbo\0";
const EXTREME: &[u8] = b"extreme\0";

#[unsafe(no_mangle)]
pub extern "C" fn classify_mode(mode: *const c_char) -> c_int {
    unsafe {
        if strcmp(mode, STANDARD.as_ptr().cast()) == 0 {
            0x10
        } else if strcmp(mode, ENHANCED.as_ptr().cast()) == 0 {
            0x20
        } else if strcmp(mode, TURBO.as_ptr().cast()) == 0 {
            0x30
        } else if strcmp(mode, EXTREME.as_ptr().cast()) == 0 {
            0x40
        } else {
            0x00
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn apply_multiplier(base: c_int, level: c_int) -> c_int {
    match level {
        4 => base
            .wrapping_add(0xFF)
            .wrapping_add(0xAB)
            .wrapping_add(0x7E)
            .wrapping_add(0x1C)
            .wrapping_add(0x05),
        3 => base
            .wrapping_add(0xAB)
            .wrapping_add(0x7E)
            .wrapping_add(0x1C)
            .wrapping_add(0x05),
        2 => base
            .wrapping_add(0x7E)
            .wrapping_add(0x1C)
            .wrapping_add(0x05),
        1 => base.wrapping_add(0x1C).wrapping_add(0x05),
        0 => base.wrapping_add(0x05),
        _ => 0xDEAD,
    }
}

fn x86_double_to_int(value: c_double) -> c_int {
    if value.is_nan() || !(-2147483648.0..2147483648.0).contains(&value) {
        c_int::MIN
    } else {
        value as c_int
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn convert_time_factor(factor: c_double) -> c_int {
    x86_double_to_int(factor * 1e12)
}

#[unsafe(no_mangle)]
pub extern "C" fn convert_negative_overflow(value: c_double) -> c_int {
    x86_double_to_int(value * -1e15)
}

#[unsafe(no_mangle)]
pub extern "C" fn get_modified_time(offset_days: c_int, offset_hours: c_int) -> c_long {
    let current = unsafe { time(std::ptr::null_mut()) };
    let offset = offset_days
        .wrapping_mul(86400)
        .wrapping_add(offset_hours.wrapping_mul(3600));

    (current >> 29).wrapping_add(c_long::from(offset))
}

#[unsafe(no_mangle)]
pub extern "C" fn hash_time_value(t: c_long) -> c_int {
    let mut hash: c_int = 0x5A5A5A5A;
    let bytes = &t as *const c_long as *const u8;

    for i in 0..size_of::<c_long>() {
        let byte = unsafe { *bytes.add(i) };
        hash ^= c_int::from(byte).wrapping_shl(((i % 4) * 8) as u32);
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
    const SELECTED_MODE_FORMAT: &[u8] = b"Selected mode: %s (0x%X)\n\0";
    const COMPLEXITY_FORMAT: &[u8] = b"Complexity level: %d, Multiplier: 0x%X\n\0";
    const TIME_FORMAT: &[u8] = b"Modified time: %ld, Hash: 0x%X\n\0";
    const CONVERTING_INT_FORMAT: &[u8] =
        b"Converting double %.2e to int (may overflow)...\n\0";
    const RESULT_ONE_FORMAT: &[u8] = b"Result 1: %d (0x%X)\n\0";
    const CONVERTING_NEGATIVE_FORMAT: &[u8] =
        b"Converting double %.2e to int (may underflow)...\n\0";
    const RESULT_TWO_FORMAT: &[u8] = b"Result 2: %d (0x%X)\n\0";
    const FINAL_FORMAT: &[u8] = b"\nFinal result: %d (0x%X)\n\0";

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
        printf(
            SELECTED_MODE_FORMAT.as_ptr().cast(),
            selected_mode,
            mode_value,
        );
    }
    result = result.wrapping_add(mode_value);

    let complexity_level = complexity % 5;
    let multiplier = apply_multiplier(0xA0, complexity_level);

    unsafe {
        printf(
            COMPLEXITY_FORMAT.as_ptr().cast(),
            complexity_level,
            multiplier,
        );
    }
    result = result.wrapping_add(multiplier);

    let modified_time = get_modified_time(time_offset, seed % 24);
    let time_hash = hash_time_value(modified_time);

    unsafe {
        printf(
            TIME_FORMAT.as_ptr().cast(),
            modified_time,
            time_hash,
        );
    }
    result = result.wrapping_add(time_hash % 0x1000);

    let factor1 = c_double::from(seed) * 1e8;
    let factor2 = c_double::from(time_offset) * -1e7;

    unsafe {
        printf(CONVERTING_INT_FORMAT.as_ptr().cast(), factor1);
    }

    let result1 = convert_time_factor(factor1);
    unsafe {
        printf(RESULT_ONE_FORMAT.as_ptr().cast(), result1, result1);
        printf(CONVERTING_NEGATIVE_FORMAT.as_ptr().cast(), factor2);
    }

    let result2 = convert_negative_overflow(factor2);
    unsafe {
        printf(RESULT_TWO_FORMAT.as_ptr().cast(), result2, result2);
    }

    result ^= result1 & 0xFF;
    result ^= result2 & 0xFF00;

    result = result.wrapping_mul(0x10).wrapping_add(0xBEEF);

    unsafe {
        printf(FINAL_FORMAT.as_ptr().cast(), result, result);
    }

    result
}
