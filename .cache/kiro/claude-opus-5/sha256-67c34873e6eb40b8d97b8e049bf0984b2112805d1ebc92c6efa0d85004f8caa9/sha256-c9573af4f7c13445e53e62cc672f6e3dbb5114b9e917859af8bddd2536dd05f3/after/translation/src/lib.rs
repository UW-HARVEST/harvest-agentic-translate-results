//! Rust translation of `c_src/src/lib.c` (public header `c_src/include/lib.h`).
//!
//! The C library is built as a single shared object exporting 20 public symbols:
//!   c2V, c2Maxv, c2Minv, c2Clampv, c2Sub, c2Dot, c2CircletoCircle,
//!   c2CircletoAABB, c2AABBtoAABB, f2, f3, f4, f5, f7, f9, f10, f11, f12, f13,
//!   agglom
//!
//! There are no namespace-renaming macros in the public header, so the linker
//! names are identical to the source-level names.
//!
//! Semantics are reproduced exactly, including the original bugs (e.g. the
//! `h < 120.0f && h < 180.0f` branch in `f11`, which is unreachable-as-written
//! in the C original and must stay that way).

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

mod tables;

use core::ffi::{c_int, c_uint, c_void};

use tables::{M_EXPONENT, M_MANTISSA, M_OFFSET};

// ---------------------------------------------------------------------------
// libm bindings
//
// The C translation unit calls fabsf/fmodf/floorf from <math.h> (CMake links
// `m`).  Bind to the very same symbols so rounding is bit-for-bit identical.
// ---------------------------------------------------------------------------
extern "C" {
    fn fmodf(x: f32, y: f32) -> f32;
    fn floorf(x: f32) -> f32;
}

/// GCC inlines `fabsf` as `andps` against `0x7fffffff`: it clears the sign bit
/// and touches nothing else, so a signalling NaN stays signalling.  A libm
/// *call* would be free to quiet it, hence the explicit bit operation.
#[inline(always)]
fn absf(x: f32) -> f32 {
    f32::from_bits(x.to_bits() & 0x7FFF_FFFF)
}

/// C's `(int)` cast from `float`, as compiled on x86-64 (`cvttss2si`):
/// out-of-range values and NaN yield `0x80000000`.  Rust's `as` saturates
/// instead, so it cannot be used directly.
#[inline]
fn c_float_to_int(v: f32) -> c_int {
    if v.is_nan() || v >= 2147483648.0f32 || v < -2147483648.0f32 {
        i32::MIN as c_int
    } else {
        v as i32 as c_int
    }
}

// ---------------------------------------------------------------------------
// Scalar SSE arithmetic with exact x86 NaN propagation.
//
// `ADDSS/SUBSS/MULSS/DIVSS src1, src2` return src1 quieted when src1 is NaN,
// otherwise src2 quieted when src2 is NaN.  So when *both* operands can be
// NaN the operand order decides which payload/sign survives, and GCC is free
// to swap the operands of the commutative ops (and to rewrite `a - k*b` as
// `a + (-k)*b`).  LLVM makes different choices, which showed up as differing
// NaN payloads in `f9` and `f11`.
//
// These helpers pin the selection down explicitly.  For non-NaN operands they
// are plain IEEE-754 single-precision ops, i.e. the same instruction, so
// nothing else changes.  `a` is src1, `b` is src2.
// ---------------------------------------------------------------------------

