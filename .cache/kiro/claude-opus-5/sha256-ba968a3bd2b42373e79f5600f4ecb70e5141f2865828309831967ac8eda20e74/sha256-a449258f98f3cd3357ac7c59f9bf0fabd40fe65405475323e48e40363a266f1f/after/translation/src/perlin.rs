//! Faithful Rust translation of `c_src/src/stb_perlin.h`
//! (stb_perlin.h - v0.5 - public domain, by Sean Barrett).
//!
//! Every arithmetic step is kept in `f32` exactly as the C does, and the
//! integer index arithmetic mirrors C's conversions (including the wrapping
//! behaviour of `int` overflow and x86's float->int truncation semantics).

use crate::memimage::{
    BASIS_ADDR, GRAD_IDX_ADDR, IMAGE, IMAGE_BASE, IMAGE_END, RANDTAB_ADDR,
};
use crate::tables::{RANDTAB, RANDTAB_GRAD_IDX};
use crate::trap;

// ---------------------------------------------------------------------------
// Table access.
//
// `stb_perlin_noise3_internal` can only ever produce in-range indices: every
// coordinate is masked with `& 255`, `seed` is an `unsigned char`, and each
// table lookup feeds an `unsigned char` into the next one, so no index exceeds
// 255 + 255 = 510 < 512. Those lookups use the transcribed tables directly.
//
// `stb_perlin_noise3_wrap_nonpow2` is different: `x0`/`y0`/`z0` come from
// `% x_wrap2` and are bounded only by the caller-supplied wrap values, so the
// C program happily indexes far outside the tables. Those lookups go through
// the process memory image in `memimage.rs`.
// ---------------------------------------------------------------------------

/// `stb__perlin_randtab[i]` for the provably in-range `noise3_internal` path.
#[inline]
fn randtab(i: i32) -> i32 {
    debug_assert!(i >= 0 && (i as usize) < RANDTAB.len());
    RANDTAB[i as usize] as i32
}

/// `stb__perlin_randtab_grad_idx[i]`, same caveat as [`randtab`].
#[inline]
fn randtab_grad_idx(i: i32) -> i32 {
    debug_assert!(i >= 0 && (i as usize) < RANDTAB_GRAD_IDX.len());
    RANDTAB_GRAD_IDX[i as usize] as i32
}

/// One byte of the C process' address space, or a `SIGSEGV` if the address is
/// not mapped there.
#[inline]
fn load_u8(addr: i64) -> u8 {
    if (IMAGE_BASE..IMAGE_END).contains(&addr) {
        IMAGE[(addr - IMAGE_BASE) as usize]
    } else {
        trap::sigsegv();
    }
}

/// `stb__perlin_randtab[i]` as the C compiler emits it: an `int` index, sign
/// extended and added to the array's fixed address.
#[inline]
fn randtab_raw(i: i32) -> i32 {
    load_u8(RANDTAB_ADDR + i as i64) as i32
}

/// `stb__perlin_randtab_grad_idx[i]`, unchecked, as above.
#[inline]
fn randtab_grad_idx_raw(i: i32) -> i32 {
    load_u8(GRAD_IDX_ADDR + i as i64) as i32
}

/// One `float` of `basis`, read from the C process' address space.
#[inline]
fn load_f32(addr: i64) -> f32 {
    // The C code only ever reads 4-byte aligned floats out of `basis`, so a
    // read either lies wholly inside a mapped page or wholly outside it.
    if addr >= IMAGE_BASE && addr + 4 <= IMAGE_END {
        let o = (addr - IMAGE_BASE) as usize;
        f32::from_le_bytes([IMAGE[o], IMAGE[o + 1], IMAGE[o + 2], IMAGE[o + 3]])
    } else {
        trap::sigsegv();
    }
}

// ---------------------------------------------------------------------------
// x86-64 SSE-faithful scalar `float` arithmetic.
//
// `addss`/`subss`/`mulss` return the *first* source operand when it is a NaN,
// otherwise the second one if that is a NaN, otherwise the computed result
// (which, for an invalid operation such as `0*inf` or `inf-inf`, is the x86
// "indefinite" QNaN `0xffc00000`). gcc emits these instructions with the
// left-hand C operand as the first source, so the sign of a propagated NaN is
// observable through `printf("%g")`. LLVM is free to commute the operands, so
// the order is pinned down explicitly here.
// ---------------------------------------------------------------------------

