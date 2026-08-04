use std::env;
use std::process::ExitCode;

// Mimic C atof: parse the leading numeric portion of a string as f64.
// C's atof returns 0.0 if no valid conversion could be performed.
fn c_atof(s: &str) -> f64 {
    // Skip leading whitespace per C semantics.
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
        i += 1;
    }
    let rest = &s[i..];

    // Try progressively shorter prefixes until one parses, mimicking
    // strtod's "longest valid prefix" behavior.
    let mut end = rest.len();
    while end > 0 {
        if let Ok(v) = rest[..end].parse::<f64>() {
            return v;
        }
        end -= 1;
    }
    0.0
}

// Q_rsqrt: famous fast inverse square root using the 0x5f3759df constant.
fn q_rsqrt(number: f32) -> f32 {
    let x2: f32 = number * 0.5_f32;
    let mut y: f32 = number;

    // memcpy(&i, &y, sizeof(float))
    let mut i: u32 = y.to_bits();
    i = 0x5f3759df_u32.wrapping_sub(i >> 1);
    // memcpy(&y, &i, sizeof(float))
    y = f32::from_bits(i);

    let threehalfs: f32 = 1.5_f32;
    y = y * (threehalfs - (x2 * y * y));
    y
}

fn dot_product(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn vector_normalize_fast(v: &mut [f32; 3]) {
    let ilength = q_rsqrt(dot_product(v, v));
    v[0] *= ilength;
    v[1] *= ilength;
    v[2] *= ilength;
}

// Format a double the way glibc's printf("%f", ...) does (precision 6).
// Rust's {:.6} on f64 produces the same output for finite numbers.
// For NaN/Inf, glibc produces "nan", "-nan", "inf", "-inf"; Rust produces
// "NaN", "inf", "-inf". Normalize to match glibc's lowercase form.
fn format_f(value: f64) -> String {
    if value.is_nan() {
        // glibc prints "nan" or "-nan" depending on the sign bit.
        if value.is_sign_negative() {
            "-nan".to_string()
        } else {
            "nan".to_string()
        }
    } else if value.is_infinite() {
        if value.is_sign_negative() {
            "-inf".to_string()
        } else {
            "inf".to_string()
        }
    } else {
        format!("{:.6}", value)
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let argc = args.len();

    if argc != 4 {
        // fprintf(stderr, "%s requires 4 inputs\n", argv[0]);
        eprintln!("{} requires 4 inputs", args[0]);
        return ExitCode::from(1);
    }

    // Inputs[i] = atof(argv[i+1]); stored into a float (vec3_t element).
    let mut inputs: [f32; 3] = [
        c_atof(&args[1]) as f32,
        c_atof(&args[2]) as f32,
        c_atof(&args[3]) as f32,
    ];

    vector_normalize_fast(&mut inputs);

    // printf("%f %f %f\n", Inputs[0], Inputs[1], Inputs[2]);
    // floats are promoted to double when passed to printf.
    println!(
        "{} {} {}",
        format_f(inputs[0] as f64),
        format_f(inputs[1] as f64),
        format_f(inputs[2] as f64)
    );

    ExitCode::from(0)
}
