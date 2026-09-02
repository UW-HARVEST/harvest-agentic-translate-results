//! Rust translation of the C library in `c_src/`.
//!
//! This is a faithful, ABI-compatible translation of `c_src/src/lib.c`
//! (a trimmed-down subset of Randy Gaul's `cute_c2`). Every non-static C
//! function is re-exported here with the exact same linker symbol name and
//! the exact same C signature.
//!
//! Behaviour (including quirks/bugs of the original) is preserved verbatim:
//! no error checks are reordered, no comparisons are "fixed up", and the
//! floating point arithmetic is performed in the same order with the same
//! single-precision types so results are bit-identical.

#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use core::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// C2_TYPE
// ---------------------------------------------------------------------------

// typedef enum { C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE } C2_TYPE;
const C2_TYPE_CIRCLE: c_int = 0;
const C2_TYPE_AABB: c_int = 1;
const C2_TYPE_CAPSULE: c_int = 2;

// ---------------------------------------------------------------------------
// Float constants used literally by the C source
// ---------------------------------------------------------------------------

/// `3.40282346638528859811704183484516925e+38F` (FLT_MAX)
const FLT_MAX: f32 = 3.402_823_466_385_288_598_117_041_834_845_169_25e+38;
/// `1.19209289550781250000000000000000000e-7F` (FLT_EPSILON)
const FLT_EPSILON: f32 = 1.192_092_895_507_812_5e-7;

// ---------------------------------------------------------------------------
// Public-by-layout types
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

/// `typedef struct { c2v sA; c2v sB; c2v p; float u; int iA; int iB; } c2sv;`
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
/// The four `c2sv` members are modelled as an array because the C code takes
/// `&s.a` and indexes it as an array (`c2sv *verts = &s.a;`). `#[repr(C)]`
/// gives the array exactly the same layout as four consecutive members.
#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2Simplex {
    /// `a`, `b`, `c`, `d`
    pub v: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

// ---------------------------------------------------------------------------
// Vector helpers
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
    a.x *= b;
    a.y *= b;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    // Reproduces the C ternaries exactly (including NaN behaviour, which
    // differs from f32::max).
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

/// `void c2BBVerts(c2v *out, c2AABB *bb)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2BBVerts(out: *mut c2v, bb: *mut c2AABB) {
    unsafe {
        let bb = &*bb;
        *out.add(0) = bb.min;
        *out.add(1) = c2V(bb.max.x, bb.min.y);
        *out.add(2) = bb.max;
        *out.add(3) = c2V(bb.min.x, bb.max.y);
    }
}

/// `void c2MakeProxy(const void *shape, C2_TYPE type, c2Proxy *p)`
///
/// Note: as in the C original, an unrecognised `type` leaves `*p` completely
/// untouched (the C `switch` has no `default` label).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(shape: *const c_void, type_: c_int, p: *mut c2Proxy) {
    unsafe {
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
}

// ---------------------------------------------------------------------------
// Simplex helpers
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> f32 {
    c2Dot(a, a).sqrt()
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: c2v, b: c2v) -> f32 {
    a.x * b.y - a.y * b.x
}

/// `float c2GJKSimplexMetric(c2Simplex *s)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJKSimplexMetric(s: *mut c2Simplex) -> f32 {
    unsafe {
        let s = &*s;
        match s.count {
            2 => c2Len(c2Sub(s.v[1].p, s.v[0].p)),
            3 => c2Det2(c2Sub(s.v[1].p, s.v[0].p), c2Sub(s.v[2].p, s.v[0].p)),
            // `default:` falls through into `case 1:` in the C source.
            _ => 0.0,
        }
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

/// `void c22(c2Simplex *s)` — 2-simplex (line segment) reduction.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c22(s: *mut c2Simplex) {
    unsafe {
        let a = (*s).v[0].p;
        let b = (*s).v[1].p;
        let u = c2Dot(b, c2Sub(b, a));
        let v = c2Dot(a, c2Sub(a, b));
        if v <= 0.0 {
            (*s).v[0].u = 1.0f32;
            (*s).div = 1.0f32;
            (*s).count = 1;
        } else if u <= 0.0 {
            (*s).v[0] = (*s).v[1];
            (*s).v[0].u = 1.0f32;
            (*s).div = 1.0f32;
            (*s).count = 1;
        } else {
            (*s).v[0].u = u;
            (*s).v[1].u = v;
            (*s).div = u + v;
            (*s).count = 2;
        }
    }
}