/// x86 quiets an incoming NaN by setting the mantissa MSB; the sign and the
/// rest of the payload are passed through untouched.
#[inline(always)]
fn quiet(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

#[inline(always)]
fn nan_src(a: f32, b: f32) -> Option<f32> {
    if a.is_nan() {
        Some(quiet(a))
    } else if b.is_nan() {
        Some(quiet(b))
    } else {
        None
    }
}

#[inline(always)]
fn addss(a: f32, b: f32) -> f32 {
    match nan_src(a, b) {
        Some(n) => n,
        None => a + b,
    }
}

#[inline(always)]
fn subss(a: f32, b: f32) -> f32 {
    match nan_src(a, b) {
        Some(n) => n,
        None => a - b,
    }
}

#[inline(always)]
fn mulss(a: f32, b: f32) -> f32 {
    match nan_src(a, b) {
        Some(n) => n,
        None => a * b,
    }
}

#[inline(always)]
fn divss(a: f32, b: f32) -> f32 {
    match nan_src(a, b) {
        Some(n) => n,
        None => a / b,
    }
}

// ---------------------------------------------------------------------------
// tinyc2 (`c2*`)
// ---------------------------------------------------------------------------

/// `typedef enum { C2_TYPE_CIRCLE, C2_TYPE_AABB } C2_TYPE;`
///
/// GCC gives this all-non-negative enum a 32-bit unsigned representation.
pub const C2_TYPE_CIRCLE: c_uint = 0;
pub const C2_TYPE_AABB: c_uint = 1;

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
    a.x -= b.x;
    a.y -= b.y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    // GCC (-O0, the CMake default) emits, verbatim:
    //     xmm1 = a.x; xmm0 = b.x; mulss %xmm0,%xmm1   -> P1 = mulss(a.x, b.x)
    //     xmm2 = a.y; xmm0 = b.y; mulss %xmm2,%xmm0   -> P2 = mulss(b.y, a.y)
    //                             addss %xmm1,%xmm0   -> addss(P2, P1)
    // i.e. the second product and the addend are both operand-swapped
    // relative to the C source.  Only observable when two NaNs meet, but
    // then the surviving payload depends on it (ADDSS/MULSS quiet src1 first).
    addss(mulss(b.y, a.y), mulss(a.x, b.x))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircle(A: c2Circle, B: c2Circle) -> c_int {
    let c = c2Sub(B.p, A.p);
    let d2 = c2Dot(c, c);
    let mut r2 = A.r + B.r;
    r2 = r2 * r2;
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(A: c2Circle, B: c2AABB) -> c_int {
    let L = c2Clampv(A.p, B.min, B.max);
    let ab = c2Sub(A.p, L);
    let d2 = c2Dot(ab, ab);
    let r2 = A.r * A.r;
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

/// `int f2(const void *A, C2_TYPE typeA, const void *B, C2_TYPE typeB)`
///
/// # Safety
/// `A` and `B` must point to objects of the type indicated by `typeA`/`typeB`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn f2(
    A: *const c_void,
    typeA: c_uint,
    B: *const c_void,
    typeB: c_uint,
) -> c_int {
    match typeA {
        C2_TYPE_CIRCLE => match typeB {
            C2_TYPE_CIRCLE => c2CircletoCircle(
                *(A as *const c2Circle),
                *(B as *const c2Circle),
            ),
            C2_TYPE_AABB => {
                c2CircletoAABB(*(A as *const c2Circle), *(B as *const c2AABB))
            }
            _ => 0,
        },
        C2_TYPE_AABB => match typeB {
            C2_TYPE_CIRCLE => {
                c2CircletoAABB(*(B as *const c2Circle), *(A as *const c2AABB))
            }
            C2_TYPE_AABB => {
                c2AABBtoAABB(*(A as *const c2AABB), *(B as *const c2AABB))
            }
            _ => 0,
        },
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// f3 - floored integer division
//
// Transcribed statement-for-statement, including the comma expressions.  All
// arithmetic uses wrapping ops so that the guarded INT_MIN paths behave like
// the compiled C rather than panicking.
// ---------------------------------------------------------------------------

const C_INT_MIN: c_int = -0x7fffffff - 1;

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
            // `q = 1, r = v1 - q * v2;` - q is assigned before r is evaluated.
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
// f4 - xorshift128+ derived double in [0, 1)
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

/// `double f4(cn_rnd_t *rnd)`
///
/// # Safety
/// `rnd` must be a valid pointer to a `cn_rnd_t`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn f4(rnd: *mut cn_rnd_t) -> f64 {
    let value = cn_rnd_next(&mut *rnd);
    let exponent: u64 = 1023;
    let mantissa: u64 = value >> 12;
    let result: u64 = (exponent << 52) | mantissa;
    // C does `*(double *)&result` - a raw reinterpretation of the bits.
    f64::from_bits(result) - 1.0
}

// ---------------------------------------------------------------------------
// f5 - reverse the low 16 bits
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
// f7 - tflac frame size upper bound
// ---------------------------------------------------------------------------

/// `typedef uint32_t tflac_u32;`
pub type tflac_u32 = u32;

#[unsafe(no_mangle)]
pub extern "C" fn f7(blocksize: tflac_u32, channels: tflac_u32, bitdepth: tflac_u32) -> tflac_u32 {
    // Every operation below is unsigned 32-bit and therefore wraps in C too.
    let a = blocksize
        .wrapping_mul(bitdepth)
        .wrapping_mul(channels.wrapping_mul((channels != 2) as u32));
    let b = blocksize
        .wrapping_mul(bitdepth)
        .wrapping_mul((channels == 2) as u32);
    let c = blocksize
        .wrapping_mul(bitdepth.wrapping_add((bitdepth != 32) as u32))
        .wrapping_mul((channels == 2) as u32);
    18u32
        .wrapping_add(channels)
        .wrapping_add(
            a.wrapping_add(b)
                .wrapping_add(c)
                .wrapping_add(7)
                / 8,
        )
}

// ---------------------------------------------------------------------------
// f9 - lightmapper barycentric coordinates
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
    lm_v2(subss(a.x, b.x), subss(a.y, b.y))
}

/// `static float lm_dot2(lm_vec2 a, lm_vec2 b)`
///
/// GCC compiles this byte-identically to `c2Dot`: the second `mulss` and the
/// `addss` both have their operands swapped relative to the C source, so the
/// surviving NaN payload is `mulss(b.y, a.y)`'s, not `mulss(a.x, b.x)`'s.
fn lm_dot2(a: lm_vec2, b: lm_vec2) -> f32 {
    addss(mulss(b.y, a.y), mulss(a.x, b.x))
}

#[unsafe(no_mangle)]
pub extern "C" fn f9(p1: lm_vec2, p2: lm_vec2, p3: lm_vec2, p: lm_vec2) -> lm_vec2 {
    // Operand order below is transcribed from the GCC -O0 disassembly of `f9`
    // (see the `mulss`/`subss`/`divss` sequence at 0x1bed..0x1c5f).  For
    // non-NaN inputs every line is plain IEEE-754 single precision, so the
    // ordering is only observable through NaN payload selection.
    let v0 = lm_sub2(p3, p1);
    let v1 = lm_sub2(p2, p1);
    let v2 = lm_sub2(p, p1);

    let dot00 = lm_dot2(v0, v0);
    let dot01 = lm_dot2(v0, v1);
    let dot02 = lm_dot2(v0, v2);
    let dot11 = lm_dot2(v1, v1);
    let dot12 = lm_dot2(v1, v2);

    // invDenom = 1.0f / (dot00 * dot11 - dot01 * dot01)
    let inv_denom = divss(1.0f32, subss(mulss(dot00, dot11), mulss(dot01, dot01)));
    // u = (dot11 * dot02 - dot01 * dot12) * invDenom
    let u = mulss(
        subss(mulss(dot11, dot02), mulss(dot01, dot12)),
        inv_denom,
    );
    // v = (dot00 * dot12 - dot01 * dot02) * invDenom
    let v = mulss(
        subss(mulss(dot00, dot12), mulss(dot01, dot02)),
        inv_denom,
    );
    lm_v2(u, v)
}

// ---------------------------------------------------------------------------
// f10 - half-precision to float via lookup tables
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn f10(h: u16) -> f32 {
    // `int n = h >> 10;` - h promotes to int, so n is in 0..=63.
    let n = (h >> 10) as usize;
    let num = M_MANTISSA[((h & 0x3ff) as usize) + (M_OFFSET[n] as usize)]
        .wrapping_add(M_EXPONENT[n]);
    f32::from_bits(num)
}

// ---------------------------------------------------------------------------
// f11 - HSL to RGB
// ---------------------------------------------------------------------------

/// `void f11(float *dest, const float *src)`
///
/// # Safety
/// `dest` must be writable for 3 floats and `src` readable for 3 floats.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn f11(dest: *mut f32, src: *const f32) {
    let h = *src.add(0);
    let s = *src.add(1);
    let l = *src.add(2);
    if s == 0.0 {
        *dest.add(0) = l;
        *dest.add(1) = l;
        *dest.add(2) = l;
        return;
    }
    // c = (1.0f - fabsf(2.0f * l - 1.0f)) * s
    // GCC emits `2.0f * l` as `addss %xmm0,%xmm0` (l + l) and inlines `fabsf`
    // as `andps` against 0x7fffffff, which clears the sign bit *without*
    // quieting a signalling NaN.  `f32::abs` is the same bit operation.
    let c = mulss(subss(1.0f32, absf(subss(addss(l, l), 1.0f32))), s);
    // m = 1.0f * (l - 0.5f * c)   -- the `1.0f *` is a no-op and GCC drops it.
    let m = subss(l, mulss(c, 0.5f32));
    // x = c * (1.0f - fabsf(fmodf(h / 60.0f, 2) - 1.0f))
    // GCC emits this multiply with the operands swapped (src1 is the
    // parenthesised term, src2 is `c`).
    let x = mulss(
        subss(1.0f32, absf(subss(fmodf(divss(h, 60.0f32), 2.0f32), 1.0f32))),
        c,
    );
    // Every `dest[..] = <a> + m` below is `addss(src1 = <a>, src2 = m)` in the
    // emitted code, i.e. exactly the source order.
    if h >= 0.0f32 && h < 60.0f32 {
        *dest.add(0) = addss(c, m);
        *dest.add(1) = addss(x, m);
        *dest.add(2) = m;
    } else if h >= 60.0f32 && h < 120.0f32 {
        *dest.add(0) = addss(x, m);
        *dest.add(1) = addss(c, m);
        *dest.add(2) = m;
    } else if h < 120.0f32 && h < 180.0f32 {
        // Reproduced verbatim from the C: the first condition should be
        // `h >= 120.0f`.  Do not "fix" it.
        *dest.add(0) = m;
        *dest.add(1) = addss(c, m);
        *dest.add(2) = addss(x, m);
    } else if h >= 180.0f32 && h < 240.0f32 {
        *dest.add(0) = m;
        *dest.add(1) = addss(x, m);
        *dest.add(2) = addss(c, m);
    } else if h >= 240.0f32 && h < 300.0f32 {
        *dest.add(0) = addss(x, m);
        *dest.add(1) = m;
        *dest.add(2) = addss(c, m);
    } else if h >= 300.0f32 && h < 360.0f32 {
        *dest.add(0) = addss(c, m);
        *dest.add(1) = m;
        *dest.add(2) = addss(x, m);
    } else {
        *dest.add(0) = m;
        *dest.add(1) = m;
        *dest.add(2) = m;
    }
}

// ---------------------------------------------------------------------------
// f12 - HSV to RGB
// ---------------------------------------------------------------------------

/// `void f12(float *dest, const float *src)`
///
/// # Safety
/// `dest` must be writable for 3 floats and `src` readable for 3 floats.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn f12(dest: *mut f32, src: *const f32) {
    let r: f32;
    let g: f32;
    let b: f32;
    let mut h = *src.add(0);
    let s = *src.add(1);
    let v = *src.add(2);
    if s == 0.0 {
        *dest.add(0) = v;
        *dest.add(1) = v;
        *dest.add(2) = v;
        return;
    }
    h = divss(h, 60.0f32);
    let i: c_int = c_float_to_int(floorf(h));
    let f = subss(h, i as f32);
    // p = v * (1 - s);  q = v * (1 - s*f);  t = v * (1 - s*(1 - f));
    // GCC evaluates each product with the parenthesised term as src1.
    let p = mulss(subss(1.0f32, s), v);
    let q = mulss(subss(1.0f32, mulss(s, f)), v);
    let t = mulss(subss(1.0f32, mulss(subss(1.0f32, f), s)), v);
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

// ---------------------------------------------------------------------------
// f13 - RGB to HSV
// ---------------------------------------------------------------------------

/// `void f13(float *dest, const float *src)`
///
/// # Safety
/// `dest` must be writable for 3 floats and `src` readable for 3 floats.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn f13(dest: *mut f32, src: *const f32) {
    let r = *src.add(0);
    let g = *src.add(1);
    let b = *src.add(2);
    let mut h: f32 = 0.0;
    let mut s: f32 = 0.0;
    let v: f32;
    let mut min = r;
    let mut max = r;
    min = if min < g { min } else { g };
    min = if min < b { min } else { b };
    max = if max > g { max } else { g };
    max = if max > b { max } else { b };
    let delta = max - min;
    v = max;
    if delta == 0.0 || max == 0.0 {
        *dest.add(0) = h;
        *dest.add(1) = s;
        *dest.add(2) = v;
        return;
    }
    s = delta / max;
    if r == max {
        h = (g - b) / delta;
    } else if g == max {
        h = 2.0f32 + (b - r) / delta;
    } else {
        h = 4.0f32 + (r - g) / delta;
    }
    h *= 60.0f32;
    if h < 0.0 {
        h += 360.0f32;
    }
    *dest.add(0) = h;
    *dest.add(1) = s;
    *dest.add(2) = v;
}

// ---------------------------------------------------------------------------
// agglom - the public aggregate entry point declared in include/lib.h
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
