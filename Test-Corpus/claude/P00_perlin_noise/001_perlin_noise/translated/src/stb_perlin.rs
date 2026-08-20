//! Translation of `c_src/src/stb_perlin.h` (stb_perlin.h v0.5 by Sean Barrett).

use crate::tables::{RANDTAB, RANDTAB_GRAD_IDX};

/// Reads a byte out of the pair of permutation tables.
///
/// In the compiled C program the two tables are emitted next to each other in
/// declaration order, with `stb__perlin_randtab_grad_idx` right behind
/// `stb__perlin_randtab`.  The index arithmetic used by
/// `stb_perlin_noise3_wrap_nonpow2` can leave the bounds of a single table when
/// the wrap arguments are outside 1..=256, so both tables are modelled as one
/// contiguous 1024 byte block: offset 0..512 is the permutation table and
/// offset 512..1024 is the gradient index table.  Anything further out reads as
/// 0 rather than panicking.
fn read_table_mem(offset: i64) -> u8 {
    if (0..512).contains(&offset) {
        RANDTAB[offset as usize]
    } else if (512..1024).contains(&offset) {
        RANDTAB_GRAD_IDX[(offset - 512) as usize]
    } else {
        0
    }
}

/// `stb__perlin_randtab[index]`
fn randtab(index: i64) -> u8 {
    read_table_mem(index)
}

/// `stb__perlin_randtab_grad_idx[index]`
fn randtab_grad_idx(index: i64) -> u8 {
    read_table_mem(index + 512)
}

/// C's `(int)` conversion of a `float`, as implemented by x86-64 `cvttss2si`:
/// values that cannot be represented yield the "integer indefinite" value.
fn f32_to_i32(a: f32) -> i32 {
    if a.is_nan() || !(-2_147_483_648.0f32..2_147_483_648.0f32).contains(&a) {
        i32::MIN
    } else {
        a as i32
    }
}

fn stb_perlin_lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn stb_perlin_fastfloor(a: f32) -> i32 {
    let ai = f32_to_i32(a);
    if a < ai as f32 {
        ai.wrapping_sub(1)
    } else {
        ai
    }
}

/// `#define stb__perlin_ease(a) (((a*6-15)*a + 10) * a * a * a)`
fn stb_perlin_ease(a: f32) -> f32 {
    ((a * 6.0 - 15.0) * a + 10.0) * a * a * a
}

/// `static float basis[12][4]` from `stb__perlin_grad`; the unspecified fourth
/// component of every row is zero.
static BASIS: [[f32; 4]; 12] = [
    [1.0, 1.0, 0.0, 0.0],
    [-1.0, 1.0, 0.0, 0.0],
    [1.0, -1.0, 0.0, 0.0],
    [-1.0, -1.0, 0.0, 0.0],
    [1.0, 0.0, 1.0, 0.0],
    [-1.0, 0.0, 1.0, 0.0],
    [1.0, 0.0, -1.0, 0.0],
    [-1.0, 0.0, -1.0, 0.0],
    [0.0, 1.0, 1.0, 0.0],
    [0.0, -1.0, 1.0, 0.0],
    [0.0, 1.0, -1.0, 0.0],
    [0.0, -1.0, -1.0, 0.0],
];

fn stb_perlin_grad(grad_idx: i32, x: f32, y: f32, z: f32) -> f32 {
    // A gradient index outside 0..12 can only come from an out-of-bounds table
    // read; C would then read past `basis`, which has no meaningful value here.
    let grad = if (0..12).contains(&grad_idx) {
        BASIS[grad_idx as usize]
    } else {
        [0.0f32; 4]
    };
    grad[0] * x + grad[1] * y + grad[2] * z
}

