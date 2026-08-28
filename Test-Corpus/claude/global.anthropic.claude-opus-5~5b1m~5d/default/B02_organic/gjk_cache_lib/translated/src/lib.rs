//! Rust translation of c_src/src/lib.c (tinyc2-style 2D GJK routines).
//!
//! The C library is compiled by CMake as one shared object that exports every
//! non-static function in `src/lib.c`.  This crate reproduces that exact ABI:
//! same symbol names, same C signatures, same struct layouts, and bit-for-bit
//! identical floating point behaviour (all arithmetic is plain IEEE-754
//! single precision, no fused operations, no reassociation).
//!
//! Bugs / quirks in the original C are preserved verbatim, including:
//!   * the inverted cache-validity test in `c2GJK`
//!   * `gjk_cache` never writing through its `a9` / `b9` out-parameters
//!   * NaN-propagating min/max ordering in `c2Maxv` / `c2Minv`

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use core::ffi::{c_char, c_int, c_uint, c_void};
use core::ptr;

// ---------------------------------------------------------------------------
// C2_TYPE (C enum -> unsigned int on the SysV x86-64 ABI)
// ---------------------------------------------------------------------------

pub const C2_TYPE_CIRCLE: c_uint = 0;
pub const C2_TYPE_AABB: c_uint = 1;
pub const C2_TYPE_CAPSULE: c_uint = 2;

// ---------------------------------------------------------------------------
// Public / internal structs.  Layouts verified against the C compiler:
//   c2v = 8, c2r = 8, c2x = 16, c2Circle = 12, c2AABB = 16, c2Capsule = 20,
//   c2GJKCache = 36 (div @ 32), c2Proxy = 72 (verts @ 8),
//   c2sv = 36, c2Simplex = 152 (b @ 36, div @ 144, count @ 148)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
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
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
}

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

/// The C declaration is `c2sv a, b, c, d;` -- four consecutive `c2sv` values,
/// which `c2GJK` walks with `c2sv *verts = &s.a;`.  The array below is
/// layout-identical.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

const ZERO_V: c2v = c2v { x: 0.0, y: 0.0 };

const ZERO_SV: c2sv = c2sv {
    sA: ZERO_V,
    sB: ZERO_V,
    p: ZERO_V,
    u: 0.0,
    iA: 0,
    iB: 0,
};

// FLT_MAX as spelled in the C source.
const C2_FLT_MAX: f32 = 3.402_823_466_385_288_6e38_f32;
// FLT_EPSILON as spelled in the C source.
const C2_FLT_EPSILON: f32 = 1.192_092_895_507_812_5e-7_f32;

