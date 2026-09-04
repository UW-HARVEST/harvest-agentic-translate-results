//! Rust translation of the C library in `c_src/` (a cute_c2 derived 2D
//! collision library plus the `capsule` entry point declared in `include/lib.h`).
//!
//! Every non-static C function is reproduced with the exact same linker name,
//! signature and observable behaviour, including its quirks (fall-through
//! `switch` labels, the inverted GJK cache-validity test, reading a shape
//! through an unchecked `void*` cast, ...).
//!
//! Bit-exactness note: all arithmetic is single precision, evaluated in the
//! same association order as the C.  On top of that, the tiny `*_ss` helpers
//! below reproduce x86 SSE NaN-propagation (the *destination* operand of
//! `mulss`/`addss`/`subss`/`divss` wins, a signalling NaN is quieted, and an
//! invalid operation on two non-NaN operands yields the "indefinite" QNaN
//! `0xFFC00000`).  The destination operand chosen at each call site mirrors the
//! instruction GCC emits for the original source, so even NaN payloads and
//! sign bits come out identical.

#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    unused_assignments,
    unused_variables,
    unused_mut
)]

use std::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// typedef enum { C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE } C2_TYPE;
// ---------------------------------------------------------------------------
pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

/// `FLT_MAX`, spelled exactly as in the C source.
const C2_FLT_MAX: f32 = 3.402_823_466_385_288_598_117_041_834_845_169_25e+38;
/// `FLT_EPSILON`, spelled exactly as in the C source.
const C2_FLT_EPSILON: f32 = 1.192_092_895_507_812_5e-7;
/// `FLT_EPSILON * FLT_EPSILON` (exactly 2^-46, matching GCC's constant folding).
const C2_FLT_EPSILON_SQ: f32 = C2_FLT_EPSILON * C2_FLT_EPSILON;

// ---------------------------------------------------------------------------
// x86 SSE scalar arithmetic helpers (see the bit-exactness note above)
// ---------------------------------------------------------------------------

#[inline(always)]
fn quiet_nan(x: f32) -> f32 {
    // x86 turns a signalling NaN into a quiet one by setting the significand MSB
    // while preserving the sign bit and the rest of the payload.
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

/// `mulss dst, src`
#[inline(always)]
fn mul_ss(dst: f32, src: f32) -> f32 {
    if dst.is_nan() {
        quiet_nan(dst)
    } else if src.is_nan() {
        quiet_nan(src)
    } else {
        // Neither operand is a NaN, so the hardware result no longer depends on
        // which operand is the destination.
        dst * src
    }
}

/// `addss dst, src`
#[inline(always)]
fn add_ss(dst: f32, src: f32) -> f32 {
    if dst.is_nan() {
        quiet_nan(dst)
    } else if src.is_nan() {
        quiet_nan(src)
    } else {
        dst + src
    }
}

/// `subss dst, src` (computes `dst - src`)
#[inline(always)]
fn sub_ss(dst: f32, src: f32) -> f32 {
    if dst.is_nan() {
        quiet_nan(dst)
    } else if src.is_nan() {
        quiet_nan(src)
    } else {
        dst - src
    }
}

/// `divss dst, src` (computes `dst / src`)
#[inline(always)]
fn div_ss(dst: f32, src: f32) -> f32 {
    if dst.is_nan() {
        quiet_nan(dst)
    } else if src.is_nan() {
        quiet_nan(src)
    } else {
        dst / src
    }
}

// ---------------------------------------------------------------------------
// Plain-old-data types (layout identical to the C structs)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

/// `typedef struct { float radius; int count; c2v verts[8]; } c2Proxy;`
#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
}

/// `typedef struct { c2v sA, sB, p; float u; int iA, iB; } c2sv;`
#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: c_int,
    pub iB: c_int,
}

/// `typedef struct { c2sv a, b, c, d; float div; int count; } c2Simplex;`
///
/// The four `c2sv` members are held in an array so that the C idiom
/// `c2sv* verts = &s.a; verts[i]` can be reproduced verbatim; the memory layout
/// is byte-for-byte identical to the C declaration (4 * 36 + 4 + 4 = 152 bytes).
#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

