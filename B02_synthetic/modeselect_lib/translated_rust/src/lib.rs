use std::ffi::{c_char, c_int, CStr};
use std::os::raw::c_double;

use libc::time_t;

fn classify_mode(mode: *const c_char) -> c_int {
    let s = unsafe { CStr::from_ptr(mode) }.to_bytes();
    if s == b"standard" {
        0x10
    } else if s == b"enhanced" {
        0x20
    } else if s == b"turbo" {
        0x30
    } else if s == b"extreme" {
        0x40
    } else {
        0x00
    }
}

fn apply_multiplier(base: c_int, level: c_int) -> c_int {
    let mut result = base;
    // Reproduce C switch fallthrough
    match level {
        0 => {
            result += 0x05;
        }
        1 => {
            result += 0x1C + 0x05;
        }
        2 => {
            result += 0x7E + 0x1C + 0x05;
        }
        3 => {
            result += 0xAB + 0x7E + 0x1C + 0x05;
        }
        4 => {
            result += 0xFF + 0xAB + 0x7E + 0x1C + 0x05;
        }
        _ => {
            result = 0xDEADu32 as c_int;
        }
    }
    result
}

fn convert_time_factor(factor: c_double) -> c_int {
    let scaled = factor * 1e12;
    scaled as c_int
}

fn convert_negative_overflow(value: c_double) -> c_int {
    let extreme = value * -1e15;
    extreme as c_int
}

fn get_modified_time(offset_days: c_int, offset_hours: c_int) -> time_t {
    let mut current: time_t = unsafe { libc::time(std::ptr::null_mut()) };
    current >>= 29;
    let offset: time_t = (offset_days as time_t) * 86400 + (offset_hours as time_t) * 3600;
    current + offset
}

fn hash_time_value(t: time_t) -> c_int {
    let mut hash: c_int = 0x5A5A5A5Au32 as c_int;
    let bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(&t as *const time_t as *const u8, std::mem::size_of::<time_t>())
    };

    for (i, &b) in bytes.iter().enumerate() {
        hash ^= (b as c_int) << ((i % 4) * 8);
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

    static MODES: [&[u8]; 4] = [b"standard\0", b"enhanced\0", b"turbo\0", b"extreme\0"];

    // C: mode_selector % 4  (truncation toward zero)
    let mode_index = (mode_selector % 4) as isize;
    let mode_index = if mode_index < 0 {
        (mode_index + 4) as usize
    } else {
        mode_index as usize
    };
    let selected_mode = MODES[mode_index].as_ptr() as *const c_char;
    let mode_value = classify_mode(selected_mode);

    let mode_name = unsafe { CStr::from_ptr(selected_mode) };
    unsafe {
        libc::printf(
            b"Selected mode: %s (0x%X)\n\0".as_ptr() as *const c_char,
            mode_name.as_ptr(),
            mode_value as c_int,
        );
    }
    result += mode_value;

    // C: complexity % 5 (truncation toward zero, then used as index 0..4)
    let complexity_level = complexity % 5;
    let multiplier = apply_multiplier(0xA0, complexity_level);

    unsafe {
        libc::printf(
            b"Complexity level: %d, Multiplier: 0x%X\n\0".as_ptr() as *const c_char,
            complexity_level as c_int,
            multiplier as c_int,
        );
    }
    result += multiplier;

    let modified_time = get_modified_time(time_offset, seed % 24);
    let time_hash = hash_time_value(modified_time);

    unsafe {
        libc::printf(
            b"Modified time: %ld, Hash: 0x%X\n\0".as_ptr() as *const c_char,
            modified_time as libc::c_long,
            time_hash as c_int,
        );
    }
    result += time_hash % 0x1000;

    let factor1: c_double = (seed as c_double) * 1e8;
    let factor2: c_double = (time_offset as c_double) * -1e7;

    unsafe {
        libc::printf(
            b"Converting double %.2e to int (may overflow)...\n\0".as_ptr() as *const c_char,
            factor1,
        );
    }

    let result1 = convert_time_factor(factor1);
    unsafe {
        libc::printf(
            b"Result 1: %d (0x%X)\n\0".as_ptr() as *const c_char,
            result1 as c_int,
            result1 as c_int,
        );
    }

    unsafe {
        libc::printf(
            b"Converting double %.2e to int (may underflow)...\n\0".as_ptr() as *const c_char,
            factor2,
        );
    }
    let result2 = convert_negative_overflow(factor2);
    unsafe {
        libc::printf(
            b"Result 2: %d (0x%X)\n\0".as_ptr() as *const c_char,
            result2 as c_int,
            result2 as c_int,
        );
    }

    result ^= result1 & 0xFF;
    result ^= result2 & 0xFF00u32 as c_int;

    result = result.wrapping_mul(0x10).wrapping_add(0xBEEF);

    unsafe {
        libc::printf(
            b"\nFinal result: %d (0x%X)\n\0".as_ptr() as *const c_char,
            result as c_int,
            result as c_int,
        );
    }

    result
}
