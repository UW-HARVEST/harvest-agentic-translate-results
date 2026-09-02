//! Rust translation of c_src/src/lib.c (a cute_c2-derived 2D GJK implementation).
//!
//! The goal is bit-exact behavioural parity with the C original, including its
//! quirks (e.g. the inverted/typo'd GJK cache validity test). Public symbols are
//! exported with the same names and C ABI as the original shared library.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use std::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// Public / internal types (layouts must match the C originals exactly)
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
/// The four `c2sv` members are modelled as an array because the C code takes
/// `&s.a` and indexes off it; the memory layout is identical.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

const C2_TYPE_CIRCLE: c_int = 0;
const C2_TYPE_AABB: c_int = 1;
const C2_TYPE_CAPSULE: c_int = 2;

const ZERO_V: c2v = c2v { x: 0.0, y: 0.0 };

const ZERO_SV: c2sv = c2sv {
    sA: ZERO_V,
    sB: ZERO_V,
    p: ZERO_V,
    u: 0.0,
    iA: 0,
    iB: 0,
};

/// `FLT_MAX` as spelled out in the C source.
const C2_FLT_MAX: f32 = 3.402_823_5e38;
/// `FLT_EPSILON` as spelled out in the C source.
const C2_FLT_EPSILON: f32 = 1.192_092_9e-7;

// ---------------------------------------------------------------------------
// Basic vector maths
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
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
    c2r { c: 1.0, s: 0.0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2xIdentity() -> c2x {
    c2x {
        p: c2V(0.0, 0.0),
        r: c2RotIdentity(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> f32 {
    c2Dot(a, a).sqrt()
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: c2v, b: c2v) -> f32 {
    a.x * b.y - a.y * b.x
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

#[unsafe(no_mangle)]
pub extern "C" fn c2Neg(a: c2v) -> c2v {
    c2V(-a.x, -a.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v {
    c2v { x: -a.y, y: a.x }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v {
    c2v { x: a.y, y: -a.x }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: c2v, b: f32) -> c2v {
    c2Mulvs(a, 1.0 / b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

// ---------------------------------------------------------------------------
// Shape helpers
// ---------------------------------------------------------------------------

/// # Safety
/// `out` must point to at least 4 writable `c2v`, `bb` to a readable `c2AABB`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2BBVerts(out: *mut c2v, bb: *mut c2AABB) {
    unsafe {
        let min = (*bb).min;
        let max = (*bb).max;
        *out.add(0) = min;
        *out.add(1) = c2V(max.x, min.y);
        *out.add(2) = max;
        *out.add(3) = c2V(min.x, max.y);
    }
}

/// # Safety
/// `shape` must point to a shape matching `type_`; `p` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(shape: *const c_void, type_: c_int, p: *mut c2Proxy) {
    unsafe {
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
                c2BBVerts((*p).verts.as_mut_ptr(), bb);
            }
            C2_TYPE_CAPSULE => {
                let c = shape as *const c2Capsule;
                (*p).radius = (*c).r;
                (*p).count = 2;
                (*p).verts[0] = (*c).a;
                (*p).verts[1] = (*c).b;
            }
            // The C `switch` has no `default`: the proxy is left untouched.
            _ => {}
        }
    }
}

/// # Safety
/// `verts` must point to at least one `c2v` (the C code reads `verts[0]`
/// unconditionally, even when `count <= 0`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Support(verts: *const c2v, count: c_int, d: c2v) -> c_int {
    unsafe {
        let mut imax: c_int = 0;
        let mut dmax = c2Dot(*verts, d);
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
}

// ---------------------------------------------------------------------------
// Simplex routines
// ---------------------------------------------------------------------------

/// # Safety
/// `s` must point to a valid `c2Simplex`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJKSimplexMetric(s: *mut c2Simplex) -> f32 {
    unsafe {
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
}

/// # Safety
/// `s` must point to a valid `c2Simplex`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c22(s: *mut c2Simplex) {
    unsafe {
        let s = &mut *s;
        let a = s.verts[0].p;
        let b = s.verts[1].p;
        let u = c2Dot(b, c2Sub(b, a));
        let v = c2Dot(a, c2Sub(a, b));
        if v <= 0.0 {
            s.verts[0].u = 1.0;
            s.div = 1.0;
            s.count = 1;
        } else if u <= 0.0 {
            s.verts[0] = s.verts[1];
            s.verts[0].u = 1.0;
            s.div = 1.0;
            s.count = 1;
        } else {
            s.verts[0].u = u;
            s.verts[1].u = v;
            s.div = u + v;
            s.count = 2;
        }
    }
}

/// # Safety
/// `s` must point to a valid `c2Simplex`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c23(s: *mut c2Simplex) {
    unsafe {
        let s = &mut *s;
        let a = s.verts[0].p;
        let b = s.verts[1].p;
        let c = s.verts[2].p;
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
            s.verts[0].u = 1.0;
            s.div = 1.0;
            s.count = 1;
        } else if uAB <= 0.0 && vBC <= 0.0 {
            s.verts[0] = s.verts[1];
            s.verts[0].u = 1.0;
            s.div = 1.0;
            s.count = 1;
        } else if uBC <= 0.0 && vCA <= 0.0 {
            s.verts[0] = s.verts[2];
            s.verts[0].u = 1.0;
            s.div = 1.0;
            s.count = 1;
        } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
            s.verts[0].u = uAB;
            s.verts[1].u = vAB;
            s.div = uAB + vAB;
            s.count = 2;
        } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
            s.verts[0] = s.verts[1];
            s.verts[1] = s.verts[2];
            s.verts[0].u = uBC;
            s.verts[1].u = vBC;
            s.div = uBC + vBC;
            s.count = 2;
        } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
            s.verts[1] = s.verts[0];
            s.verts[0] = s.verts[2];
            s.verts[0].u = uCA;
            s.verts[1].u = vCA;
            s.div = uCA + vCA;
            s.count = 2;
        } else {
            s.verts[0].u = uABC;
            s.verts[1].u = vABC;
            s.verts[2].u = wABC;
            s.div = uABC + vABC + wABC;
            s.count = 3;
        }
    }
}

