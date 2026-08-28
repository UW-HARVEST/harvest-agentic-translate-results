//! Rust translation of `c_src/src/lib.c` (a reduced `cute_c2`-style 2D collision
//! library) preserving the complete public C ABI and bit-exact behaviour.
//!
//! Every non-`static` C function is re-exported here with `#[unsafe(no_mangle)]`
//! and `extern "C"`, using the identical signature so the SysV struct-passing
//! classification matches the C build exactly.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(unused_assignments)]
#![allow(dead_code)]

use std::os::raw::{c_int, c_void};

// `<float.h>` constants exactly as spelled in the C source:
//   3.40282346638528859811704183484516925e+38F  == FLT_MAX
//   1.19209289550781250000000000000000000e-7F   == FLT_EPSILON
const FLT_MAX: f32 = f32::MAX;
const FLT_EPSILON: f32 = f32::EPSILON;

// ---------------------------------------------------------------------------
// include/lib.h
// ---------------------------------------------------------------------------

// typedef enum { C2_TYPE_CAPSULE, C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_POLY } C2_TYPE;
pub const C2_TYPE_CAPSULE: c_int = 0;
pub const C2_TYPE_CIRCLE: c_int = 1;
pub const C2_TYPE_AABB: c_int = 2;
pub const C2_TYPE_POLY: c_int = 3;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Manifold {
    pub count: c_int,
    pub depths: [f32; 2],
    pub contact_points: [c2v; 2],
    pub n: c2v,
}

// ---------------------------------------------------------------------------
// src/lib.c internal types
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2h {
    pub n: c2v,
    pub d: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
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

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Poly {
    pub count: c_int,
    pub verts: [c2v; 8],
    pub norms: [c2v; 8],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

/// `typedef struct { float radius; int count; c2v verts[8]; } c2Proxy;`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
}

/// `typedef struct { c2v sA; c2v sB; c2v p; float u; int iA; int iB; } c2sv;`
#[repr(C)]
#[derive(Copy, Clone)]
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
/// The C code relies on `a`, `b`, `c`, `d` being contiguous (`c2sv* verts = &s.a`),
/// which this `#[repr(C)]` layout reproduces.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Simplex {
    pub a: c2sv,
    pub b: c2sv,
    pub c: c2sv,
    pub d: c2sv,
    pub div: f32,
    pub count: c_int,
}

const ZERO_V: c2v = c2v { x: 0.0, y: 0.0 };

/// C's unary `-` on a float is an IEEE-754 sign-bit flip (gcc emits `xorps`),
/// which is also applied to NaN payloads.  Expressed as an explicit bit flip so
/// that LLVM cannot rewrite `(-a) * b` into `-(a * b)`, a transform that would
/// change the sign bit of a resulting NaN.
#[inline(always)]
fn fneg(x: f32) -> f32 {
    // `black_box` is the barrier: without it LLVM recognises the sign flip and
    // sinks it into the neighbouring arithmetic (e.g. `(-a) * b + c` becomes
    // `c - a * b`), which changes the sign bit of a NaN result.
    std::hint::black_box(f32::from_bits(x.to_bits() ^ 0x8000_0000))
}

