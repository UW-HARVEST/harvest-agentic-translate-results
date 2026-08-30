//! Translation of stb_perlin.h - v0.5 (public domain, Sean Barrett).
//!
//! Every floating point expression is written with the same operand ordering
//! that gcc emits for the C source (see `sse.rs`), so that NaN sign/payload
//! propagation matches the C program bit for bit.

use crate::sse::{addss, fabs, mulss, subss};
use crate::tables::{STB_PERLIN_RANDTAB, STB_PERLIN_RANDTAB_GRAD_IDX};

// The C code can index its static tables out of bounds (undefined behaviour)
// through `stb_perlin_noise3_wrap_nonpow2` when the wrap arguments are not in
// 1..=256, and through `basis[grad_idx]` when a garbage gradient index is read.
// To stay memory safe while reproducing the compiled C program's behaviour, the
// three statics are modelled as one contiguous byte image laid out exactly as
// gcc lays them out (`stb__perlin_randtab`, then `stb__perlin_randtab_grad_idx`,
// then `basis`), with zero bytes beyond it.
const RANDTAB_BASE: i64 = 0;
const GRAD_IDX_BASE: i64 = 512;
const BASIS_BASE: i64 = 1024;
const BASIS_BYTES: i64 = 12 * 4 * 4;

// The C program's tables live inside its data segment; reads that leave the
// pages backing that segment fault. The reference build (`cmake .. && cmake
// --build .`, gcc, non-PIE) maps its whole ELF image at 0x400000..0x406000 and
// places `stb__perlin_randtab` at 0x405040, so relative to that symbol the
// mapped pages span [-20544, 4032). Reads outside of them are turned into the
// SIGSEGV that the C program dies from; reads inside them but outside the
// tables observe whatever the C program's image holds there, which is *not*
// all zeroes (`.dynamic`, `.got`, `.data`, `.rodata` and code lie below the
// tables).
const MAPPED_LOW: i64 = -20544;
const MAPPED_HIGH: i64 = 4032;

/// Byte-for-byte copy of 0x400000..0x406000 of the reference C process, taken
/// from `/proc/<pid>/mem` while it was blocked inside `scanf` (i.e. exactly the
/// state the noise routines observe). `DATA_IMAGE[0]` is offset `MAPPED_LOW`
/// relative to `stb__perlin_randtab`.
static DATA_IMAGE: &[u8; 24576] = include_bytes!("c_data_image.bin");

/// Byte at `offset` relative to `stb__perlin_randtab` in the C program's
/// address space.
fn mem(offset: i64) -> u8 {
    if offset < MAPPED_LOW || offset >= MAPPED_HIGH {
        crate::raise_sigsegv();
    }
    if offset < RANDTAB_BASE {
        // Below the tables: other parts of the C program's image.
        DATA_IMAGE[(offset - MAPPED_LOW) as usize]
    } else if offset < GRAD_IDX_BASE {
        STB_PERLIN_RANDTAB[(offset - RANDTAB_BASE) as usize]
    } else if offset < BASIS_BASE {
        STB_PERLIN_RANDTAB_GRAD_IDX[(offset - GRAD_IDX_BASE) as usize]
    } else if offset < BASIS_BASE + BASIS_BYTES {
        let k = (offset - BASIS_BASE) as usize;
        BASIS_FLAT[k / 4].to_le_bytes()[k % 4]
    } else {
        // Past `basis`: padding to the end of the last mapped page, all zero.
        DATA_IMAGE[(offset - MAPPED_LOW) as usize]
    }
}

fn mem_f32(offset: i64) -> f32 {
    f32::from_le_bytes([
        mem(offset),
        mem(offset + 1),
        mem(offset + 2),
        mem(offset + 3),
    ])
}

fn randtab(i: i64) -> i64 {
    mem(RANDTAB_BASE + i) as i64
}

fn randtab_grad_idx(i: i64) -> i64 {
    mem(GRAD_IDX_BASE + i) as i64
}

/// `a + (b-a) * t`
fn stb_perlin_lerp(a: f32, b: f32, t: f32) -> f32 {
    addss(mulss(subss(b, a), t), a)
}

/// `(int) a`, with x86-64 `cvttss2si` semantics (out of range / NaN -> INT_MIN).
fn f32_to_int(a: f32) -> i32 {
    if a >= -2147483648.0f32 && a < 2147483648.0f32 {
        a as i32
    } else {
        i32::MIN
    }
}

fn stb_perlin_fastfloor(a: f32) -> i32 {
    let ai = f32_to_int(a);
    if a < ai as f32 {
        ai.wrapping_sub(1)
    } else {
        ai
    }
}

