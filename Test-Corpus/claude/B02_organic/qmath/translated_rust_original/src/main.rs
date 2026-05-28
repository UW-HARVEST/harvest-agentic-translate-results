use std::env;
use std::process::ExitCode;

/// Mimic C's atof: parse a leading floating-point number from the string.
/// Skips leading whitespace, returns 0.0 if no valid conversion was performed.
fn c_atof(s: &str) -> f64 {
    let bytes = s.as_bytes();
    let mut i = 0usize;

    // Skip leading whitespace
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        i += 1;
    }

    let start = i;

    // Optional sign
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }

    let mut has_digits = false;

    // Integer part digits
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
        has_digits = true;
    }

    // Fractional part
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
            has_digits = true;
        }
    }

    if !has_digits {
        return 0.0;
    }

    // Exponent
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        let exp_start = i;
        i += 1;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        let mut exp_has_digits = false;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
            exp_has_digits = true;
        }
        if !exp_has_digits {
            // Roll back the exponent piece
            i = exp_start;
        }
    }

    let slice = &s[start..i];
    slice.parse::<f64>().unwrap_or(0.0)
}

/// Quake III's fast inverse square root. Operates on f32, matching the C version.
fn q_rsqrt(number: f32) -> f32 {
    let threehalfs: f32 = 1.5;
    let x2 = number * 0.5;
    let mut y = number;

    // Type-pun float bits to integer
    let mut i: u32 = y.to_bits();
    i = 0x5f3759dfu32.wrapping_sub(i >> 1);
    y = f32::from_bits(i);

    // 1st iteration
    y = y * (threehalfs - (x2 * y * y));

    y
}

fn dot_product(v: &[f32; 3]) -> f32 {
    v[0] * v[0] + v[1] * v[1] + v[2] * v[2]
}

fn vector_normalize_fast(v: &mut [f32; 3]) {
    let ilength = q_rsqrt(dot_product(v));
    v[0] *= ilength;
    v[1] *= ilength;
    v[2] *= ilength;
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let argc = args.len();

    if argc != 4 {
        eprintln!("{} requires 4 inputs", args[0]);
        return ExitCode::from(1);
    }

    // C's atof returns a double, then assigning to a float (vec_t) truncates.
    let mut inputs: [f32; 3] = [
        c_atof(&args[1]) as f32,
        c_atof(&args[2]) as f32,
        c_atof(&args[3]) as f32,
    ];

    vector_normalize_fast(&mut inputs);

    // C printf "%f" defaults to 6 digits after the decimal.
    // The values are float (vec_t) but %f expects double, so they're promoted to double.
    println!(
        "{:.6} {:.6} {:.6}",
        inputs[0] as f64, inputs[1] as f64, inputs[2] as f64
    );

    ExitCode::from(0)
}