/// `void c23(c2Simplex *s)` — 3-simplex (triangle) reduction.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c23(s: *mut c2Simplex) {
    unsafe {
        let a = (*s).v[0].p;
        let b = (*s).v[1].p;
        let c = (*s).v[2].p;
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
            (*s).v[0].u = 1.0f32;
            (*s).div = 1.0f32;
            (*s).count = 1;
        } else if uAB <= 0.0 && vBC <= 0.0 {
            (*s).v[0] = (*s).v[1];
            (*s).v[0].u = 1.0f32;
            (*s).div = 1.0f32;
            (*s).count = 1;
        } else if uBC <= 0.0 && vCA <= 0.0 {
            (*s).v[0] = (*s).v[2];
            (*s).v[0].u = 1.0f32;
            (*s).div = 1.0f32;
            (*s).count = 1;
        } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
            (*s).v[0].u = uAB;
            (*s).v[1].u = vAB;
            (*s).div = uAB + vAB;
            (*s).count = 2;
        } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
            (*s).v[0] = (*s).v[1];
            (*s).v[1] = (*s).v[2];
            (*s).v[0].u = uBC;
            (*s).v[1].u = vBC;
            (*s).div = uBC + vBC;
            (*s).count = 2;
        } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
            (*s).v[1] = (*s).v[0];
            (*s).v[0] = (*s).v[2];
            (*s).v[0].u = uCA;
            (*s).v[1].u = vCA;
            (*s).div = uCA + vCA;
            (*s).count = 2;
        } else {
            (*s).v[0].u = uABC;
            (*s).v[1].u = vABC;
            (*s).v[2].u = wABC;
            (*s).div = uABC + vABC + wABC;
            (*s).count = 3;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Neg(a: c2v) -> c2v {
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

/// `c2v c2D(c2Simplex *s)` — search direction.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2D(s: *mut c2Simplex) -> c2v {
    unsafe {
        let s = &*s;
        match s.count {
            1 => c2Neg(s.v[0].p),
            2 => {
                let ab = c2Sub(s.v[1].p, s.v[0].p);
                if c2Det2(ab, c2Neg(s.v[0].p)) > 0.0 {
                    return c2Skew(ab);
                }
                c2CCW90(ab)
            }
            // `case 3:` and `default:` both return the zero vector.
            _ => c2V(0.0, 0.0),
        }
    }
}

/// `int c2Support(const c2v *verts, int count, c2v d)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Support(verts: *const c2v, count: c_int, d: c2v) -> c_int {
    unsafe {
        let mut imax: c_int = 0;
        // As in C, verts[0] is read unconditionally, even when count <= 0.
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

/// `void c2Witness(c2Simplex *s, c2v *a, c2v *b)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Witness(s: *mut c2Simplex, a: *mut c2v, b: *mut c2v) {
    unsafe {
        let sr = &*s;
        let den = 1.0f32 / sr.div;
        match sr.count {
            1 => {
                *a = sr.v[0].sA;
                *b = sr.v[0].sB;
            }
            2 => {
                *a = c2Add(
                    c2Mulvs(sr.v[0].sA, den * sr.v[0].u),
                    c2Mulvs(sr.v[1].sA, den * sr.v[1].u),
                );
                *b = c2Add(
                    c2Mulvs(sr.v[0].sB, den * sr.v[0].u),
                    c2Mulvs(sr.v[1].sB, den * sr.v[1].u),
                );
            }
            3 => {
                *a = c2Add(
                    c2Add(
                        c2Mulvs(sr.v[0].sA, den * sr.v[0].u),
                        c2Mulvs(sr.v[1].sA, den * sr.v[1].u),
                    ),
                    c2Mulvs(sr.v[2].sA, den * sr.v[2].u),
                );
                *b = c2Add(
                    c2Add(
                        c2Mulvs(sr.v[0].sB, den * sr.v[0].u),
                        c2Mulvs(sr.v[1].sB, den * sr.v[1].u),
                    ),
                    c2Mulvs(sr.v[2].sB, den * sr.v[2].u),
                );
            }
            _ => {
                *a = c2V(0.0, 0.0);
                *b = c2V(0.0, 0.0);
            }
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

/// `c2v c2L(c2Simplex *s)` — closest point on the simplex to the origin.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2L(s: *mut c2Simplex) -> c2v {
    unsafe {
        let s = &*s;
        let den = 1.0f32 / s.div;
        match s.count {
            1 => s.v[0].p,
            2 => c2Add(
                c2Mulvs(s.v[0].p, den * s.v[0].u),
                c2Mulvs(s.v[1].p, den * s.v[1].u),
            ),
            _ => c2V(0.0, 0.0),
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

// ---------------------------------------------------------------------------
// GJK
// ---------------------------------------------------------------------------

/// ```c
/// float c2GJK(const void *A, C2_TYPE typeA, const c2x *ax_ptr,
///             const void *B, C2_TYPE typeB, const c2x *bx_ptr,
///             c2v *outA, c2v *outB, int use_radius, int *iterations,
///             c2GJKCache *cache);
/// ```
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
        // `c2sv *verts = &s.a;`
        let verts: *mut c2sv = s.v.as_mut_ptr();

        let mut cache_was_read = 0;
        if !cache.is_null() {
            let cache_was_good = (*cache).count != 0;
            if cache_was_good {
                let mut i: c_int = 0;
                while i < (*cache).count {
                    let iA = (*cache).iA[i as usize];
                    let iB = (*cache).iB[i as usize];
                    let sA = c2Mulxv(ax, pA.verts[iA as usize]);
                    let sB = c2Mulxv(bx, pB.verts[iB as usize]);
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
                // Preserved verbatim from the C source (including the
                // surprising `metric < -1.0e8f` term).
                if !(min_metric < max_metric * 2.0f32 && metric < -1.0e8f32) {
                    cache_was_read = 1;
                }
            }
        }

        if cache_was_read == 0 {
            s.v[0].iA = 0;
            s.v[0].iB = 0;
            s.v[0].sA = c2Mulxv(ax, pA.verts[0]);
            s.v[0].sB = c2Mulxv(bx, pB.verts[0]);
            s.v[0].p = c2Sub(s.v[0].sB, s.v[0].sA);
            s.v[0].u = 1.0f32;
            s.div = 1.0f32;
            s.count = 1;
        }

        let mut saveA: [c_int; 3] = [0; 3];
        let mut saveB: [c_int; 3] = [0; 3];
        let mut save_count: c_int;
        let mut d0 = FLT_MAX;
        let mut d1;
        let mut iter: c_int = 0;
        let mut hit = 0;
        while iter < 20 {
            save_count = s.count;
            let mut i: c_int = 0;
            while i < save_count {
                saveA[i as usize] = (*verts.offset(i as isize)).iA;
                saveB[i as usize] = (*verts.offset(i as isize)).iB;
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
            if c2Dot(d, d) < FLT_EPSILON * FLT_EPSILON {
                break;
            }

            let iA = c2Support(pA.verts.as_ptr(), pA.count, c2MulrvT(ax.r, c2Neg(d)));
            let sA = c2Mulxv(ax, pA.verts[iA as usize]);
            let iB = c2Support(pB.verts.as_ptr(), pB.count, c2MulrvT(bx.r, d));
            let sB = c2Mulxv(bx, pB.verts[iB as usize]);

            let v = verts.offset(s.count as isize);
            (*v).iA = iA;
            (*v).sA = sA;
            (*v).iB = iB;
            (*v).sB = sB;
            (*v).p = c2Sub((*v).sB, (*v).sA);

            let mut dup = 0;
            let mut i: c_int = 0;
            while i < save_count {
                if iA == saveA[i as usize] && iB == saveB[i as usize] {
                    dup = 1;
                    break;
                }
                i += 1;
            }
            if dup != 0 {
                break;
            }

            s.count += 1;
            iter += 1;
        }

        let mut a = c2v::default();
        let mut b = c2v::default();
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
                let v = verts.offset(i as isize);
                (*cache).iA[i as usize] = (*v).iA;
                (*cache).iB[i as usize] = (*v).iB;
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
// Boolean collision routines
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> c_int {
    let d0 = (B.max.x < A.min.x) as c_int;
    let d1 = (A.max.x < B.min.x) as c_int;
    let d2 = (B.max.y < A.min.y) as c_int;
    let d3 = (A.max.y < B.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoCapsule(A: c2AABB, B: c2Capsule) -> c_int {
    let mut a = A;
    let mut b = B;
    unsafe {
        if c2GJK(
            &mut a as *mut c2AABB as *const c_void,
            C2_TYPE_AABB,
            core::ptr::null(),
            &mut b as *mut c2Capsule as *const c_void,
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
    let mut a = A;
    let mut b = B;
    unsafe {
        if c2GJK(
            &mut a as *mut c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            core::ptr::null(),
            &mut b as *mut c2Capsule as *const c_void,
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
    let d2;
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

/// `int c2Collided(const void *A, C2_TYPE typeA, const void *B, C2_TYPE typeB)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Collided(
    A: *const c_void,
    typeA: c_int,
    B: *const c_void,
    typeB: c_int,
) -> c_int {
    unsafe {
        match typeA {
            C2_TYPE_CIRCLE => match typeB {
                C2_TYPE_CIRCLE => c2CircletoCircle(*(A as *const c2Circle), *(B as *const c2Circle)),
                C2_TYPE_AABB => c2CircletoAABB(*(A as *const c2Circle), *(B as *const c2AABB)),
                C2_TYPE_CAPSULE => {
                    c2CircletoCapsule(*(A as *const c2Circle), *(B as *const c2Capsule))
                }
                _ => 0,
            },
            C2_TYPE_AABB => match typeB {
                C2_TYPE_CIRCLE => c2CircletoAABB(*(B as *const c2Circle), *(A as *const c2AABB)),
                C2_TYPE_AABB => c2AABBtoAABB(*(A as *const c2AABB), *(B as *const c2AABB)),
                C2_TYPE_CAPSULE => c2AABBtoCapsule(*(A as *const c2AABB), *(B as *const c2Capsule)),
                _ => 0,
            },
            C2_TYPE_CAPSULE => match typeB {
                C2_TYPE_CIRCLE => {
                    c2CircletoCapsule(*(B as *const c2Circle), *(A as *const c2Capsule))
                }
                C2_TYPE_AABB => c2AABBtoCapsule(*(B as *const c2AABB), *(A as *const c2Capsule)),
                C2_TYPE_CAPSULE => {
                    c2CapsuletoCapsule(*(A as *const c2Capsule), *(B as *const c2Capsule))
                }
                _ => 0,
            },
            _ => 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Public entry point (include/lib.h)
// ---------------------------------------------------------------------------

/// `int capsule(float min_x, float min_y, float max_x, float max_y, float r)`
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

    let mut capsule = c2Capsule::default();
    capsule.a = c2V(-40.0f32, 40.0f32);
    capsule.b = c2V(-20.0f32, 100.0f32);
    capsule.r = 10.0f32;

    unsafe {
        result += c2Collided(
            &mut circle as *mut c2Circle as *const c_void,
            C2_TYPE_CIRCLE,
            &mut capsule_in as *mut c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
        );

        result += c2Collided(
            &mut aabb as *mut c2AABB as *const c_void,
            C2_TYPE_AABB,
            &mut capsule_in as *mut c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
        ) << 1;

        result += c2Collided(
            &mut capsule as *mut c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            &mut capsule_in as *mut c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
        ) << 2;
    }

    result
}
