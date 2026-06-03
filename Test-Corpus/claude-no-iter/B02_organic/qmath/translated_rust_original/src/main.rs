use std::env;
use std::process::exit;

/// Fast inverse square root, mirroring Quake III's Q_rsqrt exactly.
fn q_rsqrt(number: f32) -> f32 {
    let threehalfs: f32 = 1.5;
    let x2 = number * 0.5_f32;
    let y_in = number;

    // memcpy(&i, &y, sizeof(float))
    let mut i: u32 = y_in.to_bits();
    // i = 0x5f3759dfu - (i >> 1)
    i = 0x5f3759df_u32.wrapping_sub(i >> 1);
    // memcpy(&y, &i, sizeof(float))
    let mut y = f32::from_bits(i);

    // 1st iteration
    y = y * (threehalfs - (x2 * y * y));
    y
}

fn vector_normalize_fast(v: &mut [f32; 3]) {
    let dot = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
    let ilength = q_rsqrt(dot);
    v[0] *= ilength;
    v[1] *= ilength;
    v[2] *= ilength;
}

/// Mimic C's atof(): skip leading whitespace, parse the longest valid numeric
/// prefix (sign, digits, optional decimal, optional exponent), and return 0.0
/// if no conversion is possible.
fn c_atof(s: &str) -> f64 {
    let bytes = s.as_bytes();
    let mut i = 0usize;

    // Skip leading whitespace (matches isspace in C locale "C").
    while i < bytes.len()
        && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
    {
        i += 1;
    }
    let start = i;

    // Optional sign.
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }

    let mut has_digit = false;

    // Integer digits.
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
        has_digit = true;
    }

    // Optional fractional part.
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
            has_digit = true;
        }
    }

    if !has_digit {
        return 0.0;
    }

    // Optional exponent.
    let mut end = i;
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        let mut j = i + 1;
        if j < bytes.len() && (bytes[j] == b'+' || bytes[j] == b'-') {
            j += 1;
        }
        let exp_digits_start = j;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        if j > exp_digits_start {
            end = j;
        }
    }

    let slice = &s[start..end];
    slice.parse::<f64>().unwrap_or(0.0)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let argc = args.len();

    if argc != 4 {
        eprintln!("{} requires 4 inputs", args[0]);
        exit(1);
    }

    let mut inputs: [f32; 3] = [
        c_atof(&args[1]) as f32,
        c_atof(&args[2]) as f32,
        c_atof(&args[3]) as f32,
    ];

    vector_normalize_fast(&mut inputs);

    // C's printf("%f %f %f\n", ...) promotes float -> double and prints with
    // default precision of 6 decimal digits.
    println!(
        "{:.6} {:.6} {:.6}",
        inputs[0] as f64, inputs[1] as f64, inputs[2] as f64
    );
}
