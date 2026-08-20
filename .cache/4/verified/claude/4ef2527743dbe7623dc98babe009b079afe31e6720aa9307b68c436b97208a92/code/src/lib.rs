//! Rust translation of the C library in `c_src/`.
//!
//! This crate reproduces the complete public ABI of the shared library built
//! from `c_src/src/lib.c` (see `c_src/CMakeLists.txt`), and is intended to be
//! byte-for-byte behaviourally identical to the original C code — including any
//! bugs or questionable constructs present in the original.
//!
//! Original C library: Maratis Tiny C library, Copyright (c) 2015 Anael Seghezzi.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

mod tables;

use core::ffi::{c_int, c_void};

use tables::{M_EXPONENT, M_MANTISSA, M_OFFSET};

// ---------------------------------------------------------------------------
// Helpers reproducing exact C semantics
// ---------------------------------------------------------------------------

/// `INT_MIN` as spelled in the C source (`-0x7fffffff - 1`).
const C_INT_MIN: c_int = -0x7fff_ffff - 1;

/// Reproduces a C `(int)` cast from `float` on x86-64 (SSE `cvttss2si`).
///
/// The C standard leaves out-of-range / NaN conversions undefined; on x86-64
/// the hardware yields the "integer indefinite" value `0x80000000`.
#[inline]
fn c_float_to_int(v: f32) -> c_int {
    if v.is_nan() || v >= 2147483648.0f32 || v < -2147483648.0f32 {
        C_INT_MIN
    } else {
        // In-range: `as` performs the same truncation-toward-zero as C.
        v as c_int
    }
}

/// `fabsf`
#[inline]
fn fabsf(x: f32) -> f32 {
    // f32::abs() is a bit-twiddle on the sign bit, exactly like fabsf().
    f32::from_bits(x.to_bits() & 0x7fff_ffff)
}

/// `fmodf`
#[inline]
fn fmodf(x: f32, y: f32) -> f32 {
    // Rust's `%` on floats lowers to `frem`, i.e. fmodf semantics.
    x % y
}

/// `floorf`
#[inline]
fn floorf(x: f32) -> f32 {
    x.floor()
}

// --- Single-precision arithmetic with x86 SSE NaN-propagation semantics ------
//
// C compilers are free to reassociate/commute `a - b*c` style expressions,
// which changes *which* NaN operand is propagated (and hence the resulting NaN
// sign/payload).  LLVM performs such canonicalisations aggressively, so plain
// Rust `a - b * c` can end up propagating the other NaN than the C build does.
//
// These helpers pin the operand order to the order written in the C source and
// implement the x86 `addss`/`subss`/`mulss`/`divss` rule:
//   * if the destination operand is a NaN, the result is that NaN, quieted;
//   * else if the source operand is a NaN, the result is that NaN, quieted;
//   * otherwise the ordinary IEEE-754 single-precision result.
// For non-NaN operands they are plain hardware ops, so finite/infinite results
// (including the invalid-operation default NaN) are bit-identical either way.

