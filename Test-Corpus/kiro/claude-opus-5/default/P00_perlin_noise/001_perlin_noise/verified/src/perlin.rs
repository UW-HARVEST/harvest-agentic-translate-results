//! Faithful Rust translation of `c_src/src/stb_perlin.h`
//! (stb_perlin.h - v0.5 - public domain, by Sean Barrett).
//!
//! Every arithmetic step is kept in `f32` exactly as the C does, and the
//! integer index arithmetic mirrors C's conversions (including the wrapping
//! behaviour of `int` overflow and x86's float->int truncation semantics).
//!
//! The operand order of every `addss`/`subss`/`mulss` below was read off the
//! assembly gcc emits for `c_src` as built by the supplied `CMakeLists.txt`
//! (i.e. no optimisation flags). It matters: when one operand is a NaN the
//! instruction returns *that* NaN, and when both are, it returns the
//! destination one — so the sign of the NaN that reaches `printf("%.9g")` is
//! decided by the order.

use crate::mem::{crem_i32, read_f32, read_u8, BASIS_ADDR, GRAD_IDX_ADDR, RANDTAB_ADDR};

/// `stb__perlin_randtab[i]`, read through the emulated data segment because the
/// C indexes it out of bounds for out-of-range `*_wrap` arguments.
#[inline]
fn randtab(i: i32) -> i32 {
    read_u8(RANDTAB_ADDR + i as i64) as i32
}

/// `stb__perlin_randtab_grad_idx[i]`, same caveat as [`randtab`].
#[inline]
fn randtab_grad_idx(i: i32) -> i32 {
    read_u8(GRAD_IDX_ADDR + i as i64) as i32
}

// ---------------------------------------------------------------------------
// x86-64 SSE-faithful scalar `float` arithmetic.
//
// `addss`/`subss`/`mulss` return the *destination* source operand when it is a
// NaN, otherwise the second one if that is a NaN, otherwise the computed result
// (which, for an invalid operation such as `0*inf` or `inf-inf`, is the x86
// "indefinite" QNaN `0xffc00000`).
// ---------------------------------------------------------------------------

/// `addss dst, src`
#[inline]
fn fadd(dst: f32, src: f32) -> f32 {
    if dst.is_nan() {
        dst
    } else if src.is_nan() {
        src
    } else {
        dst + src
    }
}

/// `subss dst, src`
#[inline]
fn fsub(dst: f32, src: f32) -> f32 {
    if dst.is_nan() {
        dst
    } else if src.is_nan() {
        src
    } else {
        dst - src
    }
}

/// `mulss dst, src`
#[inline]
fn fmul(dst: f32, src: f32) -> f32 {
    if dst.is_nan() {
        dst
    } else if src.is_nan() {
        src
    } else {
        dst * src
    }
}

/// C: `(float) fabs(r)`.
///
/// gcc expands this inline to `andps` against `0x7fffffff`, i.e. a pure bit
/// operation: it clears the sign bit of a NaN without any quieting and without
/// a detour through `double`.
#[inline]
fn fabs_bits(r: f32) -> f32 {
    f32::from_bits(r.to_bits() & 0x7fff_ffff)
}

/// C: `return a + (b-a) * t;`
///
/// gcc: `movss b; subss a; mulss t; addss a` — the product is the `addss`
/// destination.
#[inline]
fn stb_perlin_lerp(a: f32, b: f32, t: f32) -> f32 {
    fadd(fmul(fsub(b, a), t), a)
}

/// C: `int ai = (int) a; return (a < ai) ? ai-1 : ai;`
///
/// gcc: `cvttss2si`, then `comiss (float)ai, a` + `jbe`, so an unordered
/// comparison (a NaN) takes the `ai` branch.
#[inline]
fn stb_perlin_fastfloor(a: f32) -> i32 {
    let ai = f32_to_i32(a);
    if a < (ai as f32) {
        ai.wrapping_sub(1)
    } else {
        ai
    }
}

/// x86-64 `cvttss2si`-compatible float -> int truncation: NaN and out-of-range
/// values yield `INT_MIN`.
#[inline]
fn f32_to_i32(a: f32) -> i32 {
    if a.is_nan() || a >= 2147483648.0f32 || a < -2147483648.0f32 {
        i32::MIN
    } else {
        a as i32
    }
}

/// C: `float *grad = basis[grad_idx]; return grad[0]*x + grad[1]*y + grad[2]*z;`
///
/// gcc: `mulss` with the `basis` element as destination in each product, then
/// `addss (g0*x), (g1*y)` and finally `addss (g2*z), sum` — note the last add
/// has the *product* as its destination.
///
/// `grad_idx` is not range checked by the C, and the tables can hand it a value
/// far above 11 once they are themselves read out of bounds, so `basis` is read
/// through the emulated data segment too.
#[inline]
fn stb_perlin_grad(grad_idx: i32, x: f32, y: f32, z: f32) -> f32 {
    let base = BASIS_ADDR + (grad_idx as i64) * 16;
    let g0 = read_f32(base);
    let g1 = read_f32(base + 4);
    let g2 = read_f32(base + 8);
    let s = fadd(fmul(g0, x), fmul(g1, y));
    fadd(fmul(g2, z), s)
}

