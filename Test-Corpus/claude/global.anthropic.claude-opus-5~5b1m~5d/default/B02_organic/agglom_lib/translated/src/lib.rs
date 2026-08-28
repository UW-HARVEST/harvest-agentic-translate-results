//! Rust translation of the C library in `c_src/`.
//!
//! Maratis Tiny C library — Copyright (c) 2015 Anael Seghezzi.
//! See `c_src/license.txt`.
//!
//! This crate is a faithful, ABI-compatible translation of `c_src/src/lib.c`.
//! Every symbol exported by the C shared object is re-exported here with the
//! identical linker name, calling convention and signature, and every function
//! reproduces the original semantics bit-for-bit — including the original
//! quirks and bugs (see `f11`, `f7`, `f3`).
//!
//! Float arithmetic is expressed through the `addss`/`subss`/`mulss`/`divss`
//! helpers below so that NaN payload propagation matches the scalar SSE
//! instructions the C compiler emits (LLVM is free to commute and re-associate
//! `fadd`/`fmul`, which would otherwise pick a different NaN operand).

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use core::ffi::{c_int, c_uint, c_void};

mod tables;

use tables::{m__exponent, m__mantissa, m__offset};

// ---------------------------------------------------------------------------
// Scalar SSE arithmetic helpers
// ---------------------------------------------------------------------------
//
// An `xxxss dst, src` instruction returns, when an operand is NaN, the *quieted
// destination* operand if that one is NaN, otherwise the quieted source
// operand. The `dst`/`src` argument order of these helpers therefore mirrors
// the instruction the C compiler emits for the corresponding expression.