pub fn stb_perlin_noise3_internal(
    mut x: f32,
    mut y: f32,
    mut z: f32,
    x_wrap: i32,
    y_wrap: i32,
    z_wrap: i32,
    seed: u8,
) -> f32 {
    let x_mask = (x_wrap.wrapping_sub(1) & 255) as u32;
    let y_mask = (y_wrap.wrapping_sub(1) & 255) as u32;
    let z_mask = (z_wrap.wrapping_sub(1) & 255) as u32;
    let px = stb_perlin_fastfloor(x);
    let py = stb_perlin_fastfloor(y);
    let pz = stb_perlin_fastfloor(z);
    let x0 = ((px as u32) & x_mask) as i64;
    let x1 = ((px.wrapping_add(1) as u32) & x_mask) as i64;
    let y0 = ((py as u32) & y_mask) as i64;
    let y1 = ((py.wrapping_add(1) as u32) & y_mask) as i64;
    let z0 = ((pz as u32) & z_mask) as i64;
    let z1 = ((pz.wrapping_add(1) as u32) & z_mask) as i64;

    x -= px as f32;
    let u = stb_perlin_ease(x);
    y -= py as f32;
    let v = stb_perlin_ease(y);
    z -= pz as f32;
    let w = stb_perlin_ease(z);

    let seed = i64::from(seed);
    let r0 = i64::from(randtab(x0 + seed));
    let r1 = i64::from(randtab(x1 + seed));

    let r00 = i64::from(randtab(r0 + y0));
    let r01 = i64::from(randtab(r0 + y1));
    let r10 = i64::from(randtab(r1 + y0));
    let r11 = i64::from(randtab(r1 + y1));

    let n000 = stb_perlin_grad(randtab_grad_idx(r00 + z0).into(), x, y, z);
    let n001 = stb_perlin_grad(randtab_grad_idx(r00 + z1).into(), x, y, z - 1.0);
    let n010 = stb_perlin_grad(randtab_grad_idx(r01 + z0).into(), x, y - 1.0, z);
    let n011 = stb_perlin_grad(randtab_grad_idx(r01 + z1).into(), x, y - 1.0, z - 1.0);
    let n100 = stb_perlin_grad(randtab_grad_idx(r10 + z0).into(), x - 1.0, y, z);
    let n101 = stb_perlin_grad(randtab_grad_idx(r10 + z1).into(), x - 1.0, y, z - 1.0);
    let n110 = stb_perlin_grad(randtab_grad_idx(r11 + z0).into(), x - 1.0, y - 1.0, z);
    let n111 = stb_perlin_grad(randtab_grad_idx(r11 + z1).into(), x - 1.0, y - 1.0, z - 1.0);

    let n00 = stb_perlin_lerp(n000, n001, w);
    let n01 = stb_perlin_lerp(n010, n011, w);
    let n10 = stb_perlin_lerp(n100, n101, w);
    let n11 = stb_perlin_lerp(n110, n111, w);

    let n0 = stb_perlin_lerp(n00, n01, v);
    let n1 = stb_perlin_lerp(n10, n11, v);

    stb_perlin_lerp(n0, n1, u)
}

pub fn stb_perlin_noise3(x: f32, y: f32, z: f32, x_wrap: i32, y_wrap: i32, z_wrap: i32) -> f32 {
    stb_perlin_noise3_internal(x, y, z, x_wrap, y_wrap, z_wrap, 0)
}

pub fn stb_perlin_noise3_seed(
    x: f32,
    y: f32,
    z: f32,
    x_wrap: i32,
    y_wrap: i32,
    z_wrap: i32,
    seed: i32,
) -> f32 {
    stb_perlin_noise3_internal(x, y, z, x_wrap, y_wrap, z_wrap, seed as u8)
}

