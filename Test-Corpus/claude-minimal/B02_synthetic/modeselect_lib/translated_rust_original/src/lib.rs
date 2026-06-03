// Translated from c_src/src/lib.c

use std::time::{SystemTime, UNIX_EPOCH};

pub fn classify_mode(mode: &str) -> i32 {
    match mode {
        "standard" => 0x10,
        "enhanced" => 0x20,
        "turbo" => 0x30,
        "extreme" => 0x40,
        _ => 0x00,
    }
}

pub fn apply_multiplier(base: i32, level: i32) -> i32 {
    let mut result: i32 = base;

    // Mirrors the C switch statement with intentional fallthrough.
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

/// Mimics the C semantics of converting a double to an `int` on x86_64
/// (cvttsd2si), where out-of-range or NaN values produce `INT_MIN`
/// (0x80000000). Rust's `as i32` saturates instead.
fn f64_to_i32_c(d: f64) -> i32 {
    if d.is_nan() || d >= 2147483648.0 || d < -2147483648.0 {
        i32::MIN
    } else {
        d as i32
    }
}

pub fn convert_time_factor(factor: f64) -> i32 {
    let scaled = factor * 1e12;
    f64_to_i32_c(scaled)
}

pub fn convert_negative_overflow(value: f64) -> i32 {
    let extreme = value * -1e15;
    f64_to_i32_c(extreme)
}

pub fn get_modified_time(offset_days: i32, offset_hours: i32) -> i64 {
    let current = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let current = current >> 29;
    let offset = (offset_days as i64) * 86400 + (offset_hours as i64) * 3600;
    current + offset
}

pub fn hash_time_value(t: i64) -> i32 {
    let mut hash: i32 = 0x5A5A5A5Ai32;
    let bytes = t.to_ne_bytes();

    for i in 0..bytes.len() {
        hash ^= (bytes[i] as i32).wrapping_shl(((i % 4) * 8) as u32);
        hash = hash.wrapping_mul(0x1F);
    }

    hash & 0x7FFFFFFF
}

pub fn modeselect(mode_selector: i32, time_offset: i32, complexity: i32, seed: i32) -> i32 {
    let mut result: i32 = 0;
    let modes = ["standard", "enhanced", "turbo", "extreme"];

    // C uses the truncating modulo, which can be negative for negative inputs.
    let mode_index = (mode_selector % 4).rem_euclid(4) as usize;
    // To keep behavior identical to C when mode_selector is non-negative,
    // use the truncated remainder; for negative inputs, fall back via rem_euclid.
    let mode_index = if mode_selector >= 0 {
        (mode_selector % 4) as usize
    } else {
        mode_index
    };
    let selected_mode = modes[mode_index];
    let mode_value = classify_mode(selected_mode);

    println!("Selected mode: {} (0x{:X})", selected_mode, mode_value);
    result = result.wrapping_add(mode_value);

    let complexity_level = if complexity >= 0 {
        complexity % 5
    } else {
        complexity.rem_euclid(5)
    };
    let multiplier = apply_multiplier(0xA0, complexity_level);

    println!(
        "Complexity level: {}, Multiplier: 0x{:X}",
        complexity_level, multiplier
    );
    result = result.wrapping_add(multiplier);

    let seed_mod_24 = if seed >= 0 { seed % 24 } else { seed.rem_euclid(24) };
    let modified_time = get_modified_time(time_offset, seed_mod_24);
    let time_hash = hash_time_value(modified_time);

    println!(
        "Modified time: {}, Hash: 0x{:X}",
        modified_time, time_hash
    );
    result = result.wrapping_add(time_hash % 0x1000);

    let factor1: f64 = (seed as f64) * 1e8;
    let factor2: f64 = (time_offset as f64) * -1e7;

    println!("Converting double {:.2e} to int (may overflow)...", factor1);

    let result1 = convert_time_factor(factor1);
    println!("Result 1: {} (0x{:X})", result1, result1);

    println!("Converting double {:.2e} to int (may underflow)...", factor2);
    let result2 = convert_negative_overflow(factor2);
    println!("Result 2: {} (0x{:X})", result2, result2);

    result ^= result1 & 0xFF;
    result ^= result2 & 0xFF00u32 as i32;

    result = result.wrapping_mul(0x10).wrapping_add(0xBEEF);

    println!("\nFinal result: {} (0x{:X})", result, result);

    result
}