/// Set the "quiet" mantissa bit, as SSE does when it propagates a NaN.
#[inline]
fn quiet(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

#[inline]
fn nan_operand(dst: f32, src: f32) -> Option<f32> {
    if dst.is_nan() {
        Some(quiet(dst))
    } else if src.is_nan() {
        Some(quiet(src))
    } else {
        None
    }
}

/// `addss dst, src` → `dst + src`
#[inline]
fn addss(dst: f32, src: f32) -> f32 {
    match nan_operand(dst, src) {
        Some(v) => v,
        None => dst + src,
    }
}

/// `subss dst, src` → `dst - src`
#[inline]
fn subss(dst: f32, src: f32) -> f32 {
    match nan_operand(dst, src) {
        Some(v) => v,
        None => dst - src,
    }
}

/// `mulss dst, src` → `dst * src`
#[inline]
fn mulss(dst: f32, src: f32) -> f32 {
    match nan_operand(dst, src) {
        Some(v) => v,
        None => dst * src,
    }
}

/// `divss dst, src` → `dst / src`
#[inline]
fn divss(dst: f32, src: f32) -> f32 {
    match nan_operand(dst, src) {
        Some(v) => v,
        None => dst / src,
    }
}

// ---------------------------------------------------------------------------
// Other helpers reproducing C semantics exactly
// ---------------------------------------------------------------------------

/// `(int)f` as emitted by GCC/Clang on x86-64 (`cvttss2si`).
///
/// C leaves the conversion undefined when the truncated value is not
/// representable; the hardware yields the "integer indefinite" value
/// `0x80000000`. NaN maps there as well.
#[inline]
fn c_cast_f32_to_i32(f: f32) -> i32 {
    if f.is_nan() || f >= 2147483648.0f32 || f < -2147483648.0f32 {
        i32::MIN
    } else {
        f as i32
    }
}

/// C's `fmodf` (`x % y` lowers to a `fmodf` libm call).
#[inline]
fn fmodf(x: f32, y: f32) -> f32 {
    x % y
}

/// C's `fabsf` — `andps` against `0x7fffffff`.
#[inline]
fn fabsf(x: f32) -> f32 {
    f32::from_bits(x.to_bits() & 0x7fff_ffff)
}

/// C's `floorf` — `roundss $9`.
#[inline]
fn floorf(x: f32) -> f32 {
    x.floor()
}

/// C's `cond ? 1 : 0` as an `int`.
#[inline]
fn cbool(b: bool) -> c_int {
    b as c_int
}

/// C's `cond ? 1 : 0` as an `unsigned int`.
#[inline]
fn ubool(b: bool) -> c_uint {
    b as c_uint
}

// ---------------------------------------------------------------------------
// cute_c2 style 2D collision primitives
// ---------------------------------------------------------------------------

pub const C2_TYPE_CIRCLE: c_uint = 0;
pub const C2_TYPE_AABB: c_uint = 1;

/// `typedef enum { ... } C2_TYPE;` — GCC lays this out as `unsigned int`.
#[allow(dead_code)]
pub type C2_TYPE = c_uint;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[inline]
fn c2V_impl(x: f32, y: f32) -> c2v {
    let mut a = c2v { x: 0.0, y: 0.0 };
    a.x = x;
    a.y = y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    c2V_impl(x, y)
}

#[inline]
fn c2Maxv_impl(a: c2v, b: c2v) -> c2v {
    c2V_impl(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2Maxv_impl(a, b)
}

#[inline]
fn c2Minv_impl(a: c2v, b: c2v) -> c2v {
    c2V_impl(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2Minv_impl(a, b)
}

#[inline]
fn c2Clampv_impl(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv_impl(lo, c2Minv_impl(a, hi))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Clampv_impl(a, lo, hi)
}

#[inline]
fn c2Sub_impl(a: c2v, b: c2v) -> c2v {
    let mut a = a;
    a.x = subss(a.x, b.x);
    a.y = subss(a.y, b.y);
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(a: c2v, b: c2v) -> c2v {
    c2Sub_impl(a, b)
}

#[inline]
fn c2Dot_impl(a: c2v, b: c2v) -> f32 {
    addss(mulss(a.x, b.x), mulss(a.y, b.y))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    c2Dot_impl(a, b)
}

#[inline]
fn c2CircletoCircle_impl(A: c2Circle, B: c2Circle) -> c_int {
    let c = c2Sub_impl(B.p, A.p);
    let d2 = c2Dot_impl(c, c);
    let mut r2 = addss(A.r, B.r);
    r2 = mulss(r2, r2);
    cbool(d2 < r2)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircle(A: c2Circle, B: c2Circle) -> c_int {
    c2CircletoCircle_impl(A, B)
}

#[inline]
fn c2CircletoAABB_impl(A: c2Circle, B: c2AABB) -> c_int {
    let L = c2Clampv_impl(A.p, B.min, B.max);
    let ab = c2Sub_impl(A.p, L);
    let d2 = c2Dot_impl(ab, ab);
    let r2 = mulss(A.r, A.r);
    cbool(d2 < r2)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(A: c2Circle, B: c2AABB) -> c_int {
    c2CircletoAABB_impl(A, B)
}

#[inline]
fn c2AABBtoAABB_impl(A: c2AABB, B: c2AABB) -> c_int {
    let d0 = cbool(B.max.x < A.min.x);
    let d1 = cbool(A.max.x < B.min.x);
    let d2 = cbool(B.max.y < A.min.y);
    let d3 = cbool(A.max.y < B.min.y);
    cbool((d0 | d1 | d2 | d3) == 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> c_int {
    c2AABBtoAABB_impl(A, B)
}

/// Dispatching collision test over type-erased shape pointers.
///
/// # Safety
/// `A` and `B` must point to a `c2Circle`/`c2AABB` matching `typeA`/`typeB`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn f2(
    A: *const c_void,
    typeA: c_uint,
    B: *const c_void,
    typeB: c_uint,
) -> c_int {
    match typeA {
        C2_TYPE_CIRCLE => match typeB {
            C2_TYPE_CIRCLE => {
                c2CircletoCircle_impl(*(A as *const c2Circle), *(B as *const c2Circle))
            }
            C2_TYPE_AABB => c2CircletoAABB_impl(*(A as *const c2Circle), *(B as *const c2AABB)),
            _ => 0,
        },
        C2_TYPE_AABB => match typeB {
            C2_TYPE_CIRCLE => c2CircletoAABB_impl(*(B as *const c2Circle), *(A as *const c2AABB)),
            C2_TYPE_AABB => c2AABBtoAABB_impl(*(A as *const c2AABB), *(B as *const c2AABB)),
            _ => 0,
        },
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// f3 — floored integer division
// ---------------------------------------------------------------------------

const INT_MIN: c_int = -0x7fff_ffff - 1;

#[inline]
fn f3_impl(v1: c_int, v2: c_int) -> c_int {
    if v2 == 0 {
        return 0;
    }
    let q: c_int;
    let r: c_int;
    // The C source relies on dangling-`else` binding; the nesting below
    // reproduces it exactly. Wrapping arithmetic mirrors the two's-complement
    // wrap-around the original signed-overflow paths get from the hardware.
    if v1 >= 0 {
        if v2 >= 0 {
            return v1.wrapping_div(v2);
        } else if v2 != INT_MIN {
            q = v1.wrapping_div(v2.wrapping_neg()).wrapping_neg();
            r = v1.wrapping_rem(v2.wrapping_neg());
        } else {
            q = 0;
            r = v1;
        }
    } else if v1 != INT_MIN {
        if v2 >= 0 {
            q = v1.wrapping_neg().wrapping_div(v2).wrapping_neg();
            r = v1.wrapping_neg().wrapping_rem(v2).wrapping_neg();
        } else if v2 != INT_MIN {
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
    } else if v2 != INT_MIN {
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

#[unsafe(no_mangle)]
pub extern "C" fn f3(v1: c_int, v2: c_int) -> c_int {
    f3_impl(v1, v2)
}

// ---------------------------------------------------------------------------
// f4 — xorshift128+ derived double in [0, 1)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone)]
pub struct cn_rnd_t {
    pub state: [u64; 2],
}

#[inline]
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

#[inline]
fn f4_impl(rnd: &mut cn_rnd_t) -> f64 {
    let value = cn_rnd_next(rnd);
    let exponent: u64 = 1023;
    let mantissa: u64 = value >> 12;
    let result: u64 = (exponent << 52) | mantissa;
    // The C code type-puns the `uint64_t` through a `double *`.
    f64::from_bits(result) - 1.0
}

/// # Safety
/// `rnd` must be a valid, aligned, non-null pointer to a `cn_rnd_t`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn f4(rnd: *mut cn_rnd_t) -> f64 {
    f4_impl(&mut *rnd)
}

// ---------------------------------------------------------------------------
// f5 — 16-bit bit reversal performed on a 32-bit value
// ---------------------------------------------------------------------------

#[inline]
fn f5_impl(a: u32) -> u32 {
    let mut a = a;
    // The masks are 16-bit, so any bits above bit 15 are discarded — kept as-is.
    a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    a = ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8);
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn f5(a: u32) -> u32 {
    f5_impl(a)
}

// ---------------------------------------------------------------------------
// tflac
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub type tflac_u8 = u8;
#[allow(dead_code)]
pub type tflac_u16 = u16;
pub type tflac_u32 = u32;

#[inline]
fn f7_impl(blocksize: tflac_u32, channels: tflac_u32, bitdepth: tflac_u32) -> tflac_u32 {
    // Reproduces the original expression verbatim, including the stray unary
    // `+` in front of the `7` and the unsigned wrap-around.
    let term1 = blocksize
        .wrapping_mul(bitdepth)
        .wrapping_mul(channels.wrapping_mul(ubool(channels != 2)));
    let term2 = blocksize
        .wrapping_mul(bitdepth)
        .wrapping_mul(ubool(channels == 2));
    let term3 = blocksize
        .wrapping_mul(bitdepth.wrapping_add(ubool(bitdepth != 32)))
        .wrapping_mul(ubool(channels == 2));
    18u32.wrapping_add(channels).wrapping_add(
        term1
            .wrapping_add(term2)
            .wrapping_add(term3)
            .wrapping_add(7)
            / 8,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn f7(blocksize: tflac_u32, channels: tflac_u32, bitdepth: tflac_u32) -> tflac_u32 {
    f7_impl(blocksize, channels, bitdepth)
}

// ---------------------------------------------------------------------------
// lightmapper — barycentric coordinates
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone)]
pub struct lm_vec2 {
    pub x: f32,
    pub y: f32,
}

#[inline]
fn lm_v2(x: f32, y: f32) -> lm_vec2 {
    lm_vec2 { x, y }
}

#[inline]
fn lm_sub2(a: lm_vec2, b: lm_vec2) -> lm_vec2 {
    lm_v2(subss(a.x, b.x), subss(a.y, b.y))
}

#[inline]
fn f9_impl(p1: lm_vec2, p2: lm_vec2, p3: lm_vec2, p: lm_vec2) -> lm_vec2 {
    let v0 = lm_sub2(p3, p1);
    let v1 = lm_sub2(p2, p1);
    let v2 = lm_sub2(p, p1);
    // `lm_dot2` is inlined; the operand roles below match the `mulss`/`addss`
    // pairs the C compiler emits for each of the five dot products.
    let dot00 = addss(mulss(v0.x, v0.x), mulss(v0.y, v0.y));
    let dot01 = addss(mulss(v1.x, v0.x), mulss(v1.y, v0.y));
    let dot02 = addss(mulss(v0.y, v2.y), mulss(v0.x, v2.x));
    let dot11 = addss(mulss(v1.x, v1.x), mulss(v1.y, v1.y));
    let dot12 = addss(mulss(v1.x, v2.x), mulss(v2.y, v1.y));
    let inv_denom = divss(1.0f32, subss(mulss(dot11, dot00), mulss(dot01, dot01)));
    let u = mulss(
        subss(mulss(dot11, dot02), mulss(dot12, dot01)),
        inv_denom,
    );
    let v = mulss(
        subss(mulss(dot12, dot00), mulss(dot02, dot01)),
        inv_denom,
    );
    lm_v2(u, v)
}

#[unsafe(no_mangle)]
pub extern "C" fn f9(p1: lm_vec2, p2: lm_vec2, p3: lm_vec2, p: lm_vec2) -> lm_vec2 {
    f9_impl(p1, p2, p3, p)
}

// ---------------------------------------------------------------------------
// f10 — half-float to float via lookup tables
// ---------------------------------------------------------------------------

#[inline]
fn f10_impl(h: u16) -> f32 {
    let n = (h >> 10) as usize;
    let num =
        m__mantissa[((h & 0x3ff) as usize) + (m__offset[n] as usize)].wrapping_add(m__exponent[n]);
    f32::from_bits(num)
}

#[unsafe(no_mangle)]
pub extern "C" fn f10(h: u16) -> f32 {
    f10_impl(h)
}

// ---------------------------------------------------------------------------
// Colour space conversions
// ---------------------------------------------------------------------------

#[inline]
fn f11_impl(dest: &mut [f32; 3], src: &[f32; 3]) {
    let h = src[0];
    let s = src[1];
    let l = src[2];
    let c: f32;
    let m: f32;
    let x: f32;
    if s == 0.0 {
        dest[0] = l;
        dest[1] = l;
        dest[2] = l;
        return;
    }
    // c = (1.0f - fabsf(2.0f * l - 1.0f)) * s
    c = mulss(subss(1.0f32, fabsf(subss(addss(l, l), 1.0f32))), s);
    // m = 1.0f * (l - 0.5f * c)
    m = mulss(1.0f32, subss(l, mulss(c, 0.5f32)));
    // x = c * (1.0f - fabsf(fmodf(h / 60.0f, 2) - 1.0f))
    x = mulss(
        subss(
            1.0f32,
            fabsf(subss(fmodf(divss(h, 60.0f32), 2.0f32), 1.0f32)),
        ),
        c,
    );
    if h >= 0.0f32 && h < 60.0f32 {
        dest[0] = addss(c, m);
        dest[1] = addss(x, m);
        dest[2] = m;
    } else if h >= 60.0f32 && h < 120.0f32 {
        dest[0] = addss(x, m);
        dest[1] = addss(c, m);
        dest[2] = m;
    } else if h < 120.0f32 && h < 180.0f32 {
        // NOTE: the original C reads `h < 120.0f && h < 180.0f` (rather than
        // `h >= 120.0f`). Reproduced verbatim — do not "fix".
        dest[0] = m;
        dest[1] = addss(c, m);
        dest[2] = addss(m, x);
    } else if h >= 180.0f32 && h < 240.0f32 {
        dest[0] = m;
        dest[1] = addss(x, m);
        dest[2] = addss(m, c);
    } else if h >= 240.0f32 && h < 300.0f32 {
        dest[0] = addss(x, m);
        dest[1] = m;
        dest[2] = addss(m, c);
    } else if h >= 300.0f32 && h < 360.0f32 {
        dest[0] = addss(c, m);
        dest[1] = m;
        dest[2] = addss(m, x);
    } else {
        dest[0] = m;
        dest[1] = m;
        dest[2] = m;
    }
}

/// # Safety
/// `dest` must be writable for 3 `float`s and `src` readable for 3 `float`s.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn f11(dest: *mut f32, src: *const f32) {
    // All loads happen before any store, matching the C codegen so that
    // overlapping `dest`/`src` behaves the same.
    let s: [f32; 3] = [*src.add(0), *src.add(1), *src.add(2)];
    let mut d: [f32; 3] = [0.0, 0.0, 0.0];
    f11_impl(&mut d, &s);
    *dest.add(0) = d[0];
    *dest.add(1) = d[1];
    *dest.add(2) = d[2];
}

#[inline]
fn f12_impl(dest: &mut [f32; 3], src: &[f32; 3]) {
    let r: f32;
    let g: f32;
    let b: f32;
    let f: f32;
    let p: f32;
    let q: f32;
    let t: f32;
    let mut h = src[0];
    let s = src[1];
    let v = src[2];
    let i: c_int;
    if s == 0.0 {
        dest[0] = v;
        dest[1] = v;
        dest[2] = v;
        return;
    }
    h = divss(h, 60.0f32);
    i = c_cast_f32_to_i32(floorf(h));
    f = subss(h, i as f32);
    p = mulss(subss(1.0f32, s), v);
    q = mulss(subss(1.0f32, mulss(s, f)), v);
    t = mulss(subss(1.0f32, mulss(subss(1.0f32, f), s)), v);
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
    dest[0] = r;
    dest[1] = g;
    dest[2] = b;
}

/// # Safety
/// `dest` must be writable for 3 `float`s and `src` readable for 3 `float`s.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn f12(dest: *mut f32, src: *const f32) {
    let s: [f32; 3] = [*src.add(0), *src.add(1), *src.add(2)];
    let mut d: [f32; 3] = [0.0, 0.0, 0.0];
    f12_impl(&mut d, &s);
    *dest.add(0) = d[0];
    *dest.add(1) = d[1];
    *dest.add(2) = d[2];
}

#[inline]
fn f13_impl(dest: &mut [f32; 3], src: &[f32; 3]) {
    let r = src[0];
    let g = src[1];
    let b = src[2];
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
    delta = subss(max, min);
    v = max;
    if delta == 0.0 || max == 0.0 {
        dest[0] = h;
        dest[1] = s;
        dest[2] = v;
        return;
    }
    s = divss(delta, max);
    if r == max {
        h = divss(subss(g, b), delta);
    } else if g == max {
        h = addss(2.0f32, divss(subss(b, r), delta));
    } else {
        h = addss(4.0f32, divss(subss(r, g), delta));
    }
    h = mulss(h, 60.0f32);
    if h < 0.0 {
        h = addss(h, 360.0f32);
    }
    dest[0] = h;
    dest[1] = s;
    dest[2] = v;
}

/// # Safety
/// `dest` must be writable for 3 `float`s and `src` readable for 3 `float`s.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn f13(dest: *mut f32, src: *const f32) {
    let s: [f32; 3] = [*src.add(0), *src.add(1), *src.add(2)];
    let mut d: [f32; 3] = [0.0, 0.0, 0.0];
    f13_impl(&mut d, &s);
    *dest.add(0) = d[0];
    *dest.add(1) = d[1];
    *dest.add(2) = d[2];
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
    let f2_6: c_uint = C2_TYPE_CIRCLE;

    let f2_11 = c2AABB {
        min: c2v { x: f2_7, y: f2_8 },
        max: c2v { x: f2_9, y: f2_10 },
    };
    let f2_12: c_uint = C2_TYPE_AABB;

    let f2_r: c_int = unsafe {
        f2(
            &f2_5 as *const c2Circle as *const c_void,
            f2_6,
            &f2_11 as *const c2AABB as *const c_void,
            f2_12,
        )
    };
    ret += f2_r as f64;

    let f3_r: c_int = f3_impl(f3_1, f3_2);
    ret += f3_r as f64;

    let mut f4_3 = cn_rnd_t {
        state: [f4_1, f4_2],
    };
    let f4_r: f64 = f4_impl(&mut f4_3);
    if !f4_r.is_nan() {
        ret += f4_r;
    }

    let f5_r: u32 = f5_impl(f5_1);
    ret += f5_r as f64;

    let f7_r: tflac_u32 = f7_impl(f7_1, f7_2, f7_3);
    ret += f7_r as f64;

    let f9_3 = lm_vec2 { x: f9_1, y: f9_2 };
    let f9_6 = lm_vec2 { x: f9_4, y: f9_5 };
    let f9_9 = lm_vec2 { x: f9_7, y: f9_8 };
    let f9_12 = lm_vec2 { x: f9_10, y: f9_11 };

    let f9_r: lm_vec2 = f9_impl(f9_3, f9_6, f9_9, f9_12);
    if !f9_r.x.is_nan() {
        ret += f9_r.x as f64;
    }
    if !f9_r.y.is_nan() {
        ret += f9_r.y as f64;
    }

    let f10_r: f32 = f10_impl(f10_1);
    if !f10_r.is_nan() {
        ret += f10_r as f64;
    }

    let mut f11_r: [f32; 3] = [0.0, 0.0, 0.0];
    let f11_5: [f32; 3] = [f11_2, f11_3, f11_4];
    f11_impl(&mut f11_r, &f11_5);
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
    f12_impl(&mut f12_r, &f12_5);
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
    f13_impl(&mut f13_r, &f13_5);
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
