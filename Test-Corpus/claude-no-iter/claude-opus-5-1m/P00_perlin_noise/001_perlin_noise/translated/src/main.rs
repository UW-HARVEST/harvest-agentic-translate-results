use std::io::{self, Read, Write};

const RANDTAB_HALF: [u8; 256] = [
    23, 125, 161, 52, 103, 117, 70, 37, 247, 101, 203, 169, 124, 126, 44, 123,
    152, 238, 145, 45, 171, 114, 253, 10, 192, 136, 4, 157, 249, 30, 35, 72,
    175, 63, 77, 90, 181, 16, 96, 111, 133, 104, 75, 162, 93, 56, 66, 240,
    8, 50, 84, 229, 49, 210, 173, 239, 141, 1, 87, 18, 2, 198, 143, 57,
    225, 160, 58, 217, 168, 206, 245, 204, 199, 6, 73, 60, 20, 230, 211, 233,
    94, 200, 88, 9, 74, 155, 33, 15, 219, 130, 226, 202, 83, 236, 42, 172,
    165, 218, 55, 222, 46, 107, 98, 154, 109, 67, 196, 178, 127, 158, 13, 243,
    65, 79, 166, 248, 25, 224, 115, 80, 68, 51, 184, 128, 232, 208, 151, 122,
    26, 212, 105, 43, 179, 213, 235, 148, 146, 89, 14, 195, 28, 78, 112, 76,
    250, 47, 24, 251, 140, 108, 186, 190, 228, 170, 183, 139, 39, 188, 244, 246,
    132, 48, 119, 144, 180, 138, 134, 193, 82, 182, 120, 121, 86, 220, 209, 3,
    91, 241, 149, 85, 205, 150, 113, 216, 31, 100, 41, 164, 177, 214, 153, 231,
    38, 71, 185, 174, 97, 201, 29, 95, 7, 92, 54, 254, 191, 118, 34, 221,
    131, 11, 163, 99, 234, 81, 227, 147, 156, 176, 17, 142, 69, 12, 110, 62,
    27, 255, 0, 194, 59, 116, 242, 252, 19, 21, 187, 53, 207, 129, 64, 135,
    61, 40, 167, 237, 102, 223, 106, 159, 197, 189, 215, 137, 36, 32, 22, 5,
];

const RANDTAB_GRAD_IDX_HALF: [u8; 256] = [
    7, 9, 5, 0, 11, 1, 6, 9, 3, 9, 11, 1, 8, 10, 4, 7,
    8, 6, 1, 5, 3, 10, 9, 10, 0, 8, 4, 1, 5, 2, 7, 8,
    7, 11, 9, 10, 1, 0, 4, 7, 5, 0, 11, 6, 1, 4, 2, 8,
    8, 10, 4, 9, 9, 2, 5, 7, 9, 1, 7, 2, 2, 6, 11, 5,
    5, 4, 6, 9, 0, 1, 1, 0, 7, 6, 9, 8, 4, 10, 3, 1,
    2, 8, 8, 9, 10, 11, 5, 11, 11, 2, 6, 10, 3, 4, 2, 4,
    9, 10, 3, 2, 6, 3, 6, 10, 5, 3, 4, 10, 11, 2, 9, 11,
    1, 11, 10, 4, 9, 4, 11, 0, 4, 11, 4, 0, 0, 0, 7, 6,
    10, 4, 1, 3, 11, 5, 3, 4, 2, 9, 1, 3, 0, 1, 8, 0,
    6, 7, 8, 7, 0, 4, 6, 10, 8, 2, 3, 11, 11, 8, 0, 2,
    4, 8, 3, 0, 0, 10, 6, 1, 2, 2, 4, 5, 6, 0, 1, 3,
    11, 9, 5, 5, 9, 6, 9, 8, 3, 8, 1, 8, 9, 6, 9, 11,
    10, 7, 5, 6, 5, 9, 1, 3, 7, 0, 2, 10, 11, 2, 6, 1,
    3, 11, 7, 7, 2, 1, 7, 3, 0, 8, 1, 1, 5, 0, 6, 10,
    11, 11, 0, 2, 7, 0, 10, 8, 3, 5, 7, 1, 11, 1, 0, 7,
    9, 0, 11, 5, 10, 3, 2, 3, 5, 9, 7, 9, 8, 4, 6, 5,
];

