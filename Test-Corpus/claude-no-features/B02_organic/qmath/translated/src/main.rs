use std::env;
use std::process::ExitCode;

/// Quake III's fast inverse square root.
/// Operates on f32, using identical bit-level hacking as the C source.
fn q_rsqrt(number: f32) -> f32 {
    let threehalfs: f32 = 1.5;
    let x2: f32 = number * 0.5;
    let y: f32 = number;

    // memcpy(&i, &y, sizeof(float))
    let mut i: u32 = y.to_bits();
    i = 0x5f3759df_u32.wrapping_sub(i >> 1);
    // memcpy(&y, &i, sizeof(float))
    let y: f32 = f32::from_bits(i);

    // 1st iteration
    let y = y * (threehalfs - (x2 * y * y));
    y
}

/// Fast vector normalize - matches VectorNormalizeFast in q_shared.h
fn vector_normalize_fast(v: &mut [f32; 3]) {
    let dot = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
    let ilength = q_rsqrt(dot);
    v[0] *= ilength;
    v[1] *= ilength;
    v[2] *= ilength;
}

/// Mimics C's atof for a string slice: skips leading whitespace,
/// parses optional sign, optional digits, optional fractional part,
/// optional exponent. Returns 0.0 if no conversion can be performed.
fn c_atof(s: &str) -> f64 {
    let bytes = s.as_bytes();
    let mut i = 0usize;
    // skip leading whitespace (matches isspace)
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c => i += 1,
            _ => break,
        }
    }
    let start = i;
    // optional sign
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    // integer part
    let int_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    let had_int = i > int_start;
    // fractional part
    let mut had_frac = false;
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        let frac_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        had_frac = i > frac_start;
    }
    if !had_int && !had_frac {
        return 0.0;
    }
    // exponent part
    let before_exp = i;
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
            i = j;
        } else {
            i = before_exp;
        }
    }
    let to_parse = &s[start..i];
    to_parse.parse::<f64>().unwrap_or(0.0)
}

/// Format a float using C's printf("%f", ...) default, which is 6 digits
/// after the decimal point. Rust's default formatter for f64 with `{:.6}`
/// matches glibc's behavior for finite numbers.
fn format_f(v: f32) -> String {
    // printf promotes float to double in variadic args, so format as f64.
    let d = v as f64;
    if d.is_nan() {
        // C printf "%f" prints "nan" for NaN (glibc lowercase).
        return "nan".to_string();
    }
    if d.is_infinite() {
        return if d.is_sign_negative() {
            "-inf".to_string()
        } else {
            "inf".to_string()
        };
    }
    format!("{:.6}", d)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let argc = args.len();

    if argc != 4 {
        let prog = args.get(0).map(String::as_str).unwrap_or("");
        eprintln!("{} requires 4 inputs", prog);
        return ExitCode::from(1);
    }

    let mut inputs: [f32; 3] = [0.0; 3];
    inputs[0] = c_atof(&args[1]) as f32;
    inputs[1] = c_atof(&args[2]) as f32;
    inputs[2] = c_atof(&args[3]) as f32;

    vector_normalize_fast(&mut inputs);

    println!(
        "{} {} {}",
        format_f(inputs[0]),
        format_f(inputs[1]),
        format_f(inputs[2])
    );
    ExitCode::from(0)
}
