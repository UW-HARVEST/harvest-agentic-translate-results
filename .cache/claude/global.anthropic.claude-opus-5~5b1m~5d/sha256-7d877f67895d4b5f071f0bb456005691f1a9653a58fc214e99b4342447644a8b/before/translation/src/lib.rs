//! Rust translation of c_src/src/lib.c (tinyc2 / cute_c2 derived collision routines).
//!
//! The C library compiles every function in `src/lib.c` with external linkage, so the
//! shared object exports all of them.  This crate reproduces the complete public ABI
//! (38 symbols) with identical signatures, identical struct layouts and identical
//! floating point behaviour.
//!
//! Original license: see c_src/license.txt (zlib / Unlicense, Copyright (c) 2023 Randy Gaul).

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_int, c_uint, c_void};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

// typedef enum { C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE } C2_TYPE;
// GCC gives an all-non-negative enum the underlying type `unsigned int`; on the
// x86-64 SysV ABI this is passed exactly like `int`.
pub type C2_TYPE = c_uint;

pub const C2_TYPE_CIRCLE: C2_TYPE = 0;
pub const C2_TYPE_AABB: C2_TYPE = 1;
pub const C2_TYPE_CAPSULE: C2_TYPE = 2;

/// `typedef struct c2v { float x; float y; } c2v;`
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

/// `typedef struct c2r { float c; float s; } c2r;`
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

/// `typedef struct c2x { c2v p; c2r r; } c2x;`
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

/// `typedef struct c2Circle { c2v p; float r; } c2Circle;`
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

/// `typedef struct c2AABB { c2v min; c2v max; } c2AABB;`
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

/// `typedef struct c2Capsule { c2v a; c2v b; float r; } c2Capsule;`
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

/// `typedef struct c2GJKCache { float metric; int count; int iA[3]; int iB[3]; float div; } c2GJKCache;`
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

/// `typedef struct { float radius; int count; c2v verts[8]; } c2Proxy;`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
}

impl Default for c2Proxy {
    fn default() -> Self {
        c2Proxy {
            radius: 0.0,
            count: 0,
            verts: [c2v::default(); 8],
        }
    }
}

/// `typedef struct { c2v sA; c2v sB; c2v p; float u; int iA; int iB; } c2sv;`
#[repr(C)]
#[derive(Clone, Copy, Default)]
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
/// The four `c2sv` members are stored as an array; this is layout-identical to the
/// four consecutive named members in C (`c2sv` has size 36 and alignment 4), and the
/// original code relies on that by taking `&s.a` and indexing it as an array.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

// FLT_MAX / FLT_EPSILON as spelled out in the C source.
const C2_FLT_MAX: f32 = 3.402_823_466_385_288_598_117_041_834_845_169_25e+38_f32;
const C2_FLT_EPSILON: f32 = 1.192_092_895_507_812_5e-7_f32;