// ---------------------------------------------------------------------------
// Vector helpers
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    let mut a = ZERO_V;
    a.x = x;
    a.y = y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(a: c2v, b: f32) -> c2v {
    let mut a = a;
    a.x *= b;
    a.y *= b;
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
    a.x * b.x + a.y * b.y
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

// ---------------------------------------------------------------------------
// Proxies
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2BBVerts(out: *mut c2v, bb: *mut c2AABB) {
    *out.offset(0) = (*bb).min;
    *out.offset(1) = c2V((*bb).max.x, (*bb).min.y);
    *out.offset(2) = (*bb).max;
    *out.offset(3) = c2V((*bb).min.x, (*bb).max.y);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(shape: *const c_void, r#type: c_uint, p: *mut c2Proxy) {
    match r#type {
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
        // No `default:` label in the C switch -- nothing is written.
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Simplex helpers
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> f32 {
    sqrtf(c2Dot(a, a))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: c2v, b: c2v) -> f32 {
    a.x * b.y - a.y * b.x
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJKSimplexMetric(s: *mut c2Simplex) -> f32 {
    match (*s).count {
        2 => c2Len(c2Sub((*s).verts[1].p, (*s).verts[0].p)),
        3 => c2Det2(
            c2Sub((*s).verts[1].p, (*s).verts[0].p),
            c2Sub((*s).verts[2].p, (*s).verts[0].p),
        ),
        // `default:` falls through to `case 1:` in the C source.
        _ => 0.0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Add(a: c2v, b: c2v) -> c2v {
    let mut a = a;
    a.x += b.x;
    a.y += b.y;
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
        (*s).div = u + v;
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
    let uABC = c2Det2(b, c) * area;
    let vABC = c2Det2(c, a) * area;
    let wABC = c2Det2(a, b) * area;
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
        (*s).div = uAB + vAB;
        (*s).count = 2;
    } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
        (*s).verts[0] = (*s).verts[1];
        (*s).verts[1] = (*s).verts[2];
        (*s).verts[0].u = uBC;
        (*s).verts[1].u = vBC;
        (*s).div = uBC + vBC;
        (*s).count = 2;
    } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
        (*s).verts[1] = (*s).verts[0];
        (*s).verts[0] = (*s).verts[2];
        (*s).verts[0].u = uCA;
        (*s).verts[1].u = vCA;
        (*s).div = uCA + vCA;
        (*s).count = 2;
    } else {
        (*s).verts[0].u = uABC;
        (*s).verts[1].u = vABC;
        (*s).verts[2].u = wABC;
        (*s).div = uABC + vABC + wABC;
        (*s).count = 3;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Neg(a: c2v) -> c2v {
    c2V(-a.x, -a.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v {
    let mut b = ZERO_V;
    b.x = -a.y;
    b.y = a.x;
    b
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v {
    let mut b = ZERO_V;
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
        // case 3 and default
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
            *a = (*s).verts[0].sA;
            *b = (*s).verts[0].sB;
        }
        2 => {
            *a = c2Add(
                c2Mulvs((*s).verts[0].sA, den * (*s).verts[0].u),
                c2Mulvs((*s).verts[1].sA, den * (*s).verts[1].u),
            );
            *b = c2Add(
                c2Mulvs((*s).verts[0].sB, den * (*s).verts[0].u),
                c2Mulvs((*s).verts[1].sB, den * (*s).verts[1].u),
            );
        }
        3 => {
            *a = c2Add(
                c2Add(
                    c2Mulvs((*s).verts[0].sA, den * (*s).verts[0].u),
                    c2Mulvs((*s).verts[1].sA, den * (*s).verts[1].u),
                ),
                c2Mulvs((*s).verts[2].sA, den * (*s).verts[2].u),
            );
            *b = c2Add(
                c2Add(
                    c2Mulvs((*s).verts[0].sB, den * (*s).verts[0].u),
                    c2Mulvs((*s).verts[1].sB, den * (*s).verts[1].u),
                ),
                c2Mulvs((*s).verts[2].sB, den * (*s).verts[2].u),
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
    c2Mulvs(a, 1.0f32 / b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2L(s: *mut c2Simplex) -> c2v {
    let den = 1.0f32 / (*s).div;
    match (*s).count {
        1 => (*s).verts[0].p,
        2 => c2Add(
            c2Mulvs((*s).verts[0].p, den * (*s).verts[0].u),
            c2Mulvs((*s).verts[1].p, den * (*s).verts[1].u),
        ),
        _ => c2V(0.0, 0.0),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    // `opaque` stops LLVM from rewriting `(-a.s) * b.x + a.c * b.y` into
    // `a.c * b.y - a.s * b.x`: it likes to sink the `fneg` out of the `fmul`
    // and then fold `fadd (fneg x), y` into `fsub y, x`.  That rewrite is
    // bit-exact for finite values, but it changes the sign of a propagated
    // NaN, so the negation is pinned down here and happens first, exactly as
    // the C source spells it.
    c2V(a.c * b.x + a.s * b.y, opaque(-a.s) * b.x + a.c * b.y)
}

// ---------------------------------------------------------------------------
// GJK
// ---------------------------------------------------------------------------

// `save_count`'s and `d1`'s initial values are dead stores in the C as well
// (`int save_count = 0;` / `float d1 = FLT_MAX;`); they are kept so that the
// declarations line up one-for-one with the original.
#[allow(unused_assignments)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJK(
    A: *const c_void,
    typeA: c_uint,
    ax_ptr: *const c2x,
    B: *const c_void,
    typeB: c_uint,
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

    let mut pA = c2Proxy {
        radius: 0.0,
        count: 0,
        verts: [ZERO_V; 8],
    };
    let mut pB = c2Proxy {
        radius: 0.0,
        count: 0,
        verts: [ZERO_V; 8],
    };
    c2MakeProxy(A, typeA, &mut pA);
    c2MakeProxy(B, typeB, &mut pB);

    let mut simplex = c2Simplex {
        verts: [ZERO_SV; 4],
        div: 0.0,
        count: 0,
    };
    let s: *mut c2Simplex = &raw mut simplex;
    let verts: *mut c2sv = (*s).verts.as_mut_ptr();

    let mut cache_was_read: c_int = 0;
    if !cache.is_null() {
        let cache_was_good = (*cache).count != 0;
        if cache_was_good {
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
            (*s).count = (*cache).count;
            (*s).div = (*cache).div;
            let metric_old = (*cache).metric;
            let metric = c2GJKSimplexMetric(s);
            let min_metric = if metric < metric_old { metric } else { metric_old };
            let max_metric = if metric > metric_old { metric } else { metric_old };
            if !(min_metric < max_metric * 2.0f32 && metric < -1.0e8f32) {
                cache_was_read = 1;
            }
        }
    }

    if cache_was_read == 0 {
        (*verts.offset(0)).iA = 0;
        (*verts.offset(0)).iB = 0;
        (*verts.offset(0)).sA = c2Mulxv(ax, pA.verts[0]);
        (*verts.offset(0)).sB = c2Mulxv(bx, pB.verts[0]);
        (*verts.offset(0)).p = c2Sub((*verts.offset(0)).sB, (*verts.offset(0)).sA);
        (*verts.offset(0)).u = 1.0f32;
        (*s).div = 1.0f32;
        (*s).count = 1;
    }

    let mut saveA: [c_int; 3] = [0; 3];
    let mut saveB: [c_int; 3] = [0; 3];
    let mut save_count: c_int = 0;
    let mut d0: f32 = C2_FLT_MAX;
    let mut d1: f32 = C2_FLT_MAX;
    let mut iter: c_int = 0;
    let mut hit: c_int = 0;
    while iter < 20 {
        save_count = (*s).count;
        let mut i: c_int = 0;
        while i < save_count {
            *saveA.as_mut_ptr().offset(i as isize) = (*verts.offset(i as isize)).iA;
            *saveB.as_mut_ptr().offset(i as isize) = (*verts.offset(i as isize)).iB;
            i += 1;
        }

        match (*s).count {
            1 => {}
            2 => c22(s),
            3 => c23(s),
            _ => {}
        }

        if (*s).count == 3 {
            hit = 1;
            break;
        }

        let p = c2L(s);
        d1 = c2Dot(p, p);
        if d1 > d0 {
            break;
        }
        d0 = d1;

        let d = c2D(s);
        if c2Dot(d, d) < C2_FLT_EPSILON * C2_FLT_EPSILON {
            break;
        }

        let iA = c2Support(pA.verts.as_ptr(), pA.count, c2MulrvT(ax.r, c2Neg(d)));
        let sA = c2Mulxv(ax, *pA.verts.as_ptr().offset(iA as isize));
        let iB = c2Support(pB.verts.as_ptr(), pB.count, c2MulrvT(bx.r, d));
        let sB = c2Mulxv(bx, *pB.verts.as_ptr().offset(iB as isize));
        let v: *mut c2sv = verts.offset((*s).count as isize);
        (*v).iA = iA;
        (*v).sA = sA;
        (*v).iB = iB;
        (*v).sB = sB;
        (*v).p = c2Sub((*v).sB, (*v).sA);

        let mut dup: c_int = 0;
        let mut i: c_int = 0;
        while i < save_count {
            if iA == *saveA.as_ptr().offset(i as isize) && iB == *saveB.as_ptr().offset(i as isize) {
                dup = 1;
                break;
            }
            i += 1;
        }
        if dup != 0 {
            break;
        }

        (*s).count += 1;
        iter += 1;
    }

    let mut a = ZERO_V;
    let mut b = ZERO_V;
    c2Witness(s, &mut a, &mut b);
    let mut dist = c2Len(c2Sub(a, b));
    if hit != 0 {
        a = b;
        dist = 0.0;
    } else if use_radius != 0 {
        let rA = pA.radius;
        let rB = pB.radius;
        if dist > rA + rB && dist > C2_FLT_EPSILON {
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
        (*cache).metric = c2GJKSimplexMetric(s);
        (*cache).count = (*s).count;
        let mut i: c_int = 0;
        while i < (*s).count {
            let v: *mut c2sv = verts.offset(i as isize);
            *(*cache).iA.as_mut_ptr().offset(i as isize) = (*v).iA;
            *(*cache).iB.as_mut_ptr().offset(i as isize) = (*v).iB;
            i += 1;
        }
        (*cache).div = (*s).div;
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
// Public entry point declared in include/lib.h
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gjk_cache(
    reverse: c_char,
    a9: *mut c2v,
    b9: *mut c2v,
    a1: f32,
    a2: f32,
    a3: f32,
    a4: f32,
    b1: f32,
    b2: f32,
    b3: f32,
    b4: f32,
    b5: f32,
) {
    // `cache` is an uninitialised automatic in the C; only `count` is set, and
    // because `count == 0` none of the other members are ever read before
    // being written, so starting from zeros is behaviourally identical.
    let mut cache = c2GJKCache {
        metric: 0.0,
        count: 0,
        iA: [0; 3],
        iB: [0; 3],
        div: 0.0,
    };
    cache.count = 0;

    let A = c2Circle {
        p: c2v { x: 0.0, y: 0.0 },
        r: 15.0f32,
    };

    let B = c2Capsule {
        a: c2v { x: 100.0, y: -25.0 },
        b: c2v { x: 75.0, y: 100.0 },
        r: 10.0,
    };

    let mut a0 = ZERO_V;
    let mut b0 = ZERO_V;
    let mut a = ZERO_V;
    let mut b = ZERO_V;

    let mut iterations: c_int = -1;
    let mut cached_iterations: c_int = -1;
    let d0 = c2GJK(
        &A as *const c2Circle as *const c_void,
        C2_TYPE_CIRCLE,
        ptr::null(),
        &B as *const c2Capsule as *const c_void,
        C2_TYPE_CAPSULE,
        ptr::null(),
        &mut a0,
        &mut b0,
        1,
        &mut iterations,
        &mut cache,
    );
    let d1 = c2GJK(
        &A as *const c2Circle as *const c_void,
        C2_TYPE_CIRCLE,
        ptr::null(),
        &B as *const c2Capsule as *const c_void,
        C2_TYPE_CAPSULE,
        ptr::null(),
        &mut a,
        &mut b,
        1,
        &mut cached_iterations,
        &mut cache,
    );
    let _ = (d0, d1);

    let mut bb = c2AABB {
        min: ZERO_V,
        max: ZERO_V,
    };
    bb.min = c2V(a1, a2);
    bb.max = c2V(a3, a4);

    let mut cap = c2Capsule {
        a: ZERO_V,
        b: ZERO_V,
        r: 0.0,
    };
    cap.a = c2V(b1, b2);
    cap.b = c2V(b3, b4);
    cap.r = b5;

    if reverse != 0 {
        c2GJK(
            &cap as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            ptr::null(),
            &bb as *const c2AABB as *const c_void,
            C2_TYPE_AABB,
            ptr::null(),
            &mut a,
            &mut b,
            1,
            ptr::null_mut(),
            ptr::null_mut(),
        );
    } else {
        c2GJK(
            &bb as *const c2AABB as *const c_void,
            C2_TYPE_AABB,
            ptr::null(),
            &cap as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            ptr::null(),
            &mut a,
            &mut b,
            1,
            ptr::null_mut(),
            ptr::null_mut(),
        );
    }

    // The C never dereferences `a9` / `b9`; neither do we.
    let _ = (a9, b9);
}

// ---------------------------------------------------------------------------
// math
// ---------------------------------------------------------------------------

/// Single-precision square root, matching libm's `sqrtf` bit for bit
/// (`sqrtss` on x86-64, correctly rounded per IEEE-754).
#[inline]
fn sqrtf(x: f32) -> f32 {
    x.sqrt()
}

/// Identity that is opaque to the optimiser: used to pin down the evaluation
/// order of a float expression so that it matches what the C compiler emits.
/// A volatile read is a language-level guarantee (unlike an optimiser hint):
/// the value must really be materialised before it is read back.
#[inline(always)]
fn opaque(x: f32) -> f32 {
    let slot = x;
    unsafe { ptr::read_volatile(&slot) }
}
