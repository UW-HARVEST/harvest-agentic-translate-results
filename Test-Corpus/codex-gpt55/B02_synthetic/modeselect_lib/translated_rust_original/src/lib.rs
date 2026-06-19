use std::ffi::{c_char, c_double, c_int, c_long, c_uint};

type TimeT = c_long;

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn strcmp(left: *const c_char, right: *const c_char) -> c_int;
    fn time(timer: *mut TimeT) -> TimeT;
}

const STANDARD: &[u8] = b"standard\0";
const ENHANCED: &[u8] = b"enhanced\0";
const TURBO: &[u8] = b"turbo\0";
const EXTREME: &[u8] = b"extreme\0";

const SELECTED_MODE_FMT: &[u8] = b"Selected mode: %s (0x%X)\n\0";
const COMPLEXITY_FMT: &[u8] = b"Complexity level: %d, Multiplier: 0x%X\n\0";
const MODIFIED_TIME_FMT: &[u8] = b"Modified time: %ld, Hash: 0x%X\n\0";
const CONVERT_TIME_FMT: &[u8] = b"Converting double %.2e to int (may overflow)...\n\0";
const RESULT1_FMT: &[u8] = b"Result 1: %d (0x%X)\n\0";
const CONVERT_NEGATIVE_FMT: &[u8] =
    b"Converting double %.2e to int (may underflow)...\n\0";
const RESULT2_FMT: &[u8] = b"Result 2: %d (0x%X)\n\0";
const FINAL_RESULT_FMT: &[u8] = b"\nFinal result: %d (0x%X)\n\0";

fn cstr(bytes: &'static [u8]) -> *const c_char {
    bytes.as_ptr().cast()
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn double_to_c_int(value: c_double) -> c_int {
    let out: c_int;
    unsafe {
        core::arch::asm!(
            "cvttsd2si {out:e}, {input}",
            input = in(xmm_reg) value,
            out = lateout(reg) out,
            options(nostack, preserves_flags)
        );
    }
    out
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
fn double_to_c_int(value: c_double) -> c_int {
    value as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn classify_mode(mode: *const c_char) -> c_int {
    if unsafe { strcmp(mode, cstr(STANDARD)) } == 0 {
        0x10
    } else if unsafe { strcmp(mode, cstr(ENHANCED)) } == 0 {
        0x20
    } else if unsafe { strcmp(mode, cstr(TURBO)) } == 0 {
        0x30
    } else if unsafe { strcmp(mode, cstr(EXTREME)) } == 0 {
        0x40
    } else {
        0x00
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn apply_multiplier(base: c_int, level: c_int) -> c_int {
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

#[unsafe(no_mangle)]
pub extern "C" fn convert_time_factor(factor: c_double) -> c_int {
    let scaled = factor * 1e12;
    double_to_c_int(scaled)
}

#[unsafe(no_mangle)]
pub extern "C" fn convert_negative_overflow(value: c_double) -> c_int {
    let extreme = value * -1e15;
    double_to_c_int(extreme)
}

#[unsafe(no_mangle)]
pub extern "C" fn get_modified_time(offset_days: c_int, offset_hours: c_int) -> TimeT {
    let mut current = unsafe { time(std::ptr::null_mut()) };
    current >>= 29;
    let offset = offset_days
        .wrapping_mul(86400)
        .wrapping_add(offset_hours.wrapping_mul(3600)) as TimeT;
    current.wrapping_add(offset)
}

#[unsafe(no_mangle)]
pub extern "C" fn hash_time_value(t: TimeT) -> c_int {
    let mut hash: c_int = 0x5A5A5A5A;
    let bytes = t.to_ne_bytes();

    for (i, byte) in bytes.iter().enumerate() {
        let part = (*byte as c_int).wrapping_shl(((i % 4) * 8) as u32);
        hash ^= part;
        hash = hash.wrapping_mul(0x1F);
    }

    hash & 0x7FFFFFFF
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn modeselect(
    mode_selector: c_int,
    time_offset: c_int,
    complexity: c_int,
    seed: c_int,
) -> c_int {
    let mut result: c_int = 0;
    let modes = [
        cstr(STANDARD),
        cstr(ENHANCED),
        cstr(TURBO),
        cstr(EXTREME),
    ];

    let mode_index = mode_selector % 4;
    let selected_mode = unsafe { *modes.as_ptr().offset(mode_index as isize) };
    let mode_value = unsafe { classify_mode(selected_mode) };

    unsafe {
        printf(
            cstr(SELECTED_MODE_FMT),
            selected_mode,
            mode_value as c_uint,
        );
    }
    result = result.wrapping_add(mode_value);

    let complexity_level = complexity % 5;
    let multiplier = apply_multiplier(0xA0, complexity_level);

    unsafe {
        printf(
            cstr(COMPLEXITY_FMT),
            complexity_level,
            multiplier as c_uint,
        );
    }
    result = result.wrapping_add(multiplier);

    let modified_time = get_modified_time(time_offset, seed % 24);
    let time_hash = hash_time_value(modified_time);

    unsafe {
        printf(
            cstr(MODIFIED_TIME_FMT),
            modified_time,
            time_hash as c_uint,
        );
    }
    result = result.wrapping_add(time_hash % 0x1000);

    let factor1 = (seed as c_double) * 1e8;
    let factor2 = (time_offset as c_double) * -1e7;

    unsafe {
        printf(cstr(CONVERT_TIME_FMT), factor1);
    }

    let result1 = convert_time_factor(factor1);
    unsafe {
        printf(cstr(RESULT1_FMT), result1, result1 as c_uint);
    }

    unsafe {
        printf(cstr(CONVERT_NEGATIVE_FMT), factor2);
    }
    let result2 = convert_negative_overflow(factor2);
    unsafe {
        printf(cstr(RESULT2_FMT), result2, result2 as c_uint);
    }

    result ^= result1 & 0xFF;
    result ^= result2 & 0xFF00;

    result = result.wrapping_mul(0x10).wrapping_add(0xBEEF);

    unsafe {
        printf(cstr(FINAL_RESULT_FMT), result, result as c_uint);
    }

    result
}
