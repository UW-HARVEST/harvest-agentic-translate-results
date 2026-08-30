//! Direct translation of `stb_perlin.h` (v0.5, public domain, by Sean Barrett).
//!
//! All arithmetic is kept in `f32` exactly as the C code does, and index
//! arithmetic mirrors the C integer conversions (wrapping, truncation).

use crate::sse;

/// Base 256-entry permutation table. The C file stores it twice back-to-back
/// "so we don't need an extra mask"; we build the 512-entry table from it.
const RANDTAB_BASE: [u8; 256] = [
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

/// Base 256-entry gradient-index table (matches `indices[randtab[i]&63]`).
const RANDTAB_GRAD_IDX_BASE: [u8; 256] = [
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

/// Image of the C program's writable data segment around the three static
/// tables.
///
/// `stb_perlin_noise3_wrap_nonpow2` indexes the tables with unvalidated wrap
/// values, so wraps outside `0..=256` (or negative ones) make the C code read
/// past the end of a table -- undefined behaviour, but in the compiled program
/// it deterministically reads whatever follows. gcc emits the three objects
/// contiguously in declaration order, each 32-byte aligned and a multiple of 32
/// bytes in size:
///
/// ```text
///   0x405020  32 bytes of zero padding (start of .data)
///   0x405040  stb__perlin_randtab          512 bytes
///   0x405240  stb__perlin_randtab_grad_idx 512 bytes
///   0x405440  basis.0                      192 bytes  (12 x 4 floats)
///   0x405500  _edata / .bss (zeros)
/// ```
///
/// Reproducing that image keeps those reads faithful instead of merely
/// panic-free. Anything outside the modelled window reads as zero.
const PRE_PAD: usize = 32;
const RANDTAB_OFF: usize = PRE_PAD;
const GRAD_IDX_OFF: usize = PRE_PAD + 512;
const BASIS_OFF: usize = PRE_PAD + 1024;
const DATA_LEN: usize = PRE_PAD + 512 + 512 + 192;

const fn build_data() -> [u8; DATA_LEN] {
    let mut out = [0u8; DATA_LEN];
    let mut i = 0;
    while i < 256 {
        out[RANDTAB_OFF + i] = RANDTAB_BASE[i];
        out[RANDTAB_OFF + 256 + i] = RANDTAB_BASE[i];
        out[GRAD_IDX_OFF + i] = RANDTAB_GRAD_IDX_BASE[i];
        out[GRAD_IDX_OFF + 256 + i] = RANDTAB_GRAD_IDX_BASE[i];
        i += 1;
    }
    // basis[12][4], little-endian f32, row stride 16 bytes.
    let mut r = 0;
    while r < 12 {
        let mut c = 0;
        while c < 4 {
            let bytes = BASIS[r][c].to_le_bytes();
            let at = BASIS_OFF + r * 16 + c * 4;
            out[at] = bytes[0];
            out[at + 1] = bytes[1];
            out[at + 2] = bytes[2];
            out[at + 3] = bytes[3];
            c += 1;
        }
        r += 1;
    }
    out
}

static DATA: [u8; DATA_LEN] = build_data();

#[inline]
fn data_byte(off: i64) -> u8 {
    if off >= 0 && (off as u64) < DATA_LEN as u64 {
        DATA[off as usize]
    } else {
        0
    }
}

#[inline]
fn data_f32(off: i64) -> f32 {
    let b = [
        data_byte(off),
        data_byte(off + 1),
        data_byte(off + 2),
        data_byte(off + 3),
    ];
    f32::from_le_bytes(b)
}

/// `stb__perlin_randtab[i]`
#[inline]
fn randtab(i: i32) -> i32 {
    data_byte(RANDTAB_OFF as i64 + i as i64) as i32
}

/// `stb__perlin_randtab_grad_idx[i]`
#[inline]
fn randtab_grad_idx(i: i32) -> i32 {
    data_byte(GRAD_IDX_OFF as i64 + i as i64) as i32
}

/// ```c
/// static float stb__perlin_lerp(float a, float b, float t)
/// {
///    return a + (b-a) * t;
/// }
/// ```
///
/// gcc computes `(b-a)` then `*t` in `xmm0` and finishes with
/// `addss -0x4(%rbp),%xmm0`, i.e. the product is the destination operand.
#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    sse::add(sse::mul(sse::sub(b, a), t), a)
}

/// `(int) a` with x86-64 `cvttss2si` semantics: values that do not fit (and
/// NaN) yield `INT_MIN`. Rust's `as` already saturates to `INT_MIN` on the
/// negative side, so only the positive/NaN cases need handling.
#[inline]
fn c_int_cast(a: f32) -> i32 {
    if a.is_nan() || a >= 2147483648.0f32 {
        i32::MIN
    } else {
        a as i32
    }
}

/// ```c
/// static int stb__perlin_fastfloor(float a)
/// {
///     int ai = (int) a;
///     return (a < ai) ? ai-1 : ai;
/// }
/// ```
#[inline]
fn fastfloor(a: f32) -> i32 {
    let ai = c_int_cast(a);
    if a < ai as f32 {
        ai.wrapping_sub(1)
    } else {
        ai
    }
}

/// `basis[12][4]` from `stb__perlin_grad`; the unwritten 4th column is zero.
const BASIS: [[f32; 4]; 12] = [
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

#[inline]
fn grad(grad_idx: i32, x: f32, y: f32, z: f32) -> f32 {
    // `float *grad = basis[grad_idx];` -- the tables only ever yield 0..=11 for
    // well-defined inputs, but an out-of-range index reads past `basis`, so go
    // through the data-segment image (zeros beyond it) rather than indexing.
    //
    // `grad[0]*x + grad[1]*y + grad[2]*z`: gcc keeps `grad[0]*x` as the
    // destination of the first `addss` and `grad[2]*z` as the destination of the
    // second one.
    let base = BASIS_OFF as i64 + grad_idx as i64 * 16;
    let gx = sse::mul(data_f32(base), x);
    let gy = sse::mul(data_f32(base + 4), y);
    let gz = sse::mul(data_f32(base + 8), z);
    sse::add(gz, sse::add(gx, gy))
}

/// `#define stb__perlin_ease(a) (((a*6-15)*a + 10) * a * a * a)`
#[inline]
fn ease(a: f32) -> f32 {
    ((a * 6.0 - 15.0) * a + 10.0) * a * a * a
}

pub fn noise3_internal(
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

    let px = fastfloor(x);
    let py = fastfloor(y);
    let pz = fastfloor(z);

    let x0 = ((px as u32) & x_mask) as i32;
    let x1 = ((px.wrapping_add(1) as u32) & x_mask) as i32;
    let y0 = ((py as u32) & y_mask) as i32;
    let y1 = ((py.wrapping_add(1) as u32) & y_mask) as i32;
    let z0 = ((pz as u32) & z_mask) as i32;
    let z1 = ((pz.wrapping_add(1) as u32) & z_mask) as i32;

    x -= px as f32;
    let u = ease(x);
    y -= py as f32;
    let v = ease(y);
    z -= pz as f32;
    let w = ease(z);

    let seed = seed as i32;
    let r0 = randtab(x0 + seed);
    let r1 = randtab(x1 + seed);

    let r00 = randtab(r0 + y0);
    let r01 = randtab(r0 + y1);
    let r10 = randtab(r1 + y0);
    let r11 = randtab(r1 + y1);

    let n000 = grad(randtab_grad_idx(r00 + z0), x, y, z);
    let n001 = grad(randtab_grad_idx(r00 + z1), x, y, z - 1.0);
    let n010 = grad(randtab_grad_idx(r01 + z0), x, y - 1.0, z);
    let n011 = grad(randtab_grad_idx(r01 + z1), x, y - 1.0, z - 1.0);
    let n100 = grad(randtab_grad_idx(r10 + z0), x - 1.0, y, z);
    let n101 = grad(randtab_grad_idx(r10 + z1), x - 1.0, y, z - 1.0);
    let n110 = grad(randtab_grad_idx(r11 + z0), x - 1.0, y - 1.0, z);
    let n111 = grad(randtab_grad_idx(r11 + z1), x - 1.0, y - 1.0, z - 1.0);

    let n00 = lerp(n000, n001, w);
    let n01 = lerp(n010, n011, w);
    let n10 = lerp(n100, n101, w);
    let n11 = lerp(n110, n111, w);

    let n0 = lerp(n00, n01, v);
    let n1 = lerp(n10, n11, v);

    lerp(n0, n1, u)
}

pub fn noise3(x: f32, y: f32, z: f32, x_wrap: i32, y_wrap: i32, z_wrap: i32) -> f32 {
    noise3_internal(x, y, z, x_wrap, y_wrap, z_wrap, 0)
}

pub fn noise3_seed(
    x: f32,
    y: f32,
    z: f32,
    x_wrap: i32,
    y_wrap: i32,
    z_wrap: i32,
    seed: i32,
) -> f32 {
    noise3_internal(x, y, z, x_wrap, y_wrap, z_wrap, seed as u8)
}

pub fn ridge_noise3(
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
        let mut r = noise3_internal(
            sse::mul(x, frequency),
            sse::mul(y, frequency),
            sse::mul(z, frequency),
            0,
            0,
            0,
            i as u8,
        );
        r = sse::sub(offset, r.abs());
        r = sse::mul(r, r);
        // `sum += r*amplitude*prev` -- the product is the addss destination.
        sum = sse::add(sse::mul(sse::mul(r, amplitude), prev), sum);
        prev = r;
        frequency = sse::mul(frequency, lacunarity);
        amplitude = sse::mul(amplitude, gain);
        i += 1;
    }
    sum
}

pub fn fbm_noise3(x: f32, y: f32, z: f32, lacunarity: f32, gain: f32, octaves: i32) -> f32 {
    let mut frequency = 1.0f32;
    let mut amplitude = 1.0f32;
    let mut sum = 0.0f32;

    let mut i: i32 = 0;
    while i < octaves {
        let n = noise3_internal(
            sse::mul(x, frequency),
            sse::mul(y, frequency),
            sse::mul(z, frequency),
            0,
            0,
            0,
            i as u8,
        );
        sum = sse::add(sse::mul(n, amplitude), sum);
        frequency = sse::mul(frequency, lacunarity);
        amplitude = sse::mul(amplitude, gain);
        i += 1;
    }
    sum
}

pub fn turbulence_noise3(x: f32, y: f32, z: f32, lacunarity: f32, gain: f32, octaves: i32) -> f32 {
    let mut frequency = 1.0f32;
    let mut amplitude = 1.0f32;
    let mut sum = 0.0f32;

    let mut i: i32 = 0;
    while i < octaves {
        let r = sse::mul(
            noise3_internal(
                sse::mul(x, frequency),
                sse::mul(y, frequency),
                sse::mul(z, frequency),
                0,
                0,
                0,
                i as u8,
            ),
            amplitude,
        );
        sum = sse::add(r.abs(), sum);
        frequency = sse::mul(frequency, lacunarity);
        amplitude = sse::mul(amplitude, gain);
        i += 1;
    }
    sum
}

/// Reproduces `stb_perlin_noise3_wrap_nonpow2`.
///
/// Note that wrap values outside `0..=256`, and negative ones, make the C code
/// index its tables out of bounds. Reads that stay within the program's data
/// pages are reproduced faithfully via the `DATA` image above; sufficiently wild
/// indices make the C program segfault, which is not emulated here.
pub fn noise3_wrap_nonpow2(
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

    // `px % x_wrap2` with `px == INT_MIN` and a wrap of -1 overflows; the `idiv`
    // gcc emits raises SIGFPE, so the C program dies here having printed
    // nothing. Reproduce the empty stdout and a failing exit status (136 is the
    // shell's encoding of SIGFPE) rather than inventing a result.
    if (px == i32::MIN && x_wrap2 == -1)
        || (py == i32::MIN && y_wrap2 == -1)
        || (pz == i32::MIN && z_wrap2 == -1)
    {
        std::process::exit(136);
    }

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
    let u = ease(x);
    y -= py as f32;
    let v = ease(y);
    z -= pz as f32;
    let w = ease(z);

    let seed = seed as i32;
    let mut r0 = randtab(x0);
    r0 = randtab(r0 + seed);
    let mut r1 = randtab(x1);
    r1 = randtab(r1 + seed);

    let r00 = randtab(r0 + y0);
    let r01 = randtab(r0 + y1);
    let r10 = randtab(r1 + y0);
    let r11 = randtab(r1 + y1);

    let n000 = grad(randtab_grad_idx(r00 + z0), x, y, z);
    let n001 = grad(randtab_grad_idx(r00 + z1), x, y, z - 1.0);
    let n010 = grad(randtab_grad_idx(r01 + z0), x, y - 1.0, z);
    let n011 = grad(randtab_grad_idx(r01 + z1), x, y - 1.0, z - 1.0);
    let n100 = grad(randtab_grad_idx(r10 + z0), x - 1.0, y, z);
    let n101 = grad(randtab_grad_idx(r10 + z1), x - 1.0, y, z - 1.0);
    let n110 = grad(randtab_grad_idx(r11 + z0), x - 1.0, y - 1.0, z);
    let n111 = grad(randtab_grad_idx(r11 + z1), x - 1.0, y - 1.0, z - 1.0);

    let n00 = lerp(n000, n001, w);
    let n01 = lerp(n010, n011, w);
    let n10 = lerp(n100, n101, w);
    let n11 = lerp(n110, n111, w);

    let n0 = lerp(n00, n01, v);
    let n1 = lerp(n10, n11, v);

    lerp(n0, n1, u)
}