#[inline]
fn fadd(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        a
    } else if b.is_nan() {
        b
    } else {
        a + b
    }
}

#[inline]
fn fsub(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        a
    } else if b.is_nan() {
        b
    } else {
        a - b
    }
}

#[inline]
fn fmul(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        a
    } else if b.is_nan() {
        b
    } else {
        a * b
    }
}

/// C: `return a + (b-a) * t;`
///
/// gcc evaluates this as `(b-a)*t + a` (the product is the `addss`
/// destination), which decides whose NaN sign survives.
#[inline]
fn stb_perlin_lerp(a: f32, b: f32, t: f32) -> f32 {
    fadd(fmul(fsub(b, a), t), a)
}

/// C: `int ai = (int) a; return (a < ai) ? ai-1 : ai;`
///
/// The cast is emulated the way x86-64 `cvttss2si` behaves (which is what the
/// C compiler emits): NaN and out-of-range values yield `INT_MIN`.
#[inline]
fn stb_perlin_fastfloor(a: f32) -> i32 {
    let ai = f32_to_i32(a);
    if a < (ai as f32) {
        ai.wrapping_sub(1)
    } else {
        ai
    }
}

/// x86-64 `cvttss2si`-compatible float -> int truncation.
#[inline]
fn f32_to_i32(a: f32) -> i32 {
    if a.is_nan() || !(-2147483648.0f32..2147483648.0f32).contains(&a) {
        i32::MIN
    } else {
        a as i32
    }
}

/// C: `static float basis[12][4]` in `stb__perlin_grad`.
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

/// C: `grad[0]*x + grad[1]*y + grad[2]*z`
///
/// gcc keeps the running sum in `xmm1` and computes `grad[2]*z` into `xmm0`,
/// then emits `addss xmm0, xmm1` — so the *product* is the first source of the
/// final add, not the partial sum. That decides which NaN survives.
#[inline]
fn grad_dot(g0: f32, g1: f32, g2: f32, x: f32, y: f32, z: f32) -> f32 {
    let t = fadd(fmul(g0, x), fmul(g1, y));
    fadd(fmul(g2, z), t)
}

#[inline]
fn stb_perlin_grad(grad_idx: i32, x: f32, y: f32, z: f32) -> f32 {
    let grad = &BASIS[grad_idx as usize];
    grad_dot(grad[0], grad[1], grad[2], x, y, z)
}

/// `stb__perlin_grad` reached from `stb_perlin_noise3_wrap_nonpow2`, where
/// `grad_idx` is an arbitrary `unsigned char` (0..=255) rather than one of the
/// 0..=11 values the real gradient table holds, so `basis[grad_idx]` reads past
/// the 12-row array.
#[inline]
fn stb_perlin_grad_raw(grad_idx: i32, x: f32, y: f32, z: f32) -> f32 {
    let row = BASIS_ADDR + (grad_idx as i64) * 16;
    // gcc loads `grad[0]`, `grad[1]` and `grad[2]` in that order, so a fault
    // on a later element is only reached if the earlier ones are mapped.
    let g0 = load_f32(row);
    let g1 = load_f32(row + 4);
    let g2 = load_f32(row + 8);
    grad_dot(g0, g1, g2, x, y, z)
}

