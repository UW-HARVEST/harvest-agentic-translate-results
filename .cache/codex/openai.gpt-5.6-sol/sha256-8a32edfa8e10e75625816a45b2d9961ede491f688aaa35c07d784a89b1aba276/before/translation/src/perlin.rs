const RANDTAB: [u8; 256] = [
    23, 125, 161, 52, 103, 117, 70, 37, 247, 101, 203, 169, 124, 126, 44, 123, 152, 238, 145, 45,
    171, 114, 253, 10, 192, 136, 4, 157, 249, 30, 35, 72, 175, 63, 77, 90, 181, 16, 96, 111, 133,
    104, 75, 162, 93, 56, 66, 240, 8, 50, 84, 229, 49, 210, 173, 239, 141, 1, 87, 18, 2, 198, 143,
    57, 225, 160, 58, 217, 168, 206, 245, 204, 199, 6, 73, 60, 20, 230, 211, 233, 94, 200, 88, 9,
    74, 155, 33, 15, 219, 130, 226, 202, 83, 236, 42, 172, 165, 218, 55, 222, 46, 107, 98, 154,
    109, 67, 196, 178, 127, 158, 13, 243, 65, 79, 166, 248, 25, 224, 115, 80, 68, 51, 184, 128,
    232, 208, 151, 122, 26, 212, 105, 43, 179, 213, 235, 148, 146, 89, 14, 195, 28, 78, 112, 76,
    250, 47, 24, 251, 140, 108, 186, 190, 228, 170, 183, 139, 39, 188, 244, 246, 132, 48, 119, 144,
    180, 138, 134, 193, 82, 182, 120, 121, 86, 220, 209, 3, 91, 241, 149, 85, 205, 150, 113, 216,
    31, 100, 41, 164, 177, 214, 153, 231, 38, 71, 185, 174, 97, 201, 29, 95, 7, 92, 54, 254, 191,
    118, 34, 221, 131, 11, 163, 99, 234, 81, 227, 147, 156, 176, 17, 142, 69, 12, 110, 62, 27, 255,
    0, 194, 59, 116, 242, 252, 19, 21, 187, 53, 207, 129, 64, 135, 61, 40, 167, 237, 102, 223, 106,
    159, 197, 189, 215, 137, 36, 32, 22, 5,
];