/// Quiet a NaN the way x86 does (set the most significant mantissa bit).
#[inline(always)]
fn quiet_nan(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

#[inline(always)]
fn ss_add(dst: f32, src: f32) -> f32 {
    if dst.is_nan() {
        quiet_nan(dst)
    } else if src.is_nan() {
        quiet_nan(src)
    } else {
        dst + src
    }
}

#[inline(always)]
fn ss_sub(dst: f32, src: f32) -> f32 {
    if dst.is_nan() {
        quiet_nan(dst)
    } else if src.is_nan() {
        quiet_nan(src)
    } else {
        dst - src
    }
}

#[inline(always)]
fn ss_mul(dst: f32, src: f32) -> f32 {
    if dst.is_nan() {
        quiet_nan(dst)
    } else if src.is_nan() {
        quiet_nan(src)
    } else {
        dst * src
    }
}

#[inline(always)]
fn ss_div(dst: f32, src: f32) -> f32 {
    if dst.is_nan() {
        quiet_nan(dst)
    } else if src.is_nan() {
        quiet_nan(src)
    } else {
        dst / src
    }
}

// ---------------------------------------------------------------------------
// C2 types
// ---------------------------------------------------------------------------

/// `typedef enum { C2_TYPE_CIRCLE, C2_TYPE_AABB } C2_TYPE;`
pub const C2_TYPE_CIRCLE: c_int = 0;
/// `typedef enum { C2_TYPE_CIRCLE, C2_TYPE_AABB } C2_TYPE;`
pub const C2_TYPE_AABB: c_int = 1;

/// `typedef struct c2v { float x; float y; } c2v;`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

/// `typedef struct c2Circle { c2v p; float r; } c2Circle;`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

/// `typedef struct c2AABB { c2v min; c2v max; } c2AABB;`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

// ---------------------------------------------------------------------------
// c2 vector helpers (public in C, hence exported)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    let mut a = c2v { x: 0.0, y: 0.0 };
    a.x = x;
    a.y = y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv(lo, c2Minv(a, hi))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(a: c2v, b: c2v) -> c2v {
    let mut a = a;
    a.x = ss_sub(a.x, b.x);
    a.y = ss_sub(a.y, b.y);
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    // `a.x * b.x + a.y * b.y`.  gcc evaluates the second product first into the
    // accumulator, so the emitted code is `mulss` with dst = b.y and the final
    // `addss` with dst = the y term.  Reproduced here so that NaN propagation
    // (which operand's payload survives) is identical.
    let xterm = ss_mul(a.x, b.x);
    let yterm = ss_mul(b.y, a.y);
    ss_add(yterm, xterm)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircle(A: c2Circle, B: c2Circle) -> c_int {
    let c = c2Sub(B.p, A.p);
    let d2 = c2Dot(c, c);
    // gcc emits `addss` with dst = B.r and src = A.r for `A.r + B.r`, so when
    // BOTH radii are NaN it is B.r's payload that survives.
    let mut r2 = ss_add(B.r, A.r);
    r2 = ss_mul(r2, r2);
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(A: c2Circle, B: c2AABB) -> c_int {
    let L = c2Clampv(A.p, B.min, B.max);
    let ab = c2Sub(A.p, L);
    let d2 = c2Dot(ab, ab);
    let r2 = ss_mul(A.r, A.r);
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> c_int {
    let d0 = (B.max.x < A.min.x) as c_int;
    let d1 = (A.max.x < B.min.x) as c_int;
    let d2 = (B.max.y < A.min.y) as c_int;
    let d3 = (A.max.y < B.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

// ---------------------------------------------------------------------------
// f2 — collision dispatch
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn f2(
    A: *const c_void,
    typeA: c_int,
    B: *const c_void,
    typeB: c_int,
) -> c_int {
    unsafe {
        match typeA {
            C2_TYPE_CIRCLE => match typeB {
                C2_TYPE_CIRCLE => c2CircletoCircle(
                    core::ptr::read_unaligned(A as *const c2Circle),
                    core::ptr::read_unaligned(B as *const c2Circle),
                ),
                C2_TYPE_AABB => c2CircletoAABB(
                    core::ptr::read_unaligned(A as *const c2Circle),
                    core::ptr::read_unaligned(B as *const c2AABB),
                ),
                _ => 0,
            },
            C2_TYPE_AABB => match typeB {
                C2_TYPE_CIRCLE => c2CircletoAABB(
                    core::ptr::read_unaligned(B as *const c2Circle),
                    core::ptr::read_unaligned(A as *const c2AABB),
                ),
                C2_TYPE_AABB => c2AABBtoAABB(
                    core::ptr::read_unaligned(A as *const c2AABB),
                    core::ptr::read_unaligned(B as *const c2AABB),
                ),
                _ => 0,
            },
            _ => 0,
        }
    }
}

// ---------------------------------------------------------------------------
// f3 — floor division
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn f3(v1: c_int, v2: c_int) -> c_int {
    if v2 == 0 {
        return 0;
    }
    let q: c_int;
    let r: c_int;
    if v1 >= 0 {
        if v2 >= 0 {
            return v1.wrapping_div(v2);
        } else if v2 != C_INT_MIN {
            q = v1.wrapping_div(v2.wrapping_neg()).wrapping_neg();
            r = v1.wrapping_rem(v2.wrapping_neg());
        } else {
            q = 0;
            r = v1;
        }
    } else if v1 != C_INT_MIN {
        if v2 >= 0 {
            q = v1.wrapping_neg().wrapping_div(v2).wrapping_neg();
            r = v1.wrapping_neg().wrapping_rem(v2).wrapping_neg();
        } else if v2 != C_INT_MIN {
            q = v1.wrapping_neg().wrapping_div(v2.wrapping_neg());
            r = v1
                .wrapping_neg()
                .wrapping_rem(v2.wrapping_neg())
                .wrapping_neg();
        } else {
            q = 1;
            r = v1.wrapping_sub(q.wrapping_mul(v2));
        }
    } else if v2 >= 0 {
        q = v1
            .wrapping_add(v2)
            .wrapping_neg()
            .wrapping_div(v2)
            .wrapping_neg()
            .wrapping_sub(1);
        r = v1
            .wrapping_add(v2)
            .wrapping_neg()
            .wrapping_rem(v2)
            .wrapping_neg();
    } else if v2 != C_INT_MIN {
        q = v1
            .wrapping_sub(v2)
            .wrapping_neg()
            .wrapping_div(v2.wrapping_neg())
            .wrapping_add(1);
        r = v1
            .wrapping_sub(v2)
            .wrapping_neg()
            .wrapping_rem(v2.wrapping_neg())
            .wrapping_neg();
    } else {
        q = 1;
        r = 0;
    }
    if r >= 0 {
        q
    } else {
        q.wrapping_add(if v2 > 0 { -1 } else { 1 })
    }
}

// ---------------------------------------------------------------------------
// f4 — xorshift128+ derived double in [0, 1)
// ---------------------------------------------------------------------------

/// `typedef struct cn_rnd_t { uint64_t state[2]; } cn_rnd_t;`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct cn_rnd_t {
    pub state: [u64; 2],
}

fn cn_rnd_next(rnd: &mut cn_rnd_t) -> u64 {
    let mut x = rnd.state[0];
    let y = rnd.state[1];
    rnd.state[0] = y;
    x ^= x << 23;
    x ^= x >> 17;
    x ^= y ^ (y >> 26);
    rnd.state[1] = x;
    x.wrapping_add(y)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn f4(rnd: *mut cn_rnd_t) -> f64 {
    let value = cn_rnd_next(unsafe { &mut *rnd });
    let exponent: u64 = 1023;
    let mantissa: u64 = value >> 12;
    let result: u64 = (exponent << 52) | mantissa;
    f64::from_bits(result) - 1.0
}

// ---------------------------------------------------------------------------
// f5 — 16-bit reverse
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn f5(a: u32) -> u32 {
    let mut a = a;
    a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    a = ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8);
    a
}

// ---------------------------------------------------------------------------
// tflac
// ---------------------------------------------------------------------------

/// `typedef uint8_t tflac_u8;`
pub type tflac_u8 = u8;
/// `typedef uint16_t tflac_u16;`
pub type tflac_u16 = u16;
/// `typedef uint32_t tflac_u32;`
pub type tflac_u32 = u32;

#[unsafe(no_mangle)]
pub extern "C" fn f7(blocksize: tflac_u32, channels: tflac_u32, bitdepth: tflac_u32) -> tflac_u32 {
    let ne2 = (channels != 2) as tflac_u32;
    let eq2 = (channels == 2) as tflac_u32;
    let bd_ne32 = (bitdepth != 32) as tflac_u32;

    let t1 = blocksize
        .wrapping_mul(bitdepth)
        .wrapping_mul(channels.wrapping_mul(ne2));
    let t2 = blocksize.wrapping_mul(bitdepth).wrapping_mul(eq2);
    let t3 = blocksize
        .wrapping_mul(bitdepth.wrapping_add(bd_ne32))
        .wrapping_mul(eq2);

    18u32
        .wrapping_add(channels)
        .wrapping_add(
            t1.wrapping_add(t2)
                .wrapping_add(t3)
                .wrapping_add(7)
                / 8,
        )
}

// ---------------------------------------------------------------------------
// lightmapper — barycentric coordinates
// ---------------------------------------------------------------------------

/// `typedef struct lm_vec2 { float x, y; } lm_vec2;`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct lm_vec2 {
    pub x: f32,
    pub y: f32,
}

fn lm_v2(x: f32, y: f32) -> lm_vec2 {
    lm_vec2 { x, y }
}

fn lm_sub2(a: lm_vec2, b: lm_vec2) -> lm_vec2 {
    lm_v2(ss_sub(a.x, b.x), ss_sub(a.y, b.y))
}

fn lm_dot2(a: lm_vec2, b: lm_vec2) -> f32 {
    // Same operand ordering as `c2Dot` (identical C expression, identical
    // gcc codegen).
    let xterm = ss_mul(a.x, b.x);
    let yterm = ss_mul(b.y, a.y);
    ss_add(yterm, xterm)
}

#[unsafe(no_mangle)]
pub extern "C" fn f9(p1: lm_vec2, p2: lm_vec2, p3: lm_vec2, p: lm_vec2) -> lm_vec2 {
    let v0 = lm_sub2(p3, p1);
    let v1 = lm_sub2(p2, p1);
    let v2 = lm_sub2(p, p1);
    let dot00 = lm_dot2(v0, v0);
    let dot01 = lm_dot2(v0, v1);
    let dot02 = lm_dot2(v0, v2);
    let dot11 = lm_dot2(v1, v1);
    let dot12 = lm_dot2(v1, v2);
    let invDenom = ss_div(
        1.0f32,
        ss_sub(ss_mul(dot00, dot11), ss_mul(dot01, dot01)),
    );
    let u = ss_mul(ss_sub(ss_mul(dot11, dot02), ss_mul(dot01, dot12)), invDenom);
    let v = ss_mul(ss_sub(ss_mul(dot00, dot12), ss_mul(dot01, dot02)), invDenom);
    lm_v2(u, v)
}

// ---------------------------------------------------------------------------
// f10 — half-float to float
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn f10(h: u16) -> f32 {
    let n = (h >> 10) as usize;
    let num = M_MANTISSA[((h & 0x3ff) as usize) + (M_OFFSET[n] as usize)]
        .wrapping_add(M_EXPONENT[n]);
    f32::from_bits(num)
}

// ---------------------------------------------------------------------------
// f11 — HSL to RGB
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn f11(dest: *mut f32, src: *const f32) {
    unsafe {
        let h = *src.add(0);
        let s = *src.add(1);
        let l = *src.add(2);
        let c: f32;
        let m: f32;
        let x: f32;
        if s == 0.0 {
            *dest.add(0) = l;
            *dest.add(1) = l;
            *dest.add(2) = l;
            return;
        }
        c = ss_mul(
            ss_sub(1.0f32, fabsf(ss_sub(ss_mul(2.0f32, l), 1.0f32))),
            s,
        );
        m = ss_mul(1.0f32, ss_sub(l, ss_mul(0.5f32, c)));
        // NOTE: gcc (at both -O0 and -O2) evaluates the parenthesised
        // sub-expression into the destination register, i.e. it emits
        // `mulss` with dst = (1.0f - fabsf(...)) and src = c.  Matching that
        // order keeps NaN propagation identical to the C build.
        x = ss_mul(
            ss_sub(
                1.0f32,
                fabsf(ss_sub(fmodf(ss_div(h, 60.0f32), 2.0f32), 1.0f32)),
            ),
            c,
        );
        if h >= 0.0f32 && h < 60.0f32 {
            *dest.add(0) = ss_add(c, m);
            *dest.add(1) = ss_add(x, m);
            *dest.add(2) = m;
        } else if h >= 60.0f32 && h < 120.0f32 {
            *dest.add(0) = ss_add(x, m);
            *dest.add(1) = ss_add(c, m);
            *dest.add(2) = m;
        } else if h < 120.0f32 && h < 180.0f32 {
            // NOTE: reproduces the original C code verbatim (`h < 120.0f`
            // instead of `h >= 120.0f`).
            *dest.add(0) = m;
            *dest.add(1) = ss_add(c, m);
            *dest.add(2) = ss_add(x, m);
        } else if h >= 180.0f32 && h < 240.0f32 {
            *dest.add(0) = m;
            *dest.add(1) = ss_add(x, m);
            *dest.add(2) = ss_add(c, m);
        } else if h >= 240.0f32 && h < 300.0f32 {
            *dest.add(0) = ss_add(x, m);
            *dest.add(1) = m;
            *dest.add(2) = ss_add(c, m);
        } else if h >= 300.0f32 && h < 360.0f32 {
            *dest.add(0) = ss_add(c, m);
            *dest.add(1) = m;
            *dest.add(2) = ss_add(x, m);
        } else {
            *dest.add(0) = m;
            *dest.add(1) = m;
            *dest.add(2) = m;
        }
    }
}

// ---------------------------------------------------------------------------
// f12 — HSV to RGB
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn f12(dest: *mut f32, src: *const f32) {
    unsafe {
        let r: f32;
        let g: f32;
        let b: f32;
        let f: f32;
        let p: f32;
        let q: f32;
        let t: f32;
        let mut h = *src.add(0);
        let s = *src.add(1);
        let v = *src.add(2);
        let i: c_int;
        if s == 0.0 {
            *dest.add(0) = v;
            *dest.add(1) = v;
            *dest.add(2) = v;
            return;
        }
        h = ss_div(h, 60.0f32);
        i = c_float_to_int(floorf(h));
        f = ss_sub(h, i as f32);
        // As in f11, gcc emits `mulss` with the parenthesised sub-expression
        // as the destination operand.
        p = ss_mul(ss_sub(1.0f32, s), v);
        q = ss_mul(ss_sub(1.0f32, ss_mul(s, f)), v);
        // gcc computes `s * (1 - f)` with dst = (1 - f) and src = s, so when both
        // `s` and `(1 - f)` are NaN it is (1 - f)'s payload that survives.
        t = ss_mul(ss_sub(1.0f32, ss_mul(ss_sub(1.0f32, f), s)), v);
        match i {
            0 => {
                r = v;
                g = t;
                b = p;
            }
            1 => {
                r = q;
                g = v;
                b = p;
            }
            2 => {
                r = p;
                g = v;
                b = t;
            }
            3 => {
                r = p;
                g = q;
                b = v;
            }
            4 => {
                r = t;
                g = p;
                b = v;
            }
            _ => {
                r = v;
                g = p;
                b = q;
            }
        }
        *dest.add(0) = r;
        *dest.add(1) = g;
        *dest.add(2) = b;
    }
}

// ---------------------------------------------------------------------------
// f13 — RGB to HSV
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn f13(dest: *mut f32, src: *const f32) {
    unsafe {
        let r = *src.add(0);
        let g = *src.add(1);
        let b = *src.add(2);
        let mut h: f32 = 0.0;
        let mut s: f32 = 0.0;
        let v: f32;
        let mut min = r;
        let mut max = r;
        let delta: f32;
        min = if min < g { min } else { g };
        min = if min < b { min } else { b };
        max = if max > g { max } else { g };
        max = if max > b { max } else { b };
        delta = ss_sub(max, min);
        v = max;
        if delta == 0.0 || max == 0.0 {
            *dest.add(0) = h;
            *dest.add(1) = s;
            *dest.add(2) = v;
            return;
        }
        s = ss_div(delta, max);
        if r == max {
            h = ss_div(ss_sub(g, b), delta);
        } else if g == max {
            h = ss_add(2.0f32, ss_div(ss_sub(b, r), delta));
        } else {
            h = ss_add(4.0f32, ss_div(ss_sub(r, g), delta));
        }
        h = ss_mul(h, 60.0f32);
        if h < 0.0 {
            h = ss_add(h, 360.0f32);
        }
        *dest.add(0) = h;
        *dest.add(1) = s;
        *dest.add(2) = v;
    }
}

// ---------------------------------------------------------------------------
// agglom — the aggregate entry point declared in include/lib.h
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn agglom(
    f2_1: f32,
    f2_2: f32,
    f2_3: f32,
    f2_7: f32,
    f2_8: f32,
    f2_9: f32,
    f2_10: f32,
    f3_1: c_int,
    f3_2: c_int,
    f4_1: u64,
    f4_2: u64,
    f5_1: u32,
    f7_1: tflac_u32,
    f7_2: tflac_u32,
    f7_3: tflac_u32,
    f9_1: f32,
    f9_2: f32,
    f9_4: f32,
    f9_5: f32,
    f9_7: f32,
    f9_8: f32,
    f9_10: f32,
    f9_11: f32,
    f10_1: u16,
    f11_2: f32,
    f11_3: f32,
    f11_4: f32,
    f12_2: f32,
    f12_3: f32,
    f12_4: f32,
    f13_2: f32,
    f13_3: f32,
    f13_4: f32,
) -> f64 {
    let mut ret: f64 = 0.0;

    let f2_5 = c2Circle {
        p: c2v { x: f2_1, y: f2_2 },
        r: f2_3,
    };
    let f2_6: c_int = C2_TYPE_CIRCLE;

    let f2_11 = c2AABB {
        min: c2v { x: f2_7, y: f2_8 },
        max: c2v { x: f2_9, y: f2_10 },
    };
    let f2_12: c_int = C2_TYPE_AABB;

    let f2_r = unsafe {
        f2(
            &f2_5 as *const c2Circle as *const c_void,
            f2_6,
            &f2_11 as *const c2AABB as *const c_void,
            f2_12,
        )
    };
    ret += f2_r as f64;

    let f3_r = f3(f3_1, f3_2);
    ret += f3_r as f64;

    let mut f4_3 = cn_rnd_t {
        state: [f4_1, f4_2],
    };
    let f4_r = unsafe { f4(&mut f4_3 as *mut cn_rnd_t) };
    if !f4_r.is_nan() {
        ret += f4_r;
    }

    let f5_r = f5(f5_1);
    ret += f5_r as f64;

    let f7_r = f7(f7_1, f7_2, f7_3);
    ret += f7_r as f64;

    let f9_3 = lm_vec2 { x: f9_1, y: f9_2 };
    let f9_6 = lm_vec2 { x: f9_4, y: f9_5 };
    let f9_9 = lm_vec2 { x: f9_7, y: f9_8 };
    let f9_12 = lm_vec2 { x: f9_10, y: f9_11 };

    let f9_r = f9(f9_3, f9_6, f9_9, f9_12);
    if !f9_r.x.is_nan() {
        ret += f9_r.x as f64;
    }
    if !f9_r.y.is_nan() {
        ret += f9_r.y as f64;
    }

    let f10_r = f10(f10_1);
    if !f10_r.is_nan() {
        ret += f10_r as f64;
    }

    let mut f11_r: [f32; 3] = [0.0, 0.0, 0.0];
    let f11_5: [f32; 3] = [f11_2, f11_3, f11_4];
    unsafe { f11(f11_r.as_mut_ptr(), f11_5.as_ptr()) };
    if !f11_r[0].is_nan() {
        ret += f11_r[0] as f64;
    }
    if !f11_r[1].is_nan() {
        ret += f11_r[1] as f64;
    }
    if !f11_r[2].is_nan() {
        ret += f11_r[2] as f64;
    }

    let mut f12_r: [f32; 3] = [0.0, 0.0, 0.0];
    let f12_5: [f32; 3] = [f12_2, f12_3, f12_4];
    unsafe { f12(f12_r.as_mut_ptr(), f12_5.as_ptr()) };
    if !f12_r[0].is_nan() {
        ret += f12_r[0] as f64;
    }
    if !f12_r[1].is_nan() {
        ret += f12_r[1] as f64;
    }
    if !f12_r[2].is_nan() {
        ret += f12_r[2] as f64;
    }

    let mut f13_r: [f32; 3] = [0.0, 0.0, 0.0];
    let f13_5: [f32; 3] = [f13_2, f13_3, f13_4];
    unsafe { f13(f13_r.as_mut_ptr(), f13_5.as_ptr()) };
    if !f13_r[0].is_nan() {
        ret += f13_r[0] as f64;
    }
    if !f13_r[1].is_nan() {
        ret += f13_r[1] as f64;
    }
    if !f13_r[2].is_nan() {
        ret += f13_r[2] as f64;
    }

    ret
}
