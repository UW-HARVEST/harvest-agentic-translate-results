use std::io::Read;

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

/// Mimics C's printf("%.9g", value) for a f32 value (which is promoted to double
/// in C variadic calls — we promote to f64 here to match).
fn format_g_9(value: f32) -> String {
    let v = value as f64;

    if v.is_nan() {
        return "nan".to_string();
    }
    if v.is_infinite() {
        return if v < 0.0 { "-inf".to_string() } else { "inf".to_string() };
    }

    let precision: i32 = 9;

    if v == 0.0 {
        return if v.is_sign_negative() { "-0".to_string() } else { "0".to_string() };
    }

    // Determine exponent X where 1 <= |v| / 10^X < 10
    let abs = v.abs();
    let exp_approx = abs.log10().floor() as i32;
    // Correct for floating-point inaccuracies in log10 at boundaries.
    let exp = {
        let pow_low = 10.0_f64.powi(exp_approx);
        let pow_high = 10.0_f64.powi(exp_approx + 1);
        if abs >= pow_high {
            exp_approx + 1
        } else if abs < pow_low {
            exp_approx - 1
        } else {
            exp_approx
        }
    };

    if exp < -4 || exp >= precision {
        // Use %e style: mantissa with (precision - 1) fractional digits.
        let frac_digits = (precision - 1) as usize;
        let mantissa = v / 10.0_f64.powi(exp);
        let mantissa_str = format!("{:.*}", frac_digits, mantissa);
        let mantissa_stripped = strip_trailing_fractional_zeros(&mantissa_str);
        let sign_char = if exp >= 0 { '+' } else { '-' };
        format!("{}e{}{:02}", mantissa_stripped, sign_char, exp.abs())
    } else {
        // Use %f style: (precision - 1 - exp) fractional digits.
        let frac_digits = (precision - 1 - exp).max(0) as usize;
        let s = format!("{:.*}", frac_digits, v);
        strip_trailing_fractional_zeros(&s)
    }
}

fn strip_trailing_fractional_zeros(s: &str) -> String {
    if !s.contains('.') {
        return s.to_string();
    }
    let trimmed = s.trim_end_matches('0');
    let trimmed = trimmed.trim_end_matches('.');
    trimmed.to_string()
}

fn main() {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).expect("failed to read stdin");

    let mut tokens = input.split_ascii_whitespace();

    fn next_i32(it: &mut std::str::SplitAsciiWhitespace<'_>) -> i32 {
        it.next().expect("missing integer token").parse::<i32>().expect("failed to parse i32")
    }
    fn next_f32(it: &mut std::str::SplitAsciiWhitespace<'_>) -> f32 {
        it.next().expect("missing float token").parse::<f32>().expect("failed to parse f32")
    }

    let which = next_i32(&mut tokens);
    let x = next_f32(&mut tokens);
    let y = next_f32(&mut tokens);
    let z = next_f32(&mut tokens);
    let x_wrap = next_i32(&mut tokens);
    let y_wrap = next_i32(&mut tokens);
    let z_wrap = next_i32(&mut tokens);
    let seed = next_i32(&mut tokens);
    let lacunarity = next_f32(&mut tokens);
    let gain = next_f32(&mut tokens);
    let offset = next_f32(&mut tokens);
    let octaves = next_i32(&mut tokens);

    let res = inner(
        which,
        x,
        y,
        z,
        x_wrap,
        y_wrap,
        z_wrap,
        seed,
        lacunarity,
        gain,
        offset,
        octaves,
    );

    println!("{}", format_g_9(res));
}