// ---------------------------------------------------------------------------
// Basic vector / rotation math
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    let mut a = c2v { x: 0.0, y: 0.0 };
    a.x = x;
    a.y = y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(mut a: c2v, b: f32) -> c2v {
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
pub extern "C" fn c2Sub(mut a: c2v, b: c2v) -> c2v {
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
        p: c2v { x: 0.0, y: 0.0 },
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(shape: *const c_void, type_: C2_TYPE, p: *mut c2Proxy) {
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
        _ => {}
    }
}

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
    let verts = (*s).verts.as_mut_ptr();
    match (*s).count {
        2 => c2Len(c2Sub((*verts.offset(1)).p, (*verts.offset(0)).p)),
        3 => c2Det2(
            c2Sub((*verts.offset(1)).p, (*verts.offset(0)).p),
            c2Sub((*verts.offset(2)).p, (*verts.offset(0)).p),
        ),
        // `default:` falls through into `case 1:` which returns 0.
        _ => 0.0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Add(mut a: c2v, b: c2v) -> c2v {
    a.x += b.x;
    a.y += b.y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulxv(a: c2x, b: c2v) -> c2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

// ---------------------------------------------------------------------------
// Simplex solvers
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c22(s: *mut c2Simplex) {
    let verts = (*s).verts.as_mut_ptr();
    let a = (*verts.offset(0)).p;
    let b = (*verts.offset(1)).p;
    let u = c2Dot(b, c2Sub(b, a));
    let v = c2Dot(a, c2Sub(a, b));
    if v <= 0.0 {
        (*verts.offset(0)).u = 1.0f32;
        (*s).div = 1.0f32;
        (*s).count = 1;
    } else if u <= 0.0 {
        *verts.offset(0) = *verts.offset(1);
        (*verts.offset(0)).u = 1.0f32;
        (*s).div = 1.0f32;
        (*s).count = 1;
    } else {
        (*verts.offset(0)).u = u;
        (*verts.offset(1)).u = v;
        (*s).div = u + v;
        (*s).count = 2;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c23(s: *mut c2Simplex) {
    let verts = (*s).verts.as_mut_ptr();
    let a = (*verts.offset(0)).p;
    let b = (*verts.offset(1)).p;
    let c = (*verts.offset(2)).p;
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
        (*verts.offset(0)).u = 1.0f32;
        (*s).div = 1.0f32;
        (*s).count = 1;
    } else if uAB <= 0.0 && vBC <= 0.0 {
        *verts.offset(0) = *verts.offset(1);
        (*verts.offset(0)).u = 1.0f32;
        (*s).div = 1.0f32;
        (*s).count = 1;
    } else if uBC <= 0.0 && vCA <= 0.0 {
        *verts.offset(0) = *verts.offset(2);
        (*verts.offset(0)).u = 1.0f32;
        (*s).div = 1.0f32;
        (*s).count = 1;
    } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
        (*verts.offset(0)).u = uAB;
        (*verts.offset(1)).u = vAB;
        (*s).div = uAB + vAB;
        (*s).count = 2;
    } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
        *verts.offset(0) = *verts.offset(1);
        *verts.offset(1) = *verts.offset(2);
        (*verts.offset(0)).u = uBC;
        (*verts.offset(1)).u = vBC;
        (*s).div = uBC + vBC;
        (*s).count = 2;
    } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
        *verts.offset(1) = *verts.offset(0);
        *verts.offset(0) = *verts.offset(2);
        (*verts.offset(0)).u = uCA;
        (*verts.offset(1)).u = vCA;
        (*s).div = uCA + vCA;
        (*s).count = 2;
    } else {
        (*verts.offset(0)).u = uABC;
        (*verts.offset(1)).u = vABC;
        (*verts.offset(2)).u = wABC;
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
    let mut b = c2v { x: 0.0, y: 0.0 };
    b.x = -a.y;
    b.y = a.x;
    b
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v {
    let mut b = c2v { x: 0.0, y: 0.0 };
    b.x = a.y;
    b.y = -a.x;
    b
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2D(s: *mut c2Simplex) -> c2v {
    let verts = (*s).verts.as_mut_ptr();
    match (*s).count {
        1 => c2Neg((*verts.offset(0)).p),
        2 => {
            let ab = c2Sub((*verts.offset(1)).p, (*verts.offset(0)).p);
            if c2Det2(ab, c2Neg((*verts.offset(0)).p)) > 0.0 {
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
    let verts = (*s).verts.as_mut_ptr();
    let den = 1.0f32 / (*s).div;
    match (*s).count {
        1 => {
            *a = (*verts.offset(0)).sA;
            *b = (*verts.offset(0)).sB;
        }
        2 => {
            *a = c2Add(
                c2Mulvs((*verts.offset(0)).sA, den * (*verts.offset(0)).u),
                c2Mulvs((*verts.offset(1)).sA, den * (*verts.offset(1)).u),
            );
            *b = c2Add(
                c2Mulvs((*verts.offset(0)).sB, den * (*verts.offset(0)).u),
                c2Mulvs((*verts.offset(1)).sB, den * (*verts.offset(1)).u),
            );
        }
        3 => {
            *a = c2Add(
                c2Add(
                    c2Mulvs((*verts.offset(0)).sA, den * (*verts.offset(0)).u),
                    c2Mulvs((*verts.offset(1)).sA, den * (*verts.offset(1)).u),
                ),
                c2Mulvs((*verts.offset(2)).sA, den * (*verts.offset(2)).u),
            );
            *b = c2Add(
                c2Add(
                    c2Mulvs((*verts.offset(0)).sB, den * (*verts.offset(0)).u),
                    c2Mulvs((*verts.offset(1)).sB, den * (*verts.offset(1)).u),
                ),
                c2Mulvs((*verts.offset(2)).sB, den * (*verts.offset(2)).u),
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
    let verts = (*s).verts.as_mut_ptr();
    let den = 1.0f32 / (*s).div;
    match (*s).count {
        1 => (*verts.offset(0)).p,
        2 => c2Add(
            c2Mulvs((*verts.offset(0)).p, den * (*verts.offset(0)).u),
            c2Mulvs((*verts.offset(1)).p, den * (*verts.offset(1)).u),
        ),
        _ => c2V(0.0, 0.0),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

// ---------------------------------------------------------------------------
// GJK
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJK(
    A: *const c_void,
    typeA: C2_TYPE,
    ax_ptr: *const c2x,
    B: *const c_void,
    typeB: C2_TYPE,
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
    let mut s = c2Simplex::default();
    let verts: *mut c2sv = s.verts.as_mut_ptr();
    let pA_verts: *const c2v = pA.verts.as_ptr();
    let pB_verts: *const c2v = pB.verts.as_ptr();
    let mut cache_was_read: c_int = 0;
    if !cache.is_null() {
        let cache_was_good = (*cache).count != 0;
        if cache_was_good {
            let mut i: c_int = 0;
            while i < (*cache).count {
                let iA = *(*cache).iA.as_ptr().offset(i as isize);
                let iB = *(*cache).iB.as_ptr().offset(i as isize);
                let sA = c2Mulxv(ax, *pA_verts.offset(iA as isize));
                let sB = c2Mulxv(bx, *pB_verts.offset(iB as isize));
                let v = verts.offset(i as isize);
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
        (*verts.offset(0)).iA = 0;
        (*verts.offset(0)).iB = 0;
        (*verts.offset(0)).sA = c2Mulxv(ax, *pA_verts.offset(0));
        (*verts.offset(0)).sB = c2Mulxv(bx, *pB_verts.offset(0));
        (*verts.offset(0)).p = c2Sub((*verts.offset(0)).sB, (*verts.offset(0)).sA);
        (*verts.offset(0)).u = 1.0f32;
        s.div = 1.0f32;
        s.count = 1;
    }
    let mut saveA: [c_int; 3] = [0; 3];
    let mut saveB: [c_int; 3] = [0; 3];
    let saveA_p: *mut c_int = saveA.as_mut_ptr();
    let saveB_p: *mut c_int = saveB.as_mut_ptr();
    let mut save_count: c_int;
    let mut d0: f32 = C2_FLT_MAX;
    let mut d1: f32;
    let mut iter: c_int = 0;
    let mut hit: c_int = 0;
    while iter < 20 {
        save_count = s.count;
        let mut i: c_int = 0;
        while i < save_count {
            *saveA_p.offset(i as isize) = (*verts.offset(i as isize)).iA;
            *saveB_p.offset(i as isize) = (*verts.offset(i as isize)).iB;
            i += 1;
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
        if c2Dot(d, d) < C2_FLT_EPSILON * C2_FLT_EPSILON {
            break;
        }
        let iA = c2Support(pA_verts, pA.count, c2MulrvT(ax.r, c2Neg(d)));
        let sA = c2Mulxv(ax, *pA_verts.offset(iA as isize));
        let iB = c2Support(pB_verts, pB.count, c2MulrvT(bx.r, d));
        let sB = c2Mulxv(bx, *pB_verts.offset(iB as isize));
        let v = verts.offset(s.count as isize);
        (*v).iA = iA;
        (*v).sA = sA;
        (*v).iB = iB;
        (*v).sB = sB;
        (*v).p = c2Sub((*v).sB, (*v).sA);
        let mut dup: c_int = 0;
        let mut j: c_int = 0;
        while j < save_count {
            if iA == *saveA_p.offset(j as isize) && iB == *saveB_p.offset(j as isize) {
                dup = 1;
                break;
            }
            j += 1;
        }
        if dup != 0 {
            break;
        }
        s.count += 1;
        iter += 1;
    }
    let mut a = c2v { x: 0.0, y: 0.0 };
    let mut b = c2v { x: 0.0, y: 0.0 };
    c2Witness(&mut s, &mut a, &mut b);
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
        (*cache).metric = c2GJKSimplexMetric(&mut s);
        (*cache).count = s.count;
        let mut i: c_int = 0;
        while i < s.count {
            let v = verts.offset(i as isize);
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
    unsafe {
        if c2GJK(
            &A as *const c2AABB as *const c_void,
            C2_TYPE_AABB,
            core::ptr::null(),
            &B as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            core::ptr::null(),
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            1,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
        ) != 0.0
        {
            return 0;
        }
    }
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CapsuletoCapsule(A: c2Capsule, B: c2Capsule) -> c_int {
    unsafe {
        if c2GJK(
            &A as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            core::ptr::null(),
            &B as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            core::ptr::null(),
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            1,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
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
            let e = c2Sub(ap, c2Mulvs(n, da / c2Dot(n, n)));
            d2 = c2Dot(e, e);
        } else {
            let bp = c2Sub(A.p, B.b);
            d2 = c2Dot(bp, bp);
        }
    }
    let r = A.r + B.r;
    (d2 < r * r) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Collided(
    A: *const c_void,
    typeA: C2_TYPE,
    B: *const c_void,
    typeB: C2_TYPE,
) -> c_int {
    match typeA {
        C2_TYPE_CIRCLE => match typeB {
            C2_TYPE_CIRCLE => c2CircletoCircle(*(A as *const c2Circle), *(B as *const c2Circle)),
            C2_TYPE_AABB => c2CircletoAABB(*(A as *const c2Circle), *(B as *const c2AABB)),
            C2_TYPE_CAPSULE => c2CircletoCapsule(*(A as *const c2Circle), *(B as *const c2Capsule)),
            _ => 0,
        },
        C2_TYPE_AABB => match typeB {
            C2_TYPE_CIRCLE => c2CircletoAABB(*(B as *const c2Circle), *(A as *const c2AABB)),
            C2_TYPE_AABB => c2AABBtoAABB(*(A as *const c2AABB), *(B as *const c2AABB)),
            C2_TYPE_CAPSULE => c2AABBtoCapsule(*(A as *const c2AABB), *(B as *const c2Capsule)),
            _ => 0,
        },
        C2_TYPE_CAPSULE => match typeB {
            C2_TYPE_CIRCLE => c2CircletoCapsule(*(B as *const c2Circle), *(A as *const c2Capsule)),
            C2_TYPE_AABB => c2AABBtoCapsule(*(B as *const c2AABB), *(A as *const c2Capsule)),
            C2_TYPE_CAPSULE => {
                c2CapsuletoCapsule(*(A as *const c2Capsule), *(B as *const c2Capsule))
            }
            _ => 0,
        },
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// Public entry point (include/lib.h)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn reverse_collide(x: f32, y: f32, r: f32) -> c_int {
    let mut result: c_int = 0;

    let mut circle_in = c2Circle::default();
    circle_in.p = c2V(x, y);
    circle_in.r = r;

    let mut circle = c2Circle::default();
    circle.p = c2V(-70.0f32, 0.0);
    circle.r = 20.0f32;

    let mut aabb = c2AABB::default();
    aabb.min = c2V(-40.0f32, -40.0f32);
    aabb.max = c2V(-15.0f32, -15.0f32);

    let mut capsule = c2Capsule::default();
    capsule.a = c2V(-40.0f32, 40.0f32);
    capsule.b = c2V(-20.0f32, 100.0f32);
    capsule.r = 10.0f32;

    unsafe {
        result = result.wrapping_add(c2Collided(
            &circle as *const c2Circle as *const c_void,
            C2_TYPE_CIRCLE,
            &circle_in as *const c2Circle as *const c_void,
            C2_TYPE_CIRCLE,
        ));

        result = result.wrapping_add(
            c2Collided(
                &aabb as *const c2AABB as *const c_void,
                C2_TYPE_AABB,
                &circle_in as *const c2Circle as *const c_void,
                C2_TYPE_CIRCLE,
            ) << 1,
        );

        result = result.wrapping_add(
            c2Collided(
                &capsule as *const c2Capsule as *const c_void,
                C2_TYPE_CAPSULE,
                &circle_in as *const c2Circle as *const c_void,
                C2_TYPE_CIRCLE,
            ) << 2,
        );
    }

    result
}

// ---------------------------------------------------------------------------
// libm shim: sqrtf must behave exactly like the C library's sqrtf (IEEE-754
// correctly rounded single precision square root, which is what the hardware
// instruction used by `f32::sqrt` provides).
// ---------------------------------------------------------------------------

#[inline]
fn sqrtf(x: f32) -> f32 {
    x.sqrt()
}
