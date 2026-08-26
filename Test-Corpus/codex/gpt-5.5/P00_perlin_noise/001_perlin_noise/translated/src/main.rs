use std::os::raw::{c_char, c_float, c_int};

unsafe extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

const RANDTAB: [u8; 256] = [
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

const GRAD_IDX: [u8; 256] = [
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

const BASIS: [[f32; 3]; 12] = [
    [1.0, 1.0, 0.0],
    [-1.0, 1.0, 0.0],
    [1.0, -1.0, 0.0],
    [-1.0, -1.0, 0.0],
    [1.0, 0.0, 1.0],
    [-1.0, 0.0, 1.0],
    [1.0, 0.0, -1.0],
    [-1.0, 0.0, -1.0],
    [0.0, 1.0, 1.0],
    [0.0, -1.0, 1.0],
    [0.0, 1.0, -1.0],
    [0.0, -1.0, -1.0],
];

fn randtab(index: i32) -> i32 {
    RANDTAB[(index as usize) & 255] as i32
}

fn grad_idx(index: i32) -> usize {
    GRAD_IDX[(index as usize) & 255] as usize
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn fastfloor(a: f32) -> i32 {
    let ai = a as i32;
    if a < ai as f32 {
        ai - 1
    } else {
        ai
    }
}

fn grad(grad_idx: usize, x: f32, y: f32, z: f32) -> f32 {
    let grad = BASIS[grad_idx];
    grad[0] * x + grad[1] * y + grad[2] * z
}

fn ease(a: f32) -> f32 {
    ((a * 6.0 - 15.0) * a + 10.0) * a * a * a
}

fn perlin_noise3_internal(
    mut x: f32,
    mut y: f32,
    mut z: f32,
    x_wrap: i32,
    y_wrap: i32,
    z_wrap: i32,
    seed: u8,
) -> f32 {
    let x_mask = (x_wrap - 1) & 255;
    let y_mask = (y_wrap - 1) & 255;
    let z_mask = (z_wrap - 1) & 255;
    let px = fastfloor(x);
    let py = fastfloor(y);
    let pz = fastfloor(z);
    let x0 = px & x_mask;
    let x1 = (px + 1) & x_mask;
    let y0 = py & y_mask;
    let y1 = (py + 1) & y_mask;
    let z0 = pz & z_mask;
    let z1 = (pz + 1) & z_mask;
    let seed = seed as i32;

    x -= px as f32;
    let u = ease(x);
    y -= py as f32;
    let v = ease(y);
    z -= pz as f32;
    let w = ease(z);

    let r0 = randtab(x0 + seed);
    let r1 = randtab(x1 + seed);

    let r00 = randtab(r0 + y0);
    let r01 = randtab(r0 + y1);
    let r10 = randtab(r1 + y0);
    let r11 = randtab(r1 + y1);

    let n000 = grad(grad_idx(r00 + z0), x, y, z);
    let n001 = grad(grad_idx(r00 + z1), x, y, z - 1.0);
    let n010 = grad(grad_idx(r01 + z0), x, y - 1.0, z);
    let n011 = grad(grad_idx(r01 + z1), x, y - 1.0, z - 1.0);
    let n100 = grad(grad_idx(r10 + z0), x - 1.0, y, z);
    let n101 = grad(grad_idx(r10 + z1), x - 1.0, y, z - 1.0);
    let n110 = grad(grad_idx(r11 + z0), x - 1.0, y - 1.0, z);
    let n111 = grad(grad_idx(r11 + z1), x - 1.0, y - 1.0, z - 1.0);

    let n00 = lerp(n000, n001, w);
    let n01 = lerp(n010, n011, w);
    let n10 = lerp(n100, n101, w);
    let n11 = lerp(n110, n111, w);

    let n0 = lerp(n00, n01, v);
    let n1 = lerp(n10, n11, v);

    lerp(n0, n1, u)
}

fn perlin_noise3(x: f32, y: f32, z: f32, x_wrap: i32, y_wrap: i32, z_wrap: i32) -> f32 {
    perlin_noise3_internal(x, y, z, x_wrap, y_wrap, z_wrap, 0)
}

fn perlin_noise3_seed(
    x: f32,
    y: f32,
    z: f32,
    x_wrap: i32,
    y_wrap: i32,
    z_wrap: i32,
    seed: i32,
) -> f32 {
    perlin_noise3_internal(x, y, z, x_wrap, y_wrap, z_wrap, seed as u8)
}

fn perlin_ridge_noise3(
    x: f32,
    y: f32,
    z: f32,
    lacunarity: f32,
    gain: f32,
    offset: f32,
    octaves: i32,
) -> f32 {
    let mut frequency = 1.0f32;
    let mut prev = 1.0f32;
    let mut amplitude = 0.5f32;
    let mut sum = 0.0f32;

    for i in 0..octaves {
        let mut r = perlin_noise3_internal(
            x * frequency,
            y * frequency,
            z * frequency,
            0,
            0,
            0,
            i as u8,
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

fn perlin_fbm_noise3(
    x: f32,
    y: f32,
    z: f32,
    lacunarity: f32,
    gain: f32,
    octaves: i32,
) -> f32 {
    let mut frequency = 1.0f32;
    let mut amplitude = 1.0f32;
    let mut sum = 0.0f32;

    for i in 0..octaves {
        sum += perlin_noise3_internal(
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
    }
    sum
}

fn perlin_turbulence_noise3(
    x: f32,
    y: f32,
    z: f32,
    lacunarity: f32,
    gain: f32,
    octaves: i32,
) -> f32 {
    let mut frequency = 1.0f32;
    let mut amplitude = 1.0f32;
    let mut sum = 0.0f32;

    for i in 0..octaves {
        let r = perlin_noise3_internal(
            x * frequency,
            y * frequency,
            z * frequency,
            0,
            0,
            0,
            i as u8,
        ) * amplitude;
        sum += r.abs();
        frequency *= lacunarity;
        amplitude *= gain;
    }
    sum
}

fn c_mod(a: i32, b: i32) -> i32 {
    a - (a / b) * b
}

fn perlin_noise3_wrap_nonpow2(
    mut x: f32,
    mut y: f32,
    mut z: f32,
    x_wrap: i32,
    y_wrap: i32,
    z_wrap: i32,
    seed: i32,
) -> f32 {
    let px = fastfloor(x);
    let py = fastfloor(y);
    let pz = fastfloor(z);
    let x_wrap2 = if x_wrap != 0 { x_wrap } else { 256 };
    let y_wrap2 = if y_wrap != 0 { y_wrap } else { 256 };
    let z_wrap2 = if z_wrap != 0 { z_wrap } else { 256 };
    let mut x0 = c_mod(px, x_wrap2);
    let mut y0 = c_mod(py, y_wrap2);
    let mut z0 = c_mod(pz, z_wrap2);

    if x0 < 0 {
        x0 += x_wrap2;
    }
    if y0 < 0 {
        y0 += y_wrap2;
    }
    if z0 < 0 {
        z0 += z_wrap2;
    }
    let x1 = c_mod(x0 + 1, x_wrap2);
    let y1 = c_mod(y0 + 1, y_wrap2);
    let z1 = c_mod(z0 + 1, z_wrap2);

    x -= px as f32;
    let u = ease(x);
    y -= py as f32;
    let v = ease(y);
    z -= pz as f32;
    let w = ease(z);

    let seed = (seed as u8) as i32;
    let mut r0 = randtab(x0);
    r0 = randtab(r0 + seed);
    let mut r1 = randtab(x1);
    r1 = randtab(r1 + seed);

    let r00 = randtab(r0 + y0);
    let r01 = randtab(r0 + y1);
    let r10 = randtab(r1 + y0);
    let r11 = randtab(r1 + y1);

    let n000 = grad(grad_idx(r00 + z0), x, y, z);
    let n001 = grad(grad_idx(r00 + z1), x, y, z - 1.0);
    let n010 = grad(grad_idx(r01 + z0), x, y - 1.0, z);
    let n011 = grad(grad_idx(r01 + z1), x, y - 1.0, z - 1.0);
    let n100 = grad(grad_idx(r10 + z0), x - 1.0, y, z);
    let n101 = grad(grad_idx(r10 + z1), x - 1.0, y, z - 1.0);
    let n110 = grad(grad_idx(r11 + z0), x - 1.0, y - 1.0, z);
    let n111 = grad(grad_idx(r11 + z1), x - 1.0, y - 1.0, z - 1.0);

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
        0 => perlin_noise3(x, y, z, x_wrap, y_wrap, z_wrap),
        1 => perlin_noise3_seed(x, y, z, x_wrap, y_wrap, z_wrap, seed),
        2 => perlin_ridge_noise3(x, y, z, lacunarity, gain, offset, octaves),
        3 => perlin_fbm_noise3(x, y, z, lacunarity, gain, octaves),
        4 => perlin_turbulence_noise3(x, y, z, lacunarity, gain, octaves),
        5 => perlin_noise3_wrap_nonpow2(x, y, z, x_wrap, y_wrap, z_wrap, seed),
        _ => f32::NAN,
    }
}

fn main() {
    let mut which: c_int = 0;
    let mut x: c_float = 0.0;
    let mut y: c_float = 0.0;
    let mut z: c_float = 0.0;
    let mut x_wrap: c_int = 0;
    let mut y_wrap: c_int = 0;
    let mut z_wrap: c_int = 0;
    let mut seed: c_int = 0;
    let mut lacunarity: c_float = 0.0;
    let mut gain: c_float = 0.0;
    let mut offset: c_float = 0.0;
    let mut octaves: c_int = 0;

    unsafe {
        scanf(
            c"%d%f%f%f%d%d%d%d%f%f%f%d".as_ptr(),
            &mut which,
            &mut x,
            &mut y,
            &mut z,
            &mut x_wrap,
            &mut y_wrap,
            &mut z_wrap,
            &mut seed,
            &mut lacunarity,
            &mut gain,
            &mut offset,
            &mut octaves,
        );
    }

    let res = inner(
        which, x, y, z, x_wrap, y_wrap, z_wrap, seed, lacunarity, gain, offset, octaves,
    );

    unsafe {
        printf(c"%.9g\n".as_ptr(), res as f64);
    }
}