const RANDTAB: [u8; 512] = {
    let mut arr = [0u8; 512];
    let mut i = 0;
    while i < 256 {
        arr[i] = RANDTAB_HALF[i];
        arr[i + 256] = RANDTAB_HALF[i];
        i += 1;
    }
    arr
};

const RANDTAB_GRAD_IDX: [u8; 512] = {
    let mut arr = [0u8; 512];
    let mut i = 0;
    while i < 256 {
        arr[i] = RANDTAB_GRAD_IDX_HALF[i];
        arr[i + 256] = RANDTAB_GRAD_IDX_HALF[i];
        i += 1;
    }
    arr
};

const BASIS: [[f32; 3]; 12] = [
    [ 1.0,  1.0,  0.0],
    [-1.0,  1.0,  0.0],
    [ 1.0, -1.0,  0.0],
    [-1.0, -1.0,  0.0],
    [ 1.0,  0.0,  1.0],
    [-1.0,  0.0,  1.0],
    [ 1.0,  0.0, -1.0],
    [-1.0,  0.0, -1.0],
    [ 0.0,  1.0,  1.0],
    [ 0.0, -1.0,  1.0],
    [ 0.0,  1.0, -1.0],
    [ 0.0, -1.0, -1.0],
];

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[inline]
fn fastfloor(a: f32) -> i32 {
    let ai = a as i32;
    if a < (ai as f32) { ai - 1 } else { ai }
}

#[inline]
fn ease(a: f32) -> f32 {
    ((a * 6.0 - 15.0) * a + 10.0) * a * a * a
}

#[inline]
fn grad(grad_idx: u8, x: f32, y: f32, z: f32) -> f32 {
    let g = &BASIS[grad_idx as usize];
    g[0] * x + g[1] * y + g[2] * z
}

fn noise3_internal(
    mut x: f32,
    mut y: f32,
    mut z: f32,
    x_wrap: i32,
    y_wrap: i32,
    z_wrap: i32,
    seed: u8,
) -> f32 {
    let x_mask: i32 = (x_wrap.wrapping_sub(1)) & 255;
    let y_mask: i32 = (y_wrap.wrapping_sub(1)) & 255;
    let z_mask: i32 = (z_wrap.wrapping_sub(1)) & 255;

    let px = fastfloor(x);
    let py = fastfloor(y);
    let pz = fastfloor(z);
    let x0 = (px & x_mask) as usize;
    let x1 = (px.wrapping_add(1) & x_mask) as usize;
    let y0 = (py & y_mask) as usize;
    let y1 = (py.wrapping_add(1) & y_mask) as usize;
    let z0 = (pz & z_mask) as usize;
    let z1 = (pz.wrapping_add(1) & z_mask) as usize;

    x -= px as f32;
    let u = ease(x);
    y -= py as f32;
    let v = ease(y);
    z -= pz as f32;
    let w = ease(z);

    let seed_us = seed as usize;
    let r0 = RANDTAB[x0 + seed_us] as usize;
    let r1 = RANDTAB[x1 + seed_us] as usize;

    let r00 = RANDTAB[r0 + y0] as usize;
    let r01 = RANDTAB[r0 + y1] as usize;
    let r10 = RANDTAB[r1 + y0] as usize;
    let r11 = RANDTAB[r1 + y1] as usize;

    let n000 = grad(RANDTAB_GRAD_IDX[r00 + z0], x      , y      , z      );
    let n001 = grad(RANDTAB_GRAD_IDX[r00 + z1], x      , y      , z - 1.0);
    let n010 = grad(RANDTAB_GRAD_IDX[r01 + z0], x      , y - 1.0, z      );
    let n011 = grad(RANDTAB_GRAD_IDX[r01 + z1], x      , y - 1.0, z - 1.0);
    let n100 = grad(RANDTAB_GRAD_IDX[r10 + z0], x - 1.0, y      , z      );
    let n101 = grad(RANDTAB_GRAD_IDX[r10 + z1], x - 1.0, y      , z - 1.0);
    let n110 = grad(RANDTAB_GRAD_IDX[r11 + z0], x - 1.0, y - 1.0, z      );
    let n111 = grad(RANDTAB_GRAD_IDX[r11 + z1], x - 1.0, y - 1.0, z - 1.0);

    let n00 = lerp(n000, n001, w);
    let n01 = lerp(n010, n011, w);
    let n10 = lerp(n100, n101, w);
    let n11 = lerp(n110, n111, w);

    let n0 = lerp(n00, n01, v);
    let n1 = lerp(n10, n11, v);

    lerp(n0, n1, u)
}