/// `static float basis[12][4]` from `stb__perlin_grad`, flattened.
static BASIS_FLAT: [f32; 48] = [
    1.0, 1.0, 0.0, 0.0, //
    -1.0, 1.0, 0.0, 0.0, //
    1.0, -1.0, 0.0, 0.0, //
    -1.0, -1.0, 0.0, 0.0, //
    1.0, 0.0, 1.0, 0.0, //
    -1.0, 0.0, 1.0, 0.0, //
    1.0, 0.0, -1.0, 0.0, //
    -1.0, 0.0, -1.0, 0.0, //
    0.0, 1.0, 1.0, 0.0, //
    0.0, -1.0, 1.0, 0.0, //
    0.0, 1.0, -1.0, 0.0, //
    0.0, -1.0, -1.0, 0.0, //
];

/// `grad[0]*x + grad[1]*y + grad[2]*z`
fn stb_perlin_grad(grad_idx: i64, x: f32, y: f32, z: f32) -> f32 {
    let row = BASIS_BASE + grad_idx * 16;
    let g0 = mem_f32(row);
    let g1 = mem_f32(row + 4);
    let g2 = mem_f32(row + 8);
    let t01 = addss(mulss(g0, x), mulss(g1, y));
    addss(mulss(g2, z), t01)
}