/// C: `#define stb__perlin_ease(a) (((a*6-15)*a + 10) * a * a * a)`
///
/// gcc: `mulss 6.0f, a` / `subss ., 15.0f` / `mulss ., a` / `addss 10.0f, .`
/// / `mulss ., a` / `mulss ., a` / `mulss ., a`.
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
    let r0 = randtab(x0.wrapping_add(seed));
    let r1 = randtab(x1.wrapping_add(seed));

    let r00 = randtab(r0.wrapping_add(y0));
    let r01 = randtab(r0.wrapping_add(y1));
    let r10 = randtab(r1.wrapping_add(y0));
    let r11 = randtab(r1.wrapping_add(y1));

    let n000 = stb_perlin_grad(randtab_grad_idx(r00.wrapping_add(z0)), x, y, z);
    let n001 = stb_perlin_grad(randtab_grad_idx(r00.wrapping_add(z1)), x, y, fsub(z, 1.0));
    let n010 = stb_perlin_grad(randtab_grad_idx(r01.wrapping_add(z0)), x, fsub(y, 1.0), z);
    let n011 = stb_perlin_grad(
        randtab_grad_idx(r01.wrapping_add(z1)),
        x,
        fsub(y, 1.0),
        fsub(z, 1.0),
    );
    let n100 = stb_perlin_grad(randtab_grad_idx(r10.wrapping_add(z0)), fsub(x, 1.0), y, z);
    let n101 = stb_perlin_grad(
        randtab_grad_idx(r10.wrapping_add(z1)),
        fsub(x, 1.0),
        y,
        fsub(z, 1.0),
    );
    let n110 = stb_perlin_grad(
        randtab_grad_idx(r11.wrapping_add(z0)),
        fsub(x, 1.0),
        fsub(y, 1.0),
        z,
    );
    let n111 = stb_perlin_grad(
        randtab_grad_idx(r11.wrapping_add(z1)),
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
        r = fsub(offset, fabs_bits(r));
        r = fmul(r, r);
        // gcc: `mulss amplitude; mulss prev; addss sum` with the product as the
        // `addss` destination.
        sum = fadd(fmul(fmul(r, amplitude), prev), sum);
        prev = r;
        frequency = fmul(frequency, lacunarity);
        amplitude = fmul(amplitude, gain);
        i = i.wrapping_add(1);
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
            fmul(x, frequency),
            fmul(y, frequency),
            fmul(z, frequency),
            0,
            0,
            0,
            i as u8,
        );
        sum = fadd(fmul(n, amplitude), sum);
        frequency = fmul(frequency, lacunarity);
        amplitude = fmul(amplitude, gain);
        i = i.wrapping_add(1);
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
        sum = fadd(fabs_bits(r), sum);
        frequency = fmul(frequency, lacunarity);
        amplitude = fmul(amplitude, gain);
        i = i.wrapping_add(1);
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
    // `idivl`, in source order: `INT_MIN % -1` traps with SIGFPE.
    let mut x0 = crem_i32(px, x_wrap2);
    let mut y0 = crem_i32(py, y_wrap2);
    let mut z0 = crem_i32(pz, z_wrap2);

    if x0 < 0 {
        x0 = x0.wrapping_add(x_wrap2);
    }
    if y0 < 0 {
        y0 = y0.wrapping_add(y_wrap2);
    }
    if z0 < 0 {
        z0 = z0.wrapping_add(z_wrap2);
    }
    let x1 = crem_i32(x0.wrapping_add(1), x_wrap2);
    let y1 = crem_i32(y0.wrapping_add(1), y_wrap2);
    let z1 = crem_i32(z0.wrapping_add(1), z_wrap2);

    x = fsub(x, px as f32);
    let u = stb_perlin_ease(x);
    y = fsub(y, py as f32);
    let v = stb_perlin_ease(y);
    z = fsub(z, pz as f32);
    let w = stb_perlin_ease(z);

    let seed = seed as i32;
    let mut r0 = randtab(x0);
    r0 = randtab(r0.wrapping_add(seed));
    let mut r1 = randtab(x1);
    r1 = randtab(r1.wrapping_add(seed));

    let r00 = randtab(r0.wrapping_add(y0));
    let r01 = randtab(r0.wrapping_add(y1));
    let r10 = randtab(r1.wrapping_add(y0));
    let r11 = randtab(r1.wrapping_add(y1));

    let n000 = stb_perlin_grad(randtab_grad_idx(r00.wrapping_add(z0)), x, y, z);
    let n001 = stb_perlin_grad(randtab_grad_idx(r00.wrapping_add(z1)), x, y, fsub(z, 1.0));
    let n010 = stb_perlin_grad(randtab_grad_idx(r01.wrapping_add(z0)), x, fsub(y, 1.0), z);
    let n011 = stb_perlin_grad(
        randtab_grad_idx(r01.wrapping_add(z1)),
        x,
        fsub(y, 1.0),
        fsub(z, 1.0),
    );
    let n100 = stb_perlin_grad(randtab_grad_idx(r10.wrapping_add(z0)), fsub(x, 1.0), y, z);
    let n101 = stb_perlin_grad(
        randtab_grad_idx(r10.wrapping_add(z1)),
        fsub(x, 1.0),
        y,
        fsub(z, 1.0),
    );
    let n110 = stb_perlin_grad(
        randtab_grad_idx(r11.wrapping_add(z0)),
        fsub(x, 1.0),
        fsub(y, 1.0),
        z,
    );
    let n111 = stb_perlin_grad(
        randtab_grad_idx(r11.wrapping_add(z1)),
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
