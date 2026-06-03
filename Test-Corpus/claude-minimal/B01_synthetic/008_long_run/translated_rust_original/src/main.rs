// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust.

use std::process::ExitCode;

const ARRAY_SIZE: usize = 256 * 1024; // 1MB assuming sizeof(int) = 4
const ITERATIONS: usize = 2000;

// Global array (boxed via Vec to avoid stack overflow on the heap is fine in static mem too).
// Using a static mut would require unsafe; we'll use a Box<[i32; ARRAY_SIZE]>-like via Vec.
fn perform_expensive_operations(array: &mut [i32]) {
    for slot in array.iter_mut() {
        let mut x: i32 = *slot;
        for _ in 0..100 {
            // x = x * 3 + 7;
            x = x.wrapping_mul(3).wrapping_add(7);
            // x = x ^ (x >> 3);
            // In C, signed >> on negative is implementation-defined (commonly arithmetic).
            // Rust performs arithmetic shift on signed integers, matching typical C behavior.
            x ^= x >> 3;
            // x = x - (x << 1);
            // C left-shift of signed negative is UB; emulate two's complement wrap.
            let shifted = (x as u32).wrapping_shl(1) as i32;
            x = x.wrapping_sub(shifted);
            // x = x / 2 + x % 7;
            // C integer division/modulo truncate toward zero; Rust matches for signed.
            // Need to guard against overflow only on i32::MIN / -1 (not the case here).
            x = x.wrapping_div(2).wrapping_add(x.wrapping_rem(7));
        }
        *slot = x;
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <seed>", args.first().map(|s| s.as_str()).unwrap_or("driver"));
        return ExitCode::from(1);
    }

    // Parse seed: must be a non-negative integer fitting in u32 (UINT_MAX), no trailing junk.
    let seed_str = &args[1];
    let temp_seed: u64 = match parse_strtoul(seed_str) {
        Some(v) => v,
        None => {
            eprintln!("Invalid seed: '{}'", seed_str);
            return ExitCode::from(1);
        }
    };
    if temp_seed > u32::MAX as u64 {
        eprintln!("Invalid seed: '{}'", seed_str);
        return ExitCode::from(1);
    }

    let seed: u32 = temp_seed as u32;

    // Use libc srand/rand to match the C output exactly.
    unsafe {
        libc::srand(seed as libc::c_uint);
    }

    let mut array: Vec<i32> = vec![0i32; ARRAY_SIZE];
    for slot in array.iter_mut() {
        *slot = unsafe { libc::rand() } as i32;
    }

    for _ in 0..ITERATIONS {
        perform_expensive_operations(&mut array);
    }

    let mut xor_result: i32 = 0;
    for &v in array.iter() {
        xor_result ^= v;
    }

    println!("{}", xor_result);
    ExitCode::from(0)
}

// Mimic C's strtoul base-10 parsing semantics enough for this program's needs:
// - Skip leading whitespace
// - Optional '+' or '-' sign (a '-' followed by digits in C strtoul produces the negation
//   of the unsigned value, but our caller would still treat it; here we reject negatives for clarity).
// - Require at least one digit
// - Reject any trailing non-digit (matches `*endptr != '\0'` check in C).
fn parse_strtoul(s: &str) -> Option<u64> {
    let bytes = s.as_bytes();
    let mut i = 0;
    // Skip whitespace
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        i += 1;
    }
    // Optional sign
    let mut negative = false;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        if bytes[i] == b'-' {
            negative = true;
        }
        i += 1;
    }
    // Need at least one digit
    let start = i;
    let mut value: u64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        let d = (bytes[i] - b'0') as u64;
        value = value.checked_mul(10)?.checked_add(d)?;
        i += 1;
    }
    if i == start {
        return None;
    }
    // Reject trailing non-null (C used *endptr != '\0')
    if i != bytes.len() {
        return None;
    }
    if negative && value != 0 {
        // C strtoul would wrap, but our subsequent `> UINT_MAX` check is hard to
        // reproduce identically; treat negative as invalid since the C code's
        // post-conversion bounds check would also reject realistic negative inputs.
        return None;
    }
    Some(value)
}