fn noise3(x: f32, y: f32, z: f32, x_wrap: i32, y_wrap: i32, z_wrap: i32) -> f32 {
    noise3_internal(x, y, z, x_wrap, y_wrap, z_wrap, 0)
}

fn noise3_seed(x: f32, y: f32, z: f32, x_wrap: i32, y_wrap: i32, z_wrap: i32, seed: i32) -> f32 {
    noise3_internal(x, y, z, x_wrap, y_wrap, z_wrap, seed as u8)
}

fn ridge_noise3(
    x: f32, y: f32, z: f32,
    lacunarity: f32, gain: f32, offset: f32, octaves: i32,
) -> f32 {
    let mut frequency: f32 = 1.0;
    let mut prev: f32 = 1.0;
    let mut amplitude: f32 = 0.5;
    let mut sum: f32 = 0.0;

    for i in 0..octaves {
        let mut r = noise3_internal(
            x * frequency, y * frequency, z * frequency,
            0, 0, 0, i as u8,
        );
        r = offset - r.abs();
        r = r * r;
        sum += r * amplitude * prev;
        prev = r;
        frequency *= lacunarity;
        amplitude *= gain;
    }
    sum
}

fn fbm_noise3(x: f32, y: f32, z: f32, lacunarity: f32, gain: f32, octaves: i32) -> f32 {
    let mut frequency: f32 = 1.0;
    let mut amplitude: f32 = 1.0;
    let mut sum: f32 = 0.0;

    for i in 0..octaves {
        sum += noise3_internal(
            x * frequency, y * frequency, z * frequency,
            0, 0, 0, i as u8,
        ) * amplitude;
        frequency *= lacunarity;
        amplitude *= gain;
    }
    sum
}

fn turbulence_noise3(
    x: f32, y: f32, z: f32,
    lacunarity: f32, gain: f32, octaves: i32,
) -> f32 {
    let mut frequency: f32 = 1.0;
    let mut amplitude: f32 = 1.0;
    let mut sum: f32 = 0.0;

    for i in 0..octaves {
        let r = noise3_internal(
            x * frequency, y * frequency, z * frequency,
            0, 0, 0, i as u8,
        ) * amplitude;
        sum += r.abs();
        frequency *= lacunarity;
        amplitude *= gain;
    }
    sum
}