/// # Safety
/// `s` must point to a valid `c2Simplex`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2D(s: *mut c2Simplex) -> c2v {
    unsafe {
        match (*s).count {
            1 => c2Neg((*s).verts[0].p),
            2 => {
                let ab = c2Sub((*s).verts[1].p, (*s).verts[0].p);
                if c2Det2(ab, c2Neg((*s).verts[0].p)) > 0.0 {
                    c2Skew(ab)
                } else {
                    c2CCW90(ab)
                }
            }
            // case 3 and default
            _ => c2V(0.0, 0.0),
        }
    }
}

/// # Safety
/// `s`, `a` and `b` must be valid pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Witness(s: *mut c2Simplex, a: *mut c2v, b: *mut c2v) {
    unsafe {
        let den = 1.0f32 / (*s).div;
        let v = &(*s).verts;
        match (*s).count {
            1 => {
                *a = v[0].sA;
                *b = v[0].sB;
            }
            2 => {
                *a = c2Add(
                    c2Mulvs(v[0].sA, den * v[0].u),
                    c2Mulvs(v[1].sA, den * v[1].u),
                );
                *b = c2Add(
                    c2Mulvs(v[0].sB, den * v[0].u),
                    c2Mulvs(v[1].sB, den * v[1].u),
                );
            }
            3 => {
                *a = c2Add(
                    c2Add(
                        c2Mulvs(v[0].sA, den * v[0].u),
                        c2Mulvs(v[1].sA, den * v[1].u),
                    ),
                    c2Mulvs(v[2].sA, den * v[2].u),
                );
                *b = c2Add(
                    c2Add(
                        c2Mulvs(v[0].sB, den * v[0].u),
                        c2Mulvs(v[1].sB, den * v[1].u),
                    ),
                    c2Mulvs(v[2].sB, den * v[2].u),
                );
            }
            _ => {
                *a = c2V(0.0, 0.0);
                *b = c2V(0.0, 0.0);
            }
        }
    }
}

/// # Safety
/// `s` must point to a valid `c2Simplex`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2L(s: *mut c2Simplex) -> c2v {
    unsafe {
        let den = 1.0f32 / (*s).div;
        let v = &(*s).verts;
        match (*s).count {
            1 => v[0].p,
            2 => c2Add(
                c2Mulvs(v[0].p, den * v[0].u),
                c2Mulvs(v[1].p, den * v[1].u),
            ),
            _ => c2V(0.0, 0.0),
        }
    }
}

// ---------------------------------------------------------------------------
// GJK
// ---------------------------------------------------------------------------