const GRAD_IDX: [u8; 256] = [
    7, 9, 5, 0, 11, 1, 6, 9, 3, 9, 11, 1, 8, 10, 4, 7, 8, 6, 1, 5, 3, 10, 9, 10, 0, 8, 4, 1, 5, 2,
    7, 8, 7, 11, 9, 10, 1, 0, 4, 7, 5, 0, 11, 6, 1, 4, 2, 8, 8, 10, 4, 9, 9, 2, 5, 7, 9, 1, 7, 2,
    2, 6, 11, 5, 5, 4, 6, 9, 0, 1, 1, 0, 7, 6, 9, 8, 4, 10, 3, 1, 2, 8, 8, 9, 10, 11, 5, 11, 11, 2,
    6, 10, 3, 4, 2, 4, 9, 10, 3, 2, 6, 3, 6, 10, 5, 3, 4, 10, 11, 2, 9, 11, 1, 11, 10, 4, 9, 4, 11,
    0, 4, 11, 4, 0, 0, 0, 7, 6, 10, 4, 1, 3, 11, 5, 3, 4, 2, 9, 1, 3, 0, 1, 8, 0, 6, 7, 8, 7, 0, 4,
    6, 10, 8, 2, 3, 11, 11, 8, 0, 2, 4, 8, 3, 0, 0, 10, 6, 1, 2, 2, 4, 5, 6, 0, 1, 3, 11, 9, 5, 5,
    9, 6, 9, 8, 3, 8, 1, 8, 9, 6, 9, 11, 10, 7, 5, 6, 5, 9, 1, 3, 7, 0, 2, 10, 11, 2, 6, 1, 3, 11,
    7, 7, 2, 1, 7, 3, 0, 8, 1, 1, 5, 0, 6, 10, 11, 11, 0, 2, 7, 0, 10, 8, 3, 5, 7, 1, 11, 1, 0, 7,
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

#[inline]
fn randtab(index: usize) -> usize {
    RANDTAB[index & 255] as usize
}

#[inline]
fn grad_idx(index: usize) -> usize {
    GRAD_IDX[index & 255] as usize
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[inline]
fn fastfloor(a: f32) -> i32 {
    let ai = a as i32;
    if a < ai as f32 {
        ai.wrapping_sub(1)
    } else {
        ai
    }
}

#[inline]
fn grad(index: usize, x: f32, y: f32, z: f32) -> f32 {
    let g = BASIS[index];
    g[0] * x + g[1] * y + g[2] * z
}

#[inline]
fn ease(a: f32) -> f32 {
    (((a * 6.0 - 15.0) * a + 10.0) * a * a) * a
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
    let x_mask = (x_wrap.wrapping_sub(1) as u32) & 255;
    let y_mask = (y_wrap.wrapping_sub(1) as u32) & 255;
    let z_mask = (z_wrap.wrapping_sub(1) as u32) & 255;
    let px = fastfloor(x);
    let py = fastfloor(y);
    let pz = fastfloor(z);
    let x0 = ((px as u32) & x_mask) as usize;
    let x1 = ((px.wrapping_add(1) as u32) & x_mask) as usize;
    let y0 = ((py as u32) & y_mask) as usize;
    let y1 = ((py.wrapping_add(1) as u32) & y_mask) as usize;
    let z0 = ((pz as u32) & z_mask) as usize;
    let z1 = ((pz.wrapping_add(1) as u32) & z_mask) as usize;

    x -= px as f32;
    let u = ease(x);
    y -= py as f32;
    let v = ease(y);
    z -= pz as f32;
    let w = ease(z);

    let r0 = randtab(x0 + seed as usize);
    let r1 = randtab(x1 + seed as usize);
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

fn noise3(x: f32, y: f32, z: f32, x_wrap: i32, y_wrap: i32, z_wrap: i32) -> f32 {
    noise3_internal(x, y, z, x_wrap, y_wrap, z_wrap, 0)
}

fn noise3_seed(x: f32, y: f32, z: f32, x_wrap: i32, y_wrap: i32, z_wrap: i32, seed: i32) -> f32 {
    noise3_internal(x, y, z, x_wrap, y_wrap, z_wrap, seed as u8)
}

fn ridge_noise3(
    x: f32,
    y: f32,
    z: f32,
    lacunarity: f32,
    gain: f32,
    offset: f32,
    octaves: i32,
) -> f32 {
    let mut frequency = 1.0f32;
    let mut previous = 1.0f32;
    let mut amplitude = 0.5f32;
    let mut sum = 0.0f32;
    for i in 0..octaves {
        let mut r = noise3_internal(
            x * frequency,
            y * frequency,
            z * frequency,
            0,
            0,
            0,
            i as u8,
        );
        r = offset - r.abs();
        r *= r;
        sum += r * amplitude * previous;
        previous = r;
        frequency *= lacunarity;
        amplitude *= gain;
    }
    sum
}

fn fbm_noise3(x: f32, y: f32, z: f32, lacunarity: f32, gain: f32, octaves: i32) -> f32 {
    let mut frequency = 1.0f32;
    let mut amplitude = 1.0f32;
    let mut sum = 0.0f32;
    for i in 0..octaves {
        sum += noise3_internal(
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

fn turbulence_noise3(x: f32, y: f32, z: f32, lacunarity: f32, gain: f32, octaves: i32) -> f32 {
    let mut frequency = 1.0f32;
    let mut amplitude = 1.0f32;
    let mut sum = 0.0f32;
    for i in 0..octaves {
        let r = noise3_internal(
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

fn noise3_wrap_nonpow2(
    mut x: f32,
    mut y: f32,
    mut z: f32,
    x_wrap: i32,
    y_wrap: i32,
    z_wrap: i32,
    seed: u8,
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
    if x0 < 0 {
        x0 += x_wrap2;
    }
    if y0 < 0 {
        y0 += y_wrap2;
    }
    if z0 < 0 {
        z0 += z_wrap2;
    }
    let x1 = (x0 + 1) % x_wrap2;
    let y1 = (y0 + 1) % y_wrap2;
    let z1 = (z0 + 1) % z_wrap2;
    let x0 = x0 as usize;
    let x1 = x1 as usize;
    let y0 = y0 as usize;
    let y1 = y1 as usize;
    let z0 = z0 as usize;
    let z1 = z1 as usize;

    x -= px as f32;
    let u = ease(x);
    y -= py as f32;
    let v = ease(y);
    z -= pz as f32;
    let w = ease(z);

    let r0 = randtab(randtab(x0) + seed as usize);
    let r1 = randtab(randtab(x1) + seed as usize);
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

#[allow(clippy::too_many_arguments)]
pub fn inner(
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
        0 => noise3(x, y, z, x_wrap, y_wrap, z_wrap),
        1 => noise3_seed(x, y, z, x_wrap, y_wrap, z_wrap, seed),
        2 => ridge_noise3(x, y, z, lacunarity, gain, offset, octaves),
        3 => fbm_noise3(x, y, z, lacunarity, gain, octaves),
        4 => turbulence_noise3(x, y, z, lacunarity, gain, octaves),
        5 => noise3_wrap_nonpow2(x, y, z, x_wrap, y_wrap, z_wrap, seed as u8),
        _ => f32::NAN,
    }
}