pub fn stb_perlin_ridge_noise3(
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

    let mut i = 0i32;
    while i < octaves {
        let mut r = stb_perlin_noise3_internal(
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
        i += 1;
    }
    sum
}

pub fn stb_perlin_fbm_noise3(
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

    let mut i = 0i32;
    while i < octaves {
        sum += stb_perlin_noise3_internal(
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

pub fn stb_perlin_turbulence_noise3(
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

    let mut i = 0i32;
    while i < octaves {
        let r = stb_perlin_noise3_internal(
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
        i += 1;
    }
    sum
}

pub fn stb_perlin_noise3_wrap_nonpow2(
    mut x: f32,
    mut y: f32,
    mut z: f32,
    x_wrap: i32,
    y_wrap: i32,
    z_wrap: i32,
    seed: u8,
) -> f32 {
    let px = stb_perlin_fastfloor(x);
    let py = stb_perlin_fastfloor(y);
    let pz = stb_perlin_fastfloor(z);
    let x_wrap2 = if x_wrap != 0 { x_wrap } else { 256 };
    let y_wrap2 = if y_wrap != 0 { y_wrap } else { 256 };
    let z_wrap2 = if z_wrap != 0 { z_wrap } else { 256 };
    let mut x0 = px.wrapping_rem(x_wrap2);
    let mut y0 = py.wrapping_rem(y_wrap2);
    let mut z0 = pz.wrapping_rem(z_wrap2);

    if x0 < 0 {
        x0 = x0.wrapping_add(x_wrap2);
    }
    if y0 < 0 {
        y0 = y0.wrapping_add(y_wrap2);
    }
    if z0 < 0 {
        z0 = z0.wrapping_add(z_wrap2);
    }
    let x1 = x0.wrapping_add(1).wrapping_rem(x_wrap2);
    let y1 = y0.wrapping_add(1).wrapping_rem(y_wrap2);
    let z1 = z0.wrapping_add(1).wrapping_rem(z_wrap2);

    x -= px as f32;
    let u = stb_perlin_ease(x);
    y -= py as f32;
    let v = stb_perlin_ease(y);
    z -= pz as f32;
    let w = stb_perlin_ease(z);

    let seed = i64::from(seed);
    let (x0, x1) = (i64::from(x0), i64::from(x1));
    let (y0, y1) = (i64::from(y0), i64::from(y1));
    let (z0, z1) = (i64::from(z0), i64::from(z1));

    let mut r0 = i64::from(randtab(x0));
    r0 = i64::from(randtab(r0 + seed));
    let mut r1 = i64::from(randtab(x1));
    r1 = i64::from(randtab(r1 + seed));

    let r00 = i64::from(randtab(r0 + y0));
    let r01 = i64::from(randtab(r0 + y1));
    let r10 = i64::from(randtab(r1 + y0));
    let r11 = i64::from(randtab(r1 + y1));

    let n000 = stb_perlin_grad(randtab_grad_idx(r00 + z0).into(), x, y, z);
    let n001 = stb_perlin_grad(randtab_grad_idx(r00 + z1).into(), x, y, z - 1.0);
    let n010 = stb_perlin_grad(randtab_grad_idx(r01 + z0).into(), x, y - 1.0, z);
    let n011 = stb_perlin_grad(randtab_grad_idx(r01 + z1).into(), x, y - 1.0, z - 1.0);
    let n100 = stb_perlin_grad(randtab_grad_idx(r10 + z0).into(), x - 1.0, y, z);
    let n101 = stb_perlin_grad(randtab_grad_idx(r10 + z1).into(), x - 1.0, y, z - 1.0);
    let n110 = stb_perlin_grad(randtab_grad_idx(r11 + z0).into(), x - 1.0, y - 1.0, z);
    let n111 = stb_perlin_grad(randtab_grad_idx(r11 + z1).into(), x - 1.0, y - 1.0, z - 1.0);

    let n00 = stb_perlin_lerp(n000, n001, w);
    let n01 = stb_perlin_lerp(n010, n011, w);
    let n10 = stb_perlin_lerp(n100, n101, w);
    let n11 = stb_perlin_lerp(n110, n111, w);

    let n0 = stb_perlin_lerp(n00, n01, v);
    let n1 = stb_perlin_lerp(n10, n11, v);

    stb_perlin_lerp(n0, n1, u)
}