/// SSE quiets a signalling NaN operand before propagating it: the quiet bit
/// (mantissa MSB) is set while the sign and the rest of the payload survive.
/// Idempotent for NaNs that are already quiet.
#[inline(always)]
fn quiet_nan(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

/// x86 SSE scalar ops select which NaN propagates by *operand position*:
/// `op dst, src` yields `quiet(dst)` when `dst` is NaN, else `quiet(src)` when
/// `src` is NaN.  gcc's register allocation therefore fixes which NaN a
/// mixed-NaN expression returns; these helpers let the translation name that
/// choice explicitly.  For non-NaN operands they are exactly the plain
/// IEEE-754 operation.
#[inline(always)]
fn x86_mul(dst: f32, src: f32) -> f32 {
    let r = if dst.is_nan() {
        quiet_nan(dst)
    } else if src.is_nan() {
        quiet_nan(src)
    } else {
        dst * src
    };
    std::hint::black_box(r)
}

#[inline(always)]
fn x86_add(dst: f32, src: f32) -> f32 {
    let r = if dst.is_nan() {
        quiet_nan(dst)
    } else if src.is_nan() {
        quiet_nan(src)
    } else {
        dst + src
    };
    std::hint::black_box(r)
}

#[inline(always)]
fn x86_sub(dst: f32, src: f32) -> f32 {
    let r = if dst.is_nan() {
        quiet_nan(dst)
    } else if src.is_nan() {
        quiet_nan(src)
    } else {
        dst - src
    };
    std::hint::black_box(r)
}

/// Plain IEEE-754 division as gcc emits it (`divss dst, src`): same NaN
/// operand-position rule as the other SSE scalar ops.
#[inline(always)]
fn x86_div(dst: f32, src: f32) -> f32 {
    let r = if dst.is_nan() {
        quiet_nan(dst)
    } else if src.is_nan() {
        quiet_nan(src)
    } else {
        dst / src
    };
    std::hint::black_box(r)
}

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

// ---------------------------------------------------------------------------
// Vector helpers
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    let mut a: c2v = ZERO_V;
    a.x = x;
    a.y = y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(a: c2v, b: f32) -> c2v {
    // gcc: mulss dst=a.x,src=b ; mulss dst=a.y,src=b
    let mut a = a;
    a.x = x86_mul(a.x, b);
    a.y = x86_mul(a.y, b);
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
    // gcc: subss dst=a.x,src=b.x ; subss dst=a.y,src=b.y
    let mut a = a;
    a.x = x86_sub(a.x, b.x);
    a.y = x86_sub(a.y, b.y);
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    // gcc: mulss dst=a.x,src=b.x ; mulss dst=b.y,src=a.y ; addss dst=p2,src=p1
    let p1 = x86_mul(a.x, b.x);
    let p2 = x86_mul(b.y, a.y);
    x86_add(p2, p1)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dist(h: c2h, p: c2v) -> f32 {
    c2Dot(h.n, p) - h.d
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2PlaneAt(p: *const c2Poly, i: c_int) -> c2h {
    let mut h = c2h { n: ZERO_V, d: 0.0 };
    let norms = (*p).norms.as_ptr();
    let verts = (*p).verts.as_ptr();
    h.n = *norms.offset(i as isize);
    h.d = c2Dot(*norms.offset(i as isize), *verts.offset(i as isize));
    h
}

#[unsafe(no_mangle)]
pub extern "C" fn c2RotIdentity() -> c2r {
    let mut r = c2r { c: 0.0, s: 0.0 };
    r.c = 1.0f32;
    r.s = 0.0;
    r
}

#[unsafe(no_mangle)]
pub extern "C" fn c2xIdentity() -> c2x {
    let mut x = c2x {
        p: ZERO_V,
        r: c2r { c: 0.0, s: 0.0 },
    };
    x.p = c2V(0.0, 0.0);
    x.r = c2RotIdentity();
    x
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2BBVerts(out: *mut c2v, bb: *mut c2AABB) {
    *out.offset(0) = (*bb).min;
    *out.offset(1) = c2V((*bb).max.x, (*bb).min.y);
    *out.offset(2) = (*bb).max;
    *out.offset(3) = c2V((*bb).min.x, (*bb).max.y);
}

/// NOTE: the C source has **no** `C2_TYPE_POLY` case, so a poly proxy is left
/// completely untouched (uninitialised in C).  Reproduced faithfully: nothing is
/// written for any other type value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(shape: *const c_void, type_: c_int, p: *mut c2Proxy) {
    match type_ {
        C2_TYPE_CIRCLE => {
            let c = shape as *const c2Circle;
            (*p).radius = (*c).r;
            (*p).count = 1;
            (*p).verts[0] = (*c).p;
        }
        C2_TYPE_AABB => {
            let bb = shape as *mut c2AABB;
            (*p).radius = 0.0;
            (*p).count = 4;
            let verts = (*p).verts.as_mut_ptr();
            c2BBVerts(verts, bb);
        }
        C2_TYPE_CAPSULE => {
            let c = shape as *const c2Capsule;
            (*p).radius = (*c).r;
            (*p).count = 2;
            (*p).verts[0] = (*c).a;
            (*p).verts[1] = (*c).b;
        }
        _ => {}
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> f32 {
    c2Dot(a, a).sqrt()
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: c2v, b: c2v) -> f32 {
    // gcc: mulss dst=b.y,src=a.x ; mulss dst=b.x,src=a.y ; subss dst=p1,src=p2
    let p1 = x86_mul(b.y, a.x);
    let p2 = x86_mul(b.x, a.y);
    x86_sub(p1, p2)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJKSimplexMetric(s: *mut c2Simplex) -> f32 {
    match (*s).count {
        2 => c2Len(c2Sub((*s).b.p, (*s).a.p)),
        3 => c2Det2(c2Sub((*s).b.p, (*s).a.p), c2Sub((*s).c.p, (*s).a.p)),
        // `default:` and `case 1:` fall together in the C switch.
        _ => 0.0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    // gcc -O0 emits, for `a.c * b.x - a.s * b.y`:
    //   mulss dst=b.x,src=a.c ; mulss dst=b.y,src=a.s ; subss dst=p1,src=p2
    // and for `a.s * b.x + a.c * b.y`:
    //   mulss dst=a.s,src=b.x ; mulss dst=b.y,src=a.c ; addss dst=p1,src=p2
    c2V(
        x86_sub(x86_mul(b.x, a.c), x86_mul(b.y, a.s)),
        x86_add(x86_mul(a.s, b.x), x86_mul(b.y, a.c)),
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    // gcc -O0 emits, for `a.c * b.x + a.s * b.y`:
    //   mulss dst=a.c,src=b.x ; mulss dst=b.y,src=a.s ; addss dst=p1,src=p2
    // and for `-a.s * b.x + a.c * b.y`:
    //   xorps (negate) ; mulss dst=-a.s,src=b.x ; mulss dst=b.y,src=a.c ;
    //   addss dst=p1,src=p2
    c2V(
        x86_add(x86_mul(a.c, b.x), x86_mul(b.y, a.s)),
        x86_add(x86_mul(fneg(a.s), b.x), x86_mul(b.y, a.c)),
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Add(a: c2v, b: c2v) -> c2v {
    // gcc: addss dst=b.x,src=a.x ; addss dst=b.y,src=a.y
    let mut a = a;
    a.x = x86_add(b.x, a.x);
    a.y = x86_add(b.y, a.y);
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulxv(a: c2x, b: c2v) -> c2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulxvT(a: c2x, b: c2v) -> c2v {
    c2MulrvT(a.r, c2Sub(b, a.p))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Intersect(a: c2v, b: c2v, da: f32, db: f32) -> c2v {
    c2Add(a, c2Mulvs(c2Sub(b, a), da / (da - db)))
}

/// `static int c2Clip(c2v *seg, c2h h)` — not exported.
///
/// The C version leaves `out[]` (partly) uninitialised when it returns < 2; the
/// two callers always bail out in that case without inspecting `seg`, so the
/// zero-initialisation used here is unobservable.
unsafe fn c2Clip(seg: *mut c2v, h: c2h) -> c_int {
    let mut out = [ZERO_V; 4];
    let mut sp: usize = 0;
    let d0: f32;
    let d1: f32;
    d0 = c2Dist(h, *seg.offset(0));
    if d0 < 0.0 {
        out[sp] = *seg.offset(0);
        sp += 1;
    }
    d1 = c2Dist(h, *seg.offset(1));
    if d1 < 0.0 {
        out[sp] = *seg.offset(1);
        sp += 1;
    }
    if d0 == 0.0 && d1 == 0.0 {
        out[sp] = *seg.offset(0);
        sp += 1;
        out[sp] = *seg.offset(1);
        sp += 1;
    } else if d0 * d1 <= 0.0 {
        out[sp] = c2Intersect(*seg.offset(0), *seg.offset(1), d0, d1);
        sp += 1;
    }
    *seg.offset(0) = out[0];
    *seg.offset(1) = out[1];
    sp as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: c2v, b: f32) -> c2v {
    // gcc -O0: `movss $1.0,%xmm0 ; divss b,%xmm0` — dst is the 1.0 constant.
    c2Mulvs(a, x86_div(1.0f32, b))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Neg(a: c2v) -> c2v {
    c2V(fneg(a.x), fneg(a.y))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v {
    let mut b: c2v = ZERO_V;
    b.x = a.y;
    b.y = fneg(a.x);
    b
}

/// `static int c2SidePlanes(c2v *seg, c2v ra, c2v rb, c2h *h)` — not exported.
unsafe fn c2SidePlanes(seg: *mut c2v, ra: c2v, rb: c2v, h: *mut c2h) -> c_int {
    let in_ = c2Norm(c2Sub(rb, ra));
    let left = c2h {
        n: c2Neg(in_),
        d: c2Dot(c2Neg(in_), ra),
    };
    let right = c2h {
        n: in_,
        d: c2Dot(in_, rb),
    };
    if c2Clip(seg, left) < 2 {
        return 0;
    }
    if c2Clip(seg, right) < 2 {
        return 0;
    }
    if !h.is_null() {
        (*h).n = c2CCW90(in_);
        (*h).d = c2Dot(c2CCW90(in_), ra);
    }
    1
}

/// `static int c2SidePlanesFromPoly(...)` — not exported.
unsafe fn c2SidePlanesFromPoly(
    seg: *mut c2v,
    x: c2x,
    p: *const c2Poly,
    e: c_int,
    h: *mut c2h,
) -> c_int {
    let verts = (*p).verts.as_ptr();
    let ra = c2Mulxv(x, *verts.offset(e as isize));
    let next = if e + 1 == (*p).count { 0 } else { e + 1 };
    let rb = c2Mulxv(x, *verts.offset(next as isize));
    c2SidePlanes(seg, ra, rb, h)
}

// ---------------------------------------------------------------------------
// GJK
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c22(s: *mut c2Simplex) {
    let a = (*s).a.p;
    let b = (*s).b.p;
    let u = c2Dot(b, c2Sub(b, a));
    let v = c2Dot(a, c2Sub(a, b));
    if v <= 0.0 {
        (*s).a.u = 1.0f32;
        (*s).div = 1.0f32;
        (*s).count = 1;
    } else if u <= 0.0 {
        (*s).a = (*s).b;
        (*s).a.u = 1.0f32;
        (*s).div = 1.0f32;
        (*s).count = 1;
    } else {
        (*s).a.u = u;
        (*s).b.u = v;
        (*s).div = x86_add(u, v); // gcc: addss dst=u,src=v
        (*s).count = 2;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c23(s: *mut c2Simplex) {
    let a = (*s).a.p;
    let b = (*s).b.p;
    let c = (*s).c.p;
    let uAB = c2Dot(b, c2Sub(b, a));
    let vAB = c2Dot(a, c2Sub(a, b));
    let uBC = c2Dot(c, c2Sub(c, b));
    let vBC = c2Dot(b, c2Sub(b, c));
    let uCA = c2Dot(a, c2Sub(a, c));
    let vCA = c2Dot(c, c2Sub(c, a));
    let area = c2Det2(c2Sub(b, a), c2Sub(c, a));
    // gcc: mulss dst=<c2Det2 result>, src=area
    let uABC = x86_mul(c2Det2(b, c), area);
    let vABC = x86_mul(c2Det2(c, a), area);
    let wABC = x86_mul(c2Det2(a, b), area);
    if vAB <= 0.0 && uCA <= 0.0 {
        (*s).a.u = 1.0f32;
        (*s).div = 1.0f32;
        (*s).count = 1;
    } else if uAB <= 0.0 && vBC <= 0.0 {
        (*s).a = (*s).b;
        (*s).a.u = 1.0f32;
        (*s).div = 1.0f32;
        (*s).count = 1;
    } else if uBC <= 0.0 && vCA <= 0.0 {
        (*s).a = (*s).c;
        (*s).a.u = 1.0f32;
        (*s).div = 1.0f32;
        (*s).count = 1;
    } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
        (*s).a.u = uAB;
        (*s).b.u = vAB;
        (*s).div = x86_add(uAB, vAB);
        (*s).count = 2;
    } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
        (*s).a = (*s).b;
        (*s).b = (*s).c;
        (*s).a.u = uBC;
        (*s).b.u = vBC;
        (*s).div = x86_add(uBC, vBC);
        (*s).count = 2;
    } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
        (*s).b = (*s).a;
        (*s).a = (*s).c;
        (*s).a.u = uCA;
        (*s).b.u = vCA;
        (*s).div = x86_add(uCA, vCA);
        (*s).count = 2;
    } else {
        (*s).a.u = uABC;
        (*s).b.u = vABC;
        (*s).c.u = wABC;
        (*s).div = x86_add(x86_add(uABC, vABC), wABC);
        (*s).count = 3;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v {
    let mut b: c2v = ZERO_V;
    b.x = fneg(a.y);
    b.y = a.x;
    b
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2D(s: *mut c2Simplex) -> c2v {
    match (*s).count {
        1 => c2Neg((*s).a.p),
        2 => {
            let ab = c2Sub((*s).b.p, (*s).a.p);
            if c2Det2(ab, c2Neg((*s).a.p)) > 0.0 {
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
    let mut dmax = c2Dot(*verts.offset(0), d);
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
    let den = 1.0f32 / (*s).div;
    match (*s).count {
        1 => {
            *a = (*s).a.sA;
            *b = (*s).a.sB;
        }
        2 => {
            *a = c2Add(
                c2Mulvs((*s).a.sA, x86_mul((*s).a.u, den)),
                c2Mulvs((*s).b.sA, x86_mul((*s).b.u, den)),
            );
            *b = c2Add(
                c2Mulvs((*s).a.sB, x86_mul((*s).a.u, den)),
                c2Mulvs((*s).b.sB, x86_mul((*s).b.u, den)),
            );
        }
        3 => {
            *a = c2Add(
                c2Add(
                    c2Mulvs((*s).a.sA, x86_mul((*s).a.u, den)),
                    c2Mulvs((*s).b.sA, x86_mul((*s).b.u, den)),
                ),
                c2Mulvs((*s).c.sA, x86_mul((*s).c.u, den)),
            );
            *b = c2Add(
                c2Add(
                    c2Mulvs((*s).a.sB, x86_mul((*s).a.u, den)),
                    c2Mulvs((*s).b.sB, x86_mul((*s).b.u, den)),
                ),
                c2Mulvs((*s).c.sB, x86_mul((*s).c.u, den)),
            );
        }
        _ => {
            *a = c2V(0.0, 0.0);
            *b = c2V(0.0, 0.0);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2L(s: *mut c2Simplex) -> c2v {
    let den = 1.0f32 / (*s).div;
    match (*s).count {
        1 => (*s).a.p,
        2 => c2Add(
            c2Mulvs((*s).a.p, x86_mul((*s).a.u, den)),
            c2Mulvs((*s).b.p, x86_mul((*s).b.u, den)),
        ),
        _ => c2V(0.0, 0.0),
    }
}

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
    // In C these two proxies are uninitialised locals; `c2MakeProxy` has no
    // `C2_TYPE_POLY` case, so for a poly the proxy keeps whatever the stack held
    // (radius 0 / count 0 / verts {0,0} on a freshly used stack).
    let mut pA: c2Proxy = std::mem::zeroed();
    let mut pB: c2Proxy = std::mem::zeroed();
    c2MakeProxy(A, typeA, &mut pA);
    c2MakeProxy(B, typeB, &mut pB);
    let mut s: c2Simplex = std::mem::zeroed();
    // `c2sv* verts = &s.a;` — indexes a, b, c, d contiguously.
    let verts: *mut c2sv = &mut s as *mut c2Simplex as *mut c2sv;
    let mut cache_was_read: c_int = 0;
    if !cache.is_null() {
        let cache_was_good: c_int = ((*cache).count != 0) as c_int;
        if cache_was_good != 0 {
            let mut i: c_int = 0;
            while i < (*cache).count {
                let iA = *(*cache).iA.as_ptr().offset(i as isize);
                let iB = *(*cache).iB.as_ptr().offset(i as isize);
                let sA = c2Mulxv(ax, *pA.verts.as_ptr().offset(iA as isize));
                let sB = c2Mulxv(bx, *pB.verts.as_ptr().offset(iB as isize));
                let v: *mut c2sv = verts.offset(i as isize);
                (*v).iA = iA;
                (*v).sA = sA;
                (*v).iB = iB;
                (*v).sB = sB;
                (*v).p = c2Sub((*v).sB, (*v).sA);
                (*v).u = 0.0;
                i += 1;
            }
            s.count = (*cache).count;
            s.div = (*cache).div;
            let metric_old = (*cache).metric;
            let metric = c2GJKSimplexMetric(&mut s);
            let min_metric = if metric < metric_old { metric } else { metric_old };
            let max_metric = if metric > metric_old { metric } else { metric_old };
            if !(min_metric < max_metric * 2.0f32 && metric < -1.0e8f32) {
                cache_was_read = 1;
            }
        }
    }
    if cache_was_read == 0 {
        s.a.iA = 0;
        s.a.iB = 0;
        s.a.sA = c2Mulxv(ax, pA.verts[0]);
        s.a.sB = c2Mulxv(bx, pB.verts[0]);
        s.a.p = c2Sub(s.a.sB, s.a.sA);
        s.a.u = 1.0f32;
        s.div = 1.0f32;
        s.count = 1;
    }
    let mut saveA: [c_int; 3] = [0; 3];
    let mut saveB: [c_int; 3] = [0; 3];
    let mut save_count: c_int = 0;
    let mut d0: f32 = FLT_MAX;
    let mut d1: f32 = FLT_MAX;
    let mut iter: c_int = 0;
    let mut hit: c_int = 0;
    while iter < 20 {
        save_count = s.count;
        {
            let mut i: c_int = 0;
            while i < save_count {
                *saveA.as_mut_ptr().offset(i as isize) = (*verts.offset(i as isize)).iA;
                *saveB.as_mut_ptr().offset(i as isize) = (*verts.offset(i as isize)).iB;
                i += 1;
            }
        }
        match s.count {
            1 => {}
            2 => c22(&mut s),
            3 => c23(&mut s),
            _ => {}
        }
        if s.count == 3 {
            hit = 1;
            break;
        }
        let p = c2L(&mut s);
        d1 = c2Dot(p, p);
        if d1 > d0 {
            break;
        }
        d0 = d1;
        let d = c2D(&mut s);
        if c2Dot(d, d) < FLT_EPSILON * FLT_EPSILON {
            break;
        }
        let iA = c2Support(pA.verts.as_ptr(), pA.count, c2MulrvT(ax.r, c2Neg(d)));
        let sA = c2Mulxv(ax, *pA.verts.as_ptr().offset(iA as isize));
        let iB = c2Support(pB.verts.as_ptr(), pB.count, c2MulrvT(bx.r, d));
        let sB = c2Mulxv(bx, *pB.verts.as_ptr().offset(iB as isize));
        let v: *mut c2sv = verts.offset(s.count as isize);
        (*v).iA = iA;
        (*v).sA = sA;
        (*v).iB = iB;
        (*v).sB = sB;
        (*v).p = c2Sub((*v).sB, (*v).sA);
        let mut dup: c_int = 0;
        {
            let mut i: c_int = 0;
            while i < save_count {
                if iA == *saveA.as_ptr().offset(i as isize)
                    && iB == *saveB.as_ptr().offset(i as isize)
                {
                    dup = 1;
                    break;
                }
                i += 1;
            }
        }
        if dup != 0 {
            break;
        }
        s.count += 1;
        iter += 1;
    }
    let mut a: c2v = ZERO_V;
    let mut b: c2v = ZERO_V;
    c2Witness(&mut s, &mut a, &mut b);
    let mut dist = c2Len(c2Sub(a, b));
    if hit != 0 {
        a = b;
        dist = 0.0;
    } else if use_radius != 0 {
        let rA = pA.radius;
        let rB = pB.radius;
        if dist > rA + rB && dist > FLT_EPSILON {
            dist -= rA + rB;
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
        (*cache).metric = c2GJKSimplexMetric(&mut s);
        (*cache).count = s.count;
        let mut i: c_int = 0;
        while i < s.count {
            let v: *mut c2sv = verts.offset(i as isize);
            *(*cache).iA.as_mut_ptr().offset(i as isize) = (*v).iA;
            *(*cache).iB.as_mut_ptr().offset(i as isize) = (*v).iB;
            i += 1;
        }
        (*cache).div = s.div;
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
    let _ = d1;
    let _ = save_count;
    dist
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Absv(a: c2v) -> c2v {
    c2V(
        if a.x < 0.0 { fneg(a.x) } else { a.x },
        if a.y < 0.0 { fneg(a.y) } else { a.y },
    )
}

// ---------------------------------------------------------------------------
// Manifold generation
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CircletoCircleManifold(A: c2Circle, B: c2Circle, m: *mut c2Manifold) {
    (*m).count = 0;
    let d = c2Sub(B.p, A.p);
    let d2 = c2Dot(d, d);
    let r = A.r + B.r;
    if d2 < r * r {
        let l = d2.sqrt();
        let n = if l != 0.0 {
            c2Mulvs(d, 1.0f32 / l)
        } else {
            c2V(0.0, 1.0f32)
        };
        (*m).count = 1;
        (*m).depths[0] = r - l;
        (*m).contact_points[0] = c2Sub(B.p, c2Mulvs(n, B.r));
        (*m).n = n;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CircletoAABBManifold(A: c2Circle, B: c2AABB, m: *mut c2Manifold) {
    (*m).count = 0;
    let L = c2Clampv(A.p, B.min, B.max);
    let ab = c2Sub(L, A.p);
    let d2 = c2Dot(ab, ab);
    let r2 = A.r * A.r;
    if d2 < r2 {
        if d2 != 0.0 {
            let d = d2.sqrt();
            let n = c2Norm(ab);
            (*m).count = 1;
            (*m).depths[0] = A.r - d;
            (*m).contact_points[0] = c2Add(A.p, c2Mulvs(n, d));
            (*m).n = n;
        } else {
            let mid = c2Mulvs(c2Add(B.min, B.max), 0.5f32);
            let e = c2Mulvs(c2Sub(B.max, B.min), 0.5f32);
            let d = c2Sub(A.p, mid);
            let abs_d = c2Absv(d);
            let x_overlap = e.x - abs_d.x;
            let y_overlap = e.y - abs_d.y;
            let depth: f32;
            let mut n: c2v;
            if x_overlap < y_overlap {
                depth = x_overlap;
                n = c2V(1.0f32, 0.0);
                n = c2Mulvs(n, if d.x < 0.0 { 1.0f32 } else { -1.0f32 });
            } else {
                depth = y_overlap;
                n = c2V(0.0, 1.0f32);
                n = c2Mulvs(n, if d.y < 0.0 { 1.0f32 } else { -1.0f32 });
            }
            (*m).count = 1;
            (*m).depths[0] = A.r + depth;
            (*m).contact_points[0] = c2Sub(A.p, c2Mulvs(n, depth));
            (*m).n = n;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CircletoCapsuleManifold(A: c2Circle, B: c2Capsule, m: *mut c2Manifold) {
    (*m).count = 0;
    let mut a: c2v = ZERO_V;
    let mut b: c2v = ZERO_V;
    let r = A.r + B.r;
    let d = c2GJK(
        &A as *const c2Circle as *const c_void,
        C2_TYPE_CIRCLE,
        std::ptr::null(),
        &B as *const c2Capsule as *const c_void,
        C2_TYPE_CAPSULE,
        std::ptr::null(),
        &mut a,
        &mut b,
        0,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
    );
    if d < r {
        let n: c2v;
        if d == 0.0 {
            n = c2Norm(c2Skew(c2Sub(B.b, B.a)));
        } else {
            n = c2Norm(c2Sub(b, a));
        }
        (*m).count = 1;
        (*m).depths[0] = r - d;
        (*m).contact_points[0] = c2Sub(b, c2Mulvs(n, B.r));
        (*m).n = n;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2AABBtoAABBManifold(A: c2AABB, B: c2AABB, m: *mut c2Manifold) {
    (*m).count = 0;
    let mid_a = c2Mulvs(c2Add(A.min, A.max), 0.5f32);
    let mid_b = c2Mulvs(c2Add(B.min, B.max), 0.5f32);
    let eA = c2Absv(c2Mulvs(c2Sub(A.max, A.min), 0.5f32));
    let eB = c2Absv(c2Mulvs(c2Sub(B.max, B.min), 0.5f32));
    let d = c2Sub(mid_b, mid_a);
    // gcc: addss dst=eA.x,src=eB.x ; subss dst=<sum>,src=|d.x|
    let dx = x86_sub(
        x86_add(eA.x, eB.x),
        if d.x < 0.0 { fneg(d.x) } else { d.x },
    );
    if dx < 0.0 {
        return;
    }
    let dy = x86_sub(
        x86_add(eA.y, eB.y),
        if d.y < 0.0 { fneg(d.y) } else { d.y },
    );
    if dy < 0.0 {
        return;
    }
    let n: c2v;
    let depth: f32;
    let p: c2v;
    if dx < dy {
        depth = dx;
        if d.x < 0.0 {
            n = c2V(-1.0f32, 0.0);
            p = c2Sub(mid_a, c2V(eA.x, 0.0));
        } else {
            n = c2V(1.0f32, 0.0);
            p = c2Add(mid_a, c2V(eA.x, 0.0));
        }
    } else {
        depth = dy;
        if d.y < 0.0 {
            n = c2V(0.0, -1.0f32);
            p = c2Sub(mid_a, c2V(0.0, eA.y));
        } else {
            n = c2V(0.0, 1.0f32);
            p = c2Add(mid_a, c2V(0.0, eA.y));
        }
    }
    (*m).count = 1;
    (*m).contact_points[0] = p;
    (*m).depths[0] = depth;
    (*m).n = n;
}

/// `static void c2KeepDeep(c2v *seg, c2h h, c2Manifold *m)` — not exported.
unsafe fn c2KeepDeep(seg: *mut c2v, h: c2h, m: *mut c2Manifold) {
    let mut cp: c_int = 0;
    let mut i: c_int = 0;
    while i < 2 {
        let p = *seg.offset(i as isize);
        let d = c2Dist(h, p);
        if d <= 0.0 {
            *(*m).contact_points.as_mut_ptr().offset(cp as isize) = p;
            *(*m).depths.as_mut_ptr().offset(cp as isize) = fneg(d);
            cp += 1;
        }
        i += 1;
    }
    (*m).count = cp;
    (*m).n = h.n;
}

/// `static void c2Incident(...)` — not exported.
unsafe fn c2Incident(incident: *mut c2v, ip: *const c2Poly, ix: c2x, rn_in_incident_space: c2v) {
    let mut index: c_int = !0;
    let mut min_dot: f32 = FLT_MAX;
    let norms = (*ip).norms.as_ptr();
    let verts = (*ip).verts.as_ptr();
    let mut i: c_int = 0;
    while i < (*ip).count {
        let dot = c2Dot(rn_in_incident_space, *norms.offset(i as isize));
        if dot < min_dot {
            min_dot = dot;
            index = i;
        }
        i += 1;
    }
    *incident.offset(0) = c2Mulxv(ix, *verts.offset(index as isize));
    let next = if index + 1 == (*ip).count { 0 } else { index + 1 };
    *incident.offset(1) = c2Mulxv(ix, *verts.offset(next as isize));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CapsuletoPolyManifold(
    A: c2Capsule,
    B: *const c2Poly,
    bx_ptr: *const c2x,
    m: *mut c2Manifold,
) {
    (*m).count = 0;
    let mut a: c2v = ZERO_V;
    let mut b: c2v = ZERO_V;
    let d = c2GJK(
        &A as *const c2Capsule as *const c_void,
        C2_TYPE_CAPSULE,
        std::ptr::null(),
        B as *const c_void,
        C2_TYPE_POLY,
        bx_ptr,
        &mut a,
        &mut b,
        0,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
    );
    if d < 1.0e-6f32 {
        let bx = if !bx_ptr.is_null() {
            *bx_ptr
        } else {
            c2xIdentity()
        };
        let mut A_in_B = c2Capsule {
            a: ZERO_V,
            b: ZERO_V,
            r: 0.0,
        };
        A_in_B.a = c2MulxvT(bx, A.a);
        A_in_B.b = c2MulxvT(bx, A.b);
        let ab = c2Norm(c2Sub(A_in_B.a, A_in_B.b));
        let mut ab_h0 = c2h { n: ZERO_V, d: 0.0 };
        ab_h0.n = c2CCW90(ab);
        ab_h0.d = c2Dot(A_in_B.a, ab_h0.n);
        let bverts = (*B).verts.as_ptr();
        let v0 = c2Support(bverts, (*B).count, c2Neg(ab_h0.n));
        let s0 = c2Dist(ab_h0, *bverts.offset(v0 as isize));
        let mut ab_h1 = c2h { n: ZERO_V, d: 0.0 };
        ab_h1.n = c2Skew(ab);
        ab_h1.d = c2Dot(A_in_B.a, ab_h1.n);
        let v1 = c2Support(bverts, (*B).count, c2Neg(ab_h1.n));
        let s1 = c2Dist(ab_h1, *bverts.offset(v1 as isize));
        let mut index: c_int = !0;
        let mut sep: f32 = -FLT_MAX;
        let mut code: c_int = 0;
        let mut i: c_int = 0;
        while i < (*B).count {
            let h = c2PlaneAt(B, i);
            let da = c2Dot(A_in_B.a, c2Neg(h.n));
            let db = c2Dot(A_in_B.b, c2Neg(h.n));
            let d: f32;
            if da > db {
                d = c2Dist(h, A_in_B.a);
            } else {
                d = c2Dist(h, A_in_B.b);
            }
            if d > sep {
                sep = d;
                index = i;
            }
            i += 1;
        }
        if s0 > sep {
            sep = s0;
            index = v0;
            code = 1;
        }
        if s1 > sep {
            sep = s1;
            index = v1;
            code = 2;
        }
        match code {
            0 => {
                let mut seg: [c2v; 2] = [A.a, A.b];
                let mut h = c2h { n: ZERO_V, d: 0.0 };
                if c2SidePlanesFromPoly(seg.as_mut_ptr(), bx, B, index, &mut h) == 0 {
                    return;
                }
                c2KeepDeep(seg.as_mut_ptr(), h, m);
                (*m).n = c2Neg((*m).n);
            }
            1 => {
                let mut incident: [c2v; 2] = [ZERO_V; 2];
                c2Incident(incident.as_mut_ptr(), B, bx, ab_h0.n);
                let mut h = c2h { n: ZERO_V, d: 0.0 };
                if c2SidePlanes(incident.as_mut_ptr(), A_in_B.b, A_in_B.a, &mut h) == 0 {
                    return;
                }
                c2KeepDeep(incident.as_mut_ptr(), h, m);
            }
            2 => {
                let mut incident: [c2v; 2] = [ZERO_V; 2];
                c2Incident(incident.as_mut_ptr(), B, bx, ab_h1.n);
                let mut h = c2h { n: ZERO_V, d: 0.0 };
                if c2SidePlanes(incident.as_mut_ptr(), A_in_B.a, A_in_B.b, &mut h) == 0 {
                    return;
                }
                c2KeepDeep(incident.as_mut_ptr(), h, m);
            }
            _ => {
                return;
            }
        }
        let mut i: c_int = 0;
        while i < (*m).count {
            *(*m).depths.as_mut_ptr().offset(i as isize) += A.r;
            i += 1;
        }
    } else if d < A.r {
        (*m).count = 1;
        (*m).n = c2Norm(c2Sub(b, a));
        (*m).contact_points[0] = c2Add(a, c2Mulvs((*m).n, A.r));
        (*m).depths[0] = A.r - d;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Norms(verts: *mut c2v, norms: *mut c2v, count: c_int) {
    let mut i: c_int = 0;
    while i < count {
        let a = i;
        let b = if i + 1 < count { i + 1 } else { 0 };
        let e = c2Sub(*verts.offset(b as isize), *verts.offset(a as isize));
        *norms.offset(i as isize) = c2Norm(c2CCW90(e));
        i += 1;
    }
}

/// Mirrors gcc's stack frame for `c2AABBtoCapsuleManifold`, where the spilled
/// `A` parameter sits immediately below the `c2Poly` local (`A` at `-0xb0(%rbp)`,
/// `p` at `-0xa0(%rbp)`).  That matters because a degenerate AABB makes every
/// polygon normal NaN, which leaves `c2Incident`'s `index` at `~0 == -1` and
/// makes it read `p.verts[-1]`, i.e. the 8 bytes `{A.max.y, p.count}`.
#[repr(C)]
struct AabbCapsuleFrame {
    A: c2AABB,
    p: c2Poly,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2AABBtoCapsuleManifold(A: c2AABB, B: c2Capsule, m: *mut c2Manifold) {
    let mut fr: AabbCapsuleFrame = std::mem::zeroed();
    fr.A = A;
    (*m).count = 0;
    c2BBVerts(fr.p.verts.as_mut_ptr(), &mut fr.A);
    fr.p.count = 4;
    let vp = fr.p.verts.as_mut_ptr();
    let np = fr.p.norms.as_mut_ptr();
    c2Norms(vp, np, 4);
    c2CapsuletoPolyManifold(B, &fr.p, std::ptr::null(), m);
    (*m).n = c2Neg((*m).n);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CapsuletoCapsuleManifold(A: c2Capsule, B: c2Capsule, m: *mut c2Manifold) {
    (*m).count = 0;
    let mut a: c2v = ZERO_V;
    let mut b: c2v = ZERO_V;
    let r = A.r + B.r;
    let d = c2GJK(
        &A as *const c2Capsule as *const c_void,
        C2_TYPE_CAPSULE,
        std::ptr::null(),
        &B as *const c2Capsule as *const c_void,
        C2_TYPE_CAPSULE,
        std::ptr::null(),
        &mut a,
        &mut b,
        0,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
    );
    if d < r {
        let n: c2v;
        if d == 0.0 {
            n = c2Norm(c2Skew(c2Sub(A.b, A.a)));
        } else {
            n = c2Norm(c2Sub(b, a));
        }
        (*m).count = 1;
        (*m).depths[0] = r - d;
        (*m).contact_points[0] = c2Sub(b, c2Mulvs(n, B.r));
        (*m).n = n;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Collide(
    A: *const c_void,
    typeA: c_int,
    B: *const c_void,
    typeB: c_int,
    m: *mut c2Manifold,
) {
    (*m).count = 0;
    match typeA {
        C2_TYPE_CIRCLE => match typeB {
            C2_TYPE_CIRCLE => {
                c2CircletoCircleManifold(
                    std::ptr::read_unaligned(A as *const c2Circle),
                    std::ptr::read_unaligned(B as *const c2Circle),
                    m,
                );
            }
            C2_TYPE_AABB => {
                c2CircletoAABBManifold(
                    std::ptr::read_unaligned(A as *const c2Circle),
                    std::ptr::read_unaligned(B as *const c2AABB),
                    m,
                );
            }
            C2_TYPE_CAPSULE => {
                c2CircletoCapsuleManifold(
                    std::ptr::read_unaligned(A as *const c2Circle),
                    std::ptr::read_unaligned(B as *const c2Capsule),
                    m,
                );
            }
            _ => {}
        },
        C2_TYPE_AABB => match typeB {
            C2_TYPE_CIRCLE => {
                c2CircletoAABBManifold(
                    std::ptr::read_unaligned(B as *const c2Circle),
                    std::ptr::read_unaligned(A as *const c2AABB),
                    m,
                );
                (*m).n = c2Neg((*m).n);
            }
            C2_TYPE_AABB => {
                c2AABBtoAABBManifold(
                    std::ptr::read_unaligned(A as *const c2AABB),
                    std::ptr::read_unaligned(B as *const c2AABB),
                    m,
                );
            }
            C2_TYPE_CAPSULE => {
                c2AABBtoCapsuleManifold(
                    std::ptr::read_unaligned(A as *const c2AABB),
                    std::ptr::read_unaligned(B as *const c2Capsule),
                    m,
                );
            }
            _ => {}
        },
        C2_TYPE_CAPSULE => match typeB {
            C2_TYPE_CIRCLE => {
                c2CircletoCapsuleManifold(
                    std::ptr::read_unaligned(B as *const c2Circle),
                    std::ptr::read_unaligned(A as *const c2Capsule),
                    m,
                );
                (*m).n = c2Neg((*m).n);
            }
            C2_TYPE_AABB => {
                c2AABBtoCapsuleManifold(
                    std::ptr::read_unaligned(B as *const c2AABB),
                    std::ptr::read_unaligned(A as *const c2Capsule),
                    m,
                );
                (*m).n = c2Neg((*m).n);
            }
            C2_TYPE_CAPSULE => {
                c2CapsuletoCapsuleManifold(
                    std::ptr::read_unaligned(A as *const c2Capsule),
                    std::ptr::read_unaligned(B as *const c2Capsule),
                    m,
                );
            }
            _ => {}
        },
        _ => {}
    }
}

/// NOTE: the C function has no `default:` case and no trailing `return`, so for
/// `C2_TYPE_POLY` (or any other value) it falls off the end and returns an
/// indeterminate value.  The pointer is never dereferenced by `c2Collide` for
/// those types.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ptr_from_parts(
    typ: c_int,
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
) -> *mut c_void {
    match typ {
        C2_TYPE_CIRCLE => {
            let circle = malloc(std::mem::size_of::<c2Circle>()) as *mut c2Circle;
            (*circle).p = c2V(a, b);
            (*circle).r = c;
            circle as *mut c_void
        }
        C2_TYPE_AABB => {
            let aabb = malloc(std::mem::size_of::<c2AABB>()) as *mut c2AABB;
            (*aabb).min = c2V(a, b);
            (*aabb).max = c2V(c, d);
            aabb as *mut c_void
        }
        C2_TYPE_CAPSULE => {
            let capsule = malloc(std::mem::size_of::<c2Capsule>()) as *mut c2Capsule;
            (*capsule).a = c2V(a, b);
            (*capsule).b = c2V(c, d);
            (*capsule).r = e;
            capsule as *mut c_void
        }
        _ => std::ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omni_manifold(
    m: *mut c2Manifold,
    type_a: c_int,
    a1: f32,
    a2: f32,
    a3: f32,
    a4: f32,
    a5: f32,
    type_b: c_int,
    b1: f32,
    b2: f32,
    b3: f32,
    b4: f32,
    b5: f32,
) {
    let A = ptr_from_parts(type_a, a1, a2, a3, a4, a5);
    let B = ptr_from_parts(type_b, b1, b2, b3, b4, b5);

    c2Collide(A, type_a, B, type_b, m);
}
