use libc::time_t;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

fn classify_mode(mode: &CStr) -> c_int {
    if mode.to_bytes() == b"standard" {
        0x10
    } else if mode.to_bytes() == b"enhanced" {
        0x20
    } else if mode.to_bytes() == b"turbo" {
        0x30
    } else if mode.to_bytes() == b"extreme" {
        0x40
    } else {
        0x00
    }
}

fn apply_multiplier(base: c_int, level: c_int) -> c_int {
    let mut result = base;
    match level {
        4 => {
            result += 0xFF;
            result += 0xAB;
            result += 0x7E;
            result += 0x1C;
            result += 0x05;
        }
        3 => {
            result += 0xAB;
            result += 0x7E;
            result += 0x1C;
            result += 0x05;
        }
        2 => {
            result += 0x7E;
            result += 0x1C;
            result += 0x05;
        }
        1 => {
            result += 0x1C;
            result += 0x05;
        }
        0 => {
            result += 0x05;
        }
        _ => {
            result = 0xDEAD;
        }
    }
    result
}

fn convert_time_factor(factor: f64) -> c_int {
    let scaled = factor * 1e12;
    scaled as c_int
}

fn convert_negative_overflow(value: f64) -> c_int {
    let extreme = value * -1e15;
    extreme as c_int
}

fn get_modified_time(offset_days: c_int, offset_hours: c_int) -> time_t {
    let mut current = unsafe { libc::time(std::ptr::null_mut()) };
    current >>= 29;
    let offset = (offset_days as time_t * 86400) + (offset_hours as time_t * 3600);
    current + offset
}

fn hash_time_value(t: time_t) -> c_int {
    let mut hash: c_int = 0x5A5A5A5A;
    let bytes = t.to_ne_bytes();
    for (i, byte) in bytes.iter().enumerate() {
        hash ^= (*byte as c_int) << ((i % 4) * 8);
        hash = hash.wrapping_mul(0x1F);
    }
    hash & 0x7FFFFFFF
}

#[unsafe(no_mangle)]
pub extern "C" fn modeselect(mode_selector: c_int, time_offset: c_int, complexity: c_int, seed: c_int) -> c_int {
    let mut result: c_int = 0;
    let modes: [&[u8]; 4] = [b"standard\0", b"enhanced\0", b"turbo\0", b"extreme\0"];

    let mode_index = mode_selector % 4;
    let selected_mode_bytes = modes[mode_index as usize];
    let selected_mode = CStr::from_bytes_with_nul(selected_mode_bytes).unwrap();
    let mode_value = classify_mode(selected_mode);

    println!("Selected mode: {} (0x{:X})", selected_mode.to_string_lossy(), mode_value);
    result += mode_value;

    let complexity_level = complexity % 5;
    let multiplier = apply_multiplier(0xA0, complexity_level);

    println!("Complexity level: {}, Multiplier: 0x{:X}", complexity_level, multiplier);
    result += multiplier;

    let modified_time = get_modified_time(time_offset, seed % 24);
    let time_hash = hash_time_value(modified_time);

    println!("Modified time: {}, Hash: 0x{:X}", modified_time as i64, time_hash);
    result += time_hash % 0x1000;

    let factor1 = seed as f64 * 1e8;
    let factor2 = time_offset as f64 * -1e7;

    println!("Converting double {:.2e} to int (may overflow)...", factor1);
    let result1 = convert_time_factor(factor1);
    println!("Result 1: {} (0x{:X})", result1, result1);

    println!("Converting double {:.2e} to int (may underflow)...", factor2);
    let result2 = convert_negative_overflow(factor2);
    println!("Result 2: {} (0x{:X})", result2, result2);

    result ^= result1 & 0xFF;
    result ^= result2 & 0xFF00;

    result = result.wrapping_mul(0x10).wrapping_add(0xBEEF);

    println!("\nFinal result: {} (0x{:X})", result, result);

    result
}