/// C: `#define stb__perlin_ease(a) (((a*6-15)*a + 10) * a * a * a)`
///
/// gcc materialises the literals into the first source operand of `mulss`/
/// `addss` (`mulss xmm0(=6), xmm1(=a)` and `addss xmm0(=10), xmm1`), which is
/// mirrored here even though only one operand can ever be a NaN.
#[inline]
fn stb_perlin_ease(a: f32) -> f32 {
    let t = fsub(fmul(6.0, a), 15.0);
    let t = fadd(10.0, fmul(t, a));
    fmul(fmul(fmul(t, a), a), a)
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
    let x_mask: u32 = (x_wrap.wrapping_sub(1) as u32) & 255;
    let y_mask: u32 = (y_wrap.wrapping_sub(1) as u32) & 255;
    let z_mask: u32 = (z_wrap.wrapping_sub(1) as u32) & 255;
    let px = stb_perlin_fastfloor(x);
    let py = stb_perlin_fastfloor(y);
    let pz = stb_perlin_fastfloor(z);
    let x0 = ((px as u32) & x_mask) as i32;
    let x1 = ((px.wrapping_add(1) as u32) & x_mask) as i32;
    let y0 = ((py as u32) & y_mask) as i32;
    let y1 = ((py.wrapping_add(1) as u32) & y_mask) as i32;
    let z0 = ((pz as u32) & z_mask) as i32;
    let z1 = ((pz.wrapping_add(1) as u32) & z_mask) as i32;

    x = fsub(x, px as f32);
    let u = stb_perlin_ease(x);
    y = fsub(y, py as f32);
    let v = stb_perlin_ease(y);
    z = fsub(z, pz as f32);
    let w = stb_perlin_ease(z);

    let seed = seed as i32;
    let r0 = randtab(x0 + seed);
    let r1 = randtab(x1 + seed);

    let r00 = randtab(r0 + y0);
    let r01 = randtab(r0 + y1);
    let r10 = randtab(r1 + y0);
    let r11 = randtab(r1 + y1);

    let n000 = stb_perlin_grad(randtab_grad_idx(r00 + z0), x, y, z);
    let n001 = stb_perlin_grad(randtab_grad_idx(r00 + z1), x, y, fsub(z, 1.0));
    let n010 = stb_perlin_grad(randtab_grad_idx(r01 + z0), x, fsub(y, 1.0), z);
    let n011 = stb_perlin_grad(randtab_grad_idx(r01 + z1), x, fsub(y, 1.0), fsub(z, 1.0));
    let n100 = stb_perlin_grad(randtab_grad_idx(r10 + z0), fsub(x, 1.0), y, z);
    let n101 = stb_perlin_grad(randtab_grad_idx(r10 + z1), fsub(x, 1.0), y, fsub(z, 1.0));
    let n110 = stb_perlin_grad(randtab_grad_idx(r11 + z0), fsub(x, 1.0), fsub(y, 1.0), z);
    let n111 = stb_perlin_grad(randtab_grad_idx(r11 + z1), fsub(x, 1.0), fsub(y, 1.0), fsub(z, 1.0));

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
    // C: `(unsigned char) seed`
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
            fmul(x, frequency),
            fmul(y, frequency),
            fmul(z, frequency),
            0,
            0,
            0,
            i as u8,
        );
        r = fsub(offset, fabsf(r));
        r = fmul(r, r);
        sum = fadd(fmul(fmul(r, amplitude), prev), sum);
        prev = r;
        frequency = fmul(frequency, lacunarity);
        amplitude = fmul(amplitude, gain);
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
        sum = fadd(
            fmul(
                stb_perlin_noise3_internal(
                    fmul(x, frequency),
                    fmul(y, frequency),
                    fmul(z, frequency),
                    0,
                    0,
                    0,
                    i as u8,
                ),
                amplitude,
            ),
            sum,
        );
        frequency = fmul(frequency, lacunarity);
        amplitude = fmul(amplitude, gain);
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
        let r = fmul(
            stb_perlin_noise3_internal(
                fmul(x, frequency),
                fmul(y, frequency),
                fmul(z, frequency),
                0,
                0,
                0,
                i as u8,
            ),
            amplitude,
        );
        sum = fadd(fabsf(r), sum);
        frequency = fmul(frequency, lacunarity);
        amplitude = fmul(amplitude, gain);
        i += 1;
    }
    sum
}

/// C: `(float) fabs(r)`.
///
/// gcc folds the `double` round trip away entirely and emits a single
/// `andps` against `0x7fffffff`, i.e. it just clears the sign bit. That is
/// observable for NaN, where `printf("%g")` prints `nan` rather than `-nan`.
#[inline]
fn fabsf(r: f32) -> f32 {
    f32::from_bits(r.to_bits() & 0x7fff_ffff)
}