/// # Safety
/// All non-null pointers must be valid for their respective types.
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
    unsafe {
        let ax: c2x = if ax_ptr.is_null() {
            c2xIdentity()
        } else {
            *ax_ptr
        };
        let bx: c2x = if bx_ptr.is_null() {
            c2xIdentity()
        } else {
            *bx_ptr
        };

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

        let mut s = c2Simplex {
            verts: [ZERO_SV; 4],
            div: 0.0,
            count: 0,
        };
        // `c2sv *verts = &s.a;`
        let verts: *mut c2sv = s.verts.as_mut_ptr();

        let mut cache_was_read = 0;
        if !cache.is_null() {
            let cache_was_good = (*cache).count != 0;
            if cache_was_good {
                let mut i: c_int = 0;
                while i < (*cache).count {
                    let iA = *(*cache).iA.as_ptr().offset(i as isize);
                    let iB = *(*cache).iB.as_ptr().offset(i as isize);
                    let sA = c2Mulxv(ax, *pA.verts.as_ptr().offset(iA as isize));
                    let sB = c2Mulxv(bx, *pB.verts.as_ptr().offset(iB as isize));
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
                // Reproduced verbatim from the C source (note the `-1.0e8f`).
                if !(min_metric < max_metric * 2.0 && metric < -1.0e8f32) {
                    cache_was_read = 1;
                }
            }
        }

        if cache_was_read == 0 {
            s.verts[0].iA = 0;
            s.verts[0].iB = 0;
            s.verts[0].sA = c2Mulxv(ax, pA.verts[0]);
            s.verts[0].sB = c2Mulxv(bx, pB.verts[0]);
            s.verts[0].p = c2Sub(s.verts[0].sB, s.verts[0].sA);
            s.verts[0].u = 1.0;
            s.div = 1.0;
            s.count = 1;
        }

        let mut saveA: [c_int; 3] = [0; 3];
        let mut saveB: [c_int; 3] = [0; 3];
        let mut save_count: c_int;
        // `d1` in the C source is a loop-local carrier for `c2Dot(p, p)`; it is
        // never read after the loop, so it lives inside the loop body here.
        let mut d0 = C2_FLT_MAX;
        let mut iter: c_int = 0;
        let mut hit = 0;

        loop {
            if iter >= 20 {
                break;
            }
            save_count = s.count;
            let mut i: c_int = 0;
            while i < save_count {
                *saveA.as_mut_ptr().offset(i as isize) = (*verts.offset(i as isize)).iA;
                *saveB.as_mut_ptr().offset(i as isize) = (*verts.offset(i as isize)).iB;
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
            let d1 = c2Dot(p, p);
            if d1 > d0 {
                break;
            }
            d0 = d1;

            let d = c2D(&mut s);
            if c2Dot(d, d) < C2_FLT_EPSILON * C2_FLT_EPSILON {
                break;
            }

            let iA = c2Support(pA.verts.as_ptr(), pA.count, c2MulrvT(ax.r, c2Neg(d)));
            let sA = c2Mulxv(ax, *pA.verts.as_ptr().offset(iA as isize));
            let iB = c2Support(pB.verts.as_ptr(), pB.count, c2MulrvT(bx.r, d));
            let sB = c2Mulxv(bx, *pB.verts.as_ptr().offset(iB as isize));

            let v = verts.offset(s.count as isize);
            (*v).iA = iA;
            (*v).sA = sA;
            (*v).iB = iB;
            (*v).sB = sB;
            (*v).p = c2Sub((*v).sB, (*v).sA);

            let mut dup = 0;
            let mut j: c_int = 0;
            while j < save_count {
                if iA == *saveA.as_ptr().offset(j as isize)
                    && iB == *saveB.as_ptr().offset(j as isize)
                {
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

        let mut a = ZERO_V;
        let mut b = ZERO_V;
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
                let p = c2Mulvs(c2Add(a, b), 0.5);
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
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// # Safety
/// `a` and `b` must be valid writable pointers to `c2v`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gjk(
    reverse: c_char,
    a: *mut c2v,
    b: *mut c2v,
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
    unsafe {
        let mut bb = c2AABB {
            min: c2V(a1, a2),
            max: c2V(a3, a4),
        };

        let mut cap = c2Capsule {
            a: c2V(b1, b2),
            b: c2V(b3, b4),
            r: b5,
        };

        if reverse != 0 {
            c2GJK(
                &mut cap as *mut c2Capsule as *const c_void,
                C2_TYPE_CAPSULE,
                std::ptr::null(),
                &mut bb as *mut c2AABB as *const c_void,
                C2_TYPE_AABB,
                std::ptr::null(),
                a,
                b,
                1,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
        } else {
            c2GJK(
                &mut bb as *mut c2AABB as *const c_void,
                C2_TYPE_AABB,
                std::ptr::null(),
                &mut cap as *mut c2Capsule as *const c_void,
                C2_TYPE_CAPSULE,
                std::ptr::null(),
                a,
                b,
                1,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
        }
    }
}
