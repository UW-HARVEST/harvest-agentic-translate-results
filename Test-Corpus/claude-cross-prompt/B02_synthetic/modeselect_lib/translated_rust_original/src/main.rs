use std::io::{self, Read};
use std::time::{SystemTime, UNIX_EPOCH};

fn classify_mode(mode: &str) -> i32 {
    if mode == "standard" {
        0x10
    } else if mode == "enhanced" {
        0x20
    } else if mode == "turbo" {
        0x30
    } else if mode == "extreme" {
        0x40
    } else {
        0x00
    }
}

fn apply_multiplier(base: i32, level: i32) -> i32 {
    // Replicates C's switch fall-through.
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

/// Emulates x86-64 cvttsd2si: out-of-range or NaN -> i32::MIN (0x80000000).
fn d2i_x86(v: f64) -> i32 {
    if v.is_nan() || v >= 2147483648.0 || v < -2147483648.0 {
        i32::MIN
    } else {
        // In-range conversion truncates toward zero, matching cvttsd2si.
        v as i32
    }
}

fn convert_time_factor(factor: f64) -> i32 {
    let scaled = factor * 1e12;
    d2i_x86(scaled)
}

fn convert_negative_overflow(value: f64) -> i32 {
    let extreme = value * -1e15;
    d2i_x86(extreme)
}

fn get_modified_time(offset_days: i32, offset_hours: i32) -> i64 {
    let current_full = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let current = current_full >> 29;
    // C does the multiplications and additions as int, then assigns to time_t.
    let offset_i32 = offset_days
        .wrapping_mul(86400)
        .wrapping_add(offset_hours.wrapping_mul(3600));
    current.wrapping_add(offset_i32 as i64)
}

fn hash_time_value(t: i64) -> i32 {
    let mut hash: i32 = 0x5A5A5A5Ai32;
    // x86-64 Linux: time_t is little-endian 8-byte signed.
    let bytes = t.to_le_bytes();
    for i in 0..bytes.len() {
        let shift = ((i % 4) * 8) as u32;
        // C: bytes[i] (unsigned char) is promoted to int, then shifted.
        let val = (bytes[i] as i32).wrapping_shl(shift);
        hash ^= val;
        hash = hash.wrapping_mul(0x1F);
    }
    hash & 0x7FFFFFFF
}

/// Format f64 like C's `%.2e` on glibc: e.g. "1.00e+08", "-1.50e-03", "0.00e+00".
fn fmt_c_e(v: f64) -> String {
    // Rust's `{:.2e}` produces the right mantissa rounding, but exponent
    // lacks sign and zero-padding, e.g. "1.00e8" instead of "1.00e+08".
    let s = format!("{:.2e}", v);
    if let Some((mantissa, exp)) = s.split_once('e') {
        let exp_num: i32 = exp.parse().unwrap_or(0);
        if exp_num >= 0 {
            format!("{}e+{:02}", mantissa, exp_num)
        } else {
            format!("{}e-{:02}", mantissa, -exp_num)
        }
    } else {
        s
    }
}

fn modeselect(mode_selector: i32, time_offset: i32, complexity: i32, seed: i32) -> i32 {
    let mut result: i32 = 0;
    let modes = ["standard", "enhanced", "turbo", "extreme"];

    let mode_index = mode_selector % 4;
    // Match C's UB-on-negative behavior pragmatically: if negative, panic
    // (there is no defined behavior in the C either).
    let selected_mode = modes[mode_index as usize];
    let mode_value = classify_mode(selected_mode);

    println!(
        "Selected mode: {} (0x{:X})",
        selected_mode,
        mode_value as u32
    );
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

    println!(
        "Converting double {} to int (may overflow)...",
        fmt_c_e(factor1)
    );

    let result1 = convert_time_factor(factor1);
    println!("Result 1: {} (0x{:X})", result1, result1 as u32);

    println!(
        "Converting double {} to int (may underflow)...",
        fmt_c_e(factor2)
    );
    let result2 = convert_negative_overflow(factor2);
    println!("Result 2: {} (0x{:X})", result2, result2 as u32);

    result ^= result1 & 0xFF;
    result ^= result2 & 0xFF00;

    result = result.wrapping_mul(0x10).wrapping_add(0xBEEF);

    println!();
    println!("Final result: {} (0x{:X})", result, result as u32);

    result
}

fn read_4_ints() -> (i32, i32, i32, i32) {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).ok();
    let mut iter = input.split_whitespace();
    let a: i32 = iter.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let b: i32 = iter.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let c: i32 = iter.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let d: i32 = iter.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    (a, b, c, d)
}

fn main() {
    let (a, b, c, d) = read_4_ints();
    modeselect(a, b, c, d);
}