// ---------------------------------------------------------------------------
// Basic vector maths
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    let mut a = c2v::default();
    a.x = x;
    a.y = y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(mut a: c2v, b: f32) -> c2v {
    a.x = mul_ss(a.x, b);
    a.y = mul_ss(a.y, b);
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
pub extern "C" fn c2Sub(mut a: c2v, b: c2v) -> c2v {
    a.x = sub_ss(a.x, b.x);
    a.y = sub_ss(a.y, b.y);
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    // mulss dst=a.x ; mulss dst=b.y ; addss dst=(a.y*b.y term)
    let p1 = mul_ss(a.x, b.x);
    let p2 = mul_ss(b.y, a.y);
    add_ss(p2, p1)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2RotIdentity() -> c2r {
    let mut r = c2r::default();
    r.c = 1.0f32;
    r.s = 0.0;
    r
}

#[unsafe(no_mangle)]
pub extern "C" fn c2xIdentity() -> c2x {
    let mut x = c2x::default();
    x.p = c2V(0.0, 0.0);
    x.r = c2RotIdentity();
    x
}

// ---------------------------------------------------------------------------
// Proxies
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2BBVerts(out: *mut c2v, bb: *mut c2AABB) {
    *out.add(0) = (*bb).min;
    *out.add(1) = c2V((*bb).max.x, (*bb).min.y);
    *out.add(2) = (*bb).max;
    *out.add(3) = c2V((*bb).min.x, (*bb).max.y);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(shape: *const c_void, type_: c_int, p: *mut c2Proxy) {
    match type_ {
        C2_TYPE_CIRCLE => {
            let c = shape as *mut c2Circle;
            (*p).radius = (*c).r;
            (*p).count = 1;
            (*p).verts[0] = (*c).p;
        }
        C2_TYPE_AABB => {
            let bb = shape as *mut c2AABB;
            (*p).radius = 0.0;
            (*p).count = 4;
            c2BBVerts((*p).verts.as_mut_ptr(), bb);
        }
        C2_TYPE_CAPSULE => {
            let c = shape as *mut c2Capsule;
            (*p).radius = (*c).r;
            (*p).count = 2;
            (*p).verts[0] = (*c).a;
            (*p).verts[1] = (*c).b;
        }
        // The C `switch` has no `default:` label, so nothing is written.
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Simplex helpers
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> f32 {
    // sqrtf(c2Dot(a, a)) -- `sqrtss`, so a negative operand yields the
    // indefinite QNaN and a NaN operand is merely quieted.
    c2Dot(a, a).sqrt()
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: c2v, b: c2v) -> f32 {
    // mulss dst=b.y ; mulss dst=b.x ; subss dst=first term
    let t1 = mul_ss(b.y, a.x);
    let t2 = mul_ss(b.x, a.y);
    sub_ss(t1, t2)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJKSimplexMetric(s: *mut c2Simplex) -> f32 {
    match (*s).count {
        2 => c2Len(c2Sub((*s).verts[1].p, (*s).verts[0].p)),
        3 => c2Det2(
            c2Sub((*s).verts[1].p, (*s).verts[0].p),
            c2Sub((*s).verts[2].p, (*s).verts[0].p),
        ),
        // `default:` falls through into `case 1:`, which returns 0.
        _ => 0.0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    // x = a.c * b.x - a.s * b.y  ->  subss(mulss dst=b.x, mulss dst=b.y)
    // y = a.s * b.x + a.c * b.y  ->  addss(mulss dst=a.s, mulss dst=b.y)
    let x = sub_ss(mul_ss(b.x, a.c), mul_ss(b.y, a.s));
    let y = add_ss(mul_ss(a.s, b.x), mul_ss(b.y, a.c));
    c2V(x, y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Add(mut a: c2v, b: c2v) -> c2v {
    // addss dst=b.x / dst=b.y
    a.x = add_ss(b.x, a.x);
    a.y = add_ss(b.y, a.y);
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulxv(a: c2x, b: c2v) -> c2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c22(s: *mut c2Simplex) {
    let a = (*s).verts[0].p;
    let b = (*s).verts[1].p;
    let u = c2Dot(b, c2Sub(b, a));
    let v = c2Dot(a, c2Sub(a, b));
    if v <= 0.0 {
        (*s).verts[0].u = 1.0f32;
        (*s).div = 1.0f32;
        (*s).count = 1;
    } else if u <= 0.0 {
        (*s).verts[0] = (*s).verts[1];
        (*s).verts[0].u = 1.0f32;
        (*s).div = 1.0f32;
        (*s).count = 1;
    } else {
        (*s).verts[0].u = u;
        (*s).verts[1].u = v;
        (*s).div = add_ss(u, v);
        (*s).count = 2;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c23(s: *mut c2Simplex) {
    let a = (*s).verts[0].p;
    let b = (*s).verts[1].p;
    let c = (*s).verts[2].p;
    let uAB = c2Dot(b, c2Sub(b, a));
    let vAB = c2Dot(a, c2Sub(a, b));
    let uBC = c2Dot(c, c2Sub(c, b));
    let vBC = c2Dot(b, c2Sub(b, c));
    let uCA = c2Dot(a, c2Sub(a, c));
    let vCA = c2Dot(c, c2Sub(c, a));
    let area = c2Det2(c2Sub(b, a), c2Sub(c, a));
    let uABC = mul_ss(c2Det2(b, c), area);
    let vABC = mul_ss(c2Det2(c, a), area);
    let wABC = mul_ss(c2Det2(a, b), area);
    if vAB <= 0.0 && uCA <= 0.0 {
        (*s).verts[0].u = 1.0f32;
        (*s).div = 1.0f32;
        (*s).count = 1;
    } else if uAB <= 0.0 && vBC <= 0.0 {
        (*s).verts[0] = (*s).verts[1];
        (*s).verts[0].u = 1.0f32;
        (*s).div = 1.0f32;
        (*s).count = 1;
    } else if uBC <= 0.0 && vCA <= 0.0 {
        (*s).verts[0] = (*s).verts[2];
        (*s).verts[0].u = 1.0f32;
        (*s).div = 1.0f32;
        (*s).count = 1;
    } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
        (*s).verts[0].u = uAB;
        (*s).verts[1].u = vAB;
        (*s).div = add_ss(uAB, vAB);
        (*s).count = 2;
    } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
        (*s).verts[0] = (*s).verts[1];
        (*s).verts[1] = (*s).verts[2];
        (*s).verts[0].u = uBC;
        (*s).verts[1].u = vBC;
        (*s).div = add_ss(uBC, vBC);
        (*s).count = 2;
    } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
        (*s).verts[1] = (*s).verts[0];
        (*s).verts[0] = (*s).verts[2];
        (*s).verts[0].u = uCA;
        (*s).verts[1].u = vCA;
        (*s).div = add_ss(uCA, vCA);
        (*s).count = 2;
    } else {
        (*s).verts[0].u = uABC;
        (*s).verts[1].u = vABC;
        (*s).verts[2].u = wABC;
        (*s).div = add_ss(add_ss(uABC, vABC), wABC);
        (*s).count = 3;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Neg(a: c2v) -> c2v {
    // `xorps` with the sign mask: flips the sign bit of NaNs too.
    c2V(-a.x, -a.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v {
    let mut b = c2v::default();
    b.x = -a.y;
    b.y = a.x;
    b
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v {
    let mut b = c2v::default();
    b.x = a.y;
    b.y = -a.x;
    b
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2D(s: *mut c2Simplex) -> c2v {
    match (*s).count {
        1 => c2Neg((*s).verts[0].p),
        2 => {
            let ab = c2Sub((*s).verts[1].p, (*s).verts[0].p);
            if c2Det2(ab, c2Neg((*s).verts[0].p)) > 0.0 {
                return c2Skew(ab);
            }
            c2CCW90(ab)
        }
        // `case 3:` and `default:`
        _ => c2V(0.0, 0.0),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Support(verts: *const c2v, count: c_int, d: c2v) -> c_int {
    let mut imax: c_int = 0;
    let mut dmax = c2Dot(*verts.add(0), d);
    let mut i: c_int = 1;
    while i < count {
        let dot = c2Dot(*verts.offset(i as isize), d);
        if dot > dmax {
            imax = i;
            dmax = dot;
        }
        i += 1;
    }
    imax
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Witness(s: *mut c2Simplex, a: *mut c2v, b: *mut c2v) {
    let den = div_ss(1.0f32, (*s).div);
    match (*s).count {
        1 => {
            *a = (*s).verts[0].sA;
            *b = (*s).verts[0].sB;
        }
        2 => {
            *a = c2Add(
                c2Mulvs((*s).verts[0].sA, mul_ss((*s).verts[0].u, den)),
                c2Mulvs((*s).verts[1].sA, mul_ss((*s).verts[1].u, den)),
            );
            *b = c2Add(
                c2Mulvs((*s).verts[0].sB, mul_ss((*s).verts[0].u, den)),
                c2Mulvs((*s).verts[1].sB, mul_ss((*s).verts[1].u, den)),
            );
        }
        3 => {
            *a = c2Add(
                c2Add(
                    c2Mulvs((*s).verts[0].sA, mul_ss((*s).verts[0].u, den)),
                    c2Mulvs((*s).verts[1].sA, mul_ss((*s).verts[1].u, den)),
                ),
                c2Mulvs((*s).verts[2].sA, mul_ss((*s).verts[2].u, den)),
            );
            *b = c2Add(
                c2Add(
                    c2Mulvs((*s).verts[0].sB, mul_ss((*s).verts[0].u, den)),
                    c2Mulvs((*s).verts[1].sB, mul_ss((*s).verts[1].u, den)),
                ),
                c2Mulvs((*s).verts[2].sB, mul_ss((*s).verts[2].u, den)),
            );
        }
        _ => {
            *a = c2V(0.0, 0.0);
            *b = c2V(0.0, 0.0);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: c2v, b: f32) -> c2v {
    c2Mulvs(a, div_ss(1.0f32, b))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2L(s: *mut c2Simplex) -> c2v {
    let den = div_ss(1.0f32, (*s).div);
    match (*s).count {
        1 => (*s).verts[0].p,
        2 => c2Add(
            c2Mulvs((*s).verts[0].p, mul_ss((*s).verts[0].u, den)),
            c2Mulvs((*s).verts[1].p, mul_ss((*s).verts[1].u, den)),
        ),
        _ => c2V(0.0, 0.0),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    // x =  a.c * b.x + a.s * b.y  ->  addss(mulss dst=a.c, mulss dst=b.y)
    // y = -a.s * b.x + a.c * b.y  ->  addss(mulss dst=(-a.s), mulss dst=b.y)
    let x = add_ss(mul_ss(a.c, b.x), mul_ss(b.y, a.s));
    let y = add_ss(mul_ss(-a.s, b.x), mul_ss(b.y, a.c));
    c2V(x, y)
}

// ---------------------------------------------------------------------------
// GJK
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJK(
    A: *const c_void,
    typeA: c_int,
    ax_ptr: *const c2x,
    B: *const c_void,
    typeB: c_int,
    bx_ptr: *const c2x,
    outA: *mut c2v,
    outB: *mut c2v,
    use_radius: c_int,
    iterations: *mut c_int,
    cache: *mut c2GJKCache,
) -> f32 {
    let ax: c2x;
    let bx: c2x;
    if ax_ptr.is_null() {
        ax = c2xIdentity();
    } else {
        ax = *ax_ptr;
    }
    if bx_ptr.is_null() {
        bx = c2xIdentity();
    } else {
        bx = *bx_ptr;
    }
    let mut pA = c2Proxy::default();
    let mut pB = c2Proxy::default();
    c2MakeProxy(A, typeA, &mut pA);
    c2MakeProxy(B, typeB, &mut pB);
    // Raw views of the fixed-size C arrays: the original indexes them without
    // any bounds check, so Rust must not introduce a panicking one either.
    let pAv: *const c2v = pA.verts.as_ptr();
    let pBv: *const c2v = pB.verts.as_ptr();
    let mut s = c2Simplex::default();
    let sp: *mut c2Simplex = &mut s;
    let verts: *mut c2sv = (*sp).verts.as_mut_ptr();
    let mut cache_was_read: c_int = 0;
    if !cache.is_null() {
        let cache_iA: *const c_int = (*cache).iA.as_ptr();
        let cache_iB: *const c_int = (*cache).iB.as_ptr();
        let cache_was_good: c_int = ((*cache).count != 0) as c_int;
        if cache_was_good != 0 {
            let mut i: c_int = 0;
            while i < (*cache).count {
                let iA = *cache_iA.offset(i as isize);
                let iB = *cache_iB.offset(i as isize);
                let sA = c2Mulxv(ax, *pAv.offset(iA as isize));
                let sB = c2Mulxv(bx, *pBv.offset(iB as isize));
                let v: *mut c2sv = verts.offset(i as isize);
                (*v).iA = iA;
                (*v).sA = sA;
                (*v).iB = iB;
                (*v).sB = sB;
                (*v).p = c2Sub((*v).sB, (*v).sA);
                (*v).u = 0.0;
                i += 1;
            }
            (*sp).count = (*cache).count;
            (*sp).div = (*cache).div;
            let metric_old = (*cache).metric;
            let metric = c2GJKSimplexMetric(sp);
            let min_metric = if metric < metric_old { metric } else { metric_old };
            let max_metric = if metric > metric_old { metric } else { metric_old };
            // NOTE: reproduced verbatim from the C -- the `metric < -1.0e8f`
            // term makes this test almost always true.
            if !(min_metric < mul_ss(max_metric, 2.0f32) && metric < -1.0e8f32) {
                cache_was_read = 1;
            }
        }
    }
    if cache_was_read == 0 {
        (*sp).verts[0].iA = 0;
        (*sp).verts[0].iB = 0;
        (*sp).verts[0].sA = c2Mulxv(ax, *pAv.offset(0));
        (*sp).verts[0].sB = c2Mulxv(bx, *pBv.offset(0));
        (*sp).verts[0].p = c2Sub((*sp).verts[0].sB, (*sp).verts[0].sA);
        (*sp).verts[0].u = 1.0f32;
        (*sp).div = 1.0f32;
        (*sp).count = 1;
    }
    let mut saveA: [c_int; 3] = [0; 3];
    let mut saveB: [c_int; 3] = [0; 3];
    let sA_p: *mut c_int = saveA.as_mut_ptr();
    let sB_p: *mut c_int = saveB.as_mut_ptr();
    let mut save_count: c_int = 0;
    let mut d0: f32 = C2_FLT_MAX;
    let mut d1: f32 = C2_FLT_MAX;
    let mut iter: c_int = 0;
    let mut hit: c_int = 0;
    while iter < 20 {
        save_count = (*sp).count;
        let mut i: c_int = 0;
        while i < save_count {
            *sA_p.offset(i as isize) = (*verts.offset(i as isize)).iA;
            *sB_p.offset(i as isize) = (*verts.offset(i as isize)).iB;
            i += 1;
        }
        match (*sp).count {
            1 => {}
            2 => c22(sp),
            3 => c23(sp),
            _ => {}
        }
        if (*sp).count == 3 {
            hit = 1;
            break;
        }
        let p = c2L(sp);
        d1 = c2Dot(p, p);
        if d1 > d0 {
            break;
        }
        d0 = d1;
        let d = c2D(sp);
        if c2Dot(d, d) < C2_FLT_EPSILON_SQ {
            break;
        }
        let iA = c2Support(pAv, pA.count, c2MulrvT(ax.r, c2Neg(d)));
        let sA = c2Mulxv(ax, *pAv.offset(iA as isize));
        let iB = c2Support(pBv, pB.count, c2MulrvT(bx.r, d));
        let sB = c2Mulxv(bx, *pBv.offset(iB as isize));
        let v: *mut c2sv = verts.offset((*sp).count as isize);
        (*v).iA = iA;
        (*v).sA = sA;
        (*v).iB = iB;
        (*v).sB = sB;
        (*v).p = c2Sub((*v).sB, (*v).sA);
        let mut dup: c_int = 0;
        let mut i: c_int = 0;
        while i < save_count {
            if iA == *sA_p.offset(i as isize) && iB == *sB_p.offset(i as isize) {
                dup = 1;
                break;
            }
            i += 1;
        }
        if dup != 0 {
            break;
        }
        (*sp).count += 1;
        iter += 1;
    }
    let mut a = c2v::default();
    let mut b = c2v::default();
    c2Witness(sp, &mut a, &mut b);
    let mut dist = c2Len(c2Sub(a, b));
    if hit != 0 {
        a = b;
        dist = 0.0;
    } else if use_radius != 0 {
        let rA = pA.radius;
        let rB = pB.radius;
        if dist > add_ss(rA, rB) && dist > C2_FLT_EPSILON {
            dist = sub_ss(dist, add_ss(rA, rB));
            let n = c2Norm(c2Sub(b, a));
            a = c2Add(a, c2Mulvs(n, rA));
            b = c2Sub(b, c2Mulvs(n, rB));
            if a.x == b.x && a.y == b.y {
                dist = 0.0;
            }
        } else {
            let p = c2Mulvs(c2Add(a, b), 0.5f32);
            a = p;
            b = p;
            dist = 0.0;
        }
    }
    if !cache.is_null() {
        (*cache).metric = c2GJKSimplexMetric(sp);
        (*cache).count = (*sp).count;
        let out_iA: *mut c_int = (*cache).iA.as_mut_ptr();
        let out_iB: *mut c_int = (*cache).iB.as_mut_ptr();
        let mut i: c_int = 0;
        while i < (*sp).count {
            let v: *mut c2sv = verts.offset(i as isize);
            *out_iA.offset(i as isize) = (*v).iA;
            *out_iB.offset(i as isize) = (*v).iB;
            i += 1;
        }
        (*cache).div = (*sp).div;
    }
    if !outA.is_null() {
        *outA = a;
    }
    if !outB.is_null() {
        *outB = b;
    }
    if !iterations.is_null() {
        *iterations = iter;
    }
    dist
}

// ---------------------------------------------------------------------------
// Boolean collision routines
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> c_int {
    let d0: c_int = (B.max.x < A.min.x) as c_int;
    let d1: c_int = (A.max.x < B.min.x) as c_int;
    let d2: c_int = (B.max.y < A.min.y) as c_int;
    let d3: c_int = (A.max.y < B.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoCapsule(A: c2AABB, B: c2Capsule) -> c_int {
    let a = A;
    let b = B;
    unsafe {
        if c2GJK(
            &a as *const c2AABB as *const c_void,
            C2_TYPE_AABB,
            std::ptr::null(),
            &b as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        ) != 0.0
        {
            return 0;
        }
    }
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CapsuletoCapsule(A: c2Capsule, B: c2Capsule) -> c_int {
    let a = A;
    let b = B;
    unsafe {
        if c2GJK(
            &a as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            &b as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        ) != 0.0
        {
            return 0;
        }
    }
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircle(A: c2Circle, B: c2Circle) -> c_int {
    let c = c2Sub(B.p, A.p);
    let d2 = c2Dot(c, c);
    let mut r2 = add_ss(A.r, B.r);
    r2 = mul_ss(r2, r2);
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(A: c2Circle, B: c2AABB) -> c_int {
    let L = c2Clampv(A.p, B.min, B.max);
    let ab = c2Sub(A.p, L);
    let d2 = c2Dot(ab, ab);
    let r2 = mul_ss(A.r, A.r);
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCapsule(A: c2Circle, B: c2Capsule) -> c_int {
    let n = c2Sub(B.b, B.a);
    let ap = c2Sub(A.p, B.a);
    let da = c2Dot(ap, n);
    let d2: f32;
    if da < 0.0 {
        d2 = c2Dot(ap, ap);
    } else {
        let db = c2Dot(c2Sub(A.p, B.b), n);
        if db < 0.0 {
            let e = c2Sub(ap, c2Mulvs(n, div_ss(da, c2Dot(n, n))));
            d2 = c2Dot(e, e);
        } else {
            let bp = c2Sub(A.p, B.b);
            d2 = c2Dot(bp, bp);
        }
    }
    let r = add_ss(A.r, B.r);
    (d2 < mul_ss(r, r)) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Collided(
    A: *const c_void,
    typeA: c_int,
    B: *const c_void,
    typeB: c_int,
) -> c_int {
    match typeA {
        C2_TYPE_CIRCLE => match typeB {
            C2_TYPE_CIRCLE => c2CircletoCircle(
                (A as *const c2Circle).read_unaligned(),
                (B as *const c2Circle).read_unaligned(),
            ),
            C2_TYPE_AABB => c2CircletoAABB(
                (A as *const c2Circle).read_unaligned(),
                (B as *const c2AABB).read_unaligned(),
            ),
            C2_TYPE_CAPSULE => c2CircletoCapsule(
                (A as *const c2Circle).read_unaligned(),
                (B as *const c2Capsule).read_unaligned(),
            ),
            _ => 0,
        },
        C2_TYPE_AABB => match typeB {
            C2_TYPE_CIRCLE => c2CircletoAABB(
                (B as *const c2Circle).read_unaligned(),
                (A as *const c2AABB).read_unaligned(),
            ),
            C2_TYPE_AABB => c2AABBtoAABB(
                (A as *const c2AABB).read_unaligned(),
                (B as *const c2AABB).read_unaligned(),
            ),
            C2_TYPE_CAPSULE => c2AABBtoCapsule(
                (A as *const c2AABB).read_unaligned(),
                (B as *const c2Capsule).read_unaligned(),
            ),
            _ => 0,
        },
        C2_TYPE_CAPSULE => match typeB {
            C2_TYPE_CIRCLE => c2CircletoCapsule(
                (B as *const c2Circle).read_unaligned(),
                (A as *const c2Capsule).read_unaligned(),
            ),
            C2_TYPE_AABB => c2AABBtoCapsule(
                (B as *const c2AABB).read_unaligned(),
                (A as *const c2Capsule).read_unaligned(),
            ),
            C2_TYPE_CAPSULE => c2CapsuletoCapsule(
                (A as *const c2Capsule).read_unaligned(),
                (B as *const c2Capsule).read_unaligned(),
            ),
            _ => 0,
        },
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// Public entry point declared in include/lib.h
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn capsule(min_x: f32, min_y: f32, max_x: f32, max_y: f32, r: f32) -> c_int {
    let mut result: c_int = 0;

    let mut capsule_in = c2Capsule::default();
    capsule_in.a = c2V(min_x, min_y);
    capsule_in.b = c2V(max_x, max_y);
    capsule_in.r = r;

    let mut circle = c2Circle::default();
    circle.p = c2V(-70.0f32, 0.0);
    circle.r = 20.0f32;

    let mut aabb = c2AABB::default();
    aabb.min = c2V(-40.0f32, -40.0f32);
    aabb.max = c2V(-15.0f32, -15.0f32);

    // Named `capsule` in the C source, where it shadows the function name.
    let mut capsule_local = c2Capsule::default();
    capsule_local.a = c2V(-40.0f32, 40.0f32);
    capsule_local.b = c2V(-20.0f32, 100.0f32);
    capsule_local.r = 10.0f32;

    unsafe {
        result += c2Collided(
            &circle as *const c2Circle as *const c_void,
            C2_TYPE_CIRCLE,
            &capsule_in as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
        );

        result += c2Collided(
            &aabb as *const c2AABB as *const c_void,
            C2_TYPE_AABB,
            &capsule_in as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
        ) << 1;

        result += c2Collided(
            &capsule_local as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            &capsule_in as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
        ) << 2;
    }

    result
}
