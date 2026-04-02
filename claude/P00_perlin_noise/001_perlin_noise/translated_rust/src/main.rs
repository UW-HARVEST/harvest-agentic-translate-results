use std::io::{self, Read};

mod stb_perlin;

use stb_perlin::*;

fn inner(
    which: i32,
    x: f32,
    y: f32,
    z: f32,
    x_wrap: i32,
    y_wrap: i32,
    z_wrap: i32,
    seed: i32,
    lacunarity: f32,
    gain: f32,
    offset: f32,
    octaves: i32,
) -> f32 {
    match which {
        0 => stb_perlin_noise3(x, y, z, x_wrap, y_wrap, z_wrap),
        1 => stb_perlin_noise3_seed(x, y, z, x_wrap, y_wrap, z_wrap, seed),
        2 => stb_perlin_ridge_noise3(x, y, z, lacunarity, gain, offset, octaves),
        3 => stb_perlin_fbm_noise3(x, y, z, lacunarity, gain, octaves),
        4 => stb_perlin_turbulence_noise3(x, y, z, lacunarity, gain, octaves),
        5 => stb_perlin_noise3_wrap_nonpow2(x, y, z, x_wrap, y_wrap, z_wrap, seed as u8),
        _ => f32::NAN,
    }
}

/// Parse whitespace-separated tokens from stdin matching C's scanf behavior
fn scanf_tokens(input: &str) -> Vec<&str> {
    input.split_whitespace().collect()
}

fn parse_f32_scanf(s: &str) -> f32 {
    // C's scanf %f parses the string to float
    s.parse::<f32>().unwrap()
}

fn parse_i32_scanf(s: &str) -> i32 {
    s.parse::<i32>().unwrap()
}

/// Format a f32 value using %.9g formatting (matching C's printf)
fn format_9g(val: f32) -> String {
    // %.9g uses 9 significant digits, choosing between fixed and scientific notation
    let val = val as f64; // C's printf promotes float to double
    if val == 0.0 {
        // Check for negative zero
        if val.is_sign_negative() {
            return "-0".to_string();
        }
        return "0".to_string();
    }

    if val.is_nan() {
        return "nan".to_string();
    }

    if val.is_infinite() {
        if val > 0.0 {
            return "inf".to_string();
        } else {
            return "-inf".to_string();
        }
    }

    // Use %g logic: if exponent < -4 or >= precision, use scientific notation
    let precision = 9usize;
    let abs_val = val.abs();
    let exp = abs_val.log10().floor() as i32;

    if exp < -4 || exp >= precision as i32 {
        // Scientific notation with (precision - 1) digits after decimal point
        let mut s = format!("{:.prec$e}", val, prec = precision - 1);
        // Remove trailing zeros after decimal point (but keep at least the 'e')
        if let Some(e_pos) = s.find('e') {
            let mantissa = &s[..e_pos];
            let exponent = &s[e_pos..];
            let mantissa = mantissa.trim_end_matches('0').trim_end_matches('.');
            // C uses e+XX or e-XX format with at least 2 digits
            // Rust uses e notation differently, need to convert
            let exp_part = &exponent[1..]; // skip 'e'
            let (sign, digits) = if exp_part.starts_with('-') {
                ("-", &exp_part[1..])
            } else if exp_part.starts_with('+') {
                ("+", &exp_part[1..])
            } else {
                ("+", exp_part)
            };
            let exp_num: i32 = digits.parse().unwrap();
            s = format!("{}e{}{:02}", mantissa, sign, exp_num.abs());
        }
        s
    } else {
        // Fixed notation
        // Number of decimal places = precision - (exp + 1), but at least 0
        let decimal_places = if precision as i32 > exp + 1 {
            (precision as i32 - exp - 1) as usize
        } else {
            0
        };
        let s = format!("{:.prec$}", val, prec = decimal_places);
        // Remove trailing zeros after decimal point
        if s.contains('.') {
            let s = s.trim_end_matches('0').trim_end_matches('.');
            s.to_string()
        } else {
            s
        }
    }
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let tokens: Vec<&str> = scanf_tokens(&input);

    let which = parse_i32_scanf(tokens[0]);
    let x = parse_f32_scanf(tokens[1]);
    let y = parse_f32_scanf(tokens[2]);
    let z = parse_f32_scanf(tokens[3]);
    let x_wrap = parse_i32_scanf(tokens[4]);
    let y_wrap = parse_i32_scanf(tokens[5]);
    let z_wrap = parse_i32_scanf(tokens[6]);
    let seed = parse_i32_scanf(tokens[7]);
    let lacunarity = parse_f32_scanf(tokens[8]);
    let gain = parse_f32_scanf(tokens[9]);
    let offset = parse_f32_scanf(tokens[10]);
    let octaves = parse_i32_scanf(tokens[11]);

    let res = inner(
        which, x, y, z, x_wrap, y_wrap, z_wrap, seed, lacunarity, gain, offset, octaves,
    );

    println!("{}", format_9g(res));
}