fn noise3_wrap_nonpow2(
    mut x: f32, mut y: f32, mut z: f32,
    x_wrap: i32, y_wrap: i32, z_wrap: i32, seed: u8,
) -> f32 {
    let px = fastfloor(x);
    let py = fastfloor(y);
    let pz = fastfloor(z);
    let x_wrap2 = if x_wrap != 0 { x_wrap } else { 256 };
    let y_wrap2 = if y_wrap != 0 { y_wrap } else { 256 };
    let z_wrap2 = if z_wrap != 0 { z_wrap } else { 256 };

    let mut x0 = px % x_wrap2;
    let mut y0 = py % y_wrap2;
    let mut z0 = pz % z_wrap2;

    if x0 < 0 { x0 += x_wrap2; }
    if y0 < 0 { y0 += y_wrap2; }
    if z0 < 0 { z0 += z_wrap2; }
    let x1 = (x0 + 1) % x_wrap2;
    let y1 = (y0 + 1) % y_wrap2;
    let z1 = (z0 + 1) % z_wrap2;

    x -= px as f32;
    let u = ease(x);
    y -= py as f32;
    let v = ease(y);
    z -= pz as f32;
    let w = ease(z);

    let seed_us = seed as usize;
    let r0 = RANDTAB[x0 as usize] as usize;
    let r0 = RANDTAB[r0 + seed_us] as usize;
    let r1 = RANDTAB[x1 as usize] as usize;
    let r1 = RANDTAB[r1 + seed_us] as usize;

    let r00 = RANDTAB[r0 + y0 as usize] as usize;
    let r01 = RANDTAB[r0 + y1 as usize] as usize;
    let r10 = RANDTAB[r1 + y0 as usize] as usize;
    let r11 = RANDTAB[r1 + y1 as usize] as usize;

    let n000 = grad(RANDTAB_GRAD_IDX[r00 + z0 as usize], x      , y      , z      );
    let n001 = grad(RANDTAB_GRAD_IDX[r00 + z1 as usize], x      , y      , z - 1.0);
    let n010 = grad(RANDTAB_GRAD_IDX[r01 + z0 as usize], x      , y - 1.0, z      );
    let n011 = grad(RANDTAB_GRAD_IDX[r01 + z1 as usize], x      , y - 1.0, z - 1.0);
    let n100 = grad(RANDTAB_GRAD_IDX[r10 + z0 as usize], x - 1.0, y      , z      );
    let n101 = grad(RANDTAB_GRAD_IDX[r10 + z1 as usize], x - 1.0, y      , z - 1.0);
    let n110 = grad(RANDTAB_GRAD_IDX[r11 + z0 as usize], x - 1.0, y - 1.0, z      );
    let n111 = grad(RANDTAB_GRAD_IDX[r11 + z1 as usize], x - 1.0, y - 1.0, z - 1.0);

    let n00 = lerp(n000, n001, w);
    let n01 = lerp(n010, n011, w);
    let n10 = lerp(n100, n101, w);
    let n11 = lerp(n110, n111, w);

    let n0 = lerp(n00, n01, v);
    let n1 = lerp(n10, n11, v);

    lerp(n0, n1, u)
}

fn inner(
    which: i32,
    x: f32, y: f32, z: f32,
    x_wrap: i32, y_wrap: i32, z_wrap: i32,
    seed: i32,
    lacunarity: f32, gain: f32, offset: f32,
    octaves: i32,
) -> f32 {
    match which {
        0 => noise3(x, y, z, x_wrap, y_wrap, z_wrap),
        1 => noise3_seed(x, y, z, x_wrap, y_wrap, z_wrap, seed),
        2 => ridge_noise3(x, y, z, lacunarity, gain, offset, octaves),
        3 => fbm_noise3(x, y, z, lacunarity, gain, octaves),
        4 => turbulence_noise3(x, y, z, lacunarity, gain, octaves),
        5 => noise3_wrap_nonpow2(x, y, z, x_wrap, y_wrap, z_wrap, seed as u8),
        _ => f32::NAN,
    }
}