/// `stb__perlin_ease(a)`: `((a*6-15)*a + 10) * a * a * a`
fn ease(a: f32) -> f32 {
    let t = mulss(6.0, a);
    let t = subss(t, 15.0);
    let t = mulss(t, a);
    let t = addss(10.0, t);
    let t = mulss(t, a);
    let t = mulss(t, a);
    mulss(t, a)
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
    let x_mask: u32 = (x_wrap as u32).wrapping_sub(1) & 255;
    let y_mask: u32 = (y_wrap as u32).wrapping_sub(1) & 255;
    let z_mask: u32 = (z_wrap as u32).wrapping_sub(1) & 255;
    let px = stb_perlin_fastfloor(x);
    let py = stb_perlin_fastfloor(y);
    let pz = stb_perlin_fastfloor(z);
    let x0 = ((px as u32) & x_mask) as i64;
    let x1 = ((px.wrapping_add(1) as u32) & x_mask) as i64;
    let y0 = ((py as u32) & y_mask) as i64;
    let y1 = ((py.wrapping_add(1) as u32) & y_mask) as i64;
    let z0 = ((pz as u32) & z_mask) as i64;
    let z1 = ((pz.wrapping_add(1) as u32) & z_mask) as i64;

    x = subss(x, px as f32);
    let u = ease(x);
    y = subss(y, py as f32);
    let v = ease(y);
    z = subss(z, pz as f32);
    let w = ease(z);

    let x_1 = subss(x, 1.0);
    let y_1 = subss(y, 1.0);
    let z_1 = subss(z, 1.0);

    let seed = seed as i64;
    let r0 = randtab(x0 + seed);
    let r1 = randtab(x1 + seed);

    let r00 = randtab(r0 + y0);
    let r01 = randtab(r0 + y1);
    let r10 = randtab(r1 + y0);
    let r11 = randtab(r1 + y1);

    let n000 = stb_perlin_grad(randtab_grad_idx(r00 + z0), x, y, z);
    let n001 = stb_perlin_grad(randtab_grad_idx(r00 + z1), x, y, z_1);
    let n010 = stb_perlin_grad(randtab_grad_idx(r01 + z0), x, y_1, z);
    let n011 = stb_perlin_grad(randtab_grad_idx(r01 + z1), x, y_1, z_1);
    let n100 = stb_perlin_grad(randtab_grad_idx(r10 + z0), x_1, y, z);
    let n101 = stb_perlin_grad(randtab_grad_idx(r10 + z1), x_1, y, z_1);
    let n110 = stb_perlin_grad(randtab_grad_idx(r11 + z0), x_1, y_1, z);
    let n111 = stb_perlin_grad(randtab_grad_idx(r11 + z1), x_1, y_1, z_1);

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

    let mut i: i32 = 0;
    while i < octaves {
        let mut r = stb_perlin_noise3_internal(
            mulss(x, frequency),
            mulss(y, frequency),
            mulss(z, frequency),
            0,
            0,
            0,
            i as u8,
        );
        r = subss(offset, fabs(r));
        r = mulss(r, r);
        sum = addss(mulss(mulss(r, amplitude), prev), sum);
        prev = r;
        frequency = mulss(frequency, lacunarity);
        amplitude = mulss(amplitude, gain);
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

    let mut i: i32 = 0;
    while i < octaves {
        let n = stb_perlin_noise3_internal(
            mulss(x, frequency),
            mulss(y, frequency),
            mulss(z, frequency),
            0,
            0,
            0,
            i as u8,
        );
        sum = addss(mulss(n, amplitude), sum);
        frequency = mulss(frequency, lacunarity);
        amplitude = mulss(amplitude, gain);
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

    let mut i: i32 = 0;
    while i < octaves {
        let n = stb_perlin_noise3_internal(
            mulss(x, frequency),
            mulss(y, frequency),
            mulss(z, frequency),
            0,
            0,
            0,
            i as u8,
        );
        let r = mulss(n, amplitude);
        sum = addss(fabs(r), sum);
        frequency = mulss(frequency, lacunarity);
        amplitude = mulss(amplitude, gain);
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
    let mut x0 = irem(px, x_wrap2);
    let mut y0 = irem(py, y_wrap2);
    let mut z0 = irem(pz, z_wrap2);

    if x0 < 0 {
        x0 = x0.wrapping_add(x_wrap2);
    }
    if y0 < 0 {
        y0 = y0.wrapping_add(y_wrap2);
    }
    if z0 < 0 {
        z0 = z0.wrapping_add(z_wrap2);
    }
    let x1 = irem(x0.wrapping_add(1), x_wrap2);
    let y1 = irem(y0.wrapping_add(1), y_wrap2);
    let z1 = irem(z0.wrapping_add(1), z_wrap2);

    x = subss(x, px as f32);
    let u = ease(x);
    y = subss(y, py as f32);
    let v = ease(y);
    z = subss(z, pz as f32);
    let w = ease(z);

    let x_1 = subss(x, 1.0);
    let y_1 = subss(y, 1.0);
    let z_1 = subss(z, 1.0);

    let (x0, x1) = (x0 as i64, x1 as i64);
    let (y0, y1) = (y0 as i64, y1 as i64);
    let (z0, z1) = (z0 as i64, z1 as i64);
    let seed = seed as i64;

    let mut r0 = randtab(x0);
    r0 = randtab(r0 + seed);
    let mut r1 = randtab(x1);
    r1 = randtab(r1 + seed);

    let r00 = randtab(r0 + y0);
    let r01 = randtab(r0 + y1);
    let r10 = randtab(r1 + y0);
    let r11 = randtab(r1 + y1);

    let n000 = stb_perlin_grad(randtab_grad_idx(r00 + z0), x, y, z);
    let n001 = stb_perlin_grad(randtab_grad_idx(r00 + z1), x, y, z_1);
    let n010 = stb_perlin_grad(randtab_grad_idx(r01 + z0), x, y_1, z);
    let n011 = stb_perlin_grad(randtab_grad_idx(r01 + z1), x, y_1, z_1);
    let n100 = stb_perlin_grad(randtab_grad_idx(r10 + z0), x_1, y, z);
    let n101 = stb_perlin_grad(randtab_grad_idx(r10 + z1), x_1, y, z_1);
    let n110 = stb_perlin_grad(randtab_grad_idx(r11 + z0), x_1, y_1, z);
    let n111 = stb_perlin_grad(randtab_grad_idx(r11 + z1), x_1, y_1, z_1);

    let n00 = stb_perlin_lerp(n000, n001, w);
    let n01 = stb_perlin_lerp(n010, n011, w);
    let n10 = stb_perlin_lerp(n100, n101, w);
    let n11 = stb_perlin_lerp(n110, n111, w);

    let n0 = stb_perlin_lerp(n00, n01, v);
    let n1 = stb_perlin_lerp(n10, n11, v);

    stb_perlin_lerp(n0, n1, u)
}

/// C's `%` on `int`s. `INT_MIN % -1` overflows, which traps with SIGFPE on
/// x86-64 (the C program dies the same way).
fn irem(a: i32, b: i32) -> i32 {
    if a == i32::MIN && b == -1 {
        crate::raise_sigfpe();
    }
    a.wrapping_rem(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The captured data image must agree with the tables transcribed from
    /// `stb_perlin.h` where they overlap; otherwise it was taken from a
    /// different build and the out-of-bounds model would be wrong.
    #[test]
    fn data_image_agrees_with_the_tables() {
        for (i, &b) in STB_PERLIN_RANDTAB.iter().enumerate() {
            let off = RANDTAB_BASE + i as i64;
            assert_eq!(
                DATA_IMAGE[(off - MAPPED_LOW) as usize], b,
                "randtab byte {i} differs from the captured image"
            );
        }
        for (i, &b) in STB_PERLIN_RANDTAB_GRAD_IDX.iter().enumerate() {
            let off = GRAD_IDX_BASE + i as i64;
            assert_eq!(
                DATA_IMAGE[(off - MAPPED_LOW) as usize], b,
                "grad_idx byte {i} differs from the captured image"
            );
        }
        for (i, &v) in BASIS_FLAT.iter().enumerate() {
            let off = BASIS_BASE + 4 * i as i64;
            let bytes: [u8; 4] = std::array::from_fn(|k| {
                DATA_IMAGE[(off + k as i64 - MAPPED_LOW) as usize]
            });
            assert_eq!(
                f32::from_le_bytes(bytes).to_bits(),
                v.to_bits(),
                "basis float {i} differs from the captured image"
            );
        }
        // Everything from the end of `basis` to the end of the last mapped page
        // is padding, and reads there see zeroes.
        let tail_start = (BASIS_BASE + BASIS_BYTES - MAPPED_LOW) as usize;
        let tail_end = (MAPPED_HIGH - MAPPED_LOW) as usize;
        assert!(DATA_IMAGE[tail_start..tail_end].iter().all(|&b| b == 0));
        assert_eq!(DATA_IMAGE.len(), tail_end);
    }
}
