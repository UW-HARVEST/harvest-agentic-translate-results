use std::os::raw::c_int;
use std::time::{SystemTime, UNIX_EPOCH};

fn classify_mode(mode: &str) -> i32 {
    match mode {
        "standard" => 0x10,
        "enhanced" => 0x20,
        "turbo" => 0x30,
        "extreme" => 0x40,
        _ => 0x00,
    }
}

fn apply_multiplier(base: i32, level: i32) -> i32 {
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

fn convert_time_factor(factor: f64) -> i32 {
    let scaled = factor * 1e12;
    scaled as i32
}

fn convert_negative_overflow(value: f64) -> i32 {
    let extreme = value * -1e15;
    extreme as i32
}

fn get_modified_time(offset_days: i32, offset_hours: i32) -> i64 {
    let current = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let current = current >> 29;
    let offset = (offset_days as i64 * 86400) + (offset_hours as i64 * 3600);
    current + offset
}

fn hash_time_value(t: i64) -> i32 {
    let mut hash: i32 = 0x5A5A5A5A;
    let bytes = t.to_ne_bytes();

    for i in 0..bytes.len() {
        hash ^= (bytes[i] as i32) << ((i % 4) * 8);
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
    let mut result: i32 = 0;
    let modes = ["standard", "enhanced", "turbo", "extreme"];

    let mode_index = mode_selector.rem_euclid(4) as usize;
    let selected_mode = modes[mode_index];
    let mode_value = classify_mode(selected_mode);

    println!("Selected mode: {} (0x{:X})", selected_mode, mode_value as u32);
    result = result.wrapping_add(mode_value);

    let complexity_level = complexity % 5;
    let multiplier = apply_multiplier(0xA0, complexity_level);

    println!(
        "Complexity level: {}, Multiplier: 0x{:X}",
        complexity_level, multiplier as u32
    );
    result = result.wrapping_add(multiplier);

    let modified_time = get_modified_time(time_offset, seed % 24);
    let time_hash = hash_time_value(modified_time);

    println!(
        "Modified time: {}, Hash: 0x{:X}",
        modified_time, time_hash as u32
    );
    result = result.wrapping_add(time_hash % 0x1000);

    let factor1 = (seed as f64) * 1e8;
    let factor2 = (time_offset as f64) * -1e7;

    println!("Converting double {:.2e} to int (may overflow)...", factor1);

    let result1 = convert_time_factor(factor1);
    println!("Result 1: {} (0x{:X})", result1, result1 as u32);

    println!("Converting double {:.2e} to int (may underflow)...", factor2);
    let result2 = convert_negative_overflow(factor2);
    println!("Result 2: {} (0x{:X})", result2, result2 as u32);

    result ^= result1 & 0xFF;
    result ^= result2 & 0xFF00;

    result = result.wrapping_mul(0x10).wrapping_add(0xBEEF);

    println!("\nFinal result: {} (0x{:X})", result, result as u32);

    result
}
