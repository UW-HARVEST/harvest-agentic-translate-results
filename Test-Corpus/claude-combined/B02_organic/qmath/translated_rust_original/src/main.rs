// Rust port of c_src — must produce byte-identical output to the C driver.

use std::env;
use std::io::Write;
use std::process::ExitCode;

/// Mimic C's `atof` (which is implemented in terms of `strtod`):
/// skip leading whitespace, optional sign, then either an "inf[inity]" or
/// "nan" token, or a decimal number with optional exponent. Returns 0.0 on
/// parse failure.
fn atof(s: &str) -> f64 {
    let bytes = s.as_bytes();
    let mut i = 0usize;

    // skip leading whitespace (C's isspace: ' ', '\t', '\n', '\v', '\f', '\r')
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r' => i += 1,
            _ => break,
        }
    }
    let parse_start = i;

    // optional sign
    let sign_negative = i < bytes.len() && bytes[i] == b'-';
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }

    // case-insensitive prefix match against the remaining bytes
    let starts_with_ci = |needle: &[u8]| {
        bytes.len() - i >= needle.len()
            && bytes[i..i + needle.len()]
                .iter()
                .zip(needle.iter())
                .all(|(a, b)| a.eq_ignore_ascii_case(b))
    };

    // "infinity" / "inf"
    if starts_with_ci(b"infinity") {
        return if sign_negative { f64::NEG_INFINITY } else { f64::INFINITY };
    }
    if starts_with_ci(b"inf") {
        return if sign_negative { f64::NEG_INFINITY } else { f64::INFINITY };
    }
    // "nan" — glibc's strtod yields a positive NaN regardless of the leading
    // sign, so we mirror that here.
    if starts_with_ci(b"nan") {
        let _ = sign_negative;
        return f64::NAN;
    }

    let mut saw_digit = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
        saw_digit = true;
    }
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
            saw_digit = true;
        }
    }
    if !saw_digit {
        return 0.0;
    }

    // optional exponent
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        let exp_token_start = i;
        i += 1;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        let exp_digit_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == exp_digit_start {
            // No exponent digits — roll back, exponent token is invalid.
            i = exp_token_start;
        }
    }

    let slice = &s[parse_start..i];
    slice.parse::<f64>().unwrap_or(0.0)
}

/// Quake III fast inverse square root.
fn q_rsqrt(number: f32) -> f32 {
    let x2 = number * 0.5f32;
    let mut y = number;
    let threehalfs = 1.5f32;

    // evil floating point bit level hacking
    let mut i: u32 = y.to_bits();
    i = 0x5f3759dfu32.wrapping_sub(i >> 1); // what the?
    y = f32::from_bits(i);

    y = y * (threehalfs - (x2 * y * y)); // 1st iteration
    y
}

fn vector_normalize_fast(v: &mut [f32; 3]) {
    let dot = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
    let ilength = q_rsqrt(dot);
    v[0] *= ilength;
    v[1] *= ilength;
    v[2] *= ilength;
}

/// Format a finite-or-non-finite f64 the same way glibc's `printf("%f", ...)`
/// would: 6 fractional digits, "nan"/"inf" for non-finite, sign preserved.
fn format_f(value: f64) -> String {
    if value.is_nan() {
        // glibc preserves the sign bit of NaNs: "%f" yields "nan" or "-nan".
        return if value.is_sign_negative() {
            "-nan".to_string()
        } else {
            "nan".to_string()
        };
    }
    if value.is_infinite() {
        return if value.is_sign_negative() {
            "-inf".to_string()
        } else {
            "inf".to_string()
        };
    }
    format!("{:.6}", value)
}

fn main() -> ExitCode {
    let argv: Vec<String> = env::args().collect();
    let argc = argv.len();
    let prog = argv.first().map(String::as_str).unwrap_or("");

    if argc != 4 {
        let _ = writeln!(std::io::stderr(), "{} requires 4 inputs", prog);
        return ExitCode::from(1);
    }

    // atof returns double in C, then truncates to float when stored in vec3_t.
    let mut inputs: [f32; 3] = [
        atof(&argv[1]) as f32,
        atof(&argv[2]) as f32,
        atof(&argv[3]) as f32,
    ];

    vector_normalize_fast(&mut inputs);

    // C's printf("%f %f %f\n", float, float, float) promotes the floats to
    // double for the variadic call, so we mirror that here.
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(
        out,
        "{} {} {}",
        format_f(inputs[0] as f64),
        format_f(inputs[1] as f64),
        format_f(inputs[2] as f64),
    );

    ExitCode::from(0)
}