/// Format a f32 as C's printf would for "%.9g".
fn format_g9(value: f32) -> String {
    if value.is_nan() {
        return if value.is_sign_negative() {
            "-nan".to_string()
        } else {
            "nan".to_string()
        };
    }
    if value.is_infinite() {
        return if value < 0.0 { "-inf".to_string() } else { "inf".to_string() };
    }
    if value == 0.0 {
        return if value.is_sign_negative() { "-0".to_string() } else { "0".to_string() };
    }

    const PREC: i32 = 9;

    // Format as scientific with PREC-1 digits after decimal point.
    // Rust's "{:.Ne}" produces e.g. "1.23456789e0" or "-1.23456789e-1".
    let formatted = format!("{:.*e}", (PREC - 1) as usize, value);

    let (mantissa_str, exp_str) = formatted.split_once('e').expect("scientific has 'e'");
    let exponent: i32 = exp_str.parse().expect("valid exponent");

    let (sign, num) = if let Some(rest) = mantissa_str.strip_prefix('-') {
        ("-", rest)
    } else {
        ("", mantissa_str)
    };

    let (int_part, frac_part) = match num.split_once('.') {
        Some((i, f)) => (i, f),
        None => (num, ""),
    };

    let mut all_digits = String::with_capacity(int_part.len() + frac_part.len());
    all_digits.push_str(int_part);
    all_digits.push_str(frac_part);

    if exponent < -4 || exponent >= PREC {
        // %e style: mantissa with trailing zeros stripped, then "e±dd".
        let frac_stripped = frac_part.trim_end_matches('0');
        let mantissa = if frac_stripped.is_empty() {
            int_part.to_string()
        } else {
            format!("{}.{}", int_part, frac_stripped)
        };
        let exp_sign = if exponent < 0 { '-' } else { '+' };
        let exp_abs = exponent.unsigned_abs();
        format!("{}{}e{}{:02}", sign, mantissa, exp_sign, exp_abs)
    } else {
        // %f style: shift the decimal to position (exponent + 1).
        let digits_before = exponent + 1;

        let result_str: String = if digits_before <= 0 {
            let mut s = String::from("0.");
            for _ in 0..(-digits_before) {
                s.push('0');
            }
            s.push_str(&all_digits);
            s
        } else if (digits_before as usize) >= all_digits.len() {
            let mut s = String::from(all_digits.as_str());
            for _ in all_digits.len()..(digits_before as usize) {
                s.push('0');
            }
            s
        } else {
            let pos = digits_before as usize;
            format!("{}.{}", &all_digits[..pos], &all_digits[pos..])
        };

        let stripped = if result_str.contains('.') {
            let s = result_str.trim_end_matches('0');
            let s = s.trim_end_matches('.');
            s.to_string()
        } else {
            result_str
        };

        format!("{}{}", sign, stripped)
    }
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).expect("read stdin");
    let mut tokens = input.split_ascii_whitespace();

    let mut next_token = || tokens.next().expect("not enough input tokens");

    let which: i32 = next_token().parse().expect("which: int");
    let x: f32 = next_token().parse().expect("x: float");
    let y: f32 = next_token().parse().expect("y: float");
    let z: f32 = next_token().parse().expect("z: float");
    let x_wrap: i32 = next_token().parse().expect("x_wrap: int");
    let y_wrap: i32 = next_token().parse().expect("y_wrap: int");
    let z_wrap: i32 = next_token().parse().expect("z_wrap: int");
    let seed: i32 = next_token().parse().expect("seed: int");
    let lacunarity: f32 = next_token().parse().expect("lacunarity: float");
    let gain: f32 = next_token().parse().expect("gain: float");
    let offset: f32 = next_token().parse().expect("offset: float");
    let octaves: i32 = next_token().parse().expect("octaves: int");

    let res = inner(
        which, x, y, z,
        x_wrap, y_wrap, z_wrap,
        seed,
        lacunarity, gain, offset,
        octaves,
    );

    let stdout = io::stdout();
    let mut out = stdout.lock();
    writeln!(out, "{}", format_g9(res)).expect("write stdout");
}