/// C: `a % b` for `int`s, as x86-64 `idiv` performs it.
///
/// `idiv` raises `#DE` (delivered as `SIGFPE`) both for a zero divisor and for
/// the single overflowing case `INT_MIN % -1`, where the *quotient* is not
/// representable even though the remainder would be. Rust's `%` panics there
/// instead, and `wrapping_rem` quietly yields 0, so neither matches.
#[inline]
fn crem(a: i32, b: i32) -> i32 {
    if b == 0 || (a == i32::MIN && b == -1) {
        trap::sigfpe();
    }
    a % b
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
    let mut x0 = crem(px, x_wrap2);
    let mut y0 = crem(py, y_wrap2);
    let mut z0 = crem(pz, z_wrap2);

    if x0 < 0 {
        x0 = x0.wrapping_add(x_wrap2);
    }
    if y0 < 0 {
        y0 = y0.wrapping_add(y_wrap2);
    }
    if z0 < 0 {
        z0 = z0.wrapping_add(z_wrap2);
    }
    let x1 = crem(x0.wrapping_add(1), x_wrap2);
    let y1 = crem(y0.wrapping_add(1), y_wrap2);
    let z1 = crem(z0.wrapping_add(1), z_wrap2);

    x = fsub(x, px as f32);
    let u = stb_perlin_ease(x);
    y = fsub(y, py as f32);
    let v = stb_perlin_ease(y);
    z = fsub(z, pz as f32);
    let w = stb_perlin_ease(z);

    let seed = seed as i32;
    let mut r0 = randtab_raw(x0);
    r0 = randtab_raw(r0 + seed);
    let mut r1 = randtab_raw(x1);
    r1 = randtab_raw(r1 + seed);

    let r00 = randtab_raw(r0.wrapping_add(y0));
    let r01 = randtab_raw(r0.wrapping_add(y1));
    let r10 = randtab_raw(r1.wrapping_add(y0));
    let r11 = randtab_raw(r1.wrapping_add(y1));

    let n000 = stb_perlin_grad_raw(randtab_grad_idx_raw(r00.wrapping_add(z0)), x, y, z);
    let n001 = stb_perlin_grad_raw(randtab_grad_idx_raw(r00.wrapping_add(z1)), x, y, fsub(z, 1.0));
    let n010 = stb_perlin_grad_raw(randtab_grad_idx_raw(r01.wrapping_add(z0)), x, fsub(y, 1.0), z);
    let n011 = stb_perlin_grad_raw(
        randtab_grad_idx_raw(r01.wrapping_add(z1)),
        x,
        fsub(y, 1.0),
        fsub(z, 1.0),
    );
    let n100 = stb_perlin_grad_raw(randtab_grad_idx_raw(r10.wrapping_add(z0)), fsub(x, 1.0), y, z);
    let n101 = stb_perlin_grad_raw(
        randtab_grad_idx_raw(r10.wrapping_add(z1)),
        fsub(x, 1.0),
        y,
        fsub(z, 1.0),
    );
    let n110 = stb_perlin_grad_raw(
        randtab_grad_idx_raw(r11.wrapping_add(z0)),
        fsub(x, 1.0),
        fsub(y, 1.0),
        z,
    );
    let n111 = stb_perlin_grad_raw(
        randtab_grad_idx_raw(r11.wrapping_add(z1)),
        fsub(x, 1.0),
        fsub(y, 1.0),
        fsub(z, 1.0),
    );

    let n00 = stb_perlin_lerp(n000, n001, w);
    let n01 = stb_perlin_lerp(n010, n011, w);
    let n10 = stb_perlin_lerp(n100, n101, w);
    let n11 = stb_perlin_lerp(n110, n111, w);

    let n0 = stb_perlin_lerp(n00, n01, v);
    let n1 = stb_perlin_lerp(n10, n11, v);

    stb_perlin_lerp(n0, n1, u)
}
