// Translation of stb_perlin.h (v0.5) and main.c's `inner` function from C to Rust.
// Aims to produce byte-identical output to the original C implementation.

#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]

use std::ffi::c_int;

// ---------------------------------------------------------------------------
// Tables
// ---------------------------------------------------------------------------

static stb__perlin_randtab: [u8; 512] = [
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

    // and a second copy so we don't need an extra mask or static initializer
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

static stb__perlin_randtab_grad_idx: [u8; 512] = [
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

    // and a second copy so we don't need an extra mask or static initializer
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[inline]
fn stb__perlin_lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[inline]
fn stb__perlin_fastfloor(a: f32) -> i32 {
    // C: int ai = (int) a; return (a < ai) ? ai-1 : ai;
    // Match C's float -> int truncation toward zero.
    let ai: i32 = a as i32;
    if a < (ai as f32) { ai - 1 } else { ai }
}

#[inline]
fn stb__perlin_ease(a: f32) -> f32 {
    // (((a*6-15)*a + 10) * a * a * a)
    ((a * 6.0 - 15.0) * a + 10.0) * a * a * a
}

const BASIS: [[f32; 4]; 12] = [
    [  1.0,  1.0,  0.0, 0.0 ],
    [ -1.0,  1.0,  0.0, 0.0 ],
    [  1.0, -1.0,  0.0, 0.0 ],
    [ -1.0, -1.0,  0.0, 0.0 ],
    [  1.0,  0.0,  1.0, 0.0 ],
    [ -1.0,  0.0,  1.0, 0.0 ],
    [  1.0,  0.0, -1.0, 0.0 ],
    [ -1.0,  0.0, -1.0, 0.0 ],
    [  0.0,  1.0,  1.0, 0.0 ],
    [  0.0, -1.0,  1.0, 0.0 ],
    [  0.0,  1.0, -1.0, 0.0 ],
    [  0.0, -1.0, -1.0, 0.0 ],
];

#[inline]
fn stb__perlin_grad(grad_idx: i32, x: f32, y: f32, z: f32) -> f32 {
    let grad = &BASIS[grad_idx as usize];
    grad[0] * x + grad[1] * y + grad[2] * z
}

// ---------------------------------------------------------------------------
// Core noise
// ---------------------------------------------------------------------------

fn stb_perlin_noise3_internal_impl(
    mut x: f32,
    mut y: f32,
    mut z: f32,
    x_wrap: c_int,
    y_wrap: c_int,
    z_wrap: c_int,
    seed: u8,
) -> f32 {
    // Mirror C: unsigned int x_mask = (x_wrap-1) & 255;
    // (x_wrap - 1) is computed as signed int, then bitwise & with 255.
    let x_mask: u32 = ((x_wrap.wrapping_sub(1)) as u32) & 255;
    let y_mask: u32 = ((y_wrap.wrapping_sub(1)) as u32) & 255;
    let z_mask: u32 = ((z_wrap.wrapping_sub(1)) as u32) & 255;

    let px: i32 = stb__perlin_fastfloor(x);
    let py: i32 = stb__perlin_fastfloor(y);
    let pz: i32 = stb__perlin_fastfloor(z);

    // C: int x0 = px & x_mask; (px is int, x_mask is unsigned int).
    // The result of mixed signed/unsigned & is unsigned int, then implicitly
    // converted to int when stored to x0 (signed). For our purposes, we mask
    // to 8 bits so values are always 0..255 here.
    let x0: i32 = ((px as u32) & x_mask) as i32;
    let x1: i32 = (((px.wrapping_add(1)) as u32) & x_mask) as i32;
    let y0: i32 = ((py as u32) & y_mask) as i32;
    let y1: i32 = (((py.wrapping_add(1)) as u32) & y_mask) as i32;
    let z0: i32 = ((pz as u32) & z_mask) as i32;
    let z1: i32 = (((pz.wrapping_add(1)) as u32) & z_mask) as i32;

    x -= px as f32; let u = stb__perlin_ease(x);
    y -= py as f32; let v = stb__perlin_ease(y);
    z -= pz as f32; let w = stb__perlin_ease(z);

    let seed_i = seed as i32;
    let r0: i32 = stb__perlin_randtab[(x0 + seed_i) as usize] as i32;
    let r1: i32 = stb__perlin_randtab[(x1 + seed_i) as usize] as i32;

    let r00: i32 = stb__perlin_randtab[(r0 + y0) as usize] as i32;
    let r01: i32 = stb__perlin_randtab[(r0 + y1) as usize] as i32;
    let r10: i32 = stb__perlin_randtab[(r1 + y0) as usize] as i32;
    let r11: i32 = stb__perlin_randtab[(r1 + y1) as usize] as i32;

    let n000 = stb__perlin_grad(stb__perlin_randtab_grad_idx[(r00 + z0) as usize] as i32, x,       y,       z      );
    let n001 = stb__perlin_grad(stb__perlin_randtab_grad_idx[(r00 + z1) as usize] as i32, x,       y,       z - 1.0);
    let n010 = stb__perlin_grad(stb__perlin_randtab_grad_idx[(r01 + z0) as usize] as i32, x,       y - 1.0, z      );
    let n011 = stb__perlin_grad(stb__perlin_randtab_grad_idx[(r01 + z1) as usize] as i32, x,       y - 1.0, z - 1.0);
    let n100 = stb__perlin_grad(stb__perlin_randtab_grad_idx[(r10 + z0) as usize] as i32, x - 1.0, y,       z      );
    let n101 = stb__perlin_grad(stb__perlin_randtab_grad_idx[(r10 + z1) as usize] as i32, x - 1.0, y,       z - 1.0);
    let n110 = stb__perlin_grad(stb__perlin_randtab_grad_idx[(r11 + z0) as usize] as i32, x - 1.0, y - 1.0, z      );
    let n111 = stb__perlin_grad(stb__perlin_randtab_grad_idx[(r11 + z1) as usize] as i32, x - 1.0, y - 1.0, z - 1.0);

    let n00 = stb__perlin_lerp(n000, n001, w);
    let n01 = stb__perlin_lerp(n010, n011, w);
    let n10 = stb__perlin_lerp(n100, n101, w);
    let n11 = stb__perlin_lerp(n110, n111, w);

    let n0 = stb__perlin_lerp(n00, n01, v);
    let n1 = stb__perlin_lerp(n10, n11, v);

    stb__perlin_lerp(n0, n1, u)
}

fn stb_perlin_noise3_wrap_nonpow2_impl(
    mut x: f32,
    mut y: f32,
    mut z: f32,
    x_wrap: c_int,
    y_wrap: c_int,
    z_wrap: c_int,
    seed: u8,
) -> f32 {
    let px: i32 = stb__perlin_fastfloor(x);
    let py: i32 = stb__perlin_fastfloor(y);
    let pz: i32 = stb__perlin_fastfloor(z);
    let x_wrap2: i32 = if x_wrap != 0 { x_wrap } else { 256 };
    let y_wrap2: i32 = if y_wrap != 0 { y_wrap } else { 256 };
    let z_wrap2: i32 = if z_wrap != 0 { z_wrap } else { 256 };

    let mut x0: i32 = px % x_wrap2;
    let mut y0: i32 = py % y_wrap2;
    let mut z0: i32 = pz % z_wrap2;

    if x0 < 0 { x0 += x_wrap2; }
    if y0 < 0 { y0 += y_wrap2; }
    if z0 < 0 { z0 += z_wrap2; }
    let x1: i32 = (x0 + 1) % x_wrap2;
    let y1: i32 = (y0 + 1) % y_wrap2;
    let z1: i32 = (z0 + 1) % z_wrap2;

    x -= px as f32; let u = stb__perlin_ease(x);
    y -= py as f32; let v = stb__perlin_ease(y);
    z -= pz as f32; let w = stb__perlin_ease(z);

    let seed_i = seed as i32;

    let mut r0: i32 = stb__perlin_randtab[x0 as usize] as i32;
    r0 = stb__perlin_randtab[(r0 + seed_i) as usize] as i32;
    let mut r1: i32 = stb__perlin_randtab[x1 as usize] as i32;
    r1 = stb__perlin_randtab[(r1 + seed_i) as usize] as i32;

    let r00: i32 = stb__perlin_randtab[(r0 + y0) as usize] as i32;
    let r01: i32 = stb__perlin_randtab[(r0 + y1) as usize] as i32;
    let r10: i32 = stb__perlin_randtab[(r1 + y0) as usize] as i32;
    let r11: i32 = stb__perlin_randtab[(r1 + y1) as usize] as i32;

    let n000 = stb__perlin_grad(stb__perlin_randtab_grad_idx[(r00 + z0) as usize] as i32, x,       y,       z      );
    let n001 = stb__perlin_grad(stb__perlin_randtab_grad_idx[(r00 + z1) as usize] as i32, x,       y,       z - 1.0);
    let n010 = stb__perlin_grad(stb__perlin_randtab_grad_idx[(r01 + z0) as usize] as i32, x,       y - 1.0, z      );
    let n011 = stb__perlin_grad(stb__perlin_randtab_grad_idx[(r01 + z1) as usize] as i32, x,       y - 1.0, z - 1.0);
    let n100 = stb__perlin_grad(stb__perlin_randtab_grad_idx[(r10 + z0) as usize] as i32, x - 1.0, y,       z      );
    let n101 = stb__perlin_grad(stb__perlin_randtab_grad_idx[(r10 + z1) as usize] as i32, x - 1.0, y,       z - 1.0);
    let n110 = stb__perlin_grad(stb__perlin_randtab_grad_idx[(r11 + z0) as usize] as i32, x - 1.0, y - 1.0, z      );
    let n111 = stb__perlin_grad(stb__perlin_randtab_grad_idx[(r11 + z1) as usize] as i32, x - 1.0, y - 1.0, z - 1.0);

    let n00 = stb__perlin_lerp(n000, n001, w);
    let n01 = stb__perlin_lerp(n010, n011, w);
    let n10 = stb__perlin_lerp(n100, n101, w);
    let n11 = stb__perlin_lerp(n110, n111, w);

    let n0 = stb__perlin_lerp(n00, n01, v);
    let n1 = stb__perlin_lerp(n10, n11, v);

    stb__perlin_lerp(n0, n1, u)
}

// Mirror C semantics for `(float) fabs(r)` -- `fabs` takes/returns double, so
// the value is promoted to f64, abs'd, then truncated back to f32. Since abs
// only flips the sign bit this matches f32::abs() bit-for-bit, but we go
// through the explicit promotion path to guarantee identical behavior.
#[inline]
fn c_fabs_f32(r: f32) -> f32 {
    (r as f64).abs() as f32
}

// ---------------------------------------------------------------------------
// Public C API
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn stb_perlin_noise3_internal(
    x: f32,
    y: f32,
    z: f32,
    x_wrap: c_int,
    y_wrap: c_int,
    z_wrap: c_int,
    seed: u8,
) -> f32 {
    stb_perlin_noise3_internal_impl(x, y, z, x_wrap, y_wrap, z_wrap, seed)
}

#[unsafe(no_mangle)]
pub extern "C" fn stb_perlin_noise3(
    x: f32,
    y: f32,
    z: f32,
    x_wrap: c_int,
    y_wrap: c_int,
    z_wrap: c_int,
) -> f32 {
    stb_perlin_noise3_internal_impl(x, y, z, x_wrap, y_wrap, z_wrap, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn stb_perlin_noise3_seed(
    x: f32,
    y: f32,
    z: f32,
    x_wrap: c_int,
    y_wrap: c_int,
    z_wrap: c_int,
    seed: c_int,
) -> f32 {
    stb_perlin_noise3_internal_impl(x, y, z, x_wrap, y_wrap, z_wrap, seed as u8)
}

#[unsafe(no_mangle)]
pub extern "C" fn stb_perlin_ridge_noise3(
    x: f32,
    y: f32,
    z: f32,
    lacunarity: f32,
    gain: f32,
    offset: f32,
    octaves: c_int,
) -> f32 {
    let mut frequency: f32 = 1.0;
    let mut prev: f32 = 1.0;
    let mut amplitude: f32 = 0.5;
    let mut sum: f32 = 0.0;

    let mut i: c_int = 0;
    while i < octaves {
        let mut r = stb_perlin_noise3_internal_impl(
            x * frequency,
            y * frequency,
            z * frequency,
            0,
            0,
            0,
            i as u8,
        );
        r = offset - c_fabs_f32(r);
        r = r * r;
        sum += r * amplitude * prev;
        prev = r;
        frequency *= lacunarity;
        amplitude *= gain;
        i += 1;
    }
    sum
}

#[unsafe(no_mangle)]
pub extern "C" fn stb_perlin_fbm_noise3(
    x: f32,
    y: f32,
    z: f32,
    lacunarity: f32,
    gain: f32,
    octaves: c_int,
) -> f32 {
    let mut frequency: f32 = 1.0;
    let mut amplitude: f32 = 1.0;
    let mut sum: f32 = 0.0;

    let mut i: c_int = 0;
    while i < octaves {
        sum += stb_perlin_noise3_internal_impl(
            x * frequency,
            y * frequency,
            z * frequency,
            0,
            0,
            0,
            i as u8,
        ) * amplitude;
        frequency *= lacunarity;
        amplitude *= gain;
        i += 1;
    }
    sum
}

#[unsafe(no_mangle)]
pub extern "C" fn stb_perlin_turbulence_noise3(
    x: f32,
    y: f32,
    z: f32,
    lacunarity: f32,
    gain: f32,
    octaves: c_int,
) -> f32 {
    let mut frequency: f32 = 1.0;
    let mut amplitude: f32 = 1.0;
    let mut sum: f32 = 0.0;

    let mut i: c_int = 0;
    while i < octaves {
        let r = stb_perlin_noise3_internal_impl(
            x * frequency,
            y * frequency,
            z * frequency,
            0,
            0,
            0,
            i as u8,
        ) * amplitude;
        sum += c_fabs_f32(r);
        frequency *= lacunarity;
        amplitude *= gain;
        i += 1;
    }
    sum
}

#[unsafe(no_mangle)]
pub extern "C" fn stb_perlin_noise3_wrap_nonpow2(
    x: f32,
    y: f32,
    z: f32,
    x_wrap: c_int,
    y_wrap: c_int,
    z_wrap: c_int,
    seed: u8,
) -> f32 {
    stb_perlin_noise3_wrap_nonpow2_impl(x, y, z, x_wrap, y_wrap, z_wrap, seed)
}

// ---------------------------------------------------------------------------
// Driver `inner` function from main.c
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn inner(
    which: c_int,
    x: f32,
    y: f32,
    z: f32,
    x_wrap: c_int,
    y_wrap: c_int,
    z_wrap: c_int,
    seed: c_int,
    lacunarity: f32,
    gain: f32,
    offset: f32,
    octaves: c_int,
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
